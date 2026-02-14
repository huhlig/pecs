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

//! Query filters for narrowing down entity selection.
//!
//! Filters allow you to specify additional constraints on which entities
//! should be included in query results, beyond just component presence.
//!
//! # Performance Optimizations
//!
//! Filters are evaluated at two levels:
//! - Archetype-level: Filters like `With` and `Without` can eliminate entire archetypes
//! - Entity-level: Custom filters can check individual entities
//!
//! The query iterator uses archetype-level filtering to skip non-matching archetypes entirely.

use super::Filter;
use crate::component::{Component, archetype::Archetype};
use crate::entity::EntityId;
use std::marker::PhantomData;

/// A filter that requires an entity to have a specific component.
///
/// This is useful when you want to filter by component presence without
/// actually fetching the component data.
///
/// # Performance
///
/// This is an archetype-level filter, meaning entire archetypes can be skipped
/// if they don't have the required component. This is very efficient.
///
/// # Examples
///
/// ```ignore
/// // Query for Position, but only on entities that also have Velocity
/// world.query::<&Position>().filter::<With<Velocity>>()
/// ```
pub struct With<T: Component> {
    _phantom: PhantomData<T>,
}

impl<'a, T: Component> Filter<'a> for With<T> {
    #[inline(always)]
    fn matches(archetype: &Archetype, _entity: EntityId) -> bool {
        archetype.has_component::<T>()
    }
}

/// A filter that requires an entity to NOT have a specific component.
///
/// This is useful for excluding entities with certain components.
///
/// # Performance
///
/// This is an archetype-level filter, meaning entire archetypes can be skipped
/// if they have the excluded component. This is very efficient.
///
/// # Examples
///
/// ```ignore
/// // Query for Position, but exclude entities with Dead component
/// world.query::<&Position>().filter::<Without<Dead>>()
/// ```
pub struct Without<T: Component> {
    _phantom: PhantomData<T>,
}

impl<'a, T: Component> Filter<'a> for Without<T> {
    #[inline(always)]
    fn matches(archetype: &Archetype, _entity: EntityId) -> bool {
        !archetype.has_component::<T>()
    }
}

/// A filter that matches all entities (no filtering).
///
/// This is the default filter when none is specified.
impl<'a> Filter<'a> for () {
    fn matches(_archetype: &Archetype, _entity: EntityId) -> bool {
        true
    }
}

/// A filter that combines multiple filters with AND logic.
///
/// All filters must match for the entity to be included.
pub struct And<A, B> {
    _phantom: PhantomData<(A, B)>,
}

impl<'a, A: Filter<'a>, B: Filter<'a>> Filter<'a> for And<A, B> {
    fn matches(archetype: &Archetype, entity: EntityId) -> bool {
        A::matches(archetype, entity) && B::matches(archetype, entity)
    }
}

/// A filter that combines multiple filters with OR logic.
///
/// At least one filter must match for the entity to be included.
pub struct Or<A, B> {
    _phantom: PhantomData<(A, B)>,
}

impl<'a, A: Filter<'a>, B: Filter<'a>> Filter<'a> for Or<A, B> {
    fn matches(archetype: &Archetype, entity: EntityId) -> bool {
        A::matches(archetype, entity) || B::matches(archetype, entity)
    }
}

/// A filter that inverts another filter.
///
/// Matches when the inner filter does NOT match.
pub struct Not<F> {
    _phantom: PhantomData<F>,
}

impl<'a, F: Filter<'a>> Filter<'a> for Not<F> {
    fn matches(archetype: &Archetype, entity: EntityId) -> bool {
        !F::matches(archetype, entity)
    }
}

// Macro to implement Filter for tuples (AND logic)
macro_rules! impl_filter_tuple {
    ($($T:ident),*) => {
        #[allow(non_snake_case)]
        impl<'a, $($T: Filter<'a>),*> Filter<'a> for ($($T,)*) {
            fn matches(archetype: &Archetype, entity: EntityId) -> bool {
                $($T::matches(archetype, entity))&&*
            }
        }
    };
}

// Implement for tuples up to 8 elements
impl_filter_tuple!(A);
impl_filter_tuple!(A, B);
impl_filter_tuple!(A, B, C);
impl_filter_tuple!(A, B, C, D);
impl_filter_tuple!(A, B, C, D, E);
impl_filter_tuple!(A, B, C, D, E, F);
impl_filter_tuple!(A, B, C, D, E, F, G);
impl_filter_tuple!(A, B, C, D, E, F, G, H);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::component::Component;

    #[derive(Debug)]
    #[allow(dead_code)]
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

    #[derive(Debug)]
    #[allow(dead_code)]
    struct Dead;
    impl Component for Dead {}

    #[test]
    fn with_filter_type_check() {
        fn _test_filter<F: for<'a> Filter<'a>>() {}
        _test_filter::<With<Position>>();
    }

    #[test]
    fn without_filter_type_check() {
        fn _test_filter<F: for<'a> Filter<'a>>() {}
        _test_filter::<Without<Dead>>();
    }

    #[test]
    fn and_filter_type_check() {
        fn _test_filter<F: for<'a> Filter<'a>>() {}
        _test_filter::<And<With<Position>, With<Velocity>>>();
    }

    #[test]
    fn or_filter_type_check() {
        fn _test_filter<F: for<'a> Filter<'a>>() {}
        _test_filter::<Or<With<Position>, With<Velocity>>>();
    }

    #[test]
    fn not_filter_type_check() {
        fn _test_filter<F: for<'a> Filter<'a>>() {}
        _test_filter::<Not<With<Dead>>>();
    }

    #[test]
    fn tuple_filter_type_check() {
        fn _test_filter<F: for<'a> Filter<'a>>() {}
        _test_filter::<(With<Position>, Without<Dead>)>();
    }

    #[test]
    fn empty_filter_always_matches() {
        // Create a minimal archetype for testing
        use crate::component::ComponentSet;
        use crate::component::archetype::{Archetype, ArchetypeId};

        let archetype = Archetype::new(ArchetypeId::new(0), ComponentSet::new(), Vec::new());
        let entity = EntityId::new(0, 1);

        assert!(<() as Filter>::matches(&archetype, entity));
    }
}
