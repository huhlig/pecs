# Getting Started with PECS

Welcome to PECS (Persistent Entity Component System) - a high-performance, minimalist ECS library for Rust with integrated persistence capabilities.

## Table of Contents

- [Installation](#installation)
- [Quick Start](#quick-start)
- [Core Concepts](#core-concepts)
- [Basic Operations](#basic-operations)
- [Next Steps](#next-steps)

## Installation

Add PECS to your `Cargo.toml`:

```toml
[dependencies]
pecs = "0.1.0"
```

## Quick Start

Here's a minimal example to get you started:

```rust
use pecs::prelude::*;

// 1. Define your components
#[derive(Debug)]
struct Position {
    x: f32,
    y: f32,
}
impl Component for Position {}

#[derive(Debug)]
struct Velocity {
    x: f32,
    y: f32,
}
impl Component for Velocity {}

fn main() {
    // 2. Create a world
    let mut world = World::new();

    // 3. Spawn entities with components
    let player = world.spawn()
        .with(Position { x: 0.0, y: 0.0 })
        .with(Velocity { x: 1.0, y: 0.0 })
        .id();

    let enemy = world.spawn()
        .with(Position { x: 10.0, y: 5.0 })
        .with(Velocity { x: -0.5, y: 0.0 })
        .id();

    println!("Created player: {}", player);
    println!("Created enemy: {}", enemy);
    println!("Total entities: {}", world.len());

    // 4. Check if entities are alive
    assert!(world.is_alive(player));
    assert!(world.is_alive(enemy));

    // 5. Despawn an entity
    world.despawn(enemy);
    assert!(!world.is_alive(enemy));
    println!("Entities after despawn: {}", world.len());
}
```

## Core Concepts

### Entities

Entities are unique identifiers that represent game objects or data records. In PECS, entities use a dual-ID system:

- **EntityId**: Fast 64-bit ID for runtime operations
- **StableId**: Persistent 128-bit UUID for serialization

```rust
use pecs::World;

let mut world = World::new();

// Spawn an empty entity
let entity = world.spawn_empty();

// Get the stable ID for persistence
let stable_id = world.get_stable_id(entity).unwrap();
println!("Entity {} has stable ID {}", entity, stable_id);
```

### Components

Components are plain data structures that can be attached to entities. They must implement the `Component` trait:

```rust
use pecs::component::Component;

#[derive(Debug, Clone, Copy)]
struct Health {
    current: i32,
    max: i32,
}
impl Component for Health {}

#[derive(Debug)]
struct Name(String);
impl Component for Name {}
```

**Requirements:**
- Must be `'static` (no non-static references)
- Must be `Send + Sync` (thread-safe)
- Should be plain data (no complex logic)

### World

The `World` is the main container that manages all entities and components:

```rust
use pecs::World;

// Create a new world
let mut world = World::new();

// Create with pre-allocated capacity for better performance
let mut world = World::with_capacity(1000);

// Check world state
println!("Entities: {}", world.len());
println!("Empty: {}", world.is_empty());

// Clear all entities
world.clear();
```

## Basic Operations

### Spawning Entities

```rust
use pecs::prelude::*;

#[derive(Debug)]
struct Position { x: f32, y: f32 }
impl Component for Position {}

let mut world = World::new();

// Spawn with builder pattern
let entity = world.spawn()
    .with(Position { x: 0.0, y: 0.0 })
    .id();

// Spawn empty entity
let empty = world.spawn_empty();
```

### Despawning Entities

```rust
let mut world = World::new();
let entity = world.spawn_empty();

// Despawn returns true if successful
if world.despawn(entity) {
    println!("Entity despawned");
}

// Despawning again returns false
assert!(!world.despawn(entity));
```

### Checking Entity Status

```rust
let mut world = World::new();
let entity = world.spawn_empty();

// Check if entity is alive
if world.is_alive(entity) {
    println!("Entity is alive");
}

// After despawn, entity is no longer alive
world.despawn(entity);
assert!(!world.is_alive(entity));
```

### Working with Stable IDs

```rust
let mut world = World::new();
let entity = world.spawn_empty();

// Get stable ID for persistence
let stable_id = world.get_stable_id(entity).unwrap();

// Look up entity by stable ID
let found = world.get_entity_id(stable_id);
assert_eq!(found, Some(entity));
```

### Using Command Buffers

Command buffers allow thread-safe deferred operations:

```rust
use pecs::World;

let mut world = World::new();

// Get mutable reference to command buffer
world.commands().spawn();
world.commands().spawn();

// Commands aren't applied yet
assert_eq!(world.len(), 0);

// Apply all pending commands
world.apply_commands();
assert_eq!(world.len(), 2);
```

### Iterating Entities

```rust
let mut world = World::new();
world.spawn_empty();
world.spawn_empty();
world.spawn_empty();

// Iterate over all entities with their stable IDs
for (entity, stable_id) in world.iter_entities() {
    println!("Entity {} has stable ID {}", entity, stable_id);
}
```

## Performance Tips

### Pre-allocate Capacity

If you know how many entities you'll create, pre-allocate:

```rust
// Pre-allocate for 10,000 entities
let mut world = World::with_capacity(10_000);
```

### Batch Operations

Use command buffers for batch operations:

```rust
let mut world = World::new();

// Record many operations
for _ in 0..1000 {
    world.commands().spawn();
}

// Apply all at once (more efficient)
world.apply_commands();
```

### Reserve Capacity

Reserve additional capacity before spawning many entities:

```rust
use pecs::entity::EntityManager;

let mut manager = EntityManager::new();
manager.reserve(1000); // Reserve space for 1000 more entities

for _ in 0..1000 {
    manager.spawn();
}
```

## Next Steps

Now that you understand the basics, explore these topics:

1. **[Core Concepts](CONCEPTS.md)** - Deep dive into ECS architecture
2. **[Persistence Guide](PERSISTENCE.md)** - Learn about save/load functionality
3. **[Performance Guide](PERFORMANCE.md)** - Optimize your ECS usage
4. **[API Reference](https://docs.rs/pecs)** - Complete API documentation
5. **[Examples](../examples/)** - Browse example applications

## Common Patterns

### Game Loop Integration

```rust
use pecs::prelude::*;

#[derive(Debug)]
struct Position { x: f32, y: f32 }
impl Component for Position {}

#[derive(Debug)]
struct Velocity { x: f32, y: f32 }
impl Component for Velocity {}

fn main() {
    let mut world = World::new();
    
    // Spawn some entities
    for i in 0..10 {
        world.spawn()
            .with(Position { x: i as f32, y: 0.0 })
            .with(Velocity { x: 1.0, y: 0.0 })
            .id();
    }

    // Game loop (simplified)
    for _frame in 0..60 {
        // Update systems would go here
        // In Phase 3, query system will be integrated
        
        // Apply any deferred commands
        world.apply_commands();
    }
}
```

### Entity Recycling

PECS automatically recycles entity IDs:

```rust
let mut world = World::new();

let e1 = world.spawn_empty();
println!("First entity: {}", e1);

world.despawn(e1);

let e2 = world.spawn_empty();
println!("Second entity: {}", e2);

// e2 reuses e1's index but has a different generation
assert_eq!(e1.index(), e2.index());
assert_ne!(e1.generation(), e2.generation());
```

## Troubleshooting

### Entity Not Found

If you get `None` when looking up an entity:

```rust
let entity = world.spawn_empty();
world.despawn(entity);

// This returns None because entity was despawned
assert_eq!(world.get_stable_id(entity), None);
```

### Generation Mismatch

If an entity ID doesn't work after recycling:

```rust
let e1 = world.spawn_empty();
world.despawn(e1);
let e2 = world.spawn_empty(); // Reuses e1's slot

// e1 is now stale - it has the old generation
assert!(!world.is_alive(e1));
assert!(world.is_alive(e2));
```

## Getting Help

- **Documentation**: [docs.rs/pecs](https://docs.rs/pecs)
- **Examples**: Check the `examples/` directory
- **Issues**: [GitHub Issues](https://github.com/yourusername/pecs/issues)
- **Discussions**: [GitHub Discussions](https://github.com/yourusername/pecs/discussions)

## What's Next?

Continue learning with:

- **[Core Concepts Guide](CONCEPTS.md)** - Understand ECS architecture deeply
- **[Query System](QUERIES.md)** - Learn to query entities efficiently (Coming in Phase 3)
- **[Persistence](PERSISTENCE.md)** - Save and load your world
- **[Advanced Patterns](ADVANCED.md)** - Master advanced techniques

Happy coding with PECS! 🚀