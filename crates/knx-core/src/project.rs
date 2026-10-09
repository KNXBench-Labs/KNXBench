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
///
/// 8 belongs to the schema->=21 export path (a concurrent branch at the time
/// of writing); 9 is `com_object_program_default` (ADR-0012 gap 2,
/// ADR-0027). Must always equal `knx_store::migration::CURRENT_SCHEMA_VERSION`.
pub const CURRENT_SCHEMA_VERSION: u32 = 11;

/// Synthetic, project-unique id counters. Ids start at 1; 0 is never
/// allocated, which leaves it free for tests to use as an obviously-fake id.
/// The counter is the last issued ID, not the next one. `u32::MAX` is a
/// valid final ID; subsequent allocation refuses without changing any counter.
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

/// A project-local ID space has no representable ID left. This is not a
/// malformed imported counter: an exhausted high-water mark is valid storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IdAllocationError {
    /// Static allocator field name, never an external source identifier.
    pub kind: &'static str,
}

impl std::fmt::Display for IdAllocationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "project {} ID range exhausted", self.kind)
    }
}

impl std::error::Error for IdAllocationError {}

macro_rules! next_id {
    ($self:ident, $field:ident, $id_type:ident) => {{
        let next = $self.$field.checked_add(1).ok_or(IdAllocationError {
            kind: stringify!($field),
        })?;
        $self.$field = next;
        Ok($id_type(next))
    }};
}

impl IdAllocators {
    pub fn next_device_id(&mut self) -> Result<DeviceId, IdAllocationError> {
        next_id!(self, device, DeviceId)
    }

    pub fn next_area_id(&mut self) -> Result<AreaId, IdAllocationError> {
        next_id!(self, area, AreaId)
    }

    pub fn next_line_id(&mut self) -> Result<LineId, IdAllocationError> {
        next_id!(self, line, LineId)
    }

    pub fn next_com_object_instance_id(
        &mut self,
    ) -> Result<ComObjectInstanceId, IdAllocationError> {
        next_id!(self, com_object_instance, ComObjectInstanceId)
    }

    pub fn next_group_range_id(&mut self) -> Result<GroupRangeId, IdAllocationError> {
        next_id!(self, group_range, GroupRangeId)
    }

    pub fn next_group_address_id(&mut self) -> Result<GroupAddressId, IdAllocationError> {
        next_id!(self, group_address, GroupAddressId)
    }

    pub fn next_building_part_id(&mut self) -> Result<BuildingPartId, IdAllocationError> {
        next_id!(self, building_part, BuildingPartId)
    }

    pub fn next_parameter_instance_id(&mut self) -> Result<ParameterInstanceId, IdAllocationError> {
        next_id!(self, parameter_instance, ParameterInstanceId)
    }

    pub fn next_module_instance_id(&mut self) -> Result<ModuleInstanceId, IdAllocationError> {
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

    /// Raises every counter to at least `other`'s and never lowers one —
    /// `Command::ReserveIds`'s semantics (ADR-0039 Decision 2). Returns
    /// whether any counter moved.
    pub fn raise_to(&mut self, other: &IdAllocators) -> bool {
        let before = self.clone();
        self.device = self.device.max(other.device);
        self.area = self.area.max(other.area);
        self.line = self.line.max(other.line);
        self.com_object_instance = self.com_object_instance.max(other.com_object_instance);
        self.group_range = self.group_range.max(other.group_range);
        self.group_address = self.group_address.max(other.group_address);
        self.building_part = self.building_part.max(other.building_part);
        self.parameter_instance = self.parameter_instance.max(other.parameter_instance);
        self.module_instance = self.module_instance.max(other.module_instance);
        *self != before
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
    /// `GroupAddress/@DatapointType` opaque rows the v10 migration could not
    /// attribute to one group address (unkeyed rows from early imports), so
    /// they were not lifted into `GroupAddressEntry::declared_dpt`
    /// (ADR-0078 D3). Non-zero makes an absent declaration resolve as
    /// `DeclarationNotLifted`.
    pub unlifted_group_address_dpt_declarations: u32,
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
            unlifted_group_address_dpt_declarations: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
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

    /// Compares all persisted and user-visible project content while
    /// disregarding synthetic ID allocator high-water marks. Allocators can
    /// advance during an edit that is later undone without changing what the
    /// user can save or inspect.
    pub fn same_user_content_as(&self, other: &Self) -> bool {
        let mut left = self.clone();
        let mut right = other.clone();
        left.ids = IdAllocators::default();
        right.ids = IdAllocators::default();
        left == right
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! exhaustion_test {
        ($name:ident, $field:ident, $next:ident) => {
            #[test]
            fn $name() {
                let mut ids = IdAllocators::default();
                ids.$field = u32::MAX;
                let before = ids.clone();
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| ids.$next()));
                assert!(
                    result.is_ok(),
                    "ID exhaustion must be a refusal, not a panic"
                );
                let error = result.unwrap().unwrap_err();
                assert_eq!(error.kind, stringify!($field));
                assert!(error.to_string().contains("ID range exhausted"));
                assert_eq!(ids, before, "a refused allocation must not consume IDs");
                assert_eq!(ids.$next(), Err(error), "repeated refusal must be stable");

                ids.$field = u32::MAX - 1;
                assert_eq!(ids.$next().unwrap().0, u32::MAX);
                assert_eq!(ids.$next(), Err(error));
            }
        };
    }

    exhaustion_test!(exhausted_device_is_unchanged, device, next_device_id);
    exhaustion_test!(exhausted_area_is_unchanged, area, next_area_id);
    exhaustion_test!(exhausted_line_is_unchanged, line, next_line_id);
    exhaustion_test!(
        exhausted_com_object_is_unchanged,
        com_object_instance,
        next_com_object_instance_id
    );
    exhaustion_test!(
        exhausted_group_range_is_unchanged,
        group_range,
        next_group_range_id
    );
    exhaustion_test!(
        exhausted_group_address_is_unchanged,
        group_address,
        next_group_address_id
    );
    exhaustion_test!(
        exhausted_building_part_is_unchanged,
        building_part,
        next_building_part_id
    );
    exhaustion_test!(
        exhausted_parameter_is_unchanged,
        parameter_instance,
        next_parameter_instance_id
    );
    exhaustion_test!(
        exhausted_module_is_unchanged,
        module_instance,
        next_module_instance_id
    );

    #[test]
    fn id_allocator_starts_at_one_and_increments() {
        let mut ids = IdAllocators::default();
        assert_eq!(ids.next_device_id().unwrap(), DeviceId(1));
        assert_eq!(ids.next_device_id().unwrap(), DeviceId(2));
        assert_eq!(ids.next_group_address_id().unwrap(), GroupAddressId(1));
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

    #[test]
    fn same_user_content_ignores_allocator_high_water_marks_but_not_project_fields() {
        let baseline = Project::new(Language("en".into()));
        let mut allocator_advanced = baseline.clone();
        allocator_advanced.ids.next_device_id().unwrap();

        assert!(baseline.same_user_content_as(&allocator_advanced));

        let mut renamed = baseline.clone();
        renamed.info.name = "Renamed".into();
        assert!(!baseline.same_user_content_as(&renamed));
    }
}
