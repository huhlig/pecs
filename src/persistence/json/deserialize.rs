//! JSON deserialization implementation.

use crate::World;
use crate::entity::StableId;
use crate::persistence::{PersistenceError, Result};
use serde::Deserialize;
use std::io::Read;

/// JSON format for world deserialization.
#[derive(Debug, Deserialize)]
struct JsonWorld {
    /// Format version
    version: u32,
    /// Timestamp when saved
    #[serde(default)]
    timestamp: String,
    /// Number of entities
    entity_count: usize,
    /// Component type information (optional)
    #[serde(default)]
    types: Option<Vec<TypeInfo>>,
    /// Entity data
    entities: Vec<EntityData>,
}

/// Component type information.
#[derive(Debug, Deserialize)]
struct TypeInfo {
    /// Type name
    name: String,
    /// Type version
    version: u32,
}

/// Entity data in JSON format.
#[derive(Debug, Deserialize)]
struct EntityData {
    /// Stable ID as string (UUID format)
    id: String,
    /// Component data (placeholder - will be empty for now)
    #[serde(default)]
    components: serde_json::Map<String, serde_json::Value>,
}

/// Deserialize a world from JSON format.
///
/// # Arguments
///
/// * `reader` - The reader to deserialize from
///
/// # Errors
///
/// Returns an error if deserialization fails or the format is invalid.
pub(super) fn deserialize(reader: &mut dyn Read) -> Result<World> {
    // Read all data from reader
    let mut json_data = String::new();
    reader
        .read_to_string(&mut json_data)
        .map_err(PersistenceError::Io)?;

    // Parse JSON
    let json_world: JsonWorld = serde_json::from_str(&json_data)
        .map_err(|e| PersistenceError::Deserialization(e.to_string()))?;

    // Validate version
    if json_world.version != 1 {
        return Err(PersistenceError::VersionMismatch {
            found: json_world.version,
            expected: 1,
        });
    }

    // Validate entity count
    if json_world.entities.len() != json_world.entity_count {
        return Err(PersistenceError::Deserialization(format!(
            "Entity count mismatch: expected {}, got {}",
            json_world.entity_count,
            json_world.entities.len()
        )));
    }

    // Create new world
    let mut world = World::new();

    // Restore entities
    for entity_data in json_world.entities {
        // Parse stable ID
        let stable_id = parse_stable_id(&entity_data.id)?;

        // Spawn entity with stable ID
        let _entity = world.entities_mut().spawn_with_id(stable_id).map_err(|e| {
            PersistenceError::Deserialization(format!("Failed to allocate entity: {:?}", e))
        })?;

        // TODO: Restore components when component serialization is implemented
        // For now, we just create empty entities
    }

    Ok(world)
}

/// Parse a stable ID from string format.
///
/// The string should be in UUID format (e.g., "550e8400-e29b-41d4-a716-446655440000").
fn parse_stable_id(id_str: &str) -> Result<StableId> {
    // Remove hyphens and parse as hex
    let hex_str = id_str.replace('-', "");

    if hex_str.len() != 32 {
        return Err(PersistenceError::Deserialization(format!(
            "Invalid stable ID format: {}",
            id_str
        )));
    }

    // Parse high and low parts
    let high_str = &hex_str[0..16];
    let low_str = &hex_str[16..32];

    let high = u64::from_str_radix(high_str, 16)
        .map_err(|e| PersistenceError::Deserialization(format!("Invalid stable ID high: {}", e)))?;

    let low = u64::from_str_radix(low_str, 16)
        .map_err(|e| PersistenceError::Deserialization(format!("Invalid stable ID low: {}", e)))?;

    Ok(StableId::from_u128(((high as u128) << 64) | (low as u128)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_deserialize_empty_world() {
        let json = r#"{
            "version": 1,
            "timestamp": "2026-02-13T00:00:00Z",
            "entity_count": 0,
            "entities": []
        }"#;

        let mut cursor = Cursor::new(json.as_bytes());
        let world = deserialize(&mut cursor).unwrap();

        assert_eq!(world.len(), 0);
    }

    #[test]
    fn test_deserialize_with_entities() {
        let json = r#"{
            "version": 1,
            "timestamp": "2026-02-13T00:00:00Z",
            "entity_count": 2,
            "entities": [
                {
                    "id": "550e8400-e29b-41d4-a716-446655440000",
                    "components": {}
                },
                {
                    "id": "550e8400-e29b-41d4-a716-446655440001",
                    "components": {}
                }
            ]
        }"#;

        let mut cursor = Cursor::new(json.as_bytes());
        let world = deserialize(&mut cursor).unwrap();

        assert_eq!(world.len(), 2);
    }

    #[test]
    fn test_deserialize_invalid_version() {
        let json = r#"{
            "version": 999,
            "timestamp": "2026-02-13T00:00:00Z",
            "entity_count": 0,
            "entities": []
        }"#;

        let mut cursor = Cursor::new(json.as_bytes());
        let result = deserialize(&mut cursor);

        assert!(result.is_err());
        match result {
            Err(PersistenceError::VersionMismatch { found, expected }) => {
                assert_eq!(found, 999);
                assert_eq!(expected, 1);
            }
            _ => panic!("Expected VersionMismatch error"),
        }
    }

    #[test]
    fn test_deserialize_entity_count_mismatch() {
        let json = r#"{
            "version": 1,
            "timestamp": "2026-02-13T00:00:00Z",
            "entity_count": 5,
            "entities": [
                {
                    "id": "550e8400-e29b-41d4-a716-446655440000",
                    "components": {}
                }
            ]
        }"#;

        let mut cursor = Cursor::new(json.as_bytes());
        let result = deserialize(&mut cursor);

        assert!(result.is_err());
    }

    #[test]
    fn test_deserialize_invalid_json() {
        let json = "not valid json";

        let mut cursor = Cursor::new(json.as_bytes());
        let result = deserialize(&mut cursor);

        assert!(result.is_err());
    }

    #[test]
    fn test_parse_stable_id() {
        let id_str = "550e8400-e29b-41d4-a716-446655440000";
        let stable_id = parse_stable_id(id_str).unwrap();

        // Verify it's a valid stable ID
        assert_ne!(stable_id.as_u128(), 0);
    }

    #[test]
    fn test_parse_stable_id_invalid_format() {
        let id_str = "invalid-id";
        let result = parse_stable_id(id_str);

        assert!(result.is_err());
    }

    #[test]
    fn test_parse_stable_id_invalid_hex() {
        let id_str = "gggggggg-gggg-gggg-gggg-gggggggggggg";
        let result = parse_stable_id(id_str);

        assert!(result.is_err());
    }

    #[test]
    fn test_deserialize_with_schema() {
        let json = r#"{
            "version": 1,
            "timestamp": "2026-02-13T00:00:00Z",
            "entity_count": 0,
            "types": [
                {
                    "name": "Position",
                    "version": 1
                }
            ],
            "entities": []
        }"#;

        let mut cursor = Cursor::new(json.as_bytes());
        let world = deserialize(&mut cursor).unwrap();

        assert_eq!(world.len(), 0);
    }
}

// Made with Bob
