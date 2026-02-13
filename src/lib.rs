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
//! use pecs::entity::EntityManager;
//!
//! // Create an entity manager
//! let mut manager = EntityManager::new();
//!
//! // Spawn entities
//! let entity1 = manager.spawn();
//! let entity2 = manager.spawn();
//!
//! // Check if entities are alive
//! assert!(manager.is_alive(entity1));
//!
//! // Despawn an entity
//! manager.despawn(entity1);
//! assert!(!manager.is_alive(entity1));
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
//! - `component`: Component storage and management (coming soon)
//! - `query`: Type-safe component queries (coming soon)
//! - `command`: Thread-safe command buffers (coming soon)
//! - `world`: Top-level ECS world (coming soon)

pub mod component;
pub mod entity;

// Re-export commonly used types
pub use component::Component;
pub use entity::{EntityId, EntityManager, StableId};

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
