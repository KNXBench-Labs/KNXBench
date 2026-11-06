//! Immutable captured saved-instance declarations, separate from effective and wire priority.

use crate::SourceRef;

/// KNXBench observation bounds, not source-format limits.
pub const MAX_SAVED_PRIORITY_OBJECTS: usize = 100_000;
pub const MAX_SAVED_PRIORITY_STRING_BYTES: usize = 4_096;
pub const MAX_SAVED_PRIORITY_CAPTURE_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SavedInstancePriority {
    Unavailable { reason: SavedPriorityUnavailable },
    Absent(Box<SavedPriorityAbsent>),
    Declared(Box<SavedPriorityDeclared>),
}

impl Default for SavedInstancePriority {
    fn default() -> Self {
        Self::Unavailable {
            reason: SavedPriorityUnavailable::NotCaptured,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SavedPriorityUnavailable {
    NotCaptured,
    UnsupportedSourceSchema,
    ModuleScope,
    AmbiguousOwner,
    AmbiguousDeclaration,
    SourceIntegrity,
    ResourceLimit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SavedPriorityOmission {
    AttributeOmitted,
    InstanceElementOmitted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SavedPriorityTokenStatus {
    Known,
    Empty,
    Unknown,
}

impl SavedPriorityTokenStatus {
    pub fn classify(raw: &str) -> Self {
        match raw {
            "Low" | "High" | "Alert" => Self::Known,
            "" => Self::Empty,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedPriorityAbsent {
    pub omission: SavedPriorityOmission,
    pub source: SavedPrioritySource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedPriorityDeclared {
    pub raw: String,
    pub source: SavedPrioritySource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedPrioritySource {
    pub source_schema: u32,
    pub device_source: SourceRef,
    pub object_source: SourceRef,
    pub owner_xpath: String,
    pub topology_sha256: String,
}

impl SavedPrioritySource {
    /// Structural validation only; a captured digest does not reauthenticate current bytes.
    pub fn is_valid(&self) -> bool {
        let strings = [
            self.device_source.path.as_str(),
            self.device_source.ets_id.as_str(),
            self.object_source.path.as_str(),
            self.object_source.ets_id.as_str(),
            self.owner_xpath.as_str(),
            self.topology_sha256.as_str(),
        ];
        self.source_schema == 23
            && strings
                .iter()
                .all(|s| !s.is_empty() && s.len() <= MAX_SAVED_PRIORITY_STRING_BYTES)
            && self.device_source.path == self.object_source.path
            && self.topology_sha256.len() == 64
            && self
                .topology_sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    }
}

impl SavedInstancePriority {
    pub fn source(&self) -> Option<&SavedPrioritySource> {
        match self {
            Self::Unavailable { .. } => None,
            Self::Absent(value) => Some(&value.source),
            Self::Declared(value) => Some(&value.source),
        }
    }

    pub fn is_valid(&self) -> bool {
        self.source().is_none_or(SavedPrioritySource::is_valid)
            && match self {
                Self::Declared(value) => value.raw.len() <= MAX_SAVED_PRIORITY_STRING_BYTES,
                _ => true,
            }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_instance_priority_default_and_lexical_tokens_are_not_effective_values() {
        assert_eq!(
            SavedInstancePriority::default(),
            SavedInstancePriority::Unavailable {
                reason: SavedPriorityUnavailable::NotCaptured
            }
        );
        assert!(std::mem::size_of::<SavedInstancePriority>() <= 24);
        for raw in ["Low", "High", "Alert"] {
            assert_eq!(
                SavedPriorityTokenStatus::classify(raw),
                SavedPriorityTokenStatus::Known
            );
        }
        assert_eq!(
            SavedPriorityTokenStatus::classify(""),
            SavedPriorityTokenStatus::Empty
        );
        for raw in [" ", "low", "Future", " Löw "] {
            assert_eq!(
                SavedPriorityTokenStatus::classify(raw),
                SavedPriorityTokenStatus::Unknown
            );
        }
    }
}

/// Operation-local owner/source index; build once for a complete device detail or capture.
pub struct SavedPriorityBindings<'a> {
    project: &'a crate::Project,
    device_ids: std::collections::HashMap<crate::DeviceId, usize>,
    object_ids: std::collections::HashMap<crate::ComObjectInstanceId, usize>,
    devices: std::collections::HashMap<(&'a str, &'a str), usize>,
    objects: std::collections::HashMap<(crate::DeviceId, &'a str, &'a str), usize>,
    claims: std::collections::HashMap<crate::ComObjectInstanceId, (crate::DeviceId, usize)>,
}

impl<'a> SavedPriorityBindings<'a> {
    pub fn new(project: &'a crate::Project) -> Self {
        let mut index = Self {
            project,
            device_ids: Default::default(),
            object_ids: Default::default(),
            devices: Default::default(),
            objects: Default::default(),
            claims: Default::default(),
        };
        for device in project.devices.iter() {
            *index.device_ids.entry(device.id).or_default() += 1;
            *index
                .devices
                .entry((&device.source.path, &device.source.ets_id))
                .or_default() += 1;
            for id in &device.com_objects {
                let claim = index.claims.entry(*id).or_insert((device.id, 0));
                claim.1 += 1;
            }
        }
        for com in project.devices.com_objects() {
            *index.object_ids.entry(com.id).or_default() += 1;
            *index
                .objects
                .entry((com.device, &com.source.path, &com.source.ets_id))
                .or_default() += 1;
        }
        index
    }

    pub fn unique_owner(&self, com: &crate::ComObjectInstance, owner: crate::DeviceId) -> bool {
        let Some(device) = self.project.devices.get(owner) else {
            return false;
        };
        com.device == owner
            && device.id == owner
            && self.device_ids.get(&owner) == Some(&1)
            && self.object_ids.get(&com.id) == Some(&1)
            && self
                .project
                .devices
                .com_object(com.id)
                .is_some_and(|record| std::ptr::eq(record, com))
            && self
                .devices
                .get(&(device.source.path.as_str(), device.source.ets_id.as_str()))
                == Some(&1)
            && self
                .objects
                .get(&(owner, com.source.path.as_str(), com.source.ets_id.as_str()))
                == Some(&1)
            && self.claims.get(&com.id) == Some(&(owner, 1))
    }

    pub fn captured_binding(&self, com: &crate::ComObjectInstance, owner: crate::DeviceId) -> bool {
        let Some(source) = com.saved_instance_priority.source() else {
            return false;
        };
        self.unique_owner(com, owner)
            && com.saved_instance_priority.is_valid()
            && com.module_instance.is_none()
            && !com.source.ets_id.starts_with("MD-")
            && source.object_source == com.source
            && self
                .project
                .devices
                .get(owner)
                .is_some_and(|device| device.source == source.device_source)
    }
}
