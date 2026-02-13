//! Binary format specification for PECS persistence.
//!
//! This module defines the binary format used for serializing and deserializing
//! ECS world state. The format is designed for efficiency, compactness, and
//! forward compatibility.
//!
//! # Format Structure
//!
//! ```text
//! [Header]
//! - Magic bytes: "PECS" (4 bytes)
//! - Version: u32 (4 bytes)
//! - Flags: u32 (4 bytes)
//! - Entity count: u64 (8 bytes)
//! - Component type count: u32 (4 bytes)
//!
//! [Type Registry]
//! - For each component type:
//!   - Type ID: u128 (16 bytes)
//!   - Type name length: u32 (4 bytes)
//!   - Type name: UTF-8 string
//!   - Type version: u32 (4 bytes)
//!
//! [Entity Data]
//! - For each entity:
//!   - Stable ID: u128 (16 bytes)
//!   - Component count: u32 (4 bytes)
//!   - For each component:
//!     - Type ID: u128 (16 bytes)
//!     - Data length: u32 (4 bytes)
//!     - Data: [bytes]
//!
//! [Footer]
//! - Checksum: u64 (8 bytes)
//! ```
//!
//! # Version History
//!
//! - Version 1: Initial format specification

use std::io::{self, Read, Write};

/// Magic bytes identifying a PECS binary file: "PECS"
pub const MAGIC_BYTES: [u8; 4] = *b"PECS";

/// Current binary format version
pub const FORMAT_VERSION: u32 = 1;

/// Minimum supported format version for backward compatibility
pub const MIN_SUPPORTED_VERSION: u32 = 1;

/// Format flags for optional features
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormatFlags(u32);

impl FormatFlags {
    /// No special flags
    pub const NONE: Self = Self(0);

    /// Data is compressed using zstd
    pub const COMPRESSED_ZSTD: Self = Self(1 << 0);

    /// Data is compressed using lz4
    pub const COMPRESSED_LZ4: Self = Self(1 << 1);

    /// Contains delta/incremental data only
    pub const DELTA: Self = Self(1 << 2);

    /// Contains extended metadata
    pub const EXTENDED_METADATA: Self = Self(1 << 3);

    /// Create flags from raw value
    pub const fn from_bits(bits: u32) -> Self {
        Self(bits)
    }

    /// Get raw flag bits
    pub const fn bits(&self) -> u32 {
        self.0
    }

    /// Check if a flag is set
    pub const fn contains(&self, flag: Self) -> bool {
        (self.0 & flag.0) == flag.0
    }

    /// Set a flag
    pub fn set(&mut self, flag: Self) {
        self.0 |= flag.0;
    }

    /// Clear a flag
    pub fn clear(&mut self, flag: Self) {
        self.0 &= !flag.0;
    }
}

impl Default for FormatFlags {
    fn default() -> Self {
        Self::NONE
    }
}

/// Binary format header
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    /// Format version number
    pub version: u32,

    /// Format flags
    pub flags: FormatFlags,

    /// Number of entities in the file
    pub entity_count: u64,

    /// Number of component types in the type registry
    pub component_type_count: u32,
}

impl Header {
    /// Create a new header with default values
    pub fn new(entity_count: u64, component_type_count: u32) -> Self {
        Self {
            version: FORMAT_VERSION,
            flags: FormatFlags::NONE,
            entity_count,
            component_type_count,
        }
    }

    /// Size of the header in bytes
    pub const HEADER_SIZE: usize = 4 + 4 + 4 + 8 + 4; // magic + version + flags + entity_count + type_count

    /// Write header to a writer
    pub fn write(&self, writer: &mut dyn Write) -> io::Result<()> {
        writer.write_all(&MAGIC_BYTES)?;
        writer.write_all(&self.version.to_le_bytes())?;
        writer.write_all(&self.flags.bits().to_le_bytes())?;
        writer.write_all(&self.entity_count.to_le_bytes())?;
        writer.write_all(&self.component_type_count.to_le_bytes())?;
        Ok(())
    }

    /// Read header from a reader
    pub fn read(reader: &mut dyn Read) -> io::Result<Self> {
        // Read and verify magic bytes
        let mut magic = [0u8; 4];
        reader.read_exact(&mut magic)?;
        if magic != MAGIC_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Invalid magic bytes: expected {:?}, got {:?}",
                    MAGIC_BYTES, magic
                ),
            ));
        }

        // Read version
        let mut version_bytes = [0u8; 4];
        reader.read_exact(&mut version_bytes)?;
        let version = u32::from_le_bytes(version_bytes);

        // Check version compatibility
        if version < MIN_SUPPORTED_VERSION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unsupported format version: {}", version),
            ));
        }

        // Read flags
        let mut flags_bytes = [0u8; 4];
        reader.read_exact(&mut flags_bytes)?;
        let flags = FormatFlags::from_bits(u32::from_le_bytes(flags_bytes));

        // Read entity count
        let mut entity_count_bytes = [0u8; 8];
        reader.read_exact(&mut entity_count_bytes)?;
        let entity_count = u64::from_le_bytes(entity_count_bytes);

        // Read component type count
        let mut type_count_bytes = [0u8; 4];
        reader.read_exact(&mut type_count_bytes)?;
        let component_type_count = u32::from_le_bytes(type_count_bytes);

        Ok(Self {
            version,
            flags,
            entity_count,
            component_type_count,
        })
    }
}

/// Component type information in the type registry
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeRegistryEntry {
    /// Unique type identifier (TypeId as u128)
    pub type_id: u128,

    /// Human-readable type name
    pub type_name: String,

    /// Component type version for migration support
    pub type_version: u32,
}

impl TypeRegistryEntry {
    /// Create a new type registry entry
    pub fn new(type_id: u128, type_name: String, type_version: u32) -> Self {
        Self {
            type_id,
            type_name,
            type_version,
        }
    }

    /// Write entry to a writer
    pub fn write(&self, writer: &mut dyn Write) -> io::Result<()> {
        // Write type ID
        writer.write_all(&self.type_id.to_le_bytes())?;

        // Write type name length and name
        let name_bytes = self.type_name.as_bytes();
        writer.write_all(&(name_bytes.len() as u32).to_le_bytes())?;
        writer.write_all(name_bytes)?;

        // Write type version
        writer.write_all(&self.type_version.to_le_bytes())?;

        Ok(())
    }

    /// Read entry from a reader
    pub fn read(reader: &mut dyn Read) -> io::Result<Self> {
        // Read type ID
        let mut type_id_bytes = [0u8; 16];
        reader.read_exact(&mut type_id_bytes)?;
        let type_id = u128::from_le_bytes(type_id_bytes);

        // Read type name length
        let mut name_len_bytes = [0u8; 4];
        reader.read_exact(&mut name_len_bytes)?;
        let name_len = u32::from_le_bytes(name_len_bytes) as usize;

        // Read type name
        let mut name_bytes = vec![0u8; name_len];
        reader.read_exact(&mut name_bytes)?;
        let type_name = String::from_utf8(name_bytes)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        // Read type version
        let mut version_bytes = [0u8; 4];
        reader.read_exact(&mut version_bytes)?;
        let type_version = u32::from_le_bytes(version_bytes);

        Ok(Self {
            type_id,
            type_name,
            type_version,
        })
    }
}

/// Entity data in the binary format
#[derive(Debug, Clone)]
pub struct EntityData {
    /// Stable entity ID
    pub stable_id: u128,

    /// Component data for this entity
    pub components: Vec<ComponentData>,
}

impl EntityData {
    /// Create new entity data
    pub fn new(stable_id: u128) -> Self {
        Self {
            stable_id,
            components: Vec::new(),
        }
    }

    /// Add component data
    pub fn add_component(&mut self, component: ComponentData) {
        self.components.push(component);
    }

    /// Write entity data to a writer
    pub fn write(&self, writer: &mut dyn Write) -> io::Result<()> {
        // Write stable ID
        writer.write_all(&self.stable_id.to_le_bytes())?;

        // Write component count
        writer.write_all(&(self.components.len() as u32).to_le_bytes())?;

        // Write each component
        for component in &self.components {
            component.write(writer)?;
        }

        Ok(())
    }

    /// Read entity data from a reader
    pub fn read(reader: &mut dyn Read) -> io::Result<Self> {
        // Read stable ID
        let mut stable_id_bytes = [0u8; 16];
        reader.read_exact(&mut stable_id_bytes)?;
        let stable_id = u128::from_le_bytes(stable_id_bytes);

        // Read component count
        let mut count_bytes = [0u8; 4];
        reader.read_exact(&mut count_bytes)?;
        let component_count = u32::from_le_bytes(count_bytes) as usize;

        // Read components
        let mut components = Vec::with_capacity(component_count);
        for _ in 0..component_count {
            components.push(ComponentData::read(reader)?);
        }

        Ok(Self {
            stable_id,
            components,
        })
    }
}

/// Component data in the binary format
#[derive(Debug, Clone)]
pub struct ComponentData {
    /// Type ID of the component
    pub type_id: u128,

    /// Serialized component data
    pub data: Vec<u8>,
}

impl ComponentData {
    /// Create new component data
    pub fn new(type_id: u128, data: Vec<u8>) -> Self {
        Self { type_id, data }
    }

    /// Write component data to a writer
    pub fn write(&self, writer: &mut dyn Write) -> io::Result<()> {
        // Write type ID
        writer.write_all(&self.type_id.to_le_bytes())?;

        // Write data length
        writer.write_all(&(self.data.len() as u32).to_le_bytes())?;

        // Write data
        writer.write_all(&self.data)?;

        Ok(())
    }

    /// Read component data from a reader
    pub fn read(reader: &mut dyn Read) -> io::Result<Self> {
        // Read type ID
        let mut type_id_bytes = [0u8; 16];
        reader.read_exact(&mut type_id_bytes)?;
        let type_id = u128::from_le_bytes(type_id_bytes);

        // Read data length
        let mut len_bytes = [0u8; 4];
        reader.read_exact(&mut len_bytes)?;
        let data_len = u32::from_le_bytes(len_bytes) as usize;

        // Read data
        let mut data = vec![0u8; data_len];
        reader.read_exact(&mut data)?;

        Ok(Self { type_id, data })
    }
}

/// Footer with checksum for data integrity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Footer {
    /// CRC64 checksum of all data
    pub checksum: u64,
}

impl Footer {
    /// Create a new footer with checksum
    pub fn new(checksum: u64) -> Self {
        Self { checksum }
    }

    /// Size of the footer in bytes
    pub const FOOTER_SIZE: usize = 8;

    /// Write footer to a writer
    pub fn write(&self, writer: &mut dyn Write) -> io::Result<()> {
        writer.write_all(&self.checksum.to_le_bytes())?;
        Ok(())
    }

    /// Read footer from a reader
    pub fn read(reader: &mut dyn Read) -> io::Result<Self> {
        let mut checksum_bytes = [0u8; 8];
        reader.read_exact(&mut checksum_bytes)?;
        let checksum = u64::from_le_bytes(checksum_bytes);
        Ok(Self { checksum })
    }
}

/// Calculate CRC64 checksum for data integrity
pub fn calculate_checksum(data: &[u8]) -> u64 {
    // Simple CRC64 implementation using polynomial 0x42F0E1EBA9EA3693
    const CRC64_POLY: u64 = 0x42F0E1EBA9EA3693;

    let mut crc: u64 = 0xFFFFFFFFFFFFFFFF;

    for &byte in data {
        crc ^= (byte as u64) << 56;
        for _ in 0..8 {
            if crc & 0x8000000000000000 != 0 {
                crc = (crc << 1) ^ CRC64_POLY;
            } else {
                crc <<= 1;
            }
        }
    }

    crc ^ 0xFFFFFFFFFFFFFFFF
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_format_flags() {
        let mut flags = FormatFlags::NONE;
        assert!(!flags.contains(FormatFlags::COMPRESSED_ZSTD));

        flags.set(FormatFlags::COMPRESSED_ZSTD);
        assert!(flags.contains(FormatFlags::COMPRESSED_ZSTD));

        flags.set(FormatFlags::DELTA);
        assert!(flags.contains(FormatFlags::COMPRESSED_ZSTD));
        assert!(flags.contains(FormatFlags::DELTA));

        flags.clear(FormatFlags::COMPRESSED_ZSTD);
        assert!(!flags.contains(FormatFlags::COMPRESSED_ZSTD));
        assert!(flags.contains(FormatFlags::DELTA));
    }

    #[test]
    fn test_header_roundtrip() {
        let header = Header::new(100, 5);

        let mut buffer = Vec::new();
        header.write(&mut buffer).unwrap();

        let mut cursor = Cursor::new(buffer);
        let read_header = Header::read(&mut cursor).unwrap();

        assert_eq!(header, read_header);
    }

    #[test]
    fn test_header_invalid_magic() {
        let mut buffer = vec![0xFF, 0xFF, 0xFF, 0xFF]; // Invalid magic
        buffer.extend_from_slice(&1u32.to_le_bytes()); // version
        buffer.extend_from_slice(&0u32.to_le_bytes()); // flags
        buffer.extend_from_slice(&0u64.to_le_bytes()); // entity_count
        buffer.extend_from_slice(&0u32.to_le_bytes()); // type_count

        let mut cursor = Cursor::new(buffer);
        let result = Header::read(&mut cursor);

        assert!(result.is_err());
    }

    #[test]
    fn test_type_registry_entry_roundtrip() {
        let entry = TypeRegistryEntry::new(12345, "Position".to_string(), 1);

        let mut buffer = Vec::new();
        entry.write(&mut buffer).unwrap();

        let mut cursor = Cursor::new(buffer);
        let read_entry = TypeRegistryEntry::read(&mut cursor).unwrap();

        assert_eq!(entry, read_entry);
    }

    #[test]
    fn test_component_data_roundtrip() {
        let data = ComponentData::new(12345, vec![1, 2, 3, 4, 5]);

        let mut buffer = Vec::new();
        data.write(&mut buffer).unwrap();

        let mut cursor = Cursor::new(buffer);
        let read_data = ComponentData::read(&mut cursor).unwrap();

        assert_eq!(data.type_id, read_data.type_id);
        assert_eq!(data.data, read_data.data);
    }

    #[test]
    fn test_entity_data_roundtrip() {
        let mut entity = EntityData::new(99999);
        entity.add_component(ComponentData::new(1, vec![1, 2, 3]));
        entity.add_component(ComponentData::new(2, vec![4, 5, 6]));

        let mut buffer = Vec::new();
        entity.write(&mut buffer).unwrap();

        let mut cursor = Cursor::new(buffer);
        let read_entity = EntityData::read(&mut cursor).unwrap();

        assert_eq!(entity.stable_id, read_entity.stable_id);
        assert_eq!(entity.components.len(), read_entity.components.len());
    }

    #[test]
    fn test_footer_roundtrip() {
        let footer = Footer::new(0x1234567890ABCDEF);

        let mut buffer = Vec::new();
        footer.write(&mut buffer).unwrap();

        let mut cursor = Cursor::new(buffer);
        let read_footer = Footer::read(&mut cursor).unwrap();

        assert_eq!(footer, read_footer);
    }

    #[test]
    fn test_checksum_calculation() {
        let data = b"Hello, World!";
        let checksum1 = calculate_checksum(data);
        let checksum2 = calculate_checksum(data);

        // Same data should produce same checksum
        assert_eq!(checksum1, checksum2);

        // Different data should produce different checksum
        let different_data = b"Hello, World?";
        let checksum3 = calculate_checksum(different_data);
        assert_ne!(checksum1, checksum3);
    }

    #[test]
    fn test_checksum_empty_data() {
        let checksum = calculate_checksum(&[]);
        // CRC64 with our polynomial returns 0 for empty data after XOR operations
        // This is expected behavior for this implementation
        assert_eq!(checksum, 0);
    }
}

// Made with Bob
