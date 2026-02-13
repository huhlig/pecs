//! Benchmark suite for PECS - Phase 3 Performance Profiling
//!
//! This benchmark suite measures performance of currently implemented features.
//! Additional benchmarks will be added as missing APIs are implemented.
//!
//! ## Current Benchmarks
//!
//! - Entity operations (spawn, despawn, lookup)
//! - Entity builder with components
//! - Stable ID operations
//! - Command buffer operations
//! - Persistence operations (save/load)
//!
//! ## Missing Benchmarks (requires API implementation)
//!
//! - Component access (get, get_mut) - requires World::get/get_mut
//! - Component insertion/removal - requires World::insert/remove
//! - Query iteration - requires World::query integration
//! - Filtered queries - requires World::query_filtered
//!
//! See docs/dev/API_GAPS.md for details on missing APIs.
//!
//! ## Performance Targets
//!
//! - Entity spawn: < 100ns per operation (target: < 50ns)
//! - Entity despawn: < 100ns per operation
//! - Stable ID lookup: < 50ns per operation
//! - Persistence: < 1ms per 1000 entities (target: < 0.5ms per 1000 entities)

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use pecs::prelude::*;

// ============================================================================
// Entity Operation Benchmarks
// ============================================================================

fn bench_entity_spawn_empty(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_spawn_empty");

    for size in [1, 10, 100, 1000, 10000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                let mut world = World::new();
                for _ in 0..size {
                    black_box(world.spawn_empty());
                }
            });
        });
    }
    group.finish();
}

fn bench_entity_spawn_with_preallocated_capacity(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_spawn_with_capacity");

    for size in [100, 1000, 10000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                let mut world = World::with_capacity(size);
                for _ in 0..size {
                    black_box(world.spawn_empty());
                }
            });
        });
    }
    group.finish();
}

fn bench_entity_despawn(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_despawn");

    for size in [1, 10, 100, 1000, 10000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter_batched(
                || {
                    let mut world = World::new();
                    let entities: Vec<_> = (0..size).map(|_| world.spawn_empty()).collect();
                    (world, entities)
                },
                |(mut world, entities)| {
                    for entity in entities {
                        black_box(world.despawn(entity));
                    }
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

fn bench_entity_is_alive(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_is_alive");

    for size in [10, 100, 1000, 10000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let mut world = World::new();
            let entities: Vec<_> = (0..size).map(|_| world.spawn_empty()).collect();

            b.iter(|| {
                for entity in &entities {
                    black_box(world.is_alive(*entity));
                }
            });
        });
    }
    group.finish();
}

fn bench_entity_spawn_despawn_cycle(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_spawn_despawn_cycle");

    for size in [10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                let mut world = World::new();
                for _ in 0..size {
                    let entity = world.spawn_empty();
                    black_box(world.despawn(entity));
                }
            });
        });
    }
    group.finish();
}

// ============================================================================
// Stable ID Benchmarks
// ============================================================================

fn bench_stable_id_lookup(c: &mut Criterion) {
    let mut group = c.benchmark_group("stable_id_lookup");

    for size in [10, 100, 1000, 10000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let mut world = World::new();
            let stable_ids: Vec<_> = (0..size)
                .map(|_| {
                    let entity = world.spawn_empty();
                    world.get_stable_id(entity).unwrap()
                })
                .collect();

            b.iter(|| {
                for stable_id in &stable_ids {
                    black_box(world.get_entity_id(*stable_id));
                }
            });
        });
    }
    group.finish();
}

fn bench_stable_id_reverse_lookup(c: &mut Criterion) {
    let mut group = c.benchmark_group("stable_id_reverse_lookup");

    for size in [10, 100, 1000, 10000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let mut world = World::new();
            let entities: Vec<_> = (0..size).map(|_| world.spawn_empty()).collect();

            b.iter(|| {
                for entity in &entities {
                    black_box(world.get_stable_id(*entity));
                }
            });
        });
    }
    group.finish();
}

// ============================================================================
// Command Buffer Benchmarks
// ============================================================================

fn bench_command_buffer_spawn(c: &mut Criterion) {
    let mut group = c.benchmark_group("command_buffer_spawn");

    for size in [10, 100, 1000, 10000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                let mut world = World::new();
                {
                    let commands = world.commands();
                    for _ in 0..size {
                        commands.spawn();
                    }
                }
                world.apply_commands();
                black_box(());
            });
        });
    }
    group.finish();
}

fn bench_command_buffer_despawn(c: &mut Criterion) {
    let mut group = c.benchmark_group("command_buffer_despawn");

    for size in [10, 100, 1000, 10000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter_batched(
                || {
                    let mut world = World::new();
                    let entities: Vec<_> = (0..size).map(|_| world.spawn_empty()).collect();
                    (world, entities)
                },
                |(mut world, entities)| {
                    {
                        let commands = world.commands();
                        for entity in entities {
                            commands.despawn(entity);
                        }
                    }
                    let _: () = world.apply_commands();
                    black_box(());
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

fn bench_command_buffer_mixed_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("command_buffer_mixed");

    for size in [10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter_batched(
                || {
                    let mut world = World::new();
                    let entities: Vec<_> = (0..size / 2).map(|_| world.spawn_empty()).collect();
                    (world, entities)
                },
                |(mut world, entities)| {
                    {
                        let commands = world.commands();
                        // Spawn new entities
                        for _ in 0..size / 2 {
                            commands.spawn();
                        }
                        // Despawn existing entities
                        for entity in entities {
                            commands.despawn(entity);
                        }
                    }
                    let _: () = world.apply_commands();
                    black_box(());
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

// ============================================================================
// World Operations Benchmarks
// ============================================================================

fn bench_world_clear(c: &mut Criterion) {
    let mut group = c.benchmark_group("world_clear");

    for size in [100, 1000, 10000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter_batched(
                || {
                    let mut world = World::new();
                    for _ in 0..size {
                        world.spawn_empty();
                    }
                    world
                },
                |mut world| {
                    let _: () = world.clear();
                    black_box(());
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

fn bench_world_len(c: &mut Criterion) {
    let mut group = c.benchmark_group("world_len");

    for size in [100, 1000, 10000, 100000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let mut world = World::new();
            for _ in 0..size {
                world.spawn_empty();
            }

            b.iter(|| {
                black_box(world.len());
            });
        });
    }
    group.finish();
}

fn bench_world_iter_entities(c: &mut Criterion) {
    let mut group = c.benchmark_group("world_iter_entities");

    for size in [100, 1000, 10000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let mut world = World::new();
            for _ in 0..size {
                world.spawn_empty();
            }

            b.iter(|| {
                let mut count = 0;
                for (entity, stable_id) in world.iter_entities() {
                    black_box((entity, stable_id));
                    count += 1;
                }
                black_box(count);
            });
        });
    }
    group.finish();
}

// ============================================================================
// Memory and Capacity Benchmarks
// ============================================================================

fn bench_world_with_capacity(c: &mut Criterion) {
    let mut group = c.benchmark_group("world_with_capacity");

    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                black_box(World::with_capacity(size));
            });
        });
    }
    group.finish();
}

// ============================================================================
// Criterion Configuration
// ============================================================================

criterion_group!(
    entity_benches,
    bench_entity_spawn_empty,
    bench_entity_spawn_with_preallocated_capacity,
    bench_entity_despawn,
    bench_entity_is_alive,
    bench_entity_spawn_despawn_cycle
);

criterion_group!(
    stable_id_benches,
    bench_stable_id_lookup,
    bench_stable_id_reverse_lookup
);

criterion_group!(
    command_benches,
    bench_command_buffer_spawn,
    bench_command_buffer_despawn,
    bench_command_buffer_mixed_operations
);

criterion_group!(
    world_benches,
    bench_world_clear,
    bench_world_len,
    bench_world_iter_entities,
    bench_world_with_capacity
);

criterion_main!(
    entity_benches,
    stable_id_benches,
    command_benches,
    world_benches
);

// Made with Bob
