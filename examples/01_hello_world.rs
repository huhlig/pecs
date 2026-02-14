//! # Hello World Example
//!
//! This is the simplest possible PECS example. It demonstrates:
//! - Creating a World
//! - Spawning entities
//! - Using stable IDs for entity identification
//!
//! Run with: `cargo run --example 01_hello_world`

use pecs::prelude::*;

fn main() {
    println!("=== PECS Hello World ===\n");

    // Create a new world
    let mut world = World::new();
    println!("Created a new World");

    // Spawn some entities
    let entity1 = world.spawn_empty();
    println!("Spawned entity {:?}", entity1);

    let entity2 = world.spawn_empty();
    println!("Spawned entity {:?}", entity2);

    let entity3 = world.spawn_empty();
    println!("Spawned entity {:?}", entity3);

    // Check entity count
    println!("\nWorld now has {} entities", world.len());

    // Get stable IDs for entities (useful for persistence)
    if let Some(stable_id) = world.get_stable_id(entity1) {
        println!("Entity {:?} has stable ID: {}", entity1, stable_id);
    }

    // Check if entities are alive
    println!("\nChecking entity status:");
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

    // Despawn an entity
    world.despawn(entity2);
    println!("\nDespawned entity {:?}", entity2);
    println!(
        "  Entity {:?} is alive: {}",
        entity2,
        world.is_alive(entity2)
    );
    println!("World now has {} entities", world.len());

    // Iterate over all entities
    println!("\nIterating over all entities:");
    for (entity, stable_id) in world.iter_entities() {
        println!("  Entity {:?} -> Stable ID: {}", entity, stable_id);
    }

    println!("\n=== Example Complete ===");
}
