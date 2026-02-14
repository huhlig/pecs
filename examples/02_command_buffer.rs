//! # Command Buffer Example
//!
//! This example demonstrates the command buffer system for deferred operations.
//! Command buffers are useful for:
//! - Recording operations from multiple threads
//! - Batching operations for better performance
//! - Deferring structural changes until a safe point
//!
//! Run with: `cargo run --example 02_command_buffer`

use pecs::prelude::*;

fn main() {
    println!("=== PECS Command Buffer Example ===\n");

    let mut world = World::new();

    // Spawn some initial entities directly
    let entity1 = world.spawn_empty();
    let entity2 = world.spawn_empty();
    println!("Spawned 2 entities directly");
    println!("World has {} entities\n", world.len());

    // Record commands in the command buffer
    println!("Recording commands in buffer...");
    world.commands().spawn();
    world.commands().spawn();
    world.commands().spawn();
    world.commands().despawn(entity1);
    println!("Recorded 3 spawn commands and 1 despawn command");

    // Commands haven't been applied yet
    println!(
        "World still has {} entities (commands not applied yet)\n",
        world.len()
    );

    // Apply all pending commands
    println!("Applying commands...");
    world.apply_commands();
    println!("World now has {} entities\n", world.len());

    // Verify entity1 was despawned
    println!("Checking entity status:");
    println!(
        "  Entity {:?} is alive: {}",
        entity1,
        world.is_alive(entity1)
    );
    println!(
        "  Entity {:?} is alive: {}",
        entity2,
        world.is_alive(entity2)
    );

    // Demonstrate batching multiple operations
    println!("\nBatching multiple operations:");
    for i in 0..5 {
        world.commands().spawn();
        if i % 2 == 0 {
            println!("  Queued spawn command {}", i + 1);
        }
    }

    println!("Before apply: {} entities", world.len());
    world.apply_commands();
    println!("After apply: {} entities", world.len());

    // Show all entities
    println!("\nAll entities in world:");
    for (entity, stable_id) in world.iter_entities() {
        println!("  Entity {:?} -> {}", entity, stable_id);
    }

    println!("\n=== Example Complete ===");
}

// Made with Bob
