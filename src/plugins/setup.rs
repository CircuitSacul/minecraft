use std::time::Duration;

use bevy::time::{Time, Timer, TimerMode};
use dashmap::DashSet;
use r2d2_sqlite::rusqlite::params;
use rayon::prelude::IntoParallelIterator;
use rayon::prelude::*;
use valence::prelude::*;

use crate::POOL;

#[derive(Resource)]
struct ChunkGenTimer(Timer);

pub struct SetupPlugin;

impl Plugin for SetupPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ChunkGenTimer(Timer::new(
            Duration::from_secs(1),
            TimerMode::Repeating,
        )))
        .add_startup_system(build_world)
        .add_system(init_clients)
        .add_system(generate_chunks)
        .add_system(default_event_handler.in_schedule(EventLoopSchedule))
        .add_systems(PlayerList::default_systems())
        .add_system(despawn_disconnected_clients);
    }
}

fn build_world(
    mut commands: Commands,
    server: Res<Server>,
    dimensions: Query<&DimensionType>,
    biomes: Query<&Biome>,
) {
    commands.spawn(Instance::new(
        ident!("overworld"),
        &dimensions,
        &biomes,
        &server,
    ));
}

fn init_clients(
    mut clients: Query<
        (
            &mut Position,
            &mut Location,
            &mut IsFlat,
            &mut GameMode,
            &mut ViewDistance,
        ),
        Added<Client>,
    >,
    mut instances: Query<(Entity, &mut Instance)>,
) {
    let (ent, mut inst) = instances.single_mut();
    let mut positions = Vec::new();
    for (mut pos, mut loc, mut is_flat, mut mode, mut dist) in &mut clients {
        *mode = GameMode::Survival;
        is_flat.0 = true;
        pos.0 = (0., 1., 0.).into();
        loc.0 = ent;
        dist.set(32);

        positions.push((*pos, dist.get()));
    }

    // generate the closest chunks to ensure that the user doesn't fall out of the world
    let ret = ensure_chunks(&mut inst, positions, None);
    if let Err(why) = ret {
        eprintln!("{why}");
    }
}

fn generate_chunks(
    time: Res<Time>,
    mut timer: ResMut<ChunkGenTimer>,
    mut instances: Query<&mut Instance>,
    clients: Query<View, With<Client>>,
) {
    timer.0.tick(time.delta());
    if !timer.0.finished() {
        return;
    }

    let mut inst = instances.single_mut();

    let all_chunks = DashSet::new();

    let ret = ensure_chunks(
        &mut inst,
        clients
            .into_iter()
            .map(|view| (*view.pos, view.view_dist.get()))
            .collect::<Vec<_>>(),
        Some(&all_chunks),
    );
    if let Err(why) = ret {
        eprintln!("{why}");
    }

    inst.retain_chunks(|pos, _| all_chunks.contains(&pos));
}

fn ensure_chunks<Iter: IntoParallelIterator<Item = (Position, u8)>>(
    inst: &mut Instance,
    positions: Iter,
    chunks_set: Option<&DashSet<ChunkPos>>,
) -> anyhow::Result<()> {
    let chunks: Vec<_> = positions
        .into_par_iter()
        .flat_map(|(pos, dist)| viewable_chunks(pos, dist))
        .filter_map(|pos| {
            if let Some(hs) = chunks_set {
                if !hs.insert(pos) {
                    return None;
                }
            }
            Some(pos)
        })
        .filter(|pos| inst.chunk(*pos).is_none())
        .map(|pos| (pos, single_chunk(&pos).unwrap()))
        .collect();

    for (pos, chunk) in chunks {
        inst.insert_chunk(pos, chunk);
    }

    Ok(())
}

fn single_chunk(pos: &ChunkPos) -> anyhow::Result<Chunk> {
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
    let iter = stmt.query_map(params![pos.x, pos.z], |row| {
        Ok((
            row.get_unwrap::<_, i64>(0),
            row.get_unwrap::<_, i64>(1),
            row.get_unwrap::<_, i64>(2),
            BlockState::from_raw(row.get_unwrap(3)),
        ))
    })?;
    for row in iter {
        let (x, y, z, block) = row?;

        chunk.set_block_state(
            x.try_into()?,
            (y + 64).try_into()?,
            z.try_into()?,
            block.unwrap(),
        );
    }

    Ok(chunk)
}

fn viewable_chunks(pos: Position, dist: u8) -> impl ParallelIterator<Item = ChunkPos> {
    let dist: i32 = dist.into();
    let pos = ChunkPos::at(pos.get().x, pos.get().z);

    (0..=dist)
        .into_par_iter()
        .flat_map(move |d| {
            let x_rng = (pos.x - d..=pos.x + d).into_par_iter();
            let z_rng = (pos.z - d + 1..pos.z + d).into_par_iter();

            let x_lines = x_rng.flat_map(move |x| [(x, pos.z + d), (x, pos.z - d)]);
            let z_lines = z_rng.flat_map(move |z| [(pos.x + d, z), (pos.x - d, z)]);

            x_lines.chain(z_lines)
        })
        .map(|(x, z)| ChunkPos::new(x, z))
}
