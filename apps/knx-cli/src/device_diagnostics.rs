//! The `knx device diagnostics` command.
//!
//! `knx device diagnostics`: preview or export one selected device's generic
//! identity recipe. `--simulate` never constructs a network client.
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
            flag @ ("--project" | "--device-id" | "--product-db" | "--gateway" | "--output") => {
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
    let output = PathBuf::from(
        values
            .get("--output")
            .ok_or("--output <new.zip> is required")?,
    );
    if output.exists() {
        return Err("diagnostics-create-new-failed: output exists; nothing acquired".into());
    }
    let (project, device) = if simulate {
        if values.keys().any(|k| k != "--output") {
            return Err(
                "--simulate cannot be combined with real connection or project arguments".into(),
            );
        }
        (synthetic_project(), DeviceId(1))
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
        Some(crate::open_products_db(Some(path))?)
    } else {
        None
    };
    let plan = service::prepare(&project, device, products.as_ref())?;
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
    println!("Device {} via {}. Read only: no programming, authorisation/key, Verify Mode, memory or property writes.\nScope: descriptor type 0; Device Object PID12 (2 bytes), PID78 (6 bytes), optional services.\nMemory: not requested, no admitted safe profile. Mask/manufacturer agreement does not identify a product.\nLocal selection reference: {}\n{}",
        plan.target.address(),gateway.map(|g|g.to_string()).unwrap_or("SIMULATOR (no network)".into()),plan.reference,
        if confirm{"Confirmed acquisition."}else{"Preview only; add --confirm-read to acquire."});
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
        let observer=|r:&service::Report| {let read=r.items.iter().filter(|i|i.completed_at.is_some()).count();eprintln!("Diagnostic progress: {read}/3 logical reads; raw bytes retained.");};
        let result=if simulate {
            let d=knx_net::commissioning::simulator::SimulatedDevice::with_config(knx_net::commissioning::simulator::SimulatorConfig{mask_version:0x0701,..Default::default()});
            d.preset_property(0,12,&[0x12,0x34]);d.preset_property(0,78,&[1,2,3,4,5,6]);
            Ok(service::acquire(&d,&plan,Default::default(),&crate::version_string(),&cancel,observer).await)
        } else {
            use knx_net::BusConnection;
            let _lease=service::gateway_lease()?;
            let tunnel=tokio::time::timeout(Duration::from_secs(10),knx_net::KnxNetIpClient::new().connect_tunnel(gateway.unwrap())).await.map_err(|_|"gateway timeout")?.map_err(|e|format!("gateway refused/unavailable: {e}"))?;
            let mut report=service::acquire(&tunnel,&plan,Default::default(),&crate::version_string(),&cancel,observer).await;
            if !matches!(tokio::time::timeout(Duration::from_secs(3),tunnel.disconnect()).await,Ok(Ok(()))){report.stop_reason.get_or_insert("tunnel-disconnect-unconfirmed".into());}
            Ok::<_,String>(report)
        };
        signal.abort();let report=result?;
        service::write_new_bundle(&output,&report)?;
        println!("Saved and re-read verified {}{}; this is not a recovery backup.\nReview before sharing with issue@knxbench.com; no upload occurred.",output.display(),if report.simulated{" (SIMULATED)"}else{""});Ok(())
    })
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
