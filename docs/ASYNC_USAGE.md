# Using PECS World in Async Contexts

This guide explains how to properly use the PECS `World` in asynchronous and multi-threaded contexts.

## Table of Contents

- [Understanding Thread Safety](#understanding-thread-safety)
- [The Command Buffer Pattern](#the-command-buffer-pattern)
- [Async/Await Integration](#asyncawait-integration)
- [Multi-threaded Patterns](#multi-threaded-patterns)
- [Best Practices](#best-practices)
- [Common Pitfalls](#common-pitfalls)

## Understanding Thread Safety

### Current Design

`World` is **intentionally NOT `Send` or `Sync`** by design. This encourages safe patterns for concurrent access:

- **Not `Send`**: Cannot be moved between threads directly
- **Not `Sync`**: Cannot be shared between threads via `Arc<World>`

### Why This Design?

1. **Safety First**: Prevents accidental data races and undefined behavior
2. **Clear Ownership**: Forces explicit thinking about data flow
3. **Command Pattern**: Encourages deferred, batched operations
4. **Performance**: Avoids synchronization overhead in single-threaded contexts

## The Command Buffer Pattern

The recommended way to work with `World` across threads is using `CommandBuffer`.

### Basic Pattern

```rust
use pecs::prelude::*;
use std::thread;

fn main() {
    let mut world = World::new();
    
    // Create a command buffer (CommandBuffer is Send)
    let mut commands = CommandBuffer::new();
    
    // Spawn a thread that records commands
    let handle = thread::spawn(move || {
        // Record operations without touching the world
        let entity = commands.spawn();
        commands.insert(entity, Position { x: 10.0, y: 20.0 });
        commands
    });
    
    // Get the command buffer back
    let mut commands = handle.join().unwrap();
    
    // Apply all commands to the world on the main thread
    commands.apply(&mut world);
    
    assert_eq!(world.len(), 1);
}

#[derive(Debug)]
struct Position { x: f32, y: f32 }
impl Component for Position {}
```

### Key Points

- `CommandBuffer` is `Send` but not `Sync`
- Each thread gets its own command buffer
- Commands are applied to `World` on a single thread
- No locks or synchronization needed

## Async/Await Integration

### Pattern 1: Tokio with Command Buffers

```rust
use pecs::prelude::*;
use tokio::task;

#[tokio::main]
async fn main() {
    let mut world = World::new();
    
    // Spawn async tasks that create command buffers
    let handles: Vec<_> = (0..10).map(|i| {
        task::spawn_blocking(move || {
            let mut commands = CommandBuffer::new();
            let entity = commands.spawn();
            commands.insert(entity, Health { value: i * 10 });
            commands
        })
    }).collect();
    
    // Collect all command buffers
    for handle in handles {
        let mut commands = handle.await.unwrap();
        commands.apply(&mut world);
    }
    
    println!("Created {} entities", world.len());
}

#[derive(Debug)]
struct Health { value: i32 }
impl Component for Health {}
```

### Pattern 2: Channel-Based Communication

```rust
use pecs::prelude::*;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() {
    let mut world = World::new();
    let (tx, mut rx) = mpsc::unbounded_channel::<CommandBuffer>();
    
    // Spawn multiple async tasks
    for i in 0..5 {
        let tx = tx.clone();
        tokio::spawn(async move {
            let mut commands = CommandBuffer::new();
            let entity = commands.spawn();
            commands.insert(entity, Score { points: i * 100 });
            tx.send(commands).unwrap();
        });
    }
    drop(tx); // Close the channel
    
    // Apply commands as they arrive
    while let Some(mut commands) = rx.recv().await {
        commands.apply(&mut world);
    }
    
    println!("Total entities: {}", world.len());
}

#[derive(Debug)]
struct Score { points: i32 }
impl Component for Score {}
```

### Pattern 3: Actor-Based World Manager

```rust
use pecs::prelude::*;
use tokio::sync::mpsc;

/// Messages that can be sent to the world manager
enum WorldMessage {
    SpawnEntity(tokio::sync::oneshot::Sender<EntityId>),
    InsertComponent(EntityId, Box<dyn std::any::Any + Send>),
    Query(tokio::sync::oneshot::Sender<Vec<EntityId>>),
    Shutdown,
}

/// Actor that owns the World and processes messages
struct WorldManager {
    world: World,
    rx: mpsc::UnboundedReceiver<WorldMessage>,
}

impl WorldManager {
    fn new() -> (Self, mpsc::UnboundedSender<WorldMessage>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let manager = Self {
            world: World::new(),
            rx,
        };
        (manager, tx)
    }
    
    async fn run(mut self) {
        while let Some(msg) = self.rx.recv().await {
            match msg {
                WorldMessage::SpawnEntity(reply) => {
                    let entity = self.world.spawn_empty();
                    let _ = reply.send(entity);
                }
                WorldMessage::InsertComponent(entity, component) => {
                    // Handle component insertion
                    // (requires type-specific handling)
                }
                WorldMessage::Query(reply) => {
                    let entities: Vec<_> = self.world
                        .iter_entities()
                        .map(|(id, _)| id)
                        .collect();
                    let _ = reply.send(entities);
                }
                WorldMessage::Shutdown => break,
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let (manager, tx) = WorldManager::new();
    
    // Spawn the world manager task
    let manager_handle = tokio::spawn(manager.run());
    
    // Spawn entities from multiple tasks
    let mut handles = vec![];
    for _ in 0..10 {
        let tx = tx.clone();
        let handle = tokio::spawn(async move {
            let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
            tx.send(WorldMessage::SpawnEntity(reply_tx)).unwrap();
            reply_rx.await.unwrap()
        });
        handles.push(handle);
    }
    
    // Wait for all spawns
    for handle in handles {
        let entity = handle.await.unwrap();
        println!("Spawned entity: {:?}", entity);
    }
    
    // Query entities
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    tx.send(WorldMessage::Query(reply_tx)).unwrap();
    let entities = reply_rx.await.unwrap();
    println!("Total entities: {}", entities.len());
    
    // Shutdown
    tx.send(WorldMessage::Shutdown).unwrap();
    manager_handle.await.unwrap();
}
```

## Multi-threaded Patterns

### Pattern 1: Parallel System Execution with Rayon

```rust
use pecs::prelude::*;
use rayon::prelude::*;

fn main() {
    let mut world = World::new();
    
    // Setup: Create entities
    for i in 0..1000 {
        world.spawn()
            .with(Position { x: i as f32, y: 0.0 })
            .with(Velocity { x: 1.0, y: 0.0 })
            .id();
    }
    
    // Collect entity data for parallel processing
    let entities: Vec<_> = world.query::<(&Position, &Velocity)>()
        .map(|(pos, vel)| (*pos, *vel))
        .collect();
    
    // Process in parallel
    let updates: Vec<_> = entities.par_iter()
        .map(|(pos, vel)| {
            Position {
                x: pos.x + vel.x,
                y: pos.y + vel.y,
            }
        })
        .collect();
    
    // Apply updates back to world (single-threaded)
    let mut query = world.query::<&mut Position>();
    for (i, pos) in query.enumerate() {
        *pos = updates[i];
    }
}

#[derive(Debug, Clone, Copy)]
struct Position { x: f32, y: f32 }
impl Component for Position {}

#[derive(Debug, Clone, Copy)]
struct Velocity { x: f32, y: f32 }
impl Component for Velocity {}
```

### Pattern 2: Thread Pool with Command Buffers

```rust
use pecs::prelude::*;
use std::sync::mpsc;
use std::thread;

fn main() {
    let mut world = World::new();
    let (tx, rx) = mpsc::channel::<CommandBuffer>();
    
    // Spawn worker threads
    let workers: Vec<_> = (0..4).map(|worker_id| {
        let tx = tx.clone();
        thread::spawn(move || {
            let mut commands = CommandBuffer::new();
            
            // Simulate work
            for i in 0..100 {
                let entity = commands.spawn();
                commands.insert(entity, WorkerId { 
                    worker: worker_id,
                    item: i,
                });
            }
            
            tx.send(commands).unwrap();
        })
    }).collect();
    
    drop(tx); // Close sender
    
    // Collect and apply all commands
    for mut commands in rx {
        commands.apply(&mut world);
    }
    
    // Wait for workers
    for worker in workers {
        worker.join().unwrap();
    }
    
    println!("Processed {} entities", world.len());
}

#[derive(Debug)]
struct WorkerId { worker: usize, item: usize }
impl Component for WorkerId {}
```

## Best Practices

### 1. Keep World on Main Thread

```rust
// ✅ GOOD: World stays on main thread
let mut world = World::new();
let commands = process_in_background();
commands.apply(&mut world);

// ❌ BAD: Trying to move World to another thread
// This won't compile!
// thread::spawn(move || {
//     world.spawn_empty();
// });
```

### 2. Use Command Buffers for Deferred Operations

```rust
// ✅ GOOD: Record commands, apply later
let mut commands = CommandBuffer::new();
for i in 0..1000 {
    let entity = commands.spawn();
    commands.insert(entity, Data { value: i });
}
commands.apply(&mut world);

// ❌ BAD: Immediate operations in loop
// Less efficient due to repeated archetype lookups
for i in 0..1000 {
    world.spawn().with(Data { value: i }).id();
}
```

### 3. Batch Command Application

```rust
// ✅ GOOD: Collect all commands, apply once
let command_buffers: Vec<_> = tasks.into_iter()
    .map(|task| task.execute())
    .collect();

for mut commands in command_buffers {
    commands.apply(&mut world);
}

// ❌ BAD: Applying commands one at a time in async context
// Can cause context switching overhead
for task in tasks {
    let commands = task.execute().await;
    commands.apply(&mut world);
}
```

### 4. Separate Read and Write Phases

```rust
// ✅ GOOD: Read phase, then write phase
let data: Vec<_> = world.query::<&Position>()
    .map(|pos| calculate_new_position(pos))
    .collect();

for (mut pos, new_pos) in world.query::<&mut Position>().zip(data) {
    *pos = new_pos;
}

// ❌ BAD: Interleaved reads and writes
// Can cause borrow checker issues
```

## Common Pitfalls

### Pitfall 1: Trying to Share World Directly

```rust
// ❌ This won't compile - World is not Sync
// let world = Arc::new(Mutex::new(World::new()));
// thread::spawn(move || {
//     let mut world = world.lock().unwrap();
//     world.spawn_empty();
// });

// ✅ Use command buffers instead
let mut world = World::new();
let handle = thread::spawn(|| {
    let mut commands = CommandBuffer::new();
    commands.spawn();
    commands
});
let mut commands = handle.join().unwrap();
commands.apply(&mut world);
```

### Pitfall 2: Holding References Across Await Points

```rust
// ❌ BAD: Reference held across await
// async fn process(world: &mut World) {
//     let entity = world.spawn_empty();
//     some_async_operation().await; // Error: reference held across await
//     world.despawn(entity);
// }

// ✅ GOOD: Use command buffer
async fn process() -> CommandBuffer {
    let mut commands = CommandBuffer::new();
    let entity = commands.spawn();
    some_async_operation().await;
    commands.despawn(entity);
    commands
}
```

### Pitfall 3: Forgetting to Apply Commands

```rust
// ❌ BAD: Commands created but never applied
fn create_entities() -> CommandBuffer {
    let mut commands = CommandBuffer::new();
    commands.spawn();
    commands.spawn();
    commands // Returned but never applied!
}

// ✅ GOOD: Ensure commands are applied
fn create_entities(world: &mut World) {
    let mut commands = CommandBuffer::new();
    commands.spawn();
    commands.spawn();
    commands.apply(world); // Applied immediately
}
```

## Performance Considerations

### Command Buffer Overhead

Command buffers add a small overhead:
- Memory allocation for command storage
- Indirection through trait objects
- Deferred execution

**When to use:**
- Multi-threaded scenarios (required)
- Batching many operations
- Async contexts

**When to avoid:**
- Single-threaded, immediate operations
- Hot loops with few operations
- Performance-critical paths

### Optimal Batch Sizes

```rust
// For best performance, batch 100-1000 operations per command buffer
const BATCH_SIZE: usize = 500;

let mut commands = CommandBuffer::with_capacity(BATCH_SIZE);
for i in 0..BATCH_SIZE {
    commands.spawn();
}
commands.apply(&mut world);
```

## Summary

- **Use `CommandBuffer`** for all multi-threaded and async operations
- **Keep `World` on a single thread** (typically the main thread)
- **Batch operations** for better performance
- **Use channels or actors** for complex async patterns
- **Separate read and write phases** to avoid borrow checker issues

The PECS design prioritizes safety and clarity over convenience. While it requires more explicit handling of concurrency, it prevents entire classes of bugs and makes data flow obvious.