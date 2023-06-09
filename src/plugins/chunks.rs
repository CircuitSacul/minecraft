use bevy::tasks::AsyncComputeTaskPool;
use r2d2_sqlite::rusqlite::params;
use valence::prelude::*;

use crate::{
    utils::{interleave::IntoInterleave, unique::IntoUnique},
    POOL,
};

#[derive(Resource)]
pub struct GenChunksTx(flume::Sender<ChunkPos>);

#[derive(Resource)]
pub struct InsertChunksRx(flume::Receiver<(ChunkPos, Chunk)>);

pub struct ChunksPlugin;

impl Plugin for ChunksPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(chunk_gen_worker)
            .add_system(unload_chunks)
            .add_system(insert_chunks)
            .add_system(request_chunk_gen);
    }
}

fn unload_chunks(mut insts: Query<&mut Instance>) {
    let mut inst = insts.single_mut();
    inst.retain_chunks(|_, chunk| chunk.is_viewed_mut());
}

fn insert_chunks(mut insts: Query<&mut Instance>, rx: Res<InsertChunksRx>) {
    let mut inst = insts.single_mut();

    while let Ok((pos, chunk)) = rx.0.try_recv() {
        inst.insert_chunk(pos, chunk);
    }
}

fn request_chunk_gen(
    insts: Query<&Instance>,
    views: Query<(View, OldView, Ref<Client>)>,
    tx: Res<GenChunksTx>,
) {
    let inst = insts.single();
    let chunks = views
        .into_iter()
        .map(|(current, old, client)| {
            let add_all = client.is_added();
            let current = current.get();
            let old = old.get();
            viewable_chunks(current).map(move |pos| (pos, !add_all && old.contains(pos)))
        })
        .interleave()
        .filter_map(|(pos, drop)| if drop { None } else { Some(pos) })
        .filter(|pos| inst.chunk(*pos).is_none())
        .unique();

    for chunk in chunks {
        tx.0.try_send(chunk).unwrap();
    }
}

fn chunk_gen_worker(mut commands: Commands) {
    let (gen_tx, gen_rx) = flume::unbounded();
    let (insert_tx, insert_rx) = flume::unbounded();

    commands.insert_resource(GenChunksTx(gen_tx));
    commands.insert_resource(InsertChunksRx(insert_rx));

    let thread_pool = AsyncComputeTaskPool::get();

    for _ in 0..std::thread::available_parallelism().unwrap().get() {
        let insert_tx = insert_tx.clone();
        let gen_rx = gen_rx.clone();
        thread_pool
            .spawn(async move {
                while let Ok(pos) = gen_rx.recv_async().await {
                    let chunk = match gen_chunk(pos).await {
                        Ok(chunk) => chunk,
                        Err(why) => {
                            eprintln!("Warning: Chunk generation failed: {why}");
                            continue;
                        }
                    };

                    if let Err(why) = insert_tx.try_send((pos, chunk)) {
                        eprintln!("Warning: Sending generated chunk failed: {why}");
                    }
                }
            })
            .detach();
    }
}

async fn gen_chunk(pos: ChunkPos) -> anyhow::Result<Chunk> {
    let mut chunk = Chunk::new(4);

    for x in 0..16 {
        for z in 0..16 {
            chunk.set_block_state(x, 62, z, BlockState::BEDROCK);
            chunk.set_block_state(x, 63, z, BlockState::GRASS_BLOCK);
        }
    }

    let con = POOL.get()?;
    let mut stmt =
        con.prepare("SELECT x, y, z, block FROM blocks WHERE chunk_x=? AND chunk_z=?")?;
    for row in stmt.query_map(params![pos.x, pos.z], |row| {
        Ok((
            row.get::<_, usize>(0)?,
            row.get::<_, usize>(1)?,
            row.get::<_, usize>(2)?,
            BlockState::from_raw(row.get(3)?).unwrap(),
        ))
    })? {
        let (x, y, z, block) = row?;

        chunk.set_block_state(x, y, z, block);
    }

    Ok(chunk)
}

fn viewable_chunks(view: ChunkView) -> impl Iterator<Item = ChunkPos> {
    let pos = view.pos;
    let dist = view.dist as i32 + 2;

    (0..=dist)
        .flat_map(move |d| {
            let x_rng = pos.x - d..=pos.x + d;
            let z_rng = pos.z - d + 1..pos.z + d;

            let x_lines = x_rng.flat_map(move |x| [(x, pos.z + d), (x, pos.z - d)]);
            let z_lines = z_rng.flat_map(move |z| [(pos.x + d, z), (pos.x - d, z)]);

            x_lines.chain(z_lines)
        })
        .map(|(x, z)| ChunkPos::new(x, z))
        .filter(move |pos| view.contains(*pos))
}
