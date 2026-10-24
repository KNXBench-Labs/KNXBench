//! The `knx device diagnostics` command.
//!
//! `knx device diagnostics`: preview or export one selected device's generic
//! identity recipe, or with `--profile system2-declared-segments-v1` its
//! ProductDB-declared memory ranges (ADR-0143). `--simulate` never constructs
//! a network client; without `--confirm-read` nothing is acquired.
use knx_app::device_diagnostics as service;
use knx_core::{DeviceId, DeviceInstance, Language, Project, SourceRef};
use std::{
    collections::BTreeMap,
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
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
fn execute(args: &[String]) -> Result<(), String> {
    let mut values = BTreeMap::new();
    let mut simulate = false;
    let mut confirm = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--simulate" if !simulate => simulate = true,
            "--confirm-read" if !confirm => confirm = true,
            flag @ ("--project" | "--device-id" | "--product-db" | "--gateway" | "--output"
            | "--profile") => {
                let value = args
                    .get(i + 1)
                    .filter(|v| !v.starts_with("--"))
                    .ok_or_else(|| format!("{flag} needs a value"))?;
                if values.insert(flag.to_string(), value.clone()).is_some() {
                    return Err(format!("duplicate {flag}"));
                }
                i += 1;
            }
            _ => {
                return Err(format!(
                "unsupported diagnostic argument {}: no keys, memory ranges or writes are accepted",
                args[i]
            ))
            }
        }
        i += 1;
    }
    let memory_profile = match values.get("--profile").map(String::as_str) {
        None | Some(service::PROFILE) => false,
        Some(service::MEMORY_PROFILE) => true,
        Some(other) => {
            return Err(format!(
                "unknown diagnostic profile {other}; use {} or {}",
                service::PROFILE,
                service::MEMORY_PROFILE
            ))
        }
    };
    if memory_profile && !values.contains_key("--product-db") {
        return Err(format!(
            "--profile {} requires --product-db: its memory ranges come only from the device's product data",
            service::MEMORY_PROFILE
        ));
    }
    let output = PathBuf::from(
        values
            .get("--output")
            .ok_or("--output <new.zip> is required")?,
    );
    if output.exists() {
        return Err("diagnostics-create-new-failed: output exists; nothing acquired".into());
    }
    let (project, device) = if simulate && !memory_profile {
        if values.keys().any(|k| k != "--output" && k != "--profile") {
            return Err(
                "--simulate cannot be combined with real connection or project arguments".into(),
            );
        }
        (synthetic_project(), DeviceId(1))
    } else if simulate && values.contains_key("--gateway") {
        return Err("--simulate cannot be combined with --gateway".into());
    } else {
        let path = Path::new(values.get("--project").ok_or("--project required")?);
        let conn = crate::open_native_for_reading(path).map_err(|e| e.to_string())?;
        let p = knx_store::load_project(&conn).map_err(|e| e.to_string())?;
        let id = values
            .get("--device-id")
            .ok_or("--device-id required")?
            .parse::<u32>()
            .map_err(|_| "invalid device ID")?;
        (p, DeviceId(id))
    };
    let products = if let Some(path) = values.get("--product-db") {
        Some(crate::open_products_db_for_reading(Some(path))?)
    } else {
        None
    };
    let plan = match products.as_ref() {
        Some(db) if memory_profile => Plan::Memory(service::prepare_memory(&project, device, db)?),
        _ => Plan::Identity(service::prepare(&project, device, products.as_ref())?),
    };
    let gateway = if simulate {
        None
    } else {
        let g: std::net::SocketAddrV4 = values
            .get("--gateway")
            .ok_or("--gateway required; no default interface")?
            .parse()
            .map_err(|_| "IPv4 tunnelling endpoint required")?;
        if g.port() == 0
            || g.ip().is_unspecified()
            || g.ip().is_multicast()
            || g.ip().is_broadcast()
        {
            return Err("invalid gateway".into());
        }
        Some(g)
    };
    let via = gateway
        .map(|g| g.to_string())
        .unwrap_or("SIMULATOR (no network)".into());
    match &plan {
        Plan::Identity(plan) => println!("Device {} via {}. Read only: no programming, authorisation/key, Verify Mode, memory or property writes.\nScope: descriptor type 0; Device Object PID12 (2 bytes), PID78 (6 bytes), optional services.\nMemory: not requested, no admitted safe profile. Mask/manufacturer agreement does not identify a product.\nLocal selection reference: {}",
            plan.target.address(),via,plan.reference),
        Plan::Memory(plan) => print_memory_preview(plan, &via),
    }
    println!(
        "{}",
        if confirm {
            "Confirmed acquisition."
        } else {
            "Preview only; add --confirm-read to acquire."
        }
    );
    if !confirm {
        return Ok(());
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
    runtime.block_on(async {
        let cancel=Arc::new(AtomicBool::new(false));let flag=cancel.clone();
        let signal=tokio::spawn(async move{if tokio::signal::ctrl_c().await.is_ok(){flag.store(true,Ordering::SeqCst);}});
        let result=if simulate {
            let d=simulator(&plan);
            Ok(acquire(&d,&plan,&cancel).await)
        } else {
            use knx_net::BusConnection;
            let _lease=service::gateway_lease()?;
            let tunnel=tokio::time::timeout(Duration::from_secs(10),knx_net::KnxNetIpClient::new().connect_tunnel(gateway.unwrap())).await.map_err(|_|"gateway timeout")?.map_err(|e|format!("gateway refused/unavailable: {e}"))?;
            let mut export=acquire(&tunnel,&plan,&cancel).await;
            if !matches!(tokio::time::timeout(Duration::from_secs(3),tunnel.disconnect()).await,Ok(Ok(()))){export.report.stop_reason.get_or_insert(service::TUNNEL_DISCONNECT_UNCONFIRMED.into());}
            Ok::<_,String>(export)
        };
        signal.abort();let export=result?;
        service::write_new_export(&output,&export)?;
        println!("Saved and re-read verified {}{}; this is not a recovery backup.\nReview before sharing with issue@knxbench.com; no upload occurred.",output.display(),if export.report.simulated{" (SIMULATED)"}else{""});Ok(())
    })
}
enum Plan {
    Identity(service::Prepared),
    Memory(service::MemoryPrepared),
}
/// Both profiles through their one application-service entry point.
async fn acquire<T: knx_net::ManagementTransport>(
    transport: &T,
    plan: &Plan,
    cancel: &AtomicBool,
) -> service::Export {
    let build = crate::version_string();
    match plan {
        Plan::Identity(plan) => {
            let report =
                service::acquire(transport, plan, Default::default(), &build, cancel, |r| {
                    let read = r.items.iter().filter(|i| i.completed_at.is_some()).count();
                    eprintln!("Diagnostic progress: {read}/3 logical reads; raw bytes retained.");
                })
                .await;
            service::Export {
                report,
                memory: None,
            }
        }
        Plan::Memory(plan) => {
            let mut last = usize::MAX;
            service::acquire_memory(transport, plan, Default::default(), &build, cancel, |e| {
                let chunks = e.memory.as_ref().map_or(&[][..], |m| &m.chunks[..]);
                let done = chunks.iter().filter(|c| c.completed_at.is_some()).count();
                if done != last {
                    last = done;
                    eprintln!(
                        "Diagnostic progress: {done}/{} memory reads; raw bytes retained.",
                        chunks.len()
                    );
                }
            })
            .await
        }
    }
}
/// The built-in simulator, configured to show exactly the asserted identity.
/// Its memory is the simulator's, not a device's: the archive says SIMULATED.
fn simulator(plan: &Plan) -> knx_net::commissioning::simulator::SimulatedDevice {
    use knx_net::commissioning::simulator::{SimulatedDevice, SimulatorConfig};
    match plan {
        Plan::Identity(_) => {
            let d = SimulatedDevice::with_config(SimulatorConfig {
                mask_version: 0x0701,
                ..Default::default()
            });
            d.preset_property(0, 12, &[0x12, 0x34]);
            d.preset_property(0, 78, &[1, 2, 3, 4, 5, 6]);
            d
        }
        Plan::Memory(plan) => {
            let expected = plan.base().expected;
            let d = SimulatedDevice::with_config_at(
                plan.base().target.address(),
                SimulatorConfig {
                    mask_version: expected.mask.unwrap_or_default(),
                    ..Default::default()
                },
            );
            d.preset_property(
                0,
                12,
                &expected.manufacturer.unwrap_or_default().to_be_bytes(),
            );
            let hardware = plan.profile().hardware_type();
            d.preset_property(0, 78, &hardware[..hardware.len().min(6)]);
            d
        }
    }
}
fn print_memory_preview(plan: &service::MemoryPrepared, via: &str) {
    let profile = plan.profile();
    println!(
        "Profile {} for device {} via {via}.\nRead only. Nothing will be written: no programming, authorisation/key, Verify Mode, restart, memory or property write.",
        service::MEMORY_PROFILE,
        plan.base().target.address()
    );
    println!(
        "Application {}. Required before any memory read: mask {:04X}h, manufacturer {:04X}h, PID78 matching {} (procedure position {}).",
        profile.program_id(),
        plan.base().expected.mask.unwrap_or_default(),
        plan.base().expected.manufacturer.unwrap_or_default(),
        profile
            .hardware_type()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
        profile.compare_position()
    );
    println!("Ranges (source-declared candidate locations, not proven runtime addresses):");
    for (i, (range, position)) in profile
        .ranges()
        .ranges()
        .iter()
        .zip(profile.range_positions())
        .enumerate()
    {
        println!(
            "  {}. {:04X}h, {} octets (procedure position {position})",
            i + 1,
            range.address,
            range.length
        );
    }
    println!(
        "Total: {} octets in {} A_Memory_Read requests of at most {} octets.\nNot a backup and not a restore basis. The archive contains device configuration: keep it private.\nLocal selection reference: {}",
        profile.ranges().total_octets(),
        profile.ranges().chunks().len(),
        knx_core::commissioning::declared_memory::STANDARD_FRAME_CHUNK_OCTETS,
        plan.base().reference
    );
}
fn synthetic_project() -> Project {
    let mut p = Project::new(Language("en".into()));
    p.devices.insert(DeviceInstance {
        id: DeviceId(1),
        source: SourceRef {
            path: String::new(),
            ets_id: String::new(),
        },
        name: "Synthetic device".into(),
        description: None,
        address: Some(knx_core::IndividualAddress::new(1, 1, 25).unwrap()),
        product_ref: String::new(),
        program_ref: String::new(),
        commissioning: Default::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
    p
}
