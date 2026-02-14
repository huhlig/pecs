# Core Concepts

This guide provides a deep dive into the architecture and design principles of PECS (Persistent Entity Component System).

## Table of Contents

- [What is an ECS?](#what-is-an-ecs)
- [PECS Architecture](#pecs-architecture)
- [Dual ID System](#dual-id-system)
- [Archetype-Based Storage](#archetype-based-storage)
- [Command Buffers](#command-buffers)
- [Persistence System](#persistence-system)
- [Design Philosophy](#design-philosophy)

## What is an ECS?

Entity Component System (ECS) is an architectural pattern commonly used in game development and data-oriented applications. It separates:

- **Entities**: Unique identifiers (just IDs)
- **Components**: Pure data (no behavior)
- **Systems**: Logic that operates on components

### Traditional OOP vs ECS

**Traditional Object-Oriented:**
```rust
// ❌ Tight coupling, inheritance hierarchies
class GameObject {
    position: Position,
    render() { ... },
    update() { ... }
}

class Enemy extends GameObject {
    ai: AI,
    attack() { ... }
}
```

**ECS Approach:**
```rust
// ✅ Composition over inheritance, data-oriented
struct Position { x: f32, y: f32 }
struct Velocity { x: f32, y: f32 }
struct Enemy { aggression: f32 }

// Systems operate on components
fn movement_system(world: &mut World) {
    for (pos, vel) in world.query::<(&mut Position, &Velocity)>() {
        pos.x += vel.x;
        pos.y += vel.y;
    }
}
```

### Benefits of ECS

1. **Performance**: Cache-friendly data layout
2. **Flexibility**: Easy to add/remove behaviors
3. **Parallelization**: Systems can run in parallel
4. **Composition**: Build complex entities from simple components
5. **Data-Oriented**: Optimized for modern CPU architectures

## PECS Architecture

PECS is designed as a **library, not a framework**. You integrate it into your application rather than building your application around it.

### Core Modules

```
pecs/
├── entity/          # Entity lifecycle management
│   ├── EntityId     # Fast runtime IDs
│   ├── StableId     # Persistent UUIDs
│   └── EntityManager
├── component/       # Component storage
│   ├── Component    # Component trait
│   ├── Archetype    # Storage optimization
│   └── ComponentSet
├── query/           # Data access (Phase 3)
│   ├── Query        # Type-safe queries
│   ├── Fetch        # Component fetching
│   └── Filter       # Entity filtering
├── command/         # Deferred operations
│   ├── Command      # Command trait
│   └── CommandBuffer
├── persistence/     # Save/load system
│   ├── Plugin       # Pluggable formats
│   ├── Binary       # Binary serialization
│   └── JSON         # JSON serialization
└── world/           # Main coordinator
    └── World        # Central hub
```

### Data Flow

```
┌─────────────────────────────────────────────┐
│                   World                      │
│  ┌────────────┐  ┌──────────────────────┐  │
│  │  Entities  │  │    Components        │  │
│  │            │  │  ┌────────────────┐  │  │
│  │ EntityId   │──┼─▶│  Archetype 1   │  │  │
│  │ StableId   │  │  │  [Pos, Vel]    │  │  │
│  │            │  │  └────────────────┘  │  │
│  │ Manager    │  │  ┌────────────────┐  │  │
│  │            │  │  │  Archetype 2   │  │  │
│  └────────────┘  │  │  [Pos, Health] │  │  │
│                  │  └────────────────┘  │  │
│  ┌────────────┐  └──────────────────────┘  │
│  │  Commands  │                             │
│  │  Buffer    │  Deferred Operations        │
│  └────────────┘                             │
└─────────────────────────────────────────────┘
```

## Dual ID System

PECS uses two types of entity identifiers to balance performance and persistence needs.

### EntityId (Ephemeral)

**Purpose**: Fast runtime operations

**Structure**: 64-bit packed integer
```
┌──────────────────┬──────────────────┐
│   Generation     │      Index       │
│    (32 bits)     │    (32 bits)     │
└──────────────────┴──────────────────┘
```

**Properties**:
- **Size**: 8 bytes
- **Performance**: O(1) lookup, ~5ns access
- **Recycling**: Generation counter prevents use-after-free
- **Scope**: Valid only within current session

**Example**:
```rust
use pecs::entity::EntityId;

let id = EntityId::new(42, 1);
println!("Index: {}", id.index());      // 42
println!("Generation: {}", id.generation()); // 1
println!("Display: {}", id);            // "42v1"
```

### StableId (Persistent)

**Purpose**: Cross-session persistence

**Structure**: 128-bit UUID
```
┌──────────────────────────────────────┐
│           128-bit UUID                │
│  (High 64 bits)  │  (Low 64 bits)    │
└──────────────────────────────────────┘
```

**Properties**:
- **Size**: 16 bytes
- **Uniqueness**: Globally unique (UUID v4)
- **Performance**: ~100ns generation
- **Scope**: Persistent across sessions

**Example**:
```rust
use pecs::entity::StableId;

let stable = StableId::new();
println!("Stable ID: {}", stable);
// Output: "a1b2c3d4e5f6789012345678abcdef01"
```

### Why Two IDs?

| Aspect | EntityId | StableId |
|--------|----------|----------|
| **Speed** | ✅ Very fast | ⚠️ Slower |
| **Size** | ✅ 8 bytes | ⚠️ 16 bytes |
| **Persistence** | ❌ Session-only | ✅ Permanent |
| **Uniqueness** | ⚠️ Per-session | ✅ Global |
| **Use Case** | Runtime queries | Save/load |

**Best Practice**: Use `EntityId` for all runtime operations, `StableId` only for persistence.

### ID Mapping

PECS maintains bidirectional mapping:

```rust
let mut world = World::new();
let entity = world.spawn_empty();

// EntityId → StableId
let stable = world.get_stable_id(entity).unwrap();

// StableId → EntityId
let found = world.get_entity_id(stable).unwrap();
assert_eq!(found, entity);
```

### Entity Recycling

When an entity is despawned, its slot can be reused:

```rust
let e1 = world.spawn_empty();
println!("Entity 1: {}", e1); // "0v1"

world.despawn(e1);

let e2 = world.spawn_empty();
println!("Entity 2: {}", e2); // "0v2" (same index, new generation)

// Old reference is now invalid
assert!(!world.is_alive(e1));
assert!(world.is_alive(e2));
```

**Key Point**: The generation counter prevents accidental use of stale entity references.

## Archetype-Based Storage

PECS uses an archetype-based storage system for optimal performance.

### What is an Archetype?

An archetype is a unique combination of component types. All entities with the same components belong to the same archetype.

```rust
// Archetype 1: [Position, Velocity]
entity_1: Position { x: 0.0, y: 0.0 }, Velocity { x: 1.0, y: 0.0 }
entity_2: Position { x: 5.0, y: 3.0 }, Velocity { x: 0.5, y: 0.5 }

// Archetype 2: [Position, Health]
entity_3: Position { x: 10.0, y: 10.0 }, Health { current: 100, max: 100 }

// Archetype 3: [Position, Velocity, Health]
entity_4: Position { x: 2.0, y: 2.0 }, Velocity { x: 1.0, y: 1.0 }, Health { current: 50, max: 100 }
```

### Storage Layout

Components are stored in Structure of Arrays (SoA) format:

```
Archetype [Position, Velocity]:
┌─────────────────────────────────────┐
│ Entities: [e1, e2, e3, ...]         │
├─────────────────────────────────────┤
│ Position: [p1, p2, p3, ...]         │
│   Contiguous memory                 │
├─────────────────────────────────────┤
│ Velocity: [v1, v2, v3, ...]         │
│   Contiguous memory                 │
└─────────────────────────────────────┘
```

### Benefits

1. **Cache Locality**: Components of the same type are stored contiguously
2. **Fast Iteration**: Queries iterate over dense arrays
3. **Memory Efficiency**: No padding between components
4. **SIMD Friendly**: Contiguous data enables vectorization

### Archetype Transitions

When components are added/removed, entities move between archetypes:

```rust
// Entity starts in archetype [Position]
let entity = world.spawn()
    .with(Position { x: 0.0, y: 0.0 })
    .id();

// Adding Velocity moves to archetype [Position, Velocity]
world.insert(entity, Velocity { x: 1.0, y: 0.0 });

// Removing Position moves to archetype [Velocity]
world.remove::<Position>(entity);
```

**Performance Note**: Archetype transitions involve copying data, so minimize component add/remove operations in hot paths.

### Archetype Edges

PECS caches archetype transitions for performance:

```
Archetype [Position]
    ├─ +Velocity → Archetype [Position, Velocity]
    ├─ +Health   → Archetype [Position, Health]
    └─ -Position → Archetype []

Archetype [Position, Velocity]
    ├─ +Health   → Archetype [Position, Velocity, Health]
    ├─ -Position → Archetype [Velocity]
    └─ -Velocity → Archetype [Position]
```

## Command Buffers

Command buffers enable thread-safe, deferred operations on the world.

### Why Command Buffers?

**Problem**: Direct world mutation isn't thread-safe
```rust
// ❌ Can't do this from multiple threads
world.spawn(); // Requires &mut World
```

**Solution**: Record commands, apply later
```rust
// ✅ Can do this from multiple threads
let mut buffer = CommandBuffer::new();
buffer.spawn(); // Just records the command
buffer.apply(&mut world); // Apply when safe
```

### Command Pattern

```rust
use pecs::command::CommandBuffer;

let mut buffer = CommandBuffer::new();

// Record operations
buffer.spawn();
buffer.spawn();
let entity = buffer.spawn();
buffer.despawn(entity);

// Nothing has happened yet
assert_eq!(world.len(), 0);

// Apply all commands atomically
buffer.apply(&mut world);
assert_eq!(world.len(), 2);
```

### Thread Safety

Command buffers are `Send` but not `Sync`:

```rust
use std::thread;

let mut world = World::new();

// Each thread gets its own buffer
let handle = thread::spawn(|| {
    let mut buffer = CommandBuffer::new();
    for _ in 0..100 {
        buffer.spawn();
    }
    buffer // Return buffer to main thread
});

let buffer = handle.join().unwrap();
buffer.apply(&mut world);
```

### Use Cases

1. **Parallel Systems**: Record changes from multiple threads
2. **Deferred Deletion**: Mark entities for deletion without immediate removal
3. **Batch Operations**: Group operations for better performance
4. **Event Handling**: Queue entity spawns from events

## Persistence System

PECS provides a pluggable persistence system for saving and loading worlds.

### Architecture

```
┌──────────────────────────────────────┐
│      PersistenceManager              │
│  ┌────────────────────────────────┐  │
│  │  Registered Plugins            │  │
│  │  ├─ BinaryPlugin (default)     │  │
│  │  ├─ JsonPlugin                 │  │
│  │  └─ CustomPlugin               │  │
│  └────────────────────────────────┘  │
│  ┌────────────────────────────────┐  │
│  │  WorldMetadata                 │  │
│  │  ├─ Version                    │  │
│  │  ├─ Timestamp                  │  │
│  │  └─ Component Registry         │  │
│  └────────────────────────────────┘  │
│  ┌────────────────────────────────┐  │
│  │  ChangeTracker                 │  │
│  │  ├─ Created entities           │  │
│  │  ├─ Deleted entities           │  │
│  │  └─ Modified components        │  │
│  └────────────────────────────────┘  │
└──────────────────────────────────────┘
```

### Serialization Formats

**Binary Format** (default):
- Compact size
- Fast serialization
- Version-aware
- Checksum validation

**JSON Format**:
- Human-readable
- Easy debugging
- Cross-platform
- Larger file size

### Basic Usage

```rust
use pecs::World;

// Save world
let world = World::new();
world.save("world.pecs")?;

// Load world
let loaded = World::load("world.pecs")?;
```

### Streaming API

For better performance with large worlds:

```rust
use std::fs::File;

// Save to stream
let mut file = File::create("world.pecs")?;
world.save_binary(&mut file)?;

// Load from stream
let mut file = File::open("world.pecs")?;
let world = World::load_binary(&mut file)?;
```

### Transient Components

Mark components as transient (not saved):

```rust
use pecs::persistence::SerializableComponent;

#[derive(Debug)]
struct CachedData {
    // Runtime-only data
}

impl Component for CachedData {}

// Don't implement SerializableComponent
// This component won't be saved
```

## Design Philosophy

PECS follows several key design principles:

### 1. Library, Not Framework

**Philosophy**: Integrate PECS into your application, don't build around it.

```rust
// ✅ PECS way: You control the structure
fn main() {
    let mut world = World::new();
    let mut game = MyGame::new();
    
    loop {
        game.update(&mut world);
        game.render(&world);
    }
}

// ❌ Framework way: Framework controls structure
// fn main() {
//     App::new()
//         .add_system(my_system)
//         .run();
// }
```

### 2. Performance First

- Zero-cost abstractions where possible
- Cache-friendly data layouts
- Minimal allocations
- SIMD-friendly operations

### 3. Type Safety

- Compile-time query validation
- No runtime type errors
- Rust's ownership system prevents data races

### 4. Flexibility

- Pluggable persistence
- Custom components
- No forced patterns
- Minimal dependencies

### 5. Simplicity

- Small, focused API
- Clear documentation
- Predictable behavior
- Easy to learn

## Performance Characteristics

### Entity Operations

| Operation | Complexity | Typical Time |
|-----------|------------|--------------|
| Spawn | O(1) amortized | ~100-300ns |
| Despawn | O(1) | ~50ns |
| Is Alive | O(1) | ~5ns |
| Get Stable ID | O(1) | ~10ns |

### Component Operations

| Operation | Complexity | Typical Time |
|-----------|------------|--------------|
| Insert | O(1)* | ~100ns |
| Remove | O(1)* | ~100ns |
| Get | O(1) | ~5ns |
| Get Mut | O(1) | ~5ns |

*May trigger archetype transition (O(n) where n = component count)

### Query Operations

| Operation | Complexity | Typical Time |
|-----------|------------|--------------|
| Iteration | O(n) | ~10-20ns per entity |
| Archetype Match | O(1) | ~5ns |
| Filter | O(n) | ~15-25ns per entity |

### Persistence Operations

| Operation | Complexity | Typical Time |
|-----------|------------|--------------|
| Binary Save | O(n) | ~0.36ms per 1000 entities |
| Binary Load | O(n) | ~0.28ms per 1000 entities |
| JSON Save | O(n) | ~2-3ms per 1000 entities |
| JSON Load | O(n) | ~3-4ms per 1000 entities |

## Memory Layout

### Entity Storage

```
EntityAllocator:
├─ Entities: Vec<EntityMeta>     [8 bytes per entity]
├─ Free List: Vec<u32>           [4 bytes per free slot]
├─ Stable IDs: HashMap           [~40 bytes per entity]
└─ Reverse Map: HashMap          [~40 bytes per entity]

Total: ~96 bytes per entity (worst case)
```

### Component Storage

```
Archetype:
├─ Entities: Vec<EntityId>       [8 bytes per entity]
├─ Components: Vec<Vec<u8>>      [size_of::<T>() per entity]
└─ Metadata: ComponentInfo       [~64 bytes per component type]

Total: 8 + Σ(component_sizes) bytes per entity
```

## Next Steps

- **[Getting Started](GETTING_STARTED.md)** - Basic usage tutorial
- **[Performance Guide](PERFORMANCE.md)** - Optimization techniques
- **[Advanced Patterns](ADVANCED.md)** - Expert-level usage
- **[API Reference](https://docs.rs/pecs)** - Complete API documentation
- **[ADRs](ADR/README.md)** - Architecture Decision Records

## Further Reading

- [ADR-001: Dual Entity ID System](ADR/ADR-001-dual-entity-id-system.md)
- [ADR-002: Archetype-Based Storage](ADR/ADR-002-archetype-based-storage.md)
- [ADR-003: Query System Design](ADR/ADR-003-query-system-design.md)
- [ADR-004: Command Buffer Architecture](ADR/ADR-004-command-buffer-architecture.md)
- [ADR-005: Library Not Framework](ADR/ADR-005-library-not-framework.md)