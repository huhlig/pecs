# ADR-006: Pluggable Persistence Architecture

**Status**: Proposed
**Date**: 2026-02-13
**Deciders**: Development Team
**Related**: ADR-001, ADR-005, PRD FR-4.4, Phase 2 Week 7-8

## Context

PECS requires a persistence system that can save and load world state. However, different use cases have different requirements:

- **Game developers** may need human-readable JSON for debugging or modding support
- **Performance-critical applications** require fast binary formats
- **Enterprise users** may need database integration or custom formats
- **Cross-platform applications** need portable serialization
- **Embedded systems** may have storage constraints requiring compression

A monolithic persistence implementation would force all users to accept the same trade-offs. Additionally, as a library (per ADR-005), PECS should not impose specific serialization choices on users.

The challenge is designing a persistence system that:
1. Provides excellent out-of-the-box experience with sensible defaults
2. Allows users to customize or replace serialization formats
3. Maintains type safety and correctness
4. Doesn't compromise performance for flexibility
5. Keeps the API simple and intuitive

## Decision

We will implement a **pluggable persistence architecture** using a trait-based plugin system that allows users to choose or implement custom serialization backends while providing high-quality default implementations.

### Architecture Overview

#### Core Persistence Trait

```rust
/// Main trait for persistence plugins
pub trait PersistencePlugin: Send + Sync {
    /// Serialize world state to a writer
    fn save(&self, world: &World, writer: &mut dyn Write) -> Result<(), PersistenceError>;
    
    /// Deserialize world state from a reader
    fn load(&self, reader: &mut dyn Read) -> Result<World, PersistenceError>;
    
    /// Get the format identifier (e.g., "binary", "json")
    fn format_name(&self) -> &str;
    
    /// Get the format version for compatibility checking
    fn format_version(&self) -> u32;
    
    /// Optional: Check if this plugin can handle the given data
    fn can_load(&self, reader: &mut dyn Read) -> Result<bool, PersistenceError> {
        // Default implementation checks magic bytes/header
        Ok(false)
    }
}
```

#### Component Serialization Trait

```rust
/// Trait for components that can be persisted
pub trait SerializableComponent: Component {
    /// Serialize component data
    fn serialize(&self, writer: &mut dyn Write) -> Result<(), PersistenceError>;
    
    /// Deserialize component data
    fn deserialize(reader: &mut dyn Read) -> Result<Self, PersistenceError> 
    where 
        Self: Sized;
    
    /// Get unique type identifier for this component
    fn type_id() -> ComponentTypeId;
    
    /// Get component version for migration support
    fn version() -> u32 { 1 }
    
    /// Check if this component should be persisted (default: true)
    fn is_persistent(&self) -> bool { true }
}
```

#### World API Integration

```rust
impl World {
    /// Save world using default binary format
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), PersistenceError> {
        self.save_with(path, &BinaryPlugin::default())
    }
    
    /// Save world using specific plugin
    pub fn save_with<P: AsRef<Path>>(
        &self, 
        path: P, 
        plugin: &dyn PersistencePlugin
    ) -> Result<(), PersistenceError> {
        let mut file = File::create(path)?;
        plugin.save(self, &mut file)
    }
    
    /// Load world using default binary format
    pub fn load<P: AsRef<Path>>(path: P) -> Result<World, PersistenceError> {
        Self::load_with(path, &BinaryPlugin::default())
    }
    
    /// Load world using specific plugin
    pub fn load_with<P: AsRef<Path>>(
        path: P, 
        plugin: &dyn PersistencePlugin
    ) -> Result<World, PersistenceError> {
        let mut file = File::open(path)?;
        plugin.load(&mut file)
    }
    
    /// Auto-detect format and load
    pub fn load_auto<P: AsRef<Path>>(path: P) -> Result<World, PersistenceError> {
        let mut file = File::open(path)?;
        
        // Try registered plugins
        for plugin in &REGISTERED_PLUGINS {
            if plugin.can_load(&mut file)? {
                file.seek(SeekFrom::Start(0))?;
                return plugin.load(&mut file);
            }
        }
        
        Err(PersistenceError::UnknownFormat)
    }
}
```

### Built-in Plugins

#### 1. Binary Plugin (Default)
- High-performance binary format
- Optimized for speed and size
- Default choice for production use
- See ADR-007 for format specification

#### 2. JSON Plugin
- Human-readable format
- Useful for debugging and modding
- Based on serde_json
- Slower but more accessible

#### 3. Future Plugins (Community/Optional)
- MessagePack plugin
- Database plugin (SQLite, PostgreSQL)
- Network streaming plugin
- Compressed binary plugin

### Plugin Registration

```rust
/// Global plugin registry (optional, for auto-detection)
static REGISTERED_PLUGINS: Lazy<Vec<Box<dyn PersistencePlugin>>> = Lazy::new(|| {
    vec![
        Box::new(BinaryPlugin::default()),
        Box::new(JsonPlugin::default()),
    ]
});

/// Register custom plugin for auto-detection
pub fn register_persistence_plugin(plugin: Box<dyn PersistencePlugin>) {
    REGISTERED_PLUGINS.lock().unwrap().push(plugin);
}
```

### Type Registry System

```rust
/// Manages component type information for serialization
pub struct TypeRegistry {
    types: HashMap<ComponentTypeId, TypeInfo>,
}

pub struct TypeInfo {
    type_id: ComponentTypeId,
    type_name: String,
    version: u32,
    serialize_fn: fn(&dyn Any, &mut dyn Write) -> Result<(), PersistenceError>,
    deserialize_fn: fn(&mut dyn Read) -> Result<Box<dyn Any>, PersistenceError>,
}

impl TypeRegistry {
    /// Register a component type for serialization
    pub fn register<T: SerializableComponent>(&mut self) {
        self.types.insert(
            T::type_id(),
            TypeInfo {
                type_id: T::type_id(),
                type_name: std::any::type_name::<T>().to_string(),
                version: T::version(),
                serialize_fn: |any, writer| {
                    let component = any.downcast_ref::<T>().unwrap();
                    component.serialize(writer)
                },
                deserialize_fn: |reader| {
                    T::deserialize(reader).map(|c| Box::new(c) as Box<dyn Any>)
                },
            },
        );
    }
}
```

## Consequences

### Positive

- **Flexibility**: Users can choose the format that best fits their needs
- **Extensibility**: Easy to add new formats without modifying core code
- **Library Philosophy**: Aligns with ADR-005 by not imposing choices
- **Performance**: Binary plugin can be optimized without affecting other formats
- **Debugging**: JSON plugin enables easy inspection of saved data
- **Future-Proof**: New formats can be added as separate crates
- **Type Safety**: Trait system ensures correctness at compile time
- **Separation of Concerns**: Persistence logic separated from core ECS

### Negative

- **Complexity**: More complex than single-format approach
- **API Surface**: Additional traits and types to learn
- **Type Registration**: Components must implement SerializableComponent trait
- **Runtime Overhead**: Dynamic dispatch for plugin calls (minimal in practice)
- **Documentation**: Need to explain plugin system and when to use each format
- **Testing**: Must test multiple format implementations

### Neutral

- **Default Choice**: Binary format as default balances performance and usability
- **Community Plugins**: Third-party formats can be distributed as separate crates
- **Migration Path**: Plugin system supports format evolution over time

## Alternatives Considered

### Alternative 1: Single Built-in Format (Binary Only)

```rust
impl World {
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()>;
    pub fn load<P: AsRef<Path>>(path: P) -> Result<World>;
}
```

- **Pros**:
  - Simplest implementation
  - Smallest API surface
  - Best performance
  - No plugin complexity
- **Cons**:
  - No flexibility for different use cases
  - Can't debug saved files easily
  - Forces binary format on all users
  - Hard to extend later
- **Rejected because**: Too inflexible; conflicts with library philosophy (ADR-005)

### Alternative 2: Multiple Built-in Formats (No Plugin System)

```rust
impl World {
    pub fn save_binary<P: AsRef<Path>>(&self, path: P) -> Result<()>;
    pub fn save_json<P: AsRef<Path>>(&self, path: P) -> Result<()>;
    pub fn load_binary<P: AsRef<Path>>(path: P) -> Result<World>;
    pub fn load_json<P: AsRef<Path>>(path: P) -> Result<World>;
}
```

- **Pros**:
  - Simple to use
  - No plugin complexity
  - Multiple formats available
- **Cons**:
  - API bloat with each new format
  - Can't add custom formats
  - All formats must be in core library
  - Increases compile time and binary size
- **Rejected because**: Not extensible; bloats core library

### Alternative 3: Serde-Based Approach

```rust
#[derive(Serialize, Deserialize)]
struct Position { x: f32, y: f32 }

// World automatically serializable
world.save("file.json")?; // Uses serde
```

- **Pros**:
  - Leverages existing ecosystem
  - Automatic serialization
  - Many formats supported
  - Familiar to Rust developers
- **Cons**:
  - Less control over format
  - Performance overhead
  - Harder to optimize binary format
  - Requires all components to derive Serialize
  - Doesn't handle stable IDs well
- **Rejected because**: Insufficient control for optimized binary format; can still use serde in JSON plugin

### Alternative 4: Callback-Based System

```rust
world.save_with_callbacks(
    path,
    |entity, writer| { /* custom serialization */ },
    |reader| { /* custom deserialization */ }
)?;
```

- **Pros**:
  - Maximum flexibility
  - No trait requirements
  - Simple for one-off cases
- **Cons**:
  - Not reusable
  - Error-prone
  - No type safety
  - Verbose for common cases
- **Rejected because**: Too low-level; plugins provide better abstraction

## Implementation Notes

### Phase 2 Implementation Plan

#### Week 1-2: Core Infrastructure
- Define PersistencePlugin trait
- Define SerializableComponent trait
- Implement TypeRegistry
- Create error types
- Basic World integration

#### Week 3-4: Binary Plugin
- Implement BinaryPlugin (see ADR-007)
- Optimize for performance
- Add compression support
- Comprehensive testing

#### Week 7-8: JSON Plugin & Finalization
- Implement JsonPlugin using serde
- Plugin registration system
- Auto-detection support
- Documentation and examples

### Example: Custom Plugin Implementation

```rust
pub struct CustomPlugin {
    compression: bool,
}

impl PersistencePlugin for CustomPlugin {
    fn save(&self, world: &World, writer: &mut dyn Write) -> Result<(), PersistenceError> {
        // Write custom header
        writer.write_all(b"CUSTOM")?;
        
        // Serialize entities
        for entity in world.entities() {
            self.serialize_entity(entity, writer)?;
        }
        
        Ok(())
    }
    
    fn load(&self, reader: &mut dyn Read) -> Result<World, PersistenceError> {
        // Verify header
        let mut header = [0u8; 6];
        reader.read_exact(&mut header)?;
        if &header != b"CUSTOM" {
            return Err(PersistenceError::InvalidFormat);
        }
        
        // Deserialize entities
        let mut world = World::new();
        while let Ok(entity) = self.deserialize_entity(reader) {
            world.insert_entity(entity);
        }
        
        Ok(world)
    }
    
    fn format_name(&self) -> &str { "custom" }
    fn format_version(&self) -> u32 { 1 }
}
```

### Example: Using Plugins

```rust
// Use default binary format
world.save("game.save")?;
let world = World::load("game.save")?;

// Use JSON for debugging
world.save_with("debug.json", &JsonPlugin::default())?;

// Use custom plugin
let custom = CustomPlugin { compression: true };
world.save_with("game.custom", &custom)?;

// Auto-detect format
let world = World::load_auto("unknown.save")?;
```

### Component Implementation

```rust
#[derive(Debug, Clone)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

impl SerializableComponent for Position {
    fn serialize(&self, writer: &mut dyn Write) -> Result<(), PersistenceError> {
        writer.write_all(&self.x.to_le_bytes())?;
        writer.write_all(&self.y.to_le_bytes())?;
        Ok(())
    }
    
    fn deserialize(reader: &mut dyn Read) -> Result<Self, PersistenceError> {
        let mut x_bytes = [0u8; 4];
        let mut y_bytes = [0u8; 4];
        reader.read_exact(&mut x_bytes)?;
        reader.read_exact(&mut y_bytes)?;
        
        Ok(Position {
            x: f32::from_le_bytes(x_bytes),
            y: f32::from_le_bytes(y_bytes),
        })
    }
    
    fn type_id() -> ComponentTypeId {
        ComponentTypeId::of::<Self>()
    }
}
```

## Performance Considerations

### Plugin Dispatch Overhead
- Dynamic dispatch adds ~1-2ns per call (negligible)
- Amortized over thousands of entities
- Can be optimized with monomorphization for hot paths

### Memory Usage
- Plugin trait objects: ~16 bytes per plugin
- Type registry: ~64 bytes per component type
- Minimal overhead compared to world data

### Optimization Opportunities
- Binary plugin can use unsafe for maximum performance
- Streaming support for large worlds
- Parallel serialization for independent entities
- Memory-mapped I/O for very large files

## Testing Strategy

### Unit Tests
- Test each plugin independently
- Test trait implementations
- Test error conditions
- Test type registry

### Integration Tests
- Round-trip tests for each plugin
- Cross-plugin compatibility
- Large world tests
- Concurrent access tests

### Performance Tests
- Benchmark each plugin
- Compare against alternatives
- Memory usage profiling
- Streaming performance

## Documentation Requirements

### User Guide
- When to use each format
- How to implement custom plugins
- Performance characteristics
- Best practices

### API Documentation
- Trait documentation with examples
- Plugin implementation guide
- Error handling guide
- Migration guide from other ECS systems

## Future Extensions

### Potential Plugins
- **MessagePack**: Compact binary format with schema
- **CBOR**: Concise binary object representation
- **Database**: Direct database persistence
- **Network**: Streaming over network
- **Compressed**: Transparent compression layer

### Advanced Features
- Incremental/delta serialization
- Lazy loading for large worlds
- Streaming deserialization
- Parallel serialization
- Custom compression algorithms

## References

- [Serde - Serialization Framework](https://serde.rs/)
- [MessagePack Specification](https://msgpack.org/)
- [Plugin Architecture Patterns](https://en.wikipedia.org/wiki/Plugin_(computing))
- [Rust Trait Objects](https://doc.rust-lang.org/book/ch17-02-trait-objects.html)
- ADR-001: Dual Entity ID System
- ADR-005: Library vs Framework Approach
- ADR-007: Binary Format Specification (to be written)
- PRD FR-4.4: Pluggable persistence backends
- Phase 2 Development Plan: Week 7-8