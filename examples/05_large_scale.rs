//! # Large-Scale World Management Example
//!
//! This example demonstrates managing large numbers of entities efficiently.
//! It shows:
//! - Creating and managing 100,000+ entities
//! - Efficient batch operations
//! - Memory usage considerations
//! - Performance at scale
//!
//! Run with: `cargo run --example 05_large_scale --release`

use pecs::prelude::*;
use std::time::Instant;

fn main() {
    println!("=== PECS Large-Scale World Management ===\n");

    const ENTITY_COUNT: usize = 100_000;

    println!("Creating a world with {} entities...", ENTITY_COUNT);

    // Pre-allocate for best performance
    let start = Instant::now();
    let mut world = World::with_capacity(ENTITY_COUNT);

    // Spawn entities in batches for better performance
    const BATCH_SIZE: usize = 10_000;
    let mut total_spawned = 0;

    for batch in 0..(ENTITY_COUNT / BATCH_SIZE) {
        let batch_start = Instant::now();

        for _ in 0..BATCH_SIZE {
            world.spawn_empty();
        }

        total_spawned += BATCH_SIZE;
        let batch_duration = batch_start.elapsed();

        if batch % 2 == 0 {
            println!(
                "  Batch {}: spawned {} entities in {:?}",
                batch + 1,
                BATCH_SIZE,
                batch_duration
            );
        }
    }

    let total_duration = start.elapsed();
    println!(
        "\nTotal: spawned {} entities in {:?}",
        total_spawned, total_duration
    );
    println!(
        "Average: {:?} per entity",
        total_duration / ENTITY_COUNT as u32
    );
    println!(
        "Rate: {:.0} entities/second",
        ENTITY_COUNT as f64 / total_duration.as_secs_f64()
    );

    // Verify all entities are alive
    println!("\nVerifying entities...");
    let start = Instant::now();
    let alive_count = world.len();
    let verify_duration = start.elapsed();
    println!(
        "  {} entities alive (verified in {:?})",
        alive_count, verify_duration
    );

    // Demonstrate iteration performance
    println!("\nIterating over all entities...");
    let start = Instant::now();
    let mut count = 0;
    for (_entity, _stable_id) in world.iter_entities() {
        count += 1;
    }
    let iter_duration = start.elapsed();
    println!("  Iterated {} entities in {:?}", count, iter_duration);
    println!(
        "  Rate: {:.0} entities/second",
        count as f64 / iter_duration.as_secs_f64()
    );

    // Demonstrate selective despawning
    println!("\nDespawning every 10th entity...");
    let entities_to_despawn: Vec<_> = world
        .iter_entities()
        .enumerate()
        .filter(|(i, _)| i % 10 == 0)
        .map(|(_, (entity, _))| entity)
        .collect();

    let start = Instant::now();
    for entity in entities_to_despawn {
        world.despawn(entity);
    }
    let despawn_duration = start.elapsed();
    println!(
        "  Despawned {} entities in {:?}",
        ENTITY_COUNT / 10,
        despawn_duration
    );
    println!("  Remaining: {} entities", world.len());

    // Demonstrate persistence at scale
    println!("\nTesting persistence with large world...");
    let start = Instant::now();
    let mut buffer = Vec::new();
    world.save_binary(&mut buffer).expect("Failed to save");
    let save_duration = start.elapsed();

    println!("  Saved {} entities to binary format", world.len());
    println!(
        "  Size: {} bytes ({:.2} KB)",
        buffer.len(),
        buffer.len() as f64 / 1024.0
    );
    println!("  Duration: {:?}", save_duration);
    println!(
        "  Rate: {:.0} entities/second",
        world.len() as f64 / save_duration.as_secs_f64()
    );

    // Memory usage tips
    println!("\n--- Memory Management Tips ---");
    println!("1. Pre-allocate capacity for known entity counts");
    println!("2. Use clear() to reset world without deallocating");
    println!("3. Batch operations reduce allocation overhead");
    println!("4. Entity IDs are 8 bytes, stable IDs are 16 bytes");
    println!("5. Empty entities have minimal memory overhead");

    // Performance summary
    println!("\n--- Performance Summary ---");
    println!(
        "Entity spawn rate: {:.0} entities/second",
        ENTITY_COUNT as f64 / total_duration.as_secs_f64()
    );
    println!(
        "Entity iteration rate: {:.0} entities/second",
        count as f64 / iter_duration.as_secs_f64()
    );
    println!(
        "Persistence rate: {:.0} entities/second",
        world.len() as f64 / save_duration.as_secs_f64()
    );

    println!("\n=== Example Complete ===");
}

// Made with Bob
