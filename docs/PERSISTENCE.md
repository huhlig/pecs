# Persistence Guide

This guide provides comprehensive documentation on PECS's persistence system, covering formats, plugins, best practices, and advanced usage.

## Table of Contents

1. [Overview](#overview)
2. [Quick Start](#quick-start)
3. [Entity-Specific Persistence](#entity-specific-persistence)
4. [Binary Format](#binary-format)
5. [JSON Format](#json-format)
6. [Custom Plugins](#custom-plugins)
7. [Transient Components](#transient-components)
8. [Version Migrations](#version-migrations)
9. [Performance Optimization](#performance-optimization)
10. [Error Handling](#error-handling)
11. [Best Practices](#best-practices)

---

## Overview

PECS provides a flexible, high-performance persistence system that allows you to save and load entire ECS worlds. The system is designed around several key principles:

### Design Principles

- **Pluggable Architecture**: Support multiple serialization formats
- **Performance**: < 0.5ms per 1000 entities (binary format)
- **Flexibility**: Selective persistence with filters
- **Safety**: Data validation and error recovery
- **Versioning**: Support for schema evolution

### Supported Formats

| Format | Speed | Size | Human-Readable | Use Case |
|--------|-------|------|----------------|----------|
| Binary | ⚡⚡⚡ | 📦 | ❌ | Production saves |
| JSON | ⚡ | 📦📦📦 | ✅ | Debugging, config |
| Custom | Varies | Varies | Varies | Special requirements |

---

## Quick Start

### Basic Save and Load

```rust
use pecs::World;

// Create and populate a world
let mut world = World::new();
let entity = world.spawn();
// ... add components ...

// Save to binary format (default)
world.save("game.pecs")?;

// Load from file
let loaded_world = World::load("game.pecs")?;
```

### Format-Specific Operations

```rust
// Binary format (fast, compact)
world.save_binary("game.pecs")?;
let world = World::load_binary("game.pecs")?;

// JSON format (readable, larger)
world.save_json("game.json")?;
let world = World::load_json("game.json")?;
```

### Streaming API

For large worlds, use streaming to reduce memory usage:

```rust
use std::fs::File;

// Stream to file
let mut file = File::create("large-world.pecs")?;
world.save_binary_stream(&mut file)?;

// Stream from file
let mut file = File::open("large-world.pecs")?;
let world = World::load_binary_stream(&mut file)?;
```

---

## Entity-Specific Persistence

PECS provides entity-specific persistence capabilities through the `EntityPersistencePlugin` trait, allowing you to save and load individual entities rather than entire worlds. This is ideal for database backends, key-value stores, and scenarios where you need granular control over entity persistence.

### Overview

Entity-specific persistence differs from world persistence in several key ways:

| Feature | World Persistence | Entity Persistence | Delta Persistence |
|---------|------------------|-------------------|-------------------|
| **Scope** | Entire world | Individual entities | Changed entities |
| **Use Case** | Save games, snapshots | Databases, lazy loading | Incremental updates |
| **Performance** | Batch operations | Granular operations | Efficient updates |
| **Backend** | File-based | Database, KV store | Any backend |
| **Granularity** | All entities | Single entity | Modified entities |
| **Network** | Full sync | Selective sync | Incremental sync |
| **Memory** | High (full world) | Low (single entity) | Medium (changes) |

### When to Use Each

**World Persistence** (`PersistencePlugin`):
- Save game files
- Complete world snapshots
- Backup/restore operations
- Single-player games
- Small to medium worlds

**Entity Persistence** (`EntityPersistencePlugin`):
- Database backends (SQL, NoSQL)
- Lazy loading large worlds
- Multiplayer entity sync
- Persistent world chunks
- Key-value stores (Redis, DynamoDB)

**Delta Persistence** (`DeltaPersistencePlugin`):
- Real-time synchronization
- Incremental backups
- Change streaming
- Network replication
- Audit logs

### Quick Start

```rust
use pecs::prelude::*;
use pecs::persistence::{KeyValueEntityPlugin, PersistenceManager};

// Create a world and persistence manager
let mut world = World::new();
let mut manager = PersistenceManager::new();

// Register an entity persistence plugin
manager.register_entity_plugin("kv", Box::new(KeyValueEntityPlugin::new()));

// Spawn an entity
let entity = world.spawn()
    .with(Position { x: 1.0, y: 2.0 })
    .with(Velocity { x: 0.5, y: 0.0 })
    .id();

// Save the specific entity
manager.save_entity(&world, entity)?;

// Get the stable ID for later retrieval
let stable_id = world.get_stable_id(entity).unwrap();

// Load the entity back (even in a different world)
let mut new_world = World::new();
let loaded_entity = manager.load_entity(&mut new_world, stable_id)?;
```

### EntityPersistencePlugin Trait

The `EntityPersistencePlugin` trait defines the interface for entity-specific persistence:

```rust
pub trait EntityPersistencePlugin: Send + Sync {
    /// Save a specific entity to persistent storage
    fn save_entity(&self, world: &World, entity: EntityId) -> Result<()>;
    
    /// Load a specific entity from persistent storage
    fn load_entity(&self, world: &mut World, stable_id: StableId) -> Result<EntityId>;
    
    /// Delete a specific entity from persistent storage
    fn delete_entity(&self, stable_id: StableId) -> Result<()>;
    
    /// Check if an entity exists in persistent storage
    fn entity_exists(&self, stable_id: StableId) -> Result<bool>;
    
    /// Save multiple entities in a batch operation
    fn save_entities(&self, world: &World, entities: &[EntityId]) -> Result<()>;
    
    /// Load multiple entities in a batch operation
    fn load_entities(&self, world: &mut World, stable_ids: &[StableId]) -> Result<Vec<EntityId>>;
    
    /// Get the name of this entity persistence backend
    fn backend_name(&self) -> &str;
    
    /// Get the version of this backend implementation
    fn backend_version(&self) -> u32;
}
```

### Built-in Plugin: KeyValueEntityPlugin

PECS includes a simple in-memory key-value entity plugin for testing and reference:

```rust
use pecs::persistence::KeyValueEntityPlugin;

// Create the plugin
let plugin = KeyValueEntityPlugin::new();

// Or with pre-allocated capacity
let plugin = KeyValueEntityPlugin::with_capacity(1000);

// Use with PersistenceManager
manager.register_entity_plugin("memory", Box::new(plugin));
```

### Using with PersistenceManager

The `PersistenceManager` provides convenient methods for entity-specific operations:

```rust
// Save a single entity
manager.save_entity(&world, entity)?;

// Save with a specific plugin
manager.save_entity_with(&world, entity, "redis")?;

// Load an entity
let entity_id = manager.load_entity(&mut world, stable_id)?;

// Load with a specific plugin
let entity_id = manager.load_entity_with(&mut world, stable_id, "database")?;

// Check if entity exists
if manager.entity_exists(stable_id)? {
    println!("Entity exists in storage");
}

// Delete an entity
manager.delete_entity(stable_id)?;
```

### Batch Operations

For better performance, use batch operations when working with multiple entities:

```rust
// Save multiple entities at once
let entities = vec![entity1, entity2, entity3];
manager.save_entity_with(&world, &entities, "database")?;

// Load multiple entities
let stable_ids = vec![id1, id2, id3];
let loaded_entities = manager.load_entities(&mut world, &stable_ids, "database")?;
```

### Custom Entity Persistence Plugin

You can implement custom entity persistence plugins for your specific backend:

```rust
use pecs::persistence::{EntityPersistencePlugin, Result};
use pecs::entity::{EntityId, StableId};
use pecs::World;

struct RedisEntityPlugin {
    client: redis::Client,
}

impl EntityPersistencePlugin for RedisEntityPlugin {
    fn save_entity(&self, world: &World, entity: EntityId) -> Result<()> {
        // Get stable ID
        let stable_id = world.get_stable_id(entity)
            .ok_or_else(|| PersistenceError::EntityNotFound(entity))?;
        
        // Serialize entity and components
        // ... your serialization logic ...
        
        // Store in Redis
        let key = format!("entity:{}", stable_id);
        self.client.set(key, serialized_data)?;
        
        Ok(())
    }
    
    fn load_entity(&self, world: &mut World, stable_id: StableId) -> Result<EntityId> {
        // Fetch from Redis
        let key = format!("entity:{}", stable_id);
        let data = self.client.get(key)?;
        
        // Deserialize and restore entity
        // ... your deserialization logic ...
        
        Ok(entity_id)
    }
    
    fn delete_entity(&self, stable_id: StableId) -> Result<()> {
        let key = format!("entity:{}", stable_id);
        self.client.del(key)?;
        Ok(())
    }
    
    fn entity_exists(&self, stable_id: StableId) -> Result<bool> {
        let key = format!("entity:{}", stable_id);
        Ok(self.client.exists(key)?)
    }
    
    fn backend_name(&self) -> &str {
        "redis"
    }
}
```

### Use Cases

Entity-specific persistence is ideal for:

1. **Database Backends**: Store entities in SQL or NoSQL databases
2. **Lazy Loading**: Load entities on-demand rather than all at once
3. **Multiplayer Games**: Sync individual player entities across network
4. **Persistent Worlds**: Save/load specific regions or chunks
5. **Entity Caching**: Cache frequently accessed entities in Redis/Memcached
6. **Incremental Saves**: Save only modified entities

### Best Practices

1. **Use Stable IDs**: Always use stable IDs for entity references across sessions
2. **Batch Operations**: Use batch save/load for better performance
3. **Error Handling**: Handle missing entities gracefully
4. **Versioning**: Include version information in your custom plugins
5. **Transient Components**: Mark cache/temporary data as transient
6. **Connection Pooling**: Reuse database connections in custom plugins

### Performance Considerations

- **Batch vs Individual**: Batch operations are 10-100x faster for multiple entities
- **Network Latency**: Consider latency when using remote backends
- **Serialization**: Binary formats are faster than JSON for large entities
- **Caching**: Cache frequently accessed entities in memory

### Real-World Examples

#### Example 1: Player Inventory System

```rust
use pecs::persistence::{KeyValueEntityPlugin, PersistenceManager};
use pecs::prelude::*;

#[derive(Component)]
struct PlayerInventory {
    items: Vec<Item>,
    gold: u32,
}

// Save player inventory when it changes
fn save_player_inventory(
    world: &World,
    manager: &PersistenceManager,
    player_entity: EntityId,
) -> Result<()> {
    manager.save_entity(world, player_entity)?;
    Ok(())
}

// Load player inventory on login
fn load_player_inventory(
    world: &mut World,
    manager: &PersistenceManager,
    player_id: StableId,
) -> Result<EntityId> {
    manager.load_entity(world, player_id)
}
```

#### Example 2: Chunk-Based World Loading

```rust
struct WorldChunk {
    x: i32,
    z: i32,
    entities: Vec<StableId>,
}

// Load only visible chunks
fn load_visible_chunks(
    world: &mut World,
    manager: &PersistenceManager,
    player_pos: (i32, i32),
    view_distance: i32,
) -> Result<()> {
    let (px, pz) = player_pos;
    
    for x in (px - view_distance)..=(px + view_distance) {
        for z in (pz - view_distance)..=(pz + view_distance) {
            // Load chunk entities
            let chunk_entities = get_chunk_entities(x, z);
            for stable_id in chunk_entities {
                if !manager.entity_exists(stable_id)? {
                    continue;
                }
                manager.load_entity(world, stable_id)?;
            }
        }
    }
    Ok(())
}
```

#### Example 3: Auto-Save System

```rust
use std::time::{Duration, Instant};

struct AutoSaveSystem {
    last_save: Instant,
    save_interval: Duration,
    dirty_entities: Vec<EntityId>,
}

impl AutoSaveSystem {
    fn new(save_interval_secs: u64) -> Self {
        Self {
            last_save: Instant::now(),
            save_interval: Duration::from_secs(save_interval_secs),
            dirty_entities: Vec::new(),
        }
    }
    
    fn mark_dirty(&mut self, entity: EntityId) {
        if !self.dirty_entities.contains(&entity) {
            self.dirty_entities.push(entity);
        }
    }
    
    fn update(
        &mut self,
        world: &World,
        manager: &PersistenceManager,
    ) -> Result<()> {
        if self.last_save.elapsed() >= self.save_interval {
            // Save all dirty entities
            for &entity in &self.dirty_entities {
                manager.save_entity(world, entity)?;
            }
            
            self.dirty_entities.clear();
            self.last_save = Instant::now();
        }
        Ok(())
    }
}
```

### Troubleshooting

**Problem**: Entity not found when loading  
**Solution**: Verify the stable ID is correct and the entity was saved

**Problem**: Slow performance with many entities  
**Solution**: Use batch operations instead of individual saves/loads

**Problem**: Memory usage too high  
**Solution**: Implement lazy loading and unload distant entities

**Problem**: Concurrent access issues  
**Solution**: Use appropriate locking in your custom plugin implementation


---

## Binary Format

The binary format is optimized for speed and compactness, making it ideal for production use.

### Format Structure

```text
┌─────────────────────────────────────────┐
│ Header (24 bytes)                       │
│  - Magic: "PECS" (4 bytes)             │
│  - Version: u32 (4 bytes)              │
│  - Flags: u32 (4 bytes)                │
│  - Entity count: u64 (8 bytes)         │
│  - Type count: u32 (4 bytes)           │
├─────────────────────────────────────────┤
│ Type Registry                           │
│  For each component type:               │
│    - Type ID: u128 (16 bytes)          │
│    - Name length: u32 (4 bytes)        │
│    - Name: UTF-8 string                │
│    - Version: u32 (4 bytes)            │
├─────────────────────────────────────────┤
│ Entity Data                             │
│  For each entity:                       │
│    - Stable ID: u128 (16 bytes)        │
│    - Component count: u32 (4 bytes)    │
│    - Components:                        │
│      - Type ID: u128 (16 bytes)        │
│      - Data length: u32 (4 bytes)      │
│      - Data: [bytes]                   │
├─────────────────────────────────────────┤
│ Footer (8 bytes)                        │
│  - Checksum: u64 (CRC64)               │
└─────────────────────────────────────────┘
```

### Format Features

#### Magic Bytes

The file starts with "PECS" (0x50454353) to identify the format:

```rust
const MAGIC_BYTES: [u8; 4] = *b"PECS";
```

#### Version Number

Format version for compatibility checking:

```rust
const FORMAT_VERSION: u32 = 1;
const MIN_SUPPORTED_VERSION: u32 = 1;
```

#### Format Flags

Optional features can be enabled via flags:

```rust
pub struct FormatFlags(u32);

impl FormatFlags {
    pub const NONE: Self = Self(0);
    pub const COMPRESSED_ZSTD: Self = Self(1 << 0);
    pub const COMPRESSED_LZ4: Self = Self(1 << 1);
    pub const DELTA: Self = Self(1 << 2);
    pub const EXTENDED_METADATA: Self = Self(1 << 3);
}
```

#### Data Integrity

CRC64 checksum ensures data integrity:

```rust
// Checksum is calculated over all data
let checksum = calculate_checksum(&data);

// Verified on load
if checksum != expected_checksum {
    return Err(PersistenceError::CorruptedData);
}
```

### Performance Characteristics

| Operation | Time (1000 entities) | Throughput |
|-----------|---------------------|------------|
| Save | 0.364ms | ~2.7M entities/sec |
| Load | 0.275ms | ~3.6M entities/sec |
| Roundtrip | 0.639ms | ~1.6M entities/sec |

**Note**: These are actual benchmarks from the optimized implementation.

### Binary Format Usage

```rust
use pecs::persistence::BinaryPlugin;

// Use default binary plugin
let plugin = BinaryPlugin::new();
world.save_with("game.pecs", &plugin)?;

// Load with binary plugin
let world = World::load_with("game.pecs", &plugin)?;
```

---

## JSON Format

The JSON format provides human-readable saves, useful for debugging and configuration.

### Format Structure

```json
{
  "version": 1,
  "entity_count": 2,
  "component_types": [
    {
      "type_id": "12345678901234567890",
      "type_name": "Position",
      "type_version": 1
    }
  ],
  "entities": [
    {
      "stable_id": "98765432109876543210",
      "components": [
        {
          "type_id": "12345678901234567890",
          "type_name": "Position",
          "data": {
            "x": 10.0,
            "y": 20.0
          }
        }
      ]
    }
  ]
}
```

### JSON Format Features

- **Human-Readable**: Easy to inspect and edit
- **Debugging**: Useful for troubleshooting
- **Version Control**: Diffs are meaningful
- **Configuration**: Good for level data

### JSON Format Usage

```rust
use pecs::persistence::JsonPlugin;

// Use JSON plugin
let plugin = JsonPlugin::new();
world.save_with("game.json", &plugin)?;

// Pretty-printed JSON
let plugin = JsonPlugin::pretty();
world.save_with("game.json", &plugin)?;

// Load from JSON
let world = World::load_with("game.json", &plugin)?;
```

### Performance Characteristics

| Operation | Time (1000 entities) | vs Binary |
|-----------|---------------------|-----------|
| Save | ~2-3ms | 5-8x slower |
| Load | ~3-4ms | 10-15x slower |
| File Size | ~10-20x larger | Much larger |

**Use JSON for**: Debugging, configuration, version control
**Use Binary for**: Production saves, performance-critical paths

---

## Custom Plugins

Create custom serialization formats by implementing the `PersistencePlugin` trait.

### Plugin Trait

```rust
pub trait PersistencePlugin: Send + Sync {
    /// Serialize a world to the given writer
    fn save(&self, world: &World, writer: &mut dyn Write) -> Result<()>;
    
    /// Deserialize a world from the given reader
    fn load(&self, reader: &mut dyn Read) -> Result<World>;
    
    /// Get the name of this format
    fn format_name(&self) -> &str;
    
    /// Get the version of this format
    fn format_version(&self) -> u32;
    
    /// Check if this plugin can handle the given version
    fn can_load_version(&self, version: u32) -> bool {
        version == self.format_version()
    }
}
```

### Example: MessagePack Plugin

```rust
use pecs::persistence::{PersistencePlugin, Result};
use pecs::World;
use std::io::{Read, Write};

struct MessagePackPlugin;

impl PersistencePlugin for MessagePackPlugin {
    fn save(&self, world: &World, writer: &mut dyn Write) -> Result<()> {
        // Serialize using MessagePack
        let data = rmp_serde::to_vec(world)?;
        writer.write_all(&data)?;
        Ok(())
    }

    fn load(&self, reader: &mut dyn Read) -> Result<World> {
        // Deserialize using MessagePack
        let mut data = Vec::new();
        reader.read_to_end(&mut data)?;
        let world = rmp_serde::from_slice(&data)?;
        Ok(world)
    }

    fn format_name(&self) -> &str {
        "messagepack"
    }

    fn format_version(&self) -> u32 {
        1
    }
}
```

### Example: Database Plugin

```rust
struct PostgresPlugin {
    connection_string: String,
}

impl PersistencePlugin for PostgresPlugin {
    fn save(&self, world: &World, _writer: &mut dyn Write) -> Result<()> {
        let conn = postgres::Client::connect(&self.connection_string, NoTls)?;
        
        // Begin transaction
        let mut transaction = conn.transaction()?;
        
        // Save entities
        for entity in world.entities() {
            transaction.execute(
                "INSERT INTO entities (stable_id) VALUES ($1)",
                &[&entity.stable_id().to_string()],
            )?;
            
            // Save components
            for component in entity.components() {
                // Serialize and insert component data
            }
        }
        
        transaction.commit()?;
        Ok(())
    }

    fn load(&self, _reader: &mut dyn Read) -> Result<World> {
        let mut conn = postgres::Client::connect(&self.connection_string, NoTls)?;
        let mut world = World::new();
        
        // Load entities
        for row in conn.query("SELECT * FROM entities", &[])? {
            let stable_id: String = row.get(0);
            let entity = world.spawn_with_stable_id(stable_id.parse()?);
            
            // Load components for this entity
            // ...
        }
        
        Ok(world)
    }

    fn format_name(&self) -> &str {
        "postgres"
    }

    fn format_version(&self) -> u32 {
        1
    }
}
```

### Plugin Best Practices

1. **Version Compatibility**: Implement `can_load_version()` for backward compatibility
2. **Error Handling**: Provide detailed error messages
3. **Validation**: Validate data integrity
4. **Performance**: Consider streaming for large datasets
5. **Testing**: Test with various world sizes and configurations

---

## Transient Components

Transient components are excluded from persistence. See [Advanced Features Guide](ADVANCED_FEATURES.md#transient-components) for details.

### Quick Example

```rust
use pecs::persistence::TransientComponent;

// Type-level transient
#[derive(Component)]
struct FrameCounter {
    count: u64,
}

impl TransientComponent for FrameCounter {}

// Instance-level transient
impl SerializableComponent for CachedData {
    fn is_persistent(&self) -> bool {
        self.is_valid  // Only persist if valid
    }
}
```

---

## Version Migrations

Handle schema changes between versions of your application.

### Migration Trait

```rust
pub trait Migration: Send + Sync {
    /// Get the version this migration upgrades from
    fn source_version(&self) -> u32;
    
    /// Get the version this migration upgrades to
    fn target_version(&self) -> u32;
    
    /// Perform the migration
    fn migrate(&self, world: &mut World) -> Result<()>;
}
```

### Example Migration

```rust
struct PositionMigrationV1ToV2;

impl Migration for PositionMigrationV1ToV2 {
    fn source_version(&self) -> u32 { 1 }
    fn target_version(&self) -> u32 { 2 }
    
    fn migrate(&self, world: &mut World) -> Result<()> {
        // Convert 2D positions to 3D
        for entity in world.entities() {
            if let Some(pos2d) = world.get_component::<Position2D>(entity) {
                let pos3d = Position3D {
                    x: pos2d.x,
                    y: pos2d.y,
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

### Migration Chain

```rust
use pecs::persistence::MigrationChain;

let migrations = MigrationChain::new()
    .add(PositionMigrationV1ToV2)
    .add(HealthMigrationV2ToV3)
    .add(InventoryMigrationV3ToV4);

// Automatically applies necessary migrations
let world = World::load_with_migrations("old-save.dat", migrations)?;
```

---

## Performance Optimization

### Pre-allocation

```rust
// Pre-allocate world capacity
let mut world = World::with_capacity(10000);

// Reduces reallocations during load
```

### Streaming for Large Worlds

```rust
use std::fs::File;
use std::io::BufWriter;

// Use buffered writer for better I/O performance
let file = File::create("large-world.pecs")?;
let mut writer = BufWriter::new(file);
world.save_binary_stream(&mut writer)?;
```

### Selective Persistence

```rust
use pecs::persistence::WhitelistFilter;

// Only save essential components
let filter = WhitelistFilter {
    allowed_types: hashset![
        ComponentTypeId::of::<Position>(),
        ComponentTypeId::of::<Health>(),
    ],
};

world.save_filtered("minimal.pecs", filter)?;
```

### Compression (Future)

```rust
// Compression support (planned)
let plugin = BinaryPlugin::with_compression(CompressionType::Zstd);
world.save_with("game.pecs.zst", &plugin)?;
```

### Performance Tips

1. **Use Binary Format**: 5-15x faster than JSON
2. **Pre-allocate**: Use `with_capacity()` when size is known
3. **Stream Large Worlds**: Reduce memory usage
4. **Filter Unnecessary Data**: Exclude transient components
5. **Batch Operations**: Save multiple worlds together
6. **Profile**: Measure before optimizing

---

## Error Handling

### Error Types

```rust
pub enum PersistenceError {
    /// I/O error during read/write
    Io(std::io::Error),
    
    /// Invalid format or corrupted data
    InvalidFormat(String),
    
    /// Unsupported format version
    UnsupportedVersion { found: u32, expected: u32 },
    
    /// Checksum mismatch (data corruption)
    CorruptedData,
    
    /// Component deserialization failed
    ComponentError(String),
    
    /// Migration failed
    MigrationError(String),
}
```

### Error Handling Patterns

```rust
use pecs::persistence::PersistenceError;

// Handle specific errors
match world.load("game.pecs") {
    Ok(world) => { /* Success */ }
    Err(PersistenceError::UnsupportedVersion { found, expected }) => {
        eprintln!("Version mismatch: found {}, expected {}", found, expected);
        // Attempt migration
    }
    Err(PersistenceError::CorruptedData) => {
        eprintln!("Save file corrupted, loading backup...");
        // Load backup
    }
    Err(e) => {
        eprintln!("Failed to load: {}", e);
    }
}
```

### Validation

```rust
// Validate before saving
if world.entity_count() > MAX_ENTITIES {
    return Err(PersistenceError::InvalidFormat(
        "Too many entities".to_string()
    ));
}

// Validate after loading
let world = World::load("game.pecs")?;
if !world.validate() {
    return Err(PersistenceError::CorruptedData);
}
```

---

## Best Practices

### 1. Choose the Right Format

```rust
// Production: Use binary
world.save_binary("game.pecs")?;

// Development: Use JSON for debugging
world.save_json("debug.json")?;

// Configuration: Use JSON for level data
level.save_json("levels/level1.json")?;
```

### 2. Implement Backups

```rust
use std::fs;

// Create backup before saving
if Path::new("game.pecs").exists() {
    fs::copy("game.pecs", "game.pecs.backup")?;
}

world.save("game.pecs")?;
```

### 3. Version Your Saves

```rust
// Include version in filename
let version = env!("CARGO_PKG_VERSION");
let filename = format!("game-v{}.pecs", version);
world.save(&filename)?;
```

### 4. Handle Errors Gracefully

```rust
fn save_with_retry(world: &World, path: &str, retries: u32) -> Result<()> {
    for attempt in 0..retries {
        match world.save(path) {
            Ok(()) => return Ok(()),
            Err(e) if attempt < retries - 1 => {
                eprintln!("Save failed (attempt {}): {}", attempt + 1, e);
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => return Err(e),
        }
    }
    unreachable!()
}
```

### 5. Test Persistence

```rust
#[test]
fn test_save_load_roundtrip() {
    let mut world = World::new();
    let entity = world.spawn();
    world.insert_component(entity, Position { x: 1.0, y: 2.0 });
    
    // Save
    world.save("test.pecs").unwrap();
    
    // Load
    let loaded = World::load("test.pecs").unwrap();
    
    // Verify
    assert_eq!(world.entity_count(), loaded.entity_count());
    
    // Cleanup
    std::fs::remove_file("test.pecs").unwrap();
}
```

### 6. Document Your Schema

```rust
/// Position component (v1)
/// 
/// # Persistence
/// - Format: Binary (8 bytes: 2x f32)
/// - Version: 1
/// - Transient: No
#[derive(Component)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}
```

### 7. Monitor Performance

```rust
use std::time::Instant;

let start = Instant::now();
world.save("game.pecs")?;
let duration = start.elapsed();

if duration > Duration::from_millis(100) {
    eprintln!("Warning: Save took {:?}", duration);
}
```

---

## Troubleshooting

### Common Issues

#### 1. "Invalid magic bytes" Error

**Cause**: File is not a valid PECS binary file
**Solution**: Verify file format, check for corruption

```rust
// Verify file format
let mut file = File::open("game.pecs")?;
let mut magic = [0u8; 4];
file.read_exact(&mut magic)?;

if magic != b"PECS" {
    eprintln!("Not a PECS file");
}
```

#### 2. "Unsupported version" Error

**Cause**: Save file from newer/older version
**Solution**: Implement migration or update application

```rust
// Check version before loading
let header = Header::read(&mut file)?;
if header.version > FORMAT_VERSION {
    eprintln!("Save file from newer version");
}
```

#### 3. "Corrupted data" Error

**Cause**: Checksum mismatch
**Solution**: Load backup, investigate corruption source

```rust
// Verify checksum
let calculated = calculate_checksum(&data);
if calculated != expected {
    eprintln!("Checksum mismatch: {} != {}", calculated, expected);
}
```

#### 4. Slow Save/Load Performance

**Cause**: Large world, inefficient I/O
**Solution**: Use streaming, filter unnecessary data

```rust
// Profile save operation
let start = Instant::now();
world.save("game.pecs")?;
println!("Save took: {:?}", start.elapsed());

// Use streaming for large worlds
let mut file = BufWriter::new(File::create("game.pecs")?);
world.save_binary_stream(&mut file)?;
```

---

## See Also

- [Getting Started Guide](GETTING_STARTED.md) - Basic usage
- [Advanced Features](ADVANCED_FEATURES.md) - Advanced persistence patterns
- [Performance Guide](PERFORMANCE.md) - Optimization techniques
- [API Reference](https://docs.rs/pecs) - Complete API documentation
- [ADR-006](ADR/ADR-006-pluggable-persistence.md) - Persistence architecture
- [ADR-007](ADR/ADR-007-binary-format-specification.md) - Binary format spec

---

*Made with Bob*