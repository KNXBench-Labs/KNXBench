//! Mounted caption-positive recovery keeps original evidence and confirmation authority separate.

use super::*;
use crate::device_system2::settlement::Receipt;
use knx_app::{
    property_validation::{DownloadFacts, DownloadResult, System2Settlement},
    property_validation_bundle::PreparedBundle,
};
use knx_store::project_history::NativeSnapshot;

#[path = "recovery_authority/wrong_serial.rs"]
mod wrong_serial;

const OLD_EXTENTS: [(u16, u16); 4] = [(278, 16), (608, 16), (1024, 8), (15360, 8)];
const MEMORY: ProgrammingParts = ProgrammingParts {
    address: false,
    application: true,
    parameters: true,
    group_addresses: true,
};

#[derive(Clone, Copy, Debug)]
struct Case {
    namespace: u8,
    retained: bool,
}

#[derive(Clone, Copy)]
enum RecoveryMode {
    OriginalSerial,
    WrongObservedSerial,
}

// The public-package OLD helper has larger allocation records. Recovery replays
// source-shaped controls, not captured allocator history: seed the synthetic
// fixture's independent OLD allocation shape BEFORE NEW, never normalize the
// resulting snapshot to hide a restoration difference. Unowned memory stays put.
fn source_shaped_old() -> Arc<SimulatedDevice> {
    let device = old_device();
    for (owner, mut object) in [
        SimulatedSystem2Object::AddressTable,
        SimulatedSystem2Object::AssociationTable,
        SimulatedSystem2Object::Application,
    ]
    .into_iter()
    .zip(device.system2_snapshot().objects)
    {
        for allocation in &mut object.allocations {
            allocation.length = OLD_EXTENTS
                .iter()
                .find(|(address, _)| *address == allocation.start)
                .unwrap()
                .1;
        }
        device.preset_system2_object(owner, object);
    }
    device
}

// Count the owning connector and every frame, including reads and control frames.
// Delegate the actual transport, hardware-kind provenance and first-write pause
// to the existing caption harness; no simulator behavior or timings change.
#[derive(Default)]
struct Counters {
    connects: AtomicUsize,
    disconnects: AtomicUsize,
    frames: AtomicUsize,
    services: Mutex<Vec<ApplicationService>>,
}
impl Counters {
    fn read(&self) -> [usize; 3] {
        [
            self.connects.load(Ordering::SeqCst),
            self.disconnects.load(Ordering::SeqCst),
            self.frames.load(Ordering::SeqCst),
        ]
    }
}
struct CountedConnector {
    hardware: Arc<Hardware>,
    counters: Arc<Counters>,
}
struct CountedTunnel {
    tunnel: Tunnel,
    counters: Arc<Counters>,
}
impl crate::GatewayConnector for CountedConnector {
    #[allow(clippy::type_complexity)]
    fn connect_tunnel(
        &self,
        gateway: SocketAddrV4,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<Box<dyn crate::BusTunnel>, crate::BusSessionError>>
                + Send
                + '_,
        >,
    > {
        Box::pin(async move {
            assert_eq!(gateway, GATEWAY.parse::<SocketAddrV4>().unwrap());
            self.counters.connects.fetch_add(1, Ordering::SeqCst);
            Ok(Box::new(CountedTunnel {
                tunnel: Tunnel(self.hardware.clone()),
                counters: self.counters.clone(),
            }) as Box<dyn crate::BusTunnel>)
        })
    }
    #[allow(clippy::type_complexity)]
    fn connect_tunnel_with_ack_wait(
        &self,
        gateway: SocketAddrV4,
        wait: knx_net::TunnelAckWait,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<Box<dyn crate::BusTunnel>, crate::BusSessionError>>
                + Send
                + '_,
        >,
    > {
        assert_eq!(wait, knx_net::TunnelAckWait::Patient);
        self.connect_tunnel(gateway)
    }
    #[allow(clippy::type_complexity)]
    fn discover(
        &self,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<Vec<knx_net::DiscoveredGateway>, crate::BusSessionError>>
                + Send
                + '_,
        >,
    > {
        Box::pin(async { panic!("recovery authority fixture never discovers a gateway") })
    }
}
impl crate::BusTunnel for CountedTunnel {
    fn diagnostics_are_simulated(&self) -> bool {
        crate::BusTunnel::diagnostics_are_simulated(&self.tunnel)
    }
    fn assigned_address(&self) -> IndividualAddress {
        crate::BusTunnel::assigned_address(&self.tunnel)
    }
    fn subscribe(&self) -> broadcast::Receiver<TunnelEvent> {
        crate::BusTunnel::subscribe(&self.tunnel)
    }
    fn send(
        &self,
        _: Destination,
        _: ApplicationService,
    ) -> Pin<Box<dyn Future<Output = Result<(), crate::BusSessionError>> + Send + '_>> {
        Box::pin(async { panic!("recovery must use management frames") })
    }
    fn send_frame(
        &self,
        destination: Destination,
        transport: Tpci,
        service: ApplicationService,
    ) -> Pin<Box<dyn Future<Output = Result<(), BusError>> + Send + '_>> {
        self.counters.frames.fetch_add(1, Ordering::SeqCst);
        self.counters.services.lock().unwrap().push(service.clone());
        crate::BusTunnel::send_frame(&self.tunnel, destination, transport, service)
    }
    fn disconnect(
        self: Box<Self>,
    ) -> Pin<Box<dyn Future<Output = Result<(), crate::BusSessionError>> + Send>> {
        Box::pin(async move {
            let result = crate::BusTunnel::disconnect(Box::new(self.tunnel)).await;
            self.counters.disconnects.fetch_add(1, Ordering::SeqCst);
            result
        })
    }
}
fn count_transport(
    state: &mut crate::SharedState,
    hardware: &Arc<Hardware>,
    counters: &Arc<Counters>,
) {
    Arc::get_mut(state).unwrap().connector = Box::new(CountedConnector {
        hardware: hardware.clone(),
        counters: counters.clone(),
    });
}

#[derive(Debug, PartialEq)]
struct Editor {
    snapshot: NativeSnapshot,
    stack: knx_core::CommandStack,
    undo: Vec<knx_core::Project>,
    redo: Vec<knx_core::Project>,
    revision: u64,
    generation: i64,
    logs: Vec<Value>,
}
impl Editor {
    fn read(state: &crate::SharedState) -> Self {
        let project = state.project.lock().unwrap();
        let project = project.as_ref().unwrap();
        let stack = state.command_stack.lock().unwrap();
        let (undo, redo) = stack.snapshot_states(project).unwrap();
        Self {
            snapshot: crate::project_history::snapshot(state, project),
            stack: stack.clone(),
            undo,
            redo,
            revision: crate::domain::current_project_revision(state),
            generation: *state.history_generation.lock().unwrap(),
            logs: state
                .session_log
                .lock()
                .unwrap()
                .entries()
                .iter()
                .filter(|entry| entry.source == "System2Download")
                .map(|entry| serde_json::to_value(entry).unwrap())
                .collect(),
        }
    }
}
fn project_flags(project: &knx_core::Project) -> [bool; 5] {
    let c = &project.devices.get(f::DEVICE).unwrap().commissioning;
    [
        c.individual_address_loaded,
        c.application_program_loaded,
        c.parameters_loaded,
        c.communication_part_loaded,
        c.medium_config_loaded,
    ]
}
fn apply(state: &crate::SharedState, command: knx_core::Command) {
    let binding = crate::domain::SnapshotBinding {
        server_incarnation: state.server_incarnation.clone(),
        project_incarnation: state.project_incarnation.load(Ordering::SeqCst),
        snapshot_revision: crate::domain::current_project_revision(state),
    };
    crate::domain::apply_bound_prepared(state, &binding, "Recovery authority fixture", command)
        .unwrap();
    assert_eq!(
        crate::domain::current_project_revision(state),
        binding.snapshot_revision + 1
    );
}
fn seed_recovery_history(state: &crate::SharedState) {
    // Download already sanitized pre-download history. Add real post-download
    // marks so recovery's own history sanitization is independently observable.
    for programmed in [false, true] {
        apply(
            state,
            knx_core::Command::SetDeviceProgramming {
                device: f::DEVICE,
                change: ProgrammingChange::mark(ProgrammingParts::ALL, programmed),
            },
        );
    }
    // A second mark pair is undone twice: redo must carry real programming
    // claims too, while the live project returns to the credited snapshot.
    for programmed in [false, true] {
        apply(
            state,
            knx_core::Command::SetDeviceProgramming {
                device: f::DEVICE,
                change: ProgrammingChange::mark(ProgrammingParts::ALL, programmed),
            },
        );
    }
    crate::domain::undo_impl(state).unwrap();
    crate::domain::undo_impl(state).unwrap();
    let before = Editor::read(state);
    assert_eq!(project_flags(&before.snapshot.project), [true; 5]);
    assert!(!before.undo.is_empty());
    assert_eq!(before.redo.len(), 2);
    for (direction, states) in [("undo", &before.undo), ("redo", &before.redo)] {
        assert!(
            states
                .iter()
                .any(|project| project_flags(project)[1..4] == [true; 3]),
            "post-download {direction} history can restore stale MEMORY credit"
        );
    }
}
fn assert_recovery_settlement(before: &Editor, after: &Editor, case: Case) {
    let mut expected = before.snapshot.clone();
    let c = &mut expected
        .project
        .devices
        .get_mut(f::DEVICE)
        .unwrap()
        .commissioning;
    c.application_program_loaded = false;
    c.parameters_loaded = false;
    c.communication_part_loaded = false;
    assert_eq!(
        after.snapshot, expected,
        "{case:?}: MEMORY only, no new timestamp/credit"
    );
    assert_eq!(
        project_flags(&after.snapshot.project),
        [true, false, false, false, true]
    );
    let mut expected_stack = before.stack.clone();
    expected_stack.forget_programmed_claims(f::DEVICE, MEMORY);
    assert_eq!(
        after.stack, expected_stack,
        "{case:?}: exact history sanitization"
    );
    let (undo, redo) = expected_stack.snapshot_states(&expected.project).unwrap();
    assert_eq!(after.undo, undo);
    assert_eq!(after.redo, redo);
    assert_eq!(
        after.revision,
        before.revision + 1,
        "one recovery settlement"
    );
    assert_eq!(after.generation, before.generation, "unsaved editor");
    assert_eq!(after.logs.len(), before.logs.len() + 1);
    assert_eq!(&after.logs[..before.logs.len()], before.logs.as_slice());
    for (direction, states) in [("undo", &after.undo), ("redo", &after.redo)] {
        assert!(!states.is_empty(), "{direction} history remains exercised");
        for project in states {
            assert_eq!(
                project_flags(project)[1..4],
                [false; 3],
                "no stale {direction} MEMORY claims"
            );
        }
    }
}

// Read only the original committed chain while the paused worker owns its lock.
// The real producer, not this reader, creates every bundle/checkpoint/backup.
fn backup_at_pause(operation: &Path, original: &PreparedBundle) -> RecoveryRecord {
    let journal = operation.join("journal");
    let head: Value =
        serde_json::from_slice(&std::fs::read(journal.join("HEAD")).unwrap()).unwrap();
    let mut previous = String::new();
    let mut backup = None;
    for index in 0..=head["sequence"].as_u64().unwrap() {
        let bytes = std::fs::read(journal.join(format!("checkpoint-{index:08}.json"))).unwrap();
        let envelope: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(envelope["sequence"], index);
        assert_eq!(envelope["previous_sha256"], previous);
        let checkpoint: Checkpoint =
            serde_json::from_value(envelope["checkpoint"].clone()).unwrap();
        if let Checkpoint::RecoveryAcquired { recovery } = checkpoint {
            assert!(backup.replace(recovery).is_none(), "one ORIGINAL prestate");
        }
        previous = knx_productdb::sha256_hex(&bytes);
    }
    assert_eq!(head["sha256"], previous);
    let backup = backup.expect("actual original backup before the paused data write");
    assert_eq!(backup.original_bundle, *original);
    // The producer retains a serial override only when the acquisition differs
    // from the original observed binding; None is the matching-read form.
    assert_eq!(original.binding.serial, SERIAL);
    assert_eq!(backup.observed_serial, None);
    assert!(backup.prestate_unreadable_allocations.is_empty());
    assert_eq!(backup.allocations.len(), OLD_EXTENTS.len());
    for (allocation, &(address, length)) in backup.allocations.iter().zip(&OLD_EXTENTS) {
        assert_eq!(allocation.address, address);
        assert_eq!(allocation.data, vec![0xa5; usize::from(length)]);
    }
    backup
}
fn no_worker(state: &crate::SharedState) {
    let inner = state.device_system2.inner.lock().unwrap();
    assert!(inner
        .current
        .as_ref()
        .is_none_or(|record| record.task.is_none()));
}

async fn refused(
    state: &crate::SharedState,
    router: &Router,
    data: &Path,
    hardware: &Arc<Hardware>,
    counters: &Arc<Counters>,
    request: (&str, Value, u16, &str),
) {
    let (route, body, code, kind) = request;
    no_worker(state);
    let evidence = super::super::evidence_tree(data);
    let editor = Editor::read(state);
    let device = hardware.device.system2_snapshot();
    let traffic = counters.read();
    let control = state.device_system2.control.snapshot();
    let response = http(router, route, Some(body), code).await;
    // Plain bad requests carry only `error`; conflict refusals also carry `kind`.
    let expected = match code {
        400 => json!({"error": kind}),
        409 => json!({"error": kind, "kind": kind}),
        _ => panic!("unsupported refusal status {code}"),
    };
    assert_eq!(response, expected, "{route}");
    no_worker(state);
    assert_eq!(
        state.device_system2.control.snapshot(),
        control,
        "no worker admission"
    );
    assert_eq!(
        counters.read(),
        traffic,
        "no tunnel, frame or disconnect activity"
    );
    assert_eq!(hardware.device.system2_snapshot(), device);
    assert_eq!(
        Editor::read(state),
        editor,
        "no flags/history/revision/log mutation"
    );
    assert_eq!(
        super::super::evidence_tree(data),
        evidence,
        "no evidence/HEAD/claim mutation"
    );
}

// Copy actual producer evidence to an OWNED disposable root. Remove only its
// observed bundle to exercise unavailable original evidence without damaging the
// valid neighbor or rewriting any checkpoint, hash, binding or recovery record.
fn copy_tree(source: &Path, destination: &Path) {
    std::fs::create_dir(destination).unwrap();
    std::fs::set_permissions(
        destination,
        std::fs::metadata(source).unwrap().permissions(),
    )
    .unwrap();
    for entry in std::fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let target = destination.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            assert!(entry.file_type().unwrap().is_file());
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

async fn observations(router: &Router, id: &str, facts: &DownloadFacts, receipt: &str) {
    let status = http(router, "/api/device-system2/status", None, 200).await;
    assert_eq!(status["operationId"], id);
    assert_eq!(status["state"], "restored");
    assert_eq!(status["downloadResult"], "verified");
    assert_eq!(status["restoreAttempted"], true);
    assert_eq!(status["projectSettlement"], receipt);
    assert_eq!(status["downloadCompletedSteps"], facts.completed_steps);
    assert_eq!(status["downloadTotalSteps"], json!(facts.total_steps));
    assert!(status["recoveryPhrase"].is_null());
    let report = http(
        router,
        &format!("/api/device-system2/report?operationId={id}"),
        None,
        200,
    )
    .await;
    assert_eq!(
        report["download"],
        json!({"outcome":"finished", "result":"verified",
            "completedSteps":facts.completed_steps, "totalSteps":facts.total_steps})
    );
    assert_eq!(report["recovery"]["outcome"], "restored");
}

async fn run(case: Case) {
    run_with_mode(case, RecoveryMode::OriginalSerial).await;
}

async fn run_with_mode(case: Case, mode: RecoveryMode) {
    let input = f::inputs(case.namespace, case.retained);
    let data = input.dir.path();
    let products = data.join("caption-products.sqlite");
    let mut project = input.project.clone();
    let selected = project.devices.get_mut(f::DEVICE).unwrap();
    selected.address = Some(TARGET.parse().unwrap());
    selected.commissioning.medium_config_loaded = true;
    selected.commissioning.last_download = Some("2001-02-03T04:05:06Z".parse().unwrap());
    let hardware = Arc::new(Hardware {
        device: source_shaped_old(),
        paused: AtomicBool::new(true),
        reached: Notify::new(),
        release: Notify::new(),
    });
    let old = hardware.device.system2_snapshot();
    let counters = Arc::new(Counters::default());
    drop(input.conn);
    let mut state = state_for(data, products.clone(), project, &hardware);
    count_transport(&mut state, &hardware, &counters);
    let router = crate::app(state.clone(), None);
    let preparation = preview(&state, &router, &input.profile, case.retained).await;
    let original_preparation = state
        .device_system2
        .inner
        .lock()
        .unwrap()
        .preparation
        .clone()
        .unwrap();
    let admitted = Editor::read(&state);
    assert_eq!(
        project_flags(&admitted.snapshot.project),
        [false, false, false, false, true]
    );
    let accepted = http(
        &router,
        "/api/device-system2/start",
        Some(start(&preparation)),
        200,
    )
    .await;
    let id = accepted["operationId"].as_str().unwrap().to_owned();
    assert_eq!(id, preparation["preparationId"]);
    tokio::time::timeout(Duration::from_secs(60), hardware.reached.notified())
        .await
        .expect("actual first data write after original start admission");
    let root = data.join(crate::device_system2::ROOT_NAME);
    assert_eq!(std::fs::read_to_string(root.join("selected")).unwrap(), id);
    let operation = root.join(&id);
    let original_bytes = std::fs::read(operation.join("observed.json")).unwrap();
    let original = read_bundle(&operation.join("observed.json")).unwrap();
    assert_eq!(original.format, 3);
    assert!(!original.binding.simulated);
    let source = original.project_preparation.as_ref().unwrap();
    assert_eq!(
        source.schema_namespace,
        format!("http://knx.org/xml/project/{}", case.namespace)
    );
    assert_eq!(
        matches!(
            &source.product_input,
            knx_app::system2_validation_preparation::ProjectProductInput::RetainedSourceMembers { .. }
        ),
        case.retained,
        "{case:?}: independently admitted selector family"
    );
    assert_eq!(
        original.hardware_preparation.as_ref().unwrap().operation,
        id
    );
    let backup = backup_at_pause(&operation, &original);
    assert_ne!(
        hardware.device.system2_snapshot(),
        old,
        "NEW really mutated the modeled device"
    );
    assert_eq!(
        Editor::read(&state),
        admitted,
        "worker is paused, no terminal settlement yet"
    );

    let replacement = data.join("recovery-caption-replacement.sqlite");
    f::replacement(
        &replacement,
        &f::caption(&input.package),
        &input.package,
        false,
    );
    state
        .product_db
        .as_ref()
        .unwrap()
        .restore_legacy(&products, &replacement)
        .unwrap();
    {
        let project = state.project.lock().unwrap();
        state
            .product_db
            .as_ref()
            .unwrap()
            .with_read(|conn| {
                let winner: String = conn
                    .query_row(
                        "SELECT source_sha256 FROM application_program WHERE id=?1",
                        [f::PROGRAM],
                        |row| row.get(0),
                    )
                    .unwrap();
                assert_ne!(
                    winner, original.binding.source_sha256,
                    "measured caption source changed"
                );
                assert!(
                    knx_app::system2_validation_preparation::admit_current_project_hardware(
                        conn,
                        project.as_ref().unwrap(),
                        f::DEVICE,
                        &original,
                    )
                    .is_err(),
                    "old exact-pin admission refuses"
                );
                assert_eq!(
                    knx_app::system2_validation_preparation::current_project_hardware_parts(
                        conn,
                        project.as_ref().unwrap(),
                        f::DEVICE,
                        &original,
                    )
                    .unwrap(),
                    ProgrammingParts::ALL,
                    "{case:?}: explicit caption-positive witness"
                );
                Ok::<_, std::io::Error>(())
            })
            .unwrap()
            .unwrap();
    }
    hardware.release.notify_one();
    super::super::failure_diagnostic::settled_slow(&state, &router, "finished").await;
    no_worker(&state);
    let finished = http(&router, "/api/device-system2/status", None, 200).await;
    assert_eq!(finished["projectSettlement"], "sessionOnly");
    assert_eq!(finished["downloadResult"], "verified");
    assert_eq!(finished["backupAvailable"], true);
    assert_eq!(flags(&state), [true; 5]);
    let credited = Editor::read(&state);
    let credited_at = credited
        .snapshot
        .project
        .devices
        .get(f::DEVICE)
        .unwrap()
        .commissioning
        .last_download;
    assert!(credited_at.is_some());
    assert_ne!(
        credited_at,
        admitted
            .snapshot
            .project
            .devices
            .get(f::DEVICE)
            .unwrap()
            .commissioning
            .last_download
    );
    assert_eq!(credited.revision, admitted.revision + 1);
    assert_eq!(credited.logs.len(), admitted.logs.len() + 1);
    assert_eq!(
        state
            .device_system2
            .inner
            .lock()
            .unwrap()
            .current
            .as_ref()
            .unwrap()
            .settlement,
        Receipt::SessionOnly
    );
    assert_eq!(
        counters.read()[..2],
        [1, 1],
        "download joined and disconnected"
    );
    let store = OperationStore::open(&operation.join("journal"), &original.binding).unwrap();
    assert_eq!(store.outcome(), LocalOutcome::Finished);
    let progress = store.progress();
    assert_eq!(progress.outcome, LocalOutcome::Finished);
    assert!(!progress.restore_attempted);
    assert!(progress.mutation_possible);
    assert_eq!(progress.download.outcome, LocalOutcome::Finished);
    assert_eq!(progress.download.result, DownloadResult::Verified);
    assert!(progress.download.completed_steps > 0);
    assert_eq!(
        Some(progress.download.completed_steps),
        progress.download.total_steps
    );
    assert_eq!(store.recovery(), Some(&backup));
    let facts = progress.download;
    let phrases = store.recovery_phrases().unwrap();
    assert_eq!(finished["recoveryPhrase"], phrases.recovery_phrase);
    assert_eq!(finished["downloadPhrase"], phrases.download_phrase);
    assert_eq!(finished["releasePhrase"], phrases.release_phrase);
    assert_eq!(phrases.recovery_phrase, original.restore_phrase());
    assert_eq!(phrases.download_phrase, required_download(&original));
    assert_eq!(phrases.release_phrase, required_release(&original));
    drop(store);
    assert_eq!(
        std::fs::read(operation.join("observed.json")).unwrap(),
        original_bytes
    );

    // Retain the SAME original preparation solely to probe the mounted old-start
    // boundary after completion. This route also sees the settlement's newer
    // project revision: its stale response is NOT a source-only discriminator.
    // The exact-pin refusal on the unchanged admitted project was checked above,
    // after catalogue replacement and before terminal settlement. Never re-pin.
    state.device_system2.inner.lock().unwrap().preparation = Some(original_preparation);
    refused(
        &state,
        &router,
        data,
        &hardware,
        &counters,
        (
            "/api/device-system2/start",
            start(&preparation),
            409,
            "system2PreparationStale",
        ),
    )
    .await;
    seed_recovery_history(&state);
    let before_recovery = Editor::read(&state);
    assert_eq!(
        before_recovery.snapshot, credited.snapshot,
        "only real history was added after credit"
    );
    let recovery = json!({
        "operationId":finished["operationId"], "gateway":GATEWAY,
        "recoveryConfirmation":finished["recoveryPhrase"],
        "downloadConfirmation":finished["downloadPhrase"],
        "releaseConfirmation":finished["releasePhrase"]
    });
    if matches!(mode, RecoveryMode::WrongObservedSerial) {
        assert_ne!(hardware.device.system2_snapshot(), old, "NEW is not OLD");
        wrong_serial::finish(
            &state,
            &router,
            &hardware,
            &counters,
            wrong_serial::Original {
                case,
                data,
                operation: &operation,
                original: &original,
                original_bytes: &original_bytes,
                backup: &backup,
                facts: &facts,
                before: &before_recovery,
                finished: &finished,
                recovery,
            },
        )
        .await;
        return;
    }
    for field in [
        "recoveryConfirmation",
        "downloadConfirmation",
        "releaseConfirmation",
    ] {
        let mut wrong = recovery.clone();
        wrong[field] = format!("{} ", wrong[field].as_str().unwrap()).into();
        refused(
            &state,
            &router,
            data,
            &hardware,
            &counters,
            (
                "/api/device-system2/recover",
                wrong,
                409,
                "system2RecoveryRefused",
            ),
        )
        .await;
    }
    let mut wrong_id = recovery.clone();
    let unknown = if id == "ffffffffffffffffffffffffffffffff" {
        "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"
    } else {
        "ffffffffffffffffffffffffffffffff"
    };
    assert!(!root.join(unknown).exists());
    wrong_id["operationId"] = unknown.into();
    // The existing route checks unclaimed recovery BEFORE reopen; a missing
    // operation directory maps to RecoveryConsumed, not OperationUnknown.
    refused(
        &state,
        &router,
        data,
        &hardware,
        &counters,
        (
            "/api/device-system2/recover",
            wrong_id,
            409,
            "system2RecoveryConsumed",
        ),
    )
    .await;
    let mut malformed = recovery.clone();
    malformed["operationId"] = "not-an-operation-id".into();
    refused(
        &state,
        &router,
        data,
        &hardware,
        &counters,
        (
            "/api/device-system2/recover",
            malformed,
            400,
            "system2InvalidOperation",
        ),
    )
    .await;
    assert!(!operation.join("recovery-command").exists());

    let unavailable = tempfile::tempdir().unwrap();
    let copied_root = unavailable.path().join(crate::device_system2::ROOT_NAME);
    copy_tree(&root, &copied_root);
    assert_eq!(
        super::super::evidence_tree(unavailable.path()),
        super::super::evidence_tree(data)
    );
    std::fs::remove_file(copied_root.join(&id).join("observed.json")).unwrap();
    let mut missing = state_for(
        unavailable.path(),
        products.clone(),
        before_recovery.snapshot.project.clone(),
        &hardware,
    );
    count_transport(&mut missing, &hardware, &counters);
    let missing_router = crate::app(missing.clone(), None);
    refused(
        &missing,
        &missing_router,
        unavailable.path(),
        &hardware,
        &counters,
        (
            "/api/device-system2/recover",
            recovery.clone(),
            409,
            "system2RecoveryUnavailable",
        ),
    )
    .await;
    assert!(!copied_root.join(&id).join("recovery-command").exists());
    drop(missing_router);
    drop(missing);
    assert_eq!(Editor::read(&state), before_recovery);
    assert_eq!(
        std::fs::read(operation.join("observed.json")).unwrap(),
        original_bytes
    );

    // The caption-positive terminal has already been observed with a healthy
    // changed catalogue. Make that CURRENT owner unusable now: the valid
    // neighboring recovery must need only its original durable evidence, not
    // source re-admission or a newly prepared current-project capability.
    let owner = state.product_db.as_ref().unwrap();
    let poisoned = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        owner
            .with_read::<(), std::io::Error>(|_| panic!("recovery authority poisons current owner"))
    }));
    assert!(poisoned.is_err());
    assert!(owner.with_read(|_| Ok::<_, std::io::Error>(())).is_err());

    // Preserve the exact bytes of every existing checkpoint: recovery may append
    // and advance HEAD, never rewrite the original source payload or backup.
    let checkpoints: Vec<_> = std::fs::read_dir(operation.join("journal"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("checkpoint-")
        })
        .map(|path| {
            let bytes = std::fs::read(&path).unwrap();
            (path, bytes)
        })
        .collect();
    assert!(!checkpoints.is_empty());
    let traffic = counters.read();
    let restoring = http(
        &router,
        "/api/device-system2/recover",
        Some(recovery.clone()),
        200,
    )
    .await;
    assert_eq!(restoring["operationId"], id);
    // Use the original recovery test's unchanged 10-second join deadline, not a
    // cached status or a broader wait. The caption download uses its existing helper.
    let restored = super::super::settled(&state, &router, "restored").await;
    no_worker(&state);
    assert_eq!(restored["projectSettlement"], "sessionOnly");
    assert_eq!(
        restored["downloadResult"], "verified",
        "original download facts stay frozen"
    );
    assert_eq!(restored["restoreAttempted"], true);
    assert_eq!(restored["binding"], finished["binding"]);
    assert_eq!(counters.read()[0], traffic[0] + 1);
    assert_eq!(counters.read()[1], traffic[1] + 1);
    assert!(
        counters.read()[2] > traffic[2],
        "actual recovery exchanged management frames"
    );
    assert_eq!(
        hardware.device.system2_snapshot(),
        old,
        "{case:?}: exact independent OLD restoration"
    );
    assert_eq!(
        std::fs::read_to_string(operation.join("recovery-command")).unwrap(),
        id
    );
    assert_eq!(
        state.device_system2.control.snapshot().settlement,
        System2Settlement::Settled {
            operation: format!("{id}-restore"),
            outcome: LocalOutcome::Restored,
        }
    );
    let store = OperationStore::open(&operation.join("journal"), &original.binding).unwrap();
    assert_eq!(store.outcome(), LocalOutcome::Restored);
    assert_eq!(store.progress().outcome, LocalOutcome::Restored);
    assert!(store.progress().completed_steps > 0);
    assert_eq!(
        Some(store.progress().completed_steps),
        store.progress().total_steps
    );
    assert!(store.progress().restore_attempted);
    assert!(!store.progress().restore_available);
    assert_eq!(store.progress().download, facts);
    assert_eq!(store.recovery(), Some(&backup));
    drop(store);
    assert_eq!(
        std::fs::read(operation.join("observed.json")).unwrap(),
        original_bytes
    );
    for (path, bytes) in checkpoints {
        assert_eq!(
            std::fs::read(path).unwrap(),
            bytes,
            "original checkpoint remains exact"
        );
    }
    let after_recovery = Editor::read(&state);
    assert_recovery_settlement(&before_recovery, &after_recovery, case);
    refused(
        &state,
        &router,
        data,
        &hardware,
        &counters,
        (
            "/api/device-system2/recover",
            recovery.clone(),
            409,
            "system2RecoveryConsumed",
        ),
    )
    .await;

    let evidence = super::super::evidence_tree(data);
    let traffic = counters.read();
    observations(&router, &id, &facts, "sessionOnly").await;
    let selected = http(
        &router,
        "/api/device-system2/select",
        Some(json!({"operationId":id})),
        200,
    )
    .await;
    assert_eq!(selected["projectSettlement"], "notRecorded");
    observations(&router, &id, &facts, "notRecorded").await;
    assert_eq!(
        Editor::read(&state),
        after_recovery,
        "read/select cannot re-settle"
    );
    assert_eq!(super::super::evidence_tree(data), evidence);
    assert_eq!(counters.read(), traffic);
    assert_eq!(hardware.device.system2_snapshot(), old);
    drop(router);
    drop(state);

    // Actual service restart, no project/catalogue. Historical evidence alone
    // still reports the original verified download and consumed recovery.
    let mut restarted = Arc::new(crate::AppState::with_product_db_at(data.into(), None));
    count_transport(&mut restarted, &hardware, &counters);
    assert!(restarted.project.lock().unwrap().is_none());
    assert!(restarted.product_db.is_none());
    let router = crate::app(restarted.clone(), None);
    let restarted_control = restarted.device_system2.control.snapshot();
    let restarted_revision = crate::domain::current_project_revision(&restarted);
    observations(&router, &id, &facts, "notRecorded").await;
    assert_eq!(
        http(&router, "/api/device-system2/recover", Some(recovery), 409).await["kind"],
        "system2RecoveryConsumed"
    );
    no_worker(&restarted);
    assert_eq!(
        restarted.device_system2.control.snapshot(),
        restarted_control
    );
    assert_eq!(
        crate::domain::current_project_revision(&restarted),
        restarted_revision
    );
    assert!(restarted.project.lock().unwrap().is_none());
    assert_eq!(
        super::super::evidence_tree(data),
        evidence,
        "restart/readonly cannot mutate evidence"
    );
    assert_eq!(counters.read(), traffic);
    assert_eq!(hardware.device.system2_snapshot(), old);
}

#[tokio::test]
async fn caption_mounted_recovery_authority_standalone20() {
    run(Case {
        namespace: 20,
        retained: false,
    })
    .await;
}
#[tokio::test]
async fn caption_mounted_recovery_authority_standalone23() {
    run(Case {
        namespace: 23,
        retained: false,
    })
    .await;
}
#[tokio::test]
async fn caption_mounted_recovery_authority_retained20() {
    run(Case {
        namespace: 20,
        retained: true,
    })
    .await;
}
#[tokio::test]
async fn caption_mounted_recovery_authority_retained23() {
    run(Case {
        namespace: 23,
        retained: true,
    })
    .await;
}
