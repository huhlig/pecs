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

//! Component storage implementations.
//!
//! This module provides the low-level storage mechanisms for components,
//! including type-erased storage and safe access patterns.

use super::{Component, ComponentInfo};
use std::alloc::{self, Layout};
use std::ptr::NonNull;

/// A type-erased storage for a single component type.
///
/// This stores components in a contiguous array with proper alignment,
/// allowing for cache-friendly iteration while maintaining type safety
/// through the component info.
pub struct ComponentStorage {
    /// Metadata about the stored component type
    info: ComponentInfo,

    /// Pointer to the allocated memory
    data: NonNull<u8>,

    /// Number of components currently stored
    len: usize,

    /// Capacity of the allocated memory
    capacity: usize,
}

impl ComponentStorage {
    /// Creates a new empty component storage for a specific component type.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::component::{Component, ComponentInfo, storage::ComponentStorage};
    ///
    /// #[derive(Debug)]
    /// struct Position { x: f32, y: f32 }
    /// impl Component for Position {}
    ///
    /// let info = ComponentInfo::of::<Position>();
    /// let storage = ComponentStorage::new(info);
    /// assert_eq!(storage.len(), 0);
    /// ```
    pub fn new(info: ComponentInfo) -> Self {
        Self {
            info,
            data: NonNull::dangling(),
            len: 0,
            capacity: 0,
        }
    }

    /// Creates a component storage with pre-allocated capacity.
    pub fn with_capacity(info: ComponentInfo, capacity: usize) -> Self {
        let mut storage = Self::new(info);
        if capacity > 0 {
            storage.reserve(capacity);
        }
        storage
    }

    /// Returns the component info for this storage.
    pub fn info(&self) -> &ComponentInfo {
        &self.info
    }

    /// Returns the number of components stored.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if the storage is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns the capacity of the storage.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Reserves capacity for at least `additional` more components.
    pub fn reserve(&mut self, additional: usize) {
        let required = self.len.checked_add(additional).expect("capacity overflow");
        if required <= self.capacity {
            return;
        }

        // Use a more aggressive growth strategy for better amortized performance
        // Growth factor of 1.5x is optimal for memory reuse while minimizing reallocations
        let new_capacity = required.max((self.capacity * 3) / 2).max(16); // Start with 16 instead of 4 to reduce early reallocations
        self.realloc(new_capacity);
    }

    /// Reallocates the storage to a new capacity.
    fn realloc(&mut self, new_capacity: usize) {
        assert!(new_capacity >= self.len);

        let component_size = self.info.size();
        let component_align = self.info.alignment();

        if component_size == 0 {
            // Zero-sized types don't need allocation
            self.capacity = new_capacity;
            return;
        }

        let new_layout = Layout::from_size_align(component_size * new_capacity, component_align)
            .expect("invalid layout");

        let new_ptr = if self.capacity == 0 {
            // Initial allocation
            unsafe { alloc::alloc(new_layout) }
        } else {
            // Reallocation
            let old_layout =
                Layout::from_size_align(component_size * self.capacity, component_align)
                    .expect("invalid layout");

            unsafe { alloc::realloc(self.data.as_ptr(), old_layout, new_layout.size()) }
        };

        self.data = NonNull::new(new_ptr).expect("allocation failed");
        self.capacity = new_capacity;
    }

    /// Pushes a component to the end of the storage.
    ///
    /// # Safety
    ///
    /// The component pointer must point to a valid instance of the component type
    /// for this storage. The component will be moved (not copied) into storage.
    pub unsafe fn push(&mut self, component: *const u8) {
        if self.len == self.capacity {
            self.reserve(1);
        }

        let component_size = self.info.size();
        // SAFETY: Caller ensures component is valid and we have capacity
        unsafe {
            let dst = self.data.as_ptr().add(self.len * component_size);
            std::ptr::copy_nonoverlapping(component, dst, component_size);
        }
        self.len += 1;
    }

    /// Removes and returns the component at the given index.
    ///
    /// This performs a swap-remove operation, moving the last component
    /// into the removed position for O(1) performance.
    ///
    /// # Safety
    ///
    /// - `index` must be less than `len()`
    /// - `dst` must point to valid memory with proper alignment for the component type
    pub unsafe fn swap_remove(&mut self, index: usize, dst: *mut u8) {
        assert!(index < self.len);

        let component_size = self.info.size();
        // SAFETY: Caller ensures index is valid and dst is properly aligned
        unsafe {
            let src = self.data.as_ptr().add(index * component_size);

            // Copy the component to destination
            std::ptr::copy_nonoverlapping(src, dst, component_size);

            // Move the last component into the removed position
            if index != self.len - 1 {
                let last = self.data.as_ptr().add((self.len - 1) * component_size);
                std::ptr::copy(last, src, component_size);
            }
        }

        self.len -= 1;
    }

    /// Gets a pointer to the component at the given index.
    ///
    /// # Safety
    ///
    /// `index` must be less than `len()`.
    pub unsafe fn get(&self, index: usize) -> *const u8 {
        assert!(index < self.len);
        // SAFETY: Caller ensures index is valid
        unsafe { self.data.as_ptr().add(index * self.info.size()) }
    }

    /// Gets a mutable pointer to the component at the given index.
    ///
    /// # Safety
    ///
    /// `index` must be less than `len()`.
    pub unsafe fn get_mut(&mut self, index: usize) -> *mut u8 {
        assert!(index < self.len);
        // SAFETY: Caller ensures index is valid
        unsafe { self.data.as_ptr().add(index * self.info.size()) }
    }

    /// Returns a pointer to the start of the component array.
    pub fn as_ptr(&self) -> *const u8 {
        self.data.as_ptr()
    }

    /// Returns a mutable pointer to the start of the component array.
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.data.as_ptr()
    }

    /// Clears all components from the storage, dropping them if necessary.
    pub fn clear(&mut self) {
        if self.info.needs_drop() {
            let component_size = self.info.size();
            for i in 0..self.len {
                unsafe {
                    let ptr = self.data.as_ptr().add(i * component_size);
                    self.info.drop(ptr);
                }
            }
        }
        self.len = 0;
    }
}

impl Drop for ComponentStorage {
    fn drop(&mut self) {
        // Drop all components
        self.clear();

        // Deallocate memory
        if self.capacity > 0 && self.info.size() > 0 {
            let layout =
                Layout::from_size_align(self.info.size() * self.capacity, self.info.alignment())
                    .expect("invalid layout");

            unsafe {
                alloc::dealloc(self.data.as_ptr(), layout);
            }
        }
    }
}

// Safety: ComponentStorage can be sent between threads if the component type is Send
unsafe impl Send for ComponentStorage {}
// Safety: ComponentStorage can be shared between threads if the component type is Sync
unsafe impl Sync for ComponentStorage {}

/// A typed wrapper around ComponentStorage for safe access.
///
/// This provides type-safe access to components while using the type-erased
/// storage underneath.
pub struct TypedComponentStorage<T: Component> {
    storage: ComponentStorage,
    _marker: std::marker::PhantomData<T>,
}

impl<T: Component> TypedComponentStorage<T> {
    /// Creates a new typed component storage.
    pub fn new() -> Self {
        Self {
            storage: ComponentStorage::new(ComponentInfo::of::<T>()),
            _marker: std::marker::PhantomData,
        }
    }

    /// Creates a typed component storage with capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            storage: ComponentStorage::with_capacity(ComponentInfo::of::<T>(), capacity),
            _marker: std::marker::PhantomData,
        }
    }

    /// Returns the number of components stored.
    pub fn len(&self) -> usize {
        self.storage.len()
    }

    /// Returns `true` if the storage is empty.
    pub fn is_empty(&self) -> bool {
        self.storage.is_empty()
    }

    /// Pushes a component to the storage.
    pub fn push(&mut self, component: T) {
        unsafe {
            self.storage.push(&component as *const T as *const u8);
            std::mem::forget(component); // Ownership transferred to storage
        }
    }

    /// Removes and returns the component at the given index.
    ///
    /// This performs a swap-remove operation.
    ///
    /// # Panics
    ///
    /// Panics if `index >= len()`.
    pub fn swap_remove(&mut self, index: usize) -> T {
        assert!(index < self.len());
        unsafe {
            let mut component = std::mem::MaybeUninit::<T>::uninit();
            self.storage
                .swap_remove(index, component.as_mut_ptr() as *mut u8);
            component.assume_init()
        }
    }

    /// Gets a reference to the component at the given index.
    ///
    /// # Panics
    ///
    /// Panics if `index >= len()`.
    pub fn get(&self, index: usize) -> &T {
        assert!(index < self.len());
        unsafe { &*(self.storage.get(index) as *const T) }
    }

    /// Gets a mutable reference to the component at the given index.
    ///
    /// # Panics
    ///
    /// Panics if `index >= len()`.
    pub fn get_mut(&mut self, index: usize) -> &mut T {
        assert!(index < self.len());
        unsafe { &mut *(self.storage.get_mut(index) as *mut T) }
    }

    /// Returns an iterator over the components.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        (0..self.len()).map(move |i| self.get(i))
    }

    /// Returns a mutable iterator over the components.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        let len = self.len();
        let ptr = self.storage.as_mut_ptr();
        let size = std::mem::size_of::<T>();

        (0..len).map(move |i| unsafe { &mut *(ptr.add(i * size) as *mut T) })
    }

    /// Clears all components from the storage.
    pub fn clear(&mut self) {
        self.storage.clear();
    }
}

impl<T: Component> Default for TypedComponentStorage<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq)]
    struct Position {
        x: f32,
        y: f32,
    }
    impl Component for Position {}

    #[derive(Debug, Clone, PartialEq)]
    struct Name {
        value: String,
    }
    impl Component for Name {}

    #[test]
    fn component_storage_creation() {
        let info = ComponentInfo::of::<Position>();
        let storage = ComponentStorage::new(info);

        assert_eq!(storage.len(), 0);
        assert!(storage.is_empty());
    }

    #[test]
    fn typed_storage_push_and_get() {
        let mut storage = TypedComponentStorage::<Position>::new();

        storage.push(Position { x: 1.0, y: 2.0 });
        storage.push(Position { x: 3.0, y: 4.0 });

        assert_eq!(storage.len(), 2);
        assert_eq!(storage.get(0), &Position { x: 1.0, y: 2.0 });
        assert_eq!(storage.get(1), &Position { x: 3.0, y: 4.0 });
    }

    #[test]
    fn typed_storage_swap_remove() {
        let mut storage = TypedComponentStorage::<Position>::new();

        storage.push(Position { x: 1.0, y: 2.0 });
        storage.push(Position { x: 3.0, y: 4.0 });
        storage.push(Position { x: 5.0, y: 6.0 });

        let removed = storage.swap_remove(0);
        assert_eq!(removed, Position { x: 1.0, y: 2.0 });
        assert_eq!(storage.len(), 2);

        // Last element should have moved to index 0
        assert_eq!(storage.get(0), &Position { x: 5.0, y: 6.0 });
        assert_eq!(storage.get(1), &Position { x: 3.0, y: 4.0 });
    }

    #[test]
    fn typed_storage_iteration() {
        let mut storage = TypedComponentStorage::<Position>::new();

        storage.push(Position { x: 1.0, y: 2.0 });
        storage.push(Position { x: 3.0, y: 4.0 });

        let positions: Vec<_> = storage.iter().copied().collect();
        assert_eq!(positions.len(), 2);
        assert_eq!(positions[0], Position { x: 1.0, y: 2.0 });
        assert_eq!(positions[1], Position { x: 3.0, y: 4.0 });
    }

    #[test]
    fn typed_storage_mut_iteration() {
        let mut storage = TypedComponentStorage::<Position>::new();

        storage.push(Position { x: 1.0, y: 2.0 });
        storage.push(Position { x: 3.0, y: 4.0 });

        for pos in storage.iter_mut() {
            pos.x += 10.0;
        }

        assert_eq!(storage.get(0).x, 11.0);
        assert_eq!(storage.get(1).x, 13.0);
    }

    #[test]
    fn typed_storage_with_drop() {
        let mut storage = TypedComponentStorage::<Name>::new();

        storage.push(Name {
            value: "Alice".to_string(),
        });
        storage.push(Name {
            value: "Bob".to_string(),
        });

        assert_eq!(storage.len(), 2);

        storage.clear();
        assert_eq!(storage.len(), 0);
    }

    #[test]
    fn typed_storage_capacity() {
        let storage = TypedComponentStorage::<Position>::with_capacity(10);
        assert_eq!(storage.len(), 0);
        assert!(storage.storage.capacity() >= 10);
    }
}
