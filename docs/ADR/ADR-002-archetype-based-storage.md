# ADR-002: Archetype-Based Component Storage

**Status**: Accepted
**Date**: 2026-02-13
**Deciders**: Development Team
**Related**: PRD Section 6.2, NFR-1.3, ADR-003

## Context

Entity Component Systems need an efficient way to store and access components. The storage strategy fundamentally impacts:

1. **Query Performance**: How fast can we iterate over entities with specific component combinations?
2. **Memory Layout**: How cache-friendly is component access?
3. **Structural Changes**: How expensive is adding/removing components?
4. **Memory Overhead**: How much extra memory does the storage system require?

Different ECS implementations use various storage strategies:
- **Sparse Sets**: Fast component add/remove, but poor iteration performance
- **Dense Arrays**: Good iteration, but complex bookkeeping
- **Table-based**: Simple but inflexible
- **Archetype-based**: Optimal iteration, moderate structural change cost

PECS prioritizes query iteration performance (the hot path in most applications) while maintaining acceptable performance for structural changes.

## Decision

We will implement an **archetype-based component storage system** where entities are grouped by their component composition.

### Core Concepts

#### Archetype Definition
An archetype is a unique combination of component types. All entities with the same set of components belong to the same archetype.

```rust
// Example archetypes:
// Archetype A: [Position, Velocity]
// Archetype B: [Position, Velocity, Health]
// Archetype C: [Position, Renderable]
```

#### Storage Structure
```rust
pub struct Archetype {
    // Component type metadata
    component_types: Vec<ComponentTypeId>,
    
    // Structure of Arrays (SoA) storage
    // Each Vec<u8> is a type-erased component array
    component_storage: HashMap<ComponentTypeId, Vec<u8>>,
    
    // Entity metadata for this archetype
    entities: Vec<EphemeralId>,
    
    // Edges to other archetypes (for add/remove operations)
    edges: ArchetypeEdges,
}

pub struct World {
    // All archetypes in the world
    archetypes: Vec<Archetype>,
    
    // Map from component type set to archetype index
    archetype_index: HashMap<ComponentSet, ArchetypeId>,
    
    // Map from entity to its archetype and row
    entity_locations: Vec<EntityLocation>,
}

pub struct EntityLocation {
    archetype_id: ArchetypeId,
    row: usize,
}
```

#### Structure of Arrays (SoA) Layout
Within each archetype, components are stored in separate contiguous arrays:

```
Archetype [Position, Velocity]:
  positions: [pos0, pos1, pos2, pos3, ...]
  velocities: [vel0, vel1, vel2, vel3, ...]
  entities: [e0, e1, e2, e3, ...]
```

This provides excellent cache locality when iterating over components of the same type.

### Operations

#### Entity Creation
1. Determine archetype from component set
2. Create archetype if it doesn't exist
3. Append entity and components to archetype arrays
4. Record entity location

#### Component Addition
1. Find current archetype
2. Determine target archetype (current + new component)
3. Move entity data to target archetype
4. Update entity location

#### Component Removal
1. Find current archetype
2. Determine target archetype (current - removed component)
3. Move entity data to target archetype (excluding removed component)
4. Update entity location

#### Query Iteration
1. Find all archetypes matching query
2. Iterate through matching archetypes
3. Access component arrays directly (cache-friendly)

### Archetype Edges
To optimize structural changes, we cache transitions between archetypes:

```rust
pub struct ArchetypeEdges {
    // Adding a component type leads to which archetype?
    add_edges: HashMap<ComponentTypeId, ArchetypeId>,
    
    // Removing a component type leads to which archetype?
    remove_edges: HashMap<ComponentTypeId, ArchetypeId>,
}
```

## Consequences

### Positive
- **Optimal Query Performance**: Contiguous memory layout enables cache-friendly iteration
- **Predictable Performance**: Query speed independent of total entity count, only depends on matching entities
- **Memory Efficiency**: No sparse storage overhead, components packed tightly
- **Parallel-Friendly**: Different archetypes can be processed in parallel safely
- **Type Safety**: Component types known at archetype level, enabling compile-time optimizations
- **Query Optimization**: Can skip entire archetypes that don't match query

### Negative
- **Structural Change Cost**: Adding/removing components requires moving entity data between archetypes
- **Archetype Proliferation**: Many unique component combinations create many archetypes
- **Memory Fragmentation**: Small archetypes may waste memory
- **Complexity**: More complex implementation than simpler storage strategies

### Neutral
- **Trade-off**: Optimizes the common case (iteration) at the expense of the uncommon case (structural changes)
- **Memory Usage**: Comparable to other efficient ECS implementations
- **Archetype Count**: Typically manageable in practice (dozens to hundreds, not thousands)

## Alternatives Considered

### Alternative 1: Sparse Set Storage
- **Pros**:
  - O(1) component add/remove
  - Simple implementation
  - No archetype management
- **Cons**:
  - Poor cache locality during iteration
  - Requires sparse set per component type
  - Higher memory overhead
  - Slower queries (must check multiple sparse sets)
- **Rejected because**: Query performance is more critical than structural change performance for PECS use cases

### Alternative 2: Table-Based Storage (Single Table)
- **Pros**:
  - Simple conceptual model
  - Easy to implement
  - Predictable memory layout
- **Cons**:
  - Wastes memory for entities without all components
  - Requires null/option checks during iteration
  - Poor cache utilization (skipping null entries)
  - Inflexible for dynamic component sets
- **Rejected because**: Memory waste and null checks unacceptable for performance-critical applications

### Alternative 3: Hybrid Sparse Set + Archetype
- **Pros**:
  - Can optimize for both iteration and structural changes
  - Flexible for different component patterns
- **Cons**:
  - Significantly more complex
  - Requires heuristics to decide storage strategy
  - Unpredictable performance characteristics
  - Harder to reason about and debug
- **Rejected because**: Complexity not justified; archetype-only approach is sufficient and more predictable

### Alternative 4: Bitset-Based Storage
- **Pros**:
  - Fast component presence checks
  - Compact entity metadata
- **Cons**:
  - Still requires separate component storage
  - Bitset operations add overhead
  - Doesn't solve cache locality problem
  - Limited to fixed number of component types
- **Rejected because**: Doesn't address core performance concerns; archetypes provide better cache behavior

## Implementation Notes

### Archetype Creation
```rust
fn get_or_create_archetype(&mut self, components: &[ComponentTypeId]) -> ArchetypeId {
    let component_set = ComponentSet::from_slice(components);
    
    if let Some(&id) = self.archetype_index.get(&component_set) {
        return id;
    }
    
    let archetype = Archetype::new(components);
    let id = self.archetypes.len();
    self.archetypes.push(archetype);
    self.archetype_index.insert(component_set, id);
    id
}
```

### Moving Entity Between Archetypes
```rust
fn move_entity(&mut self, entity: EphemeralId, target_archetype: ArchetypeId) {
    let location = self.entity_locations[entity.index];
    let source = &mut self.archetypes[location.archetype_id];
    let target = &mut self.archetypes[target_archetype];
    
    // Copy shared components
    for component_type in source.shared_types(target) {
        let data = source.get_component_data(location.row, component_type);
        target.push_component_data(component_type, data);
    }
    
    // Remove from source (swap-remove for O(1))
    source.swap_remove(location.row);
    
    // Update entity location
    self.entity_locations[entity.index] = EntityLocation {
        archetype_id: target_archetype,
        row: target.len() - 1,
    };
}
```

### Query Matching
```rust
fn find_matching_archetypes(&self, query: &Query) -> Vec<ArchetypeId> {
    self.archetypes
        .iter()
        .enumerate()
        .filter(|(_, archetype)| archetype.matches(query))
        .map(|(id, _)| id)
        .collect()
}
```

### Memory Layout Optimization
- Align component arrays to cache line boundaries (64 bytes)
- Pre-allocate archetype capacity based on heuristics
- Use power-of-two growth strategy for component arrays
- Consider SIMD-friendly alignment for numeric components

### Archetype Graph
Build a graph of archetype transitions to optimize common patterns:
```
[Position] --add Velocity--> [Position, Velocity]
[Position, Velocity] --add Health--> [Position, Velocity, Health]
[Position, Velocity] --remove Velocity--> [Position]
```

## Performance Characteristics

| Operation | Time Complexity | Notes |
|-----------|----------------|-------|
| Query iteration | O(n) | n = matching entities, cache-friendly |
| Component access | O(1) | Direct array indexing |
| Add component | O(m) | m = components to copy, typically small |
| Remove component | O(m) | m = components to copy, typically small |
| Entity creation | O(1) amortized | Array append |
| Entity deletion | O(1) | Swap-remove |

## References

- [Bevy ECS Archetype Storage](https://bevyengine.org/news/bevy-0-5/#archetype-component-storage)
- [Unity DOTS Archetype Chunks](https://docs.unity3d.com/Packages/com.unity.entities@0.17/manual/ecs_core.html)
- [Flecs Archetype Storage](https://www.flecs.dev/flecs/md_docs_Manual.html#archetype-storage)
- [EnTT Groups (similar concept)](https://github.com/skypjack/entt/wiki/Crash-Course:-entity-component-system#groups)
- PRD Section 6.2: Component Storage
- PRD NFR-1.3: Query iteration performance