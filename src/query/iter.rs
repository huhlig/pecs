//! Query iteration implementations.
//!
//! This module provides iterators for traversing query results across
//! multiple archetypes efficiently.

use super::{Fetch, Filter};
use crate::component::archetype::ArchetypeManager;
use crate::entity::EntityId;
use std::marker::PhantomData;

/// An iterator over query results.
///
/// This iterator traverses all archetypes that match the query's fetch
/// requirements and filters, yielding items for each matching entity.
pub struct QueryIter<'w, F, Fil = ()> {
    /// Reference to the archetype manager
    archetype_manager: &'w ArchetypeManager,

    /// Current archetype index
    archetype_index: usize,

    /// Current entity index within the archetype
    entity_index: usize,

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
            _phantom: PhantomData,
        }
    }

    /// Resets the iterator to the beginning.
    pub fn reset(&mut self) {
        self.archetype_index = 0;
        self.entity_index = 0;
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
            // Get current archetype
            let archetype_id = crate::component::archetype::ArchetypeId::new(self.archetype_index);
            let archetype = self.archetype_manager.get_archetype(archetype_id)?;

            // Check if this archetype matches our fetch requirements
            if !F::matches_archetype(archetype) {
                self.archetype_index += 1;
                self.entity_index = 0;
                continue;
            }

            // Try to get the next entity in this archetype
            if let Some(entity) = archetype.get_entity(self.entity_index) {
                self.entity_index += 1;

                // Check if the entity passes the filter
                if !Fil::matches(archetype, entity) {
                    continue;
                }

                // Fetch the data for this entity
                // SAFETY: We've verified the archetype matches and the entity exists
                let item = unsafe { F::fetch(archetype, entity) };
                return Some(item);
            }

            // Move to next archetype
            self.archetype_index += 1;
            self.entity_index = 0;
        }
    }
}

// Note: Parallel query iteration will be added in a future update
// when the `parallel` feature is implemented.

/// A query iterator that also yields the entity ID.
///
/// This is a convenience wrapper that includes the entity ID in the results.
pub struct QueryIterWithEntity<'w, F, Fil = ()> {
    inner: QueryIter<'w, F, Fil>,
}

impl<'w, F, Fil> QueryIterWithEntity<'w, F, Fil> {
    /// Creates a new query iterator with entity IDs.
    pub fn new(archetype_manager: &'w ArchetypeManager) -> Self {
        Self {
            inner: QueryIter::new(archetype_manager),
        }
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
            let archetype_id =
                crate::component::archetype::ArchetypeId::new(self.inner.archetype_index);
            let archetype = self.inner.archetype_manager.get_archetype(archetype_id)?;

            if !F::matches_archetype(archetype) {
                self.inner.archetype_index += 1;
                self.inner.entity_index = 0;
                continue;
            }

            if let Some(entity) = archetype.get_entity(self.inner.entity_index) {
                self.inner.entity_index += 1;

                if !Fil::matches(archetype, entity) {
                    continue;
                }

                // SAFETY: We've verified the archetype matches and the entity exists
                let item = unsafe { F::fetch(archetype, entity) };
                return Some((entity, item));
            }

            self.inner.archetype_index += 1;
            self.inner.entity_index = 0;
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

// Made with Bob
