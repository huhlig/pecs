//! Error types for the persistence system.

use std::fmt;
use std::io;

/// Result type for persistence operations.
pub type Result<T> = std::result::Result<T, PersistenceError>;

/// Errors that can occur during persistence operations.
#[derive(Debug)]
pub enum PersistenceError {
    /// I/O error occurred during file operations.
    Io(io::Error),

    /// Serialization error occurred.
    Serialization(String),

    /// Deserialization error occurred.
    Deserialization(String),

    /// Invalid format or corrupted data.
    InvalidFormat(String),

    /// Version mismatch between saved data and current version.
    VersionMismatch {
        /// Version found in the saved data.
        found: u32,
        /// Expected version.
        expected: u32,
    },

    /// Component type not found in registry.
    UnknownComponentType(String),

    /// Entity ID conflict during load.
    EntityIdConflict(String),

    /// Migration failed.
    MigrationFailed(String),

    /// Plugin not found.
    PluginNotFound(String),

    /// Custom error from a plugin.
    PluginError(String),

    /// Checksum mismatch detected.
    ChecksumMismatch {
        /// Expected checksum.
        expected: u64,
        /// Actual checksum.
        actual: u64,
    },
}

impl PersistenceError {
    /// Create a serialization error.
    pub fn serialization_error(msg: impl Into<String>) -> Self {
        Self::Serialization(msg.into())
    }

    /// Create a deserialization error.
    pub fn deserialization_error(msg: impl Into<String>) -> Self {
        Self::Deserialization(msg.into())
    }

    /// Create an I/O error.
    pub fn io_error(msg: impl Into<String>) -> Self {
        Self::Io(io::Error::other(msg.into()))
    }
}

impl fmt::Display for PersistenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "I/O error: {}", err),
            Self::Serialization(msg) => write!(f, "Serialization error: {}", msg),
            Self::Deserialization(msg) => write!(f, "Deserialization error: {}", msg),
            Self::InvalidFormat(msg) => write!(f, "Invalid format: {}", msg),
            Self::VersionMismatch { found, expected } => {
                write!(
                    f,
                    "Version mismatch: found version {}, expected {}",
                    found, expected
                )
            }
            Self::UnknownComponentType(name) => {
                write!(f, "Unknown component type: {}", name)
            }
            Self::EntityIdConflict(msg) => write!(f, "Entity ID conflict: {}", msg),
            Self::MigrationFailed(msg) => write!(f, "Migration failed: {}", msg),
            Self::PluginNotFound(name) => write!(f, "Plugin not found: {}", name),
            Self::PluginError(msg) => write!(f, "Plugin error: {}", msg),
            Self::ChecksumMismatch { expected, actual } => {
                write!(
                    f,
                    "Checksum mismatch: expected 0x{:016x}, got 0x{:016x}",
                    expected, actual
                )
            }
        }
    }
}

impl std::error::Error for PersistenceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for PersistenceError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

// Made with Bob
