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

//! Tests for the Component derive macro.

use pecs::prelude::*;

// Test basic derive macro usage
#[derive(Component, Debug, Clone, Copy)]
struct Position {
    x: f32,
    y: f32,
}

#[derive(Component, Debug, Clone, Copy)]
struct Velocity {
    x: f32,
    y: f32,
}

#[derive(Component, Debug, Clone)]
struct Name(String);

#[derive(Component, Debug)]
struct Health {
    current: i32,
    max: i32,
}

#[test]
fn test_derive_macro_basic() {
    let mut world = World::new();

    // Spawn entity with derived components
    let entity = world
        .spawn()
        .with(Position { x: 1.0, y: 2.0 })
        .with(Velocity { x: 0.5, y: 0.0 })
        .id();

    assert!(world.is_alive(entity));
}

#[test]
fn test_derive_macro_multiple_components() {
    let mut world = World::new();

    let entity = world
        .spawn()
        .with(Position { x: 0.0, y: 0.0 })
        .with(Velocity { x: 1.0, y: 1.0 })
        .with(Name("Player".to_string()))
        .with(Health {
            current: 100,
            max: 100,
        })
        .id();

    assert!(world.is_alive(entity));
    assert!(world.has::<Position>(entity));
    assert!(world.has::<Velocity>(entity));
    assert!(world.has::<Name>(entity));
    assert!(world.has::<Health>(entity));
}

#[test]
fn test_derive_macro_with_generics() {
    #[derive(Component, Debug)]
    struct Container<T> {
        value: T,
    }

    let mut world = World::new();

    let entity = world.spawn().with(Container { value: 42i32 }).id();

    assert!(world.is_alive(entity));
    assert!(world.has::<Container<i32>>(entity));
}

#[test]
fn test_derive_macro_component_access() {
    let mut world = World::new();

    let entity = world.spawn().with(Position { x: 5.0, y: 10.0 }).id();

    // Test immutable access
    if let Some(pos) = world.get::<Position>(entity) {
        assert_eq!(pos.x, 5.0);
        assert_eq!(pos.y, 10.0);
    } else {
        panic!("Position component not found");
    }

    // Test mutable access
    if let Some(pos) = world.get_mut::<Position>(entity) {
        pos.x = 15.0;
        pos.y = 20.0;
    }

    // Verify mutation
    if let Some(pos) = world.get::<Position>(entity) {
        assert_eq!(pos.x, 15.0);
        assert_eq!(pos.y, 20.0);
    }
}

#[test]
fn test_derive_macro_insert_remove() {
    let mut world = World::new();

    let entity = world.spawn().with(Position { x: 0.0, y: 0.0 }).id();

    // Insert a new component
    let inserted = world.insert(entity, Velocity { x: 1.0, y: 1.0 });
    assert!(inserted);
    assert!(world.has::<Velocity>(entity));

    // Remove a component
    let removed = world.remove::<Position>(entity);
    assert!(removed.is_some());
    assert!(!world.has::<Position>(entity));
}

// Made with Bob
