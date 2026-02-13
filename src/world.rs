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
use crate::component::{Component, ComponentSet, ComponentTypeId};
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
        self.commands.apply(&mut self.entities);
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
    components: Vec<(ComponentTypeId, Box<dyn std::any::Any>)>,
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
        self.components
            .push((ComponentTypeId::of::<T>(), Box::new(component)));
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
                archetype.allocate_row(self.entity_id);
            }
            return self.entity_id;
        }

        // Create component set
        let mut component_types = ComponentSet::new();

        for (type_id, _) in &self.components {
            component_types.insert(*type_id);
        }

        // Get component info for each type
        // Note: This is a simplified version. In a real implementation,
        // we'd need a registry to look up ComponentInfo by TypeId
        // For now, we'll create a minimal archetype
        let component_info = Vec::new();

        // Get or create archetype
        let archetype_id = self
            .world
            .archetypes
            .get_or_create_archetype(component_types, component_info);

        // Add entity to archetype
        if let Some(archetype) = self.world.archetypes.get_archetype_mut(archetype_id) {
            archetype.allocate_row(self.entity_id);
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
}

// Made with Bob
