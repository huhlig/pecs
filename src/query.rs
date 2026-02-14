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

//! Query system for accessing entities and components.
//!
//! This module provides a type-safe, ergonomic interface for querying entities
//! and their components. Queries are built using the type system to ensure
//! compile-time safety and optimal performance.
//!
//! # Architecture
//!
//! The query system consists of several key traits:
//! - [`Query`]: Defines what data to fetch from the world
//! - [`Fetch`]: Handles the actual component fetching
//! - [`Filter`]: Filters which entities to include
//!
//! # Examples
//!
//! ```ignore
//! // Query for entities with Position and Velocity components
//! for (entity, pos, vel) in world.query::<(Entity, &mut Position, &Velocity)>() {
//!     pos.x += vel.x;
//!     pos.y += vel.y;
//! }
//!
//! // Query with filters
//! for pos in world.query::<&Position>().with::<Velocity>() {
//!     // Only entities that have both Position and Velocity
//! }
//! ```

pub mod fetch;
pub mod filter;
pub mod iter;
mod query_impl;

use crate::entity::EntityId;
use std::marker::PhantomData;

/// A query that fetches data from the world.
///
/// Queries are type-safe and composable, allowing you to specify exactly
/// what data you need and how to access it.
///
/// # Type Parameters
///
/// * `F` - The fetch type that determines what data to retrieve
pub trait Query {
    /// The type of data fetched by this query.
    type Item<'a>;

    /// The fetch implementation for this query.
    type Fetch: for<'a> Fetch<'a>;

    /// The filter for this query (defaults to no filter).
    type Filter: for<'a> Filter<'a>;
}

/// Trait for fetching component data from archetypes.
///
/// This trait is implemented for various component access patterns,
/// such as `&T`, `&mut T`, and tuples of these.
pub trait Fetch<'a> {
    /// The item type returned by this fetch.
    type Item;

    /// Checks if this fetch can access the given archetype.
    fn matches_archetype(archetype: &crate::component::archetype::Archetype) -> bool;

    /// Fetches data for a specific entity.
    ///
    /// # Safety
    ///
    /// The caller must ensure that:
    /// - The entity exists in the archetype
    /// - The archetype matches this fetch (checked by `matches_archetype`)
    /// - Mutable access is exclusive
    unsafe fn fetch(
        archetype: &'a crate::component::archetype::Archetype,
        entity: EntityId,
    ) -> Self::Item;
}

/// Trait for filtering which entities to include in a query.
///
/// Filters allow you to narrow down query results based on component
/// presence or custom predicates.
pub trait Filter<'a> {
    /// Checks if an entity passes this filter.
    fn matches(archetype: &crate::component::archetype::Archetype, entity: EntityId) -> bool;
}

/// A query builder that allows composing queries with filters.
///
/// # Examples
///
/// ```ignore
/// let query = QueryBuilder::new()
///     .with::<Position>()
///     .without::<Dead>()
///     .build();
/// ```
pub struct QueryBuilder<F, Fil = ()> {
    _phantom: PhantomData<(F, Fil)>,
}

impl<F> QueryBuilder<F, ()> {
    /// Creates a new query builder.
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<F> Default for QueryBuilder<F, ()> {
    fn default() -> Self {
        Self::new()
    }
}

/// Query state that tracks iteration progress.
///
/// This is used internally by the query iterator to maintain state
/// across archetype boundaries.
pub struct QueryState {
    /// Current archetype index
    archetype_index: usize,
    /// Current entity index within the archetype
    entity_index: usize,
}

impl QueryState {
    /// Creates a new query state.
    pub fn new() -> Self {
        Self {
            archetype_index: 0,
            entity_index: 0,
        }
    }

    /// Resets the query state to the beginning.
    pub fn reset(&mut self) {
        self.archetype_index = 0;
        self.entity_index = 0;
    }
}

impl Default for QueryState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_state_creation() {
        let state = QueryState::new();
        assert_eq!(state.archetype_index, 0);
        assert_eq!(state.entity_index, 0);
    }

    #[test]
    fn query_state_reset() {
        let mut state = QueryState::new();
        state.archetype_index = 5;
        state.entity_index = 10;

        state.reset();
        assert_eq!(state.archetype_index, 0);
        assert_eq!(state.entity_index, 0);
    }
}
