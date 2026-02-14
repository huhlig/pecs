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

//! In-memory key-value entity persistence plugin.
//!
//! This module provides a simple in-memory implementation of the
//! `EntityPersistencePlugin` trait, useful for testing and as a reference
//! implementation.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::World;
use crate::entity::{EntityId, StableId};
use crate::persistence::{EntityData, EntityPersistencePlugin, PersistenceError, Result};

/// In-memory key-value store for entity persistence.
///
/// This plugin stores entities in memory using a HashMap, making it suitable
/// for testing, caching, or as a reference implementation. Data is lost when
/// the plugin is dropped.
///
/// # Thread Safety
///
/// This plugin uses `Arc<RwLock<>>` internally, making it safe to share
/// across threads.
///
/// # Examples
///
/// ```rust,ignore
/// use pecs::persistence::KeyValueEntityPlugin;
/// use pecs::World;
///
/// let plugin = KeyValueEntityPlugin::new();
/// let mut world = World::new();
///
/// // Spawn an entity
/// let entity = world.spawn().with(Position { x: 1.0, y: 2.0 }).id();
/// let stable_id = world.get_stable_id(entity).unwrap();
///
/// // Save it
/// plugin.save_entity(&world, entity)?;
///
/// // Load it back
/// let loaded_entity = plugin.load_entity(&mut world, stable_id)?;
/// ```
#[derive(Clone)]
pub struct KeyValueEntityPlugin {
    /// Internal storage for entity data
    ///
    /// This is public to allow direct access in tests. In production code,
    /// use the trait methods instead.
    #[doc(hidden)]
    pub storage: Arc<RwLock<HashMap<StableId, EntityData>>>,
}

impl KeyValueEntityPlugin {
    /// Creates a new empty key-value entity plugin.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::persistence::KeyValueEntityPlugin;
    ///
    /// let plugin = KeyValueEntityPlugin::new();
    /// ```
    pub fn new() -> Self {
        Self {
            storage: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Creates a new key-value entity plugin with pre-allocated capacity.
    ///
    /// # Arguments
    ///
    /// * `capacity` - Number of entities to pre-allocate space for
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::persistence::KeyValueEntityPlugin;
    ///
    /// let plugin = KeyValueEntityPlugin::with_capacity(1000);
    /// ```
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            storage: Arc::new(RwLock::new(HashMap::with_capacity(capacity))),
        }
    }

    /// Returns the number of entities currently stored.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let count = plugin.len();
    /// println!("Stored {} entities", count);
    /// ```
    pub fn len(&self) -> usize {
        self.storage.read().unwrap().len()
    }

    /// Returns true if no entities are stored.
    pub fn is_empty(&self) -> bool {
        self.storage.read().unwrap().is_empty()
    }

    /// Clears all stored entities.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// plugin.clear();
    /// assert!(plugin.is_empty());
    /// ```
    pub fn clear(&self) {
        self.storage.write().unwrap().clear();
    }

    /// Returns a list of all stored stable IDs.
    pub fn list_entities(&self) -> Vec<StableId> {
        self.storage.read().unwrap().keys().copied().collect()
    }
}

impl Default for KeyValueEntityPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl EntityPersistencePlugin for KeyValueEntityPlugin {
    fn save_entity(&self, world: &World, entity: EntityId) -> Result<()> {
        // Get the stable ID for this entity
        let stable_id = world
            .get_stable_id(entity)
            .ok_or(PersistenceError::EntityNotFound(entity))?;

        // TODO: Collect component data from the world
        // For now, we'll create an empty entity data
        let entity_data = EntityData::new(stable_id, Vec::new(), EntityData::current_timestamp());

        // Store in the HashMap
        self.storage.write().unwrap().insert(stable_id, entity_data);

        Ok(())
    }

    fn load_entity(&self, world: &mut World, stable_id: StableId) -> Result<EntityId> {
        // Get the entity data from storage
        let _entity_data = self
            .storage
            .read()
            .unwrap()
            .get(&stable_id)
            .cloned()
            .ok_or_else(|| {
                PersistenceError::Custom(format!("Entity with stable ID {} not found", stable_id))
            })?;

        // Check if entity already exists in world
        if let Some(entity_id) = world.get_entity_by_stable_id(stable_id) {
            // Entity exists, update it
            // TODO: Update components
            Ok(entity_id)
        } else {
            // Create new entity with the stable ID
            let entity_id = world
                .spawn_empty_with_stable_id(stable_id)
                .map_err(|e| PersistenceError::Custom(format!("Failed to spawn entity: {}", e)))?;

            // TODO: Restore components from entity_data

            Ok(entity_id)
        }
    }

    fn delete_entity(&self, stable_id: StableId) -> Result<()> {
        self.storage.write().unwrap().remove(&stable_id);
        Ok(())
    }

    fn entity_exists(&self, stable_id: StableId) -> Result<bool> {
        Ok(self.storage.read().unwrap().contains_key(&stable_id))
    }

    fn save_entities(&self, world: &World, entities: &[EntityId]) -> Result<()> {
        // Batch operation - acquire write lock once
        let mut storage = self.storage.write().unwrap();

        for &entity in entities {
            let stable_id = world
                .get_stable_id(entity)
                .ok_or(PersistenceError::EntityNotFound(entity))?;

            // TODO: Collect component data
            let entity_data =
                EntityData::new(stable_id, Vec::new(), EntityData::current_timestamp());

            storage.insert(stable_id, entity_data);
        }

        Ok(())
    }

    fn load_entities(&self, world: &mut World, stable_ids: &[StableId]) -> Result<Vec<EntityId>> {
        let mut entity_ids = Vec::with_capacity(stable_ids.len());

        for &stable_id in stable_ids {
            entity_ids.push(self.load_entity(world, stable_id)?);
        }

        Ok(entity_ids)
    }

    fn backend_name(&self) -> &str {
        "key_value_memory"
    }

    fn backend_version(&self) -> u32 {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_creation() {
        let plugin = KeyValueEntityPlugin::new();
        assert!(plugin.is_empty());
        assert_eq!(plugin.len(), 0);
    }

    #[test]
    fn plugin_with_capacity() {
        let plugin = KeyValueEntityPlugin::with_capacity(100);
        assert!(plugin.is_empty());
    }

    #[test]
    fn plugin_clear() {
        let plugin = KeyValueEntityPlugin::new();
        let stable_id = StableId::new();

        // Manually insert data
        plugin
            .storage
            .write()
            .unwrap()
            .insert(stable_id, EntityData::new(stable_id, Vec::new(), 0));

        assert_eq!(plugin.len(), 1);
        plugin.clear();
        assert!(plugin.is_empty());
    }

    #[test]
    fn entity_exists() {
        let plugin = KeyValueEntityPlugin::new();
        let stable_id = StableId::new();

        assert!(!plugin.entity_exists(stable_id).unwrap());

        plugin
            .storage
            .write()
            .unwrap()
            .insert(stable_id, EntityData::new(stable_id, Vec::new(), 0));

        assert!(plugin.entity_exists(stable_id).unwrap());
    }

    #[test]
    fn delete_entity() {
        let plugin = KeyValueEntityPlugin::new();
        let stable_id = StableId::new();

        plugin
            .storage
            .write()
            .unwrap()
            .insert(stable_id, EntityData::new(stable_id, Vec::new(), 0));

        assert_eq!(plugin.len(), 1);
        plugin.delete_entity(stable_id).unwrap();
        assert!(plugin.is_empty());
    }

    #[test]
    fn list_entities() {
        let plugin = KeyValueEntityPlugin::new();
        let id1 = StableId::new();
        let id2 = StableId::new();

        plugin
            .storage
            .write()
            .unwrap()
            .insert(id1, EntityData::new(id1, Vec::new(), 0));
        plugin
            .storage
            .write()
            .unwrap()
            .insert(id2, EntityData::new(id2, Vec::new(), 0));

        let entities = plugin.list_entities();
        assert_eq!(entities.len(), 2);
        assert!(entities.contains(&id1));
        assert!(entities.contains(&id2));
    }
}

// Made with Bob
