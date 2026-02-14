# PECS Examples

This directory contains examples demonstrating various features and best practices of the PECS library.

## Basic Examples

### 01. Hello World
**File**: `01_hello_world.rs`  
**Run**: `cargo run --example 01_hello_world`

The simplest possible PECS example. Demonstrates:
- Creating a World
- Spawning entities
- Using stable IDs for entity identification
- Checking entity status
- Iterating over entities

### 02. Command Buffer
**File**: `02_command_buffer.rs`  
**Run**: `cargo run --example 02_command_buffer`

Demonstrates the command buffer system for deferred operations:
- Recording operations in a command buffer
- Batching operations for better performance
- Applying commands at a safe point
- Thread-safe operation recording (conceptual)

### 03. Persistence
**File**: `03_persistence.rs`  
**Run**: `cargo run --example 03_persistence`

Shows how to save and load worlds:
- Binary format serialization
- JSON format for human-readable saves
- Stable ID preservation across save/load
- In-memory and file-based persistence

## Performance Examples

### 04. Performance Best Practices
**File**: `04_performance.rs`  
**Run**: `cargo run --example 04_performance --release`

Demonstrates performance optimization techniques:
- Pre-allocating capacity vs dynamic growth
- Batch operations with command buffers
- Entity lifecycle performance
- Stable ID lookup performance
- Performance measurement techniques

**Note**: Always run with `--release` for accurate performance measurements!

### 05. Large-Scale World Management
**File**: `05_large_scale.rs`  
**Run**: `cargo run --example 05_large_scale --release`

Shows how to manage large numbers of entities efficiently:
- Creating 100,000+ entities
- Batch spawning strategies
- Efficient iteration
- Selective despawning
- Persistence at scale
- Memory management tips

## Running Examples

To run any example:

```bash
# Debug mode (slower, more checks)
cargo run --example <example_name>

# Release mode (optimized, for performance testing)
cargo run --example <example_name> --release
```

For example:
```bash
cargo run --example 01_hello_world
cargo run --example 04_performance --release
```

## Example Categories

### Getting Started
- `01_hello_world` - Start here!
- `02_command_buffer` - Learn about deferred operations

### Persistence
- `03_persistence` - Save and load worlds

### Performance
- `04_performance` - Optimization techniques
- `05_large_scale` - Scaling to large entity counts

## Current Limitations

**Note**: The current examples work with the available API. Some features are planned for future releases:

- **Component Access**: Direct component insertion/removal/access methods are planned for Phase 3 Week 7-8
- **Query System**: Full query integration with World is planned for Phase 3 Week 7-8
- **Components in Examples**: Examples will be expanded with component usage once the API is complete

See `docs/dev/API_GAPS.md` for details on planned API improvements.

## Contributing Examples

When adding new examples:

1. Name files with a number prefix: `XX_descriptive_name.rs`
2. Include comprehensive doc comments at the top
3. Add a section to this README
4. Test in both debug and release modes
5. Keep examples focused on one concept
6. Include performance tips where relevant

## Performance Tips Summary

From the examples, key performance tips include:

1. **Pre-allocate capacity** when you know entity count
2. **Use command buffers** for batch operations
3. **Run with --release** for accurate performance measurements
4. **Batch operations** reduce allocation overhead
5. **Stable ID lookups** are O(1) but have overhead compared to direct EntityId use

## Next Steps

After exploring these examples:

1. Read the [Getting Started Guide](../docs/GETTING_STARTED.md)
2. Review [Core Concepts](../docs/CONCEPTS.md)
3. Check out [Performance Guide](../docs/PERFORMANCE.md)
4. Explore [Advanced Features](../docs/ADVANCED_FEATURES.md)
5. Learn about [Persistence](../docs/PERSISTENCE.md)

## Questions or Issues?

- Check the documentation in `docs/`
- Review the API reference: `cargo doc --open`
- Look at the test suite in `tests/`
- See the benchmarks in `benches/`