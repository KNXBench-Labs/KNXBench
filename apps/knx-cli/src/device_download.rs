//! `knx device download`: a download to the device, a plan by default, a write only on the phrase.
//!
//! "Download" is KNXBench → device over the bus (docs/GLOSSARY.md). The
//! default prints the plan and opens no socket. Writing needs `--gateway`
//! and `--confirm` with the exact phrase
//! [`required_confirmation_phrase`] gives for this device and
//! [`WriteScope::Download`] (ADR-0040: the CLI has no dialog). The address
//! is checked against the project exclusion list and the phrase against the
//! address before the project is even opened.
//!
//! Everything here but [`execute`]'s transport is socket-free, so the whole
//! path runs against the simulator in the tests below.

use std::fmt::Write as _;
use std::io::Write;

use knx_app::device_download::PreparedDownload;
use knx_core::commissioning::authorisation::AuthorisationPlan;
use knx_core::commissioning::memory_download::MemoryDownloadStep;
use knx_core::commissioning::mutation::{
    required_confirmation_phrase, WriteAuthorisation, WriteScope,
};
use knx_core::{ContactableAddress, IndividualAddress};
use knx_net::commissioning::memory_download::{
    run_memory_download_observed, MemoryDownloadReport, Progress, RestartOutcome,
};
use knx_net::{ManagementSession, ManagementTransport, SessionTiming};

/// Raw `knx device download` arguments.
#[derive(Debug, PartialEq, Eq)]
pub struct DownloadArgs {
    pub target: String,
    pub project: String,
    pub product_db: Option<String>,
    pub gateway: Option<String>,
    pub confirm: Option<String>,
}

pub fn parse_download_args(args: &[String]) -> Result<DownloadArgs, String> {
    let mut target = None;
    let mut project = None;
    let mut product_db = None;
    let mut gateway = None;
    let mut confirm = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--project" => {
                project = Some(crate::take_value(args, i + 1, "--project")?);
                i += 1;
            }
            "--product-db" => {
                product_db = Some(crate::take_value(args, i + 1, "--product-db")?);
                i += 1;
            }
            "--gateway" => {
                gateway = Some(crate::take_value(args, i + 1, "--gateway")?);
                i += 1;
            }
            // The phrase contains spaces and starts with a letter, so
            // `take_value` would accept it; a phrase starting with `--`
            // is refused like any other missing value.
            "--confirm" => {
                confirm = Some(crate::take_value(args, i + 1, "--confirm")?);
                i += 1;
            }
            flag if flag.starts_with("--") => return Err(format!("unknown flag {flag}")),
            positional => {
                if target.replace(positional.to_string()).is_some() {
                    return Err("give exactly one device address".to_string());
                }
            }
        }
        i += 1;
    }
    let target = target.ok_or("missing the device address, e.g. 1.1.67")?;
    let project = project.ok_or("--project <path.knxdb> is required")?;
    if confirm.is_some() && gateway.is_none() {
        return Err("--confirm writes to a device and needs --gateway <host:port>".to_string());
    }
    Ok(DownloadArgs {
        target,
        project,
        product_db,
        gateway,
        confirm,
    })
}

/// What the operator asked for, after the checks that need no project.
#[derive(Debug, PartialEq, Eq)]
pub enum Mode {
    /// Print the plan, write nothing.
    Plan,
    /// Write, through this gateway.
    Write {
        gateway: std::net::SocketAddrV4,
        confirmation: String,
    },
}

/// Parses the address, refuses an excluded one, and checks the phrase.
/// Runs before the project is opened and before any socket.
pub fn check_target(args: &DownloadArgs) -> Result<(ContactableAddress, Mode), String> {
    let address: IndividualAddress = args
        .target
        .parse()
        .map_err(|e| format!("invalid device address {}: {e}", args.target))?;
    let target = ContactableAddress::new(address).map_err(|e| e.to_string())?;
    let mode = match (&args.gateway, &args.confirm) {
        (Some(gateway), Some(confirmation)) => {
            let gateway = gateway
                .parse()
                .map_err(|_| "--gateway must be host:port, e.g. 192.0.2.1:3671".to_string())?;
            let expected = required_confirmation_phrase(address, WriteScope::Download);
            if *confirmation != expected {
                return Err(format!(
                    "not written: the confirmation for a download to device {address} must read exactly {expected:?}"
                ));
            }
            Mode::Write {
                gateway,
                confirmation: confirmation.clone(),
            }
        }
        _ => Mode::Plan,
    };
    Ok((target, mode))
}

/// The plan, as printed before anything is sent (and as all a dry run
/// prints).
pub fn format_plan(prepared: &PreparedDownload) -> String {
    let target = prepared.target;
    let mut out = String::new();
    let _ = writeln!(
        out,
        "== download to device {target}: plan (nothing sent yet) =="
    );
    let _ = writeln!(
        out,
        "device:  {} (project device {})",
        prepared.device_name, prepared.device.0
    );
    let _ = writeln!(out, "program: {}", prepared.request.program_id);
    let _ = writeln!(
        out,
        "mask {:04X}h, manufacturer {:04X}h",
        prepared.plan.mask.0, prepared.plan.manufacturer
    );
    let _ = writeln!(
        out,
        "configuration: {} parameter values, {} group links",
        prepared.request.values.len(),
        prepared.request.links.len()
    );
    let _ = writeln!(
        out,
        "segments (octets the device keeps itself are not written):"
    );
    for (id, address, size, written) in prepared.octets_to_write() {
        let _ = writeln!(
            out,
            "  {id} at {address:04X}h: {size} octets, {written} written, {} kept",
            size - written
        );
    }
    let _ = writeln!(
        out,
        "octets written to the device: {} (every one is read back)",
        prepared.plan.data_octets()
    );
    let _ = writeln!(out, "steps: {}", prepared.plan.steps.len());
    for (index, step) in prepared.plan.steps.iter().enumerate() {
        let _ = writeln!(out, "  {:2}: {step}", index + 1);
    }
    out
}

/// One [`Progress`] as a line of output.
pub fn format_progress(progress: &Progress) -> Option<String> {
    match progress {
        Progress::Started {
            target,
            steps,
            data_octets,
        } => Some(format!(
            "== download to device {target}: writing, {steps} steps, {data_octets} octets =="
        )),
        Progress::StepStarted { index, of, step } => {
            Some(format!("  [{:2}/{of}] {step}", index + 1))
        }
        Progress::DataWritten {
            address,
            octets,
            written,
            of,
            ..
        } => Some(format!(
            "        -> {address:04X}h {} ({written}/{of} octets, read back OK)",
            octets
                .iter()
                .map(|octet| format!("{octet:02X}"))
                .collect::<Vec<_>>()
                .join(" ")
        )),
        Progress::StepDone(done) => done
            .observed
            .as_ref()
            .map(|observed| format!("        done: {observed}")),
    }
}

/// Whether a step changes the device.
fn writes(step: &MemoryDownloadStep) -> bool {
    matches!(
        step,
        MemoryDownloadStep::LoadRecord(_)
            | MemoryDownloadStep::WriteMemory { .. }
            | MemoryDownloadStep::Restart
    )
}

/// How a run ended, for the summary and the exit code.
#[derive(Debug, PartialEq, Eq)]
pub enum Written {
    /// Every step ran.
    Yes,
    /// It stopped before any step that changes the device.
    No,
    /// It stopped after at least one step that changes the device.
    Partially,
}

/// Runs the plan through `transport` and writes the progress and the
/// summary to `out`. Returns what was written.
pub async fn execute<T: ManagementTransport>(
    transport: &T,
    authorisation: WriteAuthorisation,
    timing: SessionTiming,
    prepared: &PreparedDownload,
    out: &mut impl Write,
) -> Written {
    let target = authorisation.target().address();
    let plan = &prepared.plan;
    let mut session = match ManagementSession::authorised(
        transport,
        AuthorisationPlan::Skip,
        timing,
        authorisation,
    ) {
        Ok(session) => session,
        Err(e) => {
            let _ = writeln!(out, "== download to device {target}: refused: {e} ==");
            let _ = writeln!(out, "written to the device: no");
            return Written::No;
        }
    };
    let mut last_started = None;
    let result = run_memory_download_observed(&mut session, plan, |progress| {
        if let Progress::StepStarted { index, .. } = &progress {
            last_started = Some(*index);
        }
        if let Some(line) = format_progress(&progress) {
            let _ = writeln!(out, "{line}");
        }
    })
    .await;
    let wrote_something = last_started.is_some_and(|last| plan.steps[..=last].iter().any(writes));
    match result {
        Ok(report) => {
            summarise(out, target, &report);
            Written::Yes
        }
        Err(e) => {
            let _ = writeln!(out, "== download to device {target}: FAILED: {e} ==");
            if wrote_something {
                let _ = writeln!(
                    out,
                    "written to the device: partially (stopped in step {}); \
                     the device may not run until a complete download",
                    last_started.map_or(0, |index| index + 1)
                );
                Written::Partially
            } else {
                let _ = writeln!(out, "written to the device: no");
                Written::No
            }
        }
    }
}

fn summarise(out: &mut impl Write, target: IndividualAddress, report: &MemoryDownloadReport) {
    let _ = writeln!(out, "== download to device {target}: finished ==");
    let _ = writeln!(
        out,
        "written to the device: yes, {} octets, every one read back",
        report.data_octets
    );
    for (machine, state) in &report.final_states {
        let _ = writeln!(out, "  {machine}: {state}");
    }
    // Loud on purpose: an unconfirmed restart is a success, and someone
    // skimming for FAILED must still not miss it (goal-commission K2).
    match &report.restart {
        RestartOutcome::Acknowledged { .. } => {
            let _ = writeln!(out, "restart: acknowledged by the device");
        }
        RestartOutcome::NotInPlan => {
            let _ = writeln!(out, "restart: none in the plan");
        }
        RestartOutcome::Unconfirmed { .. } => {
            let _ = writeln!(out, "restart: NOT confirmed. {}", report.restart);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn the_default_is_a_plan() {
        let parsed = parse_download_args(&args(&["1.1.67", "--project", "p.knxdb"])).unwrap();
        assert_eq!(check_target(&parsed).unwrap().1, Mode::Plan);
    }

    #[test]
    fn a_gateway_without_the_phrase_is_still_a_plan() {
        let parsed = parse_download_args(&args(&[
            "1.1.67",
            "--project",
            "p.knxdb",
            "--gateway",
            "192.0.2.1:3671",
        ]))
        .unwrap();
        assert_eq!(check_target(&parsed).unwrap().1, Mode::Plan);
    }

    #[test]
    fn the_phrase_without_a_gateway_is_refused() {
        assert!(parse_download_args(&args(&[
            "1.1.67",
            "--project",
            "p.knxdb",
            "--confirm",
            "I confirm download to 1.1.67",
        ]))
        .is_err());
    }

    #[test]
    fn the_phrase_for_another_device_is_refused() {
        let parsed = parse_download_args(&args(&[
            "1.1.67",
            "--project",
            "p.knxdb",
            "--gateway",
            "192.0.2.1:3671",
            "--confirm",
            "I confirm download to 1.1.66",
        ]))
        .unwrap();
        let refused = check_target(&parsed).unwrap_err();
        assert!(
            refused.contains("\"I confirm download to 1.1.67\""),
            "{refused}"
        );
    }

    #[test]
    fn the_right_phrase_writes() {
        let parsed = parse_download_args(&args(&[
            "1.1.67",
            "--project",
            "p.knxdb",
            "--gateway",
            "192.0.2.1:3671",
            "--confirm",
            "I confirm download to 1.1.67",
        ]))
        .unwrap();
        assert!(matches!(
            check_target(&parsed).unwrap().1,
            Mode::Write { .. }
        ));
    }

    #[test]
    fn an_excluded_address_is_refused_even_for_a_plan() {
        let parsed = parse_download_args(&args(&["1.1.220", "--project", "p.knxdb"])).unwrap();
        assert!(check_target(&parsed).is_err());
    }

    // ---- End to end against the simulator: the saved K3 project, the
    // MDT product file, `execute` exactly as `knx device download` calls it.

    use knx_core::commissioning::load_state::LoadState;
    use knx_core::commissioning::properties::{
        ObjectIndex, PID_HARDWARE_TYPE, PID_MANUFACTURER_ID,
    };
    use knx_net::commissioning::simulator::{SimulatedDevice, SimulatorConfig};
    use std::time::Duration;

    const PACKAGE: &str = "MDT_KP_BE_01_Push_Button_V15a.knxprod";
    /// Saved by `knx-server/tests/project_download_request.rs`
    /// (`KNXBENCH_K3_KEEP_PROJECT`); gitignored like the rest of the corpus.
    const PROJECT: &str = "KNXBench 1.1.67 option C.knxdb";

    fn original_data() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../OriginalData")
    }

    fn prepared() -> (tempfile::TempDir, PreparedDownload) {
        let root = std::env::var_os("KNXBENCH_PRODUCT_CORPUS")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| original_data().join("ProductDatabases"));
        let package = knx_testsupport::find_corpus_file(&root, PACKAGE)
            .unwrap_or_else(|| panic!("{PACKAGE} not under {}", root.display()));
        let project_path = std::env::var_os("KNXBENCH_K3_PROJECT")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| original_data().join("DemoProjects").join(PROJECT));
        assert!(
            project_path.exists(),
            "{} not present; set KNXBENCH_K3_PROJECT, or recreate it with \
             KNXBENCH_K3_KEEP_PROJECT on knx-server's project_download_request test",
            project_path.display()
        );
        let dir = tempfile::tempdir().unwrap();
        // A copy: `open_and_migrate` may migrate the file in place.
        let copy = dir.path().join("project.knxdb");
        std::fs::copy(&project_path, &copy).unwrap();
        let project =
            knx_store::load_project(&knx_store::open_and_migrate(&copy).unwrap()).unwrap();
        let products = knx_productdb::open_and_migrate(&dir.path().join("p.sqlite")).unwrap();
        knx_productdb::install_package(&products, PACKAGE, &std::fs::read(package).unwrap())
            .unwrap();
        let prepared = knx_app::device_download::prepare_device_download(
            &products,
            &project,
            "1.1.67".parse().unwrap(),
        )
        .expect("prepares");
        (dir, prepared)
    }

    /// A simulated mask-`0701h` MDT device, as `memory_download_simulated.rs`
    /// sets one up.
    fn mdt(config: SimulatorConfig) -> SimulatedDevice {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            mask_version: 0x0701,
            ..config
        });
        device.preset_property(0, PID_MANUFACTURER_ID, &[0x00, 0x83]);
        device.preset_property(0, PID_HARDWARE_TYPE, &[0, 0, 0, 0, 0x01, 0x27]);
        device.preset_memory(0x4000, &[0x05]);
        device.preset_memory(0x4001, &[0x11, 0x43]);
        for machine in [1, 2, 3] {
            device.preset_load_state(ObjectIndex::new(machine), LoadState::Loaded);
        }
        device
    }

    fn fast() -> SessionTiming {
        SessionTiming {
            connection_timeout: Duration::from_millis(50),
            response_timeout: Duration::from_millis(50),
            poll_interval: Duration::from_millis(1),
            max_transition: Duration::from_millis(40),
            programming_delay: Duration::from_millis(0),
            restart_basic_t1: Duration::from_millis(1),
            restart_responsive_again: Duration::from_millis(5),
            post_restart_disconnect_wait: Duration::from_millis(5),
            programming_mode_broadcast_timeout: Duration::from_millis(20),
        }
    }

    fn run(device: &SimulatedDevice, prepared: &PreparedDownload) -> (Written, String) {
        let authorisation =
            WriteAuthorisation::for_simulator(device.address(), WriteScope::Download).unwrap();
        let mut out = Vec::new();
        let written = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap()
            .block_on(execute(device, authorisation, fast(), prepared, &mut out));
        (written, String::from_utf8(out).unwrap())
    }

    #[test]
    #[ignore = "requires the gitignored OriginalData/ corpus (product file and the saved K3 project); run with --ignored"]
    fn the_saved_project_lands_in_the_simulator_and_says_so() {
        let (_dir, prepared) = prepared();
        let device = mdt(SimulatorConfig::default());
        let (written, out) = run(&device, &prepared);
        assert_eq!(written, Written::Yes, "{out}");
        let target = device.address();
        assert!(
            out.contains(&format!(
                "== download to device {target}: writing, 25 steps, 1416 octets =="
            )),
            "{out}"
        );
        assert!(out.contains("[25/25] disconnect"), "{out}");
        assert!(
            out.contains("-> 4003h "),
            "every data block is shown: {out}"
        );
        assert!(out.contains("(1416/1416 octets, read back OK)"), "{out}");
        assert!(
            out.contains("written to the device: yes, 1416 octets"),
            "{out}"
        );
        assert!(out.contains("restart: acknowledged by the device"), "{out}");
        for segment in &prepared.image.segments {
            let stored = device.memory(segment.address, segment.octets.len());
            for (index, (found, wanted)) in stored.iter().zip(&segment.octets).enumerate() {
                let masked = segment
                    .mask
                    .as_ref()
                    .is_some_and(|mask| mask.get(index) != Some(&0xFF));
                if !masked {
                    assert_eq!(*found, Some(*wanted), "{} + {index}", segment.id);
                }
            }
        }
        assert_eq!(
            device.memory(0x4001, 2),
            vec![Some(0x11), Some(0x43)],
            "address kept"
        );
    }

    #[test]
    #[ignore = "requires the gitignored OriginalData/ corpus (product file and the saved K3 project); run with --ignored"]
    fn an_unanswered_restart_is_written_but_loudly_unconfirmed() {
        let (_dir, prepared) = prepared();
        let device = mdt(SimulatorConfig {
            restart_unanswered: true,
            ..SimulatorConfig::default()
        });
        let (written, out) = run(&device, &prepared);
        assert_eq!(written, Written::Yes, "{out}");
        assert!(
            out.contains("written to the device: yes, 1416 octets"),
            "{out}"
        );
        assert!(out.contains("restart: NOT confirmed."), "{out}");
        assert!(!out.contains("FAILED"), "{out}");
    }

    #[test]
    #[ignore = "requires the gitignored OriginalData/ corpus (product file and the saved K3 project); run with --ignored"]
    fn another_mask_stops_before_the_first_write_and_says_not_written() {
        let (_dir, prepared) = prepared();
        let device = SimulatedDevice::with_config(SimulatorConfig {
            mask_version: 0x0705,
            ..SimulatorConfig::default()
        });
        let (written, out) = run(&device, &prepared);
        assert_eq!(written, Written::No, "{out}");
        assert!(out.contains("FAILED"), "{out}");
        assert!(out.contains("written to the device: no"), "{out}");
        assert!(!device.memory_was_written());
    }

    #[test]
    #[ignore = "requires the gitignored OriginalData/ corpus (product file and the saved K3 project); run with --ignored"]
    fn a_connection_lost_mid_download_is_reported_as_partial() {
        let (_dir, prepared) = prepared();
        let device = mdt(SimulatorConfig {
            drop_connection_after: Some(12),
            ..SimulatorConfig::default()
        });
        let (written, out) = run(&device, &prepared);
        assert_eq!(written, Written::Partially, "{out}");
        assert!(out.contains("written to the device: partially"), "{out}");
    }

    #[test]
    fn two_addresses_or_an_unknown_flag_are_refused() {
        assert!(parse_download_args(&args(&["1.1.1", "1.1.2", "--project", "p"])).is_err());
        assert!(parse_download_args(&args(&["1.1.1", "--project", "p", "--force"])).is_err());
        assert!(parse_download_args(&args(&["--project", "p"])).is_err());
        assert!(parse_download_args(&args(&["1.1.1"])).is_err());
    }
}
