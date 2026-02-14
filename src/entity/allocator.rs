//! Entity ID allocation and recycling system.
//!
//! This module manages the lifecycle of entity IDs, including:
//! - Allocation of new entity IDs
//! - Recycling of freed entity IDs
//! - Mapping between ephemeral and stable IDs
//!
//! # Examples
//!
//! ```
//! use pecs::entity::allocator::EntityAllocator;
//!
//! let mut allocator = EntityAllocator::new();
//! let (entity_id, stable_id) = allocator.allocate();
//!
//! // Use the entity...
//!
//! allocator.free(entity_id);
//! ```

use super::EntityError;
use super::id::{EntityId, StableId};
use std::collections::HashMap;

/// Metadata for an entity slot in the allocator.
#[derive(Debug, Clone)]
struct EntityMeta {
    /// The current generation for this slot
    generation: u32,
    /// The stable ID associated with this entity (if allocated)
    stable_id: Option<StableId>,
}

/// Manages allocation and recycling of entity IDs.
///
/// The allocator maintains:
/// - A list of entity metadata (generation counters)
/// - A free list of recyclable entity indices
/// - Bidirectional mapping between ephemeral and stable IDs
///
/// # Performance
///
/// - Allocation: O(1) amortized
/// - Deallocation: O(1)
/// - Lookup: O(1)
#[derive(Debug)]
pub struct EntityAllocator {
    /// Metadata for all entity slots (allocated and free)
    meta: Vec<EntityMeta>,

    /// Indices of free entity slots available for recycling
    free_list: Vec<u32>,

    /// Map from ephemeral ID to stable ID
    ephemeral_to_stable: HashMap<EntityId, StableId>,

    /// Map from stable ID to ephemeral ID
    stable_to_ephemeral: HashMap<StableId, EntityId>,
}

impl EntityAllocator {
    /// Creates a new empty entity allocator.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::entity::allocator::EntityAllocator;
    ///
    /// let allocator = EntityAllocator::new();
    /// ```
    pub fn new() -> Self {
        Self::with_capacity(0)
    }

    /// Creates a new entity allocator with pre-allocated capacity.
    ///
    /// # Arguments
    ///
    /// * `capacity` - Number of entity slots to pre-allocate
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::entity::allocator::EntityAllocator;
    ///
    /// let allocator = EntityAllocator::with_capacity(1000);
    /// ```
    pub fn with_capacity(capacity: usize) -> Self {
        // Pre-allocate with a minimum capacity to avoid initial reallocations
        let initial_capacity = if capacity == 0 { 16 } else { capacity };
        Self {
            meta: Vec::with_capacity(initial_capacity),
            free_list: Vec::new(),
            ephemeral_to_stable: HashMap::with_capacity(initial_capacity),
            stable_to_ephemeral: HashMap::with_capacity(initial_capacity),
        }
    }

    /// Allocates a new entity, returning both ephemeral and stable IDs.
    ///
    /// If there are free slots available (from previously freed entities),
    /// one will be recycled with an incremented generation. Otherwise,
    /// a new slot is created.
    ///
    /// # Returns
    ///
    /// A tuple of `(EntityId, StableId)` for the newly allocated entity.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::entity::allocator::EntityAllocator;
    ///
    /// let mut allocator = EntityAllocator::new();
    /// let (entity_id, stable_id) = allocator.allocate();
    /// assert_eq!(entity_id.index(), 0);
    /// assert_eq!(entity_id.generation(), 1);
    /// ```
    pub fn allocate(&mut self) -> (EntityId, StableId) {
        let stable_id = StableId::new();

        let entity_id = if let Some(index) = self.free_list.pop() {
            // Recycle a free slot
            let meta = &mut self.meta[index as usize];
            meta.generation = meta.generation.wrapping_add(1).max(1);
            meta.stable_id = Some(stable_id);
            EntityId::new(index, meta.generation)
        } else {
            // Allocate a new slot
            let index = self.meta.len() as u32;
            self.meta.push(EntityMeta {
                generation: 1,
                stable_id: Some(stable_id),
            });
            EntityId::new(index, 1)
        };

        // Update bidirectional mapping
        // Using insert is fine here as we know these are new entries
        self.ephemeral_to_stable.insert(entity_id, stable_id);
        self.stable_to_ephemeral.insert(stable_id, entity_id);

        (entity_id, stable_id)
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
    /// use pecs::entity::allocator::EntityAllocator;
    ///
    /// let mut allocator = EntityAllocator::new();
    /// allocator.reserve(1000);
    /// ```
    pub fn reserve(&mut self, additional: usize) {
        self.meta.reserve(additional);
        self.ephemeral_to_stable.reserve(additional);
        self.stable_to_ephemeral.reserve(additional);
    }

    /// Frees an entity, making its slot available for recycling.
    ///
    /// The entity's generation is incremented to invalidate any existing
    /// references to this entity.
    ///
    /// # Arguments
    ///
    /// * `entity_id` - The entity to free
    ///
    /// # Returns
    ///
    /// `true` if the entity was freed, `false` if it was already free or invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::entity::allocator::EntityAllocator;
    ///
    /// let mut allocator = EntityAllocator::new();
    /// let (entity_id, _) = allocator.allocate();
    /// assert!(allocator.free(entity_id));
    /// assert!(!allocator.free(entity_id)); // Already freed
    /// ```
    pub fn free(&mut self, entity_id: EntityId) -> bool {
        let index = entity_id.index() as usize;

        // Validate the entity exists and matches generation
        if index >= self.meta.len() {
            return false;
        }

        let meta = &self.meta[index];
        if meta.generation != entity_id.generation() {
            return false; // Stale reference
        }

        if meta.stable_id.is_none() {
            return false; // Already freed
        }

        // Remove from mappings
        if let Some(stable_id) = meta.stable_id {
            self.ephemeral_to_stable.remove(&entity_id);
            self.stable_to_ephemeral.remove(&stable_id);
        }

        // Mark as free
        self.meta[index].stable_id = None;
        self.free_list.push(index as u32);

        true
    }

    /// Checks if an entity ID is currently valid (allocated).
    ///
    /// # Arguments
    ///
    /// * `entity_id` - The entity to check
    ///
    /// # Returns
    ///
    /// `true` if the entity is valid and allocated, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::entity::allocator::EntityAllocator;
    ///
    /// let mut allocator = EntityAllocator::new();
    /// let (entity_id, _) = allocator.allocate();
    /// assert!(allocator.is_alive(entity_id));
    /// allocator.free(entity_id);
    /// assert!(!allocator.is_alive(entity_id));
    /// ```
    pub fn is_alive(&self, entity_id: EntityId) -> bool {
        let index = entity_id.index() as usize;

        if index >= self.meta.len() {
            return false;
        }

        let meta = &self.meta[index];
        meta.generation == entity_id.generation() && meta.stable_id.is_some()
    }

    /// Gets the stable ID for an entity.
    ///
    /// # Arguments
    ///
    /// * `entity_id` - The entity to lookup
    ///
    /// # Returns
    ///
    /// The stable ID if the entity is valid, `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::entity::allocator::EntityAllocator;
    ///
    /// let mut allocator = EntityAllocator::new();
    /// let (entity_id, stable_id) = allocator.allocate();
    /// assert_eq!(allocator.get_stable_id(entity_id), Some(stable_id));
    /// ```
    pub fn get_stable_id(&self, entity_id: EntityId) -> Option<StableId> {
        self.ephemeral_to_stable.get(&entity_id).copied()
    }

    /// Gets the ephemeral ID for a stable ID.
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
    /// use pecs::entity::allocator::EntityAllocator;
    ///
    /// let mut allocator = EntityAllocator::new();
    /// let (entity_id, stable_id) = allocator.allocate();
    /// assert_eq!(allocator.get_entity_id(stable_id), Some(entity_id));
    /// ```
    pub fn get_entity_id(&self, stable_id: StableId) -> Option<EntityId> {
        self.stable_to_ephemeral.get(&stable_id).copied()
    }

    /// Returns the total number of allocated entities.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::entity::allocator::EntityAllocator;
    ///
    /// let mut allocator = EntityAllocator::new();
    /// assert_eq!(allocator.len(), 0);
    /// allocator.allocate();
    /// assert_eq!(allocator.len(), 1);
    /// ```
    pub fn len(&self) -> usize {
        self.ephemeral_to_stable.len()
    }

    /// Returns `true` if no entities are allocated.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::entity::allocator::EntityAllocator;
    ///
    /// let allocator = EntityAllocator::new();
    /// assert!(allocator.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.ephemeral_to_stable.is_empty()
    }

    /// Returns the total capacity (allocated + free slots).
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::entity::allocator::EntityAllocator;
    ///
    /// let mut allocator = EntityAllocator::new();
    /// allocator.allocate();
    /// assert_eq!(allocator.capacity(), 1);
    /// ```
    pub fn capacity(&self) -> usize {
        self.meta.len()
    }

    /// Clears all entities, resetting the allocator to empty state.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::entity::allocator::EntityAllocator;
    ///
    /// let mut allocator = EntityAllocator::new();
    /// allocator.allocate();
    /// allocator.clear();
    /// assert!(allocator.is_empty());
    /// ```
    pub fn clear(&mut self) {
        self.meta.clear();
        self.free_list.clear();
        self.ephemeral_to_stable.clear();
        self.stable_to_ephemeral.clear();
    }

    /// Allocates an entity with a specific stable ID.
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
    /// use pecs::entity::allocator::EntityAllocator;
    /// use pecs::entity::id::StableId;
    ///
    /// let mut allocator = EntityAllocator::new();
    /// let stable_id = StableId::from_raw(12345);
    /// let entity_id = allocator.allocate_with_stable_id(stable_id).unwrap();
    /// assert_eq!(allocator.get_stable_id(entity_id), Some(stable_id));
    /// ```
    pub fn allocate_with_stable_id(
        &mut self,
        stable_id: StableId,
    ) -> Result<EntityId, EntityError> {
        // Check if stable ID already exists
        if self.stable_to_ephemeral.contains_key(&stable_id) {
            return Err(EntityError::DuplicateStableId);
        }

        let entity_id = if let Some(index) = self.free_list.pop() {
            // Recycle a free slot
            let meta = &mut self.meta[index as usize];
            meta.generation = meta.generation.wrapping_add(1).max(1);
            meta.stable_id = Some(stable_id);
            EntityId::new(index, meta.generation)
        } else {
            // Allocate a new slot
            let index = self.meta.len() as u32;
            self.meta.push(EntityMeta {
                generation: 1,
                stable_id: Some(stable_id),
            });
            EntityId::new(index, 1)
        };

        // Update bidirectional mapping
        self.ephemeral_to_stable.insert(entity_id, stable_id);
        self.stable_to_ephemeral.insert(stable_id, entity_id);

        Ok(entity_id)
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
    /// use pecs::entity::allocator::EntityAllocator;
    /// use pecs::entity::id::StableId;
    ///
    /// let mut allocator = EntityAllocator::new();
    /// let (entity_id, old_stable_id) = allocator.allocate();
    /// let new_stable_id = StableId::from_raw(99999);
    ///
    /// let remapped = allocator.remap_stable_id(entity_id, new_stable_id).unwrap();
    /// assert_eq!(remapped, old_stable_id);
    /// assert_eq!(allocator.get_stable_id(entity_id), Some(new_stable_id));
    /// ```
    pub fn remap_stable_id(
        &mut self,
        entity_id: EntityId,
        new_stable_id: StableId,
    ) -> Result<StableId, EntityError> {
        let index = entity_id.index() as usize;

        // Validate the entity exists and matches generation
        if index >= self.meta.len() {
            return Err(EntityError::InvalidEntity);
        }

        let meta = &self.meta[index];
        if meta.generation != entity_id.generation() {
            return Err(EntityError::InvalidEntity); // Stale reference
        }

        let old_stable_id = meta.stable_id.ok_or(EntityError::InvalidEntity)?;

        // Check if new stable ID is already in use
        if self.stable_to_ephemeral.contains_key(&new_stable_id) {
            return Err(EntityError::DuplicateStableId);
        }

        // Remove old mapping
        self.ephemeral_to_stable.remove(&entity_id);
        self.stable_to_ephemeral.remove(&old_stable_id);

        // Add new mapping
        self.meta[index].stable_id = Some(new_stable_id);
        self.ephemeral_to_stable.insert(entity_id, new_stable_id);
        self.stable_to_ephemeral.insert(new_stable_id, entity_id);

        Ok(old_stable_id)
    }

    /// Returns an iterator over all alive entities and their stable IDs.
    ///
    /// This is useful for persistence operations that need to iterate
    /// over all entities.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::entity::allocator::EntityAllocator;
    ///
    /// let mut allocator = EntityAllocator::new();
    /// allocator.allocate();
    /// allocator.allocate();
    ///
    /// let entities: Vec<_> = allocator.iter().collect();
    /// assert_eq!(entities.len(), 2);
    /// ```
    pub fn iter(&self) -> impl Iterator<Item = (EntityId, StableId)> + '_ {
        self.ephemeral_to_stable
            .iter()
            .map(|(&entity_id, &stable_id)| (entity_id, stable_id))
    }
}

impl Default for EntityAllocator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocate_single_entity() {
        let mut allocator = EntityAllocator::new();
        let (entity_id, stable_id) = allocator.allocate();

        assert_eq!(entity_id.index(), 0);
        assert_eq!(entity_id.generation(), 1);
        assert!(allocator.is_alive(entity_id));
        assert_eq!(allocator.get_stable_id(entity_id), Some(stable_id));
        assert_eq!(allocator.get_entity_id(stable_id), Some(entity_id));
    }

    #[test]
    fn allocate_multiple_entities() {
        let mut allocator = EntityAllocator::new();
        let (id1, _) = allocator.allocate();
        let (id2, _) = allocator.allocate();
        let (id3, _) = allocator.allocate();

        assert_eq!(id1.index(), 0);
        assert_eq!(id2.index(), 1);
        assert_eq!(id3.index(), 2);
        assert_eq!(allocator.len(), 3);
    }

    #[test]
    fn free_entity() {
        let mut allocator = EntityAllocator::new();
        let (entity_id, stable_id) = allocator.allocate();

        assert!(allocator.free(entity_id));
        assert!(!allocator.is_alive(entity_id));
        assert_eq!(allocator.get_stable_id(entity_id), None);
        assert_eq!(allocator.get_entity_id(stable_id), None);
        assert_eq!(allocator.len(), 0);
    }

    #[test]
    fn recycle_entity_slot() {
        let mut allocator = EntityAllocator::new();
        let (id1, _) = allocator.allocate();
        allocator.free(id1);

        let (id2, _) = allocator.allocate();
        assert_eq!(id2.index(), 0); // Same index
        assert_eq!(id2.generation(), 2); // Incremented generation
        assert!(!allocator.is_alive(id1)); // Old ID is invalid
        assert!(allocator.is_alive(id2)); // New ID is valid
    }

    #[test]
    fn free_invalid_entity() {
        let mut allocator = EntityAllocator::new();
        let (entity_id, _) = allocator.allocate();
        allocator.free(entity_id);

        // Try to free again
        assert!(!allocator.free(entity_id));
    }

    #[test]
    fn free_stale_reference() {
        let mut allocator = EntityAllocator::new();
        let (id1, _) = allocator.allocate();
        allocator.free(id1);
        let (id2, _) = allocator.allocate();

        // id1 is now stale (same index, different generation)
        assert!(!allocator.free(id1));
        assert!(allocator.is_alive(id2));
    }

    #[test]
    fn capacity_tracking() {
        let mut allocator = EntityAllocator::new();
        assert_eq!(allocator.capacity(), 0);

        allocator.allocate();
        assert_eq!(allocator.capacity(), 1);

        allocator.allocate();
        assert_eq!(allocator.capacity(), 2);
    }

    #[test]
    fn clear_allocator() {
        let mut allocator = EntityAllocator::new();
        allocator.allocate();
        allocator.allocate();

        allocator.clear();
        assert!(allocator.is_empty());
        assert_eq!(allocator.capacity(), 0);
    }

    #[test]
    fn with_capacity() {
        let allocator = EntityAllocator::with_capacity(100);
        assert_eq!(allocator.capacity(), 0); // Capacity is reserved, not used
        assert!(allocator.is_empty());
    }

    #[test]
    fn stable_id_uniqueness() {
        let mut allocator = EntityAllocator::new();
        let (_, stable1) = allocator.allocate();
        let (_, stable2) = allocator.allocate();

        assert_ne!(stable1, stable2);
    }

    #[test]
    fn bidirectional_mapping() {
        let mut allocator = EntityAllocator::new();
        let (entity_id, stable_id) = allocator.allocate();

        assert_eq!(allocator.get_stable_id(entity_id), Some(stable_id));
        assert_eq!(allocator.get_entity_id(stable_id), Some(entity_id));

        allocator.free(entity_id);

        assert_eq!(allocator.get_stable_id(entity_id), None);
        assert_eq!(allocator.get_entity_id(stable_id), None);
    }

    #[test]
    fn allocate_with_stable_id() {
        let mut allocator = EntityAllocator::new();
        let stable_id = StableId::from_raw(12345);

        let entity_id = allocator.allocate_with_stable_id(stable_id).unwrap();
        assert_eq!(allocator.get_stable_id(entity_id), Some(stable_id));
        assert_eq!(allocator.get_entity_id(stable_id), Some(entity_id));
    }

    #[test]
    fn allocate_with_duplicate_stable_id() {
        let mut allocator = EntityAllocator::new();
        let stable_id = StableId::from_raw(12345);

        allocator.allocate_with_stable_id(stable_id).unwrap();
        let result = allocator.allocate_with_stable_id(stable_id);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), EntityError::DuplicateStableId);
    }

    #[test]
    fn remap_stable_id() {
        let mut allocator = EntityAllocator::new();
        let (entity_id, old_stable_id) = allocator.allocate();
        let new_stable_id = StableId::from_raw(99999);

        let remapped = allocator.remap_stable_id(entity_id, new_stable_id).unwrap();
        assert_eq!(remapped, old_stable_id);
        assert_eq!(allocator.get_stable_id(entity_id), Some(new_stable_id));
        assert_eq!(allocator.get_entity_id(new_stable_id), Some(entity_id));
        assert_eq!(allocator.get_entity_id(old_stable_id), None);
    }

    #[test]
    fn remap_to_existing_stable_id() {
        let mut allocator = EntityAllocator::new();
        let (entity1, _) = allocator.allocate();
        let (_, stable_id2) = allocator.allocate();

        // Try to remap entity1 to stable_id2 (which is already in use)
        let result = allocator.remap_stable_id(entity1, stable_id2);
        assert!(result.is_err());
    }

    #[test]
    fn remap_invalid_entity() {
        let mut allocator = EntityAllocator::new();
        let entity = EntityId::new(999, 1); // Non-existent entity
        let new_stable_id = StableId::from_raw(12345);

        let result = allocator.remap_stable_id(entity, new_stable_id);
        assert!(result.is_err());
    }

    #[test]
    fn iter_entities() {
        let mut allocator = EntityAllocator::new();
        let (e1, s1) = allocator.allocate();
        let (e2, s2) = allocator.allocate();
        let (e3, s3) = allocator.allocate();

        let entities: Vec<_> = allocator.iter().collect();
        assert_eq!(entities.len(), 3);
        assert!(entities.contains(&(e1, s1)));
        assert!(entities.contains(&(e2, s2)));
        assert!(entities.contains(&(e3, s3)));
    }

    #[test]
    fn iter_after_despawn() {
        let mut allocator = EntityAllocator::new();
        let (e1, _) = allocator.allocate();
        let (e2, s2) = allocator.allocate();

        allocator.free(e1);

        let entities: Vec<_> = allocator.iter().collect();
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0], (e2, s2));
    }
}

// Made with Bob
