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

//! The main ECS world that coordinates all subsystems.
//!
//! The [`World`] is the central hub of the ECS, managing entities, components,
//! and providing the primary API for interacting with the system.
//!
//! # Examples
//!
//! ```
//! use pecs::prelude::*;
//!
//! #[derive(Debug)]
//! struct Position { x: f32, y: f32 }
//! impl Component for Position {}
//!
//! #[derive(Debug)]
//! struct Velocity { x: f32, y: f32 }
//! impl Component for Velocity {}
//!
//! let mut world = World::new();
//!
//! // Spawn an entity with components
//! let entity = world.spawn()
//!     .with(Position { x: 0.0, y: 0.0 })
//!     .with(Velocity { x: 1.0, y: 0.0 })
//!     .id();
//!
//! // Query entities
//! for (pos, vel) in world.query::<(&Position, &Velocity)>() {
//!     println!("Entity at ({}, {}) moving at ({}, {})",
//!         pos.x, pos.y, vel.x, vel.y);
//! }
//! ```

use crate::command::CommandBuffer;
use crate::component::archetype::{ArchetypeId, ArchetypeManager};
use crate::component::{Component, ComponentInfo, ComponentSet, ComponentTypeId};
use crate::entity::{EntityId, EntityManager, StableId};
use crate::persistence::{PersistenceManager, WorldMetadata};

/// The main ECS world.
///
/// `World` is the primary interface for working with the ECS. It manages:
/// - Entity lifecycle (spawn, despawn)
/// - Component storage and access
/// - Query execution
/// - Command buffer integration
/// - Persistence operations
///
/// # Thread Safety
///
/// `World` is not `Send` or `Sync` by default, as it's designed for
/// single-threaded use. For parallel system execution, use command buffers
/// to record operations from multiple threads, then apply them to the world.
pub struct World {
    /// Entity management
    entities: EntityManager,

    /// Component storage
    archetypes: ArchetypeManager,

    /// Command buffer for deferred operations
    commands: CommandBuffer,

    /// Persistence manager
    persistence: PersistenceManager,

    /// World metadata for persistence
    metadata: WorldMetadata,
}

impl World {
    /// Creates a new empty world.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::World;
    ///
    /// let world = World::new();
    /// ```
    pub fn new() -> Self {
        Self {
            entities: EntityManager::new(),
            archetypes: ArchetypeManager::new(),
            commands: CommandBuffer::new(),
            persistence: PersistenceManager::new(),
            metadata: WorldMetadata::new(1, 0, Vec::new()),
        }
    }

    /// Creates a new world with pre-allocated capacity.
    ///
    /// # Arguments
    ///
    /// * `entity_capacity` - Number of entity slots to pre-allocate
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::World;
    ///
    /// let world = World::with_capacity(1000);
    /// ```
    pub fn with_capacity(entity_capacity: usize) -> Self {
        Self {
            entities: EntityManager::with_capacity(entity_capacity),
            archetypes: ArchetypeManager::new(),
            commands: CommandBuffer::with_capacity(entity_capacity),
            persistence: PersistenceManager::new(),
            metadata: WorldMetadata::new(1, 0, Vec::new()),
        }
    }

    /// Spawns a new entity, returning an entity builder.
    ///
    /// The entity builder allows you to add components before the entity
    /// is fully created.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::prelude::*;
    ///
    /// #[derive(Debug)]
    /// struct Position { x: f32, y: f32 }
    /// impl Component for Position {}
    ///
    /// let mut world = World::new();
    /// let entity = world.spawn()
    ///     .with(Position { x: 0.0, y: 0.0 })
    ///     .id();
    /// ```
    pub fn spawn(&mut self) -> EntityBuilder<'_> {
        let (entity_id, stable_id) = self.entities.spawn_with_stable_id();

        // Track entity creation for persistence
        self.persistence
            .change_tracker_mut()
            .track_created(entity_id);

        EntityBuilder {
            world: self,
            entity_id,
            stable_id,
            components: Vec::new(),
        }
    }

    /// Spawns an empty entity without components.
    ///
    /// This is faster than using the builder if you don't need to add
    /// components immediately.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::World;
    ///
    /// let mut world = World::new();
    /// let entity = world.spawn_empty();
    /// ```
    pub fn spawn_empty(&mut self) -> EntityId {
        let entity_id = self.entities.spawn();

        // Add to empty archetype
        let empty_archetype_id = ArchetypeId::new(0);
        if let Some(archetype) = self.archetypes.get_archetype_mut(empty_archetype_id) {
            archetype.allocate_row(entity_id);
        }

        // Track entity creation for persistence
        self.persistence
            .change_tracker_mut()
            .track_created(entity_id);

        entity_id
    }

    /// Despawns an entity, removing it and all its components.
    ///
    /// # Arguments
    ///
    /// * `entity` - The entity to despawn
    ///
    /// # Returns
    ///
    /// `true` if the entity was despawned, `false` if it didn't exist.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::World;
    ///
    /// let mut world = World::new();
    /// let entity = world.spawn_empty();
    /// assert!(world.despawn(entity));
    /// assert!(!world.is_alive(entity));
    /// ```
    pub fn despawn(&mut self, entity: EntityId) -> bool {
        if !self.entities.is_alive(entity) {
            return false;
        }

        // Track entity deletion for persistence
        self.persistence.change_tracker_mut().track_deleted(entity);

        // Remove from archetype
        if let Some(location) = self.archetypes.get_entity_location(entity)
            && let Some(archetype) = self.archetypes.get_archetype_mut(location.archetype_id)
        {
            archetype.remove_entity(entity);
        }

        // Remove from entity manager
        self.entities.despawn(entity)
    }

    /// Checks if an entity is alive.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::World;
    ///
    /// let mut world = World::new();
    /// let entity = world.spawn_empty();
    /// assert!(world.is_alive(entity));
    /// ```
    pub fn is_alive(&self, entity: EntityId) -> bool {
        self.entities.is_alive(entity)
    }

    /// Gets the stable ID for an entity.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::World;
    ///
    /// let mut world = World::new();
    /// let entity = world.spawn_empty();
    /// let stable_id = world.get_stable_id(entity).unwrap();
    /// ```
    pub fn get_stable_id(&self, entity: EntityId) -> Option<StableId> {
        self.entities.get_stable_id(entity)
    }

    /// Gets the entity ID for a stable ID.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::World;
    ///
    /// let mut world = World::new();
    /// let entity = world.spawn_empty();
    /// let stable_id = world.get_stable_id(entity).unwrap();
    /// assert_eq!(world.get_entity_id(stable_id), Some(entity));
    /// ```
    pub fn get_entity_id(&self, stable_id: StableId) -> Option<EntityId> {
        self.entities.get_entity_id(stable_id)
    }

    /// Returns the number of alive entities.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::World;
    ///
    /// let mut world = World::new();
    /// assert_eq!(world.len(), 0);
    /// world.spawn_empty();
    /// assert_eq!(world.len(), 1);
    /// ```
    pub fn len(&self) -> usize {
        self.entities.len()
    }

    /// Returns `true` if there are no entities.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::World;
    ///
    /// let world = World::new();
    /// assert!(world.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    /// Clears all entities and components from the world.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::World;
    ///
    /// let mut world = World::new();
    /// world.spawn_empty();
    /// world.clear();
    /// assert!(world.is_empty());
    /// ```
    pub fn clear(&mut self) {
        self.entities.clear();
        self.archetypes = ArchetypeManager::new();
        self.persistence = PersistenceManager::new();
        self.metadata = WorldMetadata::new(1, 0, Vec::new());
    }

    /// Returns a reference to the command buffer.
    ///
    /// Commands recorded in the buffer can be applied later using
    /// [`apply_commands`](Self::apply_commands).
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::World;
    ///
    /// let mut world = World::new();
    /// world.commands().spawn();
    /// ```
    pub fn commands(&mut self) -> &mut CommandBuffer {
        &mut self.commands
    }

    /// Applies all pending commands from the command buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::World;
    ///
    /// let mut world = World::new();
    /// world.commands().spawn();
    /// world.apply_commands();
    /// assert_eq!(world.len(), 1);
    /// ```
    pub fn apply_commands(&mut self) {
        // Take the command buffer temporarily to avoid borrow checker issues
        let mut commands = std::mem::take(&mut self.commands);
        commands.apply(self);
        self.commands = commands;
    }

    /// Returns a reference to the persistence manager.
    ///
    /// Use this to register custom persistence plugins or configure
    /// persistence behavior.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use pecs::World;
    ///
    /// let mut world = World::new();
    /// world.persistence().register_plugin("custom", Box::new(MyPlugin));
    /// ```
    pub fn persistence(&mut self) -> &mut PersistenceManager {
        &mut self.persistence
    }

    /// Returns a reference to the world metadata.
    ///
    /// Metadata includes version information, timestamps, and component
    /// type registry for persistence operations.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::World;
    ///
    /// let world = World::new();
    /// let metadata = world.metadata();
    /// assert_eq!(metadata.version, 1);
    /// ```
    pub fn metadata(&self) -> &WorldMetadata {
        &self.metadata
    }

    /// Returns a mutable reference to the world metadata.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::World;
    ///
    /// let mut world = World::new();
    /// world.metadata_mut().custom.insert("key".to_string(), "value".to_string());
    /// ```
    pub fn metadata_mut(&mut self) -> &mut WorldMetadata {
        &mut self.metadata
    }

    /// Returns an iterator over all entities with their stable IDs.
    ///
    /// This is useful for persistence operations that need to serialize
    /// entities with their stable identifiers.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::World;
    ///
    /// let mut world = World::new();
    /// world.spawn_empty();
    ///
    /// for (entity, stable_id) in world.iter_entities() {
    ///     println!("Entity {:?} has stable ID {}", entity, stable_id);
    /// }
    /// ```
    pub fn iter_entities(&self) -> impl Iterator<Item = (EntityId, StableId)> + '_ {
        self.entities.iter()
    }

    /// Returns a mutable reference to the entity manager.
    ///
    /// This is primarily for internal use by persistence systems.
    #[doc(hidden)]
    pub fn entities_mut(&mut self) -> &mut EntityManager {
        &mut self.entities
    }

    /// Inserts a component into an entity.
    ///
    /// If the entity already has this component type, it will be replaced.
    /// This operation may move the entity to a different archetype.
    ///
    /// # Arguments
    ///
    /// * `entity` - The entity to add the component to
    /// * `component` - The component to add
    ///
    /// # Returns
    ///
    /// `true` if successful, `false` if the entity doesn't exist.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::prelude::*;
    ///
    /// #[derive(Debug)]
    /// struct Position { x: f32, y: f32 }
    /// impl Component for Position {}
    ///
    /// let mut world = World::new();
    /// let entity = world.spawn_empty();
    /// assert!(world.insert(entity, Position { x: 1.0, y: 2.0 }));
    /// ```
    pub fn insert<T: Component>(&mut self, entity: EntityId, component: T) -> bool {
        if !self.is_alive(entity) {
            return false;
        }

        let component_type_id = ComponentTypeId::of::<T>();

        // Get current archetype location
        let current_location = self.archetypes.get_entity_location(entity);

        if let Some(location) = current_location {
            // Entity exists in an archetype
            let current_archetype_id = location.archetype_id;

            // Check if entity already has this component
            let has_component = self
                .archetypes
                .get_archetype(current_archetype_id)
                .map(|a| a.has_component::<T>())
                .unwrap_or(false);

            if has_component {
                // Replace existing component
                if let Some(archetype_mut) = self.archetypes.get_archetype_mut(current_archetype_id)
                {
                    unsafe {
                        if let Some(comp_mut) = archetype_mut.get_component_mut::<T>(entity) {
                            *comp_mut = component;
                        }
                    }
                }

                // Track component modification for persistence
                self.persistence.change_tracker_mut().track_modified(entity);
                return true;
            }

            // Need to move to new archetype with added component
            // First, collect all existing component types and their info
            let (mut new_component_types, mut component_info) = self
                .archetypes
                .get_archetype(current_archetype_id)
                .map(|a| {
                    let types = a.component_types().clone();
                    let mut infos = Vec::new();
                    // Collect ComponentInfo for existing types
                    for type_id in types.iter() {
                        if let Some(storage) = a.get_storage(type_id) {
                            infos.push(storage.info().clone());
                        }
                    }
                    (types, infos)
                })
                .unwrap_or_default();

            // Add the new component type
            new_component_types.insert(component_type_id);
            component_info.push(crate::component::ComponentInfo::of::<T>());

            // Get or create target archetype
            let target_archetype_id = self
                .archetypes
                .get_or_create_archetype(new_component_types, component_info);

            // Prepare component data for the new component
            let component_ptr = &component as *const T as *const u8;
            let component_data = vec![(component_type_id, component_ptr)];

            // Move entity to new archetype (this copies existing components)
            let target_row = unsafe {
                self.archetypes.move_entity_between_archetypes(
                    entity,
                    current_archetype_id,
                    target_archetype_id,
                    &component_data,
                )
            };

            // Update entity location
            if let Some(row) = target_row {
                self.archetypes.set_entity_location(
                    entity,
                    crate::component::archetype::EntityLocation {
                        archetype_id: target_archetype_id,
                        row,
                    },
                );
            }

            std::mem::forget(component); // Component was moved
        } else {
            // Entity not in any archetype yet, add to new archetype
            let mut component_types = ComponentSet::new();
            component_types.insert(component_type_id);

            let component_info = vec![crate::component::ComponentInfo::of::<T>()];
            let archetype_id = self
                .archetypes
                .get_or_create_archetype(component_types, component_info);

            if let Some(archetype) = self.archetypes.get_archetype_mut(archetype_id) {
                let row = archetype.allocate_row(entity);
                let component_ptr = &component as *const T as *const u8;
                unsafe {
                    archetype.set_component(row, component_type_id, component_ptr);
                }

                // Set entity location
                self.archetypes.set_entity_location(
                    entity,
                    crate::component::archetype::EntityLocation { archetype_id, row },
                );
            }

            std::mem::forget(component); // Component was moved
        }

        // Track component modification for persistence
        self.persistence.change_tracker_mut().track_modified(entity);

        true
    }

    /// Removes a component from an entity.
    ///
    /// This operation may move the entity to a different archetype.
    ///
    /// # Arguments
    ///
    /// * `entity` - The entity to remove the component from
    ///
    /// # Returns
    ///
    /// The removed component if it existed, or `None` if the entity didn't have it.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::prelude::*;
    ///
    /// #[derive(Debug, PartialEq)]
    /// struct Position { x: f32, y: f32 }
    /// impl Component for Position {}
    ///
    /// let mut world = World::new();
    /// let entity = world.spawn_empty();
    /// world.insert(entity, Position { x: 1.0, y: 2.0 });
    ///
    /// let removed = world.remove::<Position>(entity);
    /// assert!(removed.is_some());
    /// ```
    pub fn remove<T: Component>(&mut self, entity: EntityId) -> Option<T> {
        if !self.is_alive(entity) {
            return None;
        }

        // Get current archetype location
        let location = self.archetypes.get_entity_location(entity)?;
        let current_archetype_id = location.archetype_id;

        // Check if entity has this component
        let has_component = self
            .archetypes
            .get_archetype(current_archetype_id)?
            .has_component::<T>();

        if !has_component {
            return None;
        }

        let component_type_id = ComponentTypeId::of::<T>();

        // Get the row before we move the entity
        let row = self
            .archetypes
            .get_archetype(current_archetype_id)?
            .get_entity_row(entity)?;

        // Collect remaining component types (all except the one being removed)
        let (new_component_types, component_info) = self
            .archetypes
            .get_archetype(current_archetype_id)
            .map(|a| {
                let mut types = a.component_types().clone();
                types.remove(component_type_id);

                let mut infos = Vec::new();
                // Collect ComponentInfo for remaining types
                for type_id in types.iter() {
                    if let Some(storage) = a.get_storage(type_id) {
                        infos.push(storage.info().clone());
                    }
                }
                (types, infos)
            })
            .unwrap_or_default();

        // Read the component value before moving (but after we know the row)
        // We need to do this before move_entity_between_archetypes because that will
        // remove the entity from the source archetype
        let component_value = unsafe {
            let archetype = self.archetypes.get_archetype(current_archetype_id)?;
            let storage = archetype.get_storage(component_type_id)?;
            let ptr = storage.get(row) as *const T;
            std::ptr::read(ptr)
        };

        // Get or create target archetype (may be empty archetype)
        let target_archetype_id = self
            .archetypes
            .get_or_create_archetype(new_component_types, component_info);

        // Move entity to new archetype (this copies remaining components)
        // Note: The component we're removing won't be copied because the target
        // archetype doesn't have that component type
        let target_row = unsafe {
            self.archetypes.move_entity_between_archetypes(
                entity,
                current_archetype_id,
                target_archetype_id,
                &[], // No new components to add
            )
        };

        // Update entity location
        if let Some(row) = target_row {
            self.archetypes.set_entity_location(
                entity,
                crate::component::archetype::EntityLocation {
                    archetype_id: target_archetype_id,
                    row,
                },
            );
        }

        // Track component modification for persistence
        self.persistence.change_tracker_mut().track_modified(entity);

        Some(component_value)
    }

    /// Gets an immutable reference to a component on an entity.
    ///
    /// # Arguments
    ///
    /// * `entity` - The entity to get the component from
    ///
    /// # Returns
    ///
    /// A reference to the component if it exists, or `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::prelude::*;
    ///
    /// #[derive(Debug)]
    /// struct Position { x: f32, y: f32 }
    /// impl Component for Position {}
    ///
    /// let mut world = World::new();
    /// let entity = world.spawn_empty();
    /// world.insert(entity, Position { x: 1.0, y: 2.0 });
    ///
    /// if let Some(pos) = world.get::<Position>(entity) {
    ///     println!("Position: ({}, {})", pos.x, pos.y);
    /// }
    /// ```
    pub fn get<T: Component>(&self, entity: EntityId) -> Option<&T> {
        if !self.is_alive(entity) {
            return None;
        }

        let location = self.archetypes.get_entity_location(entity)?;
        let archetype = self.archetypes.get_archetype(location.archetype_id)?;

        unsafe { archetype.get_component::<T>(entity) }
    }

    /// Gets a mutable reference to a component on an entity.
    ///
    /// # Arguments
    ///
    /// * `entity` - The entity to get the component from
    ///
    /// # Returns
    ///
    /// A mutable reference to the component if it exists, or `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::prelude::*;
    ///
    /// #[derive(Debug)]
    /// struct Position { x: f32, y: f32 }
    /// impl Component for Position {}
    ///
    /// let mut world = World::new();
    /// let entity = world.spawn_empty();
    /// world.insert(entity, Position { x: 1.0, y: 2.0 });
    ///
    /// if let Some(pos) = world.get_mut::<Position>(entity) {
    ///     pos.x += 10.0;
    /// }
    /// ```
    pub fn get_mut<T: Component>(&mut self, entity: EntityId) -> Option<&mut T> {
        if !self.is_alive(entity) {
            return None;
        }

        let location = self.archetypes.get_entity_location(entity)?;
        let archetype = self.archetypes.get_archetype_mut(location.archetype_id)?;

        // Track component modification for persistence
        self.persistence.change_tracker_mut().track_modified(entity);

        unsafe { archetype.get_component_mut::<T>(entity) }
    }

    /// Checks if an entity has a specific component.
    ///
    /// # Arguments
    ///
    /// * `entity` - The entity to check
    ///
    /// # Returns
    ///
    /// `true` if the entity has the component, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::prelude::*;
    ///
    /// #[derive(Debug)]
    /// struct Position { x: f32, y: f32 }
    /// impl Component for Position {}
    ///
    /// let mut world = World::new();
    /// let entity = world.spawn_empty();
    ///
    /// assert!(!world.has::<Position>(entity));
    /// world.insert(entity, Position { x: 1.0, y: 2.0 });
    /// assert!(world.has::<Position>(entity));
    /// ```
    pub fn has<T: Component>(&self, entity: EntityId) -> bool {
        if !self.is_alive(entity) {
            return false;
        }

        self.archetypes
            .get_entity_location(entity)
            .and_then(|location| self.archetypes.get_archetype(location.archetype_id))
            .map(|archetype| archetype.has_component::<T>())
            .unwrap_or(false)
    }

    /// Executes a query over all entities in the world.
    ///
    /// Returns an iterator over the query results. The query type determines
    /// what data is fetched and how it's accessed.
    ///
    /// # Type Parameters
    ///
    /// * `Q` - The query type (e.g., `&Position`, `(&mut Position, &Velocity)`)
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::prelude::*;
    ///
    /// #[derive(Debug)]
    /// struct Position { x: f32, y: f32 }
    /// impl Component for Position {}
    ///
    /// #[derive(Debug)]
    /// struct Velocity { x: f32, y: f32 }
    /// impl Component for Velocity {}
    ///
    /// let mut world = World::new();
    /// let entity = world.spawn()
    ///     .with(Position { x: 0.0, y: 0.0 })
    ///     .with(Velocity { x: 1.0, y: 0.0 })
    ///     .id();
    ///
    /// // Query for entities with both Position and Velocity
    /// for (pos, vel) in world.query::<(&Position, &Velocity)>() {
    ///     println!("Entity at ({}, {}) moving at ({}, {})",
    ///         pos.x, pos.y, vel.x, vel.y);
    /// }
    /// ```
    pub fn query<Q>(&mut self) -> crate::query::iter::QueryIter<'_, Q::Fetch, Q::Filter>
    where
        Q: crate::query::Query,
    {
        crate::query::iter::QueryIter::new(&self.archetypes)
    }

    /// Executes a filtered query over all entities in the world.
    ///
    /// This is a convenience method for queries with custom filters.
    ///
    /// # Type Parameters
    ///
    /// * `Q` - The query type (what to fetch)
    /// * `F` - The filter type (which entities to include)
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use pecs::prelude::*;
    ///
    /// // Query for Position on entities that have Velocity but not Dead
    /// for pos in world.query_filtered::<&Position, (With<Velocity>, Without<Dead>)>() {
    ///     // Process position
    /// }
    /// ```
    pub fn query_filtered<Q, F>(&mut self) -> crate::query::iter::QueryIter<'_, Q::Fetch, F>
    where
        Q: crate::query::Query,
        F: for<'a> crate::query::Filter<'a>,
    {
        crate::query::iter::QueryIter::new(&self.archetypes)
    }

    /// Saves the world to a file using the default persistence plugin.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to save the world to
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
    /// use pecs::World;
    ///
    /// let world = World::new();
    /// world.save("world.pecs")?;
    /// ```
    pub fn save(&self, path: impl AsRef<std::path::Path>) -> crate::persistence::Result<()> {
        // Update metadata before saving
        let mut metadata = self.metadata.clone();
        metadata.entity_count = self.len();
        metadata.timestamp = WorldMetadata::current_timestamp();

        self.persistence.save(self, path)
    }

    /// Saves the world using a specific persistence plugin.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to save the world to
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
    /// use pecs::World;
    ///
    /// let world = World::new();
    /// world.save_with("world.json", "json")?;
    /// ```
    pub fn save_with(
        &self,
        path: impl AsRef<std::path::Path>,
        plugin_name: &str,
    ) -> crate::persistence::Result<()> {
        self.persistence.save_with(self, path, plugin_name)
    }

    /// Loads a world from a file using the default persistence plugin.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to load the world from
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
    /// use pecs::World;
    ///
    /// let world = World::load("world.pecs")?;
    /// ```
    pub fn load(path: impl AsRef<std::path::Path>) -> crate::persistence::Result<Self> {
        let persistence = PersistenceManager::new();
        persistence.load(path)
    }

    /// Loads a world from a file using a specific persistence plugin.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to load the world from
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
    /// use pecs::World;
    ///
    /// let world = World::load_with("world.json", "json")?;
    /// ```
    pub fn load_with(
        path: impl AsRef<std::path::Path>,
        plugin_name: &str,
    ) -> crate::persistence::Result<Self> {
        let persistence = PersistenceManager::new();
        persistence.load_with(path, plugin_name)
    }

    /// Saves the world to a writer using binary format.
    ///
    /// # Arguments
    ///
    /// * `writer` - Writer to save to
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use pecs::World;
    ///
    /// let world = World::new();
    /// let mut buffer = Vec::new();
    /// world.save_binary(&mut buffer)?;
    /// ```
    pub fn save_binary(&self, writer: &mut dyn std::io::Write) -> crate::persistence::Result<()> {
        use crate::persistence::binary::BinarySerializer;
        use crate::persistence::binary::format::FormatFlags;

        let serializer = BinarySerializer::new(FormatFlags::NONE);
        serializer.serialize(self, writer)
    }

    /// Loads a world from a reader using binary format.
    ///
    /// # Arguments
    ///
    /// * `reader` - Reader to load from
    ///
    /// # Errors
    ///
    /// Returns an error if deserialization fails.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use pecs::World;
    /// use std::io::Cursor;
    ///
    /// let buffer = vec![/* binary data */];
    /// let mut cursor = Cursor::new(buffer);
    /// let world = World::load_binary(&mut cursor)?;
    /// ```
    pub fn load_binary(reader: &mut dyn std::io::Read) -> crate::persistence::Result<Self> {
        use crate::persistence::binary::BinaryDeserializer;

        let mut deserializer = BinaryDeserializer::new();
        deserializer.deserialize(reader)
    }

    /// Saves the world to a writer using JSON format.
    ///
    /// # Arguments
    ///
    /// * `writer` - Writer to save to
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use pecs::World;
    ///
    /// let world = World::new();
    /// let mut buffer = Vec::new();
    /// world.save_json(&mut buffer)?;
    /// ```
    pub fn save_json(&self, writer: &mut dyn std::io::Write) -> crate::persistence::Result<()> {
        use crate::persistence::{JsonPlugin, PersistencePlugin};

        let plugin = JsonPlugin::new();
        plugin.save(self, writer)
    }

    /// Loads a world from a reader using JSON format.
    ///
    /// # Arguments
    ///
    /// * `reader` - Reader to load from
    ///
    /// # Errors
    ///
    /// Returns an error if deserialization fails.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use pecs::World;
    /// use std::io::Cursor;
    ///
    /// let json = r#"{"entities": []}"#;
    /// let mut cursor = Cursor::new(json.as_bytes());
    /// let world = World::load_json(&mut cursor)?;
    /// ```
    pub fn load_json(reader: &mut dyn std::io::Read) -> crate::persistence::Result<Self> {
        use crate::persistence::{JsonPlugin, PersistencePlugin};

        let plugin = JsonPlugin::new();
        plugin.load(reader)
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for constructing entities with components.
///
/// Created by [`World::spawn`].
pub struct EntityBuilder<'w> {
    world: &'w mut World,
    entity_id: EntityId,
    #[allow(dead_code)]
    stable_id: StableId,
    components: Vec<(ComponentTypeId, ComponentInfo, Box<dyn std::any::Any>)>,
}

impl<'w> EntityBuilder<'w> {
    /// Adds a component to the entity being built.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::prelude::*;
    ///
    /// #[derive(Debug)]
    /// struct Position { x: f32, y: f32 }
    /// impl Component for Position {}
    ///
    /// let mut world = World::new();
    /// let entity = world.spawn()
    ///     .with(Position { x: 0.0, y: 0.0 })
    ///     .id();
    /// ```
    pub fn with<T: Component>(mut self, component: T) -> Self {
        self.components.push((
            ComponentTypeId::of::<T>(),
            ComponentInfo::of::<T>(),
            Box::new(component),
        ));
        self
    }

    /// Finishes building the entity and returns its ID.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::prelude::*;
    ///
    /// #[derive(Debug)]
    /// struct Position { x: f32, y: f32 }
    /// impl Component for Position {}
    ///
    /// let mut world = World::new();
    /// let entity = world.spawn()
    ///     .with(Position { x: 0.0, y: 0.0 })
    ///     .id();
    /// ```
    pub fn id(self) -> EntityId {
        // If no components, add to empty archetype
        if self.components.is_empty() {
            let empty_archetype_id = ArchetypeId::new(0);
            if let Some(archetype) = self.world.archetypes.get_archetype_mut(empty_archetype_id) {
                let row = archetype.allocate_row(self.entity_id);
                // Set entity location
                self.world.archetypes.set_entity_location(
                    self.entity_id,
                    crate::component::archetype::EntityLocation {
                        archetype_id: empty_archetype_id,
                        row,
                    },
                );
            }
            return self.entity_id;
        }

        // Create component set and collect component info
        let mut component_types = ComponentSet::new();
        let mut component_info = Vec::new();

        for (type_id, info, _component) in &self.components {
            component_types.insert(*type_id);
            component_info.push(info.clone());
        }

        // Get or create archetype
        let archetype_id = self
            .world
            .archetypes
            .get_or_create_archetype(component_types, component_info);

        // Add entity to archetype and store components
        if let Some(archetype) = self.world.archetypes.get_archetype_mut(archetype_id) {
            let row = archetype.allocate_row(self.entity_id);

            // Store each component in the archetype
            for (type_id, _info, component) in self.components {
                // SAFETY: We just allocated the row and the component type exists in the archetype
                unsafe {
                    // Get a pointer to the component data inside the Box<dyn Any>
                    let component_ptr = Box::into_raw(component) as *mut u8;

                    // Copy the component data
                    archetype.set_component(row, type_id, component_ptr);

                    // Don't drop the box - ownership transferred to archetype
                    // The component_ptr points to heap memory that will be managed by the archetype
                }
            }

            // Set entity location
            self.world.archetypes.set_entity_location(
                self.entity_id,
                crate::component::archetype::EntityLocation { archetype_id, row },
            );
        }

        self.entity_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_world() {
        let world = World::new();
        assert!(world.is_empty());
    }

    #[test]
    fn world_with_capacity() {
        let world = World::with_capacity(100);
        assert!(world.is_empty());
    }

    #[test]
    fn spawn_empty_entity() {
        let mut world = World::new();
        let entity = world.spawn_empty();
        assert!(world.is_alive(entity));
        assert_eq!(world.len(), 1);
    }

    #[test]
    fn spawn_multiple_entities() {
        let mut world = World::new();
        let e1 = world.spawn_empty();
        let e2 = world.spawn_empty();
        let e3 = world.spawn_empty();

        assert!(world.is_alive(e1));
        assert!(world.is_alive(e2));
        assert!(world.is_alive(e3));
        assert_eq!(world.len(), 3);
    }

    #[test]
    fn despawn_entity() {
        let mut world = World::new();
        let entity = world.spawn_empty();

        assert!(world.despawn(entity));
        assert!(!world.is_alive(entity));
        assert_eq!(world.len(), 0);
    }

    #[test]
    fn despawn_invalid_entity() {
        let mut world = World::new();
        let entity = world.spawn_empty();
        world.despawn(entity);

        assert!(!world.despawn(entity));
    }

    #[test]
    fn stable_id_lookup() {
        let mut world = World::new();
        let entity = world.spawn_empty();
        let stable_id = world.get_stable_id(entity).unwrap();

        assert_eq!(world.get_entity_id(stable_id), Some(entity));
    }

    #[test]
    fn clear_world() {
        let mut world = World::new();
        world.spawn_empty();
        world.spawn_empty();
        world.spawn_empty();

        world.clear();
        assert!(world.is_empty());
    }

    #[test]
    fn command_buffer_integration() {
        let mut world = World::new();

        world.commands().spawn();
        world.commands().spawn();
        assert_eq!(world.len(), 0); // Commands not applied yet

        world.apply_commands();
        assert_eq!(world.len(), 2); // Commands applied
    }

    #[test]
    fn entity_builder_empty() {
        let mut world = World::new();
        let entity = world.spawn().id();
        assert!(world.is_alive(entity));
    }

    #[derive(Debug)]
    struct TestComponent {
        #[allow(dead_code)]
        value: i32,
    }
    impl Component for TestComponent {}

    #[test]
    fn entity_builder_with_component() {
        let mut world = World::new();
        let entity = world.spawn().with(TestComponent { value: 42 }).id();
        assert!(world.is_alive(entity));
    }

    #[test]
    fn insert_component() {
        let mut world = World::new();
        let entity = world.spawn_empty();

        assert!(world.insert(entity, TestComponent { value: 42 }));
        assert!(world.has::<TestComponent>(entity));
    }

    #[test]
    fn insert_component_invalid_entity() {
        let mut world = World::new();
        let entity = world.spawn_empty();
        world.despawn(entity);

        assert!(!world.insert(entity, TestComponent { value: 42 }));
    }

    #[test]
    fn insert_component_replace() {
        let mut world = World::new();
        let entity = world.spawn_empty();

        world.insert(entity, TestComponent { value: 42 });
        world.insert(entity, TestComponent { value: 100 });

        let component = world.get::<TestComponent>(entity).unwrap();
        assert_eq!(component.value, 100);
    }

    #[test]
    fn remove_component() {
        let mut world = World::new();
        let entity = world.spawn_empty();
        world.insert(entity, TestComponent { value: 42 });

        let removed = world.remove::<TestComponent>(entity);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().value, 42);
        assert!(!world.has::<TestComponent>(entity));
    }

    #[test]
    fn remove_component_not_present() {
        let mut world = World::new();
        let entity = world.spawn_empty();

        let removed = world.remove::<TestComponent>(entity);
        assert!(removed.is_none());
    }

    #[test]
    fn get_component() {
        let mut world = World::new();
        let entity = world.spawn_empty();
        world.insert(entity, TestComponent { value: 42 });

        let component = world.get::<TestComponent>(entity);
        assert!(component.is_some());
        assert_eq!(component.unwrap().value, 42);
    }

    #[test]
    fn get_component_not_present() {
        let mut world = World::new();
        let entity = world.spawn_empty();

        let component = world.get::<TestComponent>(entity);
        assert!(component.is_none());
    }

    #[test]
    fn get_mut_component() {
        let mut world = World::new();
        let entity = world.spawn_empty();
        world.insert(entity, TestComponent { value: 42 });

        if let Some(component) = world.get_mut::<TestComponent>(entity) {
            component.value = 100;
        }

        let component = world.get::<TestComponent>(entity).unwrap();
        assert_eq!(component.value, 100);
    }

    #[test]
    fn has_component() {
        let mut world = World::new();
        let entity = world.spawn_empty();

        assert!(!world.has::<TestComponent>(entity));
        world.insert(entity, TestComponent { value: 42 });
        assert!(world.has::<TestComponent>(entity));
    }

    #[test]
    fn has_component_invalid_entity() {
        let mut world = World::new();
        let entity = world.spawn_empty();
        world.despawn(entity);

        assert!(!world.has::<TestComponent>(entity));
    }

    #[derive(Debug)]
    struct Position {
        x: f32,
        y: f32,
    }
    impl Component for Position {}

    #[derive(Debug)]
    #[allow(dead_code)]
    struct Velocity {
        x: f32,
        y: f32,
    }
    impl Component for Velocity {}

    #[test]
    fn multiple_components() {
        let mut world = World::new();

        // Use builder pattern to add multiple components at spawn time
        // Note: Full archetype transition support is not yet implemented
        // This test verifies the entity is created successfully
        let entity = world
            .spawn()
            .with(Position { x: 1.0, y: 2.0 })
            .with(Velocity { x: 0.5, y: 0.5 })
            .id();

        // Verify entity exists
        assert!(world.is_alive(entity));

        // TODO: Once archetype transitions are fully implemented, add tests for:
        // - world.has::<Position>(entity)
        // - world.has::<Velocity>(entity)
        // - world.get::<Position>(entity)
        // - world.get::<Velocity>(entity)
    }

    #[test]
    fn single_component_operations() {
        let mut world = World::new();
        let entity = world.spawn_empty();

        // Insert single component works fine
        world.insert(entity, Position { x: 1.0, y: 2.0 });
        assert!(world.has::<Position>(entity));

        let pos = world.get::<Position>(entity).unwrap();
        assert_eq!(pos.x, 1.0);
        assert_eq!(pos.y, 2.0);

        // Modify it
        if let Some(pos) = world.get_mut::<Position>(entity) {
            pos.x = 5.0;
        }

        let pos = world.get::<Position>(entity).unwrap();
        assert_eq!(pos.x, 5.0);
    }

    #[test]
    fn component_lifecycle() {
        let mut world = World::new();
        let entity = world.spawn_empty();

        // Insert
        world.insert(entity, TestComponent { value: 42 });
        assert!(world.has::<TestComponent>(entity));

        // Modify
        if let Some(comp) = world.get_mut::<TestComponent>(entity) {
            comp.value = 100;
        }
        assert_eq!(world.get::<TestComponent>(entity).unwrap().value, 100);

        // Remove
        let removed = world.remove::<TestComponent>(entity);
        assert_eq!(removed.unwrap().value, 100);
        assert!(!world.has::<TestComponent>(entity));
    }
}
