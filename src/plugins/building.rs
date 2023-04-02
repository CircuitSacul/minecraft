use r2d2_sqlite::rusqlite::params;
use valence::prelude::{event::StopDestroyBlock, *};

use crate::POOL;

pub struct BuildingPlugin;

impl Plugin for BuildingPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(block_break.in_schedule(EventLoopSchedule));
    }
}

fn block_break(mut instances: Query<&mut Instance>, mut events: EventReader<StopDestroyBlock>) {
    let mut inst = instances.single_mut();

    for event in events.iter() {
        inst.set_block(event.position, BlockState::AIR);
        if let Err(why) = sql_set_block(event.position, BlockState::AIR) {
            eprintln!("{why}");
        }
    }
}

fn sql_set_block(pos: BlockPos, block: BlockState) -> anyhow::Result<()> {
    let con = POOL.get()?;
    let mut stmt = con.prepare(
        "INSERT INTO blocks (x, y, z, chunk_x, chunk_z, block) VALUES (?, ?, ?, ?, ?, ?)",
    )?;
    let chunk = ChunkPos::from_block_pos(pos);
    stmt.execute(params![
        pos.x.rem_euclid(16),
        pos.y,
        pos.z.rem_euclid(16),
        chunk.x,
        chunk.z,
        block.to_raw()
    ])?;
    Ok(())
}
