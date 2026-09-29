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

use knx_app::access_key::DownloadKeying;
use knx_app::device_backup::{write_backup, StoredBackup};
use knx_app::device_download::PreparedDownload;
use knx_app::download_support::{untested_acknowledgement, SupportLevel};
use knx_core::commissioning::authorisation::Authorisation;
use knx_core::commissioning::device_backup::DeviceBackup;
use knx_core::commissioning::memory_download::{MemoryDownloadPlan, MemoryDownloadStep};
use knx_core::commissioning::mutation::{
    required_confirmation_phrase, WriteAuthorisation, WriteScope,
};
use knx_core::commissioning::partial_memory_download::PartialDownloadParts;
use knx_core::{ContactableAddress, IndividualAddress};
use knx_net::commissioning::memory_download::{
    locked_device_hint, run_memory_download_with_backup, MemoryDownloadReport, Progress,
    RestartOutcome,
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
    /// A file holding the access key. Never the key itself on the command
    /// line: argv is visible to every user through `ps`.
    pub key_file: Option<String>,
    /// `--partial`: CP §3.9.2.4's partial download instead of the complete
    /// one.
    pub partial: Option<PartialDownloadParts>,
    /// `--backup-dir`: where the backup taken before the first write goes.
    /// Default: `<project>.backups/` beside the project.
    pub backup_dir: Option<String>,
    /// `--accept-untested`: the phrase that accepts a download to an
    /// application no one has verified on hardware.
    pub accept_untested: Option<String>,
}

/// `--partial parameters|group-addresses|both`.
fn parse_partial(text: &str) -> Result<PartialDownloadParts, String> {
    let (parameters, group_addresses) = match text {
        "parameters" => (true, false),
        "group-addresses" => (false, true),
        "both" => (true, true),
        other => {
            return Err(format!(
                "--partial must be parameters, group-addresses or both, not {other:?}"
            ))
        }
    };
    Ok(PartialDownloadParts {
        parameters,
        group_addresses,
    })
}

pub fn parse_download_args(args: &[String]) -> Result<DownloadArgs, String> {
    let mut target = None;
    let mut project = None;
    let mut product_db = None;
    let mut gateway = None;
    let mut confirm = None;
    let mut key_file = None;
    let mut partial = None;
    let mut backup_dir = None;
    let mut accept_untested = None;
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
            "--key-file" => {
                key_file = Some(crate::take_value(args, i + 1, "--key-file")?);
                i += 1;
            }
            "--partial" => {
                partial = Some(parse_partial(&crate::take_value(
                    args,
                    i + 1,
                    "--partial",
                )?)?);
                i += 1;
            }
            "--backup-dir" => {
                backup_dir = Some(crate::take_value(args, i + 1, "--backup-dir")?);
                i += 1;
            }
            "--accept-untested" => {
                accept_untested = Some(crate::take_value(args, i + 1, "--accept-untested")?);
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
        key_file,
        partial,
        backup_dir,
        accept_untested,
    })
}

/// Where a write's backup goes: `--backup-dir`, or `<project>.backups/`
/// beside the project file.
pub fn backup_dir(args: &DownloadArgs) -> std::path::PathBuf {
    match &args.backup_dir {
        Some(dir) => dir.into(),
        None => format!("{}.backups", args.project).into(),
    }
}

/// The support level as printed under the plan.
pub fn format_support(level: &SupportLevel, target: IndividualAddress) -> String {
    match level {
        SupportLevel::Verified { evidence, .. } => format!(
            "support: verified — this application was downloaded this way on hardware \
             ({}, {}; {})\n",
            evidence.device, evidence.date, evidence.reference
        ),
        SupportLevel::Untested { .. } => format!(
            "support: UNTESTED — the plan is complete and built from the product data alone, \
             but no download of this application this way has been verified on hardware.\n\
             \x20        A write needs --accept-untested {:?} as well.\n",
            untested_acknowledgement(target)
        ),
        SupportLevel::Unsupported { category, detail } => {
            format!("support: unsupported ({}): {detail}\n", category.code())
        }
    }
}

/// Refuses a write of an untested download without its acknowledgement.
/// Runs before any socket.
pub fn check_acknowledgement(
    level: &SupportLevel,
    target: IndividualAddress,
    given: Option<&str>,
) -> Result<(), String> {
    if !level.needs_acknowledgement() {
        return Ok(());
    }
    let expected = untested_acknowledgement(target);
    match given {
        Some(given) if given == expected => Ok(()),
        Some(_) => Err(format!(
            "not written: --accept-untested must read exactly {expected:?}"
        )),
        None => Err(format!(
            "not written: this download is untested on hardware; add --accept-untested {expected:?} \
             to accept that"
        )),
    }
}

/// What a backup is kept as, besides its octets.
pub struct BackupTarget<'a> {
    /// The directory.
    pub dir: &'a std::path::Path,
    /// The application program.
    pub application: &'a str,
    /// The partial selection, if any.
    pub partial: Option<PartialDownloadParts>,
    /// The time stamp (RFC 3339) the file is named after.
    pub taken: String,
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
pub fn format_plan(prepared: &PreparedDownload, keying: &DownloadKeying) -> String {
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
        "{} (octets the device keeps itself are not written):",
        if prepared.partial.is_some() {
            "segments of the complete image; the partial download writes only those its steps name"
        } else {
            "segments"
        }
    );
    for (id, address, size, written) in prepared.octets_to_write() {
        let _ = writeln!(
            out,
            "  {id} at {address:04X}h: {size} octets, {written} written, {} kept",
            size - written
        );
    }
    if let Some((parts, ignored)) = &prepared.partial {
        let _ = writeln!(
            out,
            "partial download (CP §3.9.2.4): {}; the device must already carry this \
             application with every part loaded",
            match (parts.parameters, parts.group_addresses) {
                (true, true) => "parameters and group addresses",
                (true, false) => "parameters only",
                _ => "group addresses only",
            }
        );
        for (address, octets) in ignored {
            let _ = writeln!(
                out,
                "  not written: {octets} octets at {address:04X}h (application data outside \
                 EEPROM, which CP §3.9.2.4 ignores in a partial download)"
            );
        }
    }
    let _ = writeln!(
        out,
        "octets written to the device: {} (every one is read back)",
        prepared.plan.data_octets()
    );
    let _ = writeln!(
        out,
        "access key: {}{}",
        keying.source,
        if keying.two_key {
            ", MP §3.5.2 (free key first, the key only if it is better)"
        } else {
            ""
        }
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
        Progress::BackupTaken { regions, octets } => Some(format!(
            "        backup: {octets} octets in {regions} regions read before the first write"
        )),
        Progress::Authorised {
            authorisation,
            suspicious,
        } => Some(match authorisation {
            Authorisation::FreeLevelUnknown => {
                "        access: no key sent, the device's free level".to_owned()
            }
            Authorisation::Granted { level } if *suspicious => {
                format!("        access: {level} — the minimum a wrong key earns (AL §3.5.7)")
            }
            Authorisation::Granted { level } => format!("        access: {level}"),
        }),
    }
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
    keying: DownloadKeying,
    timing: SessionTiming,
    plan: &MemoryDownloadPlan,
    backup: &BackupTarget<'_>,
    out: &mut impl Write,
) -> Written {
    let target = authorisation.target().address();
    let mut session =
        match ManagementSession::authorised(transport, keying.plan, timing, authorisation) {
            Ok(session) => {
                let session = session.with_level_count(keying.level_count);
                if keying.two_key {
                    session.with_two_key_extension()
                } else {
                    session
                }
            }
            Err(e) => {
                let _ = writeln!(out, "== download to device {target}: refused: {e} ==");
                let _ = writeln!(out, "written to the device: no");
                return Written::No;
            }
        };
    let mut last_started = None;
    let mut granted = None;
    // Shared by the backup closure (which sets it) and the observer (which
    // prints it when the backup's progress arrives, right after).
    let kept_at = std::cell::RefCell::new(None);
    let mut keep = |taken: &DeviceBackup| {
        let stored = StoredBackup {
            backup: taken.clone(),
            application: backup.application.to_owned(),
            partial: backup.partial,
            taken: backup.taken.clone(),
            plan_steps: plan.steps.iter().map(ToString::to_string).collect(),
        };
        let path = write_backup(backup.dir, &stored).map_err(|e| e.to_string())?;
        *kept_at.borrow_mut() = Some(path);
        Ok(())
    };
    let result = run_memory_download_with_backup(
        &mut session,
        plan,
        |progress| {
            if let Progress::StepStarted { index, .. } = &progress {
                last_started = Some(*index);
            }
            if let Progress::Authorised {
                authorisation,
                suspicious,
            } = &progress
            {
                granted = Some((*authorisation, *suspicious));
            }
            if let Some(line) = format_progress(&progress) {
                let _ = writeln!(out, "{line}");
            }
            if matches!(progress, Progress::BackupTaken { .. }) {
                if let Some(path) = kept_at.borrow().as_ref() {
                    let _ = writeln!(out, "        backup kept in {}", path.display());
                }
            }
        },
        &mut keep,
    )
    .await;
    let wrote_something = last_started.is_some_and(|last| {
        plan.steps[..=last]
            .iter()
            .any(MemoryDownloadStep::changes_device)
    });
    match result {
        Ok(report) => {
            summarise(out, target, &report);
            Written::Yes
        }
        Err(e) => {
            let _ = writeln!(out, "== download to device {target}: FAILED: {e} ==");
            if let Some(hint) = locked_device_hint(
                &e,
                granted.map(|(authorisation, _)| authorisation),
                granted.is_some_and(|(_, suspicious)| suspicious),
            ) {
                let _ = writeln!(out, "hint: {hint}");
            }
            if wrote_something {
                let _ = writeln!(
                    out,
                    "written to the device: partially (stopped in step {}); \
                     the device may not run until a complete download",
                    last_started.map_or(0, |index| index + 1)
                );
                if let Some(path) = kept_at.borrow().as_ref() {
                    let _ = writeln!(
                        out,
                        "what it held before is in {}; `knx device restore` can plan an attempted recovery",
                        path.display()
                    );
                }
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
    use knx_app::access_key::{download_keying, KeySource};
    use knx_core::commissioning::authorisation::AccessKey;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn the_default_is_a_plan() {
        let parsed = parse_download_args(&args(&["1.1.67", "--project", "p.knxdb"])).unwrap();
        assert_eq!(check_target(&parsed).unwrap().1, Mode::Plan);
        assert_eq!(parsed.key_file, None);
    }

    /// The key travels in a file, never as an argument `ps` would show.
    #[test]
    fn a_key_file_is_a_path_and_there_is_no_key_flag() {
        let parsed = parse_download_args(&args(&[
            "1.1.67",
            "--project",
            "p.knxdb",
            "--key-file",
            "/run/user/1000/knx.key",
        ]))
        .unwrap();
        assert_eq!(parsed.key_file.as_deref(), Some("/run/user/1000/knx.key"));
        for flag in ["--key", "--bcu-key", "--access-key"] {
            assert!(
                parse_download_args(&args(&["1.1.67", "--project", "p", flag, "1"])).is_err(),
                "{flag}"
            );
        }
        assert!(parse_download_args(&args(&["1.1.67", "--project", "p", "--key-file"])).is_err());
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
        ObjectIndex, PID_HARDWARE_TYPE, PID_MANUFACTURER_ID, PID_PROGRAM_VERSION,
    };
    use knx_net::commissioning::simulator::Seen;
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
        run_keyed(device, prepared, None)
    }

    fn run_keyed(
        device: &SimulatedDevice,
        prepared: &PreparedDownload,
        key: Option<AccessKey>,
    ) -> (Written, String) {
        let dir = tempfile::tempdir().unwrap();
        run_into(device, prepared, key, dir.path())
    }

    fn run_into(
        device: &SimulatedDevice,
        prepared: &PreparedDownload,
        key: Option<AccessKey>,
        backups: &std::path::Path,
    ) -> (Written, String) {
        let authorisation =
            WriteAuthorisation::for_simulator(device.address(), WriteScope::Download).unwrap();
        let source = if key.is_some() {
            KeySource::Project
        } else {
            KeySource::None
        };
        let keying = download_keying(prepared.plan.mask, key, source);
        let mut out = Vec::new();
        let written = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap()
            .block_on(execute(
                device,
                authorisation,
                keying,
                fast(),
                &prepared.plan,
                &BackupTarget {
                    dir: backups,
                    application: &prepared.request.program_id,
                    partial: prepared.partial.as_ref().map(|(parts, _)| *parts),
                    taken: "2026-09-29T15:00:00+02:00".into(),
                },
                &mut out,
            ));
        (written, String::from_utf8(out).unwrap())
    }

    /// The backup is on disk before the first write, and writing it back
    /// through `knx device restore`'s plan returns the device's old memory.
    #[test]
    #[ignore = "requires the gitignored OriginalData/ corpus (product file and the saved K3 project); run with --ignored"]
    fn the_backup_is_kept_before_the_first_write_and_restores_the_old_memory() {
        let (dir, prepared) = prepared();
        let device = mdt(SimulatorConfig::default());
        // An older configuration: other octets than the plan writes.
        for segment in &prepared.image.segments {
            let old: Vec<u8> = (0..segment.octets.len())
                .map(|i| (i % 251) as u8 ^ 0x5A)
                .collect();
            device.preset_memory(segment.address, &old);
        }
        device.preset_memory(0x4001, &[0x11, 0x43]);
        let regions: Vec<_> = prepared
            .image
            .segments
            .iter()
            .map(|s| (s.address, device.memory(s.address, s.octets.len())))
            .collect();
        let backups = dir.path().join("backups");
        let (written, out) = run_into(&device, &prepared, None, &backups);
        assert_eq!(written, Written::Yes, "{out}");
        let backup_line = out.find("backup: ").expect("the backup is reported");
        assert!(backup_line < out.find("[ 5/25]").unwrap(), "{out}");
        assert!(out.contains("backup kept in "), "{out}");
        let files: Vec<_> = std::fs::read_dir(&backups).unwrap().collect();
        assert_eq!(files.len(), 1);
        let path = files.into_iter().next().unwrap().unwrap().path();
        let stored = knx_app::device_backup::read_backup(&path).unwrap();
        assert_eq!(stored.application, prepared.request.program_id);
        assert_ne!(
            regions,
            prepared
                .image
                .segments
                .iter()
                .map(|s| (s.address, device.memory(s.address, s.octets.len())))
                .collect::<Vec<_>>(),
            "the download changed the device"
        );

        let products = knx_productdb::open_and_migrate(&dir.path().join("p.sqlite")).unwrap();
        let mut altered = stored.clone();
        altered.plan_steps[0] = "different load procedure".into();
        assert!(matches!(
            knx_app::device_backup::prepare_restore(&products, &altered),
            Err(knx_app::device_backup::RestorePrepareError::DifferentProcedure)
        ));
        let restore = knx_app::device_backup::prepare_restore(&products, &stored).unwrap();
        let authorisation =
            WriteAuthorisation::for_simulator(device.address(), WriteScope::Download).unwrap();
        let mut out = Vec::new();
        let written = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap()
            .block_on(execute(
                &device,
                authorisation,
                download_keying(restore.mask, None, KeySource::None),
                fast(),
                &restore,
                &BackupTarget {
                    dir: &backups,
                    application: &stored.application,
                    partial: None,
                    taken: "2026-09-29T15:05:00+02:00".into(),
                },
                &mut out,
            ));
        let out = String::from_utf8(out).unwrap();
        assert_eq!(written, Written::Yes, "{out}");
        for (address, before) in &regions {
            let now = device.memory(*address, before.len());
            for (index, (now, before)) in now.iter().zip(before).enumerate() {
                let written_here = stored.backup.regions.iter().any(|r| {
                    (r.address as usize..r.address as usize + r.octets.len())
                        .contains(&(*address as usize + index))
                });
                if written_here {
                    assert_eq!(now, before, "{address:04X}h + {index}");
                }
            }
        }
    }

    #[test]
    fn an_untested_download_needs_its_exact_acknowledgement() {
        let target: IndividualAddress = "1.1.70".parse().unwrap();
        let untested = SupportLevel::Untested {
            steps: 25,
            octets: 1416,
        };
        let error = check_acknowledgement(&untested, target, None).unwrap_err();
        assert!(
            error.contains("--accept-untested \"I accept an untested download to 1.1.70\""),
            "{error}"
        );
        assert!(check_acknowledgement(&untested, target, Some("yes")).is_err());
        assert!(check_acknowledgement(
            &untested,
            "1.1.71".parse().unwrap(),
            Some("I accept an untested download to 1.1.70")
        )
        .is_err());
        assert!(check_acknowledgement(
            &untested,
            target,
            Some("I accept an untested download to 1.1.70")
        )
        .is_ok());
        let verified = SupportLevel::Verified {
            evidence: knx_app::download_support::VerifiedEvidence {
                program_id: "P".into(),
                scopes: vec!["complete".into()],
                device: "1.1.67".into(),
                date: "2026-09-29".into(),
                reference: "docs".into(),
            },
            steps: 25,
            octets: 1416,
        };
        assert!(check_acknowledgement(&verified, target, None).is_ok());
        assert!(format_support(&untested, target).contains("UNTESTED"));
        assert!(format_support(&verified, target).starts_with("support: verified"));
    }

    #[test]
    fn the_backup_goes_beside_the_project_unless_told_otherwise() {
        let parsed = parse_download_args(&args(&["1.1.67", "--project", "/p/x.knxdb"])).unwrap();
        assert_eq!(
            backup_dir(&parsed),
            std::path::PathBuf::from("/p/x.knxdb.backups")
        );
        let parsed = parse_download_args(&args(&[
            "1.1.67",
            "--project",
            "/p/x.knxdb",
            "--backup-dir",
            "/b",
            "--accept-untested",
            "I accept an untested download to 1.1.67",
        ]))
        .unwrap();
        assert_eq!(backup_dir(&parsed), std::path::PathBuf::from("/b"));
        assert_eq!(
            parsed.accept_untested.as_deref(),
            Some("I accept an untested download to 1.1.67")
        );
    }

    /// A device locked above its free level: without the key the run
    /// stops with a hint and no guess; with the project's key it lands.
    #[test]
    #[ignore = "requires the gitignored OriginalData/ corpus (product file and the saved K3 project); run with --ignored"]
    fn a_locked_device_needs_its_key_and_the_output_never_shows_it() {
        let (_dir, prepared) = prepared();
        let locked = || {
            mdt(SimulatorConfig {
                free_access_level: 3,
                key: Some(0x0BAD_CAFE),
                key_level: 1,
                write_requires_level: 2,
                ..SimulatorConfig::default()
            })
        };
        let device = locked();
        let (written, out) = run(&device, &prepared);
        assert_ne!(written, Written::Yes, "{out}");
        assert!(out.contains("access: no key sent"), "{out}");
        assert!(out.contains("hint: no access key was sent"), "{out}");
        assert_eq!(device.authorize_requests(), 0, "nothing was guessed");

        let device = locked();
        let key = AccessKey::new(0x0BAD_CAFE).unwrap();
        let (written, out) = run_keyed(&device, &prepared, Some(key));
        assert_eq!(written, Written::Yes, "{out}");
        assert!(out.contains("access: level 1"), "{out}");
        assert_eq!(device.authorize_requests(), 2);
        for shown in ["0BADCAFE", "0badcafe", "195939070"] {
            assert!(!out.contains(shown), "the key leaked: {out}");
        }
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
        // Frame 12 now falls in the backup's reads: stopped there, with
        // nothing written. Frame 400 falls in the segment writes.
        let early = mdt(SimulatorConfig {
            drop_connection_after: Some(12),
            ..SimulatorConfig::default()
        });
        let (written, out) = run(&early, &prepared);
        assert_eq!(written, Written::No, "{out}");
        assert!(out.contains("nothing was written"), "{out}");
        assert!(!early.memory_was_written(), "{out}");

        let device = mdt(SimulatorConfig {
            drop_connection_after: Some(400),
            ..SimulatorConfig::default()
        });
        let (written, out) = run(&device, &prepared);
        assert_eq!(written, Written::Partially, "{out}");
        assert!(out.contains("written to the device: partially"), "{out}");
        assert!(
            out.contains("`knx device restore` writes it back"),
            "the way back is named: {out}"
        );
    }

    #[test]
    fn partial_takes_three_spellings_and_refuses_others() {
        for (text, parameters, group_addresses) in [
            ("parameters", true, false),
            ("group-addresses", false, true),
            ("both", true, true),
        ] {
            let parsed = parse_download_args(&args(&[
                "1.1.67",
                "--project",
                "p.knxdb",
                "--partial",
                text,
            ]))
            .unwrap();
            assert_eq!(
                parsed.partial,
                Some(PartialDownloadParts {
                    parameters,
                    group_addresses
                })
            );
        }
        assert_eq!(
            parse_download_args(&args(&["1.1.67", "--project", "p.knxdb"]))
                .unwrap()
                .partial,
            None
        );
        let error = parse_download_args(&args(&[
            "1.1.67",
            "--project",
            "p.knxdb",
            "--partial",
            "everything",
        ]))
        .unwrap_err();
        assert!(
            error.contains("parameters, group-addresses or both"),
            "{error}"
        );
    }

    /// The load records a run sent, octet 0 each: machine and event.
    fn load_events(device: &SimulatedDevice) -> Vec<u8> {
        device
            .seen()
            .into_iter()
            .filter_map(|seen| match seen {
                Seen::MemoryWrite { address, data, .. }
                    if address == 0x0104 && data.len() == 11 =>
                {
                    Some(data[0])
                }
                _ => None,
            })
            .collect()
    }

    /// K15 against the real MDT plan: the partial download of the saved
    /// project unloads nothing of the application, never allocates its
    /// segment again, and lands the same parameter octets.
    #[test]
    #[ignore = "requires the gitignored OriginalData/ corpus (product file and the saved K3 project); run with --ignored"]
    fn a_parameters_partial_download_lands_without_unloading_the_application() {
        let (_dir, prepared) = prepared();
        let complete = prepared.plan.clone();
        let prepared = prepared
            .into_partial(PartialDownloadParts {
                parameters: true,
                group_addresses: false,
            })
            .expect("a partial plan");
        let plan = format_plan(
            &prepared,
            &download_keying(prepared.plan.mask, None, KeySource::None),
        );
        assert!(
            plan.contains("partial download (CP §3.9.2.4): parameters only"),
            "{plan}"
        );
        assert!(
            plan.contains("compare property 3/13 with 00 83 00 27 15"),
            "the application check is in the printed plan: {plan}"
        );
        for step in prepared
            .plan
            .steps
            .iter()
            .filter(|step| step.changes_device())
        {
            assert!(complete.steps.contains(step), "{step}");
        }

        let device = mdt(SimulatorConfig::default());
        device.preset_property(3, PID_PROGRAM_VERSION, &[0x00, 0x83, 0x00, 0x27, 0x15]);
        let (written, out) = run(&device, &prepared);
        assert_eq!(written, Written::Yes, "{out}");
        // Application Program only (type 3): Start Loading, Load Completed.
        assert_eq!(load_events(&device), vec![0x31, 0x32], "{out}");
        let application = prepared
            .image
            .segments
            .iter()
            .find(|segment| segment.address == 0x4400)
            .expect("the application segment");
        let stored = device.memory(application.address, application.octets.len());
        for (index, (found, wanted)) in stored.iter().zip(&application.octets).enumerate() {
            if application
                .mask
                .as_ref()
                .is_none_or(|mask| mask.get(index) == Some(&0xFF))
            {
                assert_eq!(*found, Some(*wanted), "4400h + {index}");
            }
        }
        assert!(
            device.memory(0x4001, 2) == vec![Some(0x11), Some(0x43)],
            "the address table is untouched"
        );
    }

    #[test]
    #[ignore = "requires the gitignored OriginalData/ corpus (product file and the saved K3 project); run with --ignored"]
    fn a_partial_download_to_an_unloaded_device_writes_nothing() {
        let (_dir, prepared) = prepared();
        let prepared = prepared
            .into_partial(PartialDownloadParts {
                parameters: true,
                group_addresses: true,
            })
            .expect("a partial plan");
        let device = mdt(SimulatorConfig::default());
        device.preset_property(3, PID_PROGRAM_VERSION, &[0x00, 0x83, 0x00, 0x27, 0x15]);
        device.preset_load_state(ObjectIndex::new(3), LoadState::Unloaded);
        let (written, out) = run(&device, &prepared);
        assert_eq!(written, Written::No, "{out}");
        assert!(out.contains("run the complete download"), "{out}");
        assert!(load_events(&device).is_empty(), "{out}");
    }

    #[test]
    fn two_addresses_or_an_unknown_flag_are_refused() {
        assert!(parse_download_args(&args(&["1.1.1", "1.1.2", "--project", "p"])).is_err());
        assert!(parse_download_args(&args(&["1.1.1", "--project", "p", "--force"])).is_err());
        assert!(parse_download_args(&args(&["--project", "p"])).is_err());
        assert!(parse_download_args(&args(&["1.1.1"])).is_err());
    }
}
