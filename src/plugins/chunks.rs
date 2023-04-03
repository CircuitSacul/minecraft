use bevy::time::{Time, Timer, TimerMode};
use bevy_tokio_tasks::TokioTasksRuntime;
use futures_lite::future;
use fxhash::FxHashSet;
use r2d2_sqlite::rusqlite::params;
use valence::prelude::*;

use crate::{
    utils::{interleave::IntoInterleave, unique::IntoUnique},
    POOL,
};

#[derive(Resource)]
struct ChunksTimer(Timer);

pub struct ChunksPlugin;

impl Plugin for ChunksPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ChunksTimer(Timer::from_seconds(0.5, TimerMode::Repeating)))
            .add_system(ensure_vital_chunks.before(default_event_handler))
            .add_system(generate_chunks);
    }
}

fn ensure_vital_chunks(mut instances: Query<&mut Instance>, clients: Query<View, With<Client>>) {
    let mut inst = instances.single_mut();
    for view in clients.into_iter() {
        let pos = view.pos.chunk_pos();
        let poss = [
            ChunkPos::new(pos.x + 1, pos.z + 1),
            ChunkPos::new(pos.x + 1, pos.z),
            ChunkPos::new(pos.x + 1, pos.z - 1),
            ChunkPos::new(pos.x, pos.z - 1),
            pos,
            ChunkPos::new(pos.x, pos.z + 1),
            ChunkPos::new(pos.x - 1, pos.z + 1),
            ChunkPos::new(pos.x - 1, pos.z),
            ChunkPos::new(pos.x - 1, pos.z - 1),
        ];

        for pos in poss {
            if inst.chunk(pos).is_some() {
                continue;
            }

            inst.insert_chunk(
                pos,
                future::block_on(async move { load_chunk(pos).await.unwrap() }),
            );
        }
    }
}

fn generate_chunks(
    mut timer: ResMut<ChunksTimer>,
    time: Res<Time>,
    clients: Query<View, With<Client>>,
    runtime: Res<TokioTasksRuntime>,
) {
    // tick the timer
    timer.0.tick(time.delta());
    if !timer.0.just_finished() {
        return;
    }

    let positions: Vec<_> = clients.iter().map(|view| view.get()).collect();
    runtime.spawn_background_task(move |mut ctx| async move {
        // generate the chunks that are viewable by players
        let mut viewed_chunks_vec: Vec<_> = positions
            .iter()
            .copied()
            .map(viewable_chunks)
            .interleave()
            .collect();
        let viewed_chunks_set: FxHashSet<_> = viewed_chunks_vec.iter().copied().collect();

        // unload chunks that are no longer viewable
        let viewed_chunks_vec = ctx
            .run_on_main_thread(move |ctx| {
                let mut query = ctx.world.query::<&mut Instance>();
                let mut inst = query.single_mut(ctx.world);

                inst.retain_chunks(|pos, _| viewed_chunks_set.contains(&pos));
                viewed_chunks_vec.retain(|pos| inst.chunk(*pos).is_none());
                viewed_chunks_vec
            })
            .await;

        // load chunks
        let chunks = viewed_chunks_vec
            .into_iter()
            .unique()
            .take(100)
            .map(|pos| tokio::spawn(async move { (pos, load_chunk(pos).await.unwrap()) }));
        let mut chunks_to_insert = Vec::new();
        for chunk_fut in chunks {
            chunks_to_insert.push(chunk_fut.await.unwrap());
        }

        ctx.run_on_main_thread(move |ctx| {
            let mut query = ctx.world.query::<&mut Instance>();
            let mut inst = query.single_mut(ctx.world);

            for (pos, chunk) in chunks_to_insert {
                inst.insert_chunk(pos, chunk);
            }
        })
        .await;
    });
}

async fn load_chunk(pos: ChunkPos) -> anyhow::Result<Chunk> {
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
    let dist: i32 = view.dist.into();

    (0..=dist + 1)
        .flat_map(move |d| {
            let x_rng = pos.x - d..=pos.x + d;
            let z_rng = pos.z - d + 1..pos.z + d;

            let x_lines = x_rng.flat_map(move |x| [(x, pos.z + d), (x, pos.z - d)]);
            let z_lines = z_rng.flat_map(move |z| [(pos.x + d, z), (pos.x - d, z)]);

            x_lines.chain(z_lines)
        })
        .map(|(x, z)| ChunkPos::new(x, z))
}
