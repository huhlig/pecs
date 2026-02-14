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

//! Persistence manager for coordinating save/load operations.

use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

use crate::World;
use crate::entity::{EntityId, StableId};
use crate::persistence::{
    ChangeTracker, DeltaPersistencePlugin, EntityChange, EntityPersistencePlugin, Migration,
    PersistenceError, PersistencePlugin, Result,
};

/// Manages persistence operations and plugin lifecycle.
///
/// The `PersistenceManager` coordinates:
/// - Plugin registration and selection
/// - Save/load operations
/// - Version migration
/// - Change tracking for delta persistence
///
/// # Examples
///
/// ```rust,ignore
/// use pecs::persistence::PersistenceManager;
/// use pecs::World;
///
/// let mut manager = PersistenceManager::new();
///
/// // Register a custom plugin
/// manager.register_plugin("custom", Box::new(MyCustomPlugin));
///
/// // Save with default plugin
/// let world = World::new();
/// manager.save(&world, "world.pecs")?;
///
/// // Load with specific plugin
/// let loaded = manager.load_with("world.pecs", "custom")?;
/// ```
pub struct PersistenceManager {
    /// Registered persistence plugins by name
    plugins: HashMap<String, Box<dyn PersistencePlugin>>,

    /// Registered delta persistence plugins by name
    delta_plugins: HashMap<String, Box<dyn DeltaPersistencePlugin>>,

    /// Registered entity persistence plugins by name
    entity_plugins: HashMap<String, Box<dyn EntityPersistencePlugin>>,

    /// Registered migrations by version range
    migrations: Vec<Box<dyn Migration>>,

    /// Default plugin name
    default_plugin: Option<String>,

    /// Default entity plugin name
    default_entity_plugin: Option<String>,

    /// Change tracker for delta persistence
    change_tracker: ChangeTracker,
}

impl PersistenceManager {
    /// Creates a new persistence manager.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::persistence::PersistenceManager;
    ///
    /// let manager = PersistenceManager::new();
    /// ```
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
            delta_plugins: HashMap::new(),
            entity_plugins: HashMap::new(),
            migrations: Vec::new(),
            default_plugin: None,
            default_entity_plugin: None,
            change_tracker: ChangeTracker::new(),
        }
    }

    /// Registers a persistence plugin.
    ///
    /// # Arguments
    ///
    /// * `name` - Unique name for the plugin
    /// * `plugin` - The plugin implementation
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// manager.register_plugin("binary", Box::new(BinaryPlugin));
    /// ```
    pub fn register_plugin(&mut self, name: impl Into<String>, plugin: Box<dyn PersistencePlugin>) {
        let name = name.into();
        if self.default_plugin.is_none() {
            self.default_plugin = Some(name.clone());
        }
        self.plugins.insert(name, plugin);
    }

    /// Registers a delta persistence plugin.
    ///
    /// # Arguments
    ///
    /// * `name` - Unique name for the plugin
    /// * `plugin` - The delta plugin implementation
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// manager.register_delta_plugin("database", Box::new(DatabasePlugin));
    /// ```
    pub fn register_delta_plugin(
        &mut self,
        name: impl Into<String>,
        plugin: Box<dyn DeltaPersistencePlugin>,
    ) {
        self.delta_plugins.insert(name.into(), plugin);
    }

    /// Registers an entity persistence plugin.
    ///
    /// # Arguments
    ///
    /// * `name` - Unique name for the plugin
    /// * `plugin` - The entity plugin implementation
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// manager.register_entity_plugin("redis", Box::new(RedisEntityPlugin));
    /// ```
    pub fn register_entity_plugin(
        &mut self,
        name: impl Into<String>,
        plugin: Box<dyn EntityPersistencePlugin>,
    ) {
        let name = name.into();
        if self.default_entity_plugin.is_none() {
            self.default_entity_plugin = Some(name.clone());
        }
        self.entity_plugins.insert(name, plugin);
    }

    /// Registers a version migration.
    ///
    /// Migrations are automatically applied when loading older versions.
    ///
    /// # Arguments
    ///
    /// * `migration` - The migration implementation
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// manager.register_migration(Box::new(MigrationV1ToV2));
    /// ```
    pub fn register_migration(&mut self, migration: Box<dyn Migration>) {
        self.migrations.push(migration);
    }

    /// Sets the default plugin to use for save/load operations.
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the plugin to use as default
    ///
    /// # Errors
    ///
    /// Returns an error if the plugin is not registered.
    pub fn set_default_plugin(&mut self, name: impl Into<String>) -> Result<()> {
        let name = name.into();
        if !self.plugins.contains_key(&name) {
            return Err(PersistenceError::PluginNotFound(name));
        }
        self.default_plugin = Some(name);
        Ok(())
    }

    /// Sets the default entity plugin to use for entity-specific operations.
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the entity plugin to use as default
    ///
    /// # Errors
    ///
    /// Returns an error if the plugin is not registered.
    pub fn set_default_entity_plugin(&mut self, name: impl Into<String>) -> Result<()> {
        let name = name.into();
        if !self.entity_plugins.contains_key(&name) {
            return Err(PersistenceError::PluginNotFound(name));
        }
        self.default_entity_plugin = Some(name);
        Ok(())
    }

    /// Saves a world to a file using the default plugin.
    ///
    /// # Arguments
    ///
    /// * `world` - The world to save
    /// * `path` - Path to save to
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No default plugin is registered
    /// - File cannot be created
    /// - Serialization fails
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// manager.save(&world, "world.pecs")?;
    /// ```
    pub fn save(&self, world: &World, path: impl AsRef<Path>) -> Result<()> {
        let plugin_name = self
            .default_plugin
            .as_ref()
            .ok_or_else(|| PersistenceError::PluginNotFound("default".to_string()))?;
        self.save_with(world, path, plugin_name)
    }

    /// Saves a world to a file using a specific plugin.
    ///
    /// # Arguments
    ///
    /// * `world` - The world to save
    /// * `path` - Path to save to
    /// * `plugin_name` - Name of the plugin to use
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Plugin is not registered
    /// - File cannot be created
    /// - Serialization fails
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// manager.save_with(&world, "world.json", "json")?;
    /// ```
    pub fn save_with(
        &self,
        world: &World,
        path: impl AsRef<Path>,
        plugin_name: &str,
    ) -> Result<()> {
        let plugin = self
            .plugins
            .get(plugin_name)
            .ok_or_else(|| PersistenceError::PluginNotFound(plugin_name.to_string()))?;

        let mut file = File::create(path.as_ref()).map_err(PersistenceError::Io)?;

        plugin.save(world, &mut file)
    }

    /// Loads a world from a file using the default plugin.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to load from
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No default plugin is registered
    /// - File cannot be opened
    /// - Deserialization fails
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let world = manager.load("world.pecs")?;
    /// ```
    pub fn load(&self, path: impl AsRef<Path>) -> Result<World> {
        let plugin_name = self
            .default_plugin
            .as_ref()
            .ok_or_else(|| PersistenceError::PluginNotFound("default".to_string()))?;
        self.load_with(path, plugin_name)
    }

    /// Loads a world from a file using a specific plugin.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to load from
    /// * `plugin_name` - Name of the plugin to use
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Plugin is not registered
    /// - File cannot be opened
    /// - Deserialization fails
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let world = manager.load_with("world.json", "json")?;
    /// ```
    pub fn load_with(&self, path: impl AsRef<Path>, plugin_name: &str) -> Result<World> {
        let plugin = self
            .plugins
            .get(plugin_name)
            .ok_or_else(|| PersistenceError::PluginNotFound(plugin_name.to_string()))?;

        let mut file = File::open(path.as_ref()).map_err(PersistenceError::Io)?;

        let mut world = plugin.load(&mut file)?;

        // Apply migrations if needed
        self.apply_migrations(&mut world)?;

        Ok(world)
    }

    /// Saves a world to a writer using the default plugin.
    ///
    /// # Arguments
    ///
    /// * `world` - The world to save
    /// * `writer` - Writer to save to
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No default plugin is registered
    /// - Serialization fails
    pub fn save_to_writer(&self, world: &World, writer: &mut dyn std::io::Write) -> Result<()> {
        let plugin_name = self
            .default_plugin
            .as_ref()
            .ok_or_else(|| PersistenceError::PluginNotFound("default".to_string()))?;
        self.save_to_writer_with(world, writer, plugin_name)
    }

    /// Saves a world to a writer using a specific plugin.
    ///
    /// # Arguments
    ///
    /// * `world` - The world to save
    /// * `writer` - Writer to save to
    /// * `plugin_name` - Name of the plugin to use
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Plugin is not registered
    /// - Serialization fails
    pub fn save_to_writer_with(
        &self,
        world: &World,
        writer: &mut dyn std::io::Write,
        plugin_name: &str,
    ) -> Result<()> {
        let plugin = self
            .plugins
            .get(plugin_name)
            .ok_or_else(|| PersistenceError::PluginNotFound(plugin_name.to_string()))?;

        plugin.save(world, writer)
    }

    /// Loads a world from a reader using the default plugin.
    ///
    /// # Arguments
    ///
    /// * `reader` - Reader to load from
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No default plugin is registered
    /// - Deserialization fails
    pub fn load_from_reader(&self, reader: &mut dyn std::io::Read) -> Result<World> {
        let plugin_name = self
            .default_plugin
            .as_ref()
            .ok_or_else(|| PersistenceError::PluginNotFound("default".to_string()))?;
        self.load_from_reader_with(reader, plugin_name)
    }

    /// Loads a world from a reader using a specific plugin.
    ///
    /// # Arguments
    ///
    /// * `reader` - Reader to load from
    /// * `plugin_name` - Name of the plugin to use
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Plugin is not registered
    /// - Deserialization fails
    pub fn load_from_reader_with(
        &self,
        reader: &mut dyn std::io::Read,
        plugin_name: &str,
    ) -> Result<World> {
        let plugin = self
            .plugins
            .get(plugin_name)
            .ok_or_else(|| PersistenceError::PluginNotFound(plugin_name.to_string()))?;

        let mut world = plugin.load(reader)?;

        // Apply migrations if needed
        self.apply_migrations(&mut world)?;

        Ok(world)
    }

    /// Saves only the changes since the last checkpoint.
    ///
    /// This is more efficient than saving the entire world and is useful
    /// for database backends and incremental backups.
    ///
    /// # Arguments
    ///
    /// * `plugin_name` - Name of the delta plugin to use
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Plugin is not registered
    /// - Saving changes fails
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// manager.save_delta("database")?;
    /// ```
    pub fn save_delta(&mut self, plugin_name: &str) -> Result<()> {
        let plugin = self
            .delta_plugins
            .get(plugin_name)
            .ok_or_else(|| PersistenceError::PluginNotFound(plugin_name.to_string()))?;

        if !self.change_tracker.has_changes() {
            return Ok(());
        }

        // Convert tracked changes to EntityChange format
        let changes = self.collect_changes(plugin.as_ref())?;

        plugin.save_changes(&changes)?;
        self.change_tracker.checkpoint();

        Ok(())
    }

    /// Loads changes from a delta plugin since a given timestamp.
    ///
    /// # Arguments
    ///
    /// * `world` - The world to apply changes to
    /// * `plugin_name` - Name of the delta plugin to use
    /// * `since` - Unix timestamp to load changes from
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Plugin is not registered
    /// - Loading changes fails
    /// - Applying changes fails
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// manager.load_delta(&mut world, "database", 1234567890)?;
    /// ```
    pub fn load_delta(&self, world: &mut World, plugin_name: &str, since: u64) -> Result<()> {
        let plugin = self
            .delta_plugins
            .get(plugin_name)
            .ok_or_else(|| PersistenceError::PluginNotFound(plugin_name.to_string()))?;

        let changes = plugin.load_changes(since)?;
        plugin.apply_changes(world, &changes)?;

        Ok(())
    }

    /// Gets a reference to the change tracker.
    ///
    /// This allows external code to track entity changes for delta persistence.
    pub fn change_tracker(&self) -> &ChangeTracker {
        &self.change_tracker
    }

    /// Gets a mutable reference to the change tracker.
    ///
    /// This allows external code to track entity changes for delta persistence.
    pub fn change_tracker_mut(&mut self) -> &mut ChangeTracker {
        &mut self.change_tracker
    }

    /// Applies all necessary migrations to bring a world to the current version.
    ///
    /// Migrations are applied in order from the world's current version to the
    /// target version.
    fn apply_migrations(&self, world: &mut World) -> Result<()> {
        if self.migrations.is_empty() {
            return Ok(());
        }

        // Get current world version from metadata
        let mut current_version = world.metadata().version;

        // Find the target version (highest version among all migrations)
        let target_version = self
            .migrations
            .iter()
            .map(|m| m.target_version())
            .max()
            .unwrap_or(current_version);

        // If we're already at the target version, no migration needed
        if current_version >= target_version {
            return Ok(());
        }

        // Build migration chain
        while current_version < target_version {
            // Find a migration that can upgrade from current_version
            let migration = self
                .migrations
                .iter()
                .find(|m| m.source_version() == current_version)
                .ok_or_else(|| {
                    PersistenceError::MigrationFailed(format!(
                        "No migration found from version {} to {}",
                        current_version, target_version
                    ))
                })?;

            // Apply the migration
            migration.migrate(world).map_err(|e| {
                PersistenceError::MigrationFailed(format!(
                    "Migration from v{} to v{} failed: {}",
                    migration.source_version(),
                    migration.target_version(),
                    e
                ))
            })?;

            current_version = migration.target_version();
        }

        // Update world metadata version
        world.metadata_mut().version = current_version;

        Ok(())
    }

    /// Collects changes from the change tracker and converts them to EntityChange format.
    fn collect_changes(&self, plugin: &dyn DeltaPersistencePlugin) -> Result<Vec<EntityChange>> {
        let timestamp = plugin.current_timestamp();
        let mut changes = Vec::new();

        // Convert created entities
        for &entity in self.change_tracker.created() {
            changes.push(EntityChange::Created {
                entity,
                components: Vec::new(), // TODO: Collect actual component data
                timestamp,
            });
        }

        // Convert modified entities
        for &entity in self.change_tracker.modified() {
            changes.push(EntityChange::Modified {
                entity,
                added_or_modified: Vec::new(), // TODO: Collect actual component data
                removed: Vec::new(),
                timestamp,
            });
        }

        // Convert deleted entities
        for &entity in self.change_tracker.deleted() {
            changes.push(EntityChange::Deleted { entity, timestamp });
        }

        Ok(changes)
    }

    /// Lists all registered plugin names.
    pub fn list_plugins(&self) -> Vec<&str> {
        self.plugins.keys().map(|s| s.as_str()).collect()
    }

    /// Lists all registered delta plugin names.
    pub fn list_delta_plugins(&self) -> Vec<&str> {
        self.delta_plugins.keys().map(|s| s.as_str()).collect()
    }

    /// Gets the name of the default plugin, if set.
    pub fn default_plugin(&self) -> Option<&str> {
        self.default_plugin.as_deref()
    }

    /// Saves a specific entity using the default entity plugin.
    ///
    /// # Arguments
    ///
    /// * `world` - The world containing the entity
    /// * `entity` - The entity ID to save
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No default entity plugin is registered
    /// - The entity doesn't exist
    /// - Saving fails
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// manager.save_entity(&world, entity_id)?;
    /// ```
    pub fn save_entity(&self, world: &World, entity: EntityId) -> Result<()> {
        let plugin_name = self
            .default_entity_plugin
            .as_ref()
            .ok_or_else(|| PersistenceError::PluginNotFound("default entity plugin".to_string()))?;
        self.save_entity_with(world, entity, plugin_name)
    }

    /// Saves a specific entity using a named entity plugin.
    ///
    /// # Arguments
    ///
    /// * `world` - The world containing the entity
    /// * `entity` - The entity ID to save
    /// * `plugin_name` - Name of the entity plugin to use
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Plugin is not registered
    /// - The entity doesn't exist
    /// - Saving fails
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// manager.save_entity_with(&world, entity_id, "redis")?;
    /// ```
    pub fn save_entity_with(
        &self,
        world: &World,
        entity: EntityId,
        plugin_name: &str,
    ) -> Result<()> {
        let plugin = self
            .entity_plugins
            .get(plugin_name)
            .ok_or_else(|| PersistenceError::PluginNotFound(plugin_name.to_string()))?;

        plugin.save_entity(world, entity)
    }

    /// Loads a specific entity using the default entity plugin.
    ///
    /// # Arguments
    ///
    /// * `world` - The world to load the entity into
    /// * `stable_id` - The stable ID of the entity to load
    ///
    /// # Returns
    ///
    /// The `EntityId` of the loaded entity.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No default entity plugin is registered
    /// - The entity doesn't exist in storage
    /// - Loading fails
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let entity_id = manager.load_entity(&mut world, stable_id)?;
    /// ```
    pub fn load_entity(&self, world: &mut World, stable_id: StableId) -> Result<EntityId> {
        let plugin_name = self
            .default_entity_plugin
            .as_ref()
            .ok_or_else(|| PersistenceError::PluginNotFound("default entity plugin".to_string()))?;
        self.load_entity_with(world, stable_id, plugin_name)
    }

    /// Loads a specific entity using a named entity plugin.
    ///
    /// # Arguments
    ///
    /// * `world` - The world to load the entity into
    /// * `stable_id` - The stable ID of the entity to load
    /// * `plugin_name` - Name of the entity plugin to use
    ///
    /// # Returns
    ///
    /// The `EntityId` of the loaded entity.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Plugin is not registered
    /// - The entity doesn't exist in storage
    /// - Loading fails
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let entity_id = manager.load_entity_with(&mut world, stable_id, "redis")?;
    /// ```
    pub fn load_entity_with(
        &self,
        world: &mut World,
        stable_id: StableId,
        plugin_name: &str,
    ) -> Result<EntityId> {
        let plugin = self
            .entity_plugins
            .get(plugin_name)
            .ok_or_else(|| PersistenceError::PluginNotFound(plugin_name.to_string()))?;

        plugin.load_entity(world, stable_id)
    }

    /// Deletes a specific entity from storage using the default entity plugin.
    ///
    /// # Arguments
    ///
    /// * `stable_id` - The stable ID of the entity to delete
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No default entity plugin is registered
    /// - Deletion fails
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// manager.delete_entity(stable_id)?;
    /// ```
    pub fn delete_entity(&self, stable_id: StableId) -> Result<()> {
        let plugin_name = self
            .default_entity_plugin
            .as_ref()
            .ok_or_else(|| PersistenceError::PluginNotFound("default entity plugin".to_string()))?;
        self.delete_entity_with(stable_id, plugin_name)
    }

    /// Deletes a specific entity from storage using a named entity plugin.
    ///
    /// # Arguments
    ///
    /// * `stable_id` - The stable ID of the entity to delete
    /// * `plugin_name` - Name of the entity plugin to use
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Plugin is not registered
    /// - Deletion fails
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// manager.delete_entity_with(stable_id, "redis")?;
    /// ```
    pub fn delete_entity_with(&self, stable_id: StableId, plugin_name: &str) -> Result<()> {
        let plugin = self
            .entity_plugins
            .get(plugin_name)
            .ok_or_else(|| PersistenceError::PluginNotFound(plugin_name.to_string()))?;

        plugin.delete_entity(stable_id)
    }

    /// Checks if an entity exists in storage using the default entity plugin.
    ///
    /// # Arguments
    ///
    /// * `stable_id` - The stable ID of the entity to check
    ///
    /// # Returns
    ///
    /// `true` if the entity exists in storage, `false` otherwise.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No default entity plugin is registered
    /// - Check operation fails
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// if manager.entity_exists(stable_id)? {
    ///     println!("Entity exists");
    /// }
    /// ```
    pub fn entity_exists(&self, stable_id: StableId) -> Result<bool> {
        let plugin_name = self
            .default_entity_plugin
            .as_ref()
            .ok_or_else(|| PersistenceError::PluginNotFound("default entity plugin".to_string()))?;
        self.entity_exists_with(stable_id, plugin_name)
    }

    /// Checks if an entity exists in storage using a named entity plugin.
    ///
    /// # Arguments
    ///
    /// * `stable_id` - The stable ID of the entity to check
    /// * `plugin_name` - Name of the entity plugin to use
    ///
    /// # Returns
    ///
    /// `true` if the entity exists in storage, `false` otherwise.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Plugin is not registered
    /// - Check operation fails
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// if manager.entity_exists_with(stable_id, "redis")? {
    ///     println!("Entity exists in Redis");
    /// }
    /// ```
    pub fn entity_exists_with(&self, stable_id: StableId, plugin_name: &str) -> Result<bool> {
        let plugin = self
            .entity_plugins
            .get(plugin_name)
            .ok_or_else(|| PersistenceError::PluginNotFound(plugin_name.to_string()))?;

        plugin.entity_exists(stable_id)
    }

    /// Lists all registered entity plugin names.
    pub fn list_entity_plugins(&self) -> Vec<&str> {
        self.entity_plugins.keys().map(|s| s.as_str()).collect()
    }

    /// Gets the name of the default entity plugin, if set.
    pub fn default_entity_plugin(&self) -> Option<&str> {
        self.default_entity_plugin.as_deref()
    }
}

impl Default for PersistenceManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manager_creation() {
        let manager = PersistenceManager::new();
        assert!(manager.default_plugin().is_none());
        assert!(manager.list_plugins().is_empty());
    }

    #[test]
    fn change_tracker_access() {
        let mut manager = PersistenceManager::new();
        assert!(!manager.change_tracker().has_changes());

        // Track a change
        use crate::entity::EntityId;
        let entity = EntityId::new(0, 1);
        manager.change_tracker_mut().track_created(entity);

        assert!(manager.change_tracker().has_changes());
        assert_eq!(manager.change_tracker().created().len(), 1);
    }
}
