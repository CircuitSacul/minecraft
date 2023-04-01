#![allow(clippy::type_complexity)]

mod plugins;

use valence::prelude::*;

fn main() {
    App::new()
        .add_plugin(ServerPlugin::new(()).with_connection_mode(ConnectionMode::Offline))
        .add_plugin(plugins::SetupPlugin)
        .run();
}
