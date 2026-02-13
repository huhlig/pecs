# ADR-001: Dual Entity ID System

**Status**: Accepted
**Date**: 2026-02-13
**Deciders**: Development Team
**Related**: PRD Section 6.1, FR-1.5

## Context

Entity Component Systems require a way to uniquely identify entities. The challenge is balancing two competing needs:

1. **Performance**: Fast entity lookup and iteration requires compact, index-based IDs
2. **Persistence**: Stable identifiers that survive serialization and can reference entities across sessions

Traditional approaches force a choice between these requirements:
- **Index-only systems** (e.g., simple u32 indices) are fast but break when entities are deleted or the world is reloaded
- **UUID-only systems** provide stability but incur performance costs for lookups and memory overhead

PECS aims to provide both high performance during runtime and reliable persistence, requiring a solution that satisfies both needs.

## Decision

We will implement a **dual entity ID system** with two types of identifiers:

### Ephemeral IDs (Runtime Performance)
- **Structure**: 64-bit value containing:
  - 32-bit index: Direct array index for O(1) component lookup
  - 32-bit generation: Prevents use-after-free when entity IDs are recycled
- **Usage**: All runtime operations (queries, component access, system logic)
- **Lifetime**: Valid only within current session
- **Performance**: Zero-cost abstraction, optimal cache locality

### Stable IDs (Persistence)
- **Structure**: 128-bit UUID (UUID v4)
- **Usage**: Serialization, cross-session references, external integrations
- **Lifetime**: Permanent, survives save/load cycles
- **Generation**: Created once per entity, never changes

### Bidirectional Mapping
- Maintain HashMap<StableId, EphemeralId> for stable-to-ephemeral lookup
- Store StableId alongside entity metadata for ephemeral-to-stable lookup
- Update mappings automatically during entity lifecycle operations

### API Design
```rust
pub struct EphemeralId {
    index: u32,
    generation: u32,
}

pub struct StableId(uuid::Uuid);

pub struct Entity {
    ephemeral: EphemeralId,
    stable: StableId,
}

impl World {
    // Primary API uses ephemeral IDs for performance
    pub fn get_component<T>(&self, id: EphemeralId) -> Option<&T>;
    
    // Stable ID conversion for persistence scenarios
    pub fn get_stable_id(&self, ephemeral: EphemeralId) -> Option<StableId>;
    pub fn get_ephemeral_id(&self, stable: StableId) -> Option<EphemeralId>;
}
```

## Consequences

### Positive
- **Optimal Runtime Performance**: Ephemeral IDs provide O(1) component access with minimal memory overhead
- **Reliable Persistence**: Stable IDs enable robust save/load without entity reference corruption
- **Flexible Integration**: Stable IDs can be used for external systems (databases, networking, debugging)
- **Generation Safety**: Prevents dangling entity references through generation counters
- **Best of Both Worlds**: No compromise on either performance or persistence

### Negative
- **Memory Overhead**: Additional 16 bytes per entity for stable ID storage
- **Mapping Maintenance**: HashMap overhead for stable-to-ephemeral lookup (~24 bytes per entity)
- **API Complexity**: Two ID types may confuse users initially
- **Conversion Cost**: Looking up stable IDs requires HashMap access (though rare in hot paths)

### Neutral
- **Total Memory Cost**: ~40 bytes per entity for ID infrastructure (acceptable for target use cases)
- **Documentation Burden**: Requires clear explanation of when to use each ID type
- **Migration Path**: Existing ECS users familiar with single-ID systems need to learn new patterns

## Alternatives Considered

### Alternative 1: Ephemeral IDs Only
- **Pros**: 
  - Simplest implementation
  - Minimal memory overhead (8 bytes per entity)
  - Fastest possible performance
- **Cons**: 
  - Cannot persist entity references reliably
  - Breaks on entity deletion/recreation
  - Requires complex remapping logic on load
- **Rejected because**: Persistence is a core feature of PECS; this approach fundamentally conflicts with the project goals

### Alternative 2: Stable IDs Only (UUID-based)
- **Pros**: 
  - Single ID type, simpler API
  - Natural persistence support
  - No mapping overhead
- **Cons**: 
  - 16-byte IDs increase memory usage significantly
  - HashMap lookups required for all component access (performance hit)
  - Poor cache locality
  - Larger entity metadata
- **Rejected because**: Performance is critical; 10-100x slowdown for component access is unacceptable

### Alternative 3: Stable IDs with Index Cache
- **Pros**: 
  - Single ID type exposed to users
  - Can optimize hot paths with caching
- **Cons**: 
  - Cache invalidation complexity
  - Still requires HashMap for initial lookup
  - Hidden performance characteristics (confusing for users)
  - Doesn't eliminate the need for both ID types internally
- **Rejected because**: Adds complexity without solving the fundamental trade-off; dual IDs are more honest about costs

### Alternative 4: Hierarchical IDs (Slot + Generation + Session)
- **Pros**: 
  - Can encode session information in ID
  - Potentially detect cross-session references
- **Cons**: 
  - More complex ID structure
  - Still requires stable component for true persistence
  - Doesn't solve the core problem
- **Rejected because**: Adds complexity without providing the stability guarantees needed for persistence

## Implementation Notes

### Entity Creation
```rust
fn spawn_entity(&mut self) -> Entity {
    let index = self.allocate_index();
    let generation = self.generations[index];
    let ephemeral = EphemeralId { index, generation };
    let stable = StableId(Uuid::new_v4());
    
    self.stable_to_ephemeral.insert(stable, ephemeral);
    self.entity_metadata[index].stable_id = stable;
    
    Entity { ephemeral, stable }
}
```

### Entity Deletion
```rust
fn despawn_entity(&mut self, id: EphemeralId) {
    if let Some(stable) = self.get_stable_id(id) {
        self.stable_to_ephemeral.remove(&stable);
    }
    self.generations[id.index] += 1; // Invalidate ephemeral ID
    self.free_indices.push(id.index);
}
```

### Serialization
- Serialize using stable IDs only
- On load, generate new ephemeral IDs and rebuild mappings
- Preserve stable IDs to maintain entity references

### Performance Considerations
- Ephemeral IDs used in 99% of runtime operations (hot path)
- Stable ID lookups only needed for:
  - Serialization/deserialization
  - External system integration
  - Debugging/logging
- HashMap overhead amortized across entity lifetime

## References

- [EnTT Entity Identifiers](https://github.com/skypjack/entt/wiki/Crash-Course:-entity-component-system#entity-identifiers)
- [Bevy Entity IDs](https://docs.rs/bevy_ecs/latest/bevy_ecs/entity/struct.Entity.html)
- [Flecs Entity IDs](https://www.flecs.dev/flecs/md_docs_Entities.html)
- PRD Section 6.1: Entity ID System
- PRD FR-1.5: Provide both ephemeral and stable entity IDs