//! Project-configured experimental preview and explicit read-only target observation.
use knx_app::system2_validation_preparation as app;
use knx_core::DeviceId;
use knx_net::{commissioning::SessionTiming, BusConnection, KnxNetIpClient};
use std::{
    collections::BTreeMap,
    net::SocketAddrV4,
    path::Path,
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
    let mut args = args.iter();
    while let Some(flag) = args.next() {
        if !matches!(
            flag.as_str(),
            "--project"
                | "--device-id"
                | "--product-db"
                | "--profile"
                | "--operation-id"
                | "--source-mode"
                | "--recovery-track"
                | "--gateway"
                | "--observe-confirm"
                | "--output"
        ) {
            return Err("system2-project accepts --project --device-id --product-db --operation-id --source-mode retained-sources|profile [--profile file] [--recovery-track knxbench-best-effort|external-ets]; observation additionally requires --gateway --observe-confirm --output".into());
        }
        let value = args
            .next()
            .filter(|v| !v.starts_with("--"))
            .ok_or("missing option value")?;
        if values.insert(flag.as_str(), value.as_str()).is_some() {
            return Err(format!("duplicate {flag}"));
        }
    }
    let required = |flag| {
        values
            .get(flag)
            .copied()
            .ok_or_else(|| format!("{flag} required"))
    };
    let project_conn = crate::open_native_for_reading(Path::new(required("--project")?))
        .map_err(|e| e.to_string())?;
    let project = knx_store::load_project(&project_conn).map_err(|e| e.to_string())?;
    let device = DeviceId(
        required("--device-id")?
            .parse::<u32>()
            .map_err(|_| "invalid device id")?,
    );
    let products = knx_productdb::open_read_only(Path::new(required("--product-db")?))
        .map_err(|e| e.to_string())?;
    let operation = required("--operation-id")?;
    let recovery = match values
        .get("--recovery-track")
        .copied()
        .unwrap_or("knxbench-best-effort")
    {
        "knxbench-best-effort" => app::RecoveryTrack::KnxBenchBestEffort,
        "external-ets" => app::RecoveryTrack::ExternalEts,
        _ => return Err("recovery track must be knxbench-best-effort or external-ets".into()),
    };
    let source = match required("--source-mode")? {
        "retained-sources" => {
            if values.contains_key("--profile") {
                return Err(
                    "retained-sources selects its explicit App profile; do not supply --profile"
                        .into(),
                );
            }
            app::prepare_retained_project_hardware_source(
                &products.conn,
                &project,
                device,
                operation,
                recovery,
            )
        }
        "profile" => {
            let profile = std::fs::read(required("--profile")?).map_err(|e| e.to_string())?;
            app::prepare_project_hardware_source(
                &products.conn,
                &project,
                device,
                &profile,
                operation,
                recovery,
            )
        }
        _ => {
            return Err(
                "explicit --source-mode retained-sources|profile required; no fallback".into(),
            )
        }
    }
    .map_err(|e| e.to_string())?;
    let preview = source.preview_summary().map_err(|e| e.to_string())?;
    println!(
        "{}",
        serde_json::to_string_pretty(&preview).map_err(|e| e.to_string())?
    );
    let observe_phrase = format!(
        "I confirm read-only System-2 observation to {}",
        preview.address
    );
    if !values.contains_key("--observe-confirm") {
        if values
            .keys()
            .any(|k| matches!(*k, "--gateway" | "--output"))
        {
            return Err(
                "--gateway/--output requires explicit --observe-confirm; nothing connected".into(),
            );
        }
        println!("OFFLINE PREVIEW ONLY: nothing connected or written. To retain a physical preparation, repeat with --gateway <ip:port> --observe-confirm '{observe_phrase}' --output <new-private-bundle.json>. Then use system2-validation with this project/device/product DB and the displayed exact confirmations.");
        return Ok(());
    }
    if required("--observe-confirm")? != observe_phrase {
        return Err("exact read-only target observation confirmation required".into());
    }
    let gateway: SocketAddrV4 = required("--gateway")?
        .parse()
        .map_err(|_| "invalid gateway")?;
    if gateway.port() == 0
        || gateway.ip().is_unspecified()
        || gateway.ip().is_multicast()
        || gateway.ip().is_broadcast()
    {
        return Err("unicast gateway with nonzero port required".into());
    }
    let output = Path::new(required("--output")?);
    if output.exists() {
        return Err("output exists; no overwrite or connection".into());
    }
    source
        .check_current(&products.conn, &project, device)
        .map_err(|e| e.to_string())?;
    let _lease = knx_app::device_diagnostics::gateway_lease().map_err(|e| e.to_string())?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
    let result = runtime.block_on(async {
        let tunnel = tokio::time::timeout(
            Duration::from_secs(10),
            KnxNetIpClient::new().connect_tunnel(gateway),
        )
        .await
        .map_err(|_| "gateway timeout".to_string())?
        .map_err(|e| e.to_string())?;
        let cancel = Arc::new(AtomicBool::new(false));
        let flag = cancel.clone();
        let signal = tokio::spawn(async move {
            if tokio::signal::ctrl_c().await.is_ok() {
                flag.store(true, Ordering::SeqCst);
            }
        });
        let observed =
            app::observe_project_hardware(&tunnel, source, SessionTiming::default(), &cancel)
                .await
                .map_err(|e| e.to_string());
        signal.abort();
        let _ = tokio::time::timeout(Duration::from_secs(3), tunnel.disconnect()).await;
        observed
    })?;
    knx_app::property_validation::write_prepared_bundle(output, &result)
        .map_err(|e| e.to_string())?;
    println!("Observed target-bound preparation retained locally: {}. NO DEVICE WRITE. Actual affected-state backup is acquired freshly by execution, not by this identity observation. Preview/execute with system2-validation --bundle {} --project <project> --device-id <id> --product-db <db>. Keep the bundle private.", output.display(), output.display());
    Ok(())
}
