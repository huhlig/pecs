//
// Copyright 2026 Hans W. Uhlig. All Rights Reserved.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//

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
pub mod entity_kv;
pub mod error;
pub mod json;
pub mod manager;
pub mod metadata;
pub mod plugin;

pub use binary::BinaryPlugin;
pub use entity_kv::KeyValueEntityPlugin;
pub use error::{PersistenceError, Result};
pub use json::JsonPlugin;
pub use manager::PersistenceManager;
pub use metadata::{ChangeTracker, ComponentTypeInfo, WorldMetadata};
pub use plugin::{
    ComponentData, DeltaPersistencePlugin, EntityChange, EntityData, EntityPersistencePlugin,
    Migration, PersistencePlugin, SerializableComponent,
};
