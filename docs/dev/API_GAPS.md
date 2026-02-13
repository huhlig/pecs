# API Gaps Identified During Phase 3

**Date**: 2026-02-13  
**Phase**: 3 - Polish & Optimization  
**Task**: 1.1 - Performance Profiling Setup

## Overview

While setting up the benchmark suite, several critical API gaps were identified that prevent effective benchmarking and usage of the library. These need to be addressed as part of Phase 3 API refinement.

## Missing World Methods

### Component Access Methods

The `World` struct is missing essential component access methods:

```rust
// MISSING: Direct component insertion
pub fn insert<T: Component>(&mut self, entity: EntityId, component: T) -> Result<(), EntityError>

// MISSING: Direct component removal  
pub fn remove<T: Component>(&mut self, entity: EntityId) -> Option<T>

// MISSING: Immutable component access
pub fn get<T: Component>(&self, entity: EntityId) -> Option<&T>

// MISSING: Mutable component access
pub fn get_mut<T: Component>(&mut self, entity: EntityId) -> Option<&mut T>

// MISSING: Check if entity has component
pub fn has<T: Component>(&self, entity: EntityId) -> bool
```

**Impact**: Cannot benchmark component operations, cannot use library effectively for real applications.

**Priority**: **CRITICAL** - Required for basic ECS functionality

### Query System Integration

The query system exists but is not integrated with `World`:

```rust
// MISSING: Query execution
pub fn query<Q: Query>(&mut self) -> impl Iterator<Item = Q::Item<'_>>

// MISSING: Filtered query execution  
pub fn query_filtered<Q: Query, F: Filter>(&mut self) -> impl Iterator<Item = Q::Item<'_>>
```

**Impact**: Cannot iterate over entities with components, core ECS pattern not usable.

**Priority**: **CRITICAL** - Required for basic ECS functionality

## Missing SerializableComponent Implementation

The `SerializableComponent` trait requires manual implementation of serialize/deserialize methods, but there's no derive macro or default implementation for types that already implement `Serialize` + `Deserialize`.

```rust
// Current (verbose):
impl SerializableComponent for Position {
    fn serialize(&self, writer: &mut dyn Write) -> Result<()> {
        // Manual serde implementation
    }
    
    fn deserialize(reader: &mut dyn Read) -> Result<Self> {
        // Manual serde implementation  
    }
}

// Desired (automatic):
#[derive(Component, Serialize, Deserialize)]
struct Position { x: f32, y: f32 }
// SerializableComponent auto-implemented
```

**Impact**: Verbose boilerplate for every component, error-prone.

**Priority**: **HIGH** - Significantly impacts developer experience

## Missing EntityBuilder Method

The `EntityBuilder` has `.id()` but the pattern suggests it should also have `.build()`:

```rust
// Current:
let entity = world.spawn().with(Position { x: 0.0, y: 0.0 }).id();

// Expected (more idiomatic):
let entity = world.spawn().with(Position { x: 0.0, y: 0.0 }).build();
```

**Impact**: Minor API inconsistency

**Priority**: **LOW** - Cosmetic, but improves consistency

## Recommendations

### Phase 3 Week 7-8 (API Refinement)

1. **Implement missing World methods** (Task 4.2)
   - Add `insert`, `remove`, `get`, `get_mut`, `has` methods
   - Integrate query system with World
   - Add comprehensive tests for all methods

2. **Simplify SerializableComponent** (Task 4.2)
   - Create derive macro or blanket implementation
   - Reduce boilerplate for common case
   - Maintain flexibility for custom implementations

3. **Standardize builder pattern** (Task 4.2)
   - Add `.build()` method to EntityBuilder
   - Deprecate or alias `.id()` for consistency

### Testing Strategy

Once these APIs are implemented:
- Run full benchmark suite
- Measure performance against targets
- Identify optimization opportunities
- Create examples demonstrating usage

## Notes

These gaps were discovered during benchmark setup, which validates the Phase 3 approach of internal testing before public release. The missing APIs would have been discovered by early users, causing frustration.

The good news: The underlying systems (entity management, component storage, archetypes) are solid. We just need to expose them through a complete, ergonomic API.