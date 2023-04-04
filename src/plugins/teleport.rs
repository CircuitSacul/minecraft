use fxhash::FxHashMap;
use valence::prelude::*;

#[derive(Resource, Default)]
pub struct SetClientLocation(FxHashMap<Entity, (Entity, Position)>);

impl SetClientLocation {
    pub fn set_location(&mut self, client: Entity, loc: Entity, pos: Position) {
        self.0.insert(client, (loc, pos));
    }
}

pub struct TeleportPlugin;

impl Plugin for TeleportPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SetClientLocation::default())
            .add_system(process);
    }
}

fn process(
    insts: Query<&Instance>,
    mut clients: Query<(&mut Location, &mut Position), With<Client>>,
    mut queue: ResMut<SetClientLocation>,
) {
    let inst = insts.single();

    queue.0.retain(|id, (set_loc, set_pos)| {
        if inst.chunk(set_pos.chunk_pos()).is_some() {
            if let Ok((mut loc, mut pos)) = clients.get_mut(*id) {
                loc.0 = *set_loc;
                *pos = *set_pos;
            };

            false
        } else {
            true
        }
    });
}
