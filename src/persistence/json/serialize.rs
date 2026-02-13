//! JSON serialization implementation.

use crate::World;
use crate::persistence::{PersistenceError, Result};
use serde::Serialize;
use std::io::Write;

/// JSON format for world serialization.
#[derive(Debug, Serialize)]
struct JsonWorld {
    /// Format version
    version: u32,
    /// Timestamp when saved
    timestamp: String,
    /// Number of entities
    entity_count: usize,
    /// Component type information (if schema is included)
    #[serde(skip_serializing_if = "Option::is_none")]
    types: Option<Vec<TypeInfo>>,
    /// Entity data
    entities: Vec<EntityData>,
}

/// Component type information.
#[derive(Debug, Serialize)]
struct TypeInfo {
    /// Type name
    name: String,
    /// Type version
    version: u32,
}

/// Entity data in JSON format.
#[derive(Debug, Serialize)]
struct EntityData {
    /// Stable ID as string (UUID format)
    id: String,
    /// Component data (placeholder - will be empty for now)
    components: serde_json::Map<String, serde_json::Value>,
}

/// Serialize a world to JSON format.
///
/// # Arguments
///
/// * `world` - The world to serialize
/// * `writer` - The writer to serialize to
/// * `pretty` - Whether to pretty-print the JSON
/// * `include_schema` - Whether to include schema information
///
/// # Errors
///
/// Returns an error if serialization fails.
pub(super) fn serialize(
    world: &World,
    writer: &mut dyn Write,
    pretty: bool,
    include_schema: bool,
) -> Result<()> {
    // Get current timestamp
    let timestamp = chrono::Utc::now().to_rfc3339();

    // Collect entity data
    let mut entities = Vec::new();
    for (_entity, stable_id) in world.iter_entities() {
        let id = format!("{}", stable_id);

        // For now, we don't have component data serialization
        // This will be a placeholder until we implement component serialization
        let components = serde_json::Map::new();

        entities.push(EntityData { id, components });
    }

    // Build type information if requested
    let types = if include_schema {
        // For now, return empty type list
        // This will be populated when we implement component type tracking
        Some(Vec::new())
    } else {
        None
    };

    // Create JSON world structure
    let json_world = JsonWorld {
        version: 1,
        timestamp,
        entity_count: entities.len(),
        types,
        entities,
    };

    // Serialize to JSON
    let json = if pretty {
        serde_json::to_string_pretty(&json_world)
    } else {
        serde_json::to_string(&json_world)
    }
    .map_err(|e| PersistenceError::Serialization(e.to_string()))?;

    // Write to output
    writer
        .write_all(json.as_bytes())
        .map_err(PersistenceError::Io)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_empty_world() {
        let world = World::new();
        let mut buffer = Vec::new();

        serialize(&world, &mut buffer, false, false).unwrap();

        let json_str = String::from_utf8(buffer).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(parsed["version"], 1);
        assert_eq!(parsed["entity_count"], 0);
        assert!(parsed["entities"].is_array());
    }

    #[test]
    fn test_serialize_with_entities() {
        let mut world = World::new();
        world.spawn();
        world.spawn();

        let mut buffer = Vec::new();
        serialize(&world, &mut buffer, false, false).unwrap();

        let json_str = String::from_utf8(buffer).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(parsed["entity_count"], 2);
        assert_eq!(parsed["entities"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_serialize_pretty() {
        let world = World::new();
        let mut buffer = Vec::new();

        serialize(&world, &mut buffer, true, false).unwrap();

        let json_str = String::from_utf8(buffer).unwrap();
        // Pretty-printed JSON should contain newlines
        assert!(json_str.contains('\n'));
    }

    #[test]
    fn test_serialize_with_schema() {
        let world = World::new();
        let mut buffer = Vec::new();

        serialize(&world, &mut buffer, false, true).unwrap();

        let json_str = String::from_utf8(buffer).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        // Schema should be present
        assert!(parsed.get("types").is_some());
    }

    #[test]
    fn test_serialize_without_schema() {
        let world = World::new();
        let mut buffer = Vec::new();

        serialize(&world, &mut buffer, false, false).unwrap();

        let json_str = String::from_utf8(buffer).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        // Schema should not be present
        assert!(parsed.get("types").is_none());
    }

    #[test]
    fn test_serialize_entity_ids() {
        let mut world = World::new();
        let _entity1 = world.spawn().id();
        let _entity2 = world.spawn().id();

        let mut buffer = Vec::new();
        serialize(&world, &mut buffer, false, false).unwrap();

        let json_str = String::from_utf8(buffer).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        let entities = parsed["entities"].as_array().unwrap();
        assert_eq!(entities.len(), 2);

        // Each entity should have an ID
        for entity in entities {
            assert!(entity["id"].is_string());
            let id_str = entity["id"].as_str().unwrap();
            assert!(!id_str.is_empty());
        }
    }
}

// Made with Bob
