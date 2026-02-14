//! Integration tests for the query system.
//!
//! These tests verify that the query system works correctly with real-world usage patterns.

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

#[derive(Debug, Clone, Copy, PartialEq)]
struct Health {
    current: i32,
    max: i32,
}
impl Component for Health {}

#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
struct Dead;
impl Component for Dead {}

#[test]
fn query_single_component_immutable() {
    let mut world = World::new();

    // Spawn entities with Position
    let _e1 = world.spawn().with(Position { x: 1.0, y: 2.0 }).id();
    let _e2 = world.spawn().with(Position { x: 3.0, y: 4.0 }).id();
    let _e3 = world.spawn().with(Velocity { x: 1.0, y: 0.0 }).id(); // No Position

    // Query for Position
    let mut count = 0;
    let mut positions = Vec::new();
    for pos in world.query::<&Position>() {
        count += 1;
        positions.push(*pos);
    }

    assert_eq!(count, 2);
    assert!(positions.contains(&Position { x: 1.0, y: 2.0 }));
    assert!(positions.contains(&Position { x: 3.0, y: 4.0 }));
}

#[test]
fn query_single_component_mutable() {
    let mut world = World::new();

    // Spawn entities with Position
    world.spawn().with(Position { x: 1.0, y: 2.0 }).id();
    world.spawn().with(Position { x: 3.0, y: 4.0 }).id();

    // Mutate all positions
    for pos in world.query::<&mut Position>() {
        pos.x += 10.0;
        pos.y += 20.0;
    }

    // Verify mutations
    let mut count = 0;
    for pos in world.query::<&Position>() {
        count += 1;
        assert!(pos.x >= 11.0);
        assert!(pos.y >= 22.0);
    }
    assert_eq!(count, 2);
}

#[test]
fn query_two_components() {
    let mut world = World::new();

    // Spawn entities with different component combinations
    let _e1 = world
        .spawn()
        .with(Position { x: 1.0, y: 2.0 })
        .with(Velocity { x: 0.5, y: 0.0 })
        .id();

    let _e2 = world
        .spawn()
        .with(Position { x: 3.0, y: 4.0 })
        .with(Velocity { x: -0.5, y: 1.0 })
        .id();

    let _e3 = world.spawn().with(Position { x: 5.0, y: 6.0 }).id(); // No Velocity

    // Query for entities with both Position and Velocity
    let mut count = 0;
    for (pos, vel) in world.query::<(&Position, &Velocity)>() {
        count += 1;
        assert!(pos.x > 0.0);
        assert!(vel.x != 0.0 || vel.y != 0.0);
    }

    assert_eq!(count, 2, "Should find 2 entities with both components");
}

#[test]
fn query_three_components() {
    let mut world = World::new();

    // Spawn entities with different component combinations
    let _e1 = world
        .spawn()
        .with(Position { x: 1.0, y: 2.0 })
        .with(Velocity { x: 0.5, y: 0.0 })
        .with(Health {
            current: 100,
            max: 100,
        })
        .id();

    let _e2 = world
        .spawn()
        .with(Position { x: 3.0, y: 4.0 })
        .with(Velocity { x: -0.5, y: 1.0 })
        .id(); // No Health

    // Query for entities with all three components
    let mut count = 0;
    for (_pos, _vel, health) in world.query::<(&Position, &Velocity, &Health)>() {
        count += 1;
        assert_eq!(health.current, 100);
        assert_eq!(health.max, 100);
    }

    assert_eq!(count, 1, "Should find 1 entity with all three components");
}

#[test]
fn query_with_entity_id() {
    let mut world = World::new();

    let e1 = world.spawn().with(Position { x: 1.0, y: 2.0 }).id();
    let e2 = world.spawn().with(Position { x: 3.0, y: 4.0 }).id();

    // Query including EntityId
    let mut entities = Vec::new();
    for (entity, _pos) in world.query::<(EntityId, &Position)>() {
        entities.push(entity);
    }

    assert_eq!(entities.len(), 2);
    assert!(entities.contains(&e1));
    assert!(entities.contains(&e2));
}

#[test]
fn query_mixed_mutability() {
    let mut world = World::new();

    world
        .spawn()
        .with(Position { x: 0.0, y: 0.0 })
        .with(Velocity { x: 1.0, y: 2.0 })
        .id();

    world
        .spawn()
        .with(Position { x: 10.0, y: 10.0 })
        .with(Velocity { x: -1.0, y: -2.0 })
        .id();

    // Update positions based on velocities
    for (pos, vel) in world.query::<(&mut Position, &Velocity)>() {
        pos.x += vel.x;
        pos.y += vel.y;
    }

    // Verify updates
    let mut count = 0;
    for pos in world.query::<&Position>() {
        count += 1;
        // First entity should be at (1, 2)
        // Second entity should be at (9, 8)
        assert!(pos.x == 1.0 || pos.x == 9.0);
    }
    assert_eq!(count, 2);
}

#[test]
fn query_empty_world() {
    let mut world = World::new();

    let mut count = 0;
    for _pos in world.query::<&Position>() {
        count += 1;
    }

    assert_eq!(count, 0);
}

#[test]
fn query_no_matching_entities() {
    let mut world = World::new();

    // Spawn entities without Position
    world.spawn().with(Velocity { x: 1.0, y: 0.0 }).id();
    world
        .spawn()
        .with(Health {
            current: 50,
            max: 100,
        })
        .id();

    let mut count = 0;
    for _pos in world.query::<&Position>() {
        count += 1;
    }

    assert_eq!(count, 0);
}

#[test]
fn query_after_component_removal() {
    let mut world = World::new();

    let e1 = world
        .spawn()
        .with(Position { x: 1.0, y: 2.0 })
        .with(Velocity { x: 0.5, y: 0.0 })
        .id();

    let _e2 = world
        .spawn()
        .with(Position { x: 3.0, y: 4.0 })
        .with(Velocity { x: -0.5, y: 1.0 })
        .id();

    // Remove Velocity from e1
    world.remove::<Velocity>(e1);

    // Query should only find e2
    let mut count = 0;
    for (_pos, _vel) in world.query::<(&Position, &Velocity)>() {
        count += 1;
    }

    assert_eq!(count, 1);
}

#[test]
fn query_large_number_of_entities() {
    let mut world = World::new();

    // Spawn 1000 entities
    for i in 0..1000 {
        world
            .spawn()
            .with(Position {
                x: i as f32,
                y: i as f32 * 2.0,
            })
            .id();
    }

    // Query all entities
    let mut count = 0;
    for _pos in world.query::<&Position>() {
        count += 1;
    }

    assert_eq!(count, 1000);
}

#[test]
fn query_multiple_archetypes() {
    let mut world = World::new();

    // Create entities with different component combinations (different archetypes)
    world.spawn().with(Position { x: 1.0, y: 1.0 }).id();
    world
        .spawn()
        .with(Position { x: 2.0, y: 2.0 })
        .with(Velocity { x: 1.0, y: 0.0 })
        .id();
    world
        .spawn()
        .with(Position { x: 3.0, y: 3.0 })
        .with(Health {
            current: 100,
            max: 100,
        })
        .id();
    world
        .spawn()
        .with(Position { x: 4.0, y: 4.0 })
        .with(Velocity { x: 0.0, y: 1.0 })
        .with(Health {
            current: 50,
            max: 100,
        })
        .id();

    // Query for Position (should find all 4)
    let mut count = 0;
    for _pos in world.query::<&Position>() {
        count += 1;
    }
    assert_eq!(count, 4);

    // Query for Position + Velocity (should find 2)
    let mut count = 0;
    for (_pos, _vel) in world.query::<(&Position, &Velocity)>() {
        count += 1;
    }
    assert_eq!(count, 2);

    // Query for all three (should find 1)
    let mut count = 0;
    for (_pos, _vel, _health) in world.query::<(&Position, &Velocity, &Health)>() {
        count += 1;
    }
    assert_eq!(count, 1);
}

#[test]
fn query_optional_component() {
    let mut world = World::new();

    world.spawn().with(Position { x: 1.0, y: 1.0 }).id();
    world
        .spawn()
        .with(Position { x: 2.0, y: 2.0 })
        .with(Velocity { x: 1.0, y: 0.0 })
        .id();

    // Query with optional Velocity
    let mut count_with_vel = 0;
    let mut count_without_vel = 0;

    for (_pos, vel_opt) in world.query::<(&Position, Option<&Velocity>)>() {
        if vel_opt.is_some() {
            count_with_vel += 1;
        } else {
            count_without_vel += 1;
        }
    }

    assert_eq!(count_with_vel, 1);
    assert_eq!(count_without_vel, 1);
}

#[test]
#[ignore] // Performance benchmark - run with `cargo test -- --ignored`
fn query_performance_baseline() {
    let mut world = World::new();

    // Spawn 10,000 entities
    for i in 0..10_000 {
        world
            .spawn()
            .with(Position {
                x: i as f32,
                y: i as f32,
            })
            .with(Velocity { x: 1.0, y: 1.0 })
            .id();
    }

    // Measure query iteration
    let start = std::time::Instant::now();
    let mut count = 0;
    for (pos, vel) in world.query::<(&mut Position, &Velocity)>() {
        pos.x += vel.x;
        pos.y += vel.y;
        count += 1;
    }
    let duration = start.elapsed();

    assert_eq!(count, 10_000);
    println!("Query iteration of 10k entities took: {:?}", duration);

    // Should be very fast (< 1ms for 10k entities)
    assert!(duration.as_millis() < 10, "Query should be fast");
}
