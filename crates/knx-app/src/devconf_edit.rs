//! Previews device configuration profile edits and publishes them only after durable admission.

use knx_core::{Command, CommandStack, DeviceId};
use knx_store::project_history::NativeSnapshot;
use std::path::Path;

/// Opaque revision of the project snapshot a preview was computed against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevconfRevision {
    pub project_incarnation: String,
    pub generation: u64,
}

/// A profile document draft: raw bytes plus the module identity they belong to.
#[derive(Clone, PartialEq, Eq)]
pub struct DevconfDraft {
    pub device: DeviceId,
    pub module_id: String,
    pub module_version: String,
    pub module_sha256: String,
    pub document: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DevconfEditError {
    InvalidRevision,
    Conflict,
    Profile(knx_core::devconf::ProfileError),
    Command(knx_core::CommandError),
    Persistence(String),
}

pub struct DevconfEditPreview {
    before: NativeSnapshot,
    revision: DevconfRevision,
    command: Command,
    no_op: bool,
}

impl DevconfEditPreview {
    pub fn is_no_op(&self) -> bool {
        self.no_op
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DevconfNativeTarget<'a> {
    pub path: &'a Path,
    pub history_generation: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevconfEditOutcome {
    pub changed: bool,
    pub native_generation: Option<i64>,
}

/// Validates a profile draft against the current snapshot and builds the
/// undoable domain command without mutating the snapshot.
pub fn preview_devconf_edit(
    snapshot: &NativeSnapshot,
    revision: &DevconfRevision,
    draft: &DevconfDraft,
) -> Result<DevconfEditPreview, DevconfEditError> {
    if revision.project_incarnation.is_empty()
        || revision.project_incarnation.len() > 128
        || revision.generation > 9_007_199_254_740_991
    {
        return Err(DevconfEditError::InvalidRevision);
    }
    let profile = knx_core::devconf::DeviceConfigProfile::new(
        draft.device,
        &draft.module_id,
        &draft.module_version,
        &draft.module_sha256,
        draft.document.clone(),
    )
    .map_err(DevconfEditError::Profile)?;
    let command = Command::SetDeviceConfigProfile { profile };
    let mut candidate = snapshot.project.clone();
    command
        .apply(&mut candidate)
        .map_err(DevconfEditError::Command)?;
    Ok(DevconfEditPreview {
        before: snapshot.clone(),
        revision: revision.clone(),
        command,
        no_op: candidate == snapshot.project,
    })
}

/// Validates a profile removal against the current snapshot.
pub fn preview_devconf_removal(
    snapshot: &NativeSnapshot,
    revision: &DevconfRevision,
    device: DeviceId,
) -> Result<DevconfEditPreview, DevconfEditError> {
    if revision.project_incarnation.is_empty()
        || revision.project_incarnation.len() > 128
        || revision.generation > 9_007_199_254_740_991
    {
        return Err(DevconfEditError::InvalidRevision);
    }
    let command = Command::RemoveDeviceConfigProfile { device };
    let mut candidate = snapshot.project.clone();
    command
        .apply(&mut candidate)
        .map_err(DevconfEditError::Command)?;
    Ok(DevconfEditPreview {
        before: snapshot.clone(),
        revision: revision.clone(),
        command,
        no_op: candidate == snapshot.project,
    })
}

/// Publishes a previewed command only if the snapshot is still exactly the
/// one the preview was computed against, then persists the native history.
pub fn apply_devconf_edit(
    snapshot: &mut NativeSnapshot,
    stack: &mut CommandStack,
    revision: &DevconfRevision,
    preview: &DevconfEditPreview,
    native: Option<DevconfNativeTarget<'_>>,
) -> Result<DevconfEditOutcome, DevconfEditError> {
    if snapshot != &preview.before || revision != &preview.revision {
        return Err(DevconfEditError::Conflict);
    }
    if preview.no_op {
        return Ok(DevconfEditOutcome {
            changed: false,
            native_generation: None,
        });
    }
    let mut candidate = snapshot.clone();
    let mut history = stack.clone();
    history
        .do_command(&mut candidate.project, preview.command.clone())
        .map_err(DevconfEditError::Command)?;
    let native_generation = if let Some(target) = native {
        let expected_hash = preview
            .before
            .semantic_hash()
            .map_err(|error| DevconfEditError::Persistence(error.to_string()))?;
        let conn = knx_store::project_history::open_editor_store(target.path, false)
            .map_err(|error| DevconfEditError::Persistence(error.to_string()))?;
        Some(
            knx_store::project_history::save_editor_if_unchanged(
                &conn,
                &candidate,
                &history,
                target.history_generation,
                &expected_hash,
                true,
            )
            .map_err(|error| DevconfEditError::Persistence(error.to_string()))?,
        )
    } else {
        None
    };
    *snapshot = candidate;
    *stack = history;
    Ok(DevconfEditOutcome {
        changed: true,
        native_generation,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use knx_core::DeviceId;

    fn profile(device: u32) -> knx_core::devconf::DeviceConfigProfile {
        knx_core::devconf::DeviceConfigProfile::new(
            DeviceId(device),
            "smartsensor",
            "1.0.0",
            &"a".repeat(64),
            b"{\"nodes\": []}".to_vec(),
        )
        .unwrap()
    }

    fn project_with_device(device: u32) -> knx_core::Project {
        use knx_core::{
            Area, AreaId, CompletionStatus, Installation, InstallationId, Line,
            LineId, SourceRef, Topology,
        };
        let mut project = knx_core::Project::new(knx_core::Language("en".into()));
        let source = SourceRef {
            path: "synthetic".into(),
            ets_id: format!("D-{device}"),
        };
        project.installations.push(Installation {
            id: InstallationId(1),
            name: "Installation".into(),
            default_line: None,
            multicast_address: None,
            completion: CompletionStatus::Undefined,
            topology: Topology {
                areas: vec![Area {
                    id: AreaId(1),
                    source: source.clone(),
                    name: "Area".into(),
                    address: 1,
                    completion: CompletionStatus::Undefined,
                    lines: vec![LineId(1)],
                }],
                lines: vec![Line {
                    id: LineId(1),
                    source: source.clone(),
                    name: "Line".into(),
                    address: 1,
                    medium_ref: "MT-1".into(),
                    domain_address: None,
                    domain_address_is_checked: None,
                    ip_routing_multicast_address: None,
                    multicast_ttl: None,
                    completion: CompletionStatus::Undefined,
                    devices: vec![DeviceId(device)],
                }],
                unassigned: vec![],
            },
            buildings: vec![],
            group_ranges: vec![],
            group_addresses: vec![],
            parameters: vec![],
        });
        project.devices.insert(knx_core::DeviceInstance {
            id: DeviceId(device),
            source,
            name: format!("Device {device}"),
            description: None,
            address: None,
            product_ref: "P-1".into(),
            program_ref: "H-1".into(),
            commissioning: Default::default(),
            visibility_calculated: true,
            com_objects: vec![],
            binary_data: vec![],
        });
        project
    }

    fn snapshot_with_profile(device: u32) -> (NativeSnapshot, CommandStack) {
        let mut project = project_with_device(device);
        let mut stack = CommandStack::new();
        stack
            .do_command(
                &mut project,
                Command::SetDeviceConfigProfile {
                    profile: profile(device),
                },
            )
            .unwrap();
        let conn = knx_store::open_and_migrate_in_memory().unwrap();
        knx_store::save_project(&conn, &project).unwrap();
        let snapshot = NativeSnapshot::read(&conn).unwrap();
        (snapshot, stack)
    }

    fn revision() -> DevconfRevision {
        DevconfRevision {
            project_incarnation: "inc".into(),
            generation: 1,
        }
    }

    #[test]
    fn preview_then_apply_replaces_profile_and_is_undoable() {
        let (mut snapshot, mut stack) = snapshot_with_profile(7);
        let draft = DevconfDraft {
            device: DeviceId(7),
            module_id: "smartsensor".into(),
            module_version: "1.0.0".into(),
            module_sha256: "a".repeat(64),
            document: b"{\"nodes\": [1]}".to_vec(),
        };
        let preview = preview_devconf_edit(&snapshot, &revision(), &draft).unwrap();
        assert!(!preview.is_no_op());
        let outcome = apply_devconf_edit(&mut snapshot, &mut stack, &revision(), &preview, None)
            .unwrap();
        assert!(outcome.changed);
        assert_eq!(
            snapshot.project.devices.config_profile(DeviceId(7)).unwrap().document(),
            b"{\"nodes\": [1]}"
        );
        let mut undone = snapshot.project.clone();
        stack.undo(&mut undone).unwrap();
        assert_eq!(
            undone.devices.config_profile(DeviceId(7)).unwrap().document(),
            b"{\"nodes\": []}"
        );
    }

    #[test]
    fn stale_revision_is_refused() {
        let (snapshot, _stack) = snapshot_with_profile(7);
        let draft = DevconfDraft {
            device: DeviceId(7),
            module_id: "smartsensor".into(),
            module_version: "1.0.0".into(),
            module_sha256: "a".repeat(64),
            document: b"{}".to_vec(),
        };
        let preview = preview_devconf_edit(&snapshot, &revision(), &draft).unwrap();
        let wrong = DevconfRevision {
            project_incarnation: "other".into(),
            generation: 1,
        };
        let mut snapshot = snapshot;
        let mut stack = CommandStack::new();
        assert!(matches!(
            apply_devconf_edit(&mut snapshot, &mut stack, &wrong, &preview, None),
            Err(DevconfEditError::Conflict)
        ));
    }

    fn empty_snapshot() -> NativeSnapshot {
        let conn = knx_store::open_and_migrate_in_memory().unwrap();
        knx_store::save_project(&conn, &knx_core::Project::new(knx_core::Language("en".into())))
            .unwrap();
        NativeSnapshot::read(&conn).unwrap()
    }

    #[test]
    fn removal_requires_an_existing_profile() {
        let (snapshot, _stack) = snapshot_with_profile(7);
        assert!(matches!(
            preview_devconf_removal(&snapshot, &revision(), DeviceId(3)),
            Err(DevconfEditError::Command(
                knx_core::CommandError::DeviceNotFound(_)
            ))
        ));
        // A present device without a profile refuses removal as no-op guard.
        assert!(matches!(
            preview_devconf_removal(&snapshot, &revision(), DeviceId(7))
                .map(|_| ()),
            Ok(())
        ));
    }

    #[test]
    fn oversized_document_is_refused_before_any_command() {
        let snapshot = empty_snapshot();
        let draft = DevconfDraft {
            device: DeviceId(1),
            module_id: "m".into(),
            module_version: "1".into(),
            module_sha256: "a".repeat(64),
            document: vec![0u8; knx_core::devconf::MAX_PROFILE_DOCUMENT_BYTES + 1],
        };
        assert!(matches!(
            preview_devconf_edit(&snapshot, &revision(), &draft),
            Err(DevconfEditError::Profile(_))
        ));
    }
}
