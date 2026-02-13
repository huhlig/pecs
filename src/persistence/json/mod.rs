//! JSON persistence format plugin.
//!
//! This module provides a human-readable JSON format for world persistence.
//! It's useful for debugging, manual editing, and cross-platform compatibility.
//!
//! # Features
//!
//! - Human-readable format
//! - Schema validation
//! - Easy debugging and inspection
//! - Cross-platform compatibility
//!
//! # Performance
//!
//! JSON format is slower than binary format but provides better readability.
//! Use binary format for production and JSON for development/debugging.
//!
//! # Example
//!
//! ```rust,ignore
//! use pecs::persistence::JsonPlugin;
//! use pecs::World;
//!
//! let mut world = World::new();
//! let plugin = JsonPlugin::new();
//!
//! // Save to JSON
//! world.save_with("world.json", &plugin)?;
//!
//! // Load from JSON
//! let loaded = World::load_with("world.json", &plugin)?;
//! ```

mod deserialize;
mod serialize;

use crate::World;
use crate::persistence::{PersistencePlugin, Result};
use std::io::{Read, Write};

/// JSON persistence plugin.
///
/// Provides human-readable JSON serialization for world state.
/// Useful for debugging, manual editing, and cross-platform compatibility.
///
/// # Examples
///
/// ```rust,ignore
/// use pecs::persistence::{JsonPlugin, PersistenceManager};
///
/// let mut manager = PersistenceManager::new();
/// manager.register_plugin("json", Box::new(JsonPlugin::new()));
///
/// // Save with JSON format
/// manager.save_with(&world, "world.json", "json")?;
/// ```
#[derive(Debug, Clone)]
pub struct JsonPlugin {
    /// Pretty-print the JSON output
    pretty: bool,
    /// Include schema information
    include_schema: bool,
}

impl JsonPlugin {
    /// Creates a new JSON plugin with default settings.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::persistence::JsonPlugin;
    ///
    /// let plugin = JsonPlugin::new();
    /// ```
    pub fn new() -> Self {
        Self {
            pretty: true,
            include_schema: true,
        }
    }

    /// Creates a JSON plugin with compact output (no pretty-printing).
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::persistence::JsonPlugin;
    ///
    /// let plugin = JsonPlugin::compact();
    /// ```
    pub fn compact() -> Self {
        Self {
            pretty: false,
            include_schema: true,
        }
    }

    /// Sets whether to pretty-print the JSON output.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::persistence::JsonPlugin;
    ///
    /// let plugin = JsonPlugin::new().with_pretty(false);
    /// ```
    pub fn with_pretty(mut self, pretty: bool) -> Self {
        self.pretty = pretty;
        self
    }

    /// Sets whether to include schema information in the output.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::persistence::JsonPlugin;
    ///
    /// let plugin = JsonPlugin::new().with_schema(false);
    /// ```
    pub fn with_schema(mut self, include_schema: bool) -> Self {
        self.include_schema = include_schema;
        self
    }

    /// Returns whether pretty-printing is enabled.
    pub fn is_pretty(&self) -> bool {
        self.pretty
    }

    /// Returns whether schema information is included.
    pub fn includes_schema(&self) -> bool {
        self.include_schema
    }
}

impl Default for JsonPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl PersistencePlugin for JsonPlugin {
    fn save(&self, world: &World, writer: &mut dyn Write) -> Result<()> {
        serialize::serialize(world, writer, self.pretty, self.include_schema)
    }

    fn load(&self, reader: &mut dyn Read) -> Result<World> {
        deserialize::deserialize(reader)
    }

    fn format_name(&self) -> &str {
        "json"
    }

    fn format_version(&self) -> u32 {
        1
    }

    fn can_load_version(&self, version: u32) -> bool {
        // Support version 1 only for now
        version == 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_plugin_creation() {
        let plugin = JsonPlugin::new();
        assert_eq!(plugin.format_name(), "json");
        assert_eq!(plugin.format_version(), 1);
        assert!(plugin.is_pretty());
        assert!(plugin.includes_schema());
    }

    #[test]
    fn test_json_plugin_compact() {
        let plugin = JsonPlugin::compact();
        assert!(!plugin.is_pretty());
        assert!(plugin.includes_schema());
    }

    #[test]
    fn test_json_plugin_with_pretty() {
        let plugin = JsonPlugin::new().with_pretty(false);
        assert!(!plugin.is_pretty());
    }

    #[test]
    fn test_json_plugin_with_schema() {
        let plugin = JsonPlugin::new().with_schema(false);
        assert!(!plugin.includes_schema());
    }

    #[test]
    fn test_json_plugin_default() {
        let plugin = JsonPlugin::default();
        assert_eq!(plugin.format_name(), "json");
        assert!(plugin.is_pretty());
    }

    #[test]
    fn test_json_plugin_version_compatibility() {
        let plugin = JsonPlugin::new();
        assert!(plugin.can_load_version(1));
        assert!(!plugin.can_load_version(2));
        assert!(!plugin.can_load_version(0));
    }
}

// Made with Bob
