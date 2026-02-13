# Advanced Features Guide

This guide covers advanced features and patterns in PECS for experienced users who want to leverage the full power of the library.

## Table of Contents

1. [Custom Persistence Plugins](#custom-persistence-plugins)
2. [Delta Persistence](#delta-persistence)
3. [Transient Components](#transient-components)
4. [Persistence Filters](#persistence-filters)
5. [Version Migrations](#version-migrations)
6. [Complex Query Patterns](#complex-query-patterns)
7. [Command Buffer Patterns](#command-buffer-patterns)
8. [Performance Optimization](#performance-optimization)

---

## Custom Persistence Plugins

PECS provides a pluggable persistence system that allows you to implement custom serialization formats beyond the built-in binary and JSON formats.

### Implementing a Custom Plugin

```rust
use pecs::persistence::{PersistencePlugin, Result};
use pecs::World;
use std::io::{Read, Write};

struct YamlPlugin;

impl PersistencePlugin for YamlPlugin {
    fn save(&self, world: &World, writer: &mut dyn Write) -> Result<()> {
        // Implement YAML serialization
        // Access world entities and components
        // Write to the provided writer
        Ok(())
    }

    fn load(&self, reader: &mut dyn Read) -> Result<World> {
        // Implement YAML deserialization
        // Read from the provided reader
        // Reconstruct the world
        Ok(World::new())
    }

    fn format_name(&self) -> &str {
        "yaml"
    }

    fn format_version(&self) -> u32 {
        1
    }

    fn can_load_version(&self, version: u32) -> bool {
        // Support backward compatibility
        version >= 1 && version <= self.format_version()
    }
}
```

### Using Custom Plugins

```rust
use pecs::World;

let mut world = World::new();
// ... populate world ...

// Register and use custom plugin
let plugin = YamlPlugin;
world.save_with("world.yaml", &plugin)?;

// Load with custom plugin
let loaded = World::load_with("world.yaml", &plugin)?;
```

### Plugin Best Practices

1. **Version Compatibility**: Implement `can_load_version()` to support older formats
2. **Error Handling**: Provide detailed error messages for debugging
3. **Performance**: Consider streaming for large worlds
4. **Validation**: Validate data integrity during load
5. **Documentation**: Document your format specification

---

## Delta Persistence

Delta persistence allows you to save only the changes to your world, which is essential for database backends, network synchronization, and incremental backups.

### Implementing Delta Persistence

```rust
use pecs::persistence::{DeltaPersistencePlugin, EntityChange, Result};
use pecs::World;

struct DatabaseBackend {
    connection: DatabaseConnection,
}

impl DeltaPersistencePlugin for DatabaseBackend {
    fn save_changes(&self, changes: &[EntityChange]) -> Result<()> {
        for change in changes {
            match change {
                EntityChange::Created { entity, components, timestamp } => {
                    // INSERT INTO entities ...
                    self.connection.insert_entity(*entity, components, *timestamp)?;
                }
                EntityChange::Modified { entity, added_or_modified, removed, timestamp } => {
                    // UPDATE entities ...
                    self.connection.update_entity(
                        *entity,
                        added_or_modified,
                        removed,
                        *timestamp
                    )?;
                }
                EntityChange::Deleted { entity, timestamp } => {
                    // DELETE FROM entities ...
                    self.connection.delete_entity(*entity, *timestamp)?;
                }
            }
        }
        Ok(())
    }

    fn load_changes(&self, since: u64) -> Result<Vec<EntityChange>> {
        // SELECT * FROM entities WHERE timestamp > since
        self.connection.get_changes_since(since)
    }
}
```

### Using Delta Persistence

```rust
use pecs::persistence::ChangeTracker;

let mut world = World::new();
let mut tracker = ChangeTracker::new();

// Track changes
let entity = world.spawn();
tracker.track_created(entity);

// Save only changes
let backend = DatabaseBackend::new();
backend.save_changes(&tracker.get_changes())?;

// Load incremental updates
let changes = backend.load_changes(last_sync_time)?;
backend.apply_changes(&mut world, &changes)?;
```

### Use Cases

- **Database Backends**: SQL, NoSQL, key-value stores
- **Network Sync**: Multiplayer game state synchronization
- **Incremental Backups**: Save only what changed
- **Audit Logs**: Track all entity modifications
- **Undo/Redo**: Implement undo by reversing changes

---

## Transient Components

Transient components are not persisted when saving the world. This is useful for runtime state, cached data, and external resources.

### Type-Level Transient Marking

Mark entire component types as transient:

```rust
use pecs::component::Component;
use pecs::persistence::TransientComponent;

// This component will never be saved
#[derive(Debug, Clone)]
struct FrameCounter {
    count: u64,
}

impl Component for FrameCounter {}
impl TransientComponent for FrameCounter {}
```

### Instance-Level Transient Control

Control persistence per instance:

```rust
use pecs::persistence::SerializableComponent;

#[derive(Debug, Clone)]
struct PathfindingCache {
    path: Vec<Position>,
    is_valid: bool,
}

impl SerializableComponent for PathfindingCache {
    fn is_persistent(&self) -> bool {
        // Only persist if cache is valid
        self.is_valid
    }
    
    fn serialize(&self, writer: &mut dyn Write) -> Result<()> {
        // Serialization logic
        Ok(())
    }
    
    fn deserialize(reader: &mut dyn Read) -> Result<Self> {
        // Deserialization logic
        Ok(Self { path: vec![], is_valid: false })
    }
}
```

### Common Transient Patterns

#### Debug Components

```rust
#[derive(Component, Transient)]
struct DebugVisualization {
    lines: Vec<Line>,
    color: Color,
}
```

#### External Resources

```rust
#[derive(Component, Transient)]
struct TextureHandle {
    handle: GpuTextureHandle,  // Can't serialize GPU handles
}

// Companion component for persistence
#[derive(Component)]
struct TexturePath {
    path: String,  // Serialize path instead
}
```

#### Cached Data

```rust
#[derive(Component)]
struct NavigationMesh {
    mesh: Vec<Triangle>,
    dirty: bool,
}

impl SerializableComponent for NavigationMesh {
    fn is_persistent(&self) -> bool {
        !self.dirty  // Only persist clean caches
    }
}
```

---

## Persistence Filters

Filters provide fine-grained control over what gets persisted.

### Built-in Filters

```rust
use pecs::persistence::{NoTransientFilter, WhitelistFilter, BlacklistFilter};

// Exclude all transient components
world.save_filtered("game.save", NoTransientFilter)?;

// Only save specific components
let filter = WhitelistFilter {
    allowed_types: hashset![
        ComponentTypeId::of::<Position>(),
        ComponentTypeId::of::<Health>(),
    ],
};
world.save_filtered("minimal.save", filter)?;

// Exclude specific components
let filter = BlacklistFilter {
    excluded_types: hashset![
        ComponentTypeId::of::<UiElement>(),
        ComponentTypeId::of::<ParticleEffect>(),
    ],
};
world.save_filtered("no-ui.save", filter)?;
```

### Custom Filters

```rust
use pecs::persistence::PersistenceFilter;

struct PlayerOnlyFilter;

impl PersistenceFilter for PlayerOnlyFilter {
    fn should_persist(
        &self,
        entity: Entity,
        _component_type: ComponentTypeId,
        _component: &dyn Any,
    ) -> bool {
        // Only save entities with Player component
        entity.has_component::<Player>()
    }
}

world.save_filtered("player.save", PlayerOnlyFilter)?;
```

### Composite Filters

Combine multiple filters with AND/OR logic:

```rust
use pecs::persistence::{CompositeFilter, FilterMode};

let filter = CompositeFilter {
    filters: vec![
        Box::new(NoTransientFilter),
        Box::new(PlayerOnlyFilter),
    ],
    mode: FilterMode::All,  // Both filters must pass
};

world.save_filtered("player-persistent.save", filter)?;
```

---

## Version Migrations

Handle schema changes between versions of your game or application.

### Implementing Migrations

```rust
use pecs::persistence::{Migration, Result};
use pecs::World;

struct PositionMigrationV1ToV2;

impl Migration for PositionMigrationV1ToV2 {
    fn source_version(&self) -> u32 {
        1
    }

    fn target_version(&self) -> u32 {
        2
    }

    fn migrate(&self, world: &mut World) -> Result<()> {
        // Convert 2D positions to 3D by adding z=0
        for entity in world.entities() {
            if let Some(mut pos) = world.get_component_mut::<Position2D>(entity) {
                let pos3d = Position3D {
                    x: pos.x,
                    y: pos.y,
                    z: 0.0,
                };
                world.remove_component::<Position2D>(entity);
                world.insert_component(entity, pos3d);
            }
        }
        Ok(())
    }
}
```

### Migration Chains

```rust
use pecs::persistence::MigrationChain;

let migrations = MigrationChain::new()
    .add(PositionMigrationV1ToV2)
    .add(HealthMigrationV2ToV3)
    .add(InventoryMigrationV3ToV4);

// Automatically applies all necessary migrations
let world = World::load_with_migrations("old-save.dat", migrations)?;
```

### Migration Best Practices

1. **Test Thoroughly**: Test migrations with real save files
2. **Backup Data**: Always backup before migrating
3. **Incremental**: Migrate one version at a time
4. **Reversible**: Consider implementing downgrade migrations
5. **Document**: Document what each migration does

---

## Complex Query Patterns

Advanced query patterns for sophisticated entity filtering and access.

### Multi-Component Queries

```rust
// Query entities with multiple components
for (entity, pos, vel, health) in world.query::<(Entity, &mut Position, &Velocity, &Health)>() {
    if health.current > 0 {
        pos.x += vel.x;
        pos.y += vel.y;
    }
}
```

### Filtered Queries

```rust
// Query with component filters
for pos in world.query::<&mut Position>()
    .with::<Velocity>()      // Must have Velocity
    .without::<Dead>()       // Must not have Dead
{
    // Only alive entities with velocity
}
```

### Optional Components

```rust
// Handle optional components
for (entity, pos, opt_vel) in world.query::<(Entity, &mut Position, Option<&Velocity>)>() {
    if let Some(vel) = opt_vel {
        pos.x += vel.x;
        pos.y += vel.y;
    }
}
```

### Query Caching

```rust
// Cache query results for repeated access
let mut query = world.query::<(&Position, &Velocity)>();
let cached = query.cache();

// Use cached results multiple times
for (pos, vel) in cached.iter() {
    // Fast iteration
}
```

### Parallel Queries

```rust
use rayon::prelude::*;

// Parallel query execution (requires rayon feature)
world.query::<&mut Position>()
    .par_iter()
    .for_each(|pos| {
        // Process in parallel
        pos.x *= 1.1;
    });
```

---

## Command Buffer Patterns

Advanced patterns for using command buffers effectively.

### Batched Operations

```rust
use pecs::command::CommandBuffer;

let mut buffer = CommandBuffer::with_capacity(1000);

// Batch spawn many entities
for i in 0..1000 {
    let entity = buffer.spawn();
    buffer.insert(entity, Position { x: i as f32, y: 0.0 });
    buffer.insert(entity, Velocity { x: 1.0, y: 0.0 });
}

// Apply all at once
buffer.apply(&mut world);
```

### Conditional Commands

```rust
// Record commands based on conditions
for (entity, health) in world.query::<(Entity, &Health)>() {
    if health.current <= 0 {
        buffer.despawn(entity);
        buffer.spawn();  // Spawn replacement
    }
}
```

### Multi-Threaded Command Recording

```rust
use std::sync::Mutex;
use rayon::prelude::*;

let buffers: Vec<Mutex<CommandBuffer>> = (0..num_threads)
    .map(|_| Mutex::new(CommandBuffer::new()))
    .collect();

// Record commands in parallel
entities.par_iter().enumerate().for_each(|(i, entity)| {
    let thread_id = i % num_threads;
    let mut buffer = buffers[thread_id].lock().unwrap();
    
    // Record commands
    buffer.despawn(*entity);
});

// Apply all buffers
for buffer in buffers {
    buffer.into_inner().unwrap().apply(&mut world);
}
```

### Command Priorities

```rust
// Implement priority-based command execution
struct PriorityCommandBuffer {
    high_priority: CommandBuffer,
    normal_priority: CommandBuffer,
    low_priority: CommandBuffer,
}

impl PriorityCommandBuffer {
    fn apply(&mut self, world: &mut World) {
        self.high_priority.apply(world);
        self.normal_priority.apply(world);
        self.low_priority.apply(world);
    }
}
```

---

## Performance Optimization

Advanced techniques for maximizing PECS performance.

### Pre-allocation

```rust
// Pre-allocate capacity to avoid reallocations
let mut world = World::with_capacity(10000);

// Pre-allocate command buffers
let mut buffer = CommandBuffer::with_capacity(1000);
```

### Archetype Optimization

```rust
// Group entities by component composition for cache efficiency
// Entities with same components are stored together

// Good: Entities with same components
for i in 0..1000 {
    world.spawn()
        .with(Position { x: i as f32, y: 0.0 })
        .with(Velocity { x: 1.0, y: 0.0 })
        .build();
}

// Less optimal: Mixed component compositions
for i in 0..1000 {
    let mut builder = world.spawn();
    builder.with(Position { x: i as f32, y: 0.0 });
    if i % 2 == 0 {
        builder.with(Velocity { x: 1.0, y: 0.0 });
    }
    builder.build();
}
```

### Query Optimization

```rust
// Minimize query scope
// Bad: Query all entities
for (entity, pos) in world.query::<(Entity, &mut Position)>() {
    if entity.has_component::<Player>() {
        pos.x += 1.0;
    }
}

// Good: Filter at query level
for pos in world.query::<&mut Position>().with::<Player>() {
    pos.x += 1.0;
}
```

### Memory Layout

```rust
// Use smaller component types for better cache locality
// Bad: Large component
struct Transform {
    position: [f32; 3],
    rotation: [f32; 4],
    scale: [f32; 3],
    matrix: [f32; 16],  // Cached, can be recomputed
}

// Good: Split into smaller components
struct Position([f32; 3]);
struct Rotation([f32; 4]);
struct Scale([f32; 3]);
// Compute matrix on demand
```

### Persistence Optimization

```rust
// Use binary format for performance
world.save_binary("game.pecs")?;  // Fast

// Use JSON for debugging
world.save_json("game.json")?;    // Readable

// Use streaming for large worlds
let mut file = File::create("large-world.pecs")?;
world.save_binary_stream(&mut file)?;
```

### Profiling

```rust
use std::time::Instant;

// Profile query performance
let start = Instant::now();
for pos in world.query::<&mut Position>() {
    pos.x += 1.0;
}
let duration = start.elapsed();
println!("Query took: {:?}", duration);

// Profile persistence
let start = Instant::now();
world.save("game.pecs")?;
let duration = start.elapsed();
println!("Save took: {:?}", duration);
```

---

## Best Practices Summary

1. **Use Binary Format**: For production saves (fast, compact)
2. **Use JSON Format**: For debugging and human-readable saves
3. **Mark Transient Components**: Exclude runtime state from saves
4. **Pre-allocate**: Use `with_capacity()` when size is known
5. **Filter Queries**: Use `.with()` and `.without()` for efficiency
6. **Batch Commands**: Use command buffers for deferred operations
7. **Profile**: Measure performance before optimizing
8. **Test Migrations**: Always test version upgrades
9. **Document Formats**: Document custom persistence formats
10. **Handle Errors**: Provide detailed error messages

---

## See Also

- [Getting Started Guide](GETTING_STARTED.md) - Basic usage
- [Core Concepts](CONCEPTS.md) - Architecture and design
- [Performance Guide](PERFORMANCE.md) - Optimization techniques
- [API Reference](https://docs.rs/pecs) - Complete API documentation

---

*Made with Bob*