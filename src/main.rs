mod plugins;

use valence::prelude::*;

fn main() {
    App::new()
        .add_plugin(ServerPlugin::new(()))
        .add_plugin(bevy_tokio_tasks::TokioTasksPlugin::default())
        .add_plugin(plugins::SetupPlugin)
        .add_plugin(plugins::ChunkLoader)
        .run();
}
