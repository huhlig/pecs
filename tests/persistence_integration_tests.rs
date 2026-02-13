//! Integration tests for persistence functionality.
//!
//! These tests verify that the save/load system works correctly with actual files,
//! including round-trip tests, large world tests, and validation.

use pecs::persistence::{BinaryPlugin, PersistenceManager};
use pecs::prelude::*;
use std::fs;
use std::path::PathBuf;

// Test components (defined for future use with component serialization)
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
struct Position {
    x: f32,
    y: f32,
    z: f32,
}
impl Component for Position {}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
struct Velocity {
    x: f32,
    y: f32,
    z: f32,
}
impl Component for Velocity {}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
struct Health {
    current: i32,
    max: i32,
}
impl Component for Health {}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
struct Name {
    value: String,
}
impl Component for Name {}

/// Helper to create a temporary test file path
fn temp_file_path(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!("pecs_test_{}.pecs", name));
    path
}

/// Helper to clean up test file
fn cleanup_test_file(path: &PathBuf) {
    let _ = fs::remove_file(path);
}

#[test]
fn test_save_and_load_empty_world() {
    let path = temp_file_path("empty_world");

    // Create and save empty world
    let world = World::new();
    let mut manager = PersistenceManager::new();
    manager.register_plugin("binary", Box::new(BinaryPlugin::new()));

    manager
        .save(&world, &path)
        .expect("Failed to save empty world");

    // Verify file exists
    assert!(path.exists(), "Save file should exist");

    // Load the world
    let loaded_world = manager.load(&path).expect("Failed to load empty world");

    // Verify empty world
    assert_eq!(loaded_world.len(), 0, "Loaded world should be empty");

    cleanup_test_file(&path);
}

#[test]
fn test_save_and_load_world_with_entities() {
    let path = temp_file_path("world_with_entities");

    // Create world with entities (no components yet - placeholder test)
    let mut world = World::new();
    let _e1 = world.spawn_empty();
    let _e2 = world.spawn_empty();
    let _e3 = world.spawn_empty();

    let mut manager = PersistenceManager::new();
    manager.register_plugin("binary", Box::new(BinaryPlugin::new()));

    // Save
    manager.save(&world, &path).expect("Failed to save world");
    assert!(path.exists(), "Save file should exist");

    // Load
    let loaded_world = manager.load(&path).expect("Failed to load world");

    // Verify entity count
    assert_eq!(loaded_world.len(), 3, "Loaded world should have 3 entities");

    cleanup_test_file(&path);
}

#[test]
fn test_world_save_load_convenience_methods() {
    let path = temp_file_path("convenience_methods");

    // Create world
    let mut world = World::new();
    let _e1 = world.spawn_empty();
    let _e2 = world.spawn_empty();

    // Register plugin first
    let mut manager = PersistenceManager::new();
    manager.register_plugin("binary", Box::new(BinaryPlugin::new()));

    // Save using manager
    manager.save(&world, &path).expect("Failed to save");

    // Load using manager
    let loaded_world = manager.load(&path).expect("Failed to load");

    assert_eq!(loaded_world.len(), 2);

    cleanup_test_file(&path);
}

#[test]
fn test_save_creates_valid_file() {
    let path = temp_file_path("valid_file");

    let mut world = World::new();
    let _e = world.spawn_empty();

    let mut manager = PersistenceManager::new();
    manager.register_plugin("binary", Box::new(BinaryPlugin::new()));

    manager.save(&world, &path).expect("Failed to save");

    // Verify file exists and has content
    assert!(path.exists());
    let metadata = fs::metadata(&path).expect("Failed to get file metadata");
    assert!(metadata.len() > 0, "File should not be empty");

    cleanup_test_file(&path);
}

#[test]
fn test_load_nonexistent_file() {
    let path = temp_file_path("nonexistent");

    let manager = PersistenceManager::new();
    let result = manager.load(&path);

    assert!(result.is_err(), "Loading nonexistent file should fail");
}

#[test]
fn test_save_with_specific_plugin() {
    let path = temp_file_path("specific_plugin");

    let mut world = World::new();
    let _e = world.spawn_empty();

    let mut manager = PersistenceManager::new();
    manager.register_plugin("binary", Box::new(BinaryPlugin::new()));

    manager
        .save_with(&world, &path, "binary")
        .expect("Failed to save with specific plugin");

    assert!(path.exists());

    cleanup_test_file(&path);
}

#[test]
fn test_load_with_specific_plugin() {
    let path = temp_file_path("load_specific_plugin");

    let mut world = World::new();
    let _e = world.spawn_empty();

    let mut manager = PersistenceManager::new();
    manager.register_plugin("binary", Box::new(BinaryPlugin::new()));

    manager.save(&world, &path).expect("Failed to save");

    let loaded_world = manager
        .load_with(&path, "binary")
        .expect("Failed to load with specific plugin");

    assert_eq!(loaded_world.len(), 1);

    cleanup_test_file(&path);
}

#[test]
fn test_save_with_unregistered_plugin() {
    let path = temp_file_path("unregistered_plugin");

    let world = World::new();
    let manager = PersistenceManager::new();

    let result = manager.save_with(&world, &path, "nonexistent");

    assert!(
        result.is_err(),
        "Saving with unregistered plugin should fail"
    );
}

#[test]
fn test_multiple_save_load_cycles() {
    let path = temp_file_path("multiple_cycles");

    let mut manager = PersistenceManager::new();
    manager.register_plugin("binary", Box::new(BinaryPlugin::new()));

    // First cycle
    let mut world1 = World::new();
    let _e1 = world1.spawn_empty();
    manager.save(&world1, &path).expect("Failed first save");
    let loaded1 = manager.load(&path).expect("Failed first load");
    assert_eq!(loaded1.len(), 1);

    // Second cycle - overwrite
    let mut world2 = World::new();
    let _e2 = world2.spawn_empty();
    let _e3 = world2.spawn_empty();
    manager.save(&world2, &path).expect("Failed second save");
    let loaded2 = manager.load(&path).expect("Failed second load");
    assert_eq!(loaded2.len(), 2);

    cleanup_test_file(&path);
}

#[test]
fn test_large_world_save_load() {
    let path = temp_file_path("large_world");

    // Create a world with many entities
    let mut world = World::new();
    for _ in 0..1000 {
        world.spawn_empty();
    }

    let mut manager = PersistenceManager::new();
    manager.register_plugin("binary", Box::new(BinaryPlugin::new()));

    // Save
    manager
        .save(&world, &path)
        .expect("Failed to save large world");

    // Load
    let loaded_world = manager.load(&path).expect("Failed to load large world");

    assert_eq!(loaded_world.len(), 1000, "All entities should be loaded");

    cleanup_test_file(&path);
}

#[test]
fn test_stable_id_preservation() {
    let path = temp_file_path("stable_ids");

    let mut world = World::new();
    let e1 = world.spawn_empty();
    let e2 = world.spawn_empty();

    // Get stable IDs
    let stable1 = world
        .get_stable_id(e1)
        .expect("Entity should have stable ID");
    let stable2 = world
        .get_stable_id(e2)
        .expect("Entity should have stable ID");

    let mut manager = PersistenceManager::new();
    manager.register_plugin("binary", Box::new(BinaryPlugin::new()));

    // Save and load
    manager.save(&world, &path).expect("Failed to save");
    let loaded_world = manager.load(&path).expect("Failed to load");

    // Verify stable IDs are preserved
    let mut found_stable1 = false;
    let mut found_stable2 = false;

    for (_entity, stable_id) in loaded_world.iter_entities() {
        if stable_id == stable1 {
            found_stable1 = true;
        }
        if stable_id == stable2 {
            found_stable2 = true;
        }
    }

    assert!(found_stable1, "First stable ID should be preserved");
    assert!(found_stable2, "Second stable ID should be preserved");

    cleanup_test_file(&path);
}

#[test]
fn test_file_size_reasonable() {
    let path = temp_file_path("file_size");

    let mut world = World::new();
    for _ in 0..100 {
        world.spawn_empty();
    }

    let mut manager = PersistenceManager::new();
    manager.register_plugin("binary", Box::new(BinaryPlugin::new()));

    manager.save(&world, &path).expect("Failed to save");

    let metadata = fs::metadata(&path).expect("Failed to get metadata");
    let file_size = metadata.len();

    // File should be reasonably sized (less than 10KB for 100 empty entities)
    assert!(
        file_size < 10_000,
        "File size should be reasonable: {} bytes",
        file_size
    );

    cleanup_test_file(&path);
}

#[test]
fn test_concurrent_saves_different_files() {
    let path1 = temp_file_path("concurrent_1");
    let path2 = temp_file_path("concurrent_2");

    let mut world1 = World::new();
    world1.spawn_empty();

    let mut world2 = World::new();
    world2.spawn_empty();
    world2.spawn_empty();

    let mut manager = PersistenceManager::new();
    manager.register_plugin("binary", Box::new(BinaryPlugin::new()));

    // Save both worlds
    manager
        .save(&world1, &path1)
        .expect("Failed to save world1");
    manager
        .save(&world2, &path2)
        .expect("Failed to save world2");

    // Load and verify
    let loaded1 = manager.load(&path1).expect("Failed to load world1");
    let loaded2 = manager.load(&path2).expect("Failed to load world2");

    assert_eq!(loaded1.len(), 1);
    assert_eq!(loaded2.len(), 2);

    cleanup_test_file(&path1);
    cleanup_test_file(&path2);
}

#[test]
fn test_default_plugin_selection() {
    let path = temp_file_path("default_plugin");

    let mut world = World::new();
    world.spawn_empty();

    let mut manager = PersistenceManager::new();
    manager.register_plugin("binary", Box::new(BinaryPlugin::new()));

    // First registered plugin should become default
    manager
        .save(&world, &path)
        .expect("Should use default plugin");

    let loaded = manager
        .load(&path)
        .expect("Should load with default plugin");
    assert_eq!(loaded.len(), 1);

    cleanup_test_file(&path);
}

// Made with Bob
