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

//! Integration tests for entity-specific persistence.

use pecs::persistence::{EntityPersistencePlugin, KeyValueEntityPlugin, PersistenceManager};
use pecs::prelude::*;

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
struct Position {
    x: f32,
    y: f32,
}

impl Component for Position {}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
struct Velocity {
    x: f32,
    y: f32,
}

impl Component for Velocity {}

#[test]
fn test_entity_plugin_registration() {
    let mut manager = PersistenceManager::new();
    let plugin = Box::new(KeyValueEntityPlugin::new());

    manager.register_entity_plugin("kv", plugin);

    assert_eq!(manager.list_entity_plugins(), vec!["kv"]);
    assert_eq!(manager.default_entity_plugin(), Some("kv"));
}

#[test]
fn test_multiple_entity_plugins() {
    let mut manager = PersistenceManager::new();

    manager.register_entity_plugin("kv1", Box::new(KeyValueEntityPlugin::new()));
    manager.register_entity_plugin("kv2", Box::new(KeyValueEntityPlugin::new()));

    let plugins = manager.list_entity_plugins();
    assert_eq!(plugins.len(), 2);
    assert!(plugins.contains(&"kv1"));
    assert!(plugins.contains(&"kv2"));
}

#[test]
fn test_set_default_entity_plugin() {
    let mut manager = PersistenceManager::new();

    manager.register_entity_plugin("kv1", Box::new(KeyValueEntityPlugin::new()));
    manager.register_entity_plugin("kv2", Box::new(KeyValueEntityPlugin::new()));

    assert_eq!(manager.default_entity_plugin(), Some("kv1"));

    manager.set_default_entity_plugin("kv2").unwrap();
    assert_eq!(manager.default_entity_plugin(), Some("kv2"));
}

#[test]
fn test_set_nonexistent_default_plugin() {
    let mut manager = PersistenceManager::new();

    let result = manager.set_default_entity_plugin("nonexistent");
    assert!(result.is_err());
}

#[test]
fn test_entity_exists_check() {
    let plugin = KeyValueEntityPlugin::new();
    let stable_id = StableId::new();

    // Entity should not exist initially
    assert!(!plugin.entity_exists(stable_id).unwrap());
}

#[test]
fn test_delete_nonexistent_entity() {
    let plugin = KeyValueEntityPlugin::new();
    let stable_id = StableId::new();

    // Deleting a nonexistent entity should succeed (idempotent)
    assert!(plugin.delete_entity(stable_id).is_ok());
}

#[test]
fn test_plugin_clear() {
    let plugin = KeyValueEntityPlugin::new();

    // Add some entities manually for testing
    let id1 = StableId::new();
    let id2 = StableId::new();

    plugin
        .storage
        .write()
        .unwrap()
        .insert(id1, pecs::persistence::EntityData::new(id1, Vec::new(), 0));
    plugin
        .storage
        .write()
        .unwrap()
        .insert(id2, pecs::persistence::EntityData::new(id2, Vec::new(), 0));

    assert_eq!(plugin.len(), 2);

    plugin.clear();
    assert_eq!(plugin.len(), 0);
    assert!(plugin.is_empty());
}

#[test]
fn test_plugin_list_entities() {
    let plugin = KeyValueEntityPlugin::new();

    let id1 = StableId::new();
    let id2 = StableId::new();
    let id3 = StableId::new();

    plugin
        .storage
        .write()
        .unwrap()
        .insert(id1, pecs::persistence::EntityData::new(id1, Vec::new(), 0));
    plugin
        .storage
        .write()
        .unwrap()
        .insert(id2, pecs::persistence::EntityData::new(id2, Vec::new(), 0));
    plugin
        .storage
        .write()
        .unwrap()
        .insert(id3, pecs::persistence::EntityData::new(id3, Vec::new(), 0));

    let entities = plugin.list_entities();
    assert_eq!(entities.len(), 3);
    assert!(entities.contains(&id1));
    assert!(entities.contains(&id2));
    assert!(entities.contains(&id3));
}

#[test]
fn test_backend_name_and_version() {
    let plugin = KeyValueEntityPlugin::new();

    assert_eq!(plugin.backend_name(), "key_value_memory");
    assert_eq!(plugin.backend_version(), 1);
}

#[test]
fn test_save_entity_basic() {
    let plugin = KeyValueEntityPlugin::new();
    let mut world = World::new();

    // Spawn an entity
    let entity = world.spawn_empty();
    let stable_id = world.get_stable_id(entity).unwrap();

    // Save it
    let result = plugin.save_entity(&world, entity);
    assert!(result.is_ok());

    // Verify it exists in storage
    assert!(plugin.entity_exists(stable_id).unwrap());
}

#[test]
fn test_save_nonexistent_entity() {
    let plugin = KeyValueEntityPlugin::new();
    let world = World::new();

    // Try to save a nonexistent entity
    let fake_entity = EntityId::new(999, 1);
    let result = plugin.save_entity(&world, fake_entity);

    // Should fail because entity doesn't exist
    assert!(result.is_err());
}

#[test]
fn test_load_entity_creates_new() {
    let plugin = KeyValueEntityPlugin::new();
    let mut world = World::new();

    // Create entity data manually
    let stable_id = StableId::new();
    plugin.storage.write().unwrap().insert(
        stable_id,
        pecs::persistence::EntityData::new(stable_id, Vec::new(), 0),
    );

    // Load it
    let result = plugin.load_entity(&mut world, stable_id);
    assert!(result.is_ok());

    let entity_id = result.unwrap();
    assert!(world.is_alive(entity_id));
    assert_eq!(world.get_stable_id(entity_id), Some(stable_id));
}

#[test]
fn test_load_entity_updates_existing() {
    let plugin = KeyValueEntityPlugin::new();
    let mut world = World::new();

    // Create an entity with a specific stable ID
    let stable_id = StableId::new();
    let entity = world.spawn_empty_with_stable_id(stable_id).unwrap();

    // Store entity data
    plugin.storage.write().unwrap().insert(
        stable_id,
        pecs::persistence::EntityData::new(stable_id, Vec::new(), 0),
    );

    // Load it (should update existing entity)
    let result = plugin.load_entity(&mut world, stable_id);
    assert!(result.is_ok());

    let loaded_entity = result.unwrap();
    assert_eq!(loaded_entity, entity);
}

#[test]
fn test_load_nonexistent_entity() {
    let plugin = KeyValueEntityPlugin::new();
    let mut world = World::new();

    let stable_id = StableId::new();

    // Try to load entity that doesn't exist in storage
    let result = plugin.load_entity(&mut world, stable_id);
    assert!(result.is_err());
}

#[test]
fn test_save_multiple_entities() {
    let plugin = KeyValueEntityPlugin::new();
    let mut world = World::new();

    // Spawn multiple entities
    let entity1 = world.spawn_empty();
    let entity2 = world.spawn_empty();
    let entity3 = world.spawn_empty();

    let stable_id1 = world.get_stable_id(entity1).unwrap();
    let stable_id2 = world.get_stable_id(entity2).unwrap();
    let stable_id3 = world.get_stable_id(entity3).unwrap();

    // Save them all at once
    let result = plugin.save_entities(&world, &[entity1, entity2, entity3]);
    assert!(result.is_ok());

    // Verify all exist
    assert!(plugin.entity_exists(stable_id1).unwrap());
    assert!(plugin.entity_exists(stable_id2).unwrap());
    assert!(plugin.entity_exists(stable_id3).unwrap());
}

#[test]
fn test_load_multiple_entities() {
    let plugin = KeyValueEntityPlugin::new();
    let mut world = World::new();

    // Create entity data manually
    let id1 = StableId::new();
    let id2 = StableId::new();
    let id3 = StableId::new();

    plugin
        .storage
        .write()
        .unwrap()
        .insert(id1, pecs::persistence::EntityData::new(id1, Vec::new(), 0));
    plugin
        .storage
        .write()
        .unwrap()
        .insert(id2, pecs::persistence::EntityData::new(id2, Vec::new(), 0));
    plugin
        .storage
        .write()
        .unwrap()
        .insert(id3, pecs::persistence::EntityData::new(id3, Vec::new(), 0));

    // Load them all at once
    let result = plugin.load_entities(&mut world, &[id1, id2, id3]);
    assert!(result.is_ok());

    let entity_ids = result.unwrap();
    assert_eq!(entity_ids.len(), 3);

    // Verify all are alive
    for entity_id in entity_ids {
        assert!(world.is_alive(entity_id));
    }
}

#[test]
fn test_manager_save_entity() {
    let mut manager = PersistenceManager::new();
    manager.register_entity_plugin("kv", Box::new(KeyValueEntityPlugin::new()));

    let mut world = World::new();
    let entity = world.spawn_empty();

    // Save using manager
    let result = manager.save_entity(&world, entity);
    assert!(result.is_ok());
}

#[test]
fn test_manager_save_entity_with_plugin() {
    let mut manager = PersistenceManager::new();
    manager.register_entity_plugin("kv1", Box::new(KeyValueEntityPlugin::new()));
    manager.register_entity_plugin("kv2", Box::new(KeyValueEntityPlugin::new()));

    let mut world = World::new();
    let entity = world.spawn_empty();

    // Save using specific plugin
    let result = manager.save_entity_with(&world, entity, "kv2");
    assert!(result.is_ok());
}

#[test]
fn test_manager_save_without_plugin() {
    let manager = PersistenceManager::new();
    let mut world = World::new();
    let entity = world.spawn_empty();

    // Try to save without any registered plugin
    let result = manager.save_entity(&world, entity);
    assert!(result.is_err());
}

#[test]
fn test_manager_load_entity() {
    let mut manager = PersistenceManager::new();
    let plugin = KeyValueEntityPlugin::new();

    // Manually add entity data
    let stable_id = StableId::new();
    plugin.storage.write().unwrap().insert(
        stable_id,
        pecs::persistence::EntityData::new(stable_id, Vec::new(), 0),
    );

    manager.register_entity_plugin("kv", Box::new(plugin));

    let mut world = World::new();

    // Load using manager
    let result = manager.load_entity(&mut world, stable_id);
    assert!(result.is_ok());
}

#[test]
fn test_manager_delete_entity() {
    let mut manager = PersistenceManager::new();
    let plugin = KeyValueEntityPlugin::new();

    let stable_id = StableId::new();
    plugin.storage.write().unwrap().insert(
        stable_id,
        pecs::persistence::EntityData::new(stable_id, Vec::new(), 0),
    );

    manager.register_entity_plugin("kv", Box::new(plugin));

    // Delete using manager
    let result = manager.delete_entity(stable_id);
    assert!(result.is_ok());

    // Verify it's gone
    assert!(!manager.entity_exists(stable_id).unwrap());
}

#[test]
fn test_manager_entity_exists() {
    let mut manager = PersistenceManager::new();
    let plugin = KeyValueEntityPlugin::new();

    let stable_id = StableId::new();
    plugin.storage.write().unwrap().insert(
        stable_id,
        pecs::persistence::EntityData::new(stable_id, Vec::new(), 0),
    );

    manager.register_entity_plugin("kv", Box::new(plugin));

    // Check existence using manager
    assert!(manager.entity_exists(stable_id).unwrap());

    let nonexistent_id = StableId::new();
    assert!(!manager.entity_exists(nonexistent_id).unwrap());
}

#[test]
fn test_entity_data_creation() {
    let stable_id = StableId::new();
    let components = Vec::new();
    let timestamp = 12345;

    let entity_data = pecs::persistence::EntityData::new(stable_id, components, timestamp);

    assert_eq!(entity_data.stable_id, stable_id);
    assert_eq!(entity_data.timestamp, timestamp);
    assert!(entity_data.components.is_empty());
}

#[test]
fn test_entity_data_current_timestamp() {
    let timestamp1 = pecs::persistence::EntityData::current_timestamp();
    std::thread::sleep(std::time::Duration::from_millis(10));
    let timestamp2 = pecs::persistence::EntityData::current_timestamp();

    assert!(timestamp2 >= timestamp1);
}

#[test]
fn test_world_get_entity_by_stable_id() {
    let mut world = World::new();
    let entity = world.spawn_empty();
    let stable_id = world.get_stable_id(entity).unwrap();

    // Test the alias method
    assert_eq!(world.get_entity_by_stable_id(stable_id), Some(entity));

    let nonexistent_id = StableId::new();
    assert_eq!(world.get_entity_by_stable_id(nonexistent_id), None);
}

// Made with Bob
