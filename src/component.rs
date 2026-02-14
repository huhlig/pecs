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

//! Component storage and management system.
//!
//! This module provides the component storage infrastructure for the ECS,
//! using an archetype-based approach for optimal query performance.
//!
//! # Architecture
//!
//! Components are stored in archetypes, where each archetype represents a unique
//! combination of component types. This provides:
//! - Excellent cache locality for queries
//! - Fast iteration over entities with specific components
//! - Efficient memory usage
//!
//! # Examples
//!
//! ```
//! use pecs::component::Component;
//!
//! #[derive(Debug, Clone, Copy)]
//! struct Position {
//!     x: f32,
//!     y: f32,
//! }
//!
//! impl Component for Position {}
//!
//! #[derive(Debug, Clone, Copy)]
//! struct Velocity {
//!     x: f32,
//!     y: f32,
//! }
//!
//! impl Component for Velocity {}
//! ```

pub mod archetype;
pub mod storage;

use std::any::TypeId;
use std::fmt;

/// A component that can be attached to entities.
///
/// Components must be `'static` to ensure they can be safely stored and
/// retrieved using type IDs. They should typically be plain data structures.
///
/// # Safety
///
/// Components must not contain references with non-'static lifetimes.
///
/// # Examples
///
/// ```
/// use pecs::component::Component;
///
/// #[derive(Debug, Clone, Copy)]
/// struct Health {
///     current: i32,
///     max: i32,
/// }
///
/// impl Component for Health {}
/// ```
pub trait Component: 'static + Send + Sync {}

/// A unique identifier for a component type.
///
/// This is a wrapper around `TypeId` that provides additional functionality
/// for component type management.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ComponentTypeId(TypeId);

impl ComponentTypeId {
    /// Creates a new `ComponentTypeId` for a component type.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::component::{Component, ComponentTypeId};
    ///
    /// #[derive(Debug)]
    /// struct Position { x: f32, y: f32 }
    /// impl Component for Position {}
    ///
    /// let type_id = ComponentTypeId::of::<Position>();
    /// ```
    pub fn of<T: Component>() -> Self {
        Self(TypeId::of::<T>())
    }

    /// Returns the underlying `TypeId`.
    pub fn type_id(self) -> TypeId {
        self.0
    }
}

impl fmt::Display for ComponentTypeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ComponentType({:?})", self.0)
    }
}

/// Information about a component type.
///
/// This stores metadata needed for component storage and manipulation,
/// including size, alignment, and drop behavior.
#[derive(Debug, Clone)]
pub struct ComponentInfo {
    /// The type ID of the component
    type_id: ComponentTypeId,

    /// The name of the component type (for debugging)
    type_name: &'static str,

    /// Size of the component in bytes
    size: usize,

    /// Alignment requirement of the component
    alignment: usize,

    /// Whether the component needs to be dropped
    needs_drop: bool,

    /// Function to drop a component in place
    drop_fn: unsafe fn(*mut u8),
}

impl ComponentInfo {
    /// Creates component info for a specific component type.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::component::{Component, ComponentInfo};
    ///
    /// #[derive(Debug)]
    /// struct Position { x: f32, y: f32 }
    /// impl Component for Position {}
    ///
    /// let info = ComponentInfo::of::<Position>();
    /// assert_eq!(info.size(), std::mem::size_of::<Position>());
    /// ```
    pub fn of<T: Component>() -> Self {
        Self {
            type_id: ComponentTypeId::of::<T>(),
            type_name: std::any::type_name::<T>(),
            size: std::mem::size_of::<T>(),
            alignment: std::mem::align_of::<T>(),
            needs_drop: std::mem::needs_drop::<T>(),
            drop_fn: |ptr| unsafe {
                std::ptr::drop_in_place(ptr as *mut T);
            },
        }
    }

    /// Returns the component type ID.
    pub fn type_id(&self) -> ComponentTypeId {
        self.type_id
    }

    /// Returns the component type name.
    pub fn type_name(&self) -> &'static str {
        self.type_name
    }

    /// Returns the size of the component in bytes.
    pub fn size(&self) -> usize {
        self.size
    }

    /// Returns the alignment requirement of the component.
    pub fn alignment(&self) -> usize {
        self.alignment
    }

    /// Returns whether the component needs to be dropped.
    pub fn needs_drop(&self) -> bool {
        self.needs_drop
    }

    /// Drops a component at the given pointer.
    ///
    /// # Safety
    ///
    /// The pointer must point to a valid instance of this component type.
    pub unsafe fn drop(&self, ptr: *mut u8) {
        if self.needs_drop {
            // SAFETY: Caller ensures ptr points to valid component instance
            unsafe {
                (self.drop_fn)(ptr);
            }
        }
    }
}

/// A set of component types, used to identify archetypes.
///
/// Component sets are ordered by type ID to ensure consistent archetype
/// identification regardless of insertion order.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ComponentSet {
    /// Sorted list of component type IDs
    types: Vec<ComponentTypeId>,
}

impl ComponentSet {
    /// Creates a new empty component set.
    pub fn new() -> Self {
        Self { types: Vec::new() }
    }

    /// Creates a component set from a list of component types.
    ///
    /// The types are automatically sorted for consistent identification.
    pub fn from_types(mut types: Vec<ComponentTypeId>) -> Self {
        types.sort_unstable();
        types.dedup();
        Self { types }
    }

    /// Adds a component type to the set.
    ///
    /// Returns `true` if the type was added, `false` if it was already present.
    pub fn insert(&mut self, type_id: ComponentTypeId) -> bool {
        match self.types.binary_search(&type_id) {
            Ok(_) => false, // Already present
            Err(pos) => {
                self.types.insert(pos, type_id);
                true
            }
        }
    }

    /// Removes a component type from the set.
    ///
    /// Returns `true` if the type was removed, `false` if it wasn't present.
    pub fn remove(&mut self, type_id: ComponentTypeId) -> bool {
        match self.types.binary_search(&type_id) {
            Ok(pos) => {
                self.types.remove(pos);
                true
            }
            Err(_) => false,
        }
    }

    /// Checks if the set contains a component type.
    pub fn contains(&self, type_id: ComponentTypeId) -> bool {
        self.types.binary_search(&type_id).is_ok()
    }

    /// Returns the number of component types in the set.
    pub fn len(&self) -> usize {
        self.types.len()
    }

    /// Returns `true` if the set is empty.
    pub fn is_empty(&self) -> bool {
        self.types.is_empty()
    }

    /// Returns an iterator over the component type IDs.
    pub fn iter(&self) -> impl Iterator<Item = ComponentTypeId> + '_ {
        self.types.iter().copied()
    }

    /// Returns a slice of the component type IDs.
    pub fn as_slice(&self) -> &[ComponentTypeId] {
        &self.types
    }
}

impl Default for ComponentSet {
    fn default() -> Self {
        Self::new()
    }
}

impl FromIterator<ComponentTypeId> for ComponentSet {
    fn from_iter<T: IntoIterator<Item = ComponentTypeId>>(iter: T) -> Self {
        let types: Vec<_> = iter.into_iter().collect();
        Self::from_types(types)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    #[allow(dead_code)]
    struct TestComponent1 {
        value: i32,
    }
    impl Component for TestComponent1 {}

    #[derive(Debug)]
    #[allow(dead_code)]
    struct TestComponent2 {
        value: f32,
    }
    impl Component for TestComponent2 {}

    #[derive(Debug)]
    #[allow(dead_code)]
    struct TestComponent3 {
        data: Vec<u8>,
    }
    impl Component for TestComponent3 {}

    #[test]
    fn component_type_id_uniqueness() {
        let id1 = ComponentTypeId::of::<TestComponent1>();
        let id2 = ComponentTypeId::of::<TestComponent2>();
        let id3 = ComponentTypeId::of::<TestComponent1>();

        assert_ne!(id1, id2);
        assert_eq!(id1, id3);
    }

    #[test]
    fn component_info_creation() {
        let info = ComponentInfo::of::<TestComponent1>();

        assert_eq!(info.type_id(), ComponentTypeId::of::<TestComponent1>());
        assert_eq!(info.size(), std::mem::size_of::<TestComponent1>());
        assert_eq!(info.alignment(), std::mem::align_of::<TestComponent1>());
    }

    #[test]
    fn component_info_needs_drop() {
        let info1 = ComponentInfo::of::<TestComponent1>();
        let info3 = ComponentInfo::of::<TestComponent3>();

        assert!(!info1.needs_drop()); // i32 doesn't need drop
        assert!(info3.needs_drop()); // Vec<u8> needs drop
    }

    #[test]
    fn component_set_creation() {
        let set = ComponentSet::new();
        assert!(set.is_empty());
        assert_eq!(set.len(), 0);
    }

    #[test]
    fn component_set_insert() {
        let mut set = ComponentSet::new();
        let id1 = ComponentTypeId::of::<TestComponent1>();
        let id2 = ComponentTypeId::of::<TestComponent2>();

        assert!(set.insert(id1));
        assert!(set.insert(id2));
        assert!(!set.insert(id1)); // Already present

        assert_eq!(set.len(), 2);
        assert!(set.contains(id1));
        assert!(set.contains(id2));
    }

    #[test]
    fn component_set_remove() {
        let mut set = ComponentSet::new();
        let id1 = ComponentTypeId::of::<TestComponent1>();
        let id2 = ComponentTypeId::of::<TestComponent2>();

        set.insert(id1);
        set.insert(id2);

        assert!(set.remove(id1));
        assert!(!set.remove(id1)); // Already removed
        assert_eq!(set.len(), 1);
        assert!(!set.contains(id1));
        assert!(set.contains(id2));
    }

    #[test]
    fn component_set_ordering() {
        let id1 = ComponentTypeId::of::<TestComponent1>();
        let id2 = ComponentTypeId::of::<TestComponent2>();
        let id3 = ComponentTypeId::of::<TestComponent3>();

        // Insert in different orders
        let mut set1 = ComponentSet::new();
        set1.insert(id3);
        set1.insert(id1);
        set1.insert(id2);

        let mut set2 = ComponentSet::new();
        set2.insert(id1);
        set2.insert(id2);
        set2.insert(id3);

        // Should be equal regardless of insertion order
        assert_eq!(set1, set2);
    }

    #[test]
    fn component_set_from_types() {
        let id1 = ComponentTypeId::of::<TestComponent1>();
        let id2 = ComponentTypeId::of::<TestComponent2>();

        let set = ComponentSet::from_types(vec![id2, id1, id2]); // Duplicates and unsorted

        assert_eq!(set.len(), 2); // Duplicates removed
        assert!(set.contains(id1));
        assert!(set.contains(id2));
    }

    #[test]
    fn component_set_iteration() {
        let id1 = ComponentTypeId::of::<TestComponent1>();
        let id2 = ComponentTypeId::of::<TestComponent2>();

        let mut set = ComponentSet::new();
        set.insert(id1);
        set.insert(id2);

        let collected: Vec<_> = set.iter().collect();
        assert_eq!(collected.len(), 2);
    }
}
