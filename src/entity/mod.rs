//! Entity management system.
//!
//! This module provides the core entity management functionality for the ECS,
//! including entity creation, deletion, and lookup operations.
//!
//! # Architecture
//!
//! The entity system uses a dual-ID approach:
//! - **Ephemeral IDs** ([`EntityId`]): Fast, 64-bit IDs for runtime operations
//! - **Stable IDs** ([`StableId`]): Persistent, 128-bit UUIDs for serialization
//!
//! # Examples
//!
//! ```
//! use pecs::entity::EntityManager;
//!
//! let mut manager = EntityManager::new();
//!
//! // Create entities
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

pub mod allocator;
pub mod id;

pub use allocator::EntityAllocator;
pub use id::{EntityId, StableId};

/// Error type for entity operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityError {
    /// The entity does not exist or is invalid.
    InvalidEntity,
    /// The stable ID is already in use.
    DuplicateStableId,
}

impl std::fmt::Display for EntityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EntityError::InvalidEntity => write!(f, "Invalid entity"),
            EntityError::DuplicateStableId => write!(f, "Stable ID already in use"),
        }
    }
}

impl std::error::Error for EntityError {}

/// High-level entity manager that coordinates entity lifecycle operations.
///
/// The `EntityManager` provides a convenient interface for:
/// - Spawning new entities
/// - Despawning entities
/// - Checking entity validity
/// - Looking up entities by stable ID
/// - Iterating over all entities
///
/// # Performance
///
/// - Spawn: O(1) amortized
/// - Despawn: O(1)
/// - Lookup: O(1)
/// - Iteration: O(n) where n is the number of alive entities
#[derive(Debug)]
pub struct EntityManager {
    /// The underlying allocator that manages entity IDs
    allocator: EntityAllocator,
}

impl EntityManager {
    /// Creates a new empty entity manager.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::entity::EntityManager;
    ///
    /// let manager = EntityManager::new();
    /// ```
    pub fn new() -> Self {
        Self {
            allocator: EntityAllocator::new(),
        }
    }

    /// Creates a new entity manager with pre-allocated capacity.
    ///
    /// # Arguments
    ///
    /// * `capacity` - Number of entity slots to pre-allocate
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::entity::EntityManager;
    ///
    /// let manager = EntityManager::with_capacity(1000);
    /// ```
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            allocator: EntityAllocator::with_capacity(capacity),
        }
    }

    /// Spawns a new entity, returning its ephemeral ID.
    ///
    /// The entity is created with both an ephemeral ID (for fast runtime access)
    /// and a stable ID (for persistence). The stable ID can be retrieved using
    /// [`get_stable_id`](Self::get_stable_id).
    ///
    /// # Returns
    ///
    /// The [`EntityId`] of the newly spawned entity.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::entity::EntityManager;
    ///
    /// let mut manager = EntityManager::new();
    /// let entity = manager.spawn();
    /// assert!(manager.is_alive(entity));
    /// ```
    pub fn spawn(&mut self) -> EntityId {
        let (entity_id, _stable_id) = self.allocator.allocate();
        entity_id
    }

    /// Spawns a new entity and returns both its ephemeral and stable IDs.
    ///
    /// This is useful when you need immediate access to the stable ID,
    /// such as when setting up persistence or network synchronization.
    ///
    /// # Returns
    ///
    /// A tuple of `(EntityId, StableId)` for the newly spawned entity.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::entity::EntityManager;
    ///
    /// let mut manager = EntityManager::new();
    /// let (entity_id, stable_id) = manager.spawn_with_stable_id();
    /// assert_eq!(manager.get_stable_id(entity_id), Some(stable_id));
    /// ```
    pub fn spawn_with_stable_id(&mut self) -> (EntityId, StableId) {
        self.allocator.allocate()
    }

    /// Despawns an entity, removing it from the world.
    ///
    /// After despawning, the entity ID becomes invalid and any attempts to
    /// use it will fail. The entity's slot may be recycled for future entities.
    ///
    /// # Arguments
    ///
    /// * `entity` - The entity to despawn
    ///
    /// # Returns
    ///
    /// `true` if the entity was despawned, `false` if it was already dead or invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::entity::EntityManager;
    ///
    /// let mut manager = EntityManager::new();
    /// let entity = manager.spawn();
    /// assert!(manager.despawn(entity));
    /// assert!(!manager.is_alive(entity));
    /// ```
    pub fn despawn(&mut self, entity: EntityId) -> bool {
        self.allocator.free(entity)
    }

    /// Checks if an entity is currently alive (valid and allocated).
    ///
    /// # Arguments
    ///
    /// * `entity` - The entity to check
    ///
    /// # Returns
    ///
    /// `true` if the entity is alive, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::entity::EntityManager;
    ///
    /// let mut manager = EntityManager::new();
    /// let entity = manager.spawn();
    /// assert!(manager.is_alive(entity));
    /// manager.despawn(entity);
    /// assert!(!manager.is_alive(entity));
    /// ```
    pub fn is_alive(&self, entity: EntityId) -> bool {
        self.allocator.is_alive(entity)
    }

    /// Gets the stable ID for an entity.
    ///
    /// # Arguments
    ///
    /// * `entity` - The entity to lookup
    ///
    /// # Returns
    ///
    /// The stable ID if the entity is valid, `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::entity::EntityManager;
    ///
    /// let mut manager = EntityManager::new();
    /// let (entity_id, stable_id) = manager.spawn_with_stable_id();
    /// assert_eq!(manager.get_stable_id(entity_id), Some(stable_id));
    /// ```
    pub fn get_stable_id(&self, entity: EntityId) -> Option<StableId> {
        self.allocator.get_stable_id(entity)
    }

    /// Gets the ephemeral ID for a stable ID.
    ///
    /// This is useful when loading entities from persistence, where you have
    /// the stable ID and need to find the corresponding runtime entity.
    ///
    /// # Arguments
    ///
    /// * `stable_id` - The stable ID to lookup
    ///
    /// # Returns
    ///
    /// The ephemeral ID if the entity is valid, `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::entity::EntityManager;
    ///
    /// let mut manager = EntityManager::new();
    /// let (entity_id, stable_id) = manager.spawn_with_stable_id();
    /// assert_eq!(manager.get_entity_id(stable_id), Some(entity_id));
    /// ```
    pub fn get_entity_id(&self, stable_id: StableId) -> Option<EntityId> {
        self.allocator.get_entity_id(stable_id)
    }

    /// Returns the number of currently alive entities.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::entity::EntityManager;
    ///
    /// let mut manager = EntityManager::new();
    /// assert_eq!(manager.len(), 0);
    /// manager.spawn();
    /// assert_eq!(manager.len(), 1);
    /// ```
    pub fn len(&self) -> usize {
        self.allocator.len()
    }

    /// Returns `true` if no entities are alive.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::entity::EntityManager;
    ///
    /// let manager = EntityManager::new();
    /// assert!(manager.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.allocator.is_empty()
    }

    /// Returns the total capacity (allocated + free slots).
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::entity::EntityManager;
    ///
    /// let mut manager = EntityManager::new();
    /// manager.spawn();
    /// assert_eq!(manager.capacity(), 1);
    /// ```
    pub fn capacity(&self) -> usize {
        self.allocator.capacity()
    }

    /// Clears all entities, resetting the manager to empty state.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::entity::EntityManager;
    ///
    /// let mut manager = EntityManager::new();
    /// manager.spawn();
    /// manager.clear();
    /// assert!(manager.is_empty());
    /// ```
    pub fn clear(&mut self) {
        self.allocator.clear();
    }

    /// Reserves capacity for at least `additional` more entities.
    ///
    /// This can improve performance by reducing allocations when spawning
    /// many entities.
    ///
    /// # Arguments
    ///
    /// * `additional` - Number of additional entities to reserve space for
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::entity::EntityManager;
    ///
    /// let mut manager = EntityManager::new();
    /// manager.reserve(1000);
    /// ```
    pub fn reserve(&mut self, additional: usize) {
        self.allocator.reserve(additional);
    }

    /// Spawns an entity with a specific stable ID.
    ///
    /// This is used during deserialization to restore entities with their
    /// original stable IDs. If the stable ID already exists, this returns
    /// an error to prevent ID conflicts.
    ///
    /// # Arguments
    ///
    /// * `stable_id` - The stable ID to use for the entity
    ///
    /// # Returns
    ///
    /// The ephemeral ID for the entity, or an error if the stable ID is already in use.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::entity::{EntityManager, StableId};
    ///
    /// let mut manager = EntityManager::new();
    /// let stable_id = StableId::from_raw(12345);
    /// let entity_id = manager.spawn_with_id(stable_id).unwrap();
    /// assert_eq!(manager.get_stable_id(entity_id), Some(stable_id));
    /// ```
    pub fn spawn_with_id(&mut self, stable_id: StableId) -> Result<EntityId, EntityError> {
        self.allocator.allocate_with_stable_id(stable_id)
    }

    /// Remaps an existing entity to a new stable ID.
    ///
    /// This is useful for resolving ID conflicts during load operations.
    /// The old stable ID mapping is removed and replaced with the new one.
    ///
    /// # Arguments
    ///
    /// * `entity_id` - The entity to remap
    /// * `new_stable_id` - The new stable ID to assign
    ///
    /// # Returns
    ///
    /// The old stable ID if successful, or an error if the entity is invalid
    /// or the new stable ID is already in use.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::entity::{EntityManager, StableId};
    ///
    /// let mut manager = EntityManager::new();
    /// let (entity_id, old_stable_id) = manager.spawn_with_stable_id();
    /// let new_stable_id = StableId::from_raw(99999);
    ///
    /// let remapped = manager.remap_stable_id(entity_id, new_stable_id).unwrap();
    /// assert_eq!(remapped, old_stable_id);
    /// assert_eq!(manager.get_stable_id(entity_id), Some(new_stable_id));
    /// ```
    pub fn remap_stable_id(
        &mut self,
        entity_id: EntityId,
        new_stable_id: StableId,
    ) -> Result<StableId, EntityError> {
        self.allocator.remap_stable_id(entity_id, new_stable_id)
    }

    /// Returns an iterator over all alive entities and their stable IDs.
    ///
    /// This is useful for persistence operations that need to iterate
    /// over all entities.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::entity::EntityManager;
    ///
    /// let mut manager = EntityManager::new();
    /// manager.spawn();
    /// manager.spawn();
    ///
    /// let entities: Vec<_> = manager.iter().collect();
    /// assert_eq!(entities.len(), 2);
    /// ```
    pub fn iter(&self) -> impl Iterator<Item = (EntityId, StableId)> + '_ {
        self.allocator.iter()
    }
}

impl Default for EntityManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawn_entity() {
        let mut manager = EntityManager::new();
        let entity = manager.spawn();
        assert!(manager.is_alive(entity));
        assert_eq!(manager.len(), 1);
    }

    #[test]
    fn spawn_multiple_entities() {
        let mut manager = EntityManager::new();
        let e1 = manager.spawn();
        let e2 = manager.spawn();
        let e3 = manager.spawn();

        assert!(manager.is_alive(e1));
        assert!(manager.is_alive(e2));
        assert!(manager.is_alive(e3));
        assert_eq!(manager.len(), 3);
    }

    #[test]
    fn spawn_with_id() {
        let mut manager = EntityManager::new();
        let stable_id = StableId::from_raw(12345);

        let entity_id = manager.spawn_with_id(stable_id).unwrap();
        assert_eq!(manager.get_stable_id(entity_id), Some(stable_id));
        assert_eq!(manager.get_entity_id(stable_id), Some(entity_id));
    }

    #[test]
    fn spawn_with_duplicate_id() {
        let mut manager = EntityManager::new();
        let stable_id = StableId::from_raw(12345);

        manager.spawn_with_id(stable_id).unwrap();
        let result = manager.spawn_with_id(stable_id);

        assert!(result.is_err());
    }

    #[test]
    fn remap_entity_stable_id() {
        let mut manager = EntityManager::new();
        let (entity_id, old_stable_id) = manager.spawn_with_stable_id();
        let new_stable_id = StableId::from_raw(99999);

        let remapped = manager.remap_stable_id(entity_id, new_stable_id).unwrap();
        assert_eq!(remapped, old_stable_id);
        assert_eq!(manager.get_stable_id(entity_id), Some(new_stable_id));
        assert_eq!(manager.get_entity_id(new_stable_id), Some(entity_id));
        assert_eq!(manager.get_entity_id(old_stable_id), None);
    }

    #[test]
    fn iter_manager_entities() {
        let mut manager = EntityManager::new();
        let (e1, s1) = manager.spawn_with_stable_id();
        let (e2, s2) = manager.spawn_with_stable_id();

        let entities: Vec<_> = manager.iter().collect();
        assert_eq!(entities.len(), 2);
        assert!(entities.contains(&(e1, s1)));
        assert!(entities.contains(&(e2, s2)));
    }

    #[test]
    fn despawn_entity() {
        let mut manager = EntityManager::new();
        let entity = manager.spawn();

        assert!(manager.despawn(entity));
        assert!(!manager.is_alive(entity));
        assert_eq!(manager.len(), 0);
    }

    #[test]
    fn despawn_invalid_entity() {
        let mut manager = EntityManager::new();
        let entity = manager.spawn();
        manager.despawn(entity);

        // Try to despawn again
        assert!(!manager.despawn(entity));
    }

    #[test]
    fn spawn_with_stable_id() {
        let mut manager = EntityManager::new();
        let (entity_id, stable_id) = manager.spawn_with_stable_id();

        assert_eq!(manager.get_stable_id(entity_id), Some(stable_id));
        assert_eq!(manager.get_entity_id(stable_id), Some(entity_id));
    }

    #[test]
    fn stable_id_lookup_after_despawn() {
        let mut manager = EntityManager::new();
        let (entity_id, stable_id) = manager.spawn_with_stable_id();

        manager.despawn(entity_id);

        assert_eq!(manager.get_stable_id(entity_id), None);
        assert_eq!(manager.get_entity_id(stable_id), None);
    }

    #[test]
    fn entity_recycling() {
        let mut manager = EntityManager::new();
        let e1 = manager.spawn();
        manager.despawn(e1);

        let e2 = manager.spawn();
        assert_eq!(e2.index(), e1.index()); // Same index
        assert_ne!(e2.generation(), e1.generation()); // Different generation
        assert!(!manager.is_alive(e1)); // Old entity is dead
        assert!(manager.is_alive(e2)); // New entity is alive
    }

    #[test]
    fn clear_manager() {
        let mut manager = EntityManager::new();
        manager.spawn();
        manager.spawn();
        manager.spawn();

        manager.clear();
        assert!(manager.is_empty());
        assert_eq!(manager.len(), 0);
    }

    #[test]
    fn with_capacity() {
        let manager = EntityManager::with_capacity(100);
        assert!(manager.is_empty());
    }

    #[test]
    fn capacity_tracking() {
        let mut manager = EntityManager::new();
        assert_eq!(manager.capacity(), 0);

        manager.spawn();
        assert_eq!(manager.capacity(), 1);

        manager.spawn();
        assert_eq!(manager.capacity(), 2);
    }
}

// Made with Bob
