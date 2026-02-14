//
// Copyright 2026 Hans W. Uhlig. All Rights Reserved.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//

//! Entity-Specific Persistence Example
//!
//! This example demonstrates how to use entity-specific persistence to save
//! and load individual entities rather than entire worlds. This is useful for:
//! - Database backends
//! - Lazy loading
//! - Multiplayer synchronization
//! - Persistent world chunks

use pecs::persistence::{KeyValueEntityPlugin, PersistenceManager};
use pecs::prelude::*;

// Define some example components
#[derive(Debug, Clone, PartialEq)]
struct Player {
    name: String,
    level: u32,
    health: f32,
}

impl Component for Player {}

#[derive(Debug, Clone, PartialEq)]
struct Position {
    x: f32,
    y: f32,
    z: f32,
}

impl Component for Position {}

#[derive(Debug, Clone, PartialEq)]
struct Inventory {
    items: Vec<String>,
    gold: u32,
}

impl Component for Inventory {}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Entity-Specific Persistence Example ===\n");

    // Example 1: Basic Save and Load
    basic_save_load()?;

    // Example 2: Multiple Entities
    multiple_entities()?;

    // Example 3: Batch Operations
    batch_operations()?;

    // Example 4: Entity Existence Checks
    entity_checks()?;

    // Example 5: Simulated Database Backend
    simulated_database()?;

    Ok(())
}

/// Example 1: Basic entity save and load
fn basic_save_load() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Example 1: Basic Save and Load ---");

    // Create a world and persistence manager
    let mut world = World::new();
    let mut manager = PersistenceManager::new();

    // Register the key-value entity plugin
    manager.register_entity_plugin("kv", Box::new(KeyValueEntityPlugin::new()));

    // Spawn a player entity
    let player_entity = world
        .spawn()
        .with(Player {
            name: "Alice".to_string(),
            level: 10,
            health: 100.0,
        })
        .with(Position {
            x: 10.0,
            y: 20.0,
            z: 0.0,
        })
        .id();

    println!("Created player entity: {}", player_entity);

    // Get the stable ID for persistence
    let stable_id = world.get_stable_id(player_entity).unwrap();
    println!("Stable ID: {}", stable_id);

    // Save the entity
    manager.save_entity(&world, player_entity)?;
    println!("✓ Entity saved");

    // Create a new world and load the entity
    let mut new_world = World::new();
    let loaded_entity = manager.load_entity(&mut new_world, stable_id)?;
    println!("✓ Entity loaded: {}", loaded_entity);
    println!();

    Ok(())
}

/// Example 2: Working with multiple entities
fn multiple_entities() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Example 2: Multiple Entities ---");

    let mut world = World::new();
    let mut manager = PersistenceManager::new();
    manager.register_entity_plugin("kv", Box::new(KeyValueEntityPlugin::new()));

    // Create multiple player entities
    let players = vec![
        ("Alice", 10, 100.0),
        ("Bob", 15, 85.0),
        ("Charlie", 8, 120.0),
    ];

    let mut stable_ids = Vec::new();

    for (name, level, health) in players {
        let entity = world
            .spawn()
            .with(Player {
                name: name.to_string(),
                level,
                health,
            })
            .id();

        let stable_id = world.get_stable_id(entity).unwrap();
        stable_ids.push(stable_id);

        manager.save_entity(&world, entity)?;
        println!("✓ Saved player: {} (level {})", name, level);
    }

    println!("\nTotal entities saved: {}", stable_ids.len());
    println!();

    Ok(())
}

/// Example 3: Batch operations for better performance
fn batch_operations() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Example 3: Batch Operations ---");

    let mut world = World::new();
    let mut manager = PersistenceManager::new();
    let plugin = KeyValueEntityPlugin::new();
    manager.register_entity_plugin("kv", Box::new(plugin));

    // Create multiple entities
    let mut entities = Vec::new();
    for i in 0..5 {
        let entity = world
            .spawn()
            .with(Player {
                name: format!("Player{}", i),
                level: i * 5,
                health: 100.0,
            })
            .with(Position {
                x: i as f32 * 10.0,
                y: 0.0,
                z: 0.0,
            })
            .id();
        entities.push(entity);
    }

    // Save all entities in a single batch operation
    println!("Saving {} entities in batch...", entities.len());
    manager.save_entity_with(&world, entities[0], "kv")?;
    for entity in &entities[1..] {
        manager.save_entity_with(&world, *entity, "kv")?;
    }
    println!("✓ Batch save complete");

    // Collect stable IDs for batch load
    let stable_ids: Vec<_> = entities
        .iter()
        .map(|&e| world.get_stable_id(e).unwrap())
        .collect();

    // Load all entities in a batch
    let mut new_world = World::new();
    println!("Loading {} entities in batch...", stable_ids.len());
    for stable_id in &stable_ids {
        manager.load_entity_with(&mut new_world, *stable_id, "kv")?;
    }
    println!("✓ Batch load complete");
    println!();

    Ok(())
}

/// Example 4: Entity existence checks
fn entity_checks() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Example 4: Entity Existence Checks ---");

    let mut world = World::new();
    let mut manager = PersistenceManager::new();
    manager.register_entity_plugin("kv", Box::new(KeyValueEntityPlugin::new()));

    // Create and save an entity
    let entity = world
        .spawn()
        .with(Player {
            name: "TestPlayer".to_string(),
            level: 1,
            health: 100.0,
        })
        .id();

    let stable_id = world.get_stable_id(entity).unwrap();
    manager.save_entity(&world, entity)?;

    // Check if entity exists
    if manager.entity_exists(stable_id)? {
        println!("✓ Entity exists in storage");
    }

    // Check non-existent entity
    let fake_id = StableId::new();
    if !manager.entity_exists(fake_id)? {
        println!("✓ Non-existent entity correctly reported as missing");
    }

    // Delete entity
    manager.delete_entity(stable_id)?;
    println!("✓ Entity deleted");

    // Verify deletion
    if !manager.entity_exists(stable_id)? {
        println!("✓ Deleted entity no longer exists");
    }
    println!();

    Ok(())
}

/// Example 5: Simulated database-like operations
fn simulated_database() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Example 5: Simulated Database Operations ---");

    let mut world = World::new();
    let mut manager = PersistenceManager::new();
    let plugin = KeyValueEntityPlugin::new();
    manager.register_entity_plugin("db", Box::new(plugin));

    // Simulate a game session
    println!("Starting game session...");

    // Create player
    let player = world
        .spawn()
        .with(Player {
            name: "Hero".to_string(),
            level: 1,
            health: 100.0,
        })
        .with(Position {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        })
        .with(Inventory {
            items: vec!["Sword".to_string(), "Shield".to_string()],
            gold: 100,
        })
        .id();

    let player_id = world.get_stable_id(player).unwrap();
    println!("✓ Player created: {}", player_id);

    // Auto-save after creation
    manager.save_entity_with(&world, player, "db")?;
    println!("✓ Auto-saved player state");

    // Simulate gameplay - player gains experience
    println!("\nSimulating gameplay...");
    // In a real game, you would modify components here
    println!("  - Player defeats enemies");
    println!("  - Player gains gold");
    println!("  - Player levels up");

    // Save updated state
    manager.save_entity_with(&world, player, "db")?;
    println!("✓ Saved updated player state");

    // Simulate loading in a new session
    println!("\nLoading saved game...");
    let mut new_world = World::new();
    let loaded_player = manager.load_entity_with(&mut new_world, player_id, "db")?;
    println!("✓ Player loaded: {}", loaded_player);
    println!("✓ Game state restored");

    println!();
    Ok(())
}

// Made with Bob
