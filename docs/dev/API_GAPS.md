# API Gaps Identified During Phase 3

**Date**: 2026-02-13
**Phase**: 3 - Polish & Optimization
**Task**: 1.1 - Performance Profiling Setup
**Status**: ✅ **FULLY RESOLVED** (2026-02-14)

## Overview

While setting up the benchmark suite, several critical API gaps were identified that prevent effective benchmarking and usage of the library. These have been addressed as part of Phase 3 Week 7-8 API enrichment.

## ✅ Resolved: World Component Access Methods

The `World` struct was missing essential component access methods. **All have been implemented:**

```rust
// ✅ IMPLEMENTED: Direct component insertion
pub fn insert<T: Component>(&mut self, entity: EntityId, component: T) -> bool

// ✅ IMPLEMENTED: Direct component removal
pub fn remove<T: Component>(&mut self, entity: EntityId) -> Option<T>

// ✅ IMPLEMENTED: Immutable component access
pub fn get<T: Component>(&self, entity: EntityId) -> Option<&T>

// ✅ IMPLEMENTED: Mutable component access
pub fn get_mut<T: Component>(&mut self, entity: EntityId) -> Option<&mut T>

// ✅ IMPLEMENTED: Check if entity has component
pub fn has<T: Component>(&self, entity: EntityId) -> bool
```

**Status**: ✅ **COMPLETE** - All methods implemented with comprehensive tests

**Tests**: 13 new tests added, all passing (177 total tests)

### ✅ Resolved: Query System Integration

The query system has been integrated with `World`:

```rust
// ✅ IMPLEMENTED: Query execution
pub fn query<Q: Query>(&mut self) -> impl Iterator<Item = Q::Item<'_>>

// ✅ IMPLEMENTED: Filtered query execution
pub fn query_filtered<Q: Query, F: Filter>(&mut self) -> impl Iterator<Item = Q::Item<'_>>
```

**Status**: ✅ **COMPLETE** - Query system fully integrated

## Remaining Enhancements (Future Work)

### SerializableComponent Ergonomics

The `SerializableComponent` trait requires manual implementation of serialize/deserialize methods. A derive macro would improve developer experience:

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

**Priority**: **MEDIUM** - Quality of life improvement

**Status**: Deferred to future release

### EntityBuilder Method Consistency

The `EntityBuilder` has `.id()` but could also have `.build()` for consistency:

```rust
// Current:
let entity = world.spawn().with(Position { x: 0.0, y: 0.0 }).id();

// Alternative (more idiomatic):
let entity = world.spawn().with(Position { x: 0.0, y: 0.0 }).build();
```

**Impact**: Minor API inconsistency

**Priority**: **LOW** - Cosmetic improvement

**Status**: Deferred to future release

## ✅ Resolved: Archetype Transitions

**Fixed**: 2026-02-14

All archetype transition issues have been resolved:

- ✅ Single component per entity works perfectly
- ✅ Builder pattern (`.spawn().with(A).with(B)`) creates entity with multiple components
- ✅ Sequential `insert()` calls properly copy existing components during archetype transitions
- ✅ `remove()` method properly moves entities to new archetypes while preserving remaining components

### Bugs Fixed

1. **EntityBuilder Component Storage**: Fixed incorrect pointer casting in `EntityBuilder::id()` that prevented components from being stored correctly. Components are now properly transferred from `Box<dyn Any>` to archetype storage.

2. **EntityBuilder Location Tracking**: Added missing entity location tracking in `EntityBuilder::id()` so entities spawned with the builder pattern are properly registered in the archetype manager.

3. **Remove Method Archetype Transition**: Implemented proper archetype transition in `World::remove()` method. When a component is removed, the entity now moves to a new archetype containing only the remaining components, rather than being removed entirely.

**Tests**: All 202 tests passing, including:
- `query_after_component_removal` - Verifies queries work correctly after component removal
- `sequential_insert_*` tests - Verify multiple sequential inserts work correctly
- All existing component lifecycle tests

## Summary

✅ **Phase 3 API Gaps: FULLY RESOLVED**

- All critical World methods implemented
- Query system fully integrated
- Archetype transitions fully working for all operations
- EntityBuilder properly stores components and tracks entity locations
- Component removal properly handles archetype transitions
- 202 tests passing (all tests enabled and passing)
- Code clean (clippy) and formatted
- Comprehensive rustdoc documentation

The core API is now complete and fully functional for all component operations including:
- Multiple components per entity via builder pattern
- Sequential component insertion with proper archetype transitions
- Component removal with archetype transitions
- Complex queries across multiple archetypes

The library is ready for production use and further optimization work.