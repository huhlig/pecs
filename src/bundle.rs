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

//! Component bundles for ergonomic entity creation.
//!
//! Bundles allow spawning entities with multiple components in a single operation,
//! providing a more ergonomic API than chaining multiple `.with()` calls.
//!
//! # Examples
//!
//! ```
//! use pecs::prelude::*;
//!
//! #[derive(Component, Debug)]
//! struct Position { x: f32, y: f32 }
//!
//! #[derive(Component, Debug)]
//! struct Velocity { x: f32, y: f32 }
//!
//! let mut world = World::new();
//!
//! // Spawn with a bundle (tuple of components)
//! let entity = world.spawn_bundle((
//!     Position { x: 0.0, y: 0.0 },
//!     Velocity { x: 1.0, y: 0.0 },
//! ));
//! ```

use crate::World;
use crate::component::{Component, ComponentInfo, ComponentSet, ComponentTypeId};
use crate::entity::EntityId;

/// A bundle of components that can be inserted into an entity.
///
/// Bundles provide an ergonomic way to spawn entities with multiple components
/// at once. Any tuple of components automatically implements `Bundle`.
///
/// # Examples
///
/// ```
/// use pecs::prelude::*;
///
/// #[derive(Component)]
/// struct Position { x: f32, y: f32 }
///
/// #[derive(Component)]
/// struct Velocity { x: f32, y: f32 }
///
/// let mut world = World::new();
///
/// // Single component bundle
/// let e1 = world.spawn_bundle(Position { x: 0.0, y: 0.0 });
///
/// // Multi-component bundle
/// let e2 = world.spawn_bundle((
///     Position { x: 1.0, y: 1.0 },
///     Velocity { x: 0.5, y: 0.5 },
/// ));
/// ```
pub trait Bundle: 'static {
    /// Get the component type IDs in this bundle.
    fn component_types(&self) -> ComponentSet;

    /// Get the component info for all components in this bundle.
    fn component_info() -> Vec<ComponentInfo>;

    /// Insert this bundle's components into the world for the given entity.
    ///
    /// # Safety
    ///
    /// The caller must ensure the entity exists and the archetype has been
    /// properly set up with the correct component types.
    unsafe fn insert_into_world(self, world: &mut World, entity: EntityId);
}

// Implement Bundle for single components
impl<T: Component> Bundle for T {
    fn component_types(&self) -> ComponentSet {
        let mut set = ComponentSet::new();
        set.insert(ComponentTypeId::of::<T>());
        set
    }

    fn component_info() -> Vec<ComponentInfo> {
        vec![ComponentInfo::of::<T>()]
    }

    unsafe fn insert_into_world(self, world: &mut World, entity: EntityId) {
        world.insert(entity, self);
    }
}

// Macro to implement Bundle for tuples
macro_rules! impl_bundle_tuple {
    ($($T:ident),*) => {
        #[allow(non_snake_case)]
        impl<$($T: Component),*> Bundle for ($($T,)*) {
            fn component_types(&self) -> ComponentSet {
                let mut set = ComponentSet::new();
                $(
                    set.insert(ComponentTypeId::of::<$T>());
                )*
                set
            }

            fn component_info() -> Vec<ComponentInfo> {
                vec![
                    $(ComponentInfo::of::<$T>(),)*
                ]
            }

            unsafe fn insert_into_world(self, world: &mut World, entity: EntityId) {
                let ($($T,)*) = self;
                $(
                    world.insert(entity, $T);
                )*
            }
        }
    };
}

// Implement Bundle for tuples up to 16 components
impl_bundle_tuple!(T1);
impl_bundle_tuple!(T1, T2);
impl_bundle_tuple!(T1, T2, T3);
impl_bundle_tuple!(T1, T2, T3, T4);
impl_bundle_tuple!(T1, T2, T3, T4, T5);
impl_bundle_tuple!(T1, T2, T3, T4, T5, T6);
impl_bundle_tuple!(T1, T2, T3, T4, T5, T6, T7);
impl_bundle_tuple!(T1, T2, T3, T4, T5, T6, T7, T8);
impl_bundle_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9);
impl_bundle_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
impl_bundle_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);
impl_bundle_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);
impl_bundle_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13);
impl_bundle_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14);
impl_bundle_tuple!(
    T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15
);
impl_bundle_tuple!(
    T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16
);

impl World {
    /// Spawns a new entity with the given bundle of components.
    ///
    /// This is a more ergonomic alternative to using the builder pattern
    /// when you want to spawn an entity with multiple components at once.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::prelude::*;
    ///
    /// #[derive(Component)]
    /// struct Position { x: f32, y: f32 }
    ///
    /// #[derive(Component)]
    /// struct Velocity { x: f32, y: f32 }
    ///
    /// let mut world = World::new();
    ///
    /// // Spawn with single component
    /// let e1 = world.spawn_bundle(Position { x: 0.0, y: 0.0 });
    ///
    /// // Spawn with multiple components
    /// let e2 = world.spawn_bundle((
    ///     Position { x: 1.0, y: 1.0 },
    ///     Velocity { x: 0.5, y: 0.5 },
    /// ));
    /// ```
    pub fn spawn_bundle<B: Bundle>(&mut self, bundle: B) -> EntityId {
        let entity = self.spawn_empty();
        unsafe {
            bundle.insert_into_world(self, entity);
        }
        entity
    }

    /// Inserts a bundle of components into an existing entity.
    ///
    /// If the entity already has any of the component types in the bundle,
    /// they will be replaced. This operation may move the entity to a
    /// different archetype.
    ///
    /// # Arguments
    ///
    /// * `entity` - The entity to insert components into
    /// * `bundle` - The bundle of components to insert
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
    /// #[derive(Component)]
    /// struct Position { x: f32, y: f32 }
    ///
    /// #[derive(Component)]
    /// struct Velocity { x: f32, y: f32 }
    ///
    /// let mut world = World::new();
    /// let entity = world.spawn_empty();
    ///
    /// // Insert bundle into existing entity
    /// assert!(world.insert_bundle(entity, (
    ///     Position { x: 1.0, y: 1.0 },
    ///     Velocity { x: 0.5, y: 0.5 },
    /// )));
    /// ```
    pub fn insert_bundle<B: Bundle>(&mut self, entity: EntityId, bundle: B) -> bool {
        if !self.is_alive(entity) {
            return false;
        }
        unsafe {
            bundle.insert_into_world(self, entity);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    struct Position {
        x: f32,
        y: f32,
    }
    impl Component for Position {}

    #[derive(Debug, PartialEq)]
    struct Velocity {
        x: f32,
        y: f32,
    }
    impl Component for Velocity {}

    #[derive(Debug, PartialEq)]
    struct Health {
        current: i32,
        max: i32,
    }
    impl Component for Health {}

    #[test]
    fn test_spawn_bundle_single_component() {
        let mut world = World::new();
        let entity = world.spawn_bundle(Position { x: 1.0, y: 2.0 });

        assert!(world.is_alive(entity));
        assert!(world.has::<Position>(entity));
        let pos = world.get::<Position>(entity).unwrap();
        assert_eq!(pos.x, 1.0);
        assert_eq!(pos.y, 2.0);
    }

    #[test]
    fn test_spawn_bundle_two_components() {
        let mut world = World::new();
        let entity = world.spawn_bundle((Position { x: 1.0, y: 2.0 }, Velocity { x: 0.5, y: 0.5 }));

        assert!(world.is_alive(entity));
        assert!(world.has::<Position>(entity));
        assert!(world.has::<Velocity>(entity));

        let pos = world.get::<Position>(entity).unwrap();
        assert_eq!(pos.x, 1.0);
        assert_eq!(pos.y, 2.0);

        let vel = world.get::<Velocity>(entity).unwrap();
        assert_eq!(vel.x, 0.5);
        assert_eq!(vel.y, 0.5);
    }

    #[test]
    fn test_spawn_bundle_three_components() {
        let mut world = World::new();
        let entity = world.spawn_bundle((
            Position { x: 1.0, y: 2.0 },
            Velocity { x: 0.5, y: 0.5 },
            Health {
                current: 100,
                max: 100,
            },
        ));

        assert!(world.is_alive(entity));
        assert!(world.has::<Position>(entity));
        assert!(world.has::<Velocity>(entity));
        assert!(world.has::<Health>(entity));

        let health = world.get::<Health>(entity).unwrap();
        assert_eq!(health.current, 100);
        assert_eq!(health.max, 100);
    }

    #[test]
    fn test_insert_bundle_into_existing_entity() {
        let mut world = World::new();
        let entity = world.spawn_empty();

        assert!(world.insert_bundle(
            entity,
            (Position { x: 1.0, y: 2.0 }, Velocity { x: 0.5, y: 0.5 },)
        ));

        assert!(world.has::<Position>(entity));
        assert!(world.has::<Velocity>(entity));
    }

    #[test]
    fn test_insert_bundle_replaces_existing_components() {
        let mut world = World::new();
        let entity = world.spawn_bundle(Position { x: 1.0, y: 2.0 });

        // Insert bundle with Position (should replace) and Velocity (should add)
        assert!(world.insert_bundle(
            entity,
            (Position { x: 5.0, y: 6.0 }, Velocity { x: 0.5, y: 0.5 },)
        ));

        let pos = world.get::<Position>(entity).unwrap();
        assert_eq!(pos.x, 5.0);
        assert_eq!(pos.y, 6.0);

        assert!(world.has::<Velocity>(entity));
    }

    #[test]
    fn test_insert_bundle_nonexistent_entity() {
        let mut world = World::new();
        let entity = unsafe { EntityId::from_raw(999) };

        assert!(!world.insert_bundle(entity, Position { x: 1.0, y: 2.0 }));
    }

    #[test]
    fn test_spawn_multiple_entities_with_bundles() {
        let mut world = World::new();

        let e1 = world.spawn_bundle((Position { x: 1.0, y: 1.0 }, Velocity { x: 0.1, y: 0.1 }));
        let e2 = world.spawn_bundle((Position { x: 2.0, y: 2.0 }, Velocity { x: 0.2, y: 0.2 }));
        let e3 = world.spawn_bundle((Position { x: 3.0, y: 3.0 }, Velocity { x: 0.3, y: 0.3 }));

        assert_eq!(world.len(), 3);
        assert!(world.is_alive(e1));
        assert!(world.is_alive(e2));
        assert!(world.is_alive(e3));

        // Verify all have both components
        for entity in [e1, e2, e3] {
            assert!(world.has::<Position>(entity));
            assert!(world.has::<Velocity>(entity));
        }
    }

    #[test]
    fn test_query_after_bundle_spawn() {
        let mut world = World::new();

        world.spawn_bundle((Position { x: 1.0, y: 1.0 }, Velocity { x: 0.1, y: 0.1 }));
        world.spawn_bundle((Position { x: 2.0, y: 2.0 }, Velocity { x: 0.2, y: 0.2 }));
        world.spawn_bundle((Position { x: 3.0, y: 3.0 }, Velocity { x: 0.3, y: 0.3 }));

        let mut count = 0;
        for (pos, vel) in world.query::<(&Position, &Velocity)>() {
            assert!(pos.x > 0.0);
            assert!(vel.x > 0.0);
            count += 1;
        }
        assert_eq!(count, 3);
    }
}

// Made with Bob
