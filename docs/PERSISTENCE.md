# Persistence Guide

This guide provides comprehensive documentation on PECS's persistence system, covering formats, plugins, best practices, and advanced usage.

## Table of Contents

1. [Overview](#overview)
2. [Quick Start](#quick-start)
3. [Binary Format](#binary-format)
4. [JSON Format](#json-format)
5. [Custom Plugins](#custom-plugins)
6. [Transient Components](#transient-components)
7. [Version Migrations](#version-migrations)
8. [Performance Optimization](#performance-optimization)
9. [Error Handling](#error-handling)
10. [Best Practices](#best-practices)

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