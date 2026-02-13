# ADR-003: Query System Design

**Status**: Accepted
**Date**: 2026-02-13
**Deciders**: Development Team
**Related**: PRD Section 6.3, FR-3, ADR-002

## Context

The query system is the primary interface for accessing entities and components in an ECS. It must balance several concerns:

1. **Ergonomics**: Easy to use, minimal boilerplate
2. **Type Safety**: Catch errors at compile time
3. **Performance**: Zero-cost abstractions, efficient iteration
4. **Flexibility**: Support complex query patterns (filters, optional components)
5. **Safety**: Prevent data races and invalid access patterns

Different ECS implementations offer varying query APIs:
- **Tuple-based queries**: Type-safe but limited flexibility
- **Builder pattern**: Flexible but verbose
- **Macro-based**: Powerful but complex
- **Trait-based**: Composable but steep learning curve

PECS needs a query system that feels natural to Rust developers while providing the power needed for complex game logic.

## Decision

We will implement a **type-driven query system** using Rust's type system and traits to provide compile-time safety and zero-cost abstractions.

### Core Design

#### Query Definition
Queries are defined by their component access pattern using tuples:

```rust
// Immutable access to Position and Velocity
world.query::<(&Position, &Velocity)>()

// Mutable access to Position, immutable to Velocity
world.query::<(&mut Position, &Velocity)>()

// Multiple mutable accesses (different types)
world.query::<(&mut Position, &mut Health)>()
```

#### Query Trait
```rust
pub trait Query {
    type Item<'a>;
    
    // Check if an archetype matches this query
    fn matches_archetype(archetype: &Archetype) -> bool;
    
    // Fetch data from an archetype
    fn fetch<'a>(archetype: &'a Archetype, row: usize) -> Self::Item<'a>;
}

// Implement for common patterns
impl<T: Component> Query for &T {
    type Item<'a> = &'a T;
    // ...
}

impl<T: Component> Query for &mut T {
    type Item<'a> = &'a mut T;
    // ...
}

// Tuple implementations for combinations
impl<A: Query, B: Query> Query for (A, B) {
    type Item<'a> = (A::Item<'a>, B::Item<'a>);
    // ...
}
```

#### Query Iterator
```rust
pub struct QueryIter<'w, Q: Query> {
    world: &'w World,
    archetype_iter: ArchetypeIter,
    current_archetype: Option<&'w Archetype>,
    row: usize,
    _phantom: PhantomData<Q>,
}

impl<'w, Q: Query> Iterator for QueryIter<'w, Q> {
    type Item = Q::Item<'w>;
    
    fn next(&mut self) -> Option<Self::Item> {
        // Iterate through matching archetypes and rows
        // Return fetched component data
    }
}
```

### Advanced Query Features

#### Optional Components
```rust
// Option<&T> for components that may not exist
world.query::<(&Position, Option<&Velocity>)>()
    .for_each(|(pos, vel)| {
        if let Some(vel) = vel {
            // Has velocity
        }
    });
```

#### Entity Access
```rust
// Include entity ID in query results
world.query::<(Entity, &Position, &mut Health)>()
    .for_each(|(entity, pos, health)| {
        // Can reference the entity
    });
```

#### Query Filters
```rust
// Filter by additional criteria
world.query::<(&Position, &Velocity)>()
    .with::<Renderable>()      // Must have Renderable
    .without::<Dead>()          // Must not have Dead
    .for_each(|(pos, vel)| {
        // Only entities matching filters
    });
```

#### Changed Detection
```rust
// Only entities with changed components
world.query::<(&Position, Changed<&Velocity>)>()
    .for_each(|(pos, vel)| {
        // Velocity was modified since last check
    });
```

### Borrow Checking

The query system enforces Rust's borrowing rules at compile time:

```rust
// ✓ Valid: Multiple immutable borrows
world.query::<(&Position, &Velocity)>();

// ✓ Valid: One mutable, one immutable (different types)
world.query::<(&mut Position, &Velocity)>();

// ✗ Invalid: Multiple mutable borrows of same type
// world.query::<(&mut Position, &mut Position)>(); // Compile error

// ✗ Invalid: Mutable and immutable of same type
// world.query::<(&mut Position, &Position)>(); // Compile error
```

### Query API

```rust
impl World {
    // Basic query
    pub fn query<Q: Query>(&self) -> QueryBuilder<Q> {
        QueryBuilder::new(self)
    }
    
    // Mutable query
    pub fn query_mut<Q: Query>(&mut self) -> QueryBuilder<Q> {
        QueryBuilder::new_mut(self)
    }
}

pub struct QueryBuilder<'w, Q: Query> {
    world: &'w World,
    filters: Vec<Filter>,
    _phantom: PhantomData<Q>,
}

impl<'w, Q: Query> QueryBuilder<'w, Q> {
    // Add filters
    pub fn with<T: Component>(self) -> Self { /* ... */ }
    pub fn without<T: Component>(self) -> Self { /* ... */ }
    
    // Execute query
    pub fn iter(&self) -> QueryIter<'w, Q> { /* ... */ }
    pub fn for_each<F>(self, f: F) where F: FnMut(Q::Item<'_>) { /* ... */ }
    
    // Single entity queries
    pub fn get(&self, entity: EphemeralId) -> Option<Q::Item<'_>> { /* ... */ }
}
```

## Consequences

### Positive
- **Type Safety**: Borrow checking enforced at compile time, preventing data races
- **Zero Cost**: Compiles to efficient iteration code with no runtime overhead
- **Ergonomic**: Natural Rust syntax using tuples and references
- **Flexible**: Supports complex query patterns through filters and modifiers
- **Composable**: Query types can be composed and reused
- **IDE Support**: Full autocomplete and type inference
- **Performance**: Direct archetype iteration with minimal indirection

### Negative
- **Learning Curve**: Users must understand Rust's borrowing rules
- **Compile Times**: Complex queries may increase compilation time
- **Error Messages**: Type errors can be verbose and confusing
- **Limitations**: Some query patterns may be difficult to express
- **Tuple Limit**: Rust tuple implementations limited to ~12 elements

### Neutral
- **Trait Complexity**: Internal implementation is complex but hidden from users
- **Generic Code**: Heavy use of generics may impact binary size
- **Documentation**: Requires clear examples and explanations

## Alternatives Considered

### Alternative 1: Builder Pattern with Runtime Checks
```rust
world.query()
    .with::<Position>()
    .with::<Velocity>()
    .build()
```
- **Pros**:
  - More flexible, no tuple limits
  - Easier to add optional components
  - Simpler implementation
- **Cons**:
  - Runtime borrow checking overhead
  - Less type-safe
  - Verbose API
  - No compile-time guarantees
- **Rejected because**: Runtime checks conflict with zero-cost abstraction goal; type safety is critical

### Alternative 2: Macro-Based Queries
```rust
query!(world, |pos: &Position, vel: &Velocity| {
    // Query body
});
```
- **Pros**:
  - Very ergonomic
  - Can generate optimal code
  - Flexible syntax
- **Cons**:
  - Macros are complex to maintain
  - Poor IDE support
  - Confusing error messages
  - Debugging difficulties
- **Rejected because**: Macros add complexity without significant benefits over trait-based approach

### Alternative 3: System Parameters (Bevy-style)
```rust
fn my_system(query: Query<(&Position, &Velocity)>) {
    for (pos, vel) in query.iter() {
        // ...
    }
}
```
- **Pros**:
  - Very clean for system-based architecture
  - Automatic dependency injection
  - Clear system signatures
- **Cons**:
  - Requires framework-style system scheduler
  - Conflicts with library-first approach
  - More complex implementation
- **Rejected because**: PECS is a library, not a framework (see ADR-005)

### Alternative 4: SQL-Like Query Language
```rust
world.query("SELECT Position, Velocity WHERE Health > 0")
```
- **Pros**:
  - Familiar to database developers
  - Very flexible
  - Could support complex joins
- **Cons**:
  - No compile-time safety
  - Runtime parsing overhead
  - Type erasure issues
  - Poor IDE support
- **Rejected because**: Completely conflicts with Rust's type safety philosophy

## Implementation Notes

### Query Trait Implementation
```rust
// Implement Query for references
impl<T: Component> Query for &T {
    type Item<'a> = &'a T;
    
    fn matches_archetype(archetype: &Archetype) -> bool {
        archetype.has_component::<T>()
    }
    
    fn fetch<'a>(archetype: &'a Archetype, row: usize) -> Self::Item<'a> {
        archetype.get_component::<T>(row)
    }
}

// Implement for tuples (up to 12 elements)
impl<A: Query, B: Query> Query for (A, B) {
    type Item<'a> = (A::Item<'a>, B::Item<'a>);
    
    fn matches_archetype(archetype: &Archetype) -> bool {
        A::matches_archetype(archetype) && B::matches_archetype(archetype)
    }
    
    fn fetch<'a>(archetype: &'a Archetype, row: usize) -> Self::Item<'a> {
        (A::fetch(archetype, row), B::fetch(archetype, row))
    }
}
```

### Borrow Checking Implementation
```rust
pub trait QueryBorrow {
    fn borrows() -> Vec<(TypeId, BorrowType)>;
}

pub enum BorrowType {
    Immutable,
    Mutable,
}

// Check for conflicts at compile time using type system
impl<T: Component> QueryBorrow for &T {
    fn borrows() -> Vec<(TypeId, BorrowType)> {
        vec![(TypeId::of::<T>(), BorrowType::Immutable)]
    }
}

impl<T: Component> QueryBorrow for &mut T {
    fn borrows() -> Vec<(TypeId, BorrowType)> {
        vec![(TypeId::of::<T>(), BorrowType::Mutable)]
    }
}
```

### Query Optimization
- Cache matching archetypes for repeated queries
- Use archetype graph to skip non-matching archetypes early
- Inline fetch operations for common query patterns
- Pre-compute component offsets within archetypes

### Parallel Queries
```rust
// Future extension: parallel iteration
world.query::<(&mut Position, &Velocity)>()
    .par_for_each(|(pos, vel)| {
        // Parallel execution across archetypes
    });
```

## Performance Characteristics

| Operation | Time Complexity | Notes |
|-----------|----------------|-------|
| Query setup | O(a) | a = number of archetypes, cached |
| Iteration | O(n) | n = matching entities, cache-friendly |
| Component fetch | O(1) | Direct array access |
| Filter check | O(1) | Archetype-level check |
| Borrow validation | O(1) | Compile-time only |

## Usage Examples

### Basic Query
```rust
for (pos, vel) in world.query::<(&mut Position, &Velocity)>() {
    pos.x += vel.x;
    pos.y += vel.y;
}
```

### With Filters
```rust
world.query::<(&Position, &Health)>()
    .with::<Player>()
    .without::<Dead>()
    .for_each(|(pos, health)| {
        println!("Player at {:?} has {} health", pos, health.value);
    });
```

### Single Entity
```rust
if let Some((pos, health)) = world.query::<(&Position, &Health)>().get(entity) {
    println!("Entity health: {}", health.value);
}
```

### Optional Components
```rust
for (pos, maybe_vel) in world.query::<(&Position, Option<&Velocity>)>() {
    match maybe_vel {
        Some(vel) => println!("Moving entity at {:?}", pos),
        None => println!("Static entity at {:?}", pos),
    }
}
```

## References

- [Bevy Query System](https://docs.rs/bevy_ecs/latest/bevy_ecs/system/struct.Query.html)
- [Hecs Query API](https://docs.rs/hecs/latest/hecs/struct.World.html#method.query)
- [Legion Query System](https://docs.rs/legion/latest/legion/query/index.html)
- [Rust Iterator Trait](https://doc.rust-lang.org/std/iter/trait.Iterator.html)
- PRD Section 6.3: Query System
- PRD FR-3: Query Interface
- ADR-002: Archetype-Based Component Storage