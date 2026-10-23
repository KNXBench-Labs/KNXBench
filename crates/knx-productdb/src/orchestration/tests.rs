//! Tests of the preview/apply orchestration against schema-28 synthetic data.

use super::*;
use crate::catalogue_copy::{ArtifactInventory, DestinationRole, FreshDestination};
use crate::controlled::{ControlledPolicy, ControlledSource};
use crate::managed_catalogue::{LegacyMutationKind, LegacySettlementHint, ManagedCatalogue};
use crate::operation::PublicationGate;
use std::io::Write;

mod live_growth;
mod reopened_copy;
mod reopened_copy_controls;
mod reopened_copy_matrix;
mod review_regressions;
mod second_rehearsal;
mod writer_configuration;

const MASTER: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/11"><MasterData><Manufacturers><Manufacturer Id="M-0001" Name="Example"/></Manufacturers></MasterData></KNX>"#;
const HARDWARE: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0001"><Hardware><Hardware Id="H-1" Name="Example"/></Hardware></Manufacturer></ManufacturerData></KNX>"#;

fn archive(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for (path, bytes) in entries {
        writer
            .start_file(*path, zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn valid_package() -> Vec<u8> {
    archive(&[
        ("knx_master.xml", MASTER),
        ("M-0001/Hardware.xml", HARDWARE),
    ])
}

/// A real, migrated schema-28 product DB with an empty catalogue.
fn migrated_owner() -> (tempfile::TempDir, ManagedCatalogue) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let conn = crate::open_and_migrate(&path).unwrap();
    let owner = ManagedCatalogue::adopt_opened(conn, path).unwrap();
    (dir, owner)
}

/// A test artifact owner that mints fresh, equal-page-size connections in a
/// scratch directory (the server's `ArtifactRoot` implements the same seam).
struct TestArtifacts {
    dir: tempfile::TempDir,
    counter: usize,
    inventory: ArtifactInventory,
}

impl TestArtifacts {
    fn new() -> Self {
        Self {
            dir: tempfile::tempdir().unwrap(),
            counter: 0,
            inventory: ArtifactInventory::default(),
        }
    }

    fn fresh(
        &mut self,
        page_size: i64,
        role: DestinationRole,
    ) -> Result<FreshDestination, OrchestrationError> {
        let path = self
            .dir
            .path()
            .join(format!("dest-{}.sqlite", self.counter));
        self.counter += 1;
        Ok(self.inventory.create_new(role, path, page_size)?)
    }
}

impl CatalogueArtifacts for TestArtifacts {
    fn inventory(&mut self) -> &mut ArtifactInventory {
        &mut self.inventory
    }
    fn fresh_rehearsal(&mut self, page_size: i64) -> Result<FreshDestination, OrchestrationError> {
        self.fresh(page_size, DestinationRole::Rehearsal)
    }

    fn fresh_recovery(&mut self, page_size: i64) -> Result<FreshDestination, OrchestrationError> {
        self.fresh(page_size, DestinationRole::Recovery)
    }
}

fn admit<'a>(name: &'a str, bytes: &'a [u8]) -> ControlledSource<'a> {
    ControlledSource::admit(name, bytes, ControlledPolicy::Standard).unwrap()
}

fn package_count(owner: &ManagedCatalogue) -> i64 {
    owner
        .with_read(|conn| -> Result<i64, rusqlite::Error> {
            conn.query_row("SELECT count(*) FROM package", [], |r| r.get(0))
        })
        .unwrap()
        .unwrap()
}

#[test]
fn preview_rehearses_without_mutating_the_live_catalogue() {
    let (_dir, mut owner) = migrated_owner();
    let gate = PublicationGate::new();
    let before = owner.capture_stamp(&gate).unwrap();

    let bytes = valid_package();
    let source = admit("manufacturer.knxprod", &bytes);
    let op = OperationContext {
        gate: &gate,
        policy: ControlledPolicy::Standard,
    };
    let mut artifacts = TestArtifacts::new();
    let result = owner
        .preview_current_controlled(source, &op, &mut artifacts)
        .unwrap();

    // The rehearsal never touches W: generation, data_version and the full
    // witness are unchanged, and the owner stays healthy (proved by the
    // `with_read` in `package_count`).
    assert_eq!(result.binding.catalogue, before);
    assert_eq!(
        result.binding.version,
        crate::preview_binding::BINDING_VERSION
    );
    assert_eq!(result.binding.preview.package_count, 1);
    assert_eq!(package_count(&owner), 0);
    assert_eq!(
        owner
            .capture_stamp(&PublicationGate::new())
            .unwrap()
            .generation,
        0
    );
}

#[test]
fn preview_is_deterministic_for_identical_inputs() {
    let (_dir, mut owner) = migrated_owner();
    let bytes = valid_package();

    let gate = PublicationGate::new();
    let op = OperationContext {
        gate: &gate,
        policy: ControlledPolicy::Standard,
    };
    let source = admit("manufacturer.knxprod", &bytes);
    let mut first_artifacts = TestArtifacts::new();
    let first = owner
        .preview_current_controlled(source, &op, &mut first_artifacts)
        .unwrap();

    let gate = PublicationGate::new();
    let op = OperationContext {
        gate: &gate,
        policy: ControlledPolicy::Standard,
    };
    let source = admit("manufacturer.knxprod", &bytes);
    let mut second_artifacts = TestArtifacts::new();
    let second = owner
        .preview_current_controlled(source, &op, &mut second_artifacts)
        .unwrap();

    assert_eq!(first.binding, second.binding);
}

#[test]
fn preview_refuses_a_policy_mismatch() {
    let (_dir, mut owner) = migrated_owner();
    let bytes = valid_package();
    let source = admit("manufacturer.knxprod", &bytes);
    let gate = PublicationGate::new();
    let op = OperationContext {
        gate: &gate,
        policy: ControlledPolicy::Large,
    };
    let mut artifacts = TestArtifacts::new();
    let err = owner
        .preview_current_controlled(source, &op, &mut artifacts)
        .unwrap_err();
    assert!(matches!(err, OrchestrationError::PolicyMismatch));
}

#[test]
fn preview_refuses_a_precancelled_gate() {
    let (_dir, mut owner) = migrated_owner();
    let bytes = valid_package();
    let source = admit("manufacturer.knxprod", &bytes);
    let gate = PublicationGate::new();
    assert_eq!(gate.try_cancel(), crate::operation::CancelOutcome::Accepted);
    let op = OperationContext {
        gate: &gate,
        policy: ControlledPolicy::Standard,
    };
    let mut artifacts = TestArtifacts::new();
    let err = owner
        .preview_current_controlled(source, &op, &mut artifacts)
        .unwrap_err();
    assert!(matches!(err, OrchestrationError::Stop(_)));
}

#[test]
fn a_committed_catalogue_change_produces_a_different_binding() {
    let (_dir, mut owner) = migrated_owner();
    let bytes = valid_package();

    let gate = PublicationGate::new();
    let op = OperationContext {
        gate: &gate,
        policy: ControlledPolicy::Standard,
    };
    let source = admit("manufacturer.knxprod", &bytes);
    let mut artifacts = TestArtifacts::new();
    let first = owner
        .preview_current_controlled(source, &op, &mut artifacts)
        .unwrap();

    owner
        .with_legacy_mutation(
            LegacyMutationKind::DirectPackage,
            |conn| {
                conn.execute(
                    "INSERT INTO manufacturer (id, name) VALUES ('m1', 'Example Co')",
                    [],
                )?;
                Ok::<_, rusqlite::Error>(())
            },
            |_| LegacySettlementHint::SuccessfulReturn,
        )
        .unwrap()
        .unwrap();

    let gate = PublicationGate::new();
    let op = OperationContext {
        gate: &gate,
        policy: ControlledPolicy::Standard,
    };
    let source = admit("manufacturer.knxprod", &bytes);
    let mut artifacts = TestArtifacts::new();
    let second = owner
        .preview_current_controlled(source, &op, &mut artifacts)
        .unwrap();

    assert_ne!(first.binding, second.binding);
}

#[test]
fn apply_matching_preview_commits_and_installs_once() {
    let (_dir, mut owner) = migrated_owner();

    // Preview to obtain the retained binding.
    let bytes = valid_package();
    let gate = PublicationGate::new();
    let op = OperationContext {
        gate: &gate,
        policy: ControlledPolicy::Standard,
    };
    let source = admit("manufacturer.knxprod", &bytes);
    let mut artifacts = TestArtifacts::new();
    let preview = owner
        .preview_current_controlled(source, &op, &mut artifacts)
        .unwrap();

    // Apply the same source + binding.
    let gate = PublicationGate::new();
    let op = OperationContext {
        gate: &gate,
        policy: ControlledPolicy::Standard,
    };
    let source = admit("manufacturer.knxprod", &bytes);
    let mut artifacts = TestArtifacts::new();
    let settlement = owner.apply_current_controlled(source, &preview.binding, &op, &mut artifacts);

    assert!(settlement.failure.is_none(), "{:?}", settlement.failure);
    assert_eq!(settlement.commit, CommitEvidence::Committed);
    assert_eq!(settlement.health, WriterHealth::Healthy);
    assert_eq!(settlement.artifacts, ArtifactDisposition::Retain);
    assert_eq!(package_count(&owner), 1);
    assert_eq!(
        owner
            .capture_stamp(&PublicationGate::new())
            .unwrap()
            .generation,
        1
    );
}

#[test]
fn apply_refuses_a_source_mismatch_without_writing() {
    let (_dir, mut owner) = migrated_owner();

    let bytes = valid_package();
    let gate = PublicationGate::new();
    let op = OperationContext {
        gate: &gate,
        policy: ControlledPolicy::Standard,
    };
    let source = admit("manufacturer.knxprod", &bytes);
    let mut artifacts = TestArtifacts::new();
    let preview = owner
        .preview_current_controlled(source, &op, &mut artifacts)
        .unwrap();

    // A different source than the shown binding.
    let other = valid_package();
    let gate = PublicationGate::new();
    let op = OperationContext {
        gate: &gate,
        policy: ControlledPolicy::Standard,
    };
    let other_source = admit("different.knxprod", &other);
    let mut artifacts = TestArtifacts::new();
    let settlement =
        owner.apply_current_controlled(other_source, &preview.binding, &op, &mut artifacts);

    assert!(matches!(
        settlement.failure,
        Some(OrchestrationError::SourceMismatch)
    ));
    assert_eq!(settlement.commit, CommitEvidence::NotCommitted);
    assert_eq!(settlement.artifacts, ArtifactDisposition::None);
    assert_eq!(package_count(&owner), 0);
    assert_eq!(
        owner
            .capture_stamp(&PublicationGate::new())
            .unwrap()
            .generation,
        0
    );
}

#[test]
fn apply_refuses_a_stale_catalogue_without_writing() {
    let (_dir, mut owner) = migrated_owner();

    let bytes = valid_package();
    let gate = PublicationGate::new();
    let op = OperationContext {
        gate: &gate,
        policy: ControlledPolicy::Standard,
    };
    let source = admit("manufacturer.knxprod", &bytes);
    let mut artifacts = TestArtifacts::new();
    let preview = owner
        .preview_current_controlled(source, &op, &mut artifacts)
        .unwrap();

    // A committed catalogue change after the preview stales the binding.
    owner
        .with_legacy_mutation(
            LegacyMutationKind::DirectPackage,
            |conn| {
                conn.execute(
                    "INSERT INTO manufacturer (id, name) VALUES ('m1', 'Example Co')",
                    [],
                )?;
                Ok::<_, rusqlite::Error>(())
            },
            |_| LegacySettlementHint::SuccessfulReturn,
        )
        .unwrap()
        .unwrap();

    let gate = PublicationGate::new();
    let op = OperationContext {
        gate: &gate,
        policy: ControlledPolicy::Standard,
    };
    let source = admit("manufacturer.knxprod", &bytes);
    let mut artifacts = TestArtifacts::new();
    let settlement = owner.apply_current_controlled(source, &preview.binding, &op, &mut artifacts);

    assert!(matches!(
        settlement.failure,
        Some(OrchestrationError::Stale)
    ));
    assert_eq!(settlement.commit, CommitEvidence::NotCommitted);
    assert_eq!(settlement.artifacts, ArtifactDisposition::None);
    assert_eq!(package_count(&owner), 0);
}

#[test]
fn apply_refuses_a_precancelled_gate_without_writing() {
    let (_dir, mut owner) = migrated_owner();

    let bytes = valid_package();
    let gate = PublicationGate::new();
    let op = OperationContext {
        gate: &gate,
        policy: ControlledPolicy::Standard,
    };
    let source = admit("manufacturer.knxprod", &bytes);
    let mut artifacts = TestArtifacts::new();
    let preview = owner
        .preview_current_controlled(source, &op, &mut artifacts)
        .unwrap();

    let gate = PublicationGate::new();
    assert_eq!(gate.try_cancel(), crate::operation::CancelOutcome::Accepted);
    let op = OperationContext {
        gate: &gate,
        policy: ControlledPolicy::Standard,
    };
    let source = admit("manufacturer.knxprod", &bytes);
    let mut artifacts = TestArtifacts::new();
    let settlement = owner.apply_current_controlled(source, &preview.binding, &op, &mut artifacts);

    assert!(matches!(
        settlement.failure,
        Some(OrchestrationError::Stop(_))
    ));
    assert_eq!(settlement.commit, CommitEvidence::NotCommitted);
    assert_eq!(settlement.artifacts, ArtifactDisposition::None);
    assert_eq!(package_count(&owner), 0);
    assert_eq!(
        owner
            .capture_stamp(&PublicationGate::new())
            .unwrap()
            .generation,
        0
    );
}

#[test]
fn preview_and_apply_share_the_same_rehearsal_identity() {
    let (_dir, mut owner) = migrated_owner();
    let bytes = valid_package();

    let gate = PublicationGate::new();
    let op = OperationContext {
        gate: &gate,
        policy: ControlledPolicy::Standard,
    };
    let source = admit("manufacturer.knxprod", &bytes);
    let mut artifacts = TestArtifacts::new();
    let preview = owner
        .preview_current_controlled(source, &op, &mut artifacts)
        .unwrap();

    // Recompute the identity from the retained full install result: it must
    // equal the binding's retained identity exactly.
    let recomputed = crate::preview_identity::preview_identity(&preview.preview.install).unwrap();
    assert_eq!(recomputed, preview.binding.preview);
}
