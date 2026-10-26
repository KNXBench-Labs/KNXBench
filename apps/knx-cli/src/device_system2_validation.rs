//! Consent-gated property validation: preview, simulator, or a real write.
//!
//! The real-write path (ADR-0152) reads the device serial, rebinds the bundle
//! and requires the download plus release confirmations before any frame.
use knx_app::{
    commissioning_activity::{OneShotLog, WriteOutcome},
    property_validation::{self as service, OperationStore},
    property_validation_bundle::PreparedBundle,
};
use knx_core::commissioning::{
    authorisation::AuthorisationPlan,
    load_state::LoadState,
    mutation::{required_confirmation_phrase, WriteScope},
    properties::{PID_HARDWARE_TYPE, PID_MANUFACTURER_ID, PID_PROGRAM_VERSION},
    property_download::PropertyOperation,
};
use knx_net::commissioning::{
    hardware_release::required_release_phrase,
    simulator::{SimulatedDevice, SimulatorConfig},
    SessionTiming,
};
use knx_net::{BusConnection, KnxNetIpClient, ManagementSession};
use std::{
    collections::BTreeMap,
    net::SocketAddrV4,
    path::{Path, PathBuf},
    process::ExitCode,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

pub fn run(args: &[String]) -> ExitCode {
    match execute(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
fn execute(args: &[String]) -> Result<(), String> {
    let mut values = BTreeMap::new();
    let mut simulate = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--simulate" if !simulate => simulate = true,
            flag @ ("--bundle" | "--gateway" | "--operation" | "--confirm" | "--accept-hypothesis" | "--recover" | "--confirm-recovery" | "--report" | "--download-confirm" | "--release-confirm" | "--activity-history" | "--project" | "--device-id" | "--product-db") => {
                let value = args.get(index + 1).filter(|v| !v.starts_with("--")).ok_or_else(|| format!("{flag} needs a value"))?;
                if values.insert(flag, value.as_str()).is_some() { return Err(format!("duplicate {flag}")); }
                index += 1;
            }
            _ => return Err("unsupported property-validation argument; keys and arbitrary memory ranges are not accepted".into()),
        }
        index += 1;
    }
    if simulate && values.contains_key("--gateway") {
        return Err(
            "--simulate cannot be combined with --gateway; simulator never opens a socket".into(),
        );
    }
    let recover = values.contains_key("--recover");
    if recover && (values.contains_key("--operation") || values.contains_key("--confirm")) {
        return Err(
            "--recover uses its own existing operation and separate --confirm-recovery".into(),
        );
    }
    if !recover && values.contains_key("--confirm-recovery") {
        return Err("--confirm-recovery requires --recover".into());
    }
    let confirmation = values.get(if recover {
        "--confirm-recovery"
    } else {
        "--confirm"
    });
    if confirmation.is_some() && !simulate && !values.contains_key("--gateway") {
        return Err("confirmation requires explicit --simulate or --gateway".into());
    }
    if confirmation.is_some() && !values.contains_key("--accept-hypothesis") {
        return Err(
            "exact target/plan --accept-hypothesis is required in addition to ordinary consent"
                .into(),
        );
    }
    if confirmation.is_none()
        && (values.contains_key("--accept-hypothesis") || values.contains_key("--report"))
    {
        return Err("hypothesis acceptance/report requires this operation's confirmation".into());
    }
    let bundle_path = Path::new(
        values
            .get("--bundle")
            .ok_or("--bundle <immutable prepared.json> required")?,
    );
    let bundle = service::read_bundle(bundle_path).map_err(|e| e.to_string())?;
    if bundle.format != 3
        && ["--project", "--device-id", "--product-db"]
            .iter()
            .any(|key| values.contains_key(key))
    {
        return Err("project configuration arguments require a separately prepared configured hardware bundle; defaults are never substituted".into());
    }
    let support = if bundle.binding.simulated {
        "STRONG HYPOTHESIS — synthetic simulator candidate"
    } else {
        "EXPERIMENTAL BEST-EFFORT — hardware compatibility and physical recovery unverified"
    };
    println!(
        "{support}.\nTarget {} plan {}\nProfile {} version {}\nSource {}\n{} ordered operations.",
        bundle.binding.target,
        bundle.binding.plan_sha256,
        bundle.binding.profile_id,
        bundle.binding.profile_version,
        bundle.binding.source_sha256,
        bundle.normalized_plan.steps.len()
    );
    if bundle.format == 3 {
        let preview = knx_app::system2_validation_preparation::configured_hardware_preview(&bundle)
            .map_err(|e| e.to_string())?;
        println!("Configured request: {} parameter values, {} group links, {} flag override rows; {} allocation octets, {} written, {} NOT written.", preview.parameter_values, preview.group_links, preview.flag_overrides, preview.allocation_octets, preview.data_octets, preview.not_written_octets);
        for warning in preview.warnings {
            println!("WARNING: {warning}");
        }
    }
    for (i, step) in bundle.normalized_plan.steps.iter().enumerate() {
        // Show the complete procedure locally, without echoing image/parameter octets.
        let kind = match step.operation {
            knx_app::property_validation_bundle::OperationRecord::Connect => {
                "connect/read identity"
            }
            knx_app::property_validation_bundle::OperationRecord::HardwareCheck { .. } => {
                "hardware/serial/application/recovery reads then VerifyMode"
            }
            knx_app::property_validation_bundle::OperationRecord::LoadControl { .. } => {
                "load control"
            }
            knx_app::property_validation_bundle::OperationRecord::WriteMemory {
                ref data, ..
            } => {
                println!(
                    "  {i}: source {} verified memory write ({} octets)",
                    step.source_index,
                    data.len()
                );
                continue;
            }
            knx_app::property_validation_bundle::OperationRecord::ToolMetadata { .. } => {
                "tool-only metadata (no wire operation)"
            }
            knx_app::property_validation_bundle::OperationRecord::Disconnect => {
                "verify complete allocation/control outcome and disconnect"
            }
        };
        println!("  {i}: source {} {kind}", step.source_index);
    }
    let expected = if recover {
        bundle.restore_phrase()
    } else {
        bundle.consent_phrase()
    };
    if confirmation.is_none() {
        let target = bundle.plan().map_err(|e| e.to_string())?.target();
        println!("Preview only; no connection or writes.\nConfirmation: {expected}\nHypothesis acceptance: {}\nOrdinary download: {}\nBest-effort release: {}\nRecovery is experimental same-application/source-shape replay, NOT raw allocator rollback or guaranteed restoration.\n--simulate is synthetic only. --gateway admits an explicitly consented best-effort physical operation; configured bundles additionally require fresh --project --device-id --product-db on execute.", bundle.acceptance_phrase(), required_confirmation_phrase(target, WriteScope::Download), required_release_phrase(target));
        return Ok(());
    }
    if confirmation.copied() != Some(expected.as_str()) {
        return Err("exact target/plan-bound confirmation required".into());
    }
    if values.get("--accept-hypothesis").copied() != Some(bundle.acceptance_phrase().as_str()) {
        return Err("exact target/plan-bound strong-hypothesis acceptance required".into());
    }
    if !simulate {
        if bundle.format == 2 {
            return Err("project-v2 bundles are intrinsically simulator-only; prepare a separate configured hardware operation with system2-project".into());
        }
        let gateway: SocketAddrV4 = values
            .get("--gateway")
            .ok_or("--gateway <ip:port> required for a real write")?
            .parse()
            .map_err(|_| "invalid --gateway address")?;
        let download = values
            .get("--download-confirm")
            .ok_or("--download-confirm <phrase> required for a real write")?;
        let release = values
            .get("--release-confirm")
            .ok_or("--release-confirm <phrase> required for a real write")?;
        let history = values
            .get("--activity-history")
            .ok_or("--activity-history <path> required for a real write")?;
        let target = bundle.plan().map_err(|e| e.to_string())?.target();
        if *download != required_confirmation_phrase(target, WriteScope::Download) {
            return Err("exact download confirmation required".into());
        }
        if *release != required_release_phrase(target) {
            return Err("exact best-effort release confirmation required".into());
        }
        let operation = PathBuf::from(
            values
                .get(if recover { "--recover" } else { "--operation" })
                .ok_or("--operation <new-directory> required for a real write")?,
        );
        let report = values.get("--report").map(PathBuf::from);
        if report.as_ref().is_some_and(|p| p.exists()) {
            return Err("report destination already exists; no execution started".into());
        }
        let current = if bundle.format == 3 && !recover {
            let project_conn = crate::open_native_for_reading(Path::new(
                values
                    .get("--project")
                    .ok_or("configured execution requires --project")?,
            ))
            .map_err(|e| e.to_string())?;
            let project = knx_store::load_project(&project_conn).map_err(|e| e.to_string())?;
            let device = values
                .get("--device-id")
                .ok_or("configured execution requires --device-id")?
                .parse::<u32>()
                .map_err(|_| "invalid device id")?;
            let products = knx_productdb::open_read_only(Path::new(
                values
                    .get("--product-db")
                    .ok_or("configured execution requires --product-db")?,
            ))
            .map_err(|e| e.to_string())?;
            Some(
                knx_app::system2_validation_preparation::admit_current_project_hardware(
                    &products.conn,
                    &project,
                    knx_core::DeviceId(device),
                    &bundle,
                )
                .map_err(|e| e.to_string())?,
            )
        } else {
            None
        };
        let configured_store = if bundle.format == 3 {
            let store = if recover {
                OperationStore::open(&operation, &bundle.binding)
            } else {
                OperationStore::create(&operation, bundle.binding.clone())
            }
            .map_err(|e| e.to_string())?;
            if recover {
                knx_app::system2_validation_preparation::validate_project_hardware_recovery(
                    &bundle,
                    &store,
                    &knx_app::system2_validation_preparation::HardwareRecoveryConfirmations {
                        recovery: confirmation.copied().expect("confirmation checked"),
                        download,
                        risk: release,
                    },
                )
                .map_err(|e| e.to_string())?;
            }
            Some(store)
        } else {
            None
        };
        return run_hardware(
            &bundle,
            gateway,
            history,
            operation,
            report,
            recover,
            confirmation.copied().expect("confirmation checked above"),
            download,
            release,
            current,
            configured_store,
        );
    }
    if !bundle.binding.simulated {
        return Err("--simulate requires a bundle explicitly marked simulated; do not relabel physical evidence".into());
    }
    let operation = PathBuf::from(
        values
            .get(if recover { "--recover" } else { "--operation" })
            .ok_or("--operation <new-directory> required for simulator execution")?,
    );
    let report = values.get("--report").map(PathBuf::from);
    if report.as_ref().is_some_and(|p| p.exists()) {
        return Err("report destination already exists; no execution started".into());
    }
    let mut store = if recover {
        OperationStore::open(&operation, &bundle.binding)
    } else {
        OperationStore::create(&operation, bundle.binding.clone())
    }
    .map_err(|e| e.to_string())?;
    let device = simulator(&bundle)?;
    println!("SIMULATED: fresh synthetic device seeded with declared identity/pointers and {} memory; not hardware or retained device state.", if recover { "prepared-plan" } else { "0xA5" });
    if recover {
        seed_plan_memory(&device, &bundle)?;
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
    let result = runtime.block_on(async {
        let cancel = Arc::new(AtomicBool::new(false));
        let flag = cancel.clone();
        let signal = tokio::spawn(async move {
            if tokio::signal::ctrl_c().await.is_ok() {
                flag.store(true, Ordering::SeqCst);
            }
        });
        // Do not select/drop the operation future on Ctrl-C: Net cooperatively
        // stops at its next boundary and returns through durable checkpoints.
        let result = if recover {
            service::restore_simulated(
                &device,
                &bundle,
                &mut store,
                SessionTiming::default(),
                &cancel,
                &expected,
            )
            .await
        } else {
            service::execute_simulated(
                &device,
                &bundle,
                &mut store,
                SessionTiming::default(),
                &cancel,
            )
            .await
        };
        signal.abort();
        result
    });
    if let Some(path) = report {
        store
            .write_shareable_report(&path)
            .map_err(|e| e.to_string())?;
    }
    println!("SIMULATED outcome: {:?}; mutation possible: {}. Private recovery and local protocol retained. Review shareable report before sending; no upload occurred.", store.outcome(), store.mutation_possible());
    result.map_err(|e| e.to_string())
}
fn simulator(bundle: &PreparedBundle) -> Result<SimulatedDevice, String> {
    let plan = bundle.plan().map_err(|e| e.to_string())?;
    let device = SimulatedDevice::with_config_at(
        plan.target(),
        SimulatorConfig {
            mask_version: bundle.binding.mask,
            serial_number: Some(bundle.binding.serial),
            ..Default::default()
        },
    );
    device.preset_property(
        0,
        PID_MANUFACTURER_ID,
        &bundle.binding.manufacturer.to_be_bytes(),
    );
    device.preset_property(0, PID_HARDWARE_TYPE, &bundle.binding.hardware);
    device.preset_property(3, PID_PROGRAM_VERSION, &bundle.binding.application);
    let mut states =
        BTreeMap::<u8, knx_net::commissioning::simulator::SimulatedSystem2ObjectState>::new();
    for step in plan.steps() {
        if let PropertyOperation::LoadControl { object, payload } = &step.operation {
            let state = states.entry(*object).or_insert_with(|| {
                knx_net::commissioning::simulator::SimulatedSystem2ObjectState {
                    load_state: LoadState::Loaded,
                    ..Default::default()
                }
            });
            let b = payload.octets();
            if b[0] != 3 {
                continue;
            }
            let word = |i| u16::from_be_bytes([b[i], b[i + 1]]);
            match b[1] {
                0 | 1 => {
                    state.allocations.push(
                        knx_net::commissioning::simulator::SimulatedAbsoluteAllocation {
                            subtype: b[1],
                            start: word(2),
                            length: word(4),
                            access: b[6],
                            memory_type: b[7],
                            attributes: b[8],
                        },
                    );
                    state.table_reference = u32::from(word(2));
                }
                2 => {
                    state.task_segment =
                        Some(knx_net::commissioning::simulator::SimulatedTaskSegment {
                            start: word(2),
                            pei_type: b[4],
                            manufacturer: word(5),
                            application: word(7),
                            version: b[9],
                        });
                    state.table_reference = u32::from(word(2));
                }
                5 => {
                    state.task_control_2 =
                        Some(knx_net::commissioning::simulator::SimulatedTaskControl2 {
                            callback: word(2),
                            communication_objects: word(4),
                            segment_1: word(6),
                            segment_2: word(8),
                        })
                }
                _ => return Err("unsupported synthetic resource control".into()),
            }
        }
    }
    for (object, state) in states {
        let owner = match object {
            1 => knx_net::commissioning::simulator::SimulatedSystem2Object::AddressTable,
            2 => knx_net::commissioning::simulator::SimulatedSystem2Object::AssociationTable,
            3 => knx_net::commissioning::simulator::SimulatedSystem2Object::Application,
            _ => return Err("unsupported simulator resource".into()),
        };
        device.preset_system2_object(owner, state);
    }
    for step in plan.steps() {
        if let PropertyOperation::LoadControl { payload, .. } = &step.operation {
            let b = payload.octets();
            if b[0] == 3 && b[1] <= 1 {
                device.preset_memory(
                    u32::from(u16::from_be_bytes([b[2], b[3]])),
                    &vec![0xa5; usize::from(u16::from_be_bytes([b[4], b[5]]))],
                );
            }
        }
    }
    Ok(device)
}
fn seed_plan_memory(device: &SimulatedDevice, bundle: &PreparedBundle) -> Result<(), String> {
    for step in bundle.plan().map_err(|e| e.to_string())?.steps() {
        if let PropertyOperation::WriteMemory { address, data, .. } = &step.operation {
            device.preset_memory(u32::from(*address), data);
        }
    }
    Ok(())
}
#[allow(clippy::too_many_arguments)]
fn run_hardware(
    bundle: &PreparedBundle,
    gateway: SocketAddrV4,
    history: &str,
    operation: PathBuf,
    report: Option<PathBuf>,
    recover: bool,
    recovery: &str,
    download: &str,
    release: &str,
    current: Option<knx_app::system2_validation_preparation::CurrentProjectHardware>,
    configured_store: Option<OperationStore>,
) -> Result<(), String> {
    let target = bundle.plan().map_err(|e| e.to_string())?.target();
    // One tunnel at a time; a second live client is refused by the gateway.
    let _lease = knx_app::device_diagnostics::gateway_lease()
        .map_err(|e| format!("gateway lease unavailable: {e}"))?;
    let log = Arc::new(OneShotLog::for_run(PathBuf::from(history)).map_err(|e| e.to_string())?);
    log.ensure_write_available().map_err(|e| e.to_string())?;
    let guard = log
        .start_write("system2Validation", Some(target.to_string()))
        .map_err(|e| e.to_string())?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
    let executed = runtime.block_on(async {
        let tunnel = tokio::time::timeout(
            Duration::from_secs(10),
            KnxNetIpClient::new().connect_tunnel(gateway),
        )
        .await
        .map_err(|_| "gateway timeout".to_string())?
        .map_err(|e| format!("gateway refused/unavailable: {e}"))?;
        // Configured preparation already binds an observed serial. NEVER rebind
        // it at execute/recovery; the executor checks that original identity.
        let rebound = if bundle.format == 3 {
            bundle.clone()
        } else {
            let serial = {
                let mut session = ManagementSession::read_only(
                    &tunnel,
                    target,
                    AuthorisationPlan::Skip,
                    SessionTiming::default(),
                )
                .map_err(|e| format!("read-only session refused: {e}"))?;
                session.connect().await.map_err(|e| format!("connect refused: {e}"))?;
                let read = session.read_serial_number().await
                    .map_err(|e| format!("serial read refused: {e}"))?;
                session.disconnect().await;
                read.octets()
            };
            bundle.rebind_hardware(serial).map_err(|e| e.to_string())?
        };
        let mut store = if let Some(store) = configured_store {
            Ok(store)
        } else if recover {
            OperationStore::open(&operation, &rebound.binding)
        } else {
            OperationStore::create(&operation, rebound.binding.clone())
        }
        .map_err(|e| e.to_string())?;
        let cancel = Arc::new(AtomicBool::new(false));
        let flag = cancel.clone();
        let signal = tokio::spawn(async move {
            if tokio::signal::ctrl_c().await.is_ok() {
                flag.store(true, Ordering::SeqCst);
            }
        });
        // The executor marks the journal at its durable recovery barrier,
        // immediately before the first possible mutation, not here.
        store.on_send_boundary(Box::new(guard.send_marker()));
        let result = if rebound.format == 3 {
            use knx_app::system2_validation_preparation as configured;
            if recover {
                configured::restore_project_hardware(
                    &tunnel, &rebound, &mut store, SessionTiming::default(),
                    &configured::HardwareRecoveryConfirmations {
                        recovery, download, risk: release,
                    },
                    &cancel,
                ).await
            } else {
                configured::execute_project_hardware(
                    &tunnel,
                    current.as_ref().ok_or("fresh configured admission missing")?,
                    &mut store, SessionTiming::default(),
                    &configured::HardwareConfirmations {
                        operation: recovery, hypothesis: &rebound.acceptance_phrase(),
                        download, risk: release,
                    },
                    &cancel,
                ).await
            }
        } else if recover {
            service::restore_hardware(
                &tunnel,
                &rebound,
                &mut store,
                SessionTiming::default(),
                recovery,
                download,
                release,
                &cancel,
            )
            .await
        } else {
            service::execute_hardware(
                &tunnel,
                &rebound,
                &mut store,
                SessionTiming::default(),
                download,
                release,
                &cancel,
            )
            .await
        };
        signal.abort();
        let result = result.map_err(|e| e.to_string());
        // A report failure is reported, but never replaces the device result.
        let reported = report.as_ref().map_or(Ok(()), |path| {
            store.write_shareable_report(path).map_err(|e| e.to_string())
        });
        let _ = tokio::time::timeout(Duration::from_secs(3), tunnel.disconnect()).await;
        println!(
            "HARDWARE outcome: {:?}; mutation possible: {}. Private recovery and local protocol retained; no upload occurred.",
            store.outcome(),
            store.mutation_possible()
        );
        result.map(|()| reported)
    });
    // A refusal before the recorded send boundary witnessed no device write.
    let outcome = match &executed {
        Ok(_) => WriteOutcome::Verified,
        Err(_) if guard.send_possible() => WriteOutcome::EffectUnverified,
        Err(_) => WriteOutcome::NotSent,
    };
    guard.finish(outcome);
    executed.and_then(|reported| reported)
}
