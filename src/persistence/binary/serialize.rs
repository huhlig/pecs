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

//! Binary serialization implementation.
//!
//! This module handles serializing ECS world state into the binary format.

use super::format::{
    EntityData, Footer, FormatFlags, Header, TypeRegistryEntry, calculate_checksum,
};
use crate::World;
use crate::persistence::{PersistenceError, WorldMetadata};
use std::any::TypeId;
use std::io::Write;

/// Binary serializer for world state.
///
/// Converts a World into the PECS binary format with proper type registry,
/// entity data, and checksums.
pub struct BinarySerializer {
    /// Format flags
    flags: FormatFlags,
}

impl BinarySerializer {
    /// Create a new binary serializer.
    pub fn new(flags: FormatFlags) -> Self {
        Self { flags }
    }

    /// Serialize a world to a writer.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - I/O operations fail
    /// - Component serialization fails
    /// - Data is invalid
    pub fn serialize(&self, world: &World, writer: &mut dyn Write) -> Result<(), PersistenceError> {
        // Get world metadata
        let metadata = world.metadata();

        // Build type registry from metadata
        let type_registry = self.build_type_registry(metadata)?;

        // Collect entity data
        let entity_data = self.collect_entity_data(world)?;

        // Pre-allocate buffer with estimated size to reduce allocations
        // Estimate: header + type registry + entity data
        let estimated_size = Header::HEADER_SIZE
            + type_registry.len() * 64  // Rough estimate per type entry
            + entity_data.len() * 32    // Rough estimate per entity
            + Footer::FOOTER_SIZE;
        let mut buffer = Vec::with_capacity(estimated_size);

        // Write header
        let header = Header {
            version: super::FORMAT_VERSION,
            flags: self.flags,
            entity_count: entity_data.len() as u64,
            component_type_count: type_registry.len() as u32,
        };
        header
            .write(&mut buffer)
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;

        // Write type registry
        for entry in &type_registry {
            entry
                .write(&mut buffer)
                .map_err(|e| PersistenceError::Serialization(e.to_string()))?;
        }

        // Write entity data
        for entity in &entity_data {
            entity
                .write(&mut buffer)
                .map_err(|e| PersistenceError::Serialization(e.to_string()))?;
        }

        // Calculate checksum of all data
        let checksum = calculate_checksum(&buffer);

        // Write everything to the actual writer
        writer.write_all(&buffer).map_err(PersistenceError::Io)?;

        // Write footer with checksum
        let footer = Footer::new(checksum);
        footer
            .write(writer)
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;

        Ok(())
    }

    /// Build type registry from world metadata.
    fn build_type_registry(
        &self,
        metadata: &WorldMetadata,
    ) -> Result<Vec<TypeRegistryEntry>, PersistenceError> {
        let mut registry = Vec::new();

        for type_info in &metadata.component_types {
            // Convert TypeId to u128 for serialization
            // Note: TypeId doesn't have a stable representation, so we use a hash
            let type_id_hash = self.type_id_to_u128(type_info.type_id);

            let entry = TypeRegistryEntry::new(
                type_id_hash,
                type_info.type_name.clone(),
                type_info.version,
            );
            registry.push(entry);
        }

        Ok(registry)
    }

    /// Collect entity data from the world.
    fn collect_entity_data(&self, world: &World) -> Result<Vec<EntityData>, PersistenceError> {
        let mut entities = Vec::new();

        // Iterate over all entities with their stable IDs
        for (_entity, stable_id) in world.iter_entities() {
            let stable_id_u128 = self.stable_id_to_u128(stable_id);
            let entity_data = EntityData::new(stable_id_u128);

            // Get all components for this entity
            // Note: This is a placeholder - actual implementation will need
            // to iterate through archetypes and serialize components
            // For now, we'll collect what we can from the world

            // TODO: Implement component iteration and serialization
            // This requires access to the archetype system to get component data

            entities.push(entity_data);
        }

        Ok(entities)
    }

    /// Convert TypeId to u128 for serialization.
    ///
    /// Note: TypeId doesn't have a stable representation across compilations,
    /// so we use the type name hash instead for persistence.
    fn type_id_to_u128(&self, type_id: TypeId) -> u128 {
        // Use a simple hash of the TypeId's debug representation
        // In a real implementation, we'd use the type name from metadata
        let hash = self.hash_type_id(type_id);
        hash as u128
    }

    /// Convert StableId to u128.
    fn stable_id_to_u128(&self, stable_id: crate::entity::StableId) -> u128 {
        stable_id.as_u128()
    }

    /// Hash a TypeId for serialization.
    fn hash_type_id(&self, type_id: TypeId) -> u64 {
        // Simple hash implementation
        // In production, we'd use a proper hash function
        let debug_str = format!("{:?}", type_id);
        let mut hash: u64 = 0;
        for byte in debug_str.bytes() {
            hash = hash.wrapping_mul(31).wrapping_add(byte as u64);
        }
        hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serializer_creation() {
        let serializer = BinarySerializer::new(FormatFlags::NONE);
        assert_eq!(serializer.flags.bits(), 0);
    }

    #[test]
    fn test_serialize_empty_world() {
        let world = World::new();
        let serializer = BinarySerializer::new(FormatFlags::NONE);

        let mut buffer = Vec::new();
        let result = serializer.serialize(&world, &mut buffer);

        // Should succeed even with empty world
        assert!(result.is_ok());

        // Buffer should contain at least header + footer
        assert!(buffer.len() >= Header::HEADER_SIZE + Footer::FOOTER_SIZE);
    }

    #[test]
    fn test_type_id_to_u128_consistency() {
        let serializer = BinarySerializer::new(FormatFlags::NONE);
        let type_id = TypeId::of::<i32>();

        let hash1 = serializer.type_id_to_u128(type_id);
        let hash2 = serializer.type_id_to_u128(type_id);

        // Same TypeId should produce same hash
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_different_type_ids_produce_different_hashes() {
        let serializer = BinarySerializer::new(FormatFlags::NONE);

        let hash1 = serializer.type_id_to_u128(TypeId::of::<i32>());
        let hash2 = serializer.type_id_to_u128(TypeId::of::<f32>());

        // Different TypeIds should produce different hashes
        assert_ne!(hash1, hash2);
    }
}
