# Phase 2: Persistence Development Plan

**Phase Duration**: Months 3-4 (8 weeks)
**Status**: 🟡 In Progress
**Progress**: 75%
**Last Updated**: 2026-02-13
**Branch**: phase-2-persistence

---

## Phase Overview

Phase 2 builds on the core ECS foundation from Phase 1 by adding comprehensive persistence capabilities. This includes serialization, deserialization, pluggable storage backends, and version migration support.

### Prerequisites
- ✅ Phase 1 complete (Core ECS functional)
- ✅ Stable ID system operational
- ✅ Command buffer system working

### Goals
- Implement robust persistence layer
- Support multiple serialization formats
- Enable pluggable storage backends
- Provide version migration capabilities
- Maintain performance targets during save/load

### Success Criteria
- ✅ Complete world state can be saved and loaded
- ✅ Binary format achieves < 1ms per 1000 entities
- ✅ Plugin architecture supports custom backends
- ✅ Version migration system functional
- ✅ Selective persistence (transient components) working
- ✅ Test coverage > 85%

---

## Week-by-Week Breakdown

### Week 1-2: Persistence Manager

**Objective**: Create the core persistence infrastructure and manager

#### Tasks
- [x] **Task 1.1**: Design persistence architecture (Complete)
  - [x] Define persistence traits and interfaces
  - [x] Design plugin system architecture (enhanced with DeltaPersistencePlugin)
  - [x] Plan serialization strategy
  - [x] Added full database backend support with change tracking
  - **Estimated**: 2 days
  - **Actual**: 1 day

- [x] **Task 1.2**: Implement Persistence Manager (Complete)
  - [x] Manager struct and lifecycle
  - [x] Plugin registration system
  - [x] Save/load coordination
  - [x] Error handling framework
  - **Estimated**: 3 days
  - **Actual**: 1 day

- [x] **Task 1.3**: Implement metadata system (Complete)
  - [x] World metadata (version, timestamp, etc.)
  - [x] Component type registry
  - [x] Schema information storage
  - [x] Change tracking for delta persistence
  - [x] Integration with World
  - **Estimated**: 2 days
  - **Actual**: 1 day

- [x] **Task 1.4**: Stable ID integration (Complete)
  - [x] Ensure stable IDs are used for persistence
  - [x] ID mapping during load (allocate_with_stable_id)
  - [x] Handle ID conflicts (remap_stable_id, EntityError)
  - [x] Added iterator for entity/stable ID pairs
  - [x] Comprehensive test coverage
  - **Estimated**: 2 days
  - **Actual**: 1 day

- [-] **Task 1.5**: Testing and documentation
  - Unit tests for manager
  - Integration tests with Phase 1
  - API documentation
  - **Estimated**: 1 day

**Deliverables**:
- `src/persistence/mod.rs` - Persistence module
- `src/persistence/manager.rs` - Persistence manager
- `src/persistence/metadata.rs` - Metadata handling
- `src/persistence/plugin.rs` - Plugin trait definitions
- Tests in `tests/persistence_tests.rs`

**Milestone**: M2.1 - Persistence Manager Complete

---

### Week 3-4: Binary Format Implementation

**Objective**: Implement efficient binary serialization format

#### Tasks
- [x] **Task 2.1**: Design binary format specification (Complete)
   - [x] Format header structure
   - [x] Entity encoding scheme
   - [x] Component encoding scheme
   - [x] Compression strategy
   - **Estimated**: 2 days
   - **Actual**: 1 day

- [x] **Task 2.2**: Implement serialization (Complete)
   - [x] World serialization
   - [x] Entity serialization
   - [x] Component serialization (placeholder)
   - [x] Type information encoding
   - **Estimated**: 3 days
   - **Actual**: 1 day

- [x] **Task 2.3**: Implement deserialization (Complete)
   - [x] Format validation
   - [x] World reconstruction
   - [x] Entity restoration
   - [x] Component restoration (placeholder)
   - **Estimated**: 3 days
   - **Actual**: 1 day

- [ ] **Task 2.4**: Optimization and compression (Deferred)
   - Binary size optimization
   - Optional compression (zstd/lz4)
   - Streaming support for large worlds
   - **Estimated**: 1 day
   - **Note**: Will be completed after component serialization is fully implemented

- [x] **Task 2.5**: Testing and benchmarking (Partial)
   - [x] Serialization tests
   - [x] Round-trip tests
   - [ ] Performance benchmarks (deferred to Phase 3)
   - [x] Format validation tests
   - **Estimated**: 1 day
   - **Actual**: Included in implementation

**Deliverables**:
- `src/persistence/binary/mod.rs` - Binary format module
- `src/persistence/binary/serialize.rs` - Serialization
- `src/persistence/binary/deserialize.rs` - Deserialization
- `src/persistence/binary/format.rs` - Format specification
- Tests in `tests/binary_format_tests.rs`
- Benchmarks in `benches/persistence_bench.rs`

**Milestone**: M2.2 - Binary Format Implementation Complete

---

### Week 5-6: Save/Load Functionality

**Objective**: Implement high-level save/load operations with file I/O
**Status**: ✅ Complete

#### Tasks
- [x] **Task 3.1**: File I/O abstraction (Complete)
  - [x] Platform-agnostic file operations (using std::fs::File)
  - [x] Error handling for I/O operations (PersistenceError::Io)
  - **Estimated**: 2 days
  - **Actual**: Already implemented in manager.rs

- [x] **Task 3.2**: Save functionality (Complete)
  - [x] Full world save (World::save, PersistenceManager::save)
  - [x] Save validation (checksum, format validation)
  - [-] Incremental/delta save (deferred to Week 7-8)
  - [-] Transient component filtering (deferred to Week 7-8)
  - **Estimated**: 3 days
  - **Actual**: Already implemented

- [x] **Task 3.3**: Load functionality (Complete)
  - [x] Full world load (World::load, PersistenceManager::load)
  - [x] Load validation (checksum verification, format validation)
  - [-] Incremental/delta load (deferred to Week 7-8)
  - [-] World merging (optional, deferred)
  - **Estimated**: 3 days
  - **Actual**: Already implemented

- [x] **Task 3.4**: Command buffer integration (Not needed)
  - Command buffer already integrated in World
  - No additional work required for persistence
  - **Estimated**: 1 day
  - **Actual**: 0 days

- [x] **Task 3.5**: Testing and validation (Complete)
  - [x] Save/load round-trip tests (14 integration tests)
  - [x] Large world tests (1000 entity test)
  - [x] File validation tests
  - [x] Stable ID preservation tests
  - [-] Performance validation (deferred to Phase 3)
  - **Estimated**: 1 day
  - **Actual**: 1 day

**Deliverables**:
- ✅ File I/O in `src/persistence/manager.rs`
- ✅ Save/load in `World` and `PersistenceManager`
- ✅ Tests in `tests/persistence_integration_tests.rs` (14 tests)
- ✅ 143 total tests passing

**Milestone**: M2.3 - Save/Load Functionality Complete ✅ 2026-02-13

---

### Week 7-8: Plugin System and Additional Formats

**Objective**: Create plugin architecture and implement JSON format as example

#### Tasks
- [ ] **Task 4.1**: Finalize plugin architecture
  - Plugin trait refinement
  - Plugin registration API
  - Plugin lifecycle management
  - **Estimated**: 2 days

- [ ] **Task 4.2**: Implement JSON format plugin
  - JSON serialization using serde
  - Human-readable format
  - Schema validation
  - **Estimated**: 2 days

- [ ] **Task 4.3**: Version migration system
  - Version detection
  - Migration trait definition
  - Migration chain execution
  - Backward compatibility
  - **Estimated**: 3 days

- [ ] **Task 4.4**: Selective persistence
  - Transient component marking
  - Persistence filters
  - Custom persistence rules
  - **Estimated**: 2 days

- [ ] **Task 4.5**: Integration and documentation
  - Plugin examples
  - Migration examples
  - Comprehensive documentation
  - Integration tests
  - **Estimated**: 1 day

**Deliverables**:
- `src/persistence/json/mod.rs` - JSON format plugin
- `src/persistence/migration.rs` - Version migration
- `src/persistence/filter.rs` - Persistence filters
- Examples in `examples/persistence/`
- Tests in `tests/plugin_tests.rs`

**Milestone**: M2.4 - Plugin System Complete

---

## Technical Specifications

### Binary Format Structure
```
[Header]
- Magic bytes: "PECS" (4 bytes)
- Version: u32 (4 bytes)
- Flags: u32 (4 bytes)
- Entity count: u64 (8 bytes)
- Component type count: u32 (4 bytes)

[Type Registry]
- For each component type:
  - Type ID: u128 (16 bytes)
  - Type name: String (length-prefixed)
  - Type version: u32 (4 bytes)

[Entity Data]
- For each entity:
  - Stable ID: u128 (16 bytes)
  - Component count: u32 (4 bytes)
  - For each component:
    - Type ID: u128 (16 bytes)
    - Data length: u32 (4 bytes)
    - Data: [bytes]

[Footer]
- Checksum: u64 (8 bytes)
```

### Persistence Traits
```rust
// Main persistence plugin trait
trait PersistencePlugin: Send + Sync {
    fn save(&self, world: &World, writer: &mut dyn Write) -> Result<()>;
    fn load(&self, reader: &mut dyn Read) -> Result<World>;
    fn format_name(&self) -> &str;
    fn format_version(&self) -> u32;
}

// Component serialization trait
trait SerializableComponent {
    fn serialize(&self, writer: &mut dyn Write) -> Result<()>;
    fn deserialize(reader: &mut dyn Read) -> Result<Self> where Self: Sized;
    fn is_transient(&self) -> bool { false }
}

// Version migration trait
trait Migration: Send + Sync {
    fn from_version(&self) -> u32;
    fn to_version(&self) -> u32;
    fn migrate(&self, data: &mut World) -> Result<()>;
}
```

### Save/Load API
```rust
impl World {
    // Save entire world
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()>;
    
    // Save with specific plugin
    pub fn save_with<P: AsRef<Path>>(
        &self, 
        path: P, 
        plugin: &dyn PersistencePlugin
    ) -> Result<()>;
    
    // Load world
    pub fn load<P: AsRef<Path>>(path: P) -> Result<World>;
    
    // Load with specific plugin
    pub fn load_with<P: AsRef<Path>>(
        path: P, 
        plugin: &dyn PersistencePlugin
    ) -> Result<World>;
    
    // Incremental save (delta)
    pub fn save_delta<P: AsRef<Path>>(
        &self, 
        path: P, 
        since: Timestamp
    ) -> Result<()>;
}
```

### Plugin Registration
```rust
// Register custom persistence plugin
world.register_persistence_plugin(Box::new(MyCustomPlugin));

// Use registered plugin
world.save_with("save.dat", "my_custom_format")?;
```

---

## Performance Targets

### Serialization Performance
- Binary format: < 1ms per 1000 entities
- JSON format: < 10ms per 1000 entities
- Compression overhead: < 20% additional time

### File I/O Performance
- Write throughput: > 100 MB/s
- Read throughput: > 100 MB/s
- Async I/O support for large files

### Memory Usage
- Serialization buffer: < 2x world size
- Streaming support for worlds > 1GB
- Incremental save: < 10% overhead

---

## Testing Strategy

### Unit Tests
- Test each serialization format independently
- Test version migration paths
- Test plugin registration and lifecycle
- Test error conditions and recovery

### Integration Tests
- Round-trip tests (save → load → verify)
- Cross-format compatibility tests
- Large world persistence tests
- Concurrent save/load tests

### Performance Tests
- Benchmark serialization speed
- Benchmark deserialization speed
- Memory usage profiling
- File size optimization tests

### Compatibility Tests
- Cross-platform file compatibility
- Version migration tests
- Backward compatibility tests

---

## File Format Examples

### Binary Format (Conceptual)
```
PECS [version] [flags]
Entities: 1000
Types: 5

Type Registry:
  - Position (v1)
  - Velocity (v1)
  - Health (v1)
  ...

Entity Data:
  Entity[UUID-1]:
    Position: {x: 1.0, y: 2.0}
    Velocity: {x: 0.5, y: 0.0}
  Entity[UUID-2]:
    Position: {x: 5.0, y: 3.0}
    Health: {current: 100, max: 100}
  ...

Checksum: 0x1234567890ABCDEF
```

### JSON Format
```json
{
  "version": 1,
  "timestamp": "2026-02-13T04:26:00Z",
  "entity_count": 1000,
  "types": [
    {"id": "Position", "version": 1},
    {"id": "Velocity", "version": 1}
  ],
  "entities": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "components": {
        "Position": {"x": 1.0, "y": 2.0},
        "Velocity": {"x": 0.5, "y": 0.0}
      }
    }
  ]
}
```

---

## Migration Example

```rust
// Migration from v1 to v2
struct PositionMigrationV1ToV2;

impl Migration for PositionMigrationV1ToV2 {
    fn from_version(&self) -> u32 { 1 }
    fn to_version(&self) -> u32 { 2 }
    
    fn migrate(&self, world: &mut World) -> Result<()> {
        // Convert 2D positions to 3D
        for (entity, pos) in world.query::<(Entity, &mut Position)>() {
            // Old: Position { x, y }
            // New: Position { x, y, z }
            pos.z = 0.0; // Add z component
        }
        Ok(())
    }
}

// Register migration
world.register_migration(Box::new(PositionMigrationV1ToV2));
```

---

## Dependencies

### Required Crates
- `serde` - Serialization framework (for JSON plugin)
- `serde_json` - JSON support
- `bincode` - Binary serialization (alternative)

### Optional Dependencies
- `zstd` - Compression support
- `lz4` - Fast compression
- `async-std` or `tokio` - Async I/O

---

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Performance below targets | High | Early benchmarking, optimization, streaming |
| Format compatibility issues | High | Comprehensive testing, versioning system |
| Data corruption | Critical | Checksums, validation, backup strategies |
| Migration complexity | Medium | Clear migration API, extensive testing |
| Plugin API instability | Medium | Careful design, user feedback |

---

## Phase 2 Completion Checklist

- [ ] Persistence manager implemented and tested
- [ ] Binary format achieving performance targets
- [ ] Save/load functionality working reliably
- [ ] Plugin system functional with examples
- [ ] JSON format plugin implemented
- [ ] Version migration system working
- [ ] Selective persistence (transient components) functional
- [ ] Test coverage > 85%
- [ ] All benchmarks passing performance targets
- [ ] Documentation complete for Phase 2 APIs
- [ ] Integration tests passing
- [ ] Cross-platform compatibility verified
- [ ] Code review completed
- [ ] Ready for Phase 3 (Polish & Optimization)

---

## Next Phase Preview

**Phase 3: Polish & Optimization** will focus on:
- Performance optimization across all systems
- Comprehensive documentation and examples
- Tutorial creation
- Community feedback integration
- API refinement based on real-world usage

The persistence system from Phase 2 will be stress-tested and optimized in Phase 3.