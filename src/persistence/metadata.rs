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

//! Metadata tracking for world persistence.

use std::any::TypeId;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::entity::EntityId;

/// Metadata about the world state.
#[derive(Debug, Clone)]
pub struct WorldMetadata {
    pub version: u32,
    pub timestamp: u64,
    pub entity_count: usize,
    pub component_types: Vec<ComponentTypeInfo>,
    pub custom: HashMap<String, String>,
}

impl WorldMetadata {
    pub fn new(version: u32, entity_count: usize, component_types: Vec<ComponentTypeInfo>) -> Self {
        Self {
            version,
            timestamp: Self::current_timestamp(),
            entity_count,
            component_types,
            custom: HashMap::new(),
        }
    }

    pub fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }
}

/// Information about a component type.
#[derive(Debug, Clone)]
pub struct ComponentTypeInfo {
    pub type_id: TypeId,
    pub type_name: String,
    pub version: u32,
    pub size: usize,
}

/// Change tracker for delta persistence.
#[derive(Debug, Default)]
pub struct ChangeTracker {
    created: Vec<EntityId>,
    modified: Vec<EntityId>,
    deleted: Vec<EntityId>,
    last_checkpoint: u64,
    enabled: bool,
}

impl ChangeTracker {
    pub fn new() -> Self {
        Self {
            created: Vec::new(),
            modified: Vec::new(),
            deleted: Vec::new(),
            last_checkpoint: WorldMetadata::current_timestamp(),
            enabled: true,
        }
    }

    pub fn track_created(&mut self, entity: EntityId) {
        if self.enabled && !self.created.contains(&entity) {
            self.created.push(entity);
        }
    }

    pub fn track_modified(&mut self, entity: EntityId) {
        if self.enabled && !self.created.contains(&entity) && !self.modified.contains(&entity) {
            self.modified.push(entity);
        }
    }

    pub fn track_deleted(&mut self, entity: EntityId) {
        if self.enabled {
            self.created.retain(|&e| e != entity);
            self.modified.retain(|&e| e != entity);
            if !self.deleted.contains(&entity) {
                self.deleted.push(entity);
            }
        }
    }

    pub fn created(&self) -> &[EntityId] {
        &self.created
    }

    pub fn modified(&self) -> &[EntityId] {
        &self.modified
    }

    pub fn deleted(&self) -> &[EntityId] {
        &self.deleted
    }

    pub fn has_changes(&self) -> bool {
        !self.created.is_empty() || !self.modified.is_empty() || !self.deleted.is_empty()
    }

    pub fn checkpoint(&mut self) {
        self.created.clear();
        self.modified.clear();
        self.deleted.clear();
        self.last_checkpoint = WorldMetadata::current_timestamp();
    }
}
