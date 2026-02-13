//! Persistence system for PECS.
//!
//! This module provides comprehensive persistence capabilities including:
//! - Pluggable serialization formats (binary, JSON, custom)
//! - Version migration support
//! - Selective persistence (transient components)
//! - Efficient save/load operations
//!
//! # Architecture
//!
//! The persistence system is built around several key components:
//!
//! - **Plugin System**: Allows custom serialization formats via the [`PersistencePlugin`] trait
//! - **Metadata**: Tracks world version, component types, and schema information
//! - **Manager**: Coordinates save/load operations and plugin lifecycle
//! - **Migration**: Handles version upgrades via the [`Migration`] trait
//!
//! # Example
//!
//! ```rust,ignore
//! use pecs::prelude::*;
//!
//! // Create a world with entities
//! let mut world = World::new();
//! let entity = world.spawn().insert(Position { x: 1.0, y: 2.0 }).id();
//!
//! // Save the world
//! world.save("world.pecs")?;
//!
//! // Load the world
//! let loaded_world = World::load("world.pecs")?;
//! ```

pub mod binary;
pub mod error;
pub mod json;
pub mod manager;
pub mod metadata;
pub mod plugin;

pub use binary::BinaryPlugin;
pub use error::{PersistenceError, Result};
pub use json::JsonPlugin;
pub use manager::PersistenceManager;
pub use metadata::{ChangeTracker, ComponentTypeInfo, WorldMetadata};
pub use plugin::{
    ComponentData, DeltaPersistencePlugin, EntityChange, Migration, PersistencePlugin,
    SerializableComponent,
};

// Made with Bob
