use std::collections::HashSet;

use valence::prelude::*;

pub struct SetupPlugin;

impl Plugin for SetupPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(build_world)
            .add_system(init_clients.before(generate_chunks))
            .add_system(generate_chunks.before(default_event_handler))
            .add_system(default_event_handler.in_schedule(EventLoopSchedule))
            .add_systems(PlayerList::default_systems())
            .add_system(despawn_disconnected_clients);
    }
}

fn build_world(mut commands: Commands, server: Res<Server>) {
    commands.spawn(server.new_instance(DimensionId::default()));
}

fn init_clients(
    mut clients: Query<
        (
            &mut Client,
            &mut Position,
            &mut Location,
            &mut IsFlat,
            &mut GameMode,
            &ViewDistance,
        ),
        Added<Client>,
    >,
    mut instances: Query<(Entity, &mut Instance)>,
) {
    let (ent, mut inst) = instances.single_mut();
    for (mut client, mut pos, mut loc, mut is_flat, mut mode, dist) in &mut clients {
        *mode = GameMode::Survival;
        is_flat.0 = true;
        pos.0 = (0., 0.1, 0.).into();
        loc.0 = ent;

        // generate the closest chunks to ensure that the user doesn't fall out of the world
        ensure_chunks(&mut inst, &pos, dist.get(), 4, None);

        client.send_message("Welcome to my server!");
    }
}

fn generate_chunks(mut instances: Query<&mut Instance>, mut clients: Query<View, With<Client>>) {
    let mut inst = instances.single_mut();

    let mut all_chunks = HashSet::new();

    for view in &mut clients {
        ensure_chunks(
            &mut inst,
            view.pos,
            view.view_dist.get(),
            8,
            Some(&mut all_chunks),
        );
    }

    inst.retain_chunks(|pos, _| all_chunks.contains(&pos));
}

fn ensure_chunks(
    inst: &mut Instance,
    pos: &Position,
    view_dist: u8,
    limit: usize,
    mut chunks_set: Option<&mut HashSet<ChunkPos>>,
) {
    let mut generated_chunks = 0;
    let mut chunks = viewable_chunks(pos, view_dist);
    for chunk_pos in &mut chunks {
        if let Some(hs) = &mut chunks_set {
            hs.insert(chunk_pos);
        }

        if generated_chunks >= limit {
            break;
        }

        if inst.chunk(chunk_pos).is_some() {
            continue;
        }

        inst.insert_chunk(chunk_pos, single_chunk());
        generated_chunks += 1;
    }

    if let Some(hs) = &mut chunks_set {
        hs.extend(chunks)
    }
}

fn single_chunk() -> Chunk {
    let mut chunk = Chunk::new(4);

    for x in 0..16 {
        for z in 0..16 {
            chunk.set_block_state(x, 63, z, BlockState::GRASS_BLOCK);
        }
    }

    chunk
}

fn viewable_chunks(pos: &Position, dist: u8) -> impl Iterator<Item = ChunkPos> {
    let dist: i32 = dist.into();
    let pos = ChunkPos::at(pos.get().x, pos.get().z);

    (0..=dist)
        .flat_map(move |d| {
            let x_rng = pos.x - d..=pos.x + d;
            let z_rng = pos.z - d + 1..pos.z + d;

            let x_lines = x_rng.flat_map(move |x| [(x, pos.z + d), (x, pos.z - d)]);
            let z_lines = z_rng.flat_map(move |z| [(pos.x + d, z), (pos.x - d, z)]);

            x_lines.chain(z_lines)
        })
        .map(|(x, z)| ChunkPos::new(x, z))
}
