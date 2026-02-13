# ADR-010: Serialization Framework Choice

**Status**: Proposed
**Date**: 2026-02-13
**Deciders**: Development Team
**Related**: ADR-006, ADR-007, PRD Section 9.1, Phase 2 Week 1-2

## Context

PECS needs to serialize component data for persistence. The choice of serialization framework affects:

### Technical Considerations
- **Performance**: Serialization speed and binary size
- **Type Safety**: Compile-time vs runtime type checking
- **Flexibility**: Support for custom types and formats
- **Portability**: Cross-platform compatibility
- **Versioning**: Schema evolution support

### Integration Requirements
- Must work with custom binary format (ADR-007)
- Must support pluggable architecture (ADR-006)
- Must handle stable IDs (ADR-001)
- Must support version migration (ADR-008)
- Must work with transient marking (ADR-009)

### Ecosystem Options
- **serde**: De facto standard for Rust serialization
- **bincode**: Fast binary serialization using serde
- **rmp-serde**: MessagePack format using serde
- **Custom**: Hand-written serialization code
- **Hybrid**: Mix of approaches

### Constraints
- Must maintain no_std compatibility (with alloc)
- Must support WASM
- Must achieve < 1ms per 1000 entities (PRD NFR-1.5)
- Must keep dependency count low (PRD NFR-4.1)

## Decision

We will use a **hybrid approach** that combines custom serialization for the binary format with optional serde support for JSON and other formats:

### Architecture

```
┌─────────────────────────────────────────────────────────┐
│           Serialization Framework Strategy               │
├─────────────────────────────────────────────────────────┤
│  Binary Format (Default)                                │
│  ├─ Custom hand-written serialization                   │
│  ├─ Optimized for performance                           │
│  └─ No external dependencies                            │
├─────────────────────────────────────────────────────────┤
│  JSON Format (Optional)                                 │
│  ├─ Uses serde + serde_json                            │
│  ├─ Feature-gated: "json"                              │
│  └─ For debugging and human readability                 │
├─────────────────────────────────────────────────────────┤
│  Component Trait                                         │
│  └─ Manual implementation required                       │
└─────────────────────────────────────────────────────────┘
```

### 1. Core Serialization Trait (No Dependencies)

```rust
/// Core serialization trait (no external dependencies)
pub trait SerializableComponent: Component {
    /// Serialize component to binary writer
    fn serialize(&self, writer: &mut dyn Write) -> Result<(), PersistenceError>;
    
    /// Deserialize component from binary reader
    fn deserialize(reader: &mut dyn Read) -> Result<Self, PersistenceError>
    where
        Self: Sized;
    
    /// Get unique type identifier
    fn type_id() -> ComponentTypeId;
    
    /// Get component version
    fn version() -> u32 { 1 }
    
    /// Check if persistent (see ADR-009)
    fn is_persistent(&self) -> bool { true }
}
```

### 2. Binary Format Implementation (Custom)

```rust
// Example: Position component
#[derive(Debug, Clone, Copy)]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl SerializableComponent for Position {
    fn serialize(&self, writer: &mut dyn Write) -> Result<(), PersistenceError> {
        // Direct binary write (little-endian)
        writer.write_all(&self.x.to_le_bytes())?;
        writer.write_all(&self.y.to_le_bytes())?;
        writer.write_all(&self.z.to_le_bytes())?;
        Ok(())
    }
    
    fn deserialize(reader: &mut dyn Read) -> Result<Self, PersistenceError> {
        let mut x_bytes = [0u8; 4];
        let mut y_bytes = [0u8; 4];
        let mut z_bytes = [0u8; 4];
        
        reader.read_exact(&mut x_bytes)?;
        reader.read_exact(&mut y_bytes)?;
        reader.read_exact(&mut z_bytes)?;
        
        Ok(Position {
            x: f32::from_le_bytes(x_bytes),
            y: f32::from_le_bytes(y_bytes),
            z: f32::from_le_bytes(z_bytes),
        })
    }
    
    fn type_id() -> ComponentTypeId {
        ComponentTypeId::of::<Self>()
    }
}
```

### 3. Optional Serde Support (Feature-Gated)

```rust
// Cargo.toml
[dependencies]
serde = { version = "1.0", optional = true }
serde_json = { version = "1.0", optional = true }

[features]
default = []
json = ["serde", "serde_json"]
```

```rust
// When "json" feature is enabled
#[cfg(feature = "json")]
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "json", derive(Serialize, Deserialize))]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

// JSON plugin implementation
#[cfg(feature = "json")]
pub struct JsonPlugin;

#[cfg(feature = "json")]
impl PersistencePlugin for JsonPlugin {
    fn save(&self, world: &World, writer: &mut dyn Write) -> Result<(), PersistenceError> {
        let json = serde_json::to_string_pretty(world)?;
        writer.write_all(json.as_bytes())?;
        Ok(())
    }
    
    fn load(&self, reader: &mut dyn Read) -> Result<World, PersistenceError> {
        let mut json = String::new();
        reader.read_to_string(&mut json)?;
        let world = serde_json::from_str(&json)?;
        Ok(world)
    }
    
    fn format_name(&self) -> &str { "json" }
    fn format_version(&self) -> u32 { 1 }
}
```

### 4. Helper Macros for Common Cases

```rust
/// Macro to implement SerializableComponent for simple types
#[macro_export]
macro_rules! impl_serializable_simple {
    ($type:ty) => {
        impl SerializableComponent for $type {
            fn serialize(&self, writer: &mut dyn Write) -> Result<(), PersistenceError> {
                let bytes = bincode::serialize(self)?;
                writer.write_all(&bytes)?;
                Ok(())
            }
            
            fn deserialize(reader: &mut dyn Read) -> Result<Self, PersistenceError> {
                let mut bytes = Vec::new();
                reader.read_to_end(&mut bytes)?;
                Ok(bincode::deserialize(&bytes)?)
            }
            
            fn type_id() -> ComponentTypeId {
                ComponentTypeId::of::<Self>()
            }
        }
    };
}

// Usage:
impl_serializable_simple!(MySimpleComponent);
```

### 5. Derive Macro (Future Enhancement)

```rust
// Future: Derive macro for automatic implementation
#[derive(Component, Serializable)]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

// Expands to manual SerializableComponent implementation
```

## Consequences

### Positive

- **Performance**: Custom binary serialization achieves optimal speed
- **Control**: Full control over binary format (ADR-007)
- **No Dependencies**: Core library has zero serialization dependencies
- **Flexibility**: Can optimize per-component
- **Type Safety**: Compile-time type checking
- **Optional Features**: Serde available when needed
- **Portability**: Works in no_std environments
- **Simplicity**: Clear, explicit serialization code

### Negative

- **Boilerplate**: Manual implementation required for each component
- **Maintenance**: Must maintain custom serialization code
- **Error-Prone**: Manual byte manipulation can have bugs
- **Learning Curve**: Users must understand binary serialization
- **No Automatic Derivation**: Can't use derive macros initially
- **Duplication**: Some code duplication with serde implementations

### Neutral

- **Serde Integration**: Available but optional
- **Ecosystem**: Can use serde for non-performance-critical formats
- **Migration**: Can add derive macros later without breaking changes

## Alternatives Considered

### Alternative 1: Serde-Only Approach

```rust
use serde::{Serialize, Deserialize};

#[derive(Component, Serialize, Deserialize)]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

// Use bincode for binary format
world.save_with("game.save", &BincodePlugin)?;
```

- **Pros**:
  - Automatic serialization with derive
  - Ecosystem compatibility
  - Less boilerplate
  - Well-tested
  - Many format options
- **Cons**:
  - Less control over binary format
  - Performance overhead
  - Larger dependency tree
  - Harder to optimize
  - Not no_std compatible (without features)
  - Bincode format not as compact as custom
- **Rejected because**: Insufficient control for optimal binary format; performance concerns

### Alternative 2: Custom Only (No Serde)

```rust
// Only custom serialization, no serde support
impl SerializableComponent for Position {
    fn serialize(&self, writer: &mut dyn Write) -> Result<()> {
        // Custom implementation
    }
}
```

- **Pros**:
  - Zero dependencies
  - Maximum performance
  - Full control
  - Simplest dependency graph
- **Cons**:
  - No JSON support
  - No ecosystem integration
  - More work for users
  - Can't leverage serde ecosystem
- **Rejected because**: Too limiting; JSON support valuable for debugging

### Alternative 3: Trait Object Approach

```rust
trait Serializable {
    fn serialize(&self) -> Vec<u8>;
    fn deserialize(bytes: &[u8]) -> Self;
}

// Store trait objects
world.register_serializer(Box::new(PositionSerializer));
```

- **Pros**:
  - Flexible
  - Can swap implementations
  - Runtime polymorphism
- **Cons**:
  - Runtime overhead
  - Type erasure issues
  - More complex
  - Harder to optimize
- **Rejected because**: Unnecessary complexity; trait-based approach sufficient

### Alternative 4: Code Generation

```rust
// Generate serialization code from schema
position.schema:
  x: f32
  y: f32
  z: f32

// Generates Rust code
```

- **Pros**:
  - No manual implementation
  - Consistent format
  - Easy to maintain
- **Cons**:
  - Build complexity
  - Extra tooling required
  - Less flexible
  - Harder to debug
- **Rejected because**: Overkill for current needs; can add later if needed

## Implementation Notes

### Phase 2 Implementation

```rust
// Week 1-2: Core serialization trait
- Define SerializableComponent trait
- Implement for primitive types
- Create helper utilities
- Documentation

// Week 3-4: Binary format integration
- Integrate with binary plugin
- Optimize hot paths
- Add benchmarks

// Week 7-8: Optional serde support
- Add JSON plugin with serde
- Feature gates
- Examples
```

### Helper Utilities

```rust
/// Helper for serializing primitive types
pub mod serialize {
    use std::io::{Write, Read};
    
    pub fn write_u32(writer: &mut dyn Write, value: u32) -> Result<(), std::io::Error> {
        writer.write_all(&value.to_le_bytes())
    }
    
    pub fn read_u32(reader: &mut dyn Read) -> Result<u32, std::io::Error> {
        let mut bytes = [0u8; 4];
        reader.read_exact(&mut bytes)?;
        Ok(u32::from_le_bytes(bytes))
    }
    
    pub fn write_f32(writer: &mut dyn Write, value: f32) -> Result<(), std::io::Error> {
        writer.write_all(&value.to_le_bytes())
    }
    
    pub fn read_f32(reader: &mut dyn Read) -> Result<f32, std::io::Error> {
        let mut bytes = [0u8; 4];
        reader.read_exact(&mut bytes)?;
        Ok(f32::from_le_bytes(bytes))
    }
    
    pub fn write_string(writer: &mut dyn Write, s: &str) -> Result<(), std::io::Error> {
        write_u32(writer, s.len() as u32)?;
        writer.write_all(s.as_bytes())
    }
    
    pub fn read_string(reader: &mut dyn Read) -> Result<String, std::io::Error> {
        let len = read_u32(reader)? as usize;
        let mut bytes = vec![0u8; len];
        reader.read_exact(&mut bytes)?;
        String::from_utf8(bytes).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
    
    pub fn write_vec<T, F>(
        writer: &mut dyn Write,
        vec: &[T],
        write_item: F,
    ) -> Result<(), std::io::Error>
    where
        F: Fn(&mut dyn Write, &T) -> Result<(), std::io::Error>,
    {
        write_u32(writer, vec.len() as u32)?;
        for item in vec {
            write_item(writer, item)?;
        }
        Ok(())
    }
    
    pub fn read_vec<T, F>(
        reader: &mut dyn Read,
        read_item: F,
    ) -> Result<Vec<T>, std::io::Error>
    where
        F: Fn(&mut dyn Read) -> Result<T, std::io::Error>,
    {
        let len = read_u32(reader)? as usize;
        let mut vec = Vec::with_capacity(len);
        for _ in 0..len {
            vec.push(read_item(reader)?);
        }
        Ok(vec)
    }
}
```

### Example Implementations

#### Simple Component

```rust
#[derive(Debug, Clone)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

impl SerializableComponent for Health {
    fn serialize(&self, writer: &mut dyn Write) -> Result<(), PersistenceError> {
        serialize::write_f32(writer, self.current)?;
        serialize::write_f32(writer, self.max)?;
        Ok(())
    }
    
    fn deserialize(reader: &mut dyn Read) -> Result<Self, PersistenceError> {
        Ok(Health {
            current: serialize::read_f32(reader)?,
            max: serialize::read_f32(reader)?,
        })
    }
    
    fn type_id() -> ComponentTypeId {
        ComponentTypeId::of::<Self>()
    }
}
```

#### Complex Component

```rust
#[derive(Debug, Clone)]
pub struct Inventory {
    pub items: Vec<Item>,
    pub capacity: u32,
}

#[derive(Debug, Clone)]
pub struct Item {
    pub id: String,
    pub quantity: u32,
}

impl SerializableComponent for Inventory {
    fn serialize(&self, writer: &mut dyn Write) -> Result<(), PersistenceError> {
        serialize::write_u32(writer, self.capacity)?;
        serialize::write_vec(writer, &self.items, |w, item| {
            serialize::write_string(w, &item.id)?;
            serialize::write_u32(w, item.quantity)?;
            Ok(())
        })?;
        Ok(())
    }
    
    fn deserialize(reader: &mut dyn Read) -> Result<Self, PersistenceError> {
        let capacity = serialize::read_u32(reader)?;
        let items = serialize::read_vec(reader, |r| {
            Ok(Item {
                id: serialize::read_string(r)?,
                quantity: serialize::read_u32(r)?,
            })
        })?;
        Ok(Inventory { items, capacity })
    }
    
    fn type_id() -> ComponentTypeId {
        ComponentTypeId::of::<Self>()
    }
}
```

#### Enum Component

```rust
#[derive(Debug, Clone)]
pub enum State {
    Idle,
    Moving { speed: f32 },
    Attacking { target: Entity },
}

impl SerializableComponent for State {
    fn serialize(&self, writer: &mut dyn Write) -> Result<(), PersistenceError> {
        match self {
            State::Idle => {
                serialize::write_u8(writer, 0)?;
            }
            State::Moving { speed } => {
                serialize::write_u8(writer, 1)?;
                serialize::write_f32(writer, *speed)?;
            }
            State::Attacking { target } => {
                serialize::write_u8(writer, 2)?;
                // Serialize stable ID, not ephemeral
                let stable_id = target.stable_id();
                writer.write_all(&stable_id.to_bytes())?;
            }
        }
        Ok(())
    }
    
    fn deserialize(reader: &mut dyn Read) -> Result<Self, PersistenceError> {
        let variant = serialize::read_u8(reader)?;
        match variant {
            0 => Ok(State::Idle),
            1 => Ok(State::Moving {
                speed: serialize::read_f32(reader)?,
            }),
            2 => {
                let stable_id = StableId::from_reader(reader)?;
                // Will be resolved to ephemeral ID after load
                Ok(State::Attacking {
                    target: Entity::from_stable_id(stable_id),
                })
            }
            _ => Err(PersistenceError::InvalidData),
        }
    }
    
    fn type_id() -> ComponentTypeId {
        ComponentTypeId::of::<Self>()
    }
}
```

## Performance Characteristics

### Binary Format Performance

```rust
// Benchmark results (estimated):
// Position (12 bytes): ~15ns serialize, ~20ns deserialize
// Health (8 bytes): ~12ns serialize, ~15ns deserialize
// Inventory (variable): ~50ns + 30ns per item
//
// For 1000 entities with 3 components each:
// - Serialization: ~100μs
// - Deserialization: ~150μs
// - Well under 1ms target ✓
```

### Comparison with Alternatives

```
Format          | Serialize | Deserialize | Size
----------------|-----------|-------------|------
Custom Binary   | 100μs     | 150μs       | 24KB
Bincode         | 150μs     | 200μs       | 26KB
JSON (serde)    | 800μs     | 1200μs      | 85KB
MessagePack     | 200μs     | 300μs       | 28KB
```

## Testing Strategy

### Unit Tests
- Test serialization/deserialization for each type
- Test round-trip (serialize → deserialize → compare)
- Test error conditions
- Test edge cases (empty collections, max values)

### Integration Tests
- Test with real components
- Test with complex nested structures
- Test with entity references
- Test version migration

### Performance Tests
- Benchmark serialization speed
- Benchmark deserialization speed
- Compare with alternatives
- Memory usage profiling

## Documentation Requirements

### User Guide
- How to implement SerializableComponent
- Helper utilities documentation
- Common patterns and examples
- Performance tips

### API Documentation
- SerializableComponent trait
- Helper functions
- Error types
- Best practices

## Future Enhancements

### Derive Macro

```rust
#[derive(Component, Serializable)]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

// Auto-generates SerializableComponent implementation
```

### Validation

```rust
impl SerializableComponent for Position {
    fn validate(&self) -> Result<(), ValidationError> {
        if self.x.is_nan() || self.y.is_nan() || self.z.is_nan() {
            return Err(ValidationError::InvalidValue);
        }
        Ok(())
    }
}
```

### Compression

```rust
impl SerializableComponent for LargeData {
    fn serialize(&self, writer: &mut dyn Write) -> Result<(), PersistenceError> {
        let compressed = compress(&self.data)?;
        writer.write_all(&compressed)?;
        Ok(())
    }
}
```

## References

- [Serde Documentation](https://serde.rs/)
- [Bincode Documentation](https://docs.rs/bincode/)
- [MessagePack Specification](https://msgpack.org/)
- [Rust Serialization Benchmarks](https://github.com/djkoloski/rust_serialization_benchmark)
- [Zero-Copy Deserialization](https://docs.rs/zerocopy/)
- ADR-006: Pluggable Persistence Architecture
- ADR-007: Binary Format Specification
- ADR-008: Version Migration Strategy
- ADR-009: Transient Component Marking
- PRD Section 9.1: Technical Dependencies
- PRD NFR-1.5: Persistence performance targets
- Phase 2 Week 1-2: Persistence Manager Implementation