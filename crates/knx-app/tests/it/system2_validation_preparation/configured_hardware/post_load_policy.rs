//! Revision-3 post-load octet policy: a firmware-cleared download flag on simulated hardware.
use super::pid7_width::revision1;
use super::*;
use knx_app::property_validation::{
    Checkpoint, FinalStepCategory, FinalStepStage, PointerObservationRun, PostLoadOctetProgress,
    PostLoadOctetRecord,
};
use knx_net::commissioning::post_load_octets::{PostLoadOctet, PostLoadOctetAdmission};

/// The UP258 hidden `Download_Flag` octet the revision-3 policy declares.
const FLAG: u16 = 0x3CF4;

/// Hardware-kind transport whose device firmware changes one octet when object 3 reports
/// Load Completed, the way the reporter's UP258 cleared its download flag.
struct FirmwareChanges<'a> {
    device: &'a SimulatedDevice,
    octet: u16,
    value: u8,
}
impl ManagementTransport for FirmwareChanges<'_> {
    fn assigned_address(&self) -> IndividualAddress {
        ManagementTransport::assigned_address(self.device)
    }
    fn subscribe(&self) -> broadcast::Receiver<TunnelEvent> {
        ManagementTransport::subscribe(self.device)
    }
    fn target_kind(&self) -> TargetKind {
        TargetKind::Hardware
    }
    async fn send_frame(
        &self,
        destination: Destination,
        transport: Tpci,
        service: ApplicationService,
    ) -> Result<(), BusError> {
        let completed = matches!(
            &service,
            ApplicationService::PropertyValueWrite {
                object_index: 3,
                property_id: 5,
                data,
                ..
            } if data.first() == Some(&2)
        );
        let sent =
            ManagementTransport::send_frame(self.device, destination, transport, service).await;
        if completed {
            self.device
                .preset_memory(u32::from(self.octet), &[self.value]);
        }
        sent
    }
}

struct Prepared {
    _dir: tempfile::TempDir,
    device: SimulatedDevice,
    bundle: PreparedBundle,
    conn: knx_productdb::Connection,
    project: knx_core::Project,
}

async fn prepare(flag: bool) -> Prepared {
    let (dir, conn, project, profile) = if flag {
        super::super::fixture::configured_flag_inputs()
    } else {
        super::super::fixture::configured_inputs()
    };
    let device = old_device(&conn, &project, &profile);
    let source = prepare_project_hardware_source(
        &conn,
        &project,
        knx_core::DeviceId(1),
        &profile,
        "post-load-test",
        RecoveryTrack::KnxBenchBestEffort,
    )
    .unwrap();
    let cancel = AtomicBool::new(false);
    let bundle = observe_project_hardware(&Hardware(&device), source, integrated_timing(), &cancel)
        .await
        .unwrap();
    Prepared {
        _dir: dir,
        device,
        bundle,
        conn,
        project,
    }
}

/// The same observed record as revision 2: width policy kept, no post-load policy.
fn revision2(mut bundle: PreparedBundle) -> PreparedBundle {
    let record = bundle.hardware_preparation.as_mut().unwrap();
    record.revision = 2;
    record.post_load_octet_policy = None;
    bundle.binding.project_preparation_sha256 = Some(record.sha256().unwrap());
    bundle
}

async fn download(
    prepared: &Prepared,
    bundle: &PreparedBundle,
    transport: &FirmwareChanges<'_>,
    operation: &std::path::Path,
) -> (OperationStore, std::io::Result<()>) {
    let admission = admit_current_project_hardware(
        &prepared.conn,
        &prepared.project,
        knx_core::DeviceId(1),
        bundle,
    )
    .unwrap();
    let mut store = OperationStore::create(operation, bundle.binding.clone()).unwrap();
    let operation_phrase = bundle.consent_phrase();
    let hypothesis = bundle.acceptance_phrase();
    let download = required_confirmation_phrase(prepared.device.address(), WriteScope::Download);
    let release = knx_net::commissioning::hardware_release::required_release_phrase(
        prepared.device.address(),
    );
    let result = execute_project_hardware(
        transport,
        &admission,
        &mut store,
        integrated_timing(),
        &HardwareConfirmations {
            operation: &operation_phrase,
            hypothesis: &hypothesis,
            download: &download,
            risk: &release,
        },
        &AtomicBool::new(false),
    )
    .await;
    (store, result)
}

fn flag_record(prestate: Option<u8>) -> PostLoadOctetRecord {
    PostLoadOctetRecord {
        object: 3,
        address: FLAG,
        written: 1,
        observed: 0,
        prestate,
    }
}

#[tokio::test]
async fn revision3_tolerates_the_cleared_flag_journals_it_and_reopens_with_it() {
    let prepared = prepare(true).await;
    let bundle = &prepared.bundle;
    assert_eq!(bundle.hardware_preparation.as_ref().unwrap().revision, 3);
    assert_eq!(
        bundle.preconditions().unwrap().post_load_octets,
        PostLoadOctetAdmission::Declared(vec![PostLoadOctet {
            object: 3,
            address: FLAG,
            accepted: vec![0],
        }])
    );
    let transport = FirmwareChanges {
        device: &prepared.device,
        octet: FLAG,
        value: 0,
    };
    let dir = tempfile::tempdir().unwrap();
    let operation = dir.path().join("operation");
    let (store, result) = download(&prepared, bundle, &transport, &operation).await;
    assert!(result.is_ok(), "{result:?}");
    assert_eq!(store.outcome(), LocalOutcome::Finished);
    let expected = vec![PostLoadOctetProgress {
        run: PointerObservationRun::Download,
        record: flag_record(Some(0xa5)),
    }];
    let progress = store.progress();
    assert_eq!(progress.post_load_octets, expected);
    assert_eq!(
        progress.download.result,
        knx_app::property_validation::DownloadResult::Verified
    );
    assert!(progress.final_step_failures.is_empty());
    assert_eq!(prepared.device.memory(u32::from(FLAG), 1), [Some(0)]);
    drop(store);
    let reopened = OperationStore::open(&operation, &bundle.binding).unwrap();
    assert_eq!(reopened.progress().post_load_octets, expected);
    assert_eq!(reopened.outcome(), LocalOutcome::Finished);
}

#[tokio::test]
async fn an_unchanged_flag_records_nothing() {
    let prepared = prepare(true).await;
    // The firmware leaves the written value in place: nothing to tolerate or observe.
    let transport = FirmwareChanges {
        device: &prepared.device,
        octet: FLAG,
        value: 1,
    };
    let dir = tempfile::tempdir().unwrap();
    let (store, result) = download(
        &prepared,
        &prepared.bundle,
        &transport,
        &dir.path().join("op"),
    )
    .await;
    assert!(result.is_ok(), "{result:?}");
    assert!(store.progress().post_load_octets.is_empty());
}

async fn assert_strict_failure(
    bundle: &PreparedBundle,
    prepared: &Prepared,
    octet: u16,
    value: u8,
) {
    let transport = FirmwareChanges {
        device: &prepared.device,
        octet,
        value,
    };
    let dir = tempfile::tempdir().unwrap();
    let (store, result) = download(prepared, bundle, &transport, &dir.path().join("op")).await;
    assert!(result.is_err(), "octet {octet:04X}h value {value:02X}h");
    let progress = store.progress();
    assert_eq!(store.outcome(), LocalOutcome::Failed);
    assert!(progress.post_load_octets.is_empty());
    let failure = &progress.final_step_failures[0];
    assert_eq!(
        (failure.category, failure.stage),
        (
            FinalStepCategory::VerificationMismatch,
            FinalStepStage::Allocation
        )
    );
}

#[tokio::test]
async fn revision2_and_revision1_records_stay_strict_on_the_cleared_flag() {
    let prepared = prepare(true).await;
    for bundle in [
        revision2(prepared.bundle.clone()),
        revision1(prepared.bundle.clone()),
    ] {
        assert_eq!(
            bundle.preconditions().unwrap().post_load_octets,
            PostLoadOctetAdmission::Legacy
        );
        assert_strict_failure(&bundle, &prepared, FLAG, 0).await;
    }
}

#[tokio::test]
async fn another_value_or_another_octet_still_fails_verification() {
    let prepared = prepare(true).await;
    assert_strict_failure(&prepared.bundle, &prepared, FLAG, 2).await;
    assert_strict_failure(&prepared.bundle, &prepared, FLAG - 1, 0x5a).await;
}

#[tokio::test]
async fn a_plan_that_does_not_write_the_flag_declares_nothing() {
    let prepared = prepare(false).await;
    assert_eq!(
        prepared
            .bundle
            .hardware_preparation
            .as_ref()
            .unwrap()
            .revision,
        3
    );
    assert_eq!(
        prepared.bundle.preconditions().unwrap().post_load_octets,
        PostLoadOctetAdmission::Legacy
    );
    let preview =
        knx_app::system2_validation_preparation::configured_hardware_preview(&prepared.bundle)
            .unwrap();
    assert!(preview
        .warnings
        .iter()
        .all(|w| !w.starts_with("Firmware-maintained octet")));
}

#[tokio::test]
async fn source_and_observed_previews_announce_the_tolerated_octet_but_revision2_does_not() {
    let prepared = prepare(true).await;
    let announces = |warnings: &[String]| {
        warnings.iter().any(|w| {
            w.starts_with("Firmware-maintained octet")
                && w.contains("accepts 00h at object 3 octet 3CF4h")
                && w.contains("Every other octet stays strictly verified")
        })
    };
    let observed =
        knx_app::system2_validation_preparation::configured_hardware_preview(&prepared.bundle)
            .unwrap();
    assert!(announces(&observed.warnings), "{:?}", observed.warnings);
    let (_dir, conn, project, profile) = super::super::fixture::configured_flag_inputs();
    let source = prepare_project_hardware_source(
        &conn,
        &project,
        knx_core::DeviceId(1),
        &profile,
        "preview",
        RecoveryTrack::KnxBenchBestEffort,
    )
    .unwrap();
    assert!(announces(&source.preview_summary().unwrap().warnings));
    let older = knx_app::system2_validation_preparation::configured_hardware_preview(&revision2(
        prepared.bundle.clone(),
    ))
    .unwrap();
    assert!(!announces(&older.warnings));
}

#[tokio::test]
async fn tampered_policy_records_refuse_before_traffic() {
    let prepared = prepare(true).await;
    let bundle = serde_json::to_value(&prepared.bundle).unwrap();
    let policy = "/hardware_preparation/post_load_octet_policy";
    let revision = "/hardware_preparation/revision";
    let load = |v: serde_json::Value| serde_json::from_value::<PreparedBundle>(v);
    let refuses = |v: serde_json::Value| load(v).map_or(true, |b| b.plan().is_err());
    let mut v = bundle.clone();
    v.pointer_mut("/hardware_preparation")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .remove("post_load_octet_policy");
    assert!(refuses(v), "revision 3 without its post-load policy");
    let mut v = bundle.clone();
    *v.pointer_mut(policy).unwrap() =
        "up258-mv0025-object3-3cf4h-download-flag-firmware-cleared-any-v1".into();
    assert!(refuses(v), "wrong policy id");
    let mut v = bundle.clone();
    *v.pointer_mut(revision).unwrap() = 2.into();
    assert!(refuses(v), "revision 2 carrying a post-load policy");
    let mut v = bundle.clone();
    *v.pointer_mut(policy).unwrap() = serde_json::Value::Null;
    assert!(load(v).is_err(), "null policy is not absence");
}

/// Each tampered grammar is re-hashed into the binding, so only the grammar can refuse it.
#[tokio::test]
async fn rehashed_record_grammar_violations_still_refuse() {
    let prepared = prepare(true).await;
    let tamper =
        |edit: &dyn Fn(&mut knx_app::system2_validation_preparation::ProjectHardwareRecord)| {
            let mut bundle = prepared.bundle.clone();
            let record = bundle.hardware_preparation.as_mut().unwrap();
            edit(record);
            bundle.binding.project_preparation_sha256 = Some(record.sha256().unwrap());
            bundle
        };
    assert!(
        tamper(&|_| {}).plan().is_ok(),
        "control: an untouched re-hash is admitted"
    );
    assert!(
        revision2(prepared.bundle.clone()).plan().is_ok(),
        "control: revision 2"
    );
    let wrong = "up258-mv0025-object3-3cf4h-download-flag-firmware-cleared-any-v1";
    for (name, bundle) in [
        (
            "revision 2 with the post-load policy",
            tamper(&|r| r.revision = 2),
        ),
        (
            "revision 1 with the post-load policy",
            tamper(&|r| {
                r.revision = 1;
                r.pointer_width_policy = None;
            }),
        ),
        (
            "revision 3 without it",
            tamper(&|r| r.post_load_octet_policy = None),
        ),
        (
            "revision 3 with a wrong id",
            tamper(&|r| r.post_load_octet_policy = Some(wrong.into())),
        ),
        (
            "revision 3 without the width policy",
            tamper(&|r| r.pointer_width_policy = None),
        ),
    ] {
        assert!(bundle.plan().is_err(), "{name}");
    }
}

/// Replays the finished operation's checkpoints into a new store up to its final step, so
/// a single post-load checkpoint can be offered at exactly that position.
async fn store_in_final_step(prepared: &Prepared) -> (tempfile::TempDir, OperationStore) {
    let transport = FirmwareChanges {
        device: &prepared.device,
        octet: FLAG,
        value: 0,
    };
    let dir = tempfile::tempdir().unwrap();
    let (finished, result) = download(
        prepared,
        &prepared.bundle,
        &transport,
        &dir.path().join("done"),
    )
    .await;
    result.unwrap();
    let recovery = finished.recovery().unwrap().clone();
    let mut store =
        OperationStore::create(&dir.path().join("replay"), prepared.bundle.binding.clone())
            .unwrap();
    store
        .checkpoint(Checkpoint::RecoveryAcquired { recovery })
        .unwrap();
    let last = prepared.bundle.normalized_plan.steps.len() - 1;
    for index in 0..last {
        store
            .checkpoint(Checkpoint::StepStarted {
                index,
                mutation: true,
            })
            .unwrap();
        store
            .checkpoint(Checkpoint::StepFinished { index })
            .unwrap();
    }
    store
        .checkpoint(Checkpoint::StepStarted {
            index: last,
            mutation: true,
        })
        .unwrap();
    (dir, store)
}

#[tokio::test]
async fn the_journal_admits_only_the_record_the_declaration_produces() {
    let prepared = prepare(true).await;
    let (_dir, mut store) = store_in_final_step(&prepared).await;
    let genuine = flag_record(Some(0xa5));
    for forged in [
        PostLoadOctetRecord {
            observed: 2,
            ..genuine
        },
        PostLoadOctetRecord {
            written: 7,
            ..genuine
        },
        PostLoadOctetRecord {
            prestate: Some(0),
            ..genuine
        },
        PostLoadOctetRecord {
            prestate: None,
            ..genuine
        },
        PostLoadOctetRecord {
            address: FLAG - 1,
            ..genuine
        },
        PostLoadOctetRecord {
            object: 1,
            ..genuine
        },
    ] {
        assert!(
            store
                .checkpoint(Checkpoint::PostLoadOctetObservations {
                    observations: vec![forged],
                })
                .is_err(),
            "{forged:?}"
        );
    }
    for invalid in [vec![], vec![genuine, genuine]] {
        assert!(store
            .checkpoint(Checkpoint::PostLoadOctetObservations {
                observations: invalid,
            })
            .is_err());
    }
    store
        .checkpoint(Checkpoint::PostLoadOctetObservations {
            observations: vec![genuine],
        })
        .unwrap();
    assert!(
        store
            .checkpoint(Checkpoint::PostLoadOctetObservations {
                observations: vec![genuine],
            })
            .is_err(),
        "one list per run"
    );
    assert_eq!(store.progress().post_load_octets.len(), 1);
}

#[tokio::test]
async fn the_journal_refuses_a_record_before_the_final_step() {
    let prepared = prepare(true).await;
    let transport = FirmwareChanges {
        device: &prepared.device,
        octet: FLAG,
        value: 0,
    };
    let dir = tempfile::tempdir().unwrap();
    let (finished, result) = download(
        &prepared,
        &prepared.bundle,
        &transport,
        &dir.path().join("done"),
    )
    .await;
    result.unwrap();
    let mut store =
        OperationStore::create(&dir.path().join("early"), prepared.bundle.binding.clone()).unwrap();
    store
        .checkpoint(Checkpoint::RecoveryAcquired {
            recovery: finished.recovery().unwrap().clone(),
        })
        .unwrap();
    store
        .checkpoint(Checkpoint::StepStarted {
            index: 0,
            mutation: true,
        })
        .unwrap();
    assert!(store
        .checkpoint(Checkpoint::PostLoadOctetObservations {
            observations: vec![flag_record(Some(0xa5))],
        })
        .is_err());
}

#[tokio::test]
async fn a_restore_that_writes_the_old_bytes_stays_strict() {
    let prepared = prepare(true).await;
    let transport = FirmwareChanges {
        device: &prepared.device,
        octet: FLAG,
        value: 0,
    };
    let dir = tempfile::tempdir().unwrap();
    let operation = dir.path().join("operation");
    let (store, result) = download(&prepared, &prepared.bundle, &transport, &operation).await;
    result.unwrap();
    drop(store);
    let mut reopened = OperationStore::open(&operation, &prepared.bundle.binding).unwrap();
    let recovery = prepared.bundle.restore_phrase();
    let download = required_confirmation_phrase(prepared.device.address(), WriteScope::Download);
    let release = knx_net::commissioning::hardware_release::required_release_phrase(
        prepared.device.address(),
    );
    // The recovery plan writes the prestate A5h at the flag, so it differs from the
    // download: the same firmware clear is now a strict verification failure.
    let restored = restore_project_hardware(
        &transport,
        &prepared.bundle,
        &mut reopened,
        integrated_timing(),
        &HardwareRecoveryConfirmations {
            recovery: &recovery,
            download: &download,
            risk: &release,
        },
        &AtomicBool::new(false),
    )
    .await;
    assert!(restored.is_err());
    assert_eq!(reopened.outcome(), LocalOutcome::RestoreFailed);
    let progress = reopened.progress();
    assert_eq!(progress.post_load_octets.len(), 1, "only the download's");
    assert!(progress.final_step_failures.iter().any(|f| f.run
        == knx_app::property_validation::FailureDiagnosticRun::Restore
        && f.category == FinalStepCategory::VerificationMismatch));
}

async fn restore(
    prepared: &Prepared,
    transport: &FirmwareChanges<'_>,
    store: &mut OperationStore,
) -> std::io::Result<()> {
    let recovery = prepared.bundle.restore_phrase();
    let download = required_confirmation_phrase(prepared.device.address(), WriteScope::Download);
    let release = knx_net::commissioning::hardware_release::required_release_phrase(
        prepared.device.address(),
    );
    restore_project_hardware(
        transport,
        &prepared.bundle,
        store,
        integrated_timing(),
        &HardwareRecoveryConfirmations {
            recovery: &recovery,
            download: &download,
            risk: &release,
        },
        &AtomicBool::new(false),
    )
    .await
}

#[tokio::test]
async fn a_restore_that_is_the_download_tolerates_and_journals_its_own_record() {
    let prepared = prepare(true).await;
    let dir = tempfile::tempdir().unwrap();
    // First download with firmware that keeps the flag: the device then holds every written
    // value, 01h at the flag included, so the next operation's recovery plan is the download.
    let keeps = FirmwareChanges {
        device: &prepared.device,
        octet: FLAG,
        value: 1,
    };
    let (_, first) = download(
        &prepared,
        &prepared.bundle,
        &keeps,
        &dir.path().join("first"),
    )
    .await;
    first.unwrap();
    let clears = FirmwareChanges {
        device: &prepared.device,
        octet: FLAG,
        value: 0,
    };
    let operation = dir.path().join("second");
    let (store, second) = download(&prepared, &prepared.bundle, &clears, &operation).await;
    second.unwrap();
    let download_record = PostLoadOctetProgress {
        run: PointerObservationRun::Download,
        record: flag_record(Some(1)),
    };
    assert_eq!(store.progress().post_load_octets, [download_record]);
    drop(store);
    let mut reopened = OperationStore::open(&operation, &prepared.bundle.binding).unwrap();
    restore(&prepared, &clears, &mut reopened).await.unwrap();
    assert_eq!(reopened.outcome(), LocalOutcome::Restored);
    let expected = vec![
        download_record,
        PostLoadOctetProgress {
            run: PointerObservationRun::Restore,
            record: flag_record(Some(1)),
        },
    ];
    assert_eq!(reopened.progress().post_load_octets, expected);
    drop(reopened);
    let again = OperationStore::open(&operation, &prepared.bundle.binding).unwrap();
    assert_eq!(again.progress().post_load_octets, expected);
}

#[tokio::test]
async fn a_restore_with_a_differing_plan_refuses_the_download_record() {
    let prepared = prepare(true).await;
    let transport = FirmwareChanges {
        device: &prepared.device,
        octet: FLAG,
        value: 0,
    };
    let dir = tempfile::tempdir().unwrap();
    let operation = dir.path().join("operation");
    let (store, result) = download(&prepared, &prepared.bundle, &transport, &operation).await;
    result.unwrap();
    drop(store);
    // The prestate held A5h at the flag, so the recovery plan differs from the download.
    let mut reopened = OperationStore::open(&operation, &prepared.bundle.binding).unwrap();
    reopened.checkpoint(Checkpoint::RestoreStarted).unwrap();
    let last = prepared.bundle.normalized_plan.steps.len() - 1;
    for index in 0..last {
        reopened
            .checkpoint(Checkpoint::StepStarted {
                index,
                mutation: true,
            })
            .unwrap();
        reopened
            .checkpoint(Checkpoint::StepFinished { index })
            .unwrap();
    }
    reopened
        .checkpoint(Checkpoint::StepStarted {
            index: last,
            mutation: true,
        })
        .unwrap();
    for record in [
        flag_record(Some(0xa5)),
        PostLoadOctetRecord {
            written: 0xa5,
            ..flag_record(Some(0xa5))
        },
    ] {
        assert!(
            reopened
                .checkpoint(Checkpoint::PostLoadOctetObservations {
                    observations: vec![record],
                })
                .is_err(),
            "{record:?}"
        );
    }
}
