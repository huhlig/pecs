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

//! Error types for the persistence system.
//!
//! This module provides comprehensive error types for persistence operations,
//! with detailed error messages and context information to aid in debugging.

use std::fmt;
use std::io;
use std::path::PathBuf;

/// Result type for persistence operations.
pub type Result<T> = std::result::Result<T, PersistenceError>;

/// Errors that can occur during persistence operations.
///
/// Each error variant provides detailed information about what went wrong
/// and includes suggestions for resolution where applicable.
#[derive(Debug)]
pub enum PersistenceError {
    /// I/O error occurred during file operations.
    ///
    /// This wraps standard I/O errors and provides additional context.
    Io(io::Error),

    /// Serialization error occurred.
    ///
    /// Contains a description of what failed during serialization.
    Serialization(String),

    /// Deserialization error occurred.
    ///
    /// Contains a description of what failed during deserialization.
    Deserialization(String),

    /// Invalid format or corrupted data.
    ///
    /// This indicates the file format is not recognized or has been corrupted.
    InvalidFormat(String),

    /// Version mismatch between saved data and current version.
    ///
    /// This occurs when loading data saved with a different version of the library.
    /// Consider implementing version migration to handle this case.
    VersionMismatch {
        /// Version found in the saved data.
        found: u32,
        /// Expected version.
        expected: u32,
    },

    /// Component type not found in registry.
    ///
    /// This occurs when deserializing a component type that hasn't been registered.
    /// Ensure all component types are registered before loading.
    UnknownComponentType(String),

    /// Entity ID conflict during load.
    ///
    /// This occurs when trying to load an entity with an ID that already exists.
    EntityIdConflict(String),

    /// Migration failed.
    ///
    /// This occurs when a version migration encounters an error.
    MigrationFailed(String),

    /// Plugin not found.
    ///
    /// This occurs when trying to use a persistence plugin that hasn't been registered.
    /// Register the plugin using `PersistenceManager::register_plugin()`.
    PluginNotFound(String),

    /// Custom error from a plugin.
    ///
    /// This wraps errors from custom persistence plugins.
    PluginError(String),

    /// Checksum mismatch detected.
    ///
    /// This indicates data corruption during save or load.
    /// The file may be corrupted or incompletely written.
    ChecksumMismatch {
        /// Expected checksum.
        expected: u64,
        /// Actual checksum.
        actual: u64,
    },
}

impl PersistenceError {
    /// Create a serialization error with a message.
    ///
    /// # Example
    ///
    /// ```
    /// use pecs::persistence::PersistenceError;
    ///
    /// let error = PersistenceError::serialization_error("Failed to serialize component");
    /// ```
    pub fn serialization_error(msg: impl Into<String>) -> Self {
        Self::Serialization(msg.into())
    }

    /// Create a deserialization error with a message.
    ///
    /// # Example
    ///
    /// ```
    /// use pecs::persistence::PersistenceError;
    ///
    /// let error = PersistenceError::deserialization_error("Invalid data format");
    /// ```
    pub fn deserialization_error(msg: impl Into<String>) -> Self {
        Self::Deserialization(msg.into())
    }

    /// Create an I/O error from a message.
    ///
    /// # Example
    ///
    /// ```
    /// use pecs::persistence::PersistenceError;
    ///
    /// let error = PersistenceError::io_error("Failed to open file");
    /// ```
    pub fn io_error(msg: impl Into<String>) -> Self {
        Self::Io(io::Error::other(msg.into()))
    }

    /// Create an invalid format error.
    ///
    /// # Example
    ///
    /// ```
    /// use pecs::persistence::PersistenceError;
    ///
    /// let error = PersistenceError::invalid_format("Magic bytes do not match");
    /// ```
    pub fn invalid_format(msg: impl Into<String>) -> Self {
        Self::InvalidFormat(msg.into())
    }

    /// Add context to an I/O error with file path information.
    ///
    /// This is useful for providing more detailed error messages that include
    /// the file path that caused the error.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pecs::persistence::PersistenceError;
    /// use std::fs::File;
    /// use std::path::Path;
    ///
    /// let path = Path::new("world.pecs");
    /// let result = File::open(path)
    ///     .map_err(|e| PersistenceError::from(e).with_path(path));
    /// ```
    pub fn with_path(self, path: impl Into<PathBuf>) -> Self {
        match self {
            Self::Io(err) => {
                let path_buf = path.into();
                let msg = format!("{} (file: {})", err, path_buf.display());
                Self::Io(io::Error::new(err.kind(), msg))
            }
            other => other,
        }
    }

    /// Get a suggestion for how to resolve this error, if available.
    ///
    /// Returns `Some` with a helpful suggestion, or `None` if no specific
    /// suggestion is available for this error type.
    ///
    /// # Example
    ///
    /// ```
    /// use pecs::persistence::PersistenceError;
    ///
    /// let error = PersistenceError::ChecksumMismatch {
    ///     expected: 0x1234,
    ///     actual: 0x5678,
    /// };
    ///
    /// if let Some(suggestion) = error.suggestion() {
    ///     println!("Suggestion: {}", suggestion);
    /// }
    /// ```
    pub fn suggestion(&self) -> Option<&str> {
        match self {
            Self::VersionMismatch { .. } => Some(
                "Try using a migration to convert the data to the current version, or re-save the world with the current version",
            ),
            Self::UnknownComponentType(_) => {
                Some("Ensure all component types are registered before loading the world")
            }
            Self::PluginNotFound(_) => {
                Some("Register the required plugin using PersistenceManager::register_plugin()")
            }
            Self::ChecksumMismatch { .. } => {
                Some("The file may be corrupted. Try loading from a backup or re-saving the data")
            }
            Self::InvalidFormat(_) => {
                Some("Ensure the file is a valid PECS persistence file and hasn't been corrupted")
            }
            _ => None,
        }
    }

    /// Check if this error is recoverable.
    ///
    /// Returns `true` if the operation might succeed if retried or if the
    /// issue can be fixed by the user.
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Self::Io(_) | Self::PluginNotFound(_) | Self::UnknownComponentType(_)
        )
    }

    /// Check if this error indicates data corruption.
    ///
    /// Returns `true` if the error suggests the data file is corrupted.
    pub fn is_corruption(&self) -> bool {
        matches!(self, Self::ChecksumMismatch { .. } | Self::InvalidFormat(_))
    }
}

impl fmt::Display for PersistenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => {
                write!(f, "I/O error: {}", err)?;
                if let Some(suggestion) = self.suggestion() {
                    write!(f, "\nSuggestion: {}", suggestion)?;
                }
                Ok(())
            }
            Self::Serialization(msg) => {
                write!(f, "Serialization error: {}", msg)?;
                if let Some(suggestion) = self.suggestion() {
                    write!(f, "\nSuggestion: {}", suggestion)?;
                }
                Ok(())
            }
            Self::Deserialization(msg) => {
                write!(f, "Deserialization error: {}", msg)?;
                if let Some(suggestion) = self.suggestion() {
                    write!(f, "\nSuggestion: {}", suggestion)?;
                }
                Ok(())
            }
            Self::InvalidFormat(msg) => {
                write!(f, "Invalid format: {}", msg)?;
                if let Some(suggestion) = self.suggestion() {
                    write!(f, "\nSuggestion: {}", suggestion)?;
                }
                Ok(())
            }
            Self::VersionMismatch { found, expected } => {
                write!(
                    f,
                    "Version mismatch: found version {}, expected version {}",
                    found, expected
                )?;
                if let Some(suggestion) = self.suggestion() {
                    write!(f, "\nSuggestion: {}", suggestion)?;
                }
                Ok(())
            }
            Self::UnknownComponentType(name) => {
                write!(f, "Unknown component type: '{}'", name)?;
                if let Some(suggestion) = self.suggestion() {
                    write!(f, "\nSuggestion: {}", suggestion)?;
                }
                Ok(())
            }
            Self::EntityIdConflict(msg) => {
                write!(f, "Entity ID conflict: {}", msg)?;
                write!(
                    f,
                    "\nThis usually indicates duplicate entities in the save file"
                )?;
                Ok(())
            }
            Self::MigrationFailed(msg) => {
                write!(f, "Migration failed: {}", msg)?;
                write!(f, "\nCheck the migration implementation for errors")?;
                Ok(())
            }
            Self::PluginNotFound(name) => {
                write!(f, "Plugin not found: '{}'", name)?;
                if let Some(suggestion) = self.suggestion() {
                    write!(f, "\nSuggestion: {}", suggestion)?;
                }
                Ok(())
            }
            Self::PluginError(msg) => {
                write!(f, "Plugin error: {}", msg)?;
                write!(f, "\nCheck the plugin implementation for errors")?;
                Ok(())
            }
            Self::ChecksumMismatch { expected, actual } => {
                write!(
                    f,
                    "Checksum mismatch: expected 0x{:016x}, got 0x{:016x}",
                    expected, actual
                )?;
                if let Some(suggestion) = self.suggestion() {
                    write!(f, "\nSuggestion: {}", suggestion)?;
                }
                Ok(())
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
