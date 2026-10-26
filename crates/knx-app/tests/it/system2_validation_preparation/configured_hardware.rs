//! Synthetic exact-identity fixtures exercise software admission, never a real device.
use super::*;
mod admission;
mod failures;
mod pid7_policy;
mod pid7_two_octet;
mod pid7_width;
use knx_app::system2_validation_preparation::{
    admit_current_project_hardware, execute_project_hardware, observe_project_hardware,
    restore_project_hardware, HardwareConfirmations, HardwareRecoveryConfirmations,
};
use knx_app::system2_validation_preparation::{prepare_project_hardware_source, RecoveryTrack};
use knx_core::commissioning::mutation::{required_confirmation_phrase, TargetKind, WriteScope};
use knx_net::{
    cemi::{ApplicationService, Destination, Tpci},
    client::{BusError, TunnelEvent},
    management::ManagementTransport,
};
use tokio::sync::broadcast;

// Production hardware branch against an independent persistent simulator, never sockets.
struct Hardware<'a>(&'a SimulatedDevice);
impl ManagementTransport for Hardware<'_> {
    fn assigned_address(&self) -> IndividualAddress {
        ManagementTransport::assigned_address(self.0)
    }
    fn subscribe(&self) -> broadcast::Receiver<TunnelEvent> {
        ManagementTransport::subscribe(self.0)
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
        ManagementTransport::send_frame(self.0, destination, transport, service).await
    }
}

fn old_device(
    conn: &knx_productdb::Connection,
    project: &knx_core::Project,
    profile: &[u8],
) -> SimulatedDevice {
    old_device_with_config(conn, project, profile, SimulatorConfig::default())
}
fn old_device_with_config(
    conn: &knx_productdb::Connection,
    project: &knx_core::Project,
    profile: &[u8],
    config: SimulatorConfig,
) -> SimulatedDevice {
    let mut source = project_candidate(conn, project, profile).bundle;
    source.binding.serial = [0, 1, 2, 3, 4, 5];
    let device = synthetic_old_device(&source, config);
    device.preset_property(
        0,
        knx_core::commissioning::properties::PID_MANUFACTURER_ID,
        &[0, 1],
    );
    device.preset_property(0, PID_HARDWARE_TYPE, &[0, 1, 0, 0, 0, 91]);
    device.preset_property(3, PID_PROGRAM_VERSION, &[0, 1, 146, 5, 4]);
    for (object, mut state) in [
        SimulatedSystem2Object::AddressTable,
        SimulatedSystem2Object::AssociationTable,
        SimulatedSystem2Object::Application,
    ]
    .into_iter()
    .zip(device.system2_snapshot().objects)
    {
        if let Some(task) = state.task_segment.as_mut() {
            task.manufacturer = 1;
            task.application = 0x9205;
            task.version = 4;
        }
        device.preset_system2_object(object, state);
    }
    device
}

#[tokio::test]
async fn configured_hardware_executes_and_reopened_recovery_restores_independent_old_state() {
    let (dir, conn, project, profile) = inputs();
    let device = old_device(&conn, &project, &profile);
    let old = device.system2_snapshot();
    let transport = Hardware(&device);
    let source = prepare_project_hardware_source(
        &conn,
        &project,
        knx_core::DeviceId(1),
        &profile,
        "configured-test-1",
        RecoveryTrack::KnxBenchBestEffort,
    )
    .unwrap();
    let cancel = AtomicBool::new(false);
    let bundle = observe_project_hardware(&transport, source, integrated_timing(), &cancel)
        .await
        .unwrap();
    assert!(!bundle.binding.simulated);
    assert_eq!(
        device.system2_snapshot(),
        old,
        "preparation must be read-only"
    );
    assert_eq!(write_bytes(&bundle, 15360), [1, 2]);
    assert_eq!(
        write_bytes(&bundle, 278),
        [3, 0x12, 0x34, 0x10, 0x35, 0x10, 0x36]
    );
    assert_eq!(
        write_bytes(&bundle, 1024),
        [2, 0xa7, 0x12, 0x21, 0, 0x34, 0xf7, 7]
    );
    let admission =
        admit_current_project_hardware(&conn, &project, knx_core::DeviceId(1), &bundle).unwrap();
    if let Some(root) = std::env::var_os("KNXBENCH_CONFIGURED_SYSTEM2_FIXTURE_DIR") {
        let root = std::path::PathBuf::from(root);
        std::fs::create_dir_all(&root).unwrap();
        let native = knx_store::open_and_migrate(&root.join("project.knxdb")).unwrap();
        knx_store::save_project(&native, &project).unwrap();
        conn.execute(
            "VACUUM INTO ?1",
            [root.join("products.sqlite").to_str().unwrap()],
        )
        .unwrap();
        knx_app::property_validation::write_prepared_bundle(&root.join("prepared.json"), &bundle)
            .unwrap();
        std::fs::write(root.join("profile.json"), &profile).unwrap();
    }
    let operation = dir.path().join("operation");
    let mut store = OperationStore::create(&operation, bundle.binding.clone()).unwrap();
    let progress = store.subscribe_progress();
    assert_eq!(progress.borrow().completed_steps, 0);
    assert!(!progress.borrow().backup_available);
    let operation_phrase = bundle.consent_phrase();
    let download = required_confirmation_phrase(device.address(), WriteScope::Download);
    let release =
        knx_net::commissioning::hardware_release::required_release_phrase(device.address());
    let hypothesis = bundle.acceptance_phrase();
    let confirmations = HardwareConfirmations {
        operation: &operation_phrase,
        hypothesis: &hypothesis,
        download: &download,
        risk: &release,
    };
    let result = execute_project_hardware(
        &transport,
        &admission,
        &mut store,
        integrated_timing(),
        &confirmations,
        &cancel,
    )
    .await;
    assert!(
        result.is_ok(),
        "configured download must execute: {}",
        result.err().map(|e| e.to_string()).unwrap_or_default()
    );
    assert_eq!(store.outcome(), LocalOutcome::Finished);
    assert_eq!(
        progress.borrow().completed_steps,
        bundle.normalized_plan.steps.len()
    );
    assert!(progress.borrow().backup_available);
    let original_phrases = store.recovery_phrases().unwrap();
    assert_eq!(original_phrases.download_phrase, download);
    assert_eq!(original_phrases.release_phrase, release);
    assert_eq!(original_phrases.recovery_phrase, bundle.restore_phrase());
    let recovery = store.recovery().unwrap().clone();
    assert_eq!(
        recovery
            .allocations
            .iter()
            .map(|r| r.data.len())
            .sum::<usize>(),
        48
    );
    assert_eq!(device.memory(15360, 3), [Some(1), Some(2), Some(0xa5)]);
    drop(store);
    let mut reopened = OperationStore::open(&operation, &bundle.binding).unwrap();
    assert_eq!(reopened.recovery_phrases(), Some(original_phrases));
    let recovery_phrase = bundle.restore_phrase();
    let recovery_confirmations = HardwareRecoveryConfirmations {
        recovery: &recovery_phrase,
        download: &download,
        risk: &release,
    };
    restore_project_hardware(
        &transport,
        &bundle,
        &mut reopened,
        integrated_timing(),
        &recovery_confirmations,
        &cancel,
    )
    .await
    .unwrap();
    assert_eq!(reopened.outcome(), LocalOutcome::Restored);
    assert!(!reopened.progress().restore_available);
    assert!(reopened.recovery_phrases().is_none());
    assert_eq!(reopened.recovery(), Some(&recovery));
    assert_eq!(device.system2_snapshot(), old);
}

use super::fixture::configured_inputs as inputs;

#[test]
fn configured_hardware_source_accepts_selected_nondefault_configuration() {
    let (_dir, conn, project, profile) = inputs();
    let result = prepare_project_hardware_source(
        &conn,
        &project,
        knx_core::DeviceId(1),
        &profile,
        "configured-test-1",
        RecoveryTrack::KnxBenchBestEffort,
    );
    assert!(
        result.is_ok(),
        "valid configured hardware source must prepare: {}",
        result.err().map(|e| e.to_string()).unwrap_or_default()
    );
}
