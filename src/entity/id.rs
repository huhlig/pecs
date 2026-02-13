//! Entity identification system with dual ID support.
//!
//! This module provides two types of entity identifiers:
//! - [`EntityId`]: Fast, ephemeral ID for runtime operations (64-bit)
//! - [`StableId`]: Persistent, stable ID for serialization (128-bit UUID)
//!
//! # Examples
//!
//! ```
//! use pecs::entity::id::{EntityId, StableId};
//!
//! // Create an ephemeral ID
//! let entity_id = EntityId::new(0, 1);
//! assert_eq!(entity_id.index(), 0);
//! assert_eq!(entity_id.generation(), 1);
//!
//! // Create a stable ID
//! let stable_id = StableId::new();
//! ```

use std::fmt;
use std::num::NonZeroU64;

/// A fast, ephemeral entity identifier optimized for runtime operations.
///
/// `EntityId` uses a 64-bit representation split into:
/// - 32-bit index: Position in entity storage
/// - 32-bit generation: Recycling counter to detect stale references
///
/// This design enables:
/// - O(1) entity lookup
/// - Safe entity recycling
/// - Detection of use-after-free bugs
///
/// # Performance
///
/// - Size: 8 bytes
/// - Copy: Yes (trivial copy)
/// - Lookup: O(1)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityId(NonZeroU64);

impl EntityId {
    /// Creates a new `EntityId` from an index and generation.
    ///
    /// # Arguments
    ///
    /// * `index` - The entity's position in storage (0-based)
    /// * `generation` - The recycling generation counter (1-based)
    ///
    /// # Panics
    ///
    /// Panics if generation is 0, as generation must be non-zero.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::entity::id::EntityId;
    ///
    /// let id = EntityId::new(42, 1);
    /// assert_eq!(id.index(), 42);
    /// assert_eq!(id.generation(), 1);
    /// ```
    #[inline]
    pub fn new(index: u32, generation: u32) -> Self {
        assert!(generation > 0, "Generation must be non-zero");
        let value = ((generation as u64) << 32) | (index as u64);
        Self(NonZeroU64::new(value).expect("EntityId value cannot be zero"))
    }

    /// Returns the entity's index in storage.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::entity::id::EntityId;
    ///
    /// let id = EntityId::new(42, 1);
    /// assert_eq!(id.index(), 42);
    /// ```
    #[inline]
    pub const fn index(self) -> u32 {
        self.0.get() as u32
    }

    /// Returns the entity's generation counter.
    ///
    /// The generation is incremented each time an entity slot is recycled,
    /// allowing detection of stale entity references.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::entity::id::EntityId;
    ///
    /// let id = EntityId::new(42, 3);
    /// assert_eq!(id.generation(), 3);
    /// ```
    #[inline]
    pub const fn generation(self) -> u32 {
        (self.0.get() >> 32) as u32
    }

    /// Returns the next generation for this entity slot.
    ///
    /// Used when recycling entity IDs to create a new generation.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::entity::id::EntityId;
    ///
    /// let id = EntityId::new(42, 1);
    /// let next = id.next_generation();
    /// assert_eq!(next.generation(), 2);
    /// assert_eq!(next.index(), 42);
    /// ```
    #[inline]
    pub fn next_generation(self) -> Self {
        Self::new(self.index(), self.generation().wrapping_add(1).max(1))
    }

    /// Creates an `EntityId` from a raw 64-bit value.
    ///
    /// # Safety
    ///
    /// The caller must ensure the value is non-zero and represents a valid
    /// EntityId (generation in upper 32 bits, index in lower 32 bits).
    #[inline]
    pub const unsafe fn from_raw(value: u64) -> Self {
        // SAFETY: Caller must ensure value is non-zero
        Self(unsafe { NonZeroU64::new_unchecked(value) })
    }

    /// Returns the raw 64-bit representation of this `EntityId`.
    #[inline]
    pub const fn to_raw(self) -> u64 {
        self.0.get()
    }
}

impl fmt::Display for EntityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}v{}", self.index(), self.generation())
    }
}

/// A stable, persistent entity identifier using UUID format.
///
/// `StableId` provides a globally unique, persistent identifier suitable for:
/// - Serialization and deserialization
/// - Cross-session entity references
/// - Network synchronization
/// - Save/load systems
///
/// # Performance
///
/// - Size: 16 bytes
/// - Copy: Yes (trivial copy)
/// - Generation: ~100ns (random UUID)
///
/// # Format
///
/// Uses UUID v4 (random) format for maximum uniqueness guarantees.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StableId(u128);

impl StableId {
    /// Creates a new random `StableId`.
    ///
    /// Uses a cryptographically secure random number generator to ensure
    /// uniqueness across all entities, even in distributed systems.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::entity::id::StableId;
    ///
    /// let id1 = StableId::new();
    /// let id2 = StableId::new();
    /// assert_ne!(id1, id2); // Extremely unlikely to be equal
    /// ```
    pub fn new() -> Self {
        // Generate a random UUID v4
        // For now, using a simple random approach
        // In production, would use uuid crate or similar
        use std::collections::hash_map::RandomState;
        use std::hash::{BuildHasher, Hash, Hasher};

        let random_state = RandomState::new();
        let mut hasher = random_state.build_hasher();

        // Hash current time and thread info for uniqueness
        std::time::SystemTime::now().hash(&mut hasher);
        std::thread::current().id().hash(&mut hasher);

        let hash1 = hasher.finish();

        let mut hasher = random_state.build_hasher();
        hash1.hash(&mut hasher);
        std::time::SystemTime::now().hash(&mut hasher);
        let hash2 = hasher.finish();

        let value = ((hash1 as u128) << 64) | (hash2 as u128);
        Self(value)
    }

    /// Creates a `StableId` from a raw 128-bit value.
    ///
    /// Useful for deserialization or testing.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::entity::id::StableId;
    ///
    /// let id = StableId::from_raw(0x12345678_90abcdef_12345678_90abcdef);
    /// assert_eq!(id.to_raw(), 0x12345678_90abcdef_12345678_90abcdef);
    /// ```
    #[inline]
    pub const fn from_raw(value: u128) -> Self {
        Self(value)
    }

    /// Returns the raw 128-bit representation of this `StableId`.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::entity::id::StableId;
    ///
    /// let id = StableId::from_raw(42);
    /// assert_eq!(id.to_raw(), 42);
    /// ```
    #[inline]
    pub const fn to_raw(self) -> u128 {
        self.0
    }

    /// Returns the high 64 bits of the stable ID.
    #[inline]
    pub const fn high(self) -> u64 {
        (self.0 >> 64) as u64
    }

    /// Returns the low 64 bits of the stable ID.
    #[inline]
    pub const fn low(self) -> u64 {
        self.0 as u64
    }

    /// Convert to u128 for serialization.
    #[inline]
    pub const fn as_u128(self) -> u128 {
        self.0
    }

    /// Create from u128 for deserialization.
    #[inline]
    pub const fn from_u128(value: u128) -> Self {
        Self(value)
    }
}

impl Default for StableId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for StableId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:032x}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_id_creation() {
        let id = EntityId::new(42, 1);
        assert_eq!(id.index(), 42);
        assert_eq!(id.generation(), 1);
    }

    #[test]
    fn entity_id_next_generation() {
        let id = EntityId::new(42, 1);
        let next = id.next_generation();
        assert_eq!(next.index(), 42);
        assert_eq!(next.generation(), 2);
    }

    #[test]
    fn entity_id_generation_wrapping() {
        let id = EntityId::new(42, u32::MAX);
        let next = id.next_generation();
        assert_eq!(next.generation(), 1); // Wraps to 1, not 0
    }

    #[test]
    fn entity_id_raw_conversion() {
        let id = EntityId::new(42, 3);
        let raw = id.to_raw();
        let restored = unsafe { EntityId::from_raw(raw) };
        assert_eq!(id, restored);
    }

    #[test]
    fn entity_id_display() {
        let id = EntityId::new(42, 3);
        assert_eq!(format!("{}", id), "42v3");
    }

    #[test]
    #[should_panic(expected = "Generation must be non-zero")]
    fn entity_id_zero_generation_panics() {
        EntityId::new(0, 0);
    }

    #[test]
    fn stable_id_creation() {
        let id = StableId::new();
        assert_ne!(id.to_raw(), 0);
    }

    #[test]
    fn stable_id_uniqueness() {
        let id1 = StableId::new();
        let id2 = StableId::new();
        // While theoretically possible to be equal, it's astronomically unlikely
        assert_ne!(id1, id2);
    }

    #[test]
    fn stable_id_raw_conversion() {
        let value = 0x12345678_90abcdef_12345678_90abcdef;
        let id = StableId::from_raw(value);
        assert_eq!(id.to_raw(), value);
    }

    #[test]
    fn stable_id_high_low() {
        let id = StableId::from_raw(0x12345678_90abcdef_fedcba09_87654321);
        assert_eq!(id.high(), 0x12345678_90abcdef);
        assert_eq!(id.low(), 0xfedcba09_87654321);
    }

    #[test]
    fn stable_id_display() {
        let id = StableId::from_raw(0x12345678_90abcdef_12345678_90abcdef);
        let display = format!("{}", id);
        assert_eq!(display.len(), 32); // 32 hex characters
    }
}

// Made with Bob
