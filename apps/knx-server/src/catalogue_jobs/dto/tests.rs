//! Tests for the strict serde wire DTO layer (P2/P3).

use super::*;

fn u64(v: u64) -> U64 {
    U64(v)
}

fn id(byte: u8) -> JobId {
    JobId::from_bytes([byte; 16])
}

fn inc(byte: u8) -> Incarnation {
    Incarnation::from_bytes([byte; 16])
}

fn sha(byte: u8) -> Sha256 {
    Sha256::from_bytes([byte; 32])
}

fn std_source() -> Source {
    Source {
        name: "sample.knxprod".to_string(),
        declared_bytes: u64(128),
        received_bytes: u64(128),
        sha256: Some(sha(0x11)),
    }
}

fn std_profile() -> Profile {
    Profile {
        name: ProfileName::Standard,
        max_member_bytes: u64(MAX_MEMBER_BYTES),
        max_expanded_bytes: u64(STANDARD_EXPANDED_BYTES),
    }
}

fn pending_commit() -> Commit {
    Commit {
        outcome: CommitOutcome::Pending,
        evidence: CommitEvidence::NotAttempted,
    }
}

fn pending_cleanup() -> Cleanup {
    Cleanup {
        disposition: CleanupDisposition::Pending,
        failure_kind: None,
    }
}

fn no_backup() -> Backup {
    Backup {
        state: BackupState::None,
        name: None,
        identity: None,
        publication: BackupPublication::None,
    }
}

fn base_status() -> Status {
    Status {
        schema_version: SCHEMA_VERSION,
        policy_version: POLICY_VERSION.to_string(),
        handle: Handle {
            server_incarnation: inc(0x01),
            admission_serial: u64(1),
            job_id: id(0x02),
        },
        revision: u64(1),
        source: std_source(),
        profile: std_profile(),
        state: JobState::Running,
        phase: Some(Phase::Receiving),
        gate: Gate::Open,
        stop_cause: None,
        progress: None,
        lease: Lease {
            kind: LeaseKind::Receiving,
            deadline_tick_ms: u64(1000),
        },
        apply: Apply {
            state: ApplyState::Unavailable,
            accepted_command_id: None,
        },
        preview: None,
        result: None,
        failure: None,
        commit: pending_commit(),
        cleanup: pending_cleanup(),
        backup: no_backup(),
    }
}

// ---------------------------------------------------------------------------
// Strict JSON admission
// ---------------------------------------------------------------------------

#[test]
fn rejects_duplicate_keys_at_top_level() {
    let json = r#"{"schemaVersion":1,"schemaVersion":1,"serverIncarnation":"00000000000000000000000000000001","nextAdmissionSerial":"1","serverTickMs":"0","admission":"open"}"#;
    assert_eq!(from_str::<Admission>(json), Err(WireError::DuplicateKey));
}

#[test]
fn rejects_duplicate_keys_nested() {
    let json = r#"{"schemaVersion":1,"handle":{"serverIncarnation":"00000000000000000000000000000001","admissionSerial":"1","jobId":"00000000000000000000000000000002","jobId":"00000000000000000000000000000003"},"revision":"1"}"#;
    assert_eq!(from_str::<Status>(json), Err(WireError::DuplicateKey));
}

#[test]
fn rejects_unknown_field() {
    let json = r#"{"schemaVersion":1,"policyVersion":"wp043-v1","serverIncarnation":"00000000000000000000000000000001","nextAdmissionSerial":"1","serverTickMs":"0","admission":"open","extra":true}"#;
    assert_eq!(from_str::<Admission>(json), Err(WireError::Schema));
}

#[test]
fn rejects_leading_bom() {
    let json = "\u{feff}{\"schemaVersion\":1}";
    assert_eq!(from_str::<Admission>(json), Err(WireError::InvalidJson));
}

#[test]
fn rejects_trailing_content() {
    let json = "{\"schemaVersion\":1,\"policyVersion\":\"wp043-v1\",\"serverIncarnation\":\"00000000000000000000000000000001\",\"nextAdmissionSerial\":\"1\",\"serverTickMs\":\"0\",\"admission\":\"open\"} 42";
    assert_eq!(from_str::<Admission>(json), Err(WireError::InvalidJson));
}

#[test]
fn rejects_nesting_deeper_than_eight() {
    // 10 nested arrays.
    let json = format!("{}0{}", "[".repeat(10), "]".repeat(10));
    let value: serde_json::Value = json.parse().unwrap();
    let _ = value; // valid JSON, but over the admitted depth
    let err = reject_duplicate_keys(&json);
    assert_eq!(err, Err(WireError::DuplicateKey));
}

#[test]
fn rejects_numeric_u64() {
    let json = r#"{"schemaVersion":1,"policyVersion":"wp043-v1","serverIncarnation":"00000000000000000000000000000001","nextAdmissionSerial":1,"serverTickMs":"0","admission":"open"}"#;
    assert_eq!(from_str::<Admission>(json), Err(WireError::Schema));
}

#[test]
fn rejects_overlong_u64() {
    let json = r#"{"schemaVersion":1,"policyVersion":"wp043-v1","serverIncarnation":"00000000000000000000000000000001","nextAdmissionSerial":"18446744073709551616","serverTickMs":"0","admission":"open"}"#;
    assert_eq!(from_str::<Admission>(json), Err(WireError::Schema));
}

#[test]
fn rejects_uppercase_hex_identity() {
    let json = r#"{"schemaVersion":1,"policyVersion":"wp043-v1","serverIncarnation":"0000000000000000000000000000000A","nextAdmissionSerial":"1","serverTickMs":"0","admission":"open"}"#;
    // "A" is uppercase; the identity grammar is lowercase hex only.
    assert!(from_str::<Admission>(json).is_err());
}

#[test]
fn rejects_unknown_enum_token() {
    let json = r#"{"schemaVersion":1,"policyVersion":"wp043-v1","serverIncarnation":"00000000000000000000000000000001","nextAdmissionSerial":"1","serverTickMs":"0","admission":"wideOpen"}"#;
    assert_eq!(from_str::<Admission>(json), Err(WireError::Schema));
}

// ---------------------------------------------------------------------------
// Round-trips
// ---------------------------------------------------------------------------

#[test]
fn u64_serializes_as_decimal_string() {
    assert_eq!(to_string(&u64(42)).unwrap(), "\"42\"");
    assert_eq!(to_string(&u64(0)).unwrap(), "\"0\"");
    assert_eq!(
        to_string(&u64(u64::MAX)).unwrap(),
        "\"18446744073709551615\""
    );
}

#[test]
fn identities_serialize_as_hex_strings() {
    assert_eq!(
        to_string(&id(0xab)).unwrap(),
        "\"abababababababababababababababab\""
    );
    assert_eq!(
        to_string(&sha(0x5a)).unwrap(),
        format!("\"{}\"", "5a".repeat(32))
    );
}

#[test]
fn admission_roundtrips() {
    let admission = Admission {
        schema_version: SCHEMA_VERSION,
        policy_version: POLICY_VERSION.to_string(),
        server_incarnation: inc(0x0c),
        next_admission_serial: u64(7),
        server_tick_ms: u64(123456),
        admission: AdmissionState::Open,
    };
    let wire = to_string(&admission).unwrap();
    assert_eq!(from_str::<Admission>(&wire).unwrap(), admission);
}

#[test]
fn status_roundtrips_receiving() {
    let status = base_status();
    let wire = to_string(&status).unwrap();
    let back = from_str::<Status>(&wire).unwrap();
    assert_eq!(back, status);
    assert!(status.validate().is_ok());
}

// ---------------------------------------------------------------------------
// Const fields
// ---------------------------------------------------------------------------

#[test]
fn create_rejects_wrong_schema_version() {
    let mut create = valid_create();
    create.schema_version = 2;
    assert_eq!(create.validate(), Err(WireError::Const));
}

#[test]
fn create_rejects_wrong_policy_version() {
    let mut create = valid_create();
    create.policy_version = "wp043-v2".to_string();
    assert_eq!(create.validate(), Err(WireError::Const));
}

#[test]
fn apply_command_rejects_unconfirmed() {
    let mut cmd = valid_apply_command();
    cmd.confirmed = false;
    assert_eq!(cmd.validate(), Err(WireError::Const));
}

#[test]
fn preview_rejects_growth_cliff_false() {
    let mut preview = valid_preview();
    preview.growth_cliff_possible = false;
    assert_eq!(preview.validate(), Err(WireError::Const));
}

// ---------------------------------------------------------------------------
// Source name grammar
// ---------------------------------------------------------------------------

#[test]
fn create_rejects_bad_source_names() {
    for bad in [
        "",
        ".",
        "..",
        " leading",
        "trailing ",
        "a/b",
        "a\\b",
        "a:b",
        "a\0b",
        "a\nb",
    ] {
        let mut create = valid_create();
        create.source_name = bad.to_string();
        assert_eq!(
            create.validate(),
            Err(WireError::SourceName),
            "name {bad:?}"
        );
    }
    let too_long = "a".repeat(513);
    let mut create = valid_create();
    create.source_name = too_long;
    assert_eq!(create.validate(), Err(WireError::SourceName));
}

#[test]
fn create_accepts_plain_basename() {
    let create = valid_create();
    assert!(create.validate().is_ok());
}

// ---------------------------------------------------------------------------
// Profile consistency and source bytes
// ---------------------------------------------------------------------------

#[test]
fn create_rejects_large_package_mismatch() {
    let mut create = valid_create();
    create.profile = ProfileName::Large;
    create.allow_large_package = false;
    assert_eq!(create.validate(), Err(WireError::Profile));

    create.allow_large_package = true;
    assert!(create.validate().is_ok());
}

#[test]
fn create_rejects_standard_with_large_flag() {
    let mut create = valid_create();
    create.allow_large_package = true; // standard must be false
    assert_eq!(create.validate(), Err(WireError::Profile));
}

#[test]
fn source_rejects_received_over_declared() {
    let mut source = std_source();
    source.received_bytes = u64(129);
    assert_eq!(source.validate(), Err(WireError::SourceBytes));
}

#[test]
fn source_rejects_sealed_short_received() {
    let mut source = std_source();
    source.received_bytes = u64(127);
    assert_eq!(source.validate(), Err(WireError::SourceBytes));
}

#[test]
fn profile_rejects_wrong_expanded_bytes() {
    let mut profile = std_profile();
    profile.max_expanded_bytes = u64(LARGE_EXPANDED_BYTES);
    assert_eq!(profile.validate(), Err(WireError::Profile));
}

// ---------------------------------------------------------------------------
// Commit / cleanup / backup couplings
// ---------------------------------------------------------------------------

#[test]
fn commit_rejects_pending_with_wrong_evidence() {
    let mut commit = pending_commit();
    commit.evidence = CommitEvidence::CommitReturnedOk;
    assert_eq!(commit.validate(), Err(WireError::State));
}

#[test]
fn commit_accepts_committed_with_returned_ok() {
    let commit = Commit {
        outcome: CommitOutcome::Committed,
        evidence: CommitEvidence::CommitReturnedOk,
    };
    assert!(commit.validate().is_ok());
}

#[test]
fn cleanup_requires_failure_kind_only_when_quarantined() {
    assert!(pending_cleanup().validate().is_ok());
    let mut quarantined = pending_cleanup();
    quarantined.disposition = CleanupDisposition::Quarantined;
    assert_eq!(quarantined.validate(), Err(WireError::State));
    quarantined.failure_kind = Some(CleanupFailureKind::IoFailure);
    assert!(quarantined.validate().is_ok());
}

#[test]
fn backup_staging_has_null_public_identity() {
    let mut backup = no_backup();
    backup.state = BackupState::Staging;
    assert!(backup.validate().is_ok());

    backup.state = BackupState::Published;
    assert_eq!(backup.validate(), Err(WireError::State));

    backup.name = Some("db.backup-1-abc.sqlite".to_string());
    backup.identity = Some("0000000000000000000000000000000a".to_string());
    backup.publication = BackupPublication::FileAndDirectorySyncAcknowledged;
    assert!(backup.validate().is_ok());
}

// ---------------------------------------------------------------------------
// Terminal state contracts
// ---------------------------------------------------------------------------

#[test]
fn status_rejects_stop_cause_without_cancel_accepted() {
    let mut status = base_status();
    status.stop_cause = Some(StopCause::UserCancel);
    assert_eq!(status.validate(), Err(WireError::State));
}

#[test]
fn status_rejects_cancel_accepted_without_stop_cause() {
    let mut status = base_status();
    status.gate = Gate::CancelAccepted;
    assert_eq!(status.validate(), Err(WireError::State));
}

#[test]
fn succeeded_is_valid() {
    let status = valid_succeeded();
    assert!(status.validate().is_ok());
}

#[test]
fn succeeded_rejects_result_missing() {
    let mut status = valid_succeeded();
    status.result = None;
    assert_eq!(status.validate(), Err(WireError::State));
}

#[test]
fn cancelled_is_valid_and_discriminated() {
    let status = valid_cancelled();
    assert!(status.validate().is_ok());

    let mut wrong = valid_cancelled();
    wrong.failure = Some(Failure {
        kind: FailureKind::CatalogueUpdateStale,
        context: None,
    });
    assert_eq!(wrong.validate(), Err(WireError::State));
}

#[test]
fn stale_is_valid_and_discriminated() {
    let status = valid_stale();
    assert!(status.validate().is_ok());

    let mut wrong = valid_stale();
    wrong.backup.state = BackupState::Retained;
    assert_eq!(wrong.validate(), Err(WireError::State));
}

#[test]
fn expired_is_valid() {
    let status = valid_expired();
    assert!(status.validate().is_ok());
}

#[test]
fn failed_is_valid_and_requires_failure() {
    let status = valid_failed();
    assert!(status.validate().is_ok());

    let mut missing = valid_failed();
    missing.failure = None;
    assert_eq!(missing.validate(), Err(WireError::State));
}

// ---------------------------------------------------------------------------
// Error mapping
// ---------------------------------------------------------------------------

#[test]
fn error_kinds_map_to_fixed_statuses() {
    assert_eq!(FailureKind::InvalidRequest.http_status(), 400);
    assert_eq!(FailureKind::Unauthenticated.http_status(), 401);
    assert_eq!(FailureKind::BodyLimit.http_status(), 413);
    assert_eq!(FailureKind::UnsupportedMediaType.http_status(), 415);
    assert_eq!(FailureKind::OutcomeUnavailable.http_status(), 410);
    assert_eq!(FailureKind::CatalogueAdmissionLimit.http_status(), 422);
    assert_eq!(FailureKind::IoFailure.http_status(), 500);
    assert_eq!(FailureKind::ServerStop.http_status(), 503);
    assert_eq!(FailureKind::WorkingTimeout.http_status(), 408);
}

#[test]
fn error_kinds_carry_fixed_messages() {
    assert_eq!(
        FailureKind::OutcomeUnavailable.message(),
        "The retained outcome is unavailable; do not retry installation automatically."
    );
    assert_eq!(
        FailureKind::InvalidRequest.message(),
        "The request is invalid."
    );
}

// ---------------------------------------------------------------------------
// Helpers building valid fixtures
// ---------------------------------------------------------------------------

fn valid_create() -> Create {
    Create {
        schema_version: SCHEMA_VERSION,
        policy_version: POLICY_VERSION.to_string(),
        server_incarnation: inc(0x01),
        admission_serial: u64(1),
        source_name: "sample.knxprod".to_string(),
        declared_bytes: u64(128),
        profile: ProfileName::Standard,
        allow_large_package: false,
    }
}

fn valid_apply_command() -> ApplyCommand {
    ApplyCommand {
        schema_version: SCHEMA_VERSION,
        command_id: id(0xaa),
        consent_id: id(0xbb),
        preview_artifact_id: id(0xcc),
        preview_public_sha256: sha(0xdd),
        confirmed: true,
    }
}

fn valid_artifact(kind: ArtifactKind) -> Artifact {
    Artifact {
        artifact_id: id(0x30),
        kind,
        public_sha256: sha(0x31),
        public_bytes: u64(4096),
        records: u64(12),
        withheld_fields: u64(0),
        withheld_records: u64(0),
        withheld_occurrences: u64(0),
        projection: Projection::RedactedCompleteV1,
    }
}

fn valid_preview() -> Preview {
    Preview {
        consent_id: id(0x40),
        artifact: valid_artifact(ArtifactKind::Preview),
        package_already_installed: false,
        wrapper: false,
        packages: u64(1),
        programs: u64(3),
        products_added: u64(2),
        hardware2_programs_added: u64(0),
        id_conflicts: u64(0),
        growth_cliff_possible: true,
    }
}

fn valid_result() -> JobResult {
    JobResult {
        artifact: valid_artifact(ArtifactKind::Result),
        catalogue_page_bytes: u64(1_000_000),
        next_job_input_limit_exceeded: false,
    }
}

fn terminal_status() -> Status {
    let mut status = base_status();
    status.phase = None;
    status.progress = None;
    status.lease.kind = LeaseKind::Retention;
    status.apply.state = ApplyState::Spent;
    status.apply.accepted_command_id = Some(id(0xaa));
    status
}

fn valid_succeeded() -> Status {
    let mut status = terminal_status();
    status.state = JobState::Succeeded;
    status.gate = Gate::CommitClaimed;
    status.commit = Commit {
        outcome: CommitOutcome::Committed,
        evidence: CommitEvidence::CommitReturnedOk,
    };
    status.cleanup = Cleanup {
        disposition: CleanupDisposition::Complete,
        failure_kind: None,
    };
    status.backup.state = BackupState::Retained;
    status.backup.name = Some("db.backup-1-abc.sqlite".to_string());
    status.backup.identity = Some("0000000000000000000000000000000a".to_string());
    status.backup.publication = BackupPublication::FileAndDirectorySyncAcknowledged;
    status.preview = Some(valid_preview());
    status.result = Some(valid_result());
    status.failure = None;
    status
}

fn valid_cancelled() -> Status {
    let mut status = terminal_status();
    status.state = JobState::Cancelled;
    status.gate = Gate::CancelAccepted;
    status.stop_cause = Some(StopCause::UserCancel);
    status.commit = Commit {
        outcome: CommitOutcome::NotCommitted,
        evidence: CommitEvidence::ExplicitRollback,
    };
    status.cleanup = Cleanup {
        disposition: CleanupDisposition::Complete,
        failure_kind: None,
    };
    status.failure = Some(Failure {
        kind: FailureKind::Cancelled,
        context: None,
    });
    status
}

fn valid_stale() -> Status {
    let mut status = terminal_status();
    status.state = JobState::Stale;
    status.gate = Gate::Open;
    status.commit = Commit {
        outcome: CommitOutcome::NotCommitted,
        evidence: CommitEvidence::NoLiveMutation,
    };
    status.cleanup = Cleanup {
        disposition: CleanupDisposition::Complete,
        failure_kind: None,
    };
    status.preview = Some(valid_preview());
    status.failure = Some(Failure {
        kind: FailureKind::CatalogueUpdateStale,
        context: None,
    });
    status
}

fn valid_expired() -> Status {
    let mut status = terminal_status();
    status.state = JobState::Expired;
    status.gate = Gate::CancelAccepted;
    status.stop_cause = Some(StopCause::ReadyExpired);
    status.commit = Commit {
        outcome: CommitOutcome::NotCommitted,
        evidence: CommitEvidence::AutomaticRollbackBeforeCommit,
    };
    status.cleanup = Cleanup {
        disposition: CleanupDisposition::Complete,
        failure_kind: None,
    };
    status.failure = Some(Failure {
        kind: FailureKind::ReadyExpired,
        context: None,
    });
    status
}

fn valid_failed() -> Status {
    let mut status = terminal_status();
    status.state = JobState::Failed;
    status.gate = Gate::Open;
    status.commit = Commit {
        outcome: CommitOutcome::NotCommitted,
        evidence: CommitEvidence::AutomaticRollbackBeforeCommit,
    };
    status.cleanup = Cleanup {
        disposition: CleanupDisposition::Complete,
        failure_kind: None,
    };
    status.failure = Some(Failure {
        kind: FailureKind::PackageRefused,
        context: None,
    });
    status
}
