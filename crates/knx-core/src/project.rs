//! The project root aggregate: schema version, string table, installations,
//! and the device map, plus synthetic id allocation (DATA_MODEL §2, §11).

use chrono::{DateTime, Utc};

use crate::address::GroupAddressStyle;
use crate::commissioning::CompletionStatus;
use crate::devices::Devices;
use crate::ids::{
    AreaId, BuildingPartId, ComObjectInstanceId, DeviceId, GroupAddressId, GroupRangeId, LineId,
    ModuleInstanceId, ParameterInstanceId,
};
use crate::installation::Installation;
use crate::string_table::{Language, StringTable};

/// The schema version this build of the domain model writes. Mirrored into
/// SQLite's `user_version` pragma by `knx-store`; there is no
/// version-skipping migration path and no downgrade (ADR-0003).
pub const CURRENT_SCHEMA_VERSION: u32 = 5;

/// Synthetic, project-unique id counters. Ids start at 1; 0 is never
/// allocated, which leaves it free for tests to use as an obviously-fake id.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct IdAllocators {
    device: u32,
    area: u32,
    line: u32,
    com_object_instance: u32,
    group_range: u32,
    group_address: u32,
    building_part: u32,
    parameter_instance: u32,
    module_instance: u32,
}

macro_rules! next_id {
    ($self:ident, $field:ident, $id_type:ident) => {{
        $self.$field += 1;
        $id_type($self.$field)
    }};
}

impl IdAllocators {
    pub fn next_device_id(&mut self) -> DeviceId {
        next_id!(self, device, DeviceId)
    }

    pub fn next_area_id(&mut self) -> AreaId {
        next_id!(self, area, AreaId)
    }

    pub fn next_line_id(&mut self) -> LineId {
        next_id!(self, line, LineId)
    }

    pub fn next_com_object_instance_id(&mut self) -> ComObjectInstanceId {
        next_id!(self, com_object_instance, ComObjectInstanceId)
    }

    pub fn next_group_range_id(&mut self) -> GroupRangeId {
        next_id!(self, group_range, GroupRangeId)
    }

    pub fn next_group_address_id(&mut self) -> GroupAddressId {
        next_id!(self, group_address, GroupAddressId)
    }

    pub fn next_building_part_id(&mut self) -> BuildingPartId {
        next_id!(self, building_part, BuildingPartId)
    }

    pub fn next_parameter_instance_id(&mut self) -> ParameterInstanceId {
        next_id!(self, parameter_instance, ParameterInstanceId)
    }

    pub fn next_module_instance_id(&mut self) -> ModuleInstanceId {
        next_id!(self, module_instance, ModuleInstanceId)
    }

    pub fn peek_device(&self) -> u32 {
        self.device
    }
    pub fn peek_area(&self) -> u32 {
        self.area
    }
    pub fn peek_line(&self) -> u32 {
        self.line
    }
    pub fn peek_com_object_instance(&self) -> u32 {
        self.com_object_instance
    }
    pub fn peek_group_range(&self) -> u32 {
        self.group_range
    }
    pub fn peek_group_address(&self) -> u32 {
        self.group_address
    }
    pub fn peek_building_part(&self) -> u32 {
        self.building_part
    }
    pub fn peek_parameter_instance(&self) -> u32 {
        self.parameter_instance
    }
    pub fn peek_module_instance(&self) -> u32 {
        self.module_instance
    }

    /// Reconstructs an `IdAllocators` at exactly the counts given —
    /// `knx-store::load_project`'s way of restoring allocator state so a
    /// freshly loaded project never reissues an id already in use.
    #[allow(clippy::too_many_arguments)]
    pub fn from_counts(
        device: u32,
        area: u32,
        line: u32,
        com_object_instance: u32,
        group_range: u32,
        group_address: u32,
        building_part: u32,
        parameter_instance: u32,
        module_instance: u32,
    ) -> Self {
        Self {
            device,
            area,
            line,
            com_object_instance,
            group_range,
            group_address,
            building_part,
            parameter_instance,
            module_instance,
        }
    }
}

/// `Project/@Id` plus the `ProjectInformation` element.
///
/// `group_address_style` is here rather than on `GroupAddress` because it is a
/// project-wide rendering choice: the 16-bit value never changes, only how it
/// is written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectInfo {
    /// `Project/@Id`, e.g. `"P-0512"`. The container's project part is named
    /// after it.
    pub project_id: String,
    /// `ProjectInformation/@Name`.
    pub name: String,
    /// `ProjectInformation/@ProjectId` — the user-facing project number, a
    /// different thing from `project_id` despite the attribute name.
    pub project_number: Option<String>,
    pub group_address_style: GroupAddressStyle,
    pub completion: CompletionStatus,
    pub last_modified: Option<DateTime<Utc>>,
    pub project_start: Option<DateTime<Utc>>,
    /// The ETS project XML's own schema version (11, 21 or 23) — unrelated
    /// to `CURRENT_SCHEMA_VERSION`, which is this crate's persistence-format
    /// version. Schema 11 devices carry a monolithic application program;
    /// schema ≥21 devices are module-based (ADR-0013).
    pub ets_schema_version: u32,
}

impl Default for ProjectInfo {
    fn default() -> Self {
        Self {
            project_id: String::new(),
            name: String::new(),
            project_number: None,
            // ETS's own default, and the style of both reference exports.
            group_address_style: GroupAddressStyle::ThreeLevel,
            completion: CompletionStatus::Undefined,
            last_modified: None,
            project_start: None,
            ets_schema_version: 11,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Project {
    pub schema_version: u32,
    pub strings: StringTable,
    pub info: ProjectInfo,
    pub installations: Vec<Installation>,
    pub devices: Devices,
    pub ids: IdAllocators,
}

impl Project {
    pub fn new(default_language: Language) -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            strings: StringTable::new(default_language),
            info: ProjectInfo::default(),
            installations: Vec::new(),
            devices: Devices::new(),
            ids: IdAllocators::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_allocator_starts_at_one_and_increments() {
        let mut ids = IdAllocators::default();
        assert_eq!(ids.next_device_id(), DeviceId(1));
        assert_eq!(ids.next_device_id(), DeviceId(2));
        assert_eq!(ids.next_group_address_id(), GroupAddressId(1));
    }

    #[test]
    fn new_project_carries_current_schema_version() {
        let p = Project::new(Language("en".into()));
        assert_eq!(p.schema_version, CURRENT_SCHEMA_VERSION);
        assert!(p.installations.is_empty());
    }

    #[test]
    fn a_project_carries_its_group_address_style_and_identity() {
        let p = Project::new(Language("de-DE".into()));
        assert_eq!(p.info.group_address_style, GroupAddressStyle::ThreeLevel);
        assert!(p.info.project_id.is_empty());
        assert!(p.info.name.is_empty());
    }
}
