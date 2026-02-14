//! # Persistence Example
//!
//! This example demonstrates saving and loading worlds using PECS persistence system.
//! It shows:
//! - Saving a world to binary format
//! - Loading a world from binary format
//! - Stable ID preservation across save/load
//! - JSON format for human-readable saves
//!
//! Run with: `cargo run --example 03_persistence`

use pecs::prelude::*;
use std::io::Cursor;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== PECS Persistence Example ===\n");

    // Create a world with some entities
    let mut world = World::new();

    println!("Creating entities...");
    let entity1 = world.spawn_empty();
    let entity2 = world.spawn_empty();
    let entity3 = world.spawn_empty();

    let stable_id1 = world.get_stable_id(entity1).unwrap();
    let stable_id2 = world.get_stable_id(entity2).unwrap();
    let stable_id3 = world.get_stable_id(entity3).unwrap();

    println!("Created {} entities", world.len());
    println!("  Entity {:?} -> {}", entity1, stable_id1);
    println!("  Entity {:?} -> {}", entity2, stable_id2);
    println!("  Entity {:?} -> {}", entity3, stable_id3);

    // Save to binary format (in-memory)
    println!("\n--- Binary Format ---");
    let mut binary_buffer = Vec::new();
    world.save_binary(&mut binary_buffer)?;
    println!(
        "Saved world to binary format ({} bytes)",
        binary_buffer.len()
    );

    // Load from binary format
    let mut cursor = Cursor::new(&binary_buffer);
    let loaded_world = World::load_binary(&mut cursor)?;
    println!("Loaded world from binary format");
    println!("Loaded world has {} entities", loaded_world.len());

    // Verify stable IDs are preserved
    println!("\nVerifying stable IDs are preserved:");
    let loaded_entity1 = loaded_world.get_entity_id(stable_id1);
    let loaded_entity2 = loaded_world.get_entity_id(stable_id2);
    let loaded_entity3 = loaded_world.get_entity_id(stable_id3);

    println!("  Stable ID {} -> {:?}", stable_id1, loaded_entity1);
    println!("  Stable ID {} -> {:?}", stable_id2, loaded_entity2);
    println!("  Stable ID {} -> {:?}", stable_id3, loaded_entity3);

    // Save to JSON format (human-readable)
    println!("\n--- JSON Format ---");
    let mut json_buffer = Vec::new();
    world.save_json(&mut json_buffer)?;
    println!("Saved world to JSON format ({} bytes)", json_buffer.len());

    // Show JSON content (first 200 chars)
    let json_str = String::from_utf8_lossy(&json_buffer);
    let preview = if json_str.len() > 200 {
        format!("{}...", &json_str[..200])
    } else {
        json_str.to_string()
    };
    println!("JSON preview:\n{}", preview);

    // Load from JSON format
    let mut json_cursor = Cursor::new(&json_buffer);
    let json_loaded_world = World::load_json(&mut json_cursor)?;
    println!("\nLoaded world from JSON format");
    println!("Loaded world has {} entities", json_loaded_world.len());

    // Demonstrate file-based persistence
    println!("\n--- File-Based Persistence ---");

    // Note: In a real application, you would save to actual files:
    // world.save("world.pecs")?;
    // let loaded = World::load("world.pecs")?;

    println!("To save to a file, use:");
    println!("  world.save(\"world.pecs\")?;");
    println!("To load from a file, use:");
    println!("  let world = World::load(\"world.pecs\")?;");

    println!("\n=== Example Complete ===");
    Ok(())
}

// Made with Bob
