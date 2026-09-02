//! The project root aggregate: schema version, string table, installations,
//! and the device map, plus synthetic id allocation (DATA_MODEL §2, §11).

use crate::devices::Devices;
use crate::ids::{
    AreaId, BuildingPartId, ComObjectInstanceId, DeviceId, GroupAddressId, GroupRangeId, LineId,
    ParameterInstanceId,
};
use crate::installation::Installation;
use crate::string_table::{Language, StringTable};

/// The schema version this build of the domain model writes. Mirrored into
/// SQLite's `user_version` pragma by `knx-store`; there is no
/// version-skipping migration path and no downgrade (ADR-0003).
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// Synthetic, project-unique id counters. Ids start at 1; 0 is never
/// allocated, which leaves it free for tests to use as an obviously-fake id.
#[derive(Debug, Clone, Default)]
pub struct IdAllocators {
    device: u32,
    area: u32,
    line: u32,
    com_object_instance: u32,
    group_range: u32,
    group_address: u32,
    building_part: u32,
    parameter_instance: u32,
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
}

pub struct Project {
    pub schema_version: u32,
    pub strings: StringTable,
    pub installations: Vec<Installation>,
    pub devices: Devices,
    pub ids: IdAllocators,
}

impl Project {
    pub fn new(default_language: Language) -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            strings: StringTable::new(default_language),
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
}
