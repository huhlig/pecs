# PECS - Persistent Entity Component System

[![Crates.io](https://img.shields.io/crates/v/pecs.svg)](https://crates.io/crates/pecs)
[![Documentation](https://docs.rs/pecs/badge.svg)](https://huhlig.github.io/pecs)
[![License](https://img.shields.io/crates/l/pecs.svg)](LICENSE)
[![Build Status](https://img.shields.io/github/workflow/status/yourusername/pecs/CI)](https://github.com/yourusername/pecs/actions)

A high-performance, minimalist Entity Component System (ECS) library for Rust with integrated persistence capabilities.

**Current Status**: Phase 3 Complete (202 tests passing) - Ready for Phase 4 (Release Preparation)

## Features

- 🚀 **High Performance**: Archetype-based storage, cache-friendly queries, optimized for modern CPUs
- 🔄 **Dual ID System**: Fast ephemeral IDs for runtime, stable UUIDs for persistence
- 💾 **Pluggable Persistence**: Binary and JSON formats out of the box, extensible for custom formats
- 🧵 **Thread-Safe Commands**: Deferred operations via command buffers for parallel systems
- 📦 **Zero Dependencies**: Core library has no dependencies (serde optional for JSON)
- 🎯 **Type-Safe Queries**: Compile-time validated component access
- 📚 **Library, Not Framework**: Integrate into your application, don't build around it

## Quick Start

Add PECS to your `Cargo.toml`:

```toml
[dependencies]
pecs = "0.1.0"
```

Basic usage:

```rust
use pecs::prelude::*;

// Define components
#[derive(Debug)]
struct Position {
    x: f32,
    y: f32
}
impl Component for Position {}

#[derive(Debug)]
struct Velocity {
    x: f32,
    y: f32
}
impl Component for Velocity {}

fn main() {
    // Create a world
    let mut world = World::new();

    // Spawn entities with components
    let player = world.spawn()
        .with(Position { x: 0.0, y: 0.0 })
        .with(Velocity { x: 1.0, y: 0.0 })
        .id();

    println!("Created player: {}", player);
    println!("Total entities: {}", world.len());

    // Save the world
    world.save("world.pecs").unwrap();

    // Load the world
    let loaded = World::load("world.pecs").unwrap();
    println!("Loaded {} entities", loaded.len());
}
```

## Documentation

- **[Getting Started Guide](docs/GETTING_STARTED.md)** - Learn the basics
- **[Core Concepts](docs/CONCEPTS.md)** - Understand the architecture
- **[Performance Guide](docs/PERFORMANCE.md)** - Optimize your usage
- **[API Reference](https://docs.rs/pecs)** - Complete API documentation
- **[Examples](examples/)** - Browse example applications

## Performance

PECS is designed for high performance with extensive optimizations:

| Operation             | Performance               | Status       |
|-----------------------|---------------------------|--------------|
| Entity spawn (single) | ~538ns                    | ✅ Optimized  |
| Entity spawn (batch)  | ~118-318ns per entity     | ✅ Optimized  |
| Entity lookup         | ~5-10ns                   | ✅ Optimized  |
| Component access      | ~5ns (cache hit)          | ✅ Optimized  |
| Query iteration       | 2-5x faster (optimized)   | ✅ Optimized  |
| Binary persistence    | ~0.36ms per 1000 entities | ✅ 58% faster |
| JSON persistence      | Human-readable format     | ✅ Complete   |

**Recent Optimizations** (Phase 3):

- 58% faster binary serialization (117.6µs → 48.6µs per 1000 entities)
- 2-5x faster query iteration with caching
- 10-20x faster entity location lookup (HashMap → Vec)
- 50-70% fewer allocations during entity creation

See the [Performance Guide](docs/PERFORMANCE.md) for detailed benchmarks and optimization techniques.

## Architecture

PECS uses an archetype-based storage system for optimal query performance:

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

Key design principles:

- **Dual ID System**: Fast runtime IDs + persistent UUIDs
- **Archetype Storage**: Cache-friendly component layout
- **Command Buffers**: Thread-safe deferred operations
- **Pluggable Persistence**: Flexible save/load system

See [Core Concepts](docs/CONCEPTS.md) for detailed architecture information.

## Persistence

PECS provides built-in persistence with multiple formats:

```rust
use pecs::World;

// Binary format (fast, compact)
world.save("world.pecs") ?;
let world = World::load("world.pecs") ?;

// JSON format (human-readable)
world.save_with("world.json", "json") ?;
let world = World::load_with("world.json", "json") ?;

// Streaming API for large worlds
use std::fs::File;
let mut file = File::create("world.pecs") ?;
world.save_binary( & mut file) ?;
```

Features:

- Version-aware serialization
- Checksum validation
- Transient component support
- Custom plugin system

## Examples

Check out the [examples](examples/) directory for complete applications:

- **[01_hello_world.rs](examples/01_hello_world.rs)** - Simplest PECS example
- **[02_command_buffer.rs](examples/02_command_buffer.rs)** - Deferred operations
- **[03_persistence.rs](examples/03_persistence.rs)** - Save and load worlds
- **[04_performance.rs](examples/04_performance.rs)** - Performance optimization techniques
- **[05_large_scale.rs](examples/05_large_scale.rs)** - 100,000 entity management
- **[06_simple_game.rs](examples/06_simple_game.rs)** - Asteroids-style game example

Run examples with:

```bash
cargo run --example 01_hello_world --release
```

See [examples/README.md](examples/README.md) for detailed documentation.

## Roadmap

### Phase 1: Core ECS ✅ Complete (2026-02-13)

- [x] Entity management with dual ID system
- [x] Component storage with archetypes
- [x] Query system with filters
- [x] Command buffer system
- [x] World integration and API
- [x] 94 tests passing

### Phase 2: Persistence ✅ Complete (2026-02-13)

- [x] Binary serialization format (< 0.4ms per 1000 entities)
- [x] JSON serialization format
- [x] Plugin system with delta persistence
- [x] Version migration framework
- [x] Change tracking and transient components
- [x] 164 tests passing

### Phase 3: Polish & Optimization ✅ Complete (2026-02-14)

- [x] Performance optimization (58% faster serialization, 2-5x faster queries)
- [x] Query system integration and bug fixes
- [x] Comprehensive documentation (4,500+ lines, 100% API coverage)
- [x] 6 working examples
- [x] API refinement (component access, query execution)
- [x] Error handling improvements
- [x] 202 tests passing

### Phase 4: Release 🔜 Next

- [ ] Tutorial series
- [ ] Additional complete applications
- [ ] Beta testing
- [ ] Community feedback
- [ ] Cross-platform testing
- [ ] 1.0 release

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### Development

```bash
# Clone the repository
git clone https://github.com/yourusername/pecs.git
cd pecs

# Run tests
cargo test

# Run benchmarks
cargo bench

# Check code quality
cargo clippy
cargo fmt --check
```

## Design Philosophy

PECS follows these principles:

1. **Library, Not Framework**: Integrate PECS into your application
2. **Performance First**: Zero-cost abstractions, cache-friendly design
3. **Type Safety**: Compile-time guarantees, no runtime type errors
4. **Flexibility**: Minimal constraints, maximum freedom
5. **Simplicity**: Small API surface, clear documentation

See [ADR-005: Library Not Framework](docs/ADR/ADR-005-library-not-framework.md) for details.

## Comparison with Other ECS Libraries

| Feature                 | PECS | Bevy ECS | Specs  | Legion |
|-------------------------|------|----------|--------|--------|
| Archetype-based         | ✅    | ✅        | ❌      | ✅      |
| Built-in Persistence    | ✅    | ❌        | ❌      | ❌      |
| Dual ID System          | ✅    | ❌        | ❌      | ❌      |
| Zero Dependencies       | ✅    | ❌        | ❌      | ❌      |
| Library (not framework) | ✅    | ❌        | ✅      | ✅      |
| Query Performance       | High | High     | Medium | High   |

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Acknowledgments

PECS is inspired by:

- [Bevy ECS](https://github.com/bevyengine/bevy) - Archetype-based design
- [HECS](https://github.com/Ralith/hecs) - Batch entity operations
- [Specs](https://github.com/amethyst/specs) - Component storage patterns
- [Legion](https://github.com/amethyst/legion) - Query optimization
- [EnTT](https://github.com/skypjack/entt) - Entity recycling

Special thanks to the Rust gamedev community for their insights and feedback.

## Support

- **Documentation**: [docs.rs/pecs](https://docs.rs/pecs)
- **Issues**: [GitHub Issues](https://github.com/yourusername/pecs/issues)
- **Discussions**: [GitHub Discussions](https://github.com/yourusername/pecs/discussions)
- **Discord**: [Rust Gamedev Discord](https://discord.gg/rust-gamedev)

---

Made with ❤️ and 🦀 by Wyldlands team