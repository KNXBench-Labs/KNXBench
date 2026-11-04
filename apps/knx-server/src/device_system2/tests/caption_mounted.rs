//! Real mounted hardware-kind workers settle changed captions and persist history without sockets.
use super::*;
#[allow(dead_code)]
#[path = "../../../../../crates/knx-app/tests/it/system2_validation_preparation/caption_fixture.rs"]
mod f;
use crate::device_system2::tests::diagnostic_bundle_fallback::fixture;
use knx_app::system2_validation_preparation::{prepare_project_hardware_source, RecoveryTrack};
use knx_core::commissioning::pending::{ProgrammingChange, ProgrammingParts};

#[path = "caption_mounted/persistence.rs"]
mod persistence;
#[path = "caption_mounted/target_gate.rs"]
mod target_gate;

struct Hardware {
    device: Arc<SimulatedDevice>,
    paused: AtomicBool,
    reached: Notify,
    release: Notify,
}
struct Connector(Arc<Hardware>);
struct Tunnel(Arc<Hardware>);
impl crate::GatewayConnector for Connector {
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
        assert_eq!(gateway, GATEWAY.parse::<SocketAddrV4>().unwrap());
        Box::pin(async move { Ok(Box::new(Tunnel(self.0.clone())) as Box<dyn crate::BusTunnel>) })
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
        Box::pin(async { panic!("caption fixture never discovers or contacts a gateway") })
    }
}
impl crate::BusTunnel for Tunnel {
    fn diagnostics_are_simulated(&self) -> bool {
        false
    }
    fn assigned_address(&self) -> IndividualAddress {
        self.0.device.assigned_address()
    }
    fn subscribe(&self) -> broadcast::Receiver<TunnelEvent> {
        self.0.device.subscribe()
    }
    fn send(
        &self,
        _: Destination,
        _: ApplicationService,
    ) -> Pin<Box<dyn Future<Output = Result<(), crate::BusSessionError>> + Send + '_>> {
        Box::pin(async { panic!("management frames required") })
    }
    fn send_frame(
        &self,
        destination: Destination,
        transport: Tpci,
        service: ApplicationService,
    ) -> Pin<Box<dyn Future<Output = Result<(), BusError>> + Send + '_>> {
        Box::pin(async move {
            let data = matches!(
                service,
                ApplicationService::MemoryWrite { .. } | ApplicationService::UserMemoryWrite { .. }
            );
            self.0
                .device
                .send_frame(destination, transport, service)
                .await?;
            if data && self.0.paused.swap(false, Ordering::SeqCst) {
                self.0.reached.notify_one();
                self.0.release.notified().await;
            }
            Ok(())
        })
    }
    fn disconnect(
        self: Box<Self>,
    ) -> Pin<Box<dyn Future<Output = Result<(), crate::BusSessionError>> + Send>> {
        Box::pin(async { Ok(()) })
    }
}
fn flags(state: &crate::SharedState) -> [bool; 5] {
    let guard = state.project.lock().unwrap();
    let c = &guard
        .as_ref()
        .unwrap()
        .devices
        .get(f::DEVICE)
        .unwrap()
        .commissioning;
    [
        c.individual_address_loaded,
        c.application_program_loaded,
        c.parameters_loaded,
        c.communication_part_loaded,
        c.medium_config_loaded,
    ]
}
fn mark(state: &crate::SharedState) {
    let binding = crate::domain::SnapshotBinding {
        server_incarnation: state.server_incarnation.clone(),
        project_incarnation: state.project_incarnation.load(Ordering::SeqCst),
        snapshot_revision: crate::domain::current_project_revision(state),
    };
    crate::domain::apply_bound_prepared(
        state,
        &binding,
        "Caption fixture",
        knx_core::Command::SetDeviceProgramming {
            device: f::DEVICE,
            change: ProgrammingChange::mark(ProgrammingParts::ALL, true),
        },
    )
    .unwrap();
}
fn state_for(
    data: &Path,
    products: PathBuf,
    project: knx_core::Project,
    hardware: &Arc<Hardware>,
) -> crate::SharedState {
    let mut state = crate::AppState::with_product_db_at(data.into(), Some(products));
    *state.project.lock().unwrap() = Some(project);
    state.connector = Box::new(Connector(hardware.clone()));
    let leases = data.join("gateway-leases");
    std::fs::create_dir_all(&leases).unwrap();
    state.diagnostic_lease_dir = Some(leases);
    state.system2_timing = SessionTiming {
        connection_timeout: Duration::from_millis(30),
        response_timeout: Duration::from_millis(5),
        poll_interval: Duration::from_millis(1),
        max_transition: Duration::from_millis(30),
        programming_delay: Duration::ZERO,
        ..Default::default()
    };
    Arc::new(state)
}

// The HTTP production selector remains retained-only. The standalone fixture replaces
// ONLY the private, fully prepared source before the SAME real /start admission/worker.
async fn preview(
    state: &crate::SharedState,
    router: &Router,
    profile: &[u8],
    retained: bool,
) -> Value {
    let mut result = prepare(router, f::DEVICE).await;
    if !retained {
        let project = state.project.lock().unwrap();
        let mut inner = state.device_system2.inner.lock().unwrap();
        let p = inner.preparation.as_mut().unwrap();
        let source = state
            .product_db
            .as_ref()
            .unwrap()
            .with_read(|conn| {
                prepare_project_hardware_source(
                    conn,
                    project.as_ref().unwrap(),
                    f::DEVICE,
                    profile,
                    &p.id,
                    RecoveryTrack::KnxBenchBestEffort,
                )
            })
            .unwrap()
            .unwrap();
        let summary = serde_json::to_value(source.preview_summary().unwrap()).unwrap();
        for (key, value) in summary.as_object().unwrap() {
            result[key] = value.clone();
        }
        p.source = source;
    }
    result
}

#[derive(Clone, Copy)]
enum Edit {
    None,
    Parameter,
    InvalidParameter,
    Links,
    Flags,
    Reference,
    Nonclass,
    Quarantine,
    Address,
}
async fn run(namespace: u8, retained: bool, edit: Edit, saved: bool, marked: bool) {
    let input = f::inputs(namespace, retained);
    let data = input.dir.path();
    let mut project = input.project.clone();
    project.devices.get_mut(f::DEVICE).unwrap().address = Some(TARGET.parse().unwrap());
    project
        .devices
        .get_mut(f::DEVICE)
        .unwrap()
        .commissioning
        .medium_config_loaded = true;
    let hardware = Arc::new(Hardware {
        device: old_device(),
        paused: AtomicBool::new(true),
        reached: Notify::new(),
        release: Notify::new(),
    });
    let products = data.join("caption-products.sqlite");
    let profile = input.profile.clone();
    if saved && retained {
        // Native save portability requires an ACTUAL retained package. Source-only
        // unsaved positives above remain independent; do not bypass the save policy.
        knx_productdb::install_package(&input.conn, "portable-original.knxprod", &input.package)
            .unwrap();
    }
    drop(input.conn);
    let state = state_for(data, products.clone(), project, &hardware);
    let router = crate::app(state.clone(), None);
    if marked {
        mark(&state);
    }
    let path = data.join("caption.knxdb");
    if saved {
        crate::domain::save_project_as(&state, &path).unwrap();
    }
    let preparation = preview(&state, &router, &profile, retained).await;
    http(
        &router,
        "/api/device-system2/start",
        Some(start(&preparation)),
        200,
    )
    .await;
    tokio::time::timeout(Duration::from_secs(60), hardware.reached.notified())
        .await
        .expect("actual first data write after original admission");
    let root = data.join(super::super::super::ROOT_NAME);
    let id = std::fs::read_to_string(root.join("selected")).unwrap();
    let operation = root.join(&id);
    let original_bytes = std::fs::read(operation.join("observed.json")).unwrap();
    let original = read_bundle(&operation.join("observed.json")).unwrap();
    assert_eq!(original.format, 3);
    assert!(!original.binding.simulated);
    let changed = if matches!(edit, Edit::Nonclass) {
        f::changed(&input.package, |s| {
            s.replace("Name=\"Synthetic\"", "Name='Revised caption'")
        })
    } else {
        f::caption(&input.package)
    };
    let replacement = data.join("mounted-replacement.sqlite");
    f::replacement(&replacement, &changed, &input.package, false);
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
                        |r| r.get(0),
                    )
                    .unwrap();
                assert_ne!(winner, original.binding.source_sha256);
                assert!(
                    knx_app::system2_validation_preparation::admit_current_project_hardware(
                        conn,
                        project.as_ref().unwrap(),
                        f::DEVICE,
                        &original
                    )
                    .is_err()
                );
                if matches!(edit, Edit::InvalidParameter) {
                    // Prove the changed-caption branch before injecting unsupported current input.
                    assert_eq!(
                        knx_app::system2_validation_preparation::current_project_hardware_parts(
                            conn,
                            project.as_ref().unwrap(),
                            f::DEVICE,
                            &original,
                        )
                        .unwrap(),
                        ProgrammingParts::ALL,
                        "caption comparison before malformed current input: namespace={namespace} retained={retained} marked={marked}"
                    );
                }
                Ok::<_, std::io::Error>(())
            })
            .unwrap()
            .unwrap();
    }
    {
        let mut guard = state.project.lock().unwrap();
        let project = guard.as_mut().unwrap();
        match edit {
            Edit::Parameter | Edit::InvalidParameter => {
                let raw = if matches!(edit, Edit::InvalidParameter) {
                    "not-a-number"
                } else {
                    "772"
                };
                project.installations[0]
                    .parameters
                    .iter_mut()
                    .find(|p| p.source.ets_id == format!("{}_R", f::PROGRAM))
                    .unwrap()
                    .raw = raw.into()
            }
            Edit::Links => {
                project
                    .devices
                    .com_object_mut(knx_core::ComObjectInstanceId(1))
                    .unwrap()
                    .links[0]
                    .direction = knx_core::flags::Direction::Receive
            }
            Edit::Flags => {
                project
                    .devices
                    .com_object_mut(knx_core::ComObjectInstanceId(1))
                    .unwrap()
                    .flags
                    .write = knx_core::provenance::Override::Value(knx_core::provenance::Resolved {
                    value: false,
                    layer: knx_core::provenance::Layer::UserEdit,
                })
            }
            Edit::Reference => project
                .devices
                .get_mut(f::DEVICE)
                .unwrap()
                .product_ref
                .push('!'),
            Edit::Address => {
                project.devices.get_mut(f::DEVICE).unwrap().address =
                    Some("1.1.26".parse().unwrap())
            }
            _ => project.devices.get_mut(f::DEVICE).unwrap().name.push('!'),
        }
    }
    if matches!(edit, Edit::Quarantine) {
        let owner = state.product_db.as_ref().unwrap();
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            owner.with_read::<(), std::io::Error>(|_| panic!("caption test poisons owner"))
        }));
    }
    hardware.release.notify_one();
    super::failure_diagnostic::settled_slow(&state, &router, "finished").await;
    let status = http(&router, "/api/device-system2/status", None, 200).await;
    assert_eq!(status["downloadResult"], "verified");
    assert_eq!(
        status["projectSettlement"],
        if matches!(edit, Edit::Address) {
            "deviceMissing"
        } else if saved {
            "persisted"
        } else {
            "sessionOnly"
        }
    );
    let expected = match edit {
        Edit::Parameter => [true, true, false, false, true],
        Edit::Links | Edit::Flags => [true, true, true, false, true],
        Edit::InvalidParameter | Edit::Reference | Edit::Nonclass | Edit::Quarantine => {
            [marked, false, false, false, true]
        }
        Edit::Address => [marked, marked, marked, marked, true],
        Edit::None => [true, true, true, true, true],
    };
    assert_eq!(
        flags(&state),
        expected,
        "terminal flags: namespace={namespace} retained={retained} marked={marked}"
    );
    assert_eq!(
        std::fs::read(operation.join("observed.json")).unwrap(),
        original_bytes
    );
    let store = OperationStore::open(&operation.join("journal"), &original.binding).unwrap();
    assert_eq!(store.outcome(), LocalOutcome::Finished);
    assert_eq!(store.progress().download.outcome, LocalOutcome::Finished);
    assert_eq!(
        store.progress().download.result,
        knx_app::property_validation::DownloadResult::Verified
    );
    assert_eq!(store.recovery().unwrap().original_bundle, original);
    let confirmations = knx_app::system2_validation_preparation::HardwareRecoveryConfirmations {
        recovery: &original.restore_phrase(),
        download: &required_download(&original),
        risk: &required_release(&original),
    };
    knx_app::system2_validation_preparation::validate_project_hardware_recovery(
        &original,
        &store,
        &confirmations,
    )
    .unwrap();
    let bad = knx_app::system2_validation_preparation::HardwareRecoveryConfirmations {
        recovery: "wrong",
        ..confirmations
    };
    assert!(
        knx_app::system2_validation_preparation::validate_project_hardware_recovery(
            &original, &store, &bad
        )
        .is_err()
    );
    let facts = store.progress().download;
    drop(store);
    let report = http(
        &router,
        &format!("/api/device-system2/report?operationId={id}"),
        None,
        200,
    )
    .await;
    assert_eq!(report["download"]["result"], "verified");
    assert_eq!(flags(&state), expected, "report does not settle again");
    if saved {
        // One editor at a time: stale concurrent history revisions MUST refuse writes.
        for kind in ["live", "recovered"] {
            let editor = if kind == "live" {
                state.clone()
            } else {
                let reopened = Arc::new(crate::AppState::with_product_db_at(
                    data.join("project-reader"),
                    Some(products.clone()),
                ));
                let progress =
                    crate::domain::detached_progress(crate::load_progress::LoadKind::Open, &path);
                crate::domain::open_native_project(&reopened, &path, true, &progress).unwrap();
                assert!(reopened.device_system2.status().unwrap().is_none());
                reopened
            };
            assert_eq!(
                flags(&editor),
                expected,
                "working journal, not old saved root"
            );
            crate::domain::undo_impl(&editor).unwrap();
            assert_eq!(
                flags(&editor),
                [false, false, false, false, true],
                "{kind} undo cannot restore old claims"
            );
            crate::domain::redo_impl(&editor).unwrap();
            // Undo's inverse may restore the NEW settled snapshot, never stale credit
            // for a part this run deliberately left pending.
            assert_eq!(
                flags(&editor),
                expected,
                "{kind} redo must restore only new settled claims"
            );
        }
    }
    let reopened_store =
        OperationStore::open(&operation.join("journal"), &original.binding).unwrap();
    assert_eq!(reopened_store.progress().download, facts);
    drop(reopened_store);
    drop(router);
    drop(state);
    if saved {
        // Retire the original root owner before an ACTUAL new service reopens it.
        let restarted = Arc::new(crate::AppState::with_product_db_at(
            data.into(),
            Some(products),
        ));
        let progress =
            crate::domain::detached_progress(crate::load_progress::LoadKind::Open, &path);
        crate::domain::open_native_project(&restarted, &path, true, &progress).unwrap();
        let status = restarted.device_system2.status().unwrap().unwrap();
        assert_eq!(
            status.project_settlement,
            super::super::super::settlement::Receipt::NotRecorded
        );
        assert_eq!(
            flags(&restarted),
            expected,
            "service/project reopen never replays settlement"
        );
    }
}
fn required_download(bundle: &knx_app::property_validation_bundle::PreparedBundle) -> String {
    knx_core::commissioning::mutation::required_confirmation_phrase(
        bundle.plan().unwrap().target(),
        knx_core::commissioning::mutation::WriteScope::Download,
    )
}
fn required_release(bundle: &knx_app::property_validation_bundle::PreparedBundle) -> String {
    knx_net::commissioning::hardware_release::required_release_phrase(
        bundle.plan().unwrap().target(),
    )
}

#[tokio::test]
async fn caption_mounted_retained20_verified_after_source_replacement() {
    run(20, true, Edit::None, false, false).await;
}
#[tokio::test]
async fn caption_mounted_retained23_verified_after_source_replacement() {
    run(23, true, Edit::None, false, false).await;
}
#[tokio::test]
async fn caption_mounted_standalone20_verified_after_source_replacement() {
    run(20, false, Edit::None, false, false).await;
}
#[tokio::test]
async fn caption_mounted_standalone23_verified_after_source_replacement() {
    run(23, false, Edit::None, false, false).await;
}
#[tokio::test]
async fn caption_mounted_saved_success_selective_and_refused_history_survive_fresh_server_reopen() {
    for retained in [false, true] {
        for edit in [Edit::None, Edit::Parameter, Edit::Reference] {
            run(20, retained, edit, true, true).await;
        }
    }
}
#[tokio::test]
async fn caption_mounted_links_flags_nonclass_and_quarantined_owner() {
    for edit in [Edit::Links, Edit::Flags, Edit::Nonclass, Edit::Quarantine] {
        run(20, true, edit, false, true).await;
    }
}
#[tokio::test]
async fn caption_mounted_changed_current_address_is_not_recredited() {
    run(20, true, Edit::Address, false, true).await;
}

#[tokio::test]
async fn caption_mounted_standalone_malformed_current_parameter_refuses_credit() {
    for namespace in [20, 23] {
        for marked in [false, true] {
            run(namespace, false, Edit::InvalidParameter, false, marked).await;
        }
    }
}

#[tokio::test]
async fn caption_mounted_retained_malformed_current_parameter_refuses_credit() {
    for namespace in [20, 23] {
        for marked in [false, true] {
            run(namespace, true, Edit::InvalidParameter, false, marked).await;
        }
    }
}
