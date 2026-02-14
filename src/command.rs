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

//! Command buffer system for thread-safe deferred operations.
//!
//! This module provides a command buffer system that allows operations to be
//! recorded and then applied to the world in a deferred manner. This is essential
//! for thread-safe ECS operations, as it allows systems to record changes without
//! directly mutating the world.
//!
//! # Architecture
//!
//! The command system consists of:
//! - [`Command`]: A trait for operations that can be applied to the world
//! - [`CommandBuffer`]: A buffer that records commands for later execution
//! - Built-in commands for common operations (spawn, despawn, insert, remove)
//!
//! # Examples
//!
//! ```
//! use pecs::prelude::*;
//!
//! let mut world = World::new();
//! let commands = world.commands();
//!
//! // Record commands
//! commands.spawn();
//! commands.spawn();
//!
//! // Apply commands to the world
//! world.apply_commands();
//! assert_eq!(world.len(), 2);
//! ```

use crate::component::Component;
use crate::entity::EntityId;

/// A command that can be applied to the ECS world.
///
/// Commands represent deferred operations that will be executed when the
/// command buffer is applied. All commands must be `Send` to enable
/// thread-safe command recording.
///
/// # Safety
///
/// Implementations must ensure that the command can be safely sent between
/// threads and applied to the world without causing data races.
pub trait Command: Send {
    /// Applies this command to the world.
    ///
    /// This method consumes the command and applies its effects to the
    /// provided world. The world parameter is passed as a mutable raw pointer
    /// to avoid lifetime issues with the command buffer.
    ///
    /// # Arguments
    ///
    /// * `world` - Raw pointer to the world to apply the command to
    ///
    /// # Safety
    ///
    /// The caller must ensure the world pointer is valid and that no other
    /// references to the world exist during command application.
    unsafe fn apply(self: Box<Self>, world: *mut crate::World);
}

/// A buffer for recording commands to be applied later.
///
/// `CommandBuffer` allows systems to record entity and component operations
/// without immediately mutating the world. This enables:
/// - Thread-safe parallel system execution
/// - Deferred entity/component modifications
/// - Batched operations for better performance
///
/// # Thread Safety
///
/// `CommandBuffer` is `Send` but not `Sync`, meaning it can be moved between
/// threads but not shared. Each thread should have its own command buffer.
///
/// # Examples
///
/// ```
/// use pecs::prelude::*;
///
/// let mut world = World::new();
/// let commands = world.commands();
///
/// // Record some commands
/// let entity = commands.spawn();
/// commands.despawn(entity);
///
/// // Apply all commands at once
/// world.apply_commands();
/// ```
pub struct CommandBuffer {
    /// The list of commands to be executed
    commands: Vec<Box<dyn Command>>,

    /// Entities spawned by this buffer (for tracking)
    spawned_entities: Vec<EntityId>,
}

impl CommandBuffer {
    /// Creates a new empty command buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::command::CommandBuffer;
    ///
    /// let buffer = CommandBuffer::new();
    /// ```
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
            spawned_entities: Vec::new(),
        }
    }

    /// Creates a new command buffer with pre-allocated capacity.
    ///
    /// # Arguments
    ///
    /// * `capacity` - Number of commands to pre-allocate space for
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::command::CommandBuffer;
    ///
    /// let buffer = CommandBuffer::with_capacity(100);
    /// ```
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            commands: Vec::with_capacity(capacity),
            spawned_entities: Vec::new(),
        }
    }

    /// Records a command to spawn a new entity.
    ///
    /// Returns a placeholder `EntityId` that will be valid after the buffer
    /// is applied. Note that this ID is temporary and may not match the actual
    /// ID assigned when the command is executed.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::command::CommandBuffer;
    ///
    /// let mut buffer = CommandBuffer::new();
    /// let entity = buffer.spawn();
    /// ```
    pub fn spawn(&mut self) -> EntityId {
        // Create a placeholder entity ID
        // The actual ID will be assigned when the command is applied
        // Using index as placeholder, generation 1
        let placeholder = EntityId::new(self.spawned_entities.len() as u32, 1);
        self.spawned_entities.push(placeholder);

        self.commands.push(Box::new(SpawnCommand));
        placeholder
    }

    /// Records a command to despawn an entity.
    ///
    /// # Arguments
    ///
    /// * `entity` - The entity to despawn
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::prelude::*;
    ///
    /// let mut world = World::new();
    /// let entity = world.spawn_empty();
    ///
    /// let commands = world.commands();
    /// commands.despawn(entity);
    /// world.apply_commands();
    /// assert!(!world.is_alive(entity));
    /// ```
    pub fn despawn(&mut self, entity: EntityId) {
        self.commands.push(Box::new(DespawnCommand { entity }));
    }

    /// Records a command to insert a component on an entity.
    ///
    /// # Arguments
    ///
    /// * `entity` - The entity to add the component to
    /// * `component` - The component to add
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::command::CommandBuffer;
    /// use pecs::component::Component;
    ///
    /// #[derive(Debug)]
    /// struct Position { x: f32, y: f32 }
    /// impl Component for Position {}
    ///
    /// let mut buffer = CommandBuffer::new();
    /// let entity = buffer.spawn();
    /// buffer.insert(entity, Position { x: 0.0, y: 0.0 });
    /// ```
    pub fn insert<T: Component>(&mut self, entity: EntityId, component: T) {
        self.commands
            .push(Box::new(InsertCommand { entity, component }));
    }

    /// Records a command to remove a component from an entity.
    ///
    /// # Arguments
    ///
    /// * `entity` - The entity to remove the component from
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::command::CommandBuffer;
    /// use pecs::component::Component;
    ///
    /// #[derive(Debug)]
    /// struct Position { x: f32, y: f32 }
    /// impl Component for Position {}
    ///
    /// let mut buffer = CommandBuffer::new();
    /// let entity = buffer.spawn();
    /// buffer.remove::<Position>(entity);
    /// ```
    pub fn remove<T: Component>(&mut self, entity: EntityId) {
        self.commands.push(Box::new(RemoveCommand::<T> {
            entity,
            _phantom: std::marker::PhantomData,
        }));
    }

    /// Returns the number of commands in the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::command::CommandBuffer;
    ///
    /// let mut buffer = CommandBuffer::new();
    /// assert_eq!(buffer.len(), 0);
    /// buffer.spawn();
    /// assert_eq!(buffer.len(), 1);
    /// ```
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Returns `true` if the buffer contains no commands.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::command::CommandBuffer;
    ///
    /// let buffer = CommandBuffer::new();
    /// assert!(buffer.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// Clears all commands from the buffer without executing them.
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::command::CommandBuffer;
    ///
    /// let mut buffer = CommandBuffer::new();
    /// buffer.spawn();
    /// buffer.clear();
    /// assert!(buffer.is_empty());
    /// ```
    pub fn clear(&mut self) {
        self.commands.clear();
        self.spawned_entities.clear();
    }

    /// Applies all commands in the buffer to the world.
    ///
    /// This consumes the buffer and executes all recorded commands in order.
    /// After this call, the buffer is empty and can be reused.
    ///
    /// # Arguments
    ///
    /// * `world` - The world to apply commands to
    ///
    /// # Examples
    ///
    /// ```
    /// use pecs::prelude::*;
    ///
    /// let mut world = World::new();
    /// let mut buffer = CommandBuffer::new();
    ///
    /// buffer.spawn();
    /// buffer.spawn();
    /// buffer.apply(&mut world);
    ///
    /// assert_eq!(world.len(), 2);
    /// ```
    pub fn apply(&mut self, world: &mut crate::World) {
        // Take ownership of commands to execute them
        let commands = std::mem::take(&mut self.commands);

        for command in commands {
            unsafe {
                command.apply(world as *mut crate::World);
            }
        }

        // Clear spawned entities tracking
        self.spawned_entities.clear();
    }
}

impl Default for CommandBuffer {
    fn default() -> Self {
        Self::new()
    }
}

// Built-in command implementations

/// Command to spawn a new entity.
struct SpawnCommand;

impl Command for SpawnCommand {
    unsafe fn apply(self: Box<Self>, world: *mut crate::World) {
        // SAFETY: Caller ensures world pointer is valid
        unsafe {
            (*world).spawn_empty();
        }
    }
}

/// Command to despawn an entity.
struct DespawnCommand {
    entity: EntityId,
}

impl Command for DespawnCommand {
    unsafe fn apply(self: Box<Self>, world: *mut crate::World) {
        // SAFETY: Caller ensures world pointer is valid
        unsafe {
            (*world).despawn(self.entity);
        }
    }
}

/// Command to insert a component on an entity.
struct InsertCommand<T: Component> {
    entity: EntityId,
    component: T,
}

impl<T: Component> Command for InsertCommand<T> {
    unsafe fn apply(self: Box<Self>, world: *mut crate::World) {
        // SAFETY: Caller ensures world pointer is valid
        unsafe {
            (*world).insert(self.entity, self.component);
        }
    }
}

/// Command to remove a component from an entity.
struct RemoveCommand<T: Component> {
    entity: EntityId,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Component> Command for RemoveCommand<T> {
    unsafe fn apply(self: Box<Self>, world: *mut crate::World) {
        // SAFETY: Caller ensures world pointer is valid
        unsafe {
            (*world).remove::<T>(self.entity);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_buffer() {
        let buffer = CommandBuffer::new();
        assert!(buffer.is_empty());
        assert_eq!(buffer.len(), 0);
    }

    #[test]
    fn buffer_with_capacity() {
        let buffer = CommandBuffer::with_capacity(100);
        assert!(buffer.is_empty());
    }

    #[test]
    fn spawn_command() {
        let mut buffer = CommandBuffer::new();
        let mut world = crate::World::new();

        buffer.spawn();
        assert_eq!(buffer.len(), 1);

        buffer.apply(&mut world);
        assert_eq!(world.len(), 1);
        assert!(buffer.is_empty());
    }

    #[test]
    fn multiple_spawn_commands() {
        let mut buffer = CommandBuffer::new();
        let mut world = crate::World::new();

        buffer.spawn();
        buffer.spawn();
        buffer.spawn();
        assert_eq!(buffer.len(), 3);

        buffer.apply(&mut world);
        assert_eq!(world.len(), 3);
    }

    #[test]
    fn despawn_command() {
        let mut buffer = CommandBuffer::new();
        let mut world = crate::World::new();

        let entity = world.spawn_empty();
        assert!(world.is_alive(entity));

        buffer.despawn(entity);
        buffer.apply(&mut world);

        assert!(!world.is_alive(entity));
        assert_eq!(world.len(), 0);
    }

    #[test]
    fn mixed_commands() {
        let mut buffer = CommandBuffer::new();
        let mut world = crate::World::new();

        let e1 = world.spawn_empty();
        buffer.spawn();
        buffer.despawn(e1);
        buffer.spawn();

        buffer.apply(&mut world);
        assert_eq!(world.len(), 2); // e1 despawned, 2 new spawned
    }

    #[test]
    fn clear_buffer() {
        let mut buffer = CommandBuffer::new();
        buffer.spawn();
        buffer.spawn();
        assert_eq!(buffer.len(), 2);

        buffer.clear();
        assert!(buffer.is_empty());
    }

    #[test]
    fn reuse_buffer() {
        let mut buffer = CommandBuffer::new();
        let mut world = crate::World::new();

        buffer.spawn();
        buffer.apply(&mut world);
        assert_eq!(world.len(), 1);

        buffer.spawn();
        buffer.apply(&mut world);
        assert_eq!(world.len(), 2);
    }

    #[test]
    fn buffer_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<CommandBuffer>();
    }

    #[derive(Debug)]
    struct TestComponent {
        #[allow(dead_code)]
        value: i32,
    }
    impl Component for TestComponent {}

    #[test]
    fn insert_command_recording() {
        let mut buffer = CommandBuffer::new();
        let entity = buffer.spawn();

        buffer.insert(entity, TestComponent { value: 42 });
        assert_eq!(buffer.len(), 2); // spawn + insert
    }

    #[test]
    fn remove_command_recording() {
        let mut buffer = CommandBuffer::new();
        let entity = buffer.spawn();

        buffer.remove::<TestComponent>(entity);
        assert_eq!(buffer.len(), 2); // spawn + remove
    }
}
