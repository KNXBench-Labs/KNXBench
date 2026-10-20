//! Explicit immutable-bundle property validation: offline preview or simulator.
//!
//! There is intentionally no gateway client in this module. A JSON bundle and
//! an acknowledgement cannot supply the missing physical source/recovery contract.
use knx_app::{
    property_validation::{self as service, OperationStore},
    property_validation_bundle::PreparedBundle,
};
use knx_core::commissioning::{
    load_state::LoadState,
    properties::{PID_HARDWARE_TYPE, PID_MANUFACTURER_ID, PID_PROGRAM_VERSION},
    property_download::PropertyOperation,
};
use knx_net::commissioning::{
    simulator::{SimulatedDevice, SimulatorConfig},
    SessionTiming,
};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    process::ExitCode,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
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
            flag @ ("--bundle" | "--gateway" | "--operation" | "--confirm" | "--accept-hypothesis" | "--recover" | "--confirm-recovery" | "--report") => {
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
    println!("STRONG HYPOTHESIS SIMULATOR TESTED — not physical compatibility.\nTarget {} plan {}\nProfile {} version {}\nSource {}\n{} ordered operations.",
        bundle.binding.target, bundle.binding.plan_sha256, bundle.binding.profile_id, bundle.binding.profile_version, bundle.binding.source_sha256, bundle.normalized_plan.steps.len());
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
        println!("Preview only; no connection or writes.\nConfirmation: {expected}\nHypothesis acceptance: {}\nRecovery is same-application/procedure/allocation-shape reinstallation, not raw allocator rollback.\nUse --simulate for an isolated synthetic device; no real transport is enabled.", bundle.acceptance_phrase());
        return Ok(());
    }
    if confirmation.copied() != Some(expected.as_str()) {
        return Err("exact target/plan-bound confirmation required".into());
    }
    if values.get("--accept-hypothesis").copied() != Some(bundle.acceptance_phrase().as_str()) {
        return Err("exact target/plan-bound strong-hypothesis acceptance required".into());
    }
    if !simulate {
        // Do not acquire gateway/activity leases, construct a client, resolve a
        // gateway or create a receipt before legitimate upstream admission exists.
        // A future real adapter must use device_diagnostics::gateway_lease and
        // normal commissioning activity admission before opening the one client.
        return Err("physical-source-recovery-contract-unavailable: prepared JSON and consent alone cannot authorize real writes; no socket opened".into());
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
