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

//! Binary deserialization implementation.
//!
//! This module handles deserializing ECS world state from the binary format.

use super::format::{EntityData, Footer, Header, TypeRegistryEntry, calculate_checksum};
use crate::World;
use crate::persistence::PersistenceError;
use std::collections::HashMap;
use std::io::Read;

/// Binary deserializer for world state.
///
/// Reconstructs a World from the PECS binary format, validating checksums
/// and handling version compatibility.
pub struct BinaryDeserializer {
    /// Type registry mapping type IDs to names
    type_registry: HashMap<u128, TypeRegistryEntry>,
}

impl BinaryDeserializer {
    /// Create a new binary deserializer.
    pub fn new() -> Self {
        Self {
            type_registry: HashMap::new(),
        }
    }

    /// Deserialize a world from a reader.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - I/O operations fail
    /// - Format is invalid or corrupted
    /// - Version is unsupported
    /// - Checksum validation fails
    pub fn deserialize(&mut self, reader: &mut dyn Read) -> Result<World, PersistenceError> {
        // Read all data into buffer for checksum validation
        let mut buffer = Vec::new();

        // Read header
        let header =
            Header::read(reader).map_err(|e| PersistenceError::Deserialization(e.to_string()))?;

        // Store header bytes for checksum
        let mut header_buffer = Vec::new();
        header
            .write(&mut header_buffer)
            .map_err(|e| PersistenceError::Deserialization(e.to_string()))?;
        buffer.extend_from_slice(&header_buffer);

        // Read type registry
        self.type_registry.clear();
        self.type_registry
            .reserve(header.component_type_count as usize);
        for _ in 0..header.component_type_count {
            let entry = TypeRegistryEntry::read(reader)
                .map_err(|e| PersistenceError::Deserialization(e.to_string()))?;

            // Store entry bytes for checksum
            let mut entry_buffer = Vec::new();
            entry
                .write(&mut entry_buffer)
                .map_err(|e| PersistenceError::Deserialization(e.to_string()))?;
            buffer.extend_from_slice(&entry_buffer);

            self.type_registry.insert(entry.type_id, entry);
        }

        // Read entity data - pre-allocate for better performance
        let mut entities = Vec::with_capacity(header.entity_count as usize);
        for _ in 0..header.entity_count {
            let entity = EntityData::read(reader)
                .map_err(|e| PersistenceError::Deserialization(e.to_string()))?;

            // Store entity bytes for checksum
            let mut entity_buffer = Vec::new();
            entity
                .write(&mut entity_buffer)
                .map_err(|e| PersistenceError::Deserialization(e.to_string()))?;
            buffer.extend_from_slice(&entity_buffer);

            entities.push(entity);
        }

        // Read footer
        let footer =
            Footer::read(reader).map_err(|e| PersistenceError::Deserialization(e.to_string()))?;

        // Validate checksum
        let calculated_checksum = calculate_checksum(&buffer);
        if calculated_checksum != footer.checksum {
            return Err(PersistenceError::ChecksumMismatch {
                expected: footer.checksum,
                actual: calculated_checksum,
            });
        }

        // Reconstruct world
        self.reconstruct_world(header, entities)
    }

    /// Reconstruct a world from deserialized data.
    fn reconstruct_world(
        &self,
        _header: Header,
        entities: Vec<EntityData>,
    ) -> Result<World, PersistenceError> {
        let mut world = World::new();

        // Restore entities
        for entity_data in entities {
            // Convert u128 back to StableId
            let stable_id = self.u128_to_stable_id(entity_data.stable_id);

            // Allocate entity with the stable ID
            let _entity = world.entities_mut().spawn_with_id(stable_id).map_err(|e| {
                PersistenceError::Deserialization(format!("Failed to allocate entity: {:?}", e))
            })?;

            // Restore components
            for component_data in entity_data.components {
                // Look up component type in registry
                let _type_entry =
                    self.type_registry
                        .get(&component_data.type_id)
                        .ok_or_else(|| {
                            PersistenceError::Deserialization(format!(
                                "Unknown component type ID: {}",
                                component_data.type_id
                            ))
                        })?;

                // TODO: Deserialize and insert component
                // This requires a component deserialization registry
                // For now, we just validate that the type exists
            }
        }

        Ok(world)
    }

    /// Convert u128 back to StableId.
    fn u128_to_stable_id(&self, value: u128) -> crate::entity::StableId {
        crate::entity::StableId::from_u128(value)
    }
}

impl Default for BinaryDeserializer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::super::FormatFlags;
    use super::*;
    use crate::persistence::binary::BinarySerializer;
    use std::io::Cursor;

    #[test]
    fn test_deserializer_creation() {
        let deserializer = BinaryDeserializer::new();
        assert_eq!(deserializer.type_registry.len(), 0);
    }

    #[test]
    fn test_deserialize_empty_world() {
        // Serialize an empty world
        let world = World::new();
        let serializer = BinarySerializer::new(FormatFlags::NONE);

        let mut buffer = Vec::new();
        serializer.serialize(&world, &mut buffer).unwrap();

        // Deserialize it back
        let mut deserializer = BinaryDeserializer::new();
        let mut cursor = Cursor::new(buffer);
        let result = deserializer.deserialize(&mut cursor);

        assert!(result.is_ok());
        let loaded_world = result.unwrap();

        // Should have same entity count (0)
        assert_eq!(loaded_world.len(), 0);
    }

    #[test]
    fn test_deserialize_invalid_checksum() {
        // Create a buffer with invalid checksum
        let world = World::new();
        let serializer = BinarySerializer::new(FormatFlags::NONE);

        let mut buffer = Vec::new();
        serializer.serialize(&world, &mut buffer).unwrap();

        // Corrupt the checksum (last 8 bytes)
        let len = buffer.len();
        buffer[len - 1] ^= 0xFF;

        // Try to deserialize
        let mut deserializer = BinaryDeserializer::new();
        let mut cursor = Cursor::new(buffer);
        let result = deserializer.deserialize(&mut cursor);

        // Should fail with checksum error
        assert!(matches!(
            result,
            Err(PersistenceError::ChecksumMismatch { .. })
        ));
    }

    #[test]
    fn test_deserialize_invalid_magic() {
        // Create a buffer with invalid magic bytes
        let mut buffer = vec![0xFF, 0xFF, 0xFF, 0xFF]; // Invalid magic
        buffer.extend_from_slice(&1u32.to_le_bytes()); // version
        buffer.extend_from_slice(&0u32.to_le_bytes()); // flags
        buffer.extend_from_slice(&0u64.to_le_bytes()); // entity_count
        buffer.extend_from_slice(&0u32.to_le_bytes()); // type_count
        buffer.extend_from_slice(&0u64.to_le_bytes()); // checksum

        let mut deserializer = BinaryDeserializer::new();
        let mut cursor = Cursor::new(buffer);
        let result = deserializer.deserialize(&mut cursor);

        // Should fail with deserialization error
        assert!(result.is_err());
    }

    #[test]
    fn test_roundtrip_empty_world() {
        // Create and serialize
        let world = World::new();
        let serializer = BinarySerializer::new(FormatFlags::NONE);

        let mut buffer = Vec::new();
        serializer.serialize(&world, &mut buffer).unwrap();

        // Deserialize
        let mut deserializer = BinaryDeserializer::new();
        let mut cursor = Cursor::new(buffer);
        let loaded_world = deserializer.deserialize(&mut cursor).unwrap();

        // Verify
        assert_eq!(world.len(), loaded_world.len());
    }
}
