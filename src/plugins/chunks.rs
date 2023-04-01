use std::collections::HashSet;
use std::time::Duration;
use std::time::Instant;

use bevy_tokio_tasks::TokioTasksRuntime;
use valence::bevy_ecs;
use valence::prelude::*;

#[derive(Resource, Default)]
pub struct ChunkQueue(pub Vec<ChunkPos>);

#[derive(Resource)]
struct TickStart(Instant);

pub struct ChunkLoader;

impl Plugin for ChunkLoader {
    fn build(&self, app: &mut App) {
        app.insert_resource(TickStart(Instant::now()))
            .insert_resource(ChunkQueue::default())
            .add_startup_system(unload_chunks_worker)
            .add_startup_system(generate_chunks_worker)
            .add_system(
                generate_chunks
                    .run_if(|tick: Res<TickStart>| (Instant::now() - tick.0).as_millis() > 500),
            );
    }
}

fn unload_chunks_worker(runtime: ResMut<TokioTasksRuntime>) {
    runtime.spawn_background_task(|mut ctx| async move {
        loop {
            ctx.run_on_main_thread(|ctx| {
                let mut query = ctx.world.query::<&mut Instance>();
                let mut inst = query.single_mut(ctx.world);

                inst.retain_chunks(|pos, chunk| chunk.is_viewed() || (pos.x == 0 && pos.z == 0));
            })
            .await;

            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });
}

fn generate_chunks_worker(runtime: ResMut<TokioTasksRuntime>) {
    runtime.spawn_background_task(|mut ctx| async move {
        loop {
            ctx.run_on_main_thread(|ctx| {
                let queue = {
                    let mut queue = ctx.world.resource_mut::<ChunkQueue>();
                    (0..100).filter_map(|_| queue.0.pop()).collect::<Vec<_>>()
                };

                dbg!(queue.len());
                if !queue.is_empty() {
                    dbg!(&queue[0]);
                }

                let mut inst = {
                    let mut query = ctx.world.query::<&mut Instance>();
                    query.single_mut(ctx.world)
                };

                for pos in queue {
                    if inst.chunk(pos).is_some() {
                        continue;
                    }

                    let mut chunk = Chunk::new(4);

                    for x in 0..16 {
                        for z in 0..16 {
                            chunk.set_block(x, 1, z, BlockState::GRASS_BLOCK);
                        }
                    }

                    inst.insert_chunk(pos, chunk);
                }
            })
            .await;

            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });
}

#[allow(clippy::type_complexity)]
fn generate_chunks(
    mut tick: ResMut<TickStart>,
    mut queue: ResMut<ChunkQueue>,
    instances: Query<&Instance>,
    mut clients: Query<View, (With<Client>, Or<(Changed<Position>, Changed<ViewDistance>)>)>,
) {
    tick.0 = Instant::now();

    let inst = instances.single();

    let mut all_chunk_pos = HashSet::new();
    for view in &mut clients {
        for chunk in viewable_chunks(&view) {
            if inst.chunk(chunk).is_some() {
                continue;
            }

            let dist: u32 = chunk
                .distance_squared(ChunkPos::at(view.pos.get().x, view.pos.get().z))
                .try_into()
                .unwrap();
            let priority = ((1f32 / dist as f32) * 1_000_f32) as u32;

            all_chunk_pos.insert((priority, chunk));
        }
    }

    let mut all_chunk_pos: Vec<_> = all_chunk_pos.into_iter().collect();
    all_chunk_pos.sort_by(|a, b| a.0.cmp(&b.0));
    queue.0 = all_chunk_pos.into_iter().map(|(_, chunk)| chunk).collect();
}

fn viewable_chunks(view: &ViewItem) -> impl Iterator<Item = ChunkPos> {
    let pos = ChunkPos::at(view.pos.get().x, view.pos.get().z);
    let dist: i32 = view.view_dist.get().into();

    let x_range = pos.x - dist..pos.x + dist;
    let z_range = pos.z - dist..pos.z + dist;

    x_range.flat_map(move |x| z_range.clone().map(move |z| ChunkPos::new(x, z)))
}
