# ADR-004: Command Buffer Architecture

**Status**: Accepted
**Date**: 2026-02-13
**Deciders**: Development Team
**Related**: PRD Section 5.1, FR-5, NFR-3.4

## Context

Entity Component Systems face challenges when dealing with concurrent access and deferred operations:

1. **Thread Safety**: Multiple systems may need to modify the world simultaneously
2. **Structural Changes**: Adding/removing components during iteration can invalidate iterators
3. **Determinism**: Operations must execute in a predictable order for reproducibility
4. **Persistence**: Need to record operations for replay and debugging

Traditional approaches include:
- **Immediate execution with locks**: Simple but slow, prone to deadlocks
- **Deferred execution with queues**: Better performance but complex state management
- **Immutable world with transactions**: Safe but memory-intensive
- **Command buffers**: Deferred, ordered execution with minimal overhead

PECS requires thread-safe operations that support both performance and persistence replay capabilities.

## Decision

We will implement a **command buffer system** that queues operations for deferred, ordered execution.

### Core Architecture

#### Command Types
```rust
pub enum Command {
    // Entity operations
    SpawnEntity {
        stable_id: StableId,
        components: Vec<Box<dyn Component>>,
    },
    DespawnEntity {
        entity: EphemeralId,
    },
    
    // Component operations
    AddComponent {
        entity: EphemeralId,
        component: Box<dyn Component>,
    },
    RemoveComponent {
        entity: EphemeralId,
        component_type: ComponentTypeId,
    },
    
    // Resource operations
    InsertResource {
        resource: Box<dyn Resource>,
    },
    RemoveResource {
        resource_type: TypeId,
    },
}
```

#### Command Buffer
```rust
pub struct CommandBuffer {
    commands: Vec<Command>,
    entity_allocations: Vec<StableId>,
}

impl CommandBuffer {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
            entity_allocations: Vec::new(),
        }
    }
    
    // Queue operations
    pub fn spawn(&mut self) -> EntityBuilder {
        let stable_id = StableId::new();
        self.entity_allocations.push(stable_id);
        EntityBuilder::new(self, stable_id)
    }
    
    pub fn despawn(&mut self, entity: EphemeralId) {
        self.commands.push(Command::DespawnEntity { entity });
    }
    
    pub fn add_component<T: Component>(&mut self, entity: EphemeralId, component: T) {
        self.commands.push(Command::AddComponent {
            entity,
            component: Box::new(component),
        });
    }
    
    // Apply all commands to world
    pub fn apply(self, world: &mut World) {
        for command in self.commands {
            command.execute(world);
        }
    }
}
```

#### Entity Builder
```rust
pub struct EntityBuilder<'a> {
    buffer: &'a mut CommandBuffer,
    stable_id: StableId,
    components: Vec<Box<dyn Component>>,
}

impl<'a> EntityBuilder<'a> {
    pub fn with<T: Component>(mut self, component: T) -> Self {
        self.components.push(Box::new(component));
        self
    }
    
    pub fn build(self) -> StableId {
        self.buffer.commands.push(Command::SpawnEntity {
            stable_id: self.stable_id,
            components: self.components,
        });
        self.stable_id
    }
}
```

### Integration with World

```rust
impl World {
    // Get command buffer for deferred operations
    pub fn commands(&mut self) -> CommandBuffer {
        CommandBuffer::new()
    }
    
    // Immediate operations (for single-threaded use)
    pub fn spawn_immediate(&mut self) -> EntityBuilderImmediate {
        EntityBuilderImmediate::new(self)
    }
}
```

### Thread Safety Model

#### Single-Threaded Usage
```rust
// Direct world access
let entity = world.spawn_immediate()
    .with(Position::default())
    .build();

// Or use commands for consistency
let mut commands = world.commands();
commands.spawn()
    .with(Position::default())
    .build();
commands.apply(&mut world);
```

#### Multi-Threaded Usage
```rust
// Each thread gets its own command buffer
let mut commands = world.commands();

// Can be sent across threads
thread::spawn(move || {
    commands.spawn()
        .with(Position::default())
        .build();
    commands
}).join().unwrap().apply(&mut world);
```

### Command Ordering

Commands execute in the order they were recorded:
1. Entity spawns
2. Component additions
3. Component removals
4. Entity despawns

This ensures:
- Entities exist before components are added
- Components are removed before entities are despawned
- Deterministic execution order

### Persistence Integration

Command buffers can be serialized for replay:

```rust
pub struct CommandLog {
    commands: Vec<Command>,
}

impl CommandLog {
    // Record commands for persistence
    pub fn record(&mut self, buffer: &CommandBuffer) {
        self.commands.extend(buffer.commands.clone());
    }
    
    // Replay commands on a world
    pub fn replay(&self, world: &mut World) {
        for command in &self.commands {
            command.execute(world);
        }
    }
    
    // Serialize for storage
    pub fn save(&self, path: &Path) -> Result<()> {
        // Serialize command log
    }
}
```

## Consequences

### Positive
- **Thread Safety**: Command buffers can be created and populated from any thread
- **No Locks**: World access doesn't require locks during command recording
- **Deterministic**: Commands execute in predictable order
- **Replay Support**: Commands can be recorded and replayed for persistence/debugging
- **Iterator Safety**: Structural changes don't invalidate active iterators
- **Batching**: Multiple operations can be batched for efficiency
- **Composable**: Command buffers can be merged and split

### Negative
- **Delayed Execution**: Changes not visible until commands are applied
- **Memory Overhead**: Commands must be stored before execution
- **Complexity**: Two-phase execution model (record + apply)
- **Entity References**: Spawned entities can't be referenced until applied
- **Error Handling**: Errors only detected during apply phase

### Neutral
- **API Duplication**: Both immediate and deferred APIs available
- **Learning Curve**: Users must understand when to use each approach
- **Performance**: Slight overhead for command allocation, but enables parallelism

## Alternatives Considered

### Alternative 1: Immediate Execution with Locks
```rust
world.lock().spawn().with(Position::default()).build();
```
- **Pros**:
  - Simple mental model
  - Changes immediately visible
  - No deferred execution complexity
- **Cons**:
  - Lock contention in multi-threaded scenarios
  - Potential deadlocks
  - Poor performance under contention
  - Can't record for replay
- **Rejected because**: Performance and persistence requirements demand lock-free approach

### Alternative 2: Immutable World with Transactions
```rust
let new_world = world.transaction(|tx| {
    tx.spawn().with(Position::default()).build();
});
```
- **Pros**:
  - Completely safe
  - Can rollback transactions
  - No locks needed
- **Cons**:
  - High memory overhead (copying world state)
  - Complex implementation
  - Slower for large worlds
  - Doesn't match Rust's mutable borrowing patterns
- **Rejected because**: Memory overhead unacceptable for large game worlds

### Alternative 3: Event Queue System
```rust
world.send_event(SpawnEvent { components: vec![...] });
world.process_events();
```
- **Pros**:
  - Decoupled from world
  - Can filter/transform events
  - Flexible event handling
- **Cons**:
  - More complex than needed
  - Event ordering issues
  - Harder to reason about
  - Doesn't provide entity handles
- **Rejected because**: Over-engineered for the problem; command buffers are simpler

### Alternative 4: Parallel World Sharding
```rust
let shard1 = world.shard(0);
let shard2 = world.shard(1);
// Operate on shards in parallel
```
- **Pros**:
  - True parallel execution
  - No deferred operations
  - Immediate visibility
- **Cons**:
  - Complex sharding logic
  - Cross-shard references difficult
  - Doesn't help with persistence
  - Requires careful entity distribution
- **Rejected because**: Complexity doesn't justify benefits; command buffers are more flexible

## Implementation Notes

### Command Execution
```rust
impl Command {
    pub fn execute(self, world: &mut World) {
        match self {
            Command::SpawnEntity { stable_id, components } => {
                let entity = world.spawn_with_stable_id(stable_id);
                for component in components {
                    world.add_component_boxed(entity, component);
                }
            }
            Command::DespawnEntity { entity } => {
                world.despawn(entity);
            }
            Command::AddComponent { entity, component } => {
                world.add_component_boxed(entity, component);
            }
            Command::RemoveComponent { entity, component_type } => {
                world.remove_component_by_type(entity, component_type);
            }
            Command::InsertResource { resource } => {
                world.insert_resource_boxed(resource);
            }
            Command::RemoveResource { resource_type } => {
                world.remove_resource_by_type(resource_type);
            }
        }
    }
}
```

### Entity Handle System
For referencing entities before they're spawned:

```rust
pub struct EntityHandle {
    stable_id: StableId,
}

impl CommandBuffer {
    pub fn spawn(&mut self) -> (EntityHandle, EntityBuilder) {
        let stable_id = StableId::new();
        let handle = EntityHandle { stable_id };
        let builder = EntityBuilder::new(self, stable_id);
        (handle, builder)
    }
}

// Use handle to reference entity in other commands
let (handle, builder) = commands.spawn();
builder.with(Position::default()).build();

commands.add_component(handle, Velocity::default());
```

### Command Buffer Merging
```rust
impl CommandBuffer {
    pub fn merge(&mut self, other: CommandBuffer) {
        self.commands.extend(other.commands);
        self.entity_allocations.extend(other.entity_allocations);
    }
}
```

### Optimization Strategies
- Pre-allocate command buffer capacity based on typical usage
- Use object pools for command allocation
- Batch similar commands for cache efficiency
- Compress command sequences (e.g., spawn + add components → single spawn with components)

### Error Handling
```rust
pub enum CommandError {
    EntityNotFound(EphemeralId),
    ComponentTypeMismatch,
    InvalidOperation,
}

impl CommandBuffer {
    pub fn apply_checked(self, world: &mut World) -> Result<(), Vec<CommandError>> {
        let mut errors = Vec::new();
        for command in self.commands {
            if let Err(e) = command.execute_checked(world) {
                errors.push(e);
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}
```

## Performance Characteristics

| Operation | Time Complexity | Notes |
|-----------|----------------|-------|
| Record command | O(1) | Push to vector |
| Apply commands | O(n) | n = number of commands |
| Spawn entity | O(1) amortized | Per command |
| Add component | O(m) | m = components to move (archetype change) |
| Remove component | O(m) | m = components to move |
| Merge buffers | O(n) | n = commands in merged buffer |

## Usage Examples

### Basic Usage
```rust
let mut commands = world.commands();

// Spawn entities
commands.spawn()
    .with(Position { x: 0.0, y: 0.0 })
    .with(Velocity { x: 1.0, y: 0.0 })
    .build();

// Modify existing entities
commands.add_component(entity, Health { value: 100 });
commands.remove_component::<Dead>(entity);

// Apply all commands
commands.apply(&mut world);
```

### Multi-Threaded Usage
```rust
use rayon::prelude::*;

let buffers: Vec<CommandBuffer> = (0..10)
    .into_par_iter()
    .map(|i| {
        let mut commands = CommandBuffer::new();
        commands.spawn()
            .with(Position { x: i as f32, y: 0.0 })
            .build();
        commands
    })
    .collect();

// Merge and apply all buffers
let mut merged = CommandBuffer::new();
for buffer in buffers {
    merged.merge(buffer);
}
merged.apply(&mut world);
```

### With Entity Handles
```rust
let mut commands = world.commands();

let (player_handle, builder) = commands.spawn();
builder.with(Player).with(Position::default()).build();

let (weapon_handle, builder) = commands.spawn();
builder.with(Weapon).with(Parent(player_handle)).build();

commands.apply(&mut world);
```

## References

- [Bevy Commands](https://docs.rs/bevy_ecs/latest/bevy_ecs/system/struct.Commands.html)
- [Legion Command Buffer](https://docs.rs/legion/latest/legion/world/struct.CommandBuffer.html)
- [Command Pattern](https://en.wikipedia.org/wiki/Command_pattern)
- [CQRS Pattern](https://martinfowler.com/bliki/CQRS.html)
- PRD Section 5.1: Command Buffer System
- PRD FR-5: Thread Safety
- PRD NFR-3.4: Deterministic behavior