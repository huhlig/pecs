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

//! Query trait implementations for component access patterns.
//!
//! This module provides implementations of the `Query` trait for various
//! component access patterns, enabling type-safe queries over entities.

use super::Query;
use super::fetch::{FetchEntity, FetchOptional, FetchRead, FetchWrite};
use crate::component::Component;
use crate::entity::EntityId;

// ============================================================================
// Single Component Queries
// ============================================================================

/// Query implementation for immutable component references.
///
/// Allows querying for `&T` where `T` is a component type.
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
/// for pos in world.query::<&Position>() {
///     println!("Position: ({}, {})", pos.x, pos.y);
/// }
/// ```
impl<T: Component> Query for &T {
    type Item<'a> = &'a T;
    type Fetch = FetchRead<T>;
    type Filter = ();
}

/// Query implementation for mutable component references.
///
/// Allows querying for `&mut T` where `T` is a component type.
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
/// for pos in world.query::<&mut Position>() {
///     pos.x += 1.0;
/// }
/// ```
impl<T: Component> Query for &mut T {
    type Item<'a> = &'a mut T;
    type Fetch = FetchWrite<T>;
    type Filter = ();
}

/// Query implementation for optional component references.
///
/// Allows querying for `Option<&T>` where `T` is a component type.
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
/// for pos in world.query::<Option<&Position>>() {
///     if let Some(p) = pos {
///         println!("Position: ({}, {})", p.x, p.y);
///     }
/// }
/// ```
impl<T: Component> Query for Option<&T> {
    type Item<'a> = Option<&'a T>;
    type Fetch = FetchOptional<T>;
    type Filter = ();
}

/// Query implementation for entity IDs.
///
/// Allows including the entity ID in query results.
///
/// # Examples
///
/// ```
/// use pecs::prelude::*;
///
/// let mut world = World::new();
/// for entity_id in world.query::<EntityId>() {
///     println!("Entity: {:?}", entity_id);
/// }
/// ```
impl Query for EntityId {
    type Item<'a> = EntityId;
    type Fetch = FetchEntity;
    type Filter = ();
}

// ============================================================================
// Tuple Query Implementations
// ============================================================================

// Macro to implement Query for tuples
macro_rules! impl_query_tuple {
    ($($T:ident),*) => {
        #[allow(non_snake_case)]
        impl<$($T: Query),*> Query for ($($T,)*) {
            type Item<'a> = ($($T::Item<'a>,)*);
            type Fetch = ($($T::Fetch,)*);
            type Filter = ();
        }
    };
}

// Implement Query for tuples up to 8 elements
impl_query_tuple!(A);
impl_query_tuple!(A, B);
impl_query_tuple!(A, B, C);
impl_query_tuple!(A, B, C, D);
impl_query_tuple!(A, B, C, D, E);
impl_query_tuple!(A, B, C, D, E, F);
impl_query_tuple!(A, B, C, D, E, F, G);
impl_query_tuple!(A, B, C, D, E, F, G, H);

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq)]
    struct Position {
        x: f32,
        y: f32,
    }
    impl Component for Position {}

    #[derive(Debug, Clone, Copy, PartialEq)]
    struct Velocity {
        x: f32,
        y: f32,
    }
    impl Component for Velocity {}

    #[test]
    fn query_single_immutable() {
        // Type check that Query is implemented
        fn _test<Q: Query>() {}
        _test::<&Position>();
    }

    #[test]
    fn query_single_mutable() {
        fn _test<Q: Query>() {}
        _test::<&mut Position>();
    }

    #[test]
    fn query_optional() {
        fn _test<Q: Query>() {}
        _test::<Option<&Position>>();
    }

    #[test]
    fn query_entity_id() {
        fn _test<Q: Query>() {}
        _test::<EntityId>();
    }

    #[test]
    fn query_tuple_two() {
        fn _test<Q: Query>() {}
        _test::<(&Position, &Velocity)>();
        _test::<(&mut Position, &Velocity)>();
        _test::<(EntityId, &Position)>();
    }

    #[test]
    fn query_tuple_three() {
        fn _test<Q: Query>() {}
        _test::<(EntityId, &Position, &Velocity)>();
        _test::<(&mut Position, &Velocity, Option<&Position>)>();
    }
}
