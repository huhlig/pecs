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

//! Plugin trait for custom persistence formats.

use crate::World;
use crate::entity::EntityId;
use crate::persistence::Result;
use std::io::{Read, Write};

/// Trait for implementing custom persistence formats.
///
/// This trait allows users to create custom serialization formats for world persistence.
/// The library provides built-in implementations for binary and JSON formats, but users
/// can implement this trait to support additional formats.
///
/// # Thread Safety
///
/// Implementations must be `Send + Sync` to allow use across threads.
///
/// # Example
///
/// ```rust,ignore
/// use pecs::persistence::{PersistencePlugin, Result};
/// use pecs::World;
/// use std::io::{Read, Write};
///
/// struct MyCustomFormat;
///
/// impl PersistencePlugin for MyCustomFormat {
///     fn save(&self, world: &World, writer: &mut dyn Write) -> Result<()> {
///         // Custom serialization logic
///         Ok(())
///     }
///
///     fn load(&self, reader: &mut dyn Read) -> Result<World> {
///         // Custom deserialization logic
///         Ok(World::new())
///     }
///
///     fn format_name(&self) -> &str {
///         "my_custom_format"
///     }
///
///     fn format_version(&self) -> u32 {
///         1
///     }
/// }
/// ```
pub trait PersistencePlugin: Send + Sync {
    /// Serialize a world to the given writer.
    ///
    /// # Arguments
    ///
    /// * `world` - The world to serialize
    /// * `writer` - The writer to serialize to
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails.
    fn save(&self, world: &World, writer: &mut dyn Write) -> Result<()>;

    /// Deserialize a world from the given reader.
    ///
    /// # Arguments
    ///
    /// * `reader` - The reader to deserialize from
    ///
    /// # Errors
    ///
    /// Returns an error if deserialization fails.
    fn load(&self, reader: &mut dyn Read) -> Result<World>;

    /// Get the name of this format.
    ///
    /// This is used for plugin registration and identification.
    fn format_name(&self) -> &str;

    /// Get the version of this format.
    ///
    /// This is used for version checking and migration.
    fn format_version(&self) -> u32;

    /// Check if this plugin can handle the given format version.
    ///
    /// By default, this checks for exact version match. Override this method
    /// to support backward compatibility with older versions.
    ///
    /// # Arguments
    ///
    /// * `version` - The version to check
    fn can_load_version(&self, version: u32) -> bool {
        version == self.format_version()
    }
}

/// Trait for implementing delta/incremental persistence.
///
/// This trait extends the basic persistence plugin to support incremental updates,
/// which is essential for database backends and real-time synchronization.
///
/// # Use Cases
///
/// - Database backends (SQL, NoSQL)
/// - Network synchronization
/// - Incremental backups
/// - Change streaming
///
/// # Example
///
/// ```rust,ignore
/// use pecs::persistence::{DeltaPersistencePlugin, EntityChange, Result};
/// use pecs::World;
///
/// struct DatabaseBackend {
///     connection: DatabaseConnection,
/// }
///
/// impl DeltaPersistencePlugin for DatabaseBackend {
///     fn save_changes(&self, changes: &[EntityChange]) -> Result<()> {
///         // Save only the changes to database
///         for change in changes {
///             match change {
///                 EntityChange::Created(entity) => { /* INSERT */ }
///                 EntityChange::Modified(entity) => { /* UPDATE */ }
///                 EntityChange::Deleted(entity) => { /* DELETE */ }
///             }
///         }
///         Ok(())
///     }
///
///     fn load_changes(&self, since: u64) -> Result<Vec<EntityChange>> {
///         // Load changes from database since timestamp
///         Ok(vec![])
///     }
/// }
/// ```
pub trait DeltaPersistencePlugin: Send + Sync {
    /// Save only the changes to the persistence backend.
    ///
    /// This is more efficient than saving the entire world state and is
    /// essential for database backends and real-time synchronization.
    ///
    /// # Arguments
    ///
    /// * `changes` - The list of entity changes to save
    ///
    /// # Errors
    ///
    /// Returns an error if saving changes fails.
    fn save_changes(&self, changes: &[EntityChange]) -> Result<()>;

    /// Load changes from the persistence backend since a given timestamp.
    ///
    /// This allows incremental loading and synchronization with external
    /// data sources.
    ///
    /// # Arguments
    ///
    /// * `since` - Unix timestamp (seconds) to load changes from
    ///
    /// # Errors
    ///
    /// Returns an error if loading changes fails.
    fn load_changes(&self, since: u64) -> Result<Vec<EntityChange>>;

    /// Apply changes to a world.
    ///
    /// Default implementation applies changes in order. Override for
    /// custom conflict resolution or merging strategies.
    ///
    /// # Arguments
    ///
    /// * `world` - The world to apply changes to
    /// * `changes` - The changes to apply
    ///
    /// # Errors
    ///
    /// Returns an error if applying changes fails.
    fn apply_changes(&self, world: &mut World, changes: &[EntityChange]) -> Result<()> {
        for change in changes {
            change.apply(world)?;
        }
        Ok(())
    }

    /// Get the current timestamp for this backend.
    ///
    /// This is used to track when changes were made. Default implementation
    /// uses system time.
    fn current_timestamp(&self) -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }
}

/// Represents a change to an entity.
///
/// This is used by the delta persistence system to track and apply
/// incremental changes.
#[derive(Debug, Clone)]
pub enum EntityChange {
    /// Entity was created with components.
    Created {
        /// The entity ID.
        entity: EntityId,
        /// Serialized component data.
        components: Vec<ComponentData>,
        /// Timestamp when created.
        timestamp: u64,
    },

    /// Entity was modified (components added/removed/changed).
    Modified {
        /// The entity ID.
        entity: EntityId,
        /// Components that were added or modified.
        added_or_modified: Vec<ComponentData>,
        /// Component type IDs that were removed.
        removed: Vec<std::any::TypeId>,
        /// Timestamp when modified.
        timestamp: u64,
    },

    /// Entity was deleted.
    Deleted {
        /// The entity ID.
        entity: EntityId,
        /// Timestamp when deleted.
        timestamp: u64,
    },
}

impl EntityChange {
    /// Apply this change to a world.
    ///
    /// # Arguments
    ///
    /// * `world` - The world to apply the change to
    ///
    /// # Errors
    ///
    /// Returns an error if the change cannot be applied.
    pub fn apply(&self, world: &mut World) -> Result<()> {
        match self {
            EntityChange::Created {
                entity, components, ..
            } => {
                // Create entity with components
                // This will be implemented when we integrate with World
                let _ = (entity, components);
                Ok(())
            }
            EntityChange::Modified {
                entity,
                added_or_modified,
                removed,
                ..
            } => {
                // Modify entity components
                let _ = (entity, added_or_modified, removed);
                Ok(())
            }
            EntityChange::Deleted { entity, .. } => {
                // Delete entity
                world.despawn(*entity);
                Ok(())
            }
        }
    }

    /// Get the timestamp of this change.
    pub fn timestamp(&self) -> u64 {
        match self {
            EntityChange::Created { timestamp, .. }
            | EntityChange::Modified { timestamp, .. }
            | EntityChange::Deleted { timestamp, .. } => *timestamp,
        }
    }

    /// Get the entity ID affected by this change.
    pub fn entity(&self) -> EntityId {
        match self {
            EntityChange::Created { entity, .. }
            | EntityChange::Modified { entity, .. }
            | EntityChange::Deleted { entity, .. } => *entity,
        }
    }
}

/// Serialized component data.
///
/// This represents a component in a format-agnostic way for delta persistence.
#[derive(Debug, Clone)]
pub struct ComponentData {
    /// Type ID of the component.
    pub type_id: std::any::TypeId,
    /// Type name for debugging and schema tracking.
    pub type_name: String,
    /// Serialized component bytes.
    pub data: Vec<u8>,
}

/// Trait for components that can be persisted.
///
/// This trait is automatically implemented for all types that implement
/// the `Component` trait. Users can customize persistence behavior by
/// implementing this trait manually.
///
/// # Transient Components
///
/// Components can be marked as transient by returning `true` from
/// [`is_transient`](Self::is_transient). Transient components are not
/// saved during persistence operations.
///
/// # Example
///
/// ```rust,ignore
/// use pecs::component::Component;
/// use pecs::persistence::SerializableComponent;
///
/// #[derive(Component)]
/// struct CachedData {
///     value: i32,
/// }
///
/// impl SerializableComponent for CachedData {
///     fn is_transient(&self) -> bool {
///         true // Don't persist cached data
///     }
/// }
/// ```
pub trait SerializableComponent {
    /// Serialize this component to the given writer.
    ///
    /// # Arguments
    ///
    /// * `writer` - The writer to serialize to
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails.
    fn serialize(&self, writer: &mut dyn Write) -> Result<()>;

    /// Deserialize this component from the given reader.
    ///
    /// # Arguments
    ///
    /// * `reader` - The reader to deserialize from
    ///
    /// # Errors
    ///
    /// Returns an error if deserialization fails.
    fn deserialize(reader: &mut dyn Read) -> Result<Self>
    where
        Self: Sized;

    /// Check if this component should be persisted.
    ///
    /// Transient components are not saved during persistence operations.
    /// By default, all components are persistent.
    fn is_transient(&self) -> bool {
        false
    }
}

/// Trait for version migrations.
///
/// Migrations allow upgrading saved data from older versions to newer versions.
/// Each migration handles the transition from one version to the next.
///
/// # Example
///
/// ```rust,ignore
/// use pecs::persistence::{Migration, Result};
/// use pecs::World;
///
/// struct PositionMigrationV1ToV2;
///
/// impl Migration for PositionMigrationV1ToV2 {
///     fn from_version(&self) -> u32 {
///         1
///     }
///
///     fn to_version(&self) -> u32 {
///         2
///     }
///
///     fn migrate(&self, world: &mut World) -> Result<()> {
///         // Convert 2D positions to 3D by adding z=0
///         Ok(())
///     }
/// }
/// ```
pub trait Migration: Send + Sync {
    /// Get the version this migration upgrades from.
    fn source_version(&self) -> u32;

    /// Get the version this migration upgrades to.
    fn target_version(&self) -> u32;

    /// Perform the migration on the given world.
    ///
    /// # Arguments
    ///
    /// * `world` - The world to migrate
    ///
    /// # Errors
    ///
    /// Returns an error if migration fails.
    fn migrate(&self, world: &mut World) -> Result<()>;
}
