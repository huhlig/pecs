//! PECS - Persistent Entity Component System
//!
//! A high-performance, minimalist ECS library for Rust with integrated persistence capabilities.
//!
//! # Features
//!
//! - **Dual ID System**: Fast ephemeral IDs for runtime, stable UUIDs for persistence
//! - **Archetype-based Storage**: Cache-friendly component storage
//! - **Ergonomic Queries**: Type-safe, iterator-based component access
//! - **Command Buffers**: Thread-safe deferred operations
//! - **Pluggable Persistence**: Flexible save/load system
//!
//! # Quick Start
//!
//! ```
//! use pecs::prelude::*;
//!
//! // Define components
//! #[derive(Debug)]
//! struct Position { x: f32, y: f32 }
//! impl Component for Position {}
//!
//! #[derive(Debug)]
//! struct Velocity { x: f32, y: f32 }
//! impl Component for Velocity {}
//!
//! // Create a world
//! let mut world = World::new();
//!
//! // Spawn entities with components
//! let entity = world.spawn()
//!     .with(Position { x: 0.0, y: 0.0 })
//!     .with(Velocity { x: 1.0, y: 0.0 })
//!     .id();
//!
//! // Check if entity is alive
//! assert!(world.is_alive(entity));
//!
//! // Despawn an entity
//! world.despawn(entity);
//! assert!(!world.is_alive(entity));
//! ```
//!
//! # Architecture
//!
//! PECS is designed as a library, not a framework. It provides the core ECS
//! functionality without imposing a specific application structure.
//!
//! ## Core Modules
//!
//! - [`entity`]: Entity lifecycle management with dual ID system
//! - [`component`]: Component storage and management
//! - [`query`]: Type-safe component queries
//! - [`command`]: Thread-safe command buffers
//! - [`world`]: Top-level ECS world

pub mod command;
pub mod component;
pub mod entity;
pub mod query;
pub mod world;

/// Convenient re-exports for common types.
///
/// Use `use pecs::prelude::*;` to import all commonly used types.
pub mod prelude {
    pub use crate::command::{Command, CommandBuffer};
    pub use crate::component::Component;
    pub use crate::entity::{EntityId, StableId};
    pub use crate::world::World;
}

// Re-export commonly used types
pub use command::{Command, CommandBuffer};
pub use component::Component;
pub use entity::{EntityId, EntityManager, StableId};
pub use query::{Fetch, Filter, Query};
pub use world::World;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_entity_operations() {
        let mut manager = EntityManager::new();

        // Spawn entities
        let e1 = manager.spawn();
        let e2 = manager.spawn();

        assert!(manager.is_alive(e1));
        assert!(manager.is_alive(e2));
        assert_eq!(manager.len(), 2);

        // Despawn an entity
        manager.despawn(e1);
        assert!(!manager.is_alive(e1));
        assert!(manager.is_alive(e2));
        assert_eq!(manager.len(), 1);
    }

    #[test]
    fn stable_id_persistence() {
        let mut manager = EntityManager::new();
        let (entity_id, stable_id) = manager.spawn_with_stable_id();

        // Stable ID should be retrievable
        assert_eq!(manager.get_stable_id(entity_id), Some(stable_id));
        assert_eq!(manager.get_entity_id(stable_id), Some(entity_id));

        // After despawn, mappings should be gone
        manager.despawn(entity_id);
        assert_eq!(manager.get_stable_id(entity_id), None);
        assert_eq!(manager.get_entity_id(stable_id), None);
    }
}

// Made with Bob
