# PECS Enhancement Plan

## Overview

This document outlines the enhancements needed to make PECS (Persistent Entity Component System) viable for the Wyldlands project. PECS is currently a minimal ECS implementation that needs additional features to match our requirements.

## Current PECS Capabilities

### ✅ What PECS Has:
1. **Dual ID System**: Fast EntityId (64-bit) + Persistent StableId (128-bit UUID)
2. **Entity Management**: Spawn, despawn, lifecycle tracking
3. **Archetype Storage**: Cache-friendly component storage
4. **Basic Persistence**: Save/load world state (binary and JSON)
5. **Command Buffers**: Thread-safe deferred operations

### ❌ What PECS Lacks:
1. **Component Derive Macro**: Must manually implement `Component` trait
2. **Bundle System**: No convenient multi-component spawn syntax
3. **Direct Component Access**: No `world.get::<T>(entity)` or `world.insert::<T>(entity, component)`
4. **Query System**: No ergonomic query API like `world.query::<(&Name, &Health)>()`
5. **Component Mutation**: No `world.get_mut::<T>(entity)`
6. **Entity Builder Completion**: Builder pattern exists but incomplete

## Required Enhancements

### Priority 1: Essential Features (Blocking Migration)

#### 1.1 Component Derive Macro
**Location**: `pecs/pecs_derive/` (new crate)

**Implementation**:
```rust
// Usage:
#[derive(Component)]
struct Position { x: f32, y: f32 }

// Expands to:
impl pecs::Component for Position {}
```

**Benefits**:
- Reduces boilerplate
- Matches Bevy/hecs ergonomics
- Makes migration easier

**Effort**: 2-4 hours

---

#### 1.2 Bundle System
**Location**: `pecs/src/bundle.rs`

**Implementation**:
```rust
pub trait Bundle: 'static {
    fn component_types(&self) -> Vec<ComponentTypeId>;
    fn insert_into_archetype(self, archetype: &mut Archetype, entity: EntityId);
}

// Auto-implement for tuples
impl<T: Component> Bundle for T { /* ... */ }
impl<T1: Component, T2: Component> Bundle for (T1, T2) { /* ... */ }
// ... up to 16-tuple
```

**Usage**:
```rust
// Before (current):
let entity = world.spawn()
    .with(Position { x: 0.0, y: 0.0 })
    .with(Velocity { x: 1.0, y: 0.0 })
    .id();

// After (with Bundle):
let entity = world.spawn_bundle((
    Position { x: 0.0, y: 0.0 },
    Velocity { x: 1.0, y: 0.0 },
));
```

**Benefits**:
- Ergonomic entity creation
- Type-safe component bundles
- Matches industry standard APIs

**Effort**: 4-6 hours

---

#### 1.3 Direct Component Access
**Location**: `pecs/src/world.rs`

**Implementation**:
```rust
impl World {
    /// Get a component reference
    pub fn get<T: Component>(&self, entity: EntityId) -> Option<&T> {
        let location = self.archetypes.get_entity_location(entity)?;
        let archetype = self.archetypes.get_archetype(location.archetype_id)?;
        archetype.get_component::<T>(entity)
    }
    
    /// Get a mutable component reference
    pub fn get_mut<T: Component>(&mut self, entity: EntityId) -> Option<&mut T> {
        let location = self.archetypes.get_entity_location(entity)?;
        let archetype = self.archetypes.get_archetype_mut(location.archetype_id)?;
        
        // Mark entity as dirty for persistence
        self.persistence.change_tracker_mut().track_modified(entity);
        
        archetype.get_component_mut::<T>(entity)
    }
    
    /// Insert a component (replaces if exists)
    pub fn insert<T: Component>(&mut self, entity: EntityId, component: T) -> Result<(), EntityError> {
        // Handle archetype migration if component type changes
        // Mark entity as dirty
        // ...
    }
    
    /// Remove a component
    pub fn remove<T: Component>(&mut self, entity: EntityId) -> Option<T> {
        // Handle archetype migration
        // Mark entity as dirty
        // ...
    }
    
    /// Check if entity has component
    pub fn has<T: Component>(&self, entity: EntityId) -> bool {
        self.get::<T>(entity).is_some()
    }
}
```

**Benefits**:
- Essential for game logic
- Matches hecs/Bevy API
- Enables component-based programming

**Effort**: 6-8 hours (complex due to archetype migration)

---

#### 1.4 Query System
**Location**: `pecs/src/query.rs` (enhance existing)

**Implementation**:
```rust
// Current PECS has basic query infrastructure, needs ergonomic API

impl World {
    /// Query entities with components
    pub fn query<Q: Query>(&self) -> QueryIter<Q> {
        QueryIter::new(self, Q::fetch_state(self))
    }
    
    /// Query with filter
    pub fn query_filtered<Q: Query, F: Filter>(&self) -> FilteredQueryIter<Q, F> {
        FilteredQueryIter::new(self, Q::fetch_state(self), F::filter_state(self))
    }
}

// Usage:
for (entity, (name, health)) in world.query::<(&Name, &Health)>() {
    println!("{}: {}/{}", name.0, health.current, health.max);
}

// Mutable queries:
for (entity, (pos, vel)) in world.query::<(&mut Position, &Velocity)>() {
    pos.x += vel.x;
    pos.y += vel.y;
}
```

**Benefits**:
- Core ECS functionality
- Efficient iteration
- Type-safe queries

**Effort**: 8-12 hours (complex, needs careful design)

---

### Priority 2: Quality of Life Features

#### 2.1 Spawn with Specific StableId
**Location**: `pecs/src/world.rs`

**Implementation**:
```rust
impl World {
    /// Spawn entity with specific stable ID (for loading from database)
    pub fn spawn_with_stable_id(&mut self, stable_id: StableId) -> EntityId {
        let entity_id = self.entities.spawn_with_specific_stable_id(stable_id);
        // ... rest of spawn logic
        entity_id
    }
}
```

**Benefits**:
- Essential for database loading
- Preserves entity identity across sessions

**Effort**: 2-3 hours

---

#### 2.2 Component Reflection/Metadata
**Location**: `pecs/src/component.rs`

**Implementation**:
```rust
pub trait Component: 'static + Send + Sync {
    fn type_name() -> &'static str {
        std::any::type_name::<Self>()
    }
    
    fn type_id() -> ComponentTypeId {
        ComponentTypeId::of::<Self>()
    }
}

// Component registry for runtime type info
pub struct ComponentRegistry {
    components: HashMap<ComponentTypeId, ComponentInfo>,
}

pub struct ComponentInfo {
    type_name: &'static str,
    size: usize,
    align: usize,
    // Optional: serialization functions
}
```

**Benefits**:
- Better debugging
- Dynamic component handling
- Serialization support

**Effort**: 4-6 hours

---

#### 2.3 Entity Relationships
**Location**: `pecs/src/relation.rs` (new module)

**Implementation**:
```rust
// Parent-child relationships
world.add_child(parent_entity, child_entity);
world.remove_child(parent_entity, child_entity);
world.children(parent_entity) -> Vec<EntityId>;
world.parent(child_entity) -> Option<EntityId>;

// Generic relationships
world.add_relation::<Contains>(container, item);
world.has_relation::<Contains>(container, item) -> bool;
```

**Benefits**:
- Inventory systems
- Room/area hierarchies
- NPC relationships

**Effort**: 6-8 hours

---

### Priority 3: Performance & Polish

#### 3.1 Parallel Query Execution
**Location**: `pecs/src/query.rs`

**Implementation**:
```rust
impl World {
    /// Execute query in parallel using rayon
    pub fn par_query<Q: Query>(&self) -> ParQueryIter<Q> {
        // Split archetypes across threads
        // ...
    }
}
```

**Benefits**:
- Better performance for large worlds
- Scalability

**Effort**: 8-12 hours

---

#### 3.2 Change Detection
**Location**: `pecs/src/change_detection.rs`

**Implementation**:
```rust
// Track when components are modified
pub struct Changed<T: Component>(PhantomData<T>);

// Usage:
for (entity, name) in world.query::<(&Name, Changed<Health>)>() {
    println!("{}'s health changed!", name.0);
}
```

**Benefits**:
- Efficient reactive systems
- Network synchronization
- UI updates

**Effort**: 6-8 hours

---

#### 3.3 Event System
**Location**: `pecs/src/events.rs` (new module)

**Implementation**:
```rust
pub struct Events<T> {
    events: Vec<T>,
}

impl World {
    pub fn send_event<T: Event>(&mut self, event: T);
    pub fn read_events<T: Event>(&self) -> EventReader<T>;
}
```

**Benefits**:
- Decoupled systems
- Combat events, damage, etc.

**Effort**: 4-6 hours

---

## Implementation Strategy

### Phase 1: Core Features (Week 1)
1. Component Derive Macro (2-4h)
2. Bundle System (4-6h)
3. Direct Component Access (6-8h)
4. Basic Query Enhancement (8-12h)

**Total**: ~20-30 hours

### Phase 2: Essential QoL (Week 2)
1. Spawn with StableId (2-3h)
2. Component Reflection (4-6h)
3. Query System Polish (4-6h)

**Total**: ~10-15 hours

### Phase 3: Advanced Features (Week 3)
1. Entity Relationships (6-8h)
2. Change Detection (6-8h)
3. Event System (4-6h)

**Total**: ~16-22 hours

### Phase 4: Performance (Week 4)
1. Parallel Queries (8-12h)
2. Optimization passes (8-12h)
3. Benchmarking (4-6h)

**Total**: ~20-30 hours

## Total Effort Estimate

- **Minimum (Core only)**: 20-30 hours
- **Recommended (Core + QoL)**: 30-45 hours
- **Complete (All features)**: 66-97 hours

## Alternative: Hybrid Approach

Instead of enhancing PECS, we could:

1. **Keep PECS for persistence layer only**
   - Use PECS StableId system
   - Use PECS save/load functionality
   
2. **Use hecs for runtime**
   - Keep our existing hecs-based code
   - Bridge between hecs::Entity and pecs::StableId
   
3. **Benefits**:
   - Minimal code changes
   - Best of both worlds
   - Faster migration

**Effort**: ~10-15 hours (just bridging code)

## Recommendation

**Option 1: Enhance PECS (Recommended)**
- Implement Phase 1 (Core Features) first
- Evaluate after Phase 1 completion
- Continue with Phase 2 if satisfied
- **Timeline**: 4-6 weeks for complete implementation

**Option 2: Hybrid Approach (Pragmatic)**
- Use PECS for persistence only
- Keep hecs for runtime
- Bridge the two systems
- **Timeline**: 1-2 weeks

**Option 3: Different ECS (Alternative)**
- Consider Bevy ECS (feature-complete, well-maintained)
- Consider Legion (high performance)
- **Timeline**: 2-3 weeks migration

## Next Steps

1. **Decision**: Choose enhancement strategy
2. **Prototype**: Implement Phase 1 core features in PECS
3. **Evaluate**: Test with Wyldlands use cases
4. **Iterate**: Continue or pivot based on results

## Success Criteria

PECS enhancements are successful if:
- ✅ Component access is ergonomic (no manual archetype handling)
- ✅ Query API matches hecs/Bevy patterns
- ✅ Bundle system reduces boilerplate
- ✅ Performance is comparable to hecs
- ✅ Persistence "just works" with minimal code
- ✅ Migration from hecs is straightforward

## Risk Assessment

**High Risk**:
- Query system complexity (archetype iteration)
- Component access with archetype migration
- Performance regression vs hecs

**Medium Risk**:
- Bundle system edge cases
- Parallel query safety
- Change detection overhead

**Low Risk**:
- Component derive macro
- Spawn with StableId
- Event system

## Conclusion

Enhancing PECS is viable but requires significant investment (20-30 hours minimum). The hybrid approach offers a pragmatic alternative with less risk and faster delivery. The decision should be based on:

1. **Long-term vision**: Do we want to own/control the ECS?
2. **Timeline**: Can we afford 4-6 weeks of ECS work?
3. **Maintenance**: Are we prepared to maintain PECS long-term?

If answers are "yes", enhance PECS. If "no", consider hybrid or alternative ECS.