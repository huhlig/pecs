# PECS - Persistent Entity Component System

[![Crates.io](https://img.shields.io/crates/v/pecs.svg)](https://crates.io/crates/pecs)
[![Documentation](https://docs.rs/pecs/badge.svg)](https://docs.rs/pecs)
[![License](https://img.shields.io/crates/l/pecs.svg)](LICENSE)
[![Build Status](https://img.shields.io/github/workflow/status/yourusername/pecs/CI)](https://github.com/yourusername/pecs/actions)

A high-performance, minimalist Entity Component System (ECS) library for Rust with integrated persistence capabilities.

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
struct Position { x: f32, y: f32 }
impl Component for Position {}

#[derive(Debug)]
struct Velocity { x: f32, y: f32 }
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

PECS is designed for high performance:

| Operation | Performance |
|-----------|-------------|
| Entity spawn | ~118-281ns |
| Entity lookup | ~5ns |
| Component access | ~5ns |
| Query iteration | 50-100M entities/sec |
| Binary persistence | ~0.36ms per 1000 entities |

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
world.save("world.pecs")?;
let world = World::load("world.pecs")?;

// JSON format (human-readable)
world.save_with("world.json", "json")?;
let world = World::load_with("world.json", "json")?;

// Streaming API for large worlds
use std::fs::File;
let mut file = File::create("world.pecs")?;
world.save_binary(&mut file)?;
```

Features:
- Version-aware serialization
- Checksum validation
- Transient component support
- Custom plugin system

## Examples

Check out the [examples](examples/) directory for complete applications:

- **[Basic Usage](examples/basic.rs)** - Entity and component basics
- **[Persistence](examples/persistence.rs)** - Save and load worlds
- **[Command Buffers](examples/commands.rs)** - Deferred operations
- **[Benchmarks](benches/benchmarks.rs)** - Performance testing

## Roadmap

### Phase 1: Core ECS ✅ Complete
- [x] Entity management with dual ID system
- [x] Component storage with archetypes
- [x] Command buffer system
- [x] Basic world operations

### Phase 2: Persistence ✅ Complete
- [x] Binary serialization format
- [x] JSON serialization format
- [x] Plugin system
- [x] Version migration support
- [x] Change tracking

### Phase 3: Polish & Optimization 🟡 In Progress
- [x] Performance optimization (58% faster serialization!)
- [ ] Query system integration
- [ ] Comprehensive documentation
- [ ] Example applications
- [ ] API refinement

### Phase 4: Release 🔜 Coming Soon
- [ ] Beta testing
- [ ] Community feedback
- [ ] Final polish
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

| Feature | PECS | Bevy ECS | Specs | Legion |
|---------|------|----------|-------|--------|
| Archetype-based | ✅ | ✅ | ❌ | ✅ |
| Built-in Persistence | ✅ | ❌ | ❌ | ❌ |
| Dual ID System | ✅ | ❌ | ❌ | ❌ |
| Zero Dependencies | ✅ | ❌ | ❌ | ❌ |
| Library (not framework) | ✅ | ❌ | ✅ | ✅ |
| Query Performance | High | High | Medium | High |

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Acknowledgments

PECS is inspired by:
- [Bevy ECS](https://github.com/bevyengine/bevy) - Archetype-based design
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

Made with ❤️ and 🦀 by the PECS team