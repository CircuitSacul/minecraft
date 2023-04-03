use valence::prelude::*;

pub struct SetupPlugin;

impl Plugin for SetupPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(build_world)
            .add_system(init_clients)
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
    mut instances: Query<Entity, With<Instance>>,
) {
    let inst = instances.single_mut();
    for (mut pos, mut loc, mut is_flat, mut mode, mut dist) in &mut clients {
        *mode = GameMode::Survival;
        is_flat.0 = true;
        pos.0 = (0., 1., 0.).into();
        loc.0 = inst;
        dist.set(32);
    }
}
