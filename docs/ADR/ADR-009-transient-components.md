# ADR-009: Transient Component Marking

**Status**: Proposed
**Date**: 2026-02-13
**Deciders**: Development Team
**Related**: ADR-006, ADR-007, PRD FR-4.6, Phase 2 Week 7-8

## Context

Not all component data should be persisted when saving world state. Some components represent:

### Transient Data Categories
- **Runtime state**: Frame counters, delta times, temporary flags
- **Cached values**: Derived data that can be recalculated
- **External references**: File handles, network connections, GPU resources
- **Debug information**: Profiling data, debug visualizations
- **Platform-specific data**: OS handles, pointers, platform resources

### User Requirements
- **Selective persistence**: Choose which components to save
- **Performance**: Avoid serializing unnecessary data
- **Correctness**: Prevent serialization of non-serializable data
- **Flexibility**: Different persistence rules for different contexts
- **Safety**: Compile-time prevention of invalid persistence

### Challenges
- **Granularity**: Per-component vs per-instance marking
- **Defaults**: Should components be persistent by default?
- **Discovery**: How do users know what's transient?
- **Validation**: Catching serialization of transient data
- **Reconstruction**: Handling missing components on load

Without transient marking:
- Save files bloated with unnecessary data
- Slower save/load operations
- Risk of serializing invalid data (pointers, handles)
- No way to exclude debug/temporary components

## Decision

We will implement a **multi-level transient marking system** that supports both type-level and instance-level transient marking with compile-time and runtime enforcement.

### Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│              Transient Marking System                    │
├─────────────────────────────────────────────────────────┤
│  Type-Level Marking (Compile-Time)                      │
│  ├─ Marker trait: TransientComponent                    │
│  └─ Automatic exclusion from persistence                │
├─────────────────────────────────────────────────────────┤
│  Instance-Level Marking (Runtime)                       │
│  ├─ is_persistent() method                              │
│  └─ Dynamic filtering during serialization              │
├─────────────────────────────────────────────────────────┤
│  Persistence Filters                                     │
│  └─ Custom filtering logic                              │
└─────────────────────────────────────────────────────────┘
```

### 1. Type-Level Transient Marking

#### Marker Trait Approach

```rust
/// Marker trait for components that should never be persisted
pub trait TransientComponent: Component {}

// Example: Frame counter is always transient
#[derive(Debug, Clone)]
pub struct FrameCounter {
    pub count: u64,
}

impl Component for FrameCounter {}
impl TransientComponent for FrameCounter {}

// Compile-time check: TransientComponent cannot implement SerializableComponent
impl SerializableComponent for FrameCounter {
    // This will fail to compile if TransientComponent is implemented
    fn serialize(&self, _writer: &mut dyn Write) -> Result<(), PersistenceError> {
        compile_error!("TransientComponent cannot be serialized")
    }
}
```

#### Attribute-Based Marking

```rust
/// Derive macro for automatic transient marking
#[derive(Component, Transient)]
pub struct DebugInfo {
    pub fps: f32,
    pub frame_time: f32,
}

// Expands to:
impl TransientComponent for DebugInfo {}
```

### 2. Instance-Level Transient Marking

#### Dynamic Persistence Control

```rust
pub trait SerializableComponent: Component {
    // ... other methods ...
    
    /// Check if this specific instance should be persisted
    /// Default: true (persistent)
    fn is_persistent(&self) -> bool {
        true
    }
}

// Example: Cache that's only persistent if valid
#[derive(Debug, Clone)]
pub struct PathfindingCache {
    pub path: Vec<Position>,
    pub is_valid: bool,
}

impl SerializableComponent for PathfindingCache {
    fn is_persistent(&self) -> bool {
        // Only persist if cache is valid
        self.is_valid
    }
    
    // ... serialize/deserialize methods ...
}
```

### 3. Persistence Filters

#### Filter Trait

```rust
/// Custom filter for persistence decisions
pub trait PersistenceFilter: Send + Sync {
    /// Check if component should be persisted
    fn should_persist(
        &self,
        entity: Entity,
        component_type: ComponentTypeId,
        component: &dyn Any,
    ) -> bool;
}

// Example: Only persist components on entities with specific tag
pub struct TaggedOnlyFilter {
    required_tag: ComponentTypeId,
}

impl PersistenceFilter for TaggedOnlyFilter {
    fn should_persist(
        &self,
        entity: Entity,
        _component_type: ComponentTypeId,
        _component: &dyn Any,
    ) -> bool {
        // Only persist if entity has required tag
        entity.has_component(self.required_tag)
    }
}
```

#### Filter Application

```rust
impl World {
    /// Save with custom filter
    pub fn save_filtered<P, F>(
        &self,
        path: P,
        filter: F,
    ) -> Result<(), PersistenceError>
    where
        P: AsRef<Path>,
        F: PersistenceFilter,
    {
        let plugin = BinaryPlugin::with_filter(filter);
        self.save_with(path, &plugin)
    }
}

// Usage:
world.save_filtered(
    "game.save",
    TaggedOnlyFilter { required_tag: ComponentTypeId::of::<Persistent>() }
)?;
```

### 4. Built-in Filters

```rust
/// Filter that excludes all transient components
pub struct NoTransientFilter;

impl PersistenceFilter for NoTransientFilter {
    fn should_persist(
        &self,
        _entity: Entity,
        component_type: ComponentTypeId,
        _component: &dyn Any,
    ) -> bool {
        !is_transient_type(component_type)
    }
}

/// Filter that only persists specific component types
pub struct WhitelistFilter {
    allowed_types: HashSet<ComponentTypeId>,
}

impl PersistenceFilter for WhitelistFilter {
    fn should_persist(
        &self,
        _entity: Entity,
        component_type: ComponentTypeId,
        _component: &dyn Any,
    ) -> bool {
        self.allowed_types.contains(&component_type)
    }
}

/// Filter that excludes specific component types
pub struct BlacklistFilter {
    excluded_types: HashSet<ComponentTypeId>,
}

impl PersistenceFilter for BlacklistFilter {
    fn should_persist(
        &self,
        _entity: Entity,
        component_type: ComponentTypeId,
        _component: &dyn Any,
    ) -> bool {
        !self.excluded_types.contains(&component_type)
    }
}

/// Composite filter (AND/OR logic)
pub struct CompositeFilter {
    filters: Vec<Box<dyn PersistenceFilter>>,
    mode: FilterMode,
}

pub enum FilterMode {
    All,  // All filters must pass (AND)
    Any,  // Any filter must pass (OR)
}
```

### 5. Serialization Integration

```rust
fn serialize_world_with_filter(
    world: &World,
    writer: &mut dyn Write,
    filter: &dyn PersistenceFilter,
) -> Result<(), PersistenceError> {
    // ... write header and metadata ...
    
    for archetype in world.archetypes() {
        for entity in archetype.entities() {
            // Write stable ID
            writer.write_all(&entity.stable_id().to_bytes())?;
            
            // Filter and write components
            for (component_type, component) in entity.components() {
                // Check type-level transient
                if is_transient_type(component_type) {
                    continue;
                }
                
                // Check instance-level transient
                if let Some(serializable) = component.as_serializable() {
                    if !serializable.is_persistent() {
                        continue;
                    }
                }
                
                // Check custom filter
                if !filter.should_persist(entity, component_type, component) {
                    continue;
                }
                
                // Serialize component
                serialize_component(component_type, component, writer)?;
            }
        }
    }
    
    Ok(())
}
```

### 6. Reconstruction on Load

```rust
/// Trait for components that need reconstruction after load
pub trait ReconstructibleComponent: Component {
    /// Reconstruct transient data after loading
    fn reconstruct(&mut self, world: &World, entity: Entity);
}

// Example: Rebuild cache after load
impl ReconstructibleComponent for PathfindingCache {
    fn reconstruct(&mut self, world: &World, entity: Entity) {
        if let Some(position) = world.get_component::<Position>(entity) {
            // Rebuild pathfinding cache
            self.path = calculate_path(position);
            self.is_valid = true;
        }
    }
}

// Called automatically after deserialization
fn reconstruct_world(mut world: World) -> World {
    for entity in world.entities() {
        for component in entity.components_mut() {
            if let Some(reconstructible) = component.as_reconstructible_mut() {
                reconstructible.reconstruct(&world, entity);
            }
        }
    }
    world
}
```

## Consequences

### Positive

- **Smaller Save Files**: Exclude unnecessary data
- **Faster Save/Load**: Less data to serialize
- **Safety**: Prevent serialization of invalid data
- **Flexibility**: Multiple levels of control
- **Correctness**: Compile-time checks for type-level transient
- **Extensibility**: Custom filters for complex scenarios
- **Reconstruction**: Automatic rebuilding of transient data

### Negative

- **Complexity**: Multiple marking mechanisms to understand
- **Boilerplate**: Need to implement is_persistent() for dynamic cases
- **Runtime Overhead**: Filter checks during serialization
- **Documentation**: Need to explain when to use each approach
- **Testing**: Must test all filter combinations

### Neutral

- **Default Behavior**: Components persistent by default (safe choice)
- **Migration**: Existing code works without changes
- **Performance**: Filter overhead negligible compared to I/O

## Alternatives Considered

### Alternative 1: Attribute-Only Marking

```rust
#[derive(Component)]
#[persistent(false)]  // Mark as transient
pub struct FrameCounter {
    pub count: u64,
}
```

- **Pros**:
  - Simple, declarative
  - Clear at definition site
  - No runtime overhead
- **Cons**:
  - No instance-level control
  - No dynamic filtering
  - Less flexible
- **Rejected because**: Too inflexible; can't handle dynamic cases

### Alternative 2: Opt-In Persistence

```rust
// Only components marked as persistent are saved
#[derive(Component, Persistent)]
pub struct Position { x: f32, y: f32 }
```

- **Pros**:
  - Explicit about what's saved
  - Safer default (nothing saved)
  - Clear intent
- **Cons**:
  - More boilerplate
  - Easy to forget marking
  - Breaks existing code
  - Annoying for common case
- **Rejected because**: Opt-out is more ergonomic; most components should persist

### Alternative 3: Separate Transient Storage

```rust
// Transient components stored separately
world.add_transient_component(entity, FrameCounter { count: 0 });
```

- **Pros**:
  - Clear separation
  - No filtering needed
  - Simple implementation
- **Cons**:
  - Duplicate API surface
  - Confusing for users
  - Can't query both together
  - More complex queries
- **Rejected because**: Adds unnecessary complexity; filtering is cleaner

### Alternative 4: Serialization Trait Only

```rust
// Only components implementing Serialize are persisted
impl Serialize for Position { /* ... */ }
// FrameCounter doesn't implement Serialize, so not persisted
```

- **Pros**:
  - Leverages existing trait
  - Simple rule
  - No new concepts
- **Cons**:
  - No instance-level control
  - Confusing (Serialize used for other purposes)
  - Can't have non-persistent serializable components
- **Rejected because**: Conflates serialization capability with persistence policy

## Implementation Notes

### Phase 2 Implementation (Week 7-8)

```rust
// Week 7: Basic transient marking
- Define TransientComponent trait
- Implement type-level checking
- Basic filtering in serialization

// Week 8: Advanced features
- Instance-level is_persistent()
- Custom filter system
- Reconstruction support
- Documentation and examples
```

### Common Patterns

#### Pattern 1: Debug Components

```rust
#[derive(Component, Transient)]
pub struct DebugVisualization {
    pub lines: Vec<Line>,
    pub color: Color,
}
```

#### Pattern 2: Cached Data

```rust
#[derive(Component)]
pub struct NavigationMesh {
    pub mesh: Vec<Triangle>,
    pub dirty: bool,
}

impl SerializableComponent for NavigationMesh {
    fn is_persistent(&self) -> bool {
        !self.dirty  // Only persist if clean
    }
    
    // ... other methods ...
}

impl ReconstructibleComponent for NavigationMesh {
    fn reconstruct(&mut self, world: &World, entity: Entity) {
        if self.dirty {
            self.rebuild(world, entity);
        }
    }
}
```

#### Pattern 3: External Resources

```rust
#[derive(Component, Transient)]
pub struct TextureHandle {
    pub handle: GpuTextureHandle,  // Can't serialize GPU handle
}

// Companion component for persistence
#[derive(Component)]
pub struct TexturePath {
    pub path: String,  // Serialize path instead
}

impl ReconstructibleComponent for TextureHandle {
    fn reconstruct(&mut self, world: &World, entity: Entity) {
        if let Some(path) = world.get_component::<TexturePath>(entity) {
            self.handle = load_texture(&path.path);
        }
    }
}
```

#### Pattern 4: Conditional Persistence

```rust
pub struct SaveGameFilter;

impl PersistenceFilter for SaveGameFilter {
    fn should_persist(
        &self,
        entity: Entity,
        component_type: ComponentTypeId,
        _component: &dyn Any,
    ) -> bool {
        // Don't save UI components
        if component_type == ComponentTypeId::of::<UiElement>() {
            return false;
        }
        
        // Don't save temporary effects
        if component_type == ComponentTypeId::of::<ParticleEffect>() {
            return false;
        }
        
        true
    }
}
```

### API Examples

```rust
// Example 1: Basic usage (default behavior)
world.save("game.save")?;  // Saves all persistent components

// Example 2: Exclude transient components explicitly
world.save_filtered("game.save", NoTransientFilter)?;

// Example 3: Whitelist specific components
let filter = WhitelistFilter {
    allowed_types: hashset![
        ComponentTypeId::of::<Position>(),
        ComponentTypeId::of::<Health>(),
    ],
};
world.save_filtered("minimal.save", filter)?;

// Example 4: Composite filter
let filter = CompositeFilter {
    filters: vec![
        Box::new(NoTransientFilter),
        Box::new(TaggedOnlyFilter { required_tag: ComponentTypeId::of::<SaveMe>() }),
    ],
    mode: FilterMode::All,
};
world.save_filtered("tagged.save", filter)?;

// Example 5: Custom filter
struct PlayerOnlyFilter;
impl PersistenceFilter for PlayerOnlyFilter {
    fn should_persist(&self, entity: Entity, _: ComponentTypeId, _: &dyn Any) -> bool {
        entity.has_component::<Player>()
    }
}
world.save_filtered("player.save", PlayerOnlyFilter)?;
```

## Performance Considerations

### Filtering Overhead

```rust
// Benchmark results (estimated):
// - Type-level check: ~1ns (hash lookup)
// - Instance-level check: ~5ns (virtual call)
// - Custom filter: ~10-50ns (depends on logic)
// 
// For 10,000 entities with 5 components each:
// - Total filter overhead: ~2.5ms
// - I/O time: ~10ms
// - Overhead: ~20% (acceptable)
```

### Optimization Strategies

```rust
// Cache filter results per archetype
struct CachedFilter {
    filter: Box<dyn PersistenceFilter>,
    cache: HashMap<ArchetypeId, Vec<bool>>,  // Per-component results
}

// Batch filtering
fn filter_archetype(
    archetype: &Archetype,
    filter: &dyn PersistenceFilter,
) -> Vec<bool> {
    archetype.component_types()
        .iter()
        .map(|&type_id| {
            // Check once per type, not per entity
            filter.should_persist(Entity::DUMMY, type_id, &())
        })
        .collect()
}
```

## Testing Strategy

### Unit Tests
- Test type-level transient marking
- Test instance-level is_persistent()
- Test each built-in filter
- Test filter composition

### Integration Tests
- Test save/load with transient components
- Test reconstruction after load
- Test filter combinations
- Test edge cases (all transient, none transient)

### Performance Tests
- Benchmark filtering overhead
- Compare save times with/without filtering
- Memory usage with different filters

## Documentation Requirements

### User Guide
- When to use transient marking
- Type-level vs instance-level
- Custom filter examples
- Reconstruction patterns
- Best practices

### API Documentation
- TransientComponent trait
- PersistenceFilter trait
- Built-in filters
- Reconstruction trait

## Future Enhancements

### Automatic Transient Detection

```rust
// Detect non-serializable types automatically
#[derive(Component)]
pub struct FileHandle {
    handle: std::fs::File,  // Automatically marked transient
}
```

### Transient Validation

```rust
// Compile-time check for common mistakes
#[derive(Component)]
pub struct BadComponent {
    #[transient]  // Mark field as transient
    cache: Vec<u8>,
    data: String,
}
```

### Lazy Reconstruction

```rust
// Reconstruct on first access, not on load
#[derive(Component)]
#[reconstruct(lazy)]
pub struct ExpensiveCache {
    data: Option<Vec<u8>>,
}
```

## References

- [Serde Skip Serializing](https://serde.rs/field-attrs.html#skip_serializing)
- [Unity Serialization](https://docs.unity3d.com/Manual/script-Serialization.html)
- [Unreal Property Specifiers](https://docs.unrealengine.com/en-US/ProgrammingAndScripting/GameplayArchitecture/Properties/Specifiers/)
- ADR-006: Pluggable Persistence Architecture
- ADR-007: Binary Format Specification
- PRD FR-4.6: Selective persistence
- Phase 2 Week 7-8: Selective Persistence Implementation