# ADR-005: Library vs Framework Approach

**Status**: Accepted
**Date**: 2026-02-13
**Deciders**: Development Team
**Related**: PRD Section 5.2, PRD Section 1.2

## Context

When building an Entity Component System, there's a fundamental choice between two architectural approaches:

1. **Framework**: Provides a complete application structure with inversion of control
   - Defines how applications are organized
   - Manages the main loop and system execution
   - Provides scheduling, dependency injection, and lifecycle management
   - Examples: Bevy, Amethyst

2. **Library**: Provides focused functionality that integrates into existing code
   - Users control application structure
   - No opinions about main loop or execution model
   - Minimal API surface focused on core features
   - Examples: hecs, EnTT

This decision affects:
- **User Experience**: How developers interact with PECS
- **Flexibility**: What architectural patterns users can employ
- **Adoption**: Barriers to entry and integration with existing projects
- **Maintenance**: Scope of features and complexity
- **Ecosystem**: How PECS fits into the Rust ecosystem

## Decision

PECS will be built as a **library, not a framework**. We will provide focused ECS and persistence functionality without imposing architectural patterns or controlling application flow.

### Core Principles

#### 1. No Inversion of Control
Users control their application structure and main loop:

```rust
// ✓ Library approach - user controls flow
fn main() {
    let mut world = World::new();
    
    loop {
        // User decides when and how to update
        update_physics(&mut world);
        update_rendering(&mut world);
        
        if should_quit() {
            break;
        }
    }
}

// ✗ Framework approach - framework controls flow
fn main() {
    App::new()
        .add_system(update_physics)
        .add_system(update_rendering)
        .run(); // Framework takes control
}
```

#### 2. Minimal API Surface
Focus on core ECS and persistence features:

**Included:**
- Entity and component management
- Query system
- Command buffers
- Persistence (save/load)
- Resource management

**Excluded:**
- System scheduling
- Dependency injection
- Event systems
- Asset management
- Rendering abstractions
- Input handling

#### 3. Composable with Other Libraries
PECS should work alongside other libraries:

```rust
// Use PECS with any game loop library
use winit::event_loop::EventLoop;
use pecs::World;

fn main() {
    let event_loop = EventLoop::new();
    let mut world = World::new();
    
    event_loop.run(move |event, _, control_flow| {
        // PECS integrates naturally
        update_game_logic(&mut world);
    });
}
```

#### 4. Explicit Over Implicit
All operations are explicit and visible:

```rust
// ✓ Explicit - clear what's happening
world.query::<(&mut Position, &Velocity)>()
    .for_each(|(pos, vel)| {
        pos.x += vel.x;
    });

// ✗ Implicit - hidden system registration and execution
#[system]
fn update_positions(pos: &mut Position, vel: &Velocity) {
    pos.x += vel.x;
}
```

#### 5. Zero Configuration
No required setup or configuration:

```rust
// ✓ Works immediately
let mut world = World::new();
let entity = world.spawn()
    .with(Position::default())
    .build();

// ✗ Requires configuration
let mut app = App::new();
app.configure_ecs()
    .with_threading(4)
    .with_scheduling(Schedule::Parallel)
    .build();
```

### What PECS Provides

#### Core ECS Functionality
```rust
// Entity management
let entity = world.spawn()
    .with(Position { x: 0.0, y: 0.0 })
    .with(Velocity { x: 1.0, y: 0.0 })
    .build();

// Queries
for (pos, vel) in world.query::<(&mut Position, &Velocity)>() {
    pos.x += vel.x;
}

// Command buffers
let mut commands = world.commands();
commands.spawn().with(Position::default()).build();
commands.apply(&mut world);
```

#### Persistence
```rust
// Save/load world state
world.save("game.save")?;
let loaded = World::load("game.save")?;
```

#### Resources (Singletons)
```rust
// Global state management
world.insert_resource(GameConfig { difficulty: 5 });
let config = world.resource::<GameConfig>();
```

### What Users Provide

#### Application Structure
Users decide how to organize their code:
- Module structure
- System organization
- Update order
- Main loop

#### System Execution
Users control when and how systems run:
```rust
fn main() {
    let mut world = World::new();
    
    loop {
        // User decides execution order
        physics_system(&mut world);
        ai_system(&mut world);
        rendering_system(&mut world);
    }
}
```

#### Scheduling
Users implement their own scheduling if needed:
```rust
struct Scheduler {
    systems: Vec<Box<dyn System>>,
}

impl Scheduler {
    fn run(&mut self, world: &mut World) {
        for system in &mut self.systems {
            system.run(world);
        }
    }
}
```

## Consequences

### Positive
- **Flexibility**: Users can structure applications however they want
- **Integration**: Easy to add to existing projects
- **Learning Curve**: Simpler mental model, less to learn
- **Maintenance**: Smaller scope, easier to maintain
- **Performance**: No framework overhead
- **Composability**: Works with any other libraries
- **Adoption**: Lower barrier to entry
- **Rust Ecosystem Fit**: Aligns with Rust's library-first culture

### Negative
- **Boilerplate**: Users may need to write more setup code
- **Consistency**: Different projects may use PECS differently
- **Features**: Some convenience features not provided
- **Examples**: Need more examples showing different usage patterns
- **Ecosystem**: No "blessed" way to build applications

### Neutral
- **Community**: May develop third-party frameworks on top of PECS
- **Documentation**: Must show multiple usage patterns
- **Comparison**: Different trade-offs than framework-based ECS solutions

## Alternatives Considered

### Alternative 1: Full Framework Approach
```rust
App::new()
    .add_plugin(PhysicsPlugin)
    .add_system(update_positions)
    .add_system(update_velocities)
    .run();
```
- **Pros**:
  - Batteries included
  - Consistent application structure
  - Built-in scheduling and parallelism
  - Easier for beginners
  - Rich plugin ecosystem
- **Cons**:
  - Opinionated architecture
  - Harder to integrate with existing code
  - Framework lock-in
  - Larger scope and complexity
  - Slower iteration on core features
- **Rejected because**: Conflicts with flexibility goals; Rust ecosystem prefers libraries

### Alternative 2: Hybrid Approach (Library + Optional Framework)
```rust
// Core library
let mut world = World::new();

// Optional framework layer
let mut app = App::new(world)
    .add_system(physics_system)
    .run();
```
- **Pros**:
  - Best of both worlds
  - Flexibility for advanced users
  - Convenience for beginners
- **Cons**:
  - Significantly more complex
  - Two APIs to maintain
  - Unclear which to use when
  - Splits community
  - Doubles documentation burden
- **Rejected because**: Complexity not justified; users can build frameworks on top

### Alternative 3: Minimal Framework (System Runner Only)
```rust
let mut runner = SystemRunner::new(world);
runner.add_system(physics_system);
runner.add_system(ai_system);
runner.run();
```
- **Pros**:
  - Simple system execution
  - Still flexible
  - Reduces boilerplate
- **Cons**:
  - Still opinionated about execution model
  - Adds complexity to core library
  - Users may not need it
  - Can be built as separate crate
- **Rejected because**: Can be provided as optional companion crate if needed

### Alternative 4: Plugin Architecture
```rust
world.add_plugin(PhysicsPlugin);
world.add_plugin(RenderingPlugin);
```
- **Pros**:
  - Modular design
  - Reusable components
  - Community contributions
- **Cons**:
  - Requires plugin system infrastructure
  - Adds complexity
  - Not needed for core functionality
  - Can be built on top of library
- **Rejected because**: Plugins can be regular Rust crates; no special support needed

## Implementation Notes

### API Design Guidelines

#### Keep It Simple
```rust
// ✓ Simple, direct API
world.spawn().with(Position::default()).build();

// ✗ Over-engineered
world.entity_builder()
    .configure(|config| config.with_component(Position::default()))
    .finalize();
```

#### Avoid Magic
```rust
// ✓ Explicit
world.query::<(&Position, &Velocity)>()
    .for_each(|(pos, vel)| { /* ... */ });

// ✗ Magic (hidden registration)
#[auto_query]
fn system(pos: &Position, vel: &Velocity) { /* ... */ }
```

#### Provide Escape Hatches
```rust
// Allow advanced users to access internals when needed
impl World {
    pub fn archetypes(&self) -> &[Archetype] { /* ... */ }
    pub fn entity_location(&self, entity: EphemeralId) -> Option<EntityLocation> { /* ... */ }
}
```

### Documentation Strategy

Provide examples for common patterns:
- Simple game loop
- Multi-threaded execution
- Integration with game engines
- Custom scheduling
- Event handling patterns

### Ecosystem Integration

Show how to use PECS with popular libraries:
- `winit` for windowing
- `wgpu` for rendering
- `rapier` for physics
- `tokio` for async
- `bevy_ecs` migration guide

## Usage Examples

### Simple Game Loop
```rust
fn main() {
    let mut world = World::new();
    
    // Setup
    spawn_player(&mut world);
    spawn_enemies(&mut world);
    
    // Game loop
    loop {
        handle_input(&mut world);
        update_physics(&mut world);
        update_ai(&mut world);
        render(&world);
        
        if should_quit() {
            break;
        }
    }
}

fn update_physics(world: &mut World) {
    for (pos, vel) in world.query::<(&mut Position, &Velocity)>() {
        pos.x += vel.x;
        pos.y += vel.y;
    }
}
```

### With Custom Scheduler
```rust
struct GameScheduler {
    physics_systems: Vec<Box<dyn Fn(&mut World)>>,
    render_systems: Vec<Box<dyn Fn(&World)>>,
}

impl GameScheduler {
    fn update(&mut self, world: &mut World) {
        // Run physics systems
        for system in &self.physics_systems {
            system(world);
        }
        
        // Run render systems
        for system in &self.render_systems {
            system(world);
        }
    }
}
```

### Integration with Existing Engine
```rust
struct MyEngine {
    world: World,
    renderer: Renderer,
    physics: PhysicsEngine,
}

impl MyEngine {
    fn update(&mut self, dt: f32) {
        // PECS integrates naturally
        self.update_game_logic(dt);
        
        // Use other engine components
        self.physics.step(dt);
        self.renderer.render(&self.world);
    }
    
    fn update_game_logic(&mut self, dt: f32) {
        for (pos, vel) in self.world.query::<(&mut Position, &Velocity)>() {
            pos.x += vel.x * dt;
        }
    }
}
```

## Future Considerations

### Optional Companion Crates
If demand exists, provide optional crates:
- `pecs-scheduler`: System scheduling utilities
- `pecs-events`: Event system
- `pecs-derive`: Derive macros for common patterns
- `pecs-parallel`: Parallel query execution helpers

These remain separate from core PECS library.

### Framework Built on PECS
Community can build frameworks on top:
```rust
// Hypothetical third-party framework
use pecs::World;
use pecs_framework::App;

App::new()
    .with_world(World::new())
    .add_system(physics_system)
    .run();
```

## References

- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [hecs - Minimal ECS](https://github.com/Ralith/hecs)
- [EnTT - Library Approach](https://github.com/skypjack/entt)
- [Bevy - Framework Approach](https://bevyengine.org/)
- [The Rust Programming Language - Libraries vs Binaries](https://doc.rust-lang.org/book/ch07-01-packages-and-crates.html)
- PRD Section 5.2: Design Principles
- PRD Section 1.2: Goals