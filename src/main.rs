#![allow(clippy::type_complexity)]

mod plugins;

use bevy::time::TimePlugin;
use lazy_static::lazy_static;
use r2d2_sqlite::rusqlite::params;
use valence::prelude::*;

lazy_static! {
    pub static ref POOL: r2d2::Pool<r2d2_sqlite::SqliteConnectionManager> = {
        let manager = r2d2_sqlite::SqliteConnectionManager::file("data/data.db");
        r2d2::Pool::new(manager).unwrap()
    };
}

fn main() -> anyhow::Result<()> {
    POOL.get()?.execute(
        r#"
        CREATE TABLE IF NOT EXISTS blocks (
            x int,
            y int,
            z int,
            chunk_x int,
            chunk_z int,
            block int,
            primary key (x, y, z, chunk_x, chunk_z)
        )"#,
        params![],
    )?;

    App::new()
        .add_plugin(TimePlugin)
        .add_plugin(ServerPlugin::new(()).with_connection_mode(ConnectionMode::Offline))
        .add_plugin(plugins::SetupPlugin)
        .add_plugin(plugins::BuildingPlugin)
        .run();

    Ok(())
}
