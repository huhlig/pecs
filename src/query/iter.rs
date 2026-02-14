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

//! Query iteration implementations.
//!
//! This module provides iterators for traversing query results across
//! multiple archetypes efficiently.

use super::{Fetch, Filter};
use crate::component::archetype::{Archetype, ArchetypeManager};
use crate::entity::EntityId;
use std::marker::PhantomData;

/// An iterator over query results.
///
/// This iterator traverses all archetypes that match the query's fetch
/// requirements and filters, yielding items for each matching entity.
///
/// # Performance Optimizations
///
/// - Caches current archetype reference to avoid repeated lookups
/// - Skips archetype matching check once archetype is validated
/// - Uses direct entity slice access for better cache locality
pub struct QueryIter<'w, F, Fil = ()> {
    /// Reference to the archetype manager
    archetype_manager: &'w ArchetypeManager,

    /// Current archetype index
    archetype_index: usize,

    /// Current entity index within the archetype
    entity_index: usize,

    /// Cached reference to current archetype (avoids repeated lookups)
    current_archetype: Option<&'w Archetype>,

    /// Cached entity slice from current archetype (better cache locality)
    current_entities: &'w [EntityId],

    /// Phantom data for fetch and filter types
    _phantom: PhantomData<(F, Fil)>,
}

impl<'w, F, Fil> QueryIter<'w, F, Fil> {
    /// Creates a new query iterator.
    ///
    /// # Arguments
    ///
    /// * `archetype_manager` - The archetype manager to iterate over
    pub fn new(archetype_manager: &'w ArchetypeManager) -> Self {
        Self {
            archetype_manager,
            archetype_index: 0,
            entity_index: 0,
            current_archetype: None,
            current_entities: &[],
            _phantom: PhantomData,
        }
    }

    /// Resets the iterator to the beginning.
    pub fn reset(&mut self) {
        self.archetype_index = 0;
        self.entity_index = 0;
        self.current_archetype = None;
        self.current_entities = &[];
    }
}

impl<'w, F, Fil> Iterator for QueryIter<'w, F, Fil>
where
    F: for<'a> Fetch<'a>,
    Fil: for<'a> Filter<'a>,
{
    type Item = <F as Fetch<'w>>::Item;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // Fast path: iterate within current archetype
            if self.entity_index < self.current_entities.len() {
                let entity = self.current_entities[self.entity_index];
                self.entity_index += 1;

                // SAFETY: We've verified the archetype matches and the entity exists
                // current_archetype is guaranteed to be Some when current_entities is non-empty
                let archetype = unsafe { self.current_archetype.unwrap_unchecked() };

                // Check if the entity passes the filter
                if !Fil::matches(archetype, entity) {
                    continue;
                }

                // Fetch the data for this entity
                let item = unsafe { F::fetch(archetype, entity) };
                return Some(item);
            }

            // Slow path: move to next matching archetype
            self.archetype_index += 1;
            self.entity_index = 0;

            // Find next matching archetype
            loop {
                let archetype_id =
                    crate::component::archetype::ArchetypeId::new(self.archetype_index);
                let archetype = self.archetype_manager.get_archetype(archetype_id)?;

                // Check if this archetype matches our fetch requirements
                if F::matches_archetype(archetype) {
                    // Cache the archetype and its entities for fast iteration
                    self.current_archetype = Some(archetype);
                    self.current_entities = archetype.entities();
                    break;
                }

                // Skip non-matching archetype
                self.archetype_index += 1;
            }
        }
    }
}

// Note: Parallel query iteration will be added in a future update
// when the `parallel` feature is implemented.

/// A query iterator that also yields the entity ID.
///
/// This is a convenience wrapper that includes the entity ID in the results.
///
/// # Performance Optimizations
///
/// - Uses same caching strategy as QueryIter
/// - Avoids repeated archetype lookups
/// - Direct entity slice access for better cache locality
pub struct QueryIterWithEntity<'w, F, Fil = ()> {
    /// Reference to the archetype manager
    archetype_manager: &'w ArchetypeManager,

    /// Current archetype index
    archetype_index: usize,

    /// Current entity index within the archetype
    entity_index: usize,

    /// Cached reference to current archetype
    current_archetype: Option<&'w Archetype>,

    /// Cached entity slice from current archetype
    current_entities: &'w [EntityId],

    /// Phantom data for fetch and filter types
    _phantom: PhantomData<(F, Fil)>,
}

impl<'w, F, Fil> QueryIterWithEntity<'w, F, Fil> {
    /// Creates a new query iterator with entity IDs.
    pub fn new(archetype_manager: &'w ArchetypeManager) -> Self {
        Self {
            archetype_manager,
            archetype_index: 0,
            entity_index: 0,
            current_archetype: None,
            current_entities: &[],
            _phantom: PhantomData,
        }
    }

    /// Resets the iterator to the beginning.
    pub fn reset(&mut self) {
        self.archetype_index = 0;
        self.entity_index = 0;
        self.current_archetype = None;
        self.current_entities = &[];
    }
}

impl<'w, F, Fil> Iterator for QueryIterWithEntity<'w, F, Fil>
where
    F: for<'a> Fetch<'a>,
    Fil: for<'a> Filter<'a>,
{
    type Item = (EntityId, <F as Fetch<'w>>::Item);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // Fast path: iterate within current archetype
            if self.entity_index < self.current_entities.len() {
                let entity = self.current_entities[self.entity_index];
                self.entity_index += 1;

                // SAFETY: current_archetype is guaranteed to be Some when current_entities is non-empty
                let archetype = unsafe { self.current_archetype.unwrap_unchecked() };

                // Check if the entity passes the filter
                if !Fil::matches(archetype, entity) {
                    continue;
                }

                // Fetch the data for this entity
                // SAFETY: We've verified the archetype matches and the entity exists
                let item = unsafe { F::fetch(archetype, entity) };
                return Some((entity, item));
            }

            // Slow path: move to next matching archetype
            self.archetype_index += 1;
            self.entity_index = 0;

            // Find next matching archetype
            loop {
                let archetype_id =
                    crate::component::archetype::ArchetypeId::new(self.archetype_index);
                let archetype = self.archetype_manager.get_archetype(archetype_id)?;

                // Check if this archetype matches our fetch requirements
                if F::matches_archetype(archetype) {
                    // Cache the archetype and its entities for fast iteration
                    self.current_archetype = Some(archetype);
                    self.current_entities = archetype.entities();
                    break;
                }

                // Skip non-matching archetype
                self.archetype_index += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_iter_creation() {
        use crate::component::archetype::ArchetypeManager;

        let manager = ArchetypeManager::new();
        let _iter: QueryIter<()> = QueryIter::new(&manager);
    }

    #[test]
    fn query_iter_reset() {
        use crate::component::archetype::ArchetypeManager;

        let manager = ArchetypeManager::new();
        let mut iter: QueryIter<()> = QueryIter::new(&manager);

        iter.archetype_index = 5;
        iter.entity_index = 10;

        iter.reset();
        assert_eq!(iter.archetype_index, 0);
        assert_eq!(iter.entity_index, 0);
    }

    #[test]
    fn query_iter_with_entity_creation() {
        use crate::component::archetype::ArchetypeManager;

        let manager = ArchetypeManager::new();
        let _iter: QueryIterWithEntity<()> = QueryIterWithEntity::new(&manager);
    }
}
