//! Archetype-based component storage.
//!
//! An archetype represents a unique combination of component types. All entities
//! with the same set of components belong to the same archetype, enabling
//! cache-friendly iteration and efficient queries.

use super::storage::ComponentStorage;
use super::{ComponentInfo, ComponentSet, ComponentTypeId};
use crate::entity::EntityId;
use std::collections::HashMap;

/// A unique identifier for an archetype.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArchetypeId(usize);

impl ArchetypeId {
    /// Creates a new archetype ID.
    pub const fn new(id: usize) -> Self {
        Self(id)
    }

    /// Returns the raw ID value.
    pub const fn index(self) -> usize {
        self.0
    }
}

/// The location of an entity within an archetype.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityLocation {
    /// The archetype containing the entity
    pub archetype_id: ArchetypeId,

    /// The row (index) of the entity within the archetype
    pub row: usize,
}

/// Edges to other archetypes for efficient component add/remove operations.
///
/// When a component is added or removed from an entity, it moves to a different
/// archetype. These edges cache the target archetype IDs to avoid repeated lookups.
#[derive(Debug, Default)]
pub struct ArchetypeEdges {
    /// Map from added component type to target archetype
    add_edges: HashMap<ComponentTypeId, ArchetypeId>,

    /// Map from removed component type to target archetype
    remove_edges: HashMap<ComponentTypeId, ArchetypeId>,
}

impl ArchetypeEdges {
    /// Creates new empty archetype edges.
    pub fn new() -> Self {
        Self::default()
    }

    /// Gets the target archetype when adding a component.
    pub fn get_add(&self, component_type: ComponentTypeId) -> Option<ArchetypeId> {
        self.add_edges.get(&component_type).copied()
    }

    /// Gets the target archetype when removing a component.
    pub fn get_remove(&self, component_type: ComponentTypeId) -> Option<ArchetypeId> {
        self.remove_edges.get(&component_type).copied()
    }

    /// Sets the target archetype when adding a component.
    pub fn set_add(&mut self, component_type: ComponentTypeId, target: ArchetypeId) {
        self.add_edges.insert(component_type, target);
    }

    /// Sets the target archetype when removing a component.
    pub fn set_remove(&mut self, component_type: ComponentTypeId, target: ArchetypeId) {
        self.remove_edges.insert(component_type, target);
    }
}

/// An archetype stores all entities with a specific combination of components.
///
/// Components are stored in a Structure of Arrays (SoA) layout for cache-friendly
/// iteration. Each component type has its own contiguous array.
pub struct Archetype {
    /// Unique identifier for this archetype
    id: ArchetypeId,

    /// The set of component types in this archetype
    component_types: ComponentSet,

    /// Storage for each component type
    component_storage: HashMap<ComponentTypeId, ComponentStorage>,

    /// Component metadata in the same order as component_types
    component_info: Vec<ComponentInfo>,

    /// List of entities in this archetype
    entities: Vec<EntityId>,

    /// Map from entity ID to row index for fast lookup
    entity_index: HashMap<EntityId, usize>,

    /// Edges to other archetypes for add/remove operations
    edges: ArchetypeEdges,
}

impl Archetype {
    /// Creates a new archetype with the given component types.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for this archetype
    /// * `component_types` - Set of component types in this archetype
    /// * `component_info` - Metadata for each component type
    pub fn new(
        id: ArchetypeId,
        component_types: ComponentSet,
        component_info: Vec<ComponentInfo>,
    ) -> Self {
        let mut component_storage = HashMap::new();

        for info in &component_info {
            component_storage.insert(info.type_id(), ComponentStorage::new(info.clone()));
        }

        Self {
            id,
            component_types,
            component_storage,
            component_info,
            entities: Vec::new(),
            entity_index: HashMap::new(),
            edges: ArchetypeEdges::new(),
        }
    }

    /// Returns the archetype ID.
    pub fn id(&self) -> ArchetypeId {
        self.id
    }

    /// Returns the component types in this archetype.
    pub fn component_types(&self) -> &ComponentSet {
        &self.component_types
    }

    /// Returns the number of entities in this archetype.
    pub fn len(&self) -> usize {
        self.entities.len()
    }

    /// Returns `true` if the archetype is empty.
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    /// Returns a slice of all entities in this archetype.
    pub fn entities(&self) -> &[EntityId] {
        &self.entities
    }

    /// Gets the row index for an entity.
    pub fn get_entity_row(&self, entity: EntityId) -> Option<usize> {
        self.entity_index.get(&entity).copied()
    }

    /// Checks if the archetype contains a specific component type.
    pub fn has_component(&self, component_type: ComponentTypeId) -> bool {
        self.component_types.contains(component_type)
    }

    /// Gets the component storage for a specific type.
    pub fn get_storage(&self, component_type: ComponentTypeId) -> Option<&ComponentStorage> {
        self.component_storage.get(&component_type)
    }

    /// Gets mutable component storage for a specific type.
    pub fn get_storage_mut(
        &mut self,
        component_type: ComponentTypeId,
    ) -> Option<&mut ComponentStorage> {
        self.component_storage.get_mut(&component_type)
    }

    /// Allocates a new row for an entity.
    ///
    /// This reserves space in all component arrays but doesn't initialize
    /// the components yet. Returns the row index.
    pub fn allocate_row(&mut self, entity: EntityId) -> usize {
        let row = self.entities.len();
        self.entities.push(entity);
        self.entity_index.insert(entity, row);
        row
    }

    /// Adds a component to a specific row.
    ///
    /// # Safety
    ///
    /// - `row` must be a valid row index
    /// - `component` must point to a valid instance of the component type
    /// - The component type must exist in this archetype
    pub unsafe fn set_component(
        &mut self,
        row: usize,
        component_type: ComponentTypeId,
        component: *const u8,
    ) {
        if let Some(storage) = self.component_storage.get_mut(&component_type) {
            // Ensure storage has capacity for this row
            while storage.len() <= row {
                // Push a dummy value (will be overwritten)
                let dummy = vec![0u8; storage.info().size()];
                // SAFETY: dummy is a valid pointer to initialized memory
                unsafe {
                    storage.push(dummy.as_ptr());
                }
            }

            // Now copy the actual component data
            // SAFETY: Caller ensures component is valid and row exists
            unsafe {
                let dst = storage.get_mut(row);
                std::ptr::copy_nonoverlapping(component, dst, storage.info().size());
            }
        }
    }

    /// Removes an entity from the archetype.
    ///
    /// This performs a swap-remove operation, moving the last entity into
    /// the removed position for O(1) performance.
    ///
    /// Returns the entity that was moved (if any).
    pub fn remove_entity(&mut self, entity: EntityId) -> Option<EntityId> {
        let row = self.entity_index.remove(&entity)?;

        // Swap-remove the entity
        let last_entity = self.entities.pop()?;
        let moved_entity = if row < self.entities.len() {
            self.entities[row] = last_entity;
            self.entity_index.insert(last_entity, row);
            Some(last_entity)
        } else {
            None
        };

        // Swap-remove components from all storages
        for storage in self.component_storage.values_mut() {
            if row < storage.len() {
                unsafe {
                    let mut temp = vec![0u8; storage.info().size()];
                    storage.swap_remove(row, temp.as_mut_ptr());
                }
            }
        }

        moved_entity
    }

    /// Moves an entity's components to another archetype.
    ///
    /// This is used when adding or removing components from an entity.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the target archetype has the correct
    /// component types for the operation being performed.
    pub unsafe fn move_entity_to(
        &mut self,
        entity: EntityId,
        target: &mut Archetype,
        component_data: &[(ComponentTypeId, *const u8)],
    ) -> Option<usize> {
        let row = self.get_entity_row(entity)?;

        // Allocate row in target archetype
        let target_row = target.allocate_row(entity);

        // Copy shared components
        for component_type in self.component_types.iter() {
            if target.has_component(component_type) {
                let src_storage = self.get_storage(component_type)?;
                // SAFETY: row is valid for this archetype
                unsafe {
                    let src_ptr = src_storage.get(row);
                    target.set_component(target_row, component_type, src_ptr);
                }
            }
        }

        // Add new components
        for (component_type, component_ptr) in component_data {
            if !self.has_component(*component_type) {
                // SAFETY: Caller ensures component_ptr is valid
                unsafe {
                    target.set_component(target_row, *component_type, *component_ptr);
                }
            }
        }

        // Remove entity from source archetype
        self.remove_entity(entity);

        Some(target_row)
    }

    /// Returns the archetype edges.
    pub fn edges(&self) -> &ArchetypeEdges {
        &self.edges
    }

    /// Returns mutable archetype edges.
    pub fn edges_mut(&mut self) -> &mut ArchetypeEdges {
        &mut self.edges
    }

    /// Clears all entities from the archetype.
    pub fn clear(&mut self) {
        self.entities.clear();
        self.entity_index.clear();
        for storage in self.component_storage.values_mut() {
            storage.clear();
        }
    }
}

/// Manages all archetypes in the world.
pub struct ArchetypeManager {
    /// All archetypes
    archetypes: Vec<Archetype>,

    /// Map from component set to archetype ID
    archetype_index: HashMap<ComponentSet, ArchetypeId>,

    /// Map from entity to its location
    entity_locations: HashMap<EntityId, EntityLocation>,
}

impl ArchetypeManager {
    /// Creates a new archetype manager.
    pub fn new() -> Self {
        let mut manager = Self {
            archetypes: Vec::new(),
            archetype_index: HashMap::new(),
            entity_locations: HashMap::new(),
        };

        // Create the empty archetype (archetype 0)
        let empty_archetype = Archetype::new(ArchetypeId::new(0), ComponentSet::new(), Vec::new());
        manager.archetypes.push(empty_archetype);
        manager
            .archetype_index
            .insert(ComponentSet::new(), ArchetypeId::new(0));

        manager
    }

    /// Gets or creates an archetype for the given component types.
    pub fn get_or_create_archetype(
        &mut self,
        component_types: ComponentSet,
        component_info: Vec<ComponentInfo>,
    ) -> ArchetypeId {
        if let Some(&id) = self.archetype_index.get(&component_types) {
            return id;
        }

        let id = ArchetypeId::new(self.archetypes.len());
        let archetype = Archetype::new(id, component_types.clone(), component_info);
        self.archetypes.push(archetype);
        self.archetype_index.insert(component_types, id);
        id
    }

    /// Gets an archetype by ID.
    pub fn get_archetype(&self, id: ArchetypeId) -> Option<&Archetype> {
        self.archetypes.get(id.index())
    }

    /// Gets a mutable archetype by ID.
    pub fn get_archetype_mut(&mut self, id: ArchetypeId) -> Option<&mut Archetype> {
        self.archetypes.get_mut(id.index())
    }

    /// Returns an iterator over all archetypes.
    pub fn iter(&self) -> impl Iterator<Item = &Archetype> {
        self.archetypes.iter()
    }

    /// Gets the location of an entity.
    pub fn get_entity_location(&self, entity: EntityId) -> Option<EntityLocation> {
        self.entity_locations.get(&entity).copied()
    }

    /// Sets the location of an entity.
    pub fn set_entity_location(&mut self, entity: EntityId, location: EntityLocation) {
        self.entity_locations.insert(entity, location);
    }

    /// Removes an entity's location.
    pub fn remove_entity_location(&mut self, entity: EntityId) -> Option<EntityLocation> {
        self.entity_locations.remove(&entity)
    }

    /// Returns the number of archetypes.
    pub fn len(&self) -> usize {
        self.archetypes.len()
    }

    /// Returns `true` if there are no archetypes (should never happen).
    pub fn is_empty(&self) -> bool {
        self.archetypes.is_empty()
    }
}

impl Default for ArchetypeManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::component::Component;

    #[derive(Debug, Clone, Copy)]
    struct Position {
        x: f32,
        y: f32,
    }
    impl Component for Position {}

    #[derive(Debug, Clone, Copy)]
    struct Velocity {
        x: f32,
        y: f32,
    }
    impl Component for Velocity {}

    #[test]
    fn archetype_creation() {
        let component_types = ComponentSet::new();
        let archetype = Archetype::new(ArchetypeId::new(0), component_types, Vec::new());

        assert_eq!(archetype.id(), ArchetypeId::new(0));
        assert!(archetype.is_empty());
    }

    #[test]
    fn archetype_manager_creation() {
        let manager = ArchetypeManager::new();

        assert_eq!(manager.len(), 1); // Empty archetype
        assert!(manager.get_archetype(ArchetypeId::new(0)).is_some());
    }

    #[test]
    fn archetype_manager_get_or_create() {
        let mut manager = ArchetypeManager::new();

        let mut types = ComponentSet::new();
        types.insert(ComponentTypeId::of::<Position>());

        let info = vec![ComponentInfo::of::<Position>()];
        let id1 = manager.get_or_create_archetype(types.clone(), info.clone());
        let id2 = manager.get_or_create_archetype(types.clone(), info.clone());

        assert_eq!(id1, id2); // Should return same archetype
        assert_eq!(manager.len(), 2); // Empty + Position archetype
    }

    #[test]
    fn entity_location_tracking() {
        let mut manager = ArchetypeManager::new();
        let entity = EntityId::new(0, 1);
        let location = EntityLocation {
            archetype_id: ArchetypeId::new(0),
            row: 0,
        };

        manager.set_entity_location(entity, location);
        assert_eq!(manager.get_entity_location(entity), Some(location));

        manager.remove_entity_location(entity);
        assert_eq!(manager.get_entity_location(entity), None);
    }

    #[test]
    fn archetype_edges() {
        let mut edges = ArchetypeEdges::new();
        let component_type = ComponentTypeId::of::<Position>();
        let target = ArchetypeId::new(1);

        edges.set_add(component_type, target);
        assert_eq!(edges.get_add(component_type), Some(target));

        edges.set_remove(component_type, target);
        assert_eq!(edges.get_remove(component_type), Some(target));
    }
}

// Made with Bob
