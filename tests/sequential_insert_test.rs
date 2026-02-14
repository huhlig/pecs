//! Tests for sequential insert() calls with full archetype transitions.
//!
//! This test file verifies that when multiple components are added to an entity
//! via sequential insert() calls, all components are properly copied during
//! archetype transitions.

use pecs::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq)]
struct Position {
    x: f32,
    y: f32,
}
impl Component for Position {}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Velocity {
    x: f32,
    y: f32,
}
impl Component for Velocity {}

#[derive(Debug, Clone, PartialEq)]
struct Name {
    value: String,
}
impl Component for Name {}

#[test]
fn sequential_insert_two_components() {
    let mut world = World::new();
    let entity = world.spawn_empty();

    // First insert
    assert!(world.insert(entity, Position { x: 1.0, y: 2.0 }));

    // Verify first component exists
    assert!(world.has::<Position>(entity));
    let pos = world.get::<Position>(entity).unwrap();
    assert_eq!(pos.x, 1.0);
    assert_eq!(pos.y, 2.0);

    // Second insert - this should preserve the first component
    assert!(world.insert(entity, Velocity { x: 3.0, y: 4.0 }));

    // Verify both components exist
    assert!(world.has::<Position>(entity));
    assert!(world.has::<Velocity>(entity));

    let pos = world.get::<Position>(entity).unwrap();
    assert_eq!(pos.x, 1.0);
    assert_eq!(pos.y, 2.0);

    let vel = world.get::<Velocity>(entity).unwrap();
    assert_eq!(vel.x, 3.0);
    assert_eq!(vel.y, 4.0);
}

#[test]
fn sequential_insert_three_components() {
    let mut world = World::new();
    let entity = world.spawn_empty();

    // Insert three components sequentially
    assert!(world.insert(entity, Position { x: 10.0, y: 20.0 }));
    assert!(world.insert(entity, Velocity { x: 1.0, y: 2.0 }));
    assert!(world.insert(
        entity,
        Name {
            value: "Test".to_string()
        }
    ));

    // Verify all three components exist
    assert!(world.has::<Position>(entity));
    assert!(world.has::<Velocity>(entity));
    assert!(world.has::<Name>(entity));

    let pos = world.get::<Position>(entity).unwrap();
    assert_eq!(pos.x, 10.0);
    assert_eq!(pos.y, 20.0);

    let vel = world.get::<Velocity>(entity).unwrap();
    assert_eq!(vel.x, 1.0);
    assert_eq!(vel.y, 2.0);

    let name = world.get::<Name>(entity).unwrap();
    assert_eq!(name.value, "Test");
}

#[test]
fn sequential_insert_with_command_buffer() {
    let mut world = World::new();
    let entity = world.spawn_empty();

    // Insert first component directly
    assert!(world.insert(entity, Position { x: 5.0, y: 10.0 }));

    // Insert second component via command buffer
    world.commands().insert(entity, Velocity { x: 2.0, y: 3.0 });
    world.apply_commands();

    // Verify both components exist
    assert!(world.has::<Position>(entity));
    assert!(world.has::<Velocity>(entity));

    let pos = world.get::<Position>(entity).unwrap();
    assert_eq!(pos.x, 5.0);
    assert_eq!(pos.y, 10.0);

    let vel = world.get::<Velocity>(entity).unwrap();
    assert_eq!(vel.x, 2.0);
    assert_eq!(vel.y, 3.0);
}

#[test]
fn sequential_insert_multiple_entities() {
    let mut world = World::new();

    let e1 = world.spawn_empty();
    let e2 = world.spawn_empty();
    let e3 = world.spawn_empty();

    // Add components to each entity sequentially
    world.insert(e1, Position { x: 1.0, y: 1.0 });
    world.insert(e2, Position { x: 2.0, y: 2.0 });
    world.insert(e3, Position { x: 3.0, y: 3.0 });

    world.insert(e1, Velocity { x: 0.1, y: 0.1 });
    world.insert(e2, Velocity { x: 0.2, y: 0.2 });
    world.insert(e3, Velocity { x: 0.3, y: 0.3 });

    // Verify all entities have both components
    for (i, entity) in [e1, e2, e3].iter().enumerate() {
        let expected_val = (i + 1) as f32;

        assert!(world.has::<Position>(*entity));
        assert!(world.has::<Velocity>(*entity));

        let pos = world.get::<Position>(*entity).unwrap();
        assert_eq!(pos.x, expected_val);
        assert_eq!(pos.y, expected_val);

        let vel = world.get::<Velocity>(*entity).unwrap();
        assert_eq!(vel.x, expected_val / 10.0);
        assert_eq!(vel.y, expected_val / 10.0);
    }
}

#[test]
fn sequential_insert_with_iteration() {
    let mut world = World::new();

    let mut entities = Vec::new();

    // Create entities with sequential inserts
    for i in 0..5 {
        let entity = world.spawn_empty();
        world.insert(
            entity,
            Position {
                x: i as f32,
                y: i as f32 * 2.0,
            },
        );
        world.insert(entity, Velocity { x: 1.0, y: 1.0 });
        entities.push(entity);
    }

    // Verify all entities have both components
    for entity in entities {
        assert!(world.has::<Position>(entity));
        assert!(world.has::<Velocity>(entity));

        let pos = world.get::<Position>(entity).unwrap();
        let vel = world.get::<Velocity>(entity).unwrap();

        assert_eq!(vel.x, 1.0);
        assert_eq!(vel.y, 1.0);
        assert_eq!(pos.y, pos.x * 2.0);
    }
}

#[test]
fn sequential_insert_replace_component() {
    let mut world = World::new();
    let entity = world.spawn_empty();

    // Insert initial components
    world.insert(entity, Position { x: 1.0, y: 2.0 });
    world.insert(entity, Velocity { x: 3.0, y: 4.0 });

    // Replace Position component
    world.insert(entity, Position { x: 10.0, y: 20.0 });

    // Verify Position was replaced and Velocity still exists
    assert!(world.has::<Position>(entity));
    assert!(world.has::<Velocity>(entity));

    let pos = world.get::<Position>(entity).unwrap();
    assert_eq!(pos.x, 10.0);
    assert_eq!(pos.y, 20.0);

    let vel = world.get::<Velocity>(entity).unwrap();
    assert_eq!(vel.x, 3.0);
    assert_eq!(vel.y, 4.0);
}
