# ADR-007: Binary Format Specification

**Status**: Proposed
**Date**: 2026-02-13
**Deciders**: Development Team
**Related**: ADR-001, ADR-006, PRD NFR-1.5, Phase 2 Week 3-4

## Context

The binary persistence format is the default serialization format for PECS and must meet several critical requirements:

### Performance Requirements (from PRD)
- **Speed**: < 1ms per 1000 entities for save/load operations
- **Throughput**: > 100 MB/s read/write
- **Memory**: < 2x world size during serialization

### Functional Requirements
- **Completeness**: Preserve all entity and component data
- **Correctness**: Maintain entity relationships and stable IDs
- **Versioning**: Support format evolution and migration
- **Validation**: Detect corruption and invalid data
- **Portability**: Work across platforms (endianness, alignment)

### Design Constraints
- Must integrate with dual entity ID system (ADR-001)
- Must support pluggable architecture (ADR-006)
- Must enable streaming for large worlds
- Must support optional compression
- Must be deterministic for testing and debugging

The challenge is designing a format that achieves high performance while remaining robust, portable, and evolvable.

## Decision

We will implement a **custom binary format** optimized for PECS's archetype-based storage with the following design:

### Format Overview

```
┌─────────────────────────────────────────────────────────┐
│                      File Header                         │
│  - Magic bytes, version, flags, metadata                │
├─────────────────────────────────────────────────────────┤
│                    Type Registry                         │
│  - Component type definitions and metadata               │
├─────────────────────────────────────────────────────────┤
│                   Archetype Table                        │
│  - Archetype definitions (component combinations)        │
├─────────────────────────────────────────────────────────┤
│                    Entity Data                           │
│  - Entities grouped by archetype (SoA layout)           │
├─────────────────────────────────────────────────────────┤
│                   Resource Data                          │
│  - World-level singleton resources                       │
├─────────────────────────────────────────────────────────┤
│                      Footer                              │
│  - Checksum, validation data                            │
└─────────────────────────────────────────────────────────┘
```

### Detailed Format Specification

#### 1. File Header (64 bytes)

```rust
struct FileHeader {
    magic: [u8; 4],           // "PECS" (0x50454353)
    version_major: u16,       // Format major version
    version_minor: u16,       // Format minor version
    flags: u32,               // Feature flags (compression, etc.)
    entity_count: u64,        // Total number of entities
    archetype_count: u32,     // Number of archetypes
    component_type_count: u32,// Number of component types
    resource_count: u32,      // Number of resources
    timestamp: u64,           // Unix timestamp (milliseconds)
    reserved: [u8; 24],       // Reserved for future use
}

// Flags bitfield
const FLAG_COMPRESSED: u32      = 0x0001;  // Data is compressed
const FLAG_ENCRYPTED: u32       = 0x0002;  // Data is encrypted (future)
const FLAG_INCREMENTAL: u32     = 0x0004;  // Incremental/delta save
const FLAG_BIG_ENDIAN: u32      = 0x0008;  // Big-endian encoding
const FLAG_STREAMING: u32       = 0x0010;  // Supports streaming load
```

#### 2. Type Registry Section

```rust
struct TypeRegistryHeader {
    type_count: u32,          // Number of types
    total_size: u64,          // Total size of this section
}

struct ComponentTypeEntry {
    type_id: u128,            // Unique type identifier (hash)
    type_name_len: u16,       // Length of type name
    type_name: [u8],          // UTF-8 type name (variable length)
    type_version: u32,        // Component version for migration
    flags: u32,               // Type-specific flags
    size_hint: u32,           // Expected size (for allocation)
}

// Type flags
const TYPE_FLAG_TRANSIENT: u32  = 0x0001;  // Not persisted by default
const TYPE_FLAG_RESOURCE: u32   = 0x0002;  // World-level resource
const TYPE_FLAG_ZERO_SIZED: u32 = 0x0004;  // Zero-sized type (marker)
```

#### 3. Archetype Table Section

```rust
struct ArchetypeTableHeader {
    archetype_count: u32,     // Number of archetypes
    total_size: u64,          // Total size of this section
}

struct ArchetypeEntry {
    archetype_id: u64,        // Unique archetype identifier
    entity_count: u32,        // Entities in this archetype
    component_count: u16,     // Number of component types
    component_types: [u128],  // Array of component type IDs
    data_offset: u64,         // Offset to entity data for this archetype
    data_size: u64,           // Size of entity data
}
```

#### 4. Entity Data Section

Entities are stored grouped by archetype in Structure-of-Arrays (SoA) layout for cache efficiency:

```rust
struct ArchetypeData {
    // Stable IDs for all entities in this archetype
    stable_ids: [u128; entity_count],
    
    // For each component type in archetype:
    component_data: [ComponentArray],
}

struct ComponentArray {
    type_id: u128,            // Component type
    data_size: u64,           // Total size of component data
    // Tightly packed array of component data
    data: [u8; data_size],    // entity_count * sizeof(Component)
}
```

**Example Layout:**
```
Archetype: [Position, Velocity]
Entities: 1000

Stable IDs:     [UUID1, UUID2, ..., UUID1000]  (16KB)
Position Data:  [pos1, pos2, ..., pos1000]     (8KB if 8 bytes each)
Velocity Data:  [vel1, vel2, ..., vel1000]     (8KB if 8 bytes each)
```

#### 5. Resource Data Section

```rust
struct ResourceSection {
    resource_count: u32,      // Number of resources
    resources: [ResourceEntry],
}

struct ResourceEntry {
    type_id: u128,            // Resource type ID
    data_size: u32,           // Size of resource data
    data: [u8],               // Serialized resource data
}
```

#### 6. Footer (32 bytes)

```rust
struct FileFooter {
    checksum_type: u32,       // Checksum algorithm (CRC32, XXHash, etc.)
    checksum: u64,            // Data checksum
    file_size: u64,           // Total file size for validation
    magic_end: [u8; 4],       // "SCEP" (reverse of header magic)
    reserved: [u8; 12],       // Reserved
}
```

### Serialization Algorithm

```rust
fn serialize_world(world: &World, writer: &mut dyn Write) -> Result<()> {
    // 1. Write header
    let header = build_header(world);
    writer.write_all(&header.to_bytes())?;
    
    // 2. Write type registry
    let type_registry = build_type_registry(world);
    writer.write_all(&type_registry.to_bytes())?;
    
    // 3. Write archetype table
    let archetype_table = build_archetype_table(world);
    writer.write_all(&archetype_table.to_bytes())?;
    
    // 4. Write entity data (grouped by archetype)
    for archetype in world.archetypes() {
        serialize_archetype(archetype, writer)?;
    }
    
    // 5. Write resources
    serialize_resources(world, writer)?;
    
    // 6. Write footer with checksum
    let footer = build_footer(writer.position());
    writer.write_all(&footer.to_bytes())?;
    
    Ok(())
}

fn serialize_archetype(archetype: &Archetype, writer: &mut dyn Write) -> Result<()> {
    // Write stable IDs
    for entity in archetype.entities() {
        let stable_id = entity.stable_id();
        writer.write_all(&stable_id.to_bytes())?;
    }
    
    // Write each component type's data
    for component_type in archetype.component_types() {
        let components = archetype.get_components(component_type);
        for component in components {
            component.serialize(writer)?;
        }
    }
    
    Ok(())
}
```

### Deserialization Algorithm

```rust
fn deserialize_world(reader: &mut dyn Read) -> Result<World> {
    // 1. Read and validate header
    let header = FileHeader::from_reader(reader)?;
    validate_header(&header)?;
    
    // 2. Read type registry
    let type_registry = TypeRegistry::from_reader(reader)?;
    
    // 3. Read archetype table
    let archetype_table = ArchetypeTable::from_reader(reader)?;
    
    // 4. Create world and allocate archetypes
    let mut world = World::new();
    world.reserve_entities(header.entity_count as usize);
    
    // 5. Deserialize entity data
    for archetype_entry in archetype_table.entries() {
        deserialize_archetype(&mut world, archetype_entry, reader, &type_registry)?;
    }
    
    // 6. Deserialize resources
    deserialize_resources(&mut world, reader)?;
    
    // 7. Validate footer
    let footer = FileFooter::from_reader(reader)?;
    validate_footer(&footer)?;
    
    Ok(world)
}

fn deserialize_archetype(
    world: &mut World,
    entry: &ArchetypeEntry,
    reader: &mut dyn Read,
    type_registry: &TypeRegistry,
) -> Result<()> {
    // Read stable IDs
    let mut stable_ids = Vec::with_capacity(entry.entity_count as usize);
    for _ in 0..entry.entity_count {
        let stable_id = StableId::from_reader(reader)?;
        stable_ids.push(stable_id);
    }
    
    // Read component data for each type
    let mut component_data = HashMap::new();
    for type_id in &entry.component_types {
        let type_info = type_registry.get(type_id)?;
        let mut components = Vec::with_capacity(entry.entity_count as usize);
        
        for _ in 0..entry.entity_count {
            let component = (type_info.deserialize_fn)(reader)?;
            components.push(component);
        }
        
        component_data.insert(*type_id, components);
    }
    
    // Create entities with components
    for (i, stable_id) in stable_ids.iter().enumerate() {
        let entity = world.spawn_with_stable_id(*stable_id);
        
        for (type_id, components) in &component_data {
            world.add_component_raw(entity, components[i].clone());
        }
    }
    
    Ok(())
}
```

## Consequences

### Positive

- **High Performance**: Archetype-grouped SoA layout enables fast sequential I/O
- **Cache Friendly**: Matches runtime memory layout for minimal transformation
- **Compact**: Efficient binary encoding with minimal overhead
- **Streaming**: Can load archetypes incrementally for large worlds
- **Validation**: Checksums and magic bytes detect corruption
- **Versioning**: Type versions enable migration (see ADR-008)
- **Portable**: Explicit endianness handling for cross-platform compatibility
- **Extensible**: Reserved fields and flags for future features

### Negative

- **Complexity**: More complex than simple serialization
- **Custom Format**: Requires custom tooling for inspection
- **Rigid Structure**: Changes require version migration
- **Memory Spikes**: May need temporary buffers during load
- **No Random Access**: Must read sequentially (by design)

### Neutral

- **File Size**: Comparable to other binary formats (MessagePack, bincode)
- **Compression**: Optional compression can reduce size further
- **Debugging**: JSON plugin (ADR-006) available for human-readable format

## Alternatives Considered

### Alternative 1: Simple Sequential Format

```
[Entity1: StableID, Component1, Component2, ...]
[Entity2: StableID, Component1, Component3, ...]
...
```

- **Pros**:
  - Simplest to implement
  - Easy to understand
  - Flexible entity structure
- **Cons**:
  - Poor cache locality
  - Slower iteration
  - Doesn't match runtime layout
  - More memory fragmentation
- **Rejected because**: Performance significantly worse than archetype-grouped format

### Alternative 2: JSON-Based Binary (BSON/MessagePack)

- **Pros**:
  - Existing libraries
  - Self-describing format
  - Flexible schema
  - Good tooling
- **Cons**:
  - Slower than custom format
  - More overhead
  - Less control over layout
  - Doesn't leverage archetype structure
- **Rejected because**: Can't achieve performance targets; available as plugin (ADR-006)

### Alternative 3: Database Format (SQLite)

```sql
CREATE TABLE entities (stable_id BLOB PRIMARY KEY);
CREATE TABLE components (entity_id BLOB, type_id BLOB, data BLOB);
```

- **Pros**:
  - Mature technology
  - ACID properties
  - Query capabilities
  - Random access
- **Cons**:
  - Much slower
  - Large overhead
  - Complex integration
  - Overkill for most use cases
- **Rejected because**: Performance inadequate; can be provided as plugin

### Alternative 4: Memory Dump

```rust
// Just dump memory directly
std::fs::write("save.bin", world.as_bytes())?;
```

- **Pros**:
  - Fastest possible
  - Simplest implementation
  - Zero transformation
- **Cons**:
  - Not portable (pointers, alignment)
  - Not versionable
  - Fragile to changes
  - Security issues
  - Can't migrate data
- **Rejected because**: Too fragile; not portable; can't evolve

## Implementation Notes

### Endianness Handling

```rust
// Always use little-endian for portability
fn write_u64(writer: &mut dyn Write, value: u64) -> Result<()> {
    writer.write_all(&value.to_le_bytes())
}

fn read_u64(reader: &mut dyn Read) -> Result<u64> {
    let mut bytes = [0u8; 8];
    reader.read_exact(&mut bytes)?;
    Ok(u64::from_le_bytes(bytes))
}
```

### Checksum Calculation

```rust
use crc32fast::Hasher;

fn calculate_checksum(data: &[u8]) -> u64 {
    let mut hasher = Hasher::new();
    hasher.update(data);
    hasher.finalize() as u64
}
```

### Compression Support

```rust
fn save_compressed(world: &World, path: &Path) -> Result<()> {
    let mut buffer = Vec::new();
    serialize_world(world, &mut buffer)?;
    
    let compressed = zstd::encode_all(&buffer[..], 3)?; // Level 3 compression
    std::fs::write(path, compressed)?;
    
    Ok(())
}
```

### Streaming for Large Worlds

```rust
fn load_streaming(path: &Path) -> Result<World> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    
    // Read header and metadata
    let header = FileHeader::from_reader(&mut reader)?;
    let type_registry = TypeRegistry::from_reader(&mut reader)?;
    let archetype_table = ArchetypeTable::from_reader(&mut reader)?;
    
    let mut world = World::new();
    
    // Load archetypes one at a time (streaming)
    for archetype_entry in archetype_table.entries() {
        deserialize_archetype(&mut world, archetype_entry, &mut reader, &type_registry)?;
        // Archetype data can be dropped after loading
    }
    
    Ok(world)
}
```

### Memory-Mapped I/O (Future Optimization)

```rust
// For very large files, use memory mapping
fn load_mmap(path: &Path) -> Result<World> {
    let file = File::open(path)?;
    let mmap = unsafe { Mmap::map(&file)? };
    
    // Parse directly from mapped memory
    deserialize_world_from_slice(&mmap)
}
```

## Performance Characteristics

### Serialization Performance

**Target**: < 1ms per 1000 entities

**Breakdown** (estimated):
- Header/metadata: ~10μs
- Type registry: ~50μs (one-time)
- Archetype table: ~100μs
- Entity data: ~700μs (dominant cost)
- Resources: ~50μs
- Footer/checksum: ~90μs
- **Total**: ~1ms ✓

**Optimization strategies**:
- Batch writes to reduce syscalls
- Pre-allocate buffers
- Use unsafe for hot paths if needed
- Parallel archetype serialization

### Deserialization Performance

**Target**: < 1ms per 1000 entities

**Breakdown** (estimated):
- Header validation: ~10μs
- Type registry: ~50μs
- Archetype table: ~100μs
- Entity creation: ~600μs
- Component deserialization: ~200μs
- Footer validation: ~40μs
- **Total**: ~1ms ✓

### File Size

**Example**: 10,000 entities with Position (8 bytes) + Velocity (8 bytes)

```
Header:           64 bytes
Type Registry:    ~200 bytes (2 types)
Archetype Table:  ~100 bytes (1 archetype)
Entity Data:      160,000 bytes (10,000 * 16)
  - Stable IDs:   160,000 bytes (10,000 * 16)
  - Position:     80,000 bytes (10,000 * 8)
  - Velocity:     80,000 bytes (10,000 * 8)
Resources:        ~100 bytes
Footer:           32 bytes
Total:            ~320KB (32 bytes per entity)
```

**With compression** (zstd level 3): ~100-150KB (depending on data entropy)

## Testing Strategy

### Unit Tests
- Test each section serialization/deserialization
- Test endianness handling
- Test checksum validation
- Test error conditions

### Integration Tests
- Round-trip tests (save → load → verify)
- Large world tests (1M+ entities)
- Cross-platform compatibility tests
- Corruption detection tests

### Performance Tests
- Benchmark serialization speed
- Benchmark deserialization speed
- Memory usage profiling
- File size measurements

### Compatibility Tests
- Version migration tests (see ADR-008)
- Cross-platform file exchange
- Backward compatibility tests

## Format Evolution

### Version 1.0 (Initial)
- Basic format as specified
- No compression
- No encryption
- Sequential loading only

### Version 1.1 (Planned)
- Optional compression
- Streaming support
- Incremental saves

### Version 2.0 (Future)
- Parallel loading
- Memory-mapped I/O
- Optional encryption
- Delta compression

### Migration Strategy
See ADR-008 for detailed migration strategy.

## Tools and Utilities

### Format Inspector (Future)

```bash
# Inspect binary save file
pecs-inspect game.save

# Output:
# PECS Binary Format v1.0
# Entities: 10,000
# Archetypes: 5
# Component Types: 8
# File Size: 320 KB
# Checksum: Valid
```

### Format Converter (Future)

```bash
# Convert binary to JSON for debugging
pecs-convert game.save game.json --format json

# Convert back
pecs-convert game.json game.save --format binary
```

## References

- [FlatBuffers](https://google.github.io/flatbuffers/) - Inspiration for format design
- [Cap'n Proto](https://capnproto.org/) - Zero-copy serialization
- [MessagePack](https://msgpack.org/) - Efficient binary format
- [Protocol Buffers](https://developers.google.com/protocol-buffers) - Schema evolution
- [Zstandard Compression](https://facebook.github.io/zstd/) - Fast compression
- ADR-001: Dual Entity ID System
- ADR-002: Archetype-Based Storage
- ADR-006: Pluggable Persistence Architecture
- ADR-008: Version Migration Strategy (to be written)
- PRD NFR-1.5: Persistence performance targets
- Phase 2 Week 3-4: Binary Format Implementation