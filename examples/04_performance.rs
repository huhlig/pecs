//! # Performance Example
//!
//! This example demonstrates performance best practices in PECS:
//! - Pre-allocating capacity for known entity counts
//! - Batch operations with command buffers
//! - Efficient entity lifecycle management
//! - Measuring performance
//!
//! Run with: `cargo run --example 04_performance --release`

use pecs::prelude::*;
use std::time::Instant;

fn main() {
    println!("=== PECS Performance Example ===\n");

    // Demonstrate the performance difference between pre-allocation and dynamic growth
    println!("--- Pre-allocation vs Dynamic Growth ---\n");

    // Without pre-allocation
    let start = Instant::now();
    let mut world_no_prealloc = World::new();
    for _ in 0..10_000 {
        world_no_prealloc.spawn_empty();
    }
    let duration_no_prealloc = start.elapsed();
    println!(
        "Without pre-allocation: {:?} for 10,000 entities",
        duration_no_prealloc
    );
    println!("  Average: {:?} per entity", duration_no_prealloc / 10_000);

    // With pre-allocation
    let start = Instant::now();
    let mut world_prealloc = World::with_capacity(10_000);
    for _ in 0..10_000 {
        world_prealloc.spawn_empty();
    }
    let duration_prealloc = start.elapsed();
    println!(
        "With pre-allocation:    {:?} for 10,000 entities",
        duration_prealloc
    );
    println!("  Average: {:?} per entity", duration_prealloc / 10_000);

    let speedup = duration_no_prealloc.as_nanos() as f64 / duration_prealloc.as_nanos() as f64;
    println!("  Speedup: {:.2}x faster\n", speedup);

    // Demonstrate batch operations with command buffers
    println!("--- Command Buffer Batching ---\n");

    let mut world = World::with_capacity(10_000);

    // Direct spawning
    let start = Instant::now();
    for _ in 0..1_000 {
        world.spawn_empty();
    }
    let duration_direct = start.elapsed();
    println!("Direct spawning: {:?} for 1,000 entities", duration_direct);

    world.clear();

    // Batched spawning with command buffer
    let start = Instant::now();
    for _ in 0..1_000 {
        world.commands().spawn();
    }
    world.apply_commands();
    let duration_batched = start.elapsed();
    println!(
        "Batched spawning: {:?} for 1,000 entities",
        duration_batched
    );
    println!("  (includes command recording + application)\n");

    // Demonstrate efficient entity lifecycle
    println!("--- Entity Lifecycle Performance ---\n");

    let mut world = World::with_capacity(1_000);

    // Spawn entities
    let start = Instant::now();
    let mut entities = Vec::with_capacity(1_000);
    for _ in 0..1_000 {
        entities.push(world.spawn_empty());
    }
    let spawn_duration = start.elapsed();
    println!("Spawn 1,000 entities: {:?}", spawn_duration);

    // Check if alive (fast operation)
    let start = Instant::now();
    let mut alive_count = 0;
    for &entity in &entities {
        if world.is_alive(entity) {
            alive_count += 1;
        }
    }
    let check_duration = start.elapsed();
    println!(
        "Check 1,000 entities: {:?} ({} alive)",
        check_duration, alive_count
    );

    // Despawn entities
    let start = Instant::now();
    for &entity in &entities {
        world.despawn(entity);
    }
    let despawn_duration = start.elapsed();
    println!("Despawn 1,000 entities: {:?}", despawn_duration);

    // Demonstrate stable ID lookup performance
    println!("\n--- Stable ID Lookup Performance ---\n");

    let mut world = World::with_capacity(1_000);
    let mut stable_ids = Vec::with_capacity(1_000);

    for _ in 0..1_000 {
        let entity = world.spawn_empty();
        stable_ids.push(world.get_stable_id(entity).unwrap());
    }

    // Forward lookup (EntityId -> StableId)
    let start = Instant::now();
    for (entity, _) in world.iter_entities() {
        let _ = world.get_stable_id(entity);
    }
    let forward_duration = start.elapsed();
    println!(
        "Forward lookup (EntityId -> StableId): {:?} for 1,000 lookups",
        forward_duration
    );
    println!("  Average: {:?} per lookup", forward_duration / 1_000);

    // Reverse lookup (StableId -> EntityId)
    let start = Instant::now();
    for stable_id in &stable_ids {
        let _ = world.get_entity_id(*stable_id);
    }
    let reverse_duration = start.elapsed();
    println!(
        "Reverse lookup (StableId -> EntityId): {:?} for 1,000 lookups",
        reverse_duration
    );
    println!("  Average: {:?} per lookup", reverse_duration / 1_000);

    // Performance tips
    println!("\n--- Performance Tips ---");
    println!("1. Pre-allocate capacity when you know entity count");
    println!("2. Use command buffers for batch operations");
    println!("3. Stable ID lookups are O(1) but have overhead");
    println!("4. Entity lifecycle operations are very fast (< 100ns)");
    println!("5. Run with --release for accurate performance measurements");

    println!("\n=== Example Complete ===");
}

// Made with Bob
