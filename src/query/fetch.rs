//! Component fetching implementations.
//!
//! This module provides implementations of the `Fetch` trait for various
//! component access patterns, including immutable references, mutable references,
//! and tuples of these.
//!
//! # Performance Optimizations
//!
//! - All fetch operations are marked with `#[inline(always)]` for zero-cost abstraction
//! - Archetype matching is optimized with inline hints
//! - Unsafe operations are carefully documented and optimized

use super::Fetch;
use crate::component::{Component, archetype::Archetype};
use crate::entity::EntityId;
use std::marker::PhantomData;

/// Fetch implementation for immutable component references.
///
/// This allows querying for `&T` where `T` is a component type.
///
/// # Performance
///
/// This is a zero-cost abstraction when optimized. The fetch operation
/// compiles down to a simple pointer dereference.
pub struct FetchRead<T: Component> {
    _phantom: PhantomData<T>,
}

impl<'a, T: Component> Fetch<'a> for FetchRead<T> {
    type Item = &'a T;

    #[inline(always)]
    fn matches_archetype(archetype: &Archetype) -> bool {
        archetype.has_component::<T>()
    }

    #[inline(always)]
    unsafe fn fetch(archetype: &'a Archetype, entity: EntityId) -> Self::Item {
        // SAFETY: Caller ensures entity exists and archetype matches
        // The archetype must have this component type (verified by matches_archetype)
        unsafe {
            archetype
                .get_component::<T>(entity)
                .expect("Entity must have component in matching archetype")
        }
    }
}

/// Fetch implementation for mutable component references.
///
/// This allows querying for `&mut T` where `T` is a component type.
///
/// # Performance
///
/// This is a zero-cost abstraction when optimized. The fetch operation
/// compiles down to a simple pointer dereference.
pub struct FetchWrite<T: Component> {
    _phantom: PhantomData<T>,
}

impl<'a, T: Component> Fetch<'a> for FetchWrite<T> {
    type Item = &'a mut T;

    #[inline(always)]
    fn matches_archetype(archetype: &Archetype) -> bool {
        archetype.has_component::<T>()
    }

    #[inline(always)]
    unsafe fn fetch(archetype: &'a Archetype, entity: EntityId) -> Self::Item {
        // SAFETY: Caller ensures entity exists, archetype matches, and access is exclusive
        // The archetype must have this component type (verified by matches_archetype)
        unsafe {
            let ptr = archetype
                .get_component_ptr::<T>(entity)
                .expect("Entity must have component in matching archetype");
            &mut *(ptr as *mut T)
        }
    }
}

/// Fetch implementation for optional component references.
///
/// This allows querying for `Option<&T>` where `T` is a component type.
///
/// # Performance
///
/// Slightly slower than required fetches due to the Option check,
/// but still very efficient.
pub struct FetchOptional<T: Component> {
    _phantom: PhantomData<T>,
}

impl<'a, T: Component> Fetch<'a> for FetchOptional<T> {
    type Item = Option<&'a T>;

    #[inline(always)]
    fn matches_archetype(_archetype: &Archetype) -> bool {
        // Optional fetches always match
        true
    }

    #[inline(always)]
    unsafe fn fetch(archetype: &'a Archetype, entity: EntityId) -> Self::Item {
        // SAFETY: Caller ensures entity exists
        unsafe { archetype.get_component::<T>(entity) }
    }
}

/// Fetch implementation for entity IDs.
///
/// This allows including the entity ID in query results.
///
/// # Performance
///
/// This is the fastest fetch operation as it just returns the entity ID
/// without any memory access.
pub struct FetchEntity;

impl<'a> Fetch<'a> for FetchEntity {
    type Item = EntityId;

    #[inline(always)]
    fn matches_archetype(_archetype: &Archetype) -> bool {
        // Entity fetch always matches
        true
    }

    #[inline(always)]
    unsafe fn fetch(_archetype: &'a Archetype, entity: EntityId) -> Self::Item {
        entity
    }
}

// Macro to implement Fetch for tuples
macro_rules! impl_fetch_tuple {
    ($($T:ident),*) => {
        #[allow(non_snake_case)]
        impl<'a, $($T: Fetch<'a>),*> Fetch<'a> for ($($T,)*) {
            type Item = ($($T::Item,)*);

            fn matches_archetype(archetype: &Archetype) -> bool {
                $($T::matches_archetype(archetype))&&*
            }

            unsafe fn fetch(archetype: &'a Archetype, entity: EntityId) -> Self::Item {
                // SAFETY: Caller ensures all safety requirements
                unsafe {
                    ($($T::fetch(archetype, entity),)*)
                }
            }
        }
    };
}

// Implement for tuples up to 8 elements
impl_fetch_tuple!(A);
impl_fetch_tuple!(A, B);
impl_fetch_tuple!(A, B, C);
impl_fetch_tuple!(A, B, C, D);
impl_fetch_tuple!(A, B, C, D, E);
impl_fetch_tuple!(A, B, C, D, E, F);
impl_fetch_tuple!(A, B, C, D, E, F, G);
impl_fetch_tuple!(A, B, C, D, E, F, G, H);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::component::Component;

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
    fn fetch_read_type_check() {
        // This test just ensures the types compile correctly
        fn _test_fetch<F: for<'a> Fetch<'a>>() {}
        _test_fetch::<FetchRead<Position>>();
    }

    #[test]
    fn fetch_write_type_check() {
        fn _test_fetch<F: for<'a> Fetch<'a>>() {}
        _test_fetch::<FetchWrite<Position>>();
    }

    #[test]
    fn fetch_optional_type_check() {
        fn _test_fetch<F: for<'a> Fetch<'a>>() {}
        _test_fetch::<FetchOptional<Position>>();
    }

    #[test]
    fn fetch_entity_type_check() {
        fn _test_fetch<F: for<'a> Fetch<'a>>() {}
        _test_fetch::<FetchEntity>();
    }

    #[test]
    fn fetch_tuple_type_check() {
        fn _test_fetch<F: for<'a> Fetch<'a>>() {}
        _test_fetch::<(FetchRead<Position>, FetchRead<Velocity>)>();
        _test_fetch::<(FetchEntity, FetchRead<Position>, FetchWrite<Velocity>)>();
    }
}

// Made with Bob
