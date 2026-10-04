//! SQLite project storage, schema migrations, and the opaque passthrough store.

use std::fmt;

pub mod activity_history;
pub mod building;
pub mod command_sync;
pub mod devices;
pub mod group;
pub mod manifest;
pub mod migration;
pub mod module_instance;
pub mod opaque;
pub mod parameter;
pub mod project;
pub mod representable;
pub mod strings;
pub mod topology;

pub use command_sync::sync_after_command;
pub use manifest::{insert_manufacturer_refs, load_manufacturer_refs, ManufacturerRef};
pub use migration::{
    open_and_migrate, open_and_migrate_in_memory, MigrationError, CURRENT_SCHEMA_VERSION,
};
pub use opaque::{insert_opaque, load_opaque, StoredOpaqueEntry};
pub use project::{
    load_project, load_project_reporting, save_project, save_project_if_unchanged, AllocatorRepair,
};
/// Re-exported so `knx-app` names the connection type through the storage
/// crate rather than depending on `rusqlite` directly.
pub use rusqlite::{Connection, Error as SqlError};

/// Errors from the entity-persistence layer (`project`, `strings`,
/// `topology`, `building`, `devices`, `group`, `parameter`,
/// `command_sync`) — distinct from `MigrationError`, which is only about
/// getting the schema to `CURRENT_SCHEMA_VERSION`.
#[derive(Debug)]
pub enum StoreError {
    Sqlite(rusqlite::Error),
    /// `load_project` was called against a database with no `project_info`
    /// row — it was migrated but never saved. Distinct from an empty
    /// project (`Project::new`), which is a valid in-memory value that has
    /// simply not been persisted yet.
    NotSaved,
    /// A compare-and-save operation found a different semantic project after
    /// obtaining SQLite's write lock. The caller must rebuild and re-confirm
    /// its plan rather than overwriting the newer project.
    ConcurrentModification,
    /// `save_project` found devices in `Project::devices` that no
    /// `Line::devices` or `Topology::unassigned` list names. `save_project`
    /// writes devices by walking the topology, so such a device has no
    /// installation to be filed under and would be silently dropped —
    /// refused instead (CLAUDE.md: never silently discard information).
    UnreachableDevices(Vec<knx_core::ids::DeviceId>),
    /// Devices placed more than once, or lines listed by several area
    /// entries. The schema holds one placement per device and one area per
    /// line, so saving would keep an arbitrary one — refused instead until
    /// the topology is repaired (MODEL-02).
    AmbiguousTopology {
        devices: Vec<knx_core::ids::DeviceId>,
        lines: Vec<knx_core::ids::LineId>,
    },
    /// The schema cannot hold the project exactly (duplicate ids, an
    /// orphaned line, inconsistent parent/child lists, …). Refused before
    /// any write instead of reopening different (ADR-0074).
    Unrepresentable(Vec<representable::RepresentationIssue>),
    /// The same, one level down: communication object instances owned by
    /// `Project::devices` that no `DeviceInstance::com_objects` list names.
    UnreachableComObjects(Vec<knx_core::ids::ComObjectInstanceId>),
    /// A `com_object_override.attr` value this build does not know — only
    /// reachable from a database written by something other than this code
    /// (hand-edited, corrupted, or a newer/third-party writer).
    UnknownOverrideAttr(String),
    /// A `project_info.group_address_style` value that is none of `Free`,
    /// `TwoLevel`, `ThreeLevel` — same cause as `UnknownOverrideAttr`
    /// (hand-edited, corrupted, or a newer/third-party writer). Refused
    /// rather than silently read back as `ThreeLevel`: a persisted style
    /// that cannot round-trip is data loss (KNOWN_LIMITATIONS.md §84).
    UnknownGroupAddressStyle(String),
    /// Unknown persisted space kind must not be silently rewritten as Building.
    UnknownBuildingPartType(String),
}

impl std::error::Error for StoreError {}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::Sqlite(e) => write!(f, "{e}"),
            StoreError::UnknownBuildingPartType(kind) => write!(
                f,
                "building_part.kind {kind:?} is not a supported building-space type"
            ),
            StoreError::NotSaved => write!(f, "no project has been saved to this database yet"),
            StoreError::ConcurrentModification => {
                write!(f, "project changed after preview; preview again")
            }
            StoreError::UnreachableDevices(ids) => write!(
                f,
                "{} device(s) exist in the project but are named by no line and by no \
                 unassigned list, so they have no installation to be saved under: {}",
                ids.len(),
                join_ids(ids)
            ),
            StoreError::AmbiguousTopology { devices, lines } => write!(
                f,
                "the topology is ambiguous and cannot be saved without losing a placement \
                 (devices placed more than once: [{}]; lines listed by several areas: [{}]); \
                 repair the placements first",
                join_ids(devices),
                join_ids(lines)
            ),
            StoreError::Unrepresentable(issues) => write!(
                f,
                "the project cannot be saved without losing data: {}",
                issues
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("; ")
            ),
            StoreError::UnreachableComObjects(ids) => write!(
                f,
                "{} communication object instance(s) exist in the project but are named by no \
                 device's com_objects list, so they have no device to be saved under: {}",
                ids.len(),
                join_ids(ids)
            ),
            StoreError::UnknownOverrideAttr(attr) => write!(
                f,
                "com_object_override.attr {attr:?} is not an attribute this build knows"
            ),
            StoreError::UnknownGroupAddressStyle(style) => write!(
                f,
                "project_info.group_address_style {style:?} is not Free, TwoLevel or ThreeLevel"
            ),
        }
    }
}

fn join_ids<T: fmt::Display>(ids: &[T]) -> String {
    ids.iter()
        .map(|id| id.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

impl From<rusqlite::Error> for StoreError {
    fn from(e: rusqlite::Error) -> Self {
        StoreError::Sqlite(e)
    }
}
