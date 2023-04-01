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
        ),
        Added<Client>,
    >,
    instances: Query<Entity, With<Instance>>,
) {
    let inst = instances.single();
    for (mut client, mut pos, mut loc, mut is_flat, mut mode) in &mut clients {
        *mode = GameMode::Creative;
        is_flat.0 = true;
        pos.0 = (0., -39., 0.).into();
        loc.0 = inst;

        client.send_message("Welcome to my server!");
    }
}
