# ADR-008: Version Migration Strategy

**Status**: Proposed
**Date**: 2026-02-13
**Deciders**: Development Team
**Related**: ADR-006, ADR-007, PRD FR-4.5, Phase 2 Week 7-8

## Context

As PECS evolves, both the persistence format and component schemas will change:

### Format Evolution Scenarios
- **Format changes**: Binary format improvements (ADR-007)
- **Component schema changes**: Adding/removing/renaming fields
- **Type changes**: Changing component data types
- **Structural changes**: New features, optimizations

### User Requirements
- **Backward compatibility**: Load old save files in new versions
- **Forward compatibility**: Optionally support newer formats in older versions
- **Data preservation**: No data loss during migration
- **Transparency**: Automatic migration when possible
- **Control**: Manual migration for complex cases

### Challenges
- **Breaking changes**: Some changes can't be automatically migrated
- **Performance**: Migration shouldn't significantly slow loading
- **Complexity**: Migration logic can become complex over time
- **Testing**: Need to test all migration paths
- **Documentation**: Users need to understand migration process

Without a clear migration strategy, users face:
- Lost save data when upgrading
- Manual conversion scripts
- Inability to upgrade PECS versions
- Fragmentation across versions

## Decision

We will implement a **multi-layered version migration system** that handles both format-level and component-level migrations through a combination of automatic and manual migration strategies.

### Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│                   Migration System                       │
├─────────────────────────────────────────────────────────┤
│  Format Version Migration (Binary Format)               │
│  ├─ Format v1.0 → v1.1 → v2.0                          │
│  └─ Automatic structure updates                         │
├─────────────────────────────────────────────────────────┤
│  Component Version Migration (Schema Changes)           │
│  ├─ Position v1 → v2 → v3                              │
│  └─ User-defined migration functions                    │
├─────────────────────────────────────────────────────────┤
│  Migration Chain Executor                               │
│  └─ Applies migrations in sequence                      │
└─────────────────────────────────────────────────────────┘
```

### 1. Version Numbering Scheme

#### Format Versions (Binary Format)
```rust
struct FormatVersion {
    major: u16,  // Breaking changes
    minor: u16,  // Backward-compatible additions
}

// Examples:
// 1.0 - Initial release
// 1.1 - Added compression support (backward compatible)
// 2.0 - Changed archetype layout (breaking)
```

#### Component Versions (Schema)
```rust
impl SerializableComponent for Position {
    fn version() -> u32 {
        3  // Current version
    }
}

// Version history:
// v1: { x: f32, y: f32 }
// v2: { x: f32, y: f32, z: f32 }  // Added z
// v3: { x: f64, y: f64, z: f64 }  // Changed precision
```

### 2. Migration Trait System

#### Format Migration Trait

```rust
/// Migrates between binary format versions
pub trait FormatMigration: Send + Sync {
    /// Source format version
    fn from_version(&self) -> FormatVersion;
    
    /// Target format version
    fn to_version(&self) -> FormatVersion;
    
    /// Perform migration on raw data
    fn migrate(&self, data: &[u8]) -> Result<Vec<u8>, MigrationError>;
    
    /// Check if migration is lossy
    fn is_lossy(&self) -> bool { false }
}
```

#### Component Migration Trait

```rust
/// Migrates component data between versions
pub trait ComponentMigration: Send + Sync {
    /// Component type this migration applies to
    fn component_type_id(&self) -> ComponentTypeId;
    
    /// Source component version
    fn from_version(&self) -> u32;
    
    /// Target component version
    fn to_version(&self) -> u32;
    
    /// Migrate component data
    fn migrate(&self, old_data: &[u8]) -> Result<Vec<u8>, MigrationError>;
    
    /// Provide default value if migration fails
    fn default_value(&self) -> Option<Vec<u8>> { None }
}
```

### 3. Migration Registry

```rust
pub struct MigrationRegistry {
    format_migrations: Vec<Box<dyn FormatMigration>>,
    component_migrations: HashMap<ComponentTypeId, Vec<Box<dyn ComponentMigration>>>,
}

impl MigrationRegistry {
    /// Register a format migration
    pub fn register_format_migration(&mut self, migration: Box<dyn FormatMigration>) {
        self.format_migrations.push(migration);
    }
    
    /// Register a component migration
    pub fn register_component_migration(&mut self, migration: Box<dyn ComponentMigration>) {
        let type_id = migration.component_type_id();
        self.component_migrations
            .entry(type_id)
            .or_insert_with(Vec::new)
            .push(migration);
    }
    
    /// Find migration path from version A to B
    pub fn find_migration_path(
        &self,
        from: FormatVersion,
        to: FormatVersion,
    ) -> Option<Vec<&dyn FormatMigration>> {
        // Use graph search to find shortest path
        self.find_path_dijkstra(from, to)
    }
    
    /// Get component migrations for a type
    pub fn get_component_migrations(
        &self,
        type_id: ComponentTypeId,
        from_version: u32,
        to_version: u32,
    ) -> Vec<&dyn ComponentMigration> {
        // Return ordered chain of migrations
        self.build_migration_chain(type_id, from_version, to_version)
    }
}
```

### 4. Migration Execution

#### Automatic Migration

```rust
impl World {
    /// Load with automatic migration
    pub fn load_with_migration<P: AsRef<Path>>(
        path: P,
        registry: &MigrationRegistry,
    ) -> Result<World, MigrationError> {
        let mut file = File::open(path)?;
        
        // Read format version from header
        let file_version = read_format_version(&mut file)?;
        let current_version = CURRENT_FORMAT_VERSION;
        
        if file_version == current_version {
            // No migration needed
            return World::load(path);
        }
        
        // Find migration path
        let migrations = registry
            .find_migration_path(file_version, current_version)
            .ok_or(MigrationError::NoMigrationPath)?;
        
        // Apply format migrations
        let mut data = std::fs::read(path)?;
        for migration in migrations {
            data = migration.migrate(&data)?;
        }
        
        // Load migrated data
        let mut cursor = Cursor::new(data);
        let world = deserialize_world(&mut cursor)?;
        
        // Apply component migrations
        migrate_components(world, registry)
    }
}

fn migrate_components(
    mut world: World,
    registry: &MigrationRegistry,
) -> Result<World, MigrationError> {
    // For each component type in the world
    for (type_id, stored_version) in world.component_versions() {
        let current_version = world.get_current_version(type_id);
        
        if stored_version == current_version {
            continue; // No migration needed
        }
        
        // Get migration chain
        let migrations = registry.get_component_migrations(
            type_id,
            stored_version,
            current_version,
        );
        
        // Apply migrations to all components of this type
        for entity in world.query_with_type(type_id) {
            let mut data = world.get_component_raw(entity, type_id)?;
            
            for migration in &migrations {
                data = migration.migrate(&data)?;
            }
            
            world.set_component_raw(entity, type_id, data)?;
        }
    }
    
    Ok(world)
}
```

#### Manual Migration

```rust
/// For complex migrations that need custom logic
pub struct MigrationBuilder {
    world: World,
    registry: MigrationRegistry,
}

impl MigrationBuilder {
    /// Load old format
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, MigrationError> {
        let world = World::load_raw(path)?;
        Ok(Self {
            world,
            registry: MigrationRegistry::new(),
        })
    }
    
    /// Apply custom transformation
    pub fn transform<F>(mut self, f: F) -> Self
    where
        F: FnOnce(&mut World),
    {
        f(&mut self.world);
        self
    }
    
    /// Migrate specific component type
    pub fn migrate_component<T, F>(mut self, f: F) -> Self
    where
        T: Component,
        F: Fn(&T) -> T,
    {
        for (entity, component) in self.world.query::<(Entity, &T)>() {
            let migrated = f(component);
            self.world.set_component(entity, migrated);
        }
        self
    }
    
    /// Finalize and return migrated world
    pub fn finish(self) -> World {
        self.world
    }
}

// Usage:
let world = MigrationBuilder::load("old_save.bin")?
    .transform(|world| {
        // Custom migration logic
        for entity in world.entities() {
            // Complex transformation
        }
    })
    .migrate_component::<Position, _>(|pos| {
        // Migrate Position v1 to v2
        Position { x: pos.x, y: pos.y, z: 0.0 }
    })
    .finish();
```

### 5. Migration Examples

#### Example 1: Format Version Migration (1.0 → 1.1)

```rust
struct FormatV1ToV1_1;

impl FormatMigration for FormatV1ToV1_1 {
    fn from_version(&self) -> FormatVersion {
        FormatVersion { major: 1, minor: 0 }
    }
    
    fn to_version(&self) -> FormatVersion {
        FormatVersion { major: 1, minor: 1 }
    }
    
    fn migrate(&self, data: &[u8]) -> Result<Vec<u8>, MigrationError> {
        // v1.1 added compression flag to header
        let mut migrated = Vec::with_capacity(data.len() + 4);
        
        // Copy header (first 60 bytes)
        migrated.extend_from_slice(&data[0..60]);
        
        // Add compression flag (4 bytes, set to 0)
        migrated.extend_from_slice(&[0, 0, 0, 0]);
        
        // Copy rest of data
        migrated.extend_from_slice(&data[60..]);
        
        Ok(migrated)
    }
}
```

#### Example 2: Component Schema Migration (Position v1 → v2)

```rust
// v1: 2D position
struct PositionV1 {
    x: f32,
    y: f32,
}

// v2: 3D position
struct PositionV2 {
    x: f32,
    y: f32,
    z: f32,
}

struct PositionV1ToV2;

impl ComponentMigration for PositionV1ToV2 {
    fn component_type_id(&self) -> ComponentTypeId {
        ComponentTypeId::of::<PositionV2>()
    }
    
    fn from_version(&self) -> u32 { 1 }
    fn to_version(&self) -> u32 { 2 }
    
    fn migrate(&self, old_data: &[u8]) -> Result<Vec<u8>, MigrationError> {
        // Deserialize v1
        let mut cursor = Cursor::new(old_data);
        let x = read_f32(&mut cursor)?;
        let y = read_f32(&mut cursor)?;
        
        // Create v2 with z = 0.0
        let mut result = Vec::new();
        result.extend_from_slice(&x.to_le_bytes());
        result.extend_from_slice(&y.to_le_bytes());
        result.extend_from_slice(&0.0f32.to_le_bytes());
        
        Ok(result)
    }
}
```

#### Example 3: Complex Migration with Data Loss

```rust
// v2: High precision
struct PositionV2 {
    x: f64,
    y: f64,
    z: f64,
}

// v3: Low precision (lossy migration)
struct PositionV3 {
    x: f32,
    y: f32,
    z: f32,
}

struct PositionV2ToV3;

impl ComponentMigration for PositionV2ToV3 {
    fn component_type_id(&self) -> ComponentTypeId {
        ComponentTypeId::of::<PositionV3>()
    }
    
    fn from_version(&self) -> u32 { 2 }
    fn to_version(&self) -> u32 { 3 }
    
    fn migrate(&self, old_data: &[u8]) -> Result<Vec<u8>, MigrationError> {
        let mut cursor = Cursor::new(old_data);
        let x = read_f64(&mut cursor)? as f32;  // Precision loss
        let y = read_f64(&mut cursor)? as f32;
        let z = read_f64(&mut cursor)? as f32;
        
        let mut result = Vec::new();
        result.extend_from_slice(&x.to_le_bytes());
        result.extend_from_slice(&y.to_le_bytes());
        result.extend_from_slice(&z.to_le_bytes());
        
        Ok(result)
    }
    
    fn is_lossy(&self) -> bool { true }
}
```

### 6. Migration Validation

```rust
pub struct MigrationValidator {
    registry: MigrationRegistry,
}

impl MigrationValidator {
    /// Validate that all migrations are registered
    pub fn validate_completeness(&self) -> Result<(), ValidationError> {
        // Check for gaps in migration chains
        for type_id in self.registry.component_types() {
            let versions = self.registry.get_versions(type_id);
            for i in 0..versions.len() - 1 {
                if !self.has_migration(type_id, versions[i], versions[i + 1]) {
                    return Err(ValidationError::MissingMigration {
                        type_id,
                        from: versions[i],
                        to: versions[i + 1],
                    });
                }
            }
        }
        Ok(())
    }
    
    /// Test migration round-trip
    pub fn test_migration<T: Component>(
        &self,
        component: &T,
        from_version: u32,
        to_version: u32,
    ) -> Result<(), ValidationError> {
        // Serialize at old version
        let old_data = serialize_at_version(component, from_version)?;
        
        // Migrate
        let migrations = self.registry.get_component_migrations(
            ComponentTypeId::of::<T>(),
            from_version,
            to_version,
        );
        
        let mut data = old_data;
        for migration in migrations {
            data = migration.migrate(&data)?;
        }
        
        // Deserialize at new version
        let migrated: T = deserialize_at_version(&data, to_version)?;
        
        // Validate (user-provided validation function)
        validate_migration(component, &migrated)?;
        
        Ok(())
    }
}
```

## Consequences

### Positive

- **Backward Compatibility**: Users can load old saves in new versions
- **Flexibility**: Supports both automatic and manual migrations
- **Extensibility**: Easy to add new migrations
- **Type Safety**: Compile-time checks for migration implementations
- **Transparency**: Automatic migration when possible
- **Control**: Manual migration for complex cases
- **Validation**: Built-in validation and testing tools
- **Documentation**: Clear migration history

### Negative

- **Complexity**: Migration system adds significant complexity
- **Maintenance**: Must maintain migration code indefinitely
- **Testing**: Exponential growth in test cases (all version combinations)
- **Performance**: Migration adds overhead to loading
- **Storage**: Must store version information in files
- **Documentation**: Users need to understand migration process

### Neutral

- **Migration Chain**: Long chains may be slow but are rare
- **Lossy Migrations**: Some migrations lose data (documented)
- **Breaking Changes**: Major versions may not support migration

## Alternatives Considered

### Alternative 1: No Migration Support

- **Pros**:
  - Simplest implementation
  - No maintenance burden
  - Fastest loading
- **Cons**:
  - Users lose data on upgrade
  - Prevents PECS evolution
  - Poor user experience
- **Rejected because**: Unacceptable for production use

### Alternative 2: Single-Step Migration Only

```rust
// Only support N-1 → N migration
impl Component {
    fn migrate_from_previous(&self, old_data: &[u8]) -> Self;
}
```

- **Pros**:
  - Simpler than full chain
  - Covers most cases
- **Cons**:
  - Can't skip versions
  - Must migrate through all versions
  - Slow for old saves
- **Rejected because**: Too limiting; chain approach more flexible

### Alternative 3: Serde-Based Versioning

```rust
#[derive(Serialize, Deserialize)]
#[serde(tag = "version")]
enum Position {
    V1 { x: f32, y: f32 },
    V2 { x: f32, y: f32, z: f32 },
}
```

- **Pros**:
  - Leverages serde ecosystem
  - Automatic deserialization
- **Cons**:
  - Less control over migration
  - Performance overhead
  - Doesn't work with binary format
  - Enum overhead in memory
- **Rejected because**: Insufficient control; doesn't fit binary format design

### Alternative 4: Database-Style Migrations

```rust
// SQL-like migration scripts
migration_001_add_z_to_position.sql
migration_002_change_precision.sql
```

- **Pros**:
  - Familiar to database developers
  - Clear migration history
- **Cons**:
  - Overkill for ECS
  - Requires DSL or scripting
  - Complex implementation
- **Rejected because**: Too heavyweight; trait-based approach sufficient

## Implementation Notes

### Phase 2 Implementation (Week 7-8)

```rust
// Week 7: Core migration system
- Define migration traits
- Implement migration registry
- Basic format migration support

// Week 8: Component migrations
- Component migration trait
- Migration chain execution
- Validation tools
- Documentation and examples
```

### Migration Registration

```rust
// Global registry (initialized at startup)
pub fn register_migrations() {
    let mut registry = MIGRATION_REGISTRY.lock().unwrap();
    
    // Format migrations
    registry.register_format_migration(Box::new(FormatV1ToV1_1));
    registry.register_format_migration(Box::new(FormatV1_1ToV2));
    
    // Component migrations
    registry.register_component_migration(Box::new(PositionV1ToV2));
    registry.register_component_migration(Box::new(PositionV2ToV3));
}
```

### Best Practices

1. **Always provide migrations**: Don't break backward compatibility
2. **Test migrations**: Validate with real data
3. **Document changes**: Explain what changed and why
4. **Version carefully**: Use semantic versioning
5. **Avoid lossy migrations**: Preserve data when possible
6. **Provide defaults**: Handle missing data gracefully

## Testing Strategy

### Unit Tests
- Test each migration independently
- Test migration chains
- Test error conditions
- Test validation

### Integration Tests
- Test loading old saves
- Test migration paths
- Test lossy migrations
- Test manual migrations

### Regression Tests
- Keep old save files as test fixtures
- Test loading in each new version
- Verify data integrity after migration

## Documentation Requirements

### User Guide
- Migration overview
- How to handle breaking changes
- Manual migration examples
- Troubleshooting guide

### Developer Guide
- How to add migrations
- Migration best practices
- Testing migrations
- Version numbering

## Future Enhancements

### Automatic Migration Generation
```rust
// Derive macro to generate simple migrations
#[derive(Component, Migrate)]
#[migrate(from = "PositionV1")]
struct PositionV2 {
    x: f32,
    y: f32,
    #[migrate(default = "0.0")]
    z: f32,
}
```

### Migration Analytics
- Track which migrations are used
- Identify problematic migrations
- Optimize common migration paths

### Parallel Migration
- Migrate components in parallel
- Faster for large worlds

## References

- [Semantic Versioning](https://semver.org/)
- [Database Migrations](https://en.wikipedia.org/wiki/Schema_migration)
- [Protocol Buffers Schema Evolution](https://developers.google.com/protocol-buffers/docs/proto3#updating)
- [Rust API Evolution](https://rust-lang.github.io/rfcs/1105-api-evolution.html)
- ADR-006: Pluggable Persistence Architecture
- ADR-007: Binary Format Specification
- PRD FR-4.5: Version migration support
- Phase 2 Week 7-8: Version Migration System