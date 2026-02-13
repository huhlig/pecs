//! Binary serialization format for PECS persistence.
//!
//! This module provides an efficient binary format for serializing and deserializing
//! ECS world state. The format is designed for:
//!
//! - **Performance**: Fast serialization/deserialization (< 1ms per 1000 entities)
//! - **Compactness**: Minimal file size overhead
//! - **Compatibility**: Version detection and migration support
//! - **Integrity**: Checksums for data validation
//!
//! # Format Overview
//!
//! The binary format consists of:
//! 1. Header with magic bytes, version, and metadata
//! 2. Type registry mapping component types to IDs
//! 3. Entity data with components
//! 4. Footer with checksum
//!
//! # Example
//!
//! ```rust,ignore
//! use pecs::persistence::binary::BinaryPlugin;
//! use pecs::World;
//!
//! let world = World::new();
//! // ... populate world ...
//!
//! // Save to binary format
//! let plugin = BinaryPlugin::new();
//! world.save_with("world.pecs", &plugin)?;
//!
//! // Load from binary format
//! let loaded_world = World::load_with("world.pecs", &plugin)?;
//! ```

mod deserialize;
pub mod format;
mod serialize;

pub use deserialize::BinaryDeserializer;
pub use format::{
    ComponentData, EntityData, FORMAT_VERSION, Footer, FormatFlags, Header, MAGIC_BYTES,
    MIN_SUPPORTED_VERSION, TypeRegistryEntry, calculate_checksum,
};
pub use serialize::BinarySerializer;

use crate::World;
use crate::persistence::{PersistenceError, PersistencePlugin};
use std::io::{Read, Write};

/// Binary format persistence plugin.
///
/// This plugin implements the PECS binary format for efficient world serialization.
/// It provides fast, compact serialization with data integrity checks.
///
/// # Features
///
/// - Fast serialization (< 1ms per 1000 entities target)
/// - Compact binary format
/// - Data integrity via checksums
/// - Version detection for migrations
/// - Optional compression support
///
/// # Example
///
/// ```rust,ignore
/// use pecs::persistence::binary::BinaryPlugin;
/// use pecs::World;
///
/// let plugin = BinaryPlugin::new();
/// let world = World::new();
///
/// // Save
/// world.save_with("world.pecs", &plugin)?;
///
/// // Load
/// let loaded = World::load_with("world.pecs", &plugin)?;
/// ```
#[derive(Debug, Clone)]
pub struct BinaryPlugin {
    /// Format flags for optional features
    flags: FormatFlags,
}

impl BinaryPlugin {
    /// Create a new binary plugin with default settings.
    pub fn new() -> Self {
        Self {
            flags: FormatFlags::NONE,
        }
    }

    /// Create a binary plugin with compression enabled.
    ///
    /// Note: Compression support is not yet implemented.
    pub fn with_compression(mut self) -> Self {
        self.flags.set(FormatFlags::COMPRESSED_ZSTD);
        self
    }

    /// Create a binary plugin for delta/incremental saves.
    pub fn with_delta(mut self) -> Self {
        self.flags.set(FormatFlags::DELTA);
        self
    }

    /// Get the format flags.
    pub fn flags(&self) -> FormatFlags {
        self.flags
    }
}

impl Default for BinaryPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl PersistencePlugin for BinaryPlugin {
    fn save(&self, world: &World, writer: &mut dyn Write) -> Result<(), PersistenceError> {
        let serializer = BinarySerializer::new(self.flags);
        serializer.serialize(world, writer)
    }

    fn load(&self, reader: &mut dyn Read) -> Result<World, PersistenceError> {
        let mut deserializer = BinaryDeserializer::new();
        deserializer.deserialize(reader)
    }

    fn format_name(&self) -> &str {
        "binary"
    }

    fn format_version(&self) -> u32 {
        FORMAT_VERSION
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary_plugin_creation() {
        let plugin = BinaryPlugin::new();
        assert_eq!(plugin.format_name(), "binary");
        assert_eq!(plugin.format_version(), FORMAT_VERSION);
        assert!(!plugin.flags().contains(FormatFlags::COMPRESSED_ZSTD));
    }

    #[test]
    fn test_binary_plugin_with_compression() {
        let plugin = BinaryPlugin::new().with_compression();
        assert!(plugin.flags().contains(FormatFlags::COMPRESSED_ZSTD));
    }

    #[test]
    fn test_binary_plugin_with_delta() {
        let plugin = BinaryPlugin::new().with_delta();
        assert!(plugin.flags().contains(FormatFlags::DELTA));
    }

    #[test]
    fn test_binary_plugin_default() {
        let plugin = BinaryPlugin::default();
        assert_eq!(plugin.format_name(), "binary");
    }
}

// Made with Bob
