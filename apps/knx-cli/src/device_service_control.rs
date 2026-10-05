//! `knx device service-control`: `PID_SERVICE_CONTROL` bit 2 (ADR-0051).
//!
//! RES §4.2.8 bit 2, *"Individual Address Write Enable"*. A device that keeps
//! it clear ignores an address change by serial number (KNOWN_LIMITATIONS
//! §139). KNXBench never sets it on its own; this command is the explicit
//! operator action.
//!
//! * Without `--enable`/`--disable` it reads the bit, read-only.
//! * With one of them it changes only bit 2, and needs `--confirm` with the
//!   phrase for [`WriteScope::IndividualAddressWriteEnable`]. Without the
//!   phrase it prints what it would do and opens no socket.
//!
//! The web route is further gated by a Settings opt-in; the CLI is not,
//! because it already demands the typed phrase (ADR-0051, "Alternatives").

use std::io::Write;

use knx_core::commissioning::authorisation::AuthorisationPlan;
use knx_core::commissioning::mutation::{
    required_confirmation_phrase, WriteAuthorisation, WriteScope,
};
use knx_core::{ContactableAddress, IndividualAddress};
use knx_net::commissioning::service_control::{
    read_service_control, set_individual_address_write_enable, ServiceControl,
};
use knx_net::{ManagementTransport, SessionTiming};

/// Raw `knx device service-control` arguments.
#[derive(Debug, PartialEq, Eq)]
pub struct ServiceControlArgs {
    pub address: String,
    pub gateway: Option<String>,
    /// `Some(true)` for `--enable`, `Some(false)` for `--disable`.
    pub enable: Option<bool>,
    pub confirm: Option<String>,
    pub key_file: Option<String>,
    pub backup_dir: Option<String>,
    /// Explicit metadata-only journal; plan-only never opens it.
    pub activity_history: Option<String>,
}

pub fn parse_args(args: &[String]) -> Result<ServiceControlArgs, String> {
    let mut address = None;
    let mut gateway = None;
    let mut enable = None;
    let mut confirm = None;
    let mut key_file = None;
    let mut backup_dir = None;
    let mut activity_history = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--enable" | "--disable" => {
                if enable.replace(args[i] == "--enable").is_some() {
                    return Err("give --enable or --disable once".to_string());
                }
                i += 1;
            }
            "--gateway" | "--confirm" | "--key-file" | "--backup-dir" | "--activity-history" => {
                let value = crate::take_value(args, i + 1, &args[i])?;
                let slot = match args[i].as_str() {
                    "--gateway" => &mut gateway,
                    "--confirm" => &mut confirm,
                    "--key-file" => &mut key_file,
                    "--activity-history" => &mut activity_history,
                    _ => &mut backup_dir,
                };
                *slot = Some(value);
                i += 2;
            }
            flag if flag.starts_with("--") => return Err(format!("unknown flag {flag}")),
            positional => {
                if address.replace(positional.to_string()).is_some() {
                    return Err("give exactly one device address".to_string());
                }
                i += 1;
            }
        }
    }
    let address = address.ok_or("missing the device address, e.g. 1.1.67")?;
    if confirm.is_some() && enable.is_none() {
        return Err("--confirm needs --enable or --disable".to_string());
    }
    if enable.is_none() && gateway.is_none() {
        return Err("reading the bit asks the device and needs --gateway <host:port>".to_string());
    }
    if confirm.is_some() && gateway.is_none() {
        return Err("--confirm writes to a device and needs --gateway <host:port>".to_string());
    }
    Ok(ServiceControlArgs {
        address,
        gateway,
        enable,
        confirm,
        key_file,
        backup_dir,
        activity_history,
    })
}

/// What the operator asked for, after the checks that need no socket.
#[derive(Debug)]
pub enum Mode {
    /// Read the bit.
    Read { gateway: std::net::SocketAddrV4 },
    /// `--enable`/`--disable` without a phrase: print, send nothing.
    Plan { enable: bool },
    /// Change the bit.
    Write {
        gateway: std::net::SocketAddrV4,
        enable: bool,
        authorisation: WriteAuthorisation,
    },
}

fn parse_gateway(text: &str) -> Result<std::net::SocketAddrV4, String> {
    text.parse()
        .map_err(|_| "--gateway must be host:port, e.g. 192.0.2.1:3671".to_string())
}

/// Parses the address, refuses an excluded one, and checks the phrase.
pub fn check(args: &ServiceControlArgs) -> Result<(ContactableAddress, Mode), String> {
    let address: IndividualAddress = args
        .address
        .parse()
        .map_err(|e| format!("invalid individual address {}: {e}", args.address))?;
    let target = ContactableAddress::new(address).map_err(|e| e.to_string())?;
    let gateway = || {
        args.gateway
            .as_deref()
            .ok_or_else(|| "this asks the device and needs --gateway <host:port>".to_string())
            .and_then(parse_gateway)
    };
    let mode = match (args.enable, &args.confirm) {
        (None, _) => Mode::Read {
            gateway: gateway()?,
        },
        (Some(enable), None) => Mode::Plan { enable },
        (Some(enable), Some(confirmation)) => {
            let gateway = gateway()?;
            let authorisation = WriteAuthorisation::for_hardware(
                address,
                WriteScope::IndividualAddressWriteEnable,
                confirmation,
            )
            .map_err(|e| format!("not written: {e}"))?;
            Mode::Write {
                gateway,
                enable,
                authorisation,
            }
        }
    };
    Ok((target, mode))
}

fn state(enabled: bool) -> &'static str {
    if enabled {
        "enabled"
    } else {
        "disabled"
    }
}

/// What `--enable`/`--disable` would do, printed before anything is sent.
pub fn format_plan(address: IndividualAddress, enable: bool) -> String {
    format!(
        "== {address}: {} Individual Address Write Enable: plan (nothing sent yet) ==\n\
         RES §4.2.8 PID_SERVICE_CONTROL (object 0, PID 8), bit 2 only:\n  \
         1: read the mask (mask 0021h codes the bit inversely: refused) and the property\n  \
         2: read PID_DEVICE_CONTROL and persist both original properties (mask and target included)\n  \
         3: set Verify Mode after backup, preserving other PID_DEVICE_CONTROL bits\n  \
         4: {} bit 2, write the two octets back with the other bits unchanged\n  \
         5: compare the device's answer with what was written\n\
         written: no (plan only; add --gateway and --confirm {:?} to write)\n",
        if enable { "set" } else { "clear" },
        if enable { "set" } else { "clear" },
        required_confirmation_phrase(address, WriteScope::IndividualAddressWriteEnable),
    )
}

fn describe(value: ServiceControl) -> String {
    format!(
        "PID_SERVICE_CONTROL = {:04X}h (mask {:04X}h): Individual Address Write Enable {}",
        value.raw,
        value.mask.0,
        state(value.individual_address_write_enabled()),
    )
}

/// Reads the bit and prints it. Returns whether the read succeeded.
pub async fn read<T: ManagementTransport>(
    transport: &T,
    address: IndividualAddress,
    plan: AuthorisationPlan,
    timing: SessionTiming,
    out: &mut impl Write,
) -> bool {
    match read_service_control(transport, address, plan, timing).await {
        Ok(value) => {
            let _ = writeln!(out, "{address}: {}", describe(value));
            true
        }
        Err(e) => {
            let _ = writeln!(out, "{address}: FAILED: {e}");
            false
        }
    }
}

/// Recovery bytes and durable intent must both precede every property mutation.
pub struct RecoveryAndHistory<'a> {
    pub backup_dir: &'a std::path::Path,
    pub activity: &'a knx_app::commissioning_activity::WriteGuard,
}

/// Changes the bit and prints the outcome. Returns whether the device holds
/// the requested value afterwards.
pub async fn execute<T: ManagementTransport>(
    transport: &T,
    plan: AuthorisationPlan,
    timing: SessionTiming,
    authorisation: WriteAuthorisation,
    enable: bool,
    recovery: RecoveryAndHistory<'_>,
    out: &mut impl Write,
) -> bool {
    let address = authorisation.target().address();
    let mut backup_path = None;
    let result = set_individual_address_write_enable(
        transport,
        plan,
        timing,
        authorisation,
        enable,
        |before| {
            let path = knx_app::service_control_backup::write_backup(
                recovery.backup_dir,
                address,
                before.before.mask.0,
                before.before.raw,
                before.device_control,
            )
            .map_err(|e| e.to_string())?;
            backup_path = Some(path);
            recovery
                .activity
                .mark_send_possible()
                .map_err(|e| e.to_string())
        },
    )
    .await;
    match result {
        Ok(change) => {
            let _ = writeln!(out, "{address}: before: {}", describe(change.before));
            if let Some(path) = &backup_path {
                let _ = writeln!(
                    out,
                    "{address}: pre-write property backup: {}",
                    path.display()
                );
            }
            if change.written {
                let _ = writeln!(
                    out,
                    "{address}: written: yes, {:04X}h -> {:04X}h, read back; \
                     Individual Address Write Enable {}",
                    change.before.raw,
                    change.after,
                    state(enable),
                );
            } else {
                let _ = writeln!(
                    out,
                    "{address}: written: no need, the bit was already {}",
                    state(enable)
                );
            }
            true
        }
        Err(e) => {
            let _ = writeln!(out, "{address}: FAILED: {e}");
            if let Some(path) = &backup_path {
                let _ = writeln!(
                    out,
                    "{address}: pre-write property backup: {}",
                    path.display()
                );
            }
            let _ = writeln!(
                out,
                "{address}: written: no, or not confirmed; read it again"
            );
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use knx_core::commissioning::properties::{PID_SERVICE_CONTROL, SERVICE_CONTROL_INVERTED_MASK};
    use knx_net::commissioning::simulator::{SimulatedDevice, SimulatorConfig};

    use super::*;

    fn journal(
        address: IndividualAddress,
    ) -> (
        tempfile::TempDir,
        knx_app::commissioning_activity::WriteGuard,
    ) {
        let dir = tempfile::tempdir().unwrap();
        let log = std::sync::Arc::new(knx_app::commissioning_activity::OneShotLog::persistent(
            dir.path().join("history.sqlite"),
            "synthetic-cli".into(),
        ));
        let guard = log
            .start_write("serviceControlWrite", Some(address.to_string()))
            .unwrap();
        (dir, guard)
    }

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
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

    const GW: &str = "192.0.2.1:3671";
    const PHRASE: &str = "I confirm individual-address write enable to 1.1.67";

    #[test]
    fn without_a_switch_it_reads_and_needs_a_gateway() {
        let parsed = parse_args(&args(&["1.1.67", "--gateway", GW])).unwrap();
        assert!(matches!(check(&parsed).unwrap().1, Mode::Read { .. }));
        assert!(parse_args(&args(&["1.1.67"])).is_err());
    }

    #[test]
    fn a_switch_without_the_phrase_is_a_plan_that_names_the_phrase() {
        let parsed = parse_args(&args(&["1.1.67", "--enable"])).unwrap();
        let (target, mode) = check(&parsed).unwrap();
        assert!(matches!(mode, Mode::Plan { enable: true }));
        let plan = format_plan(target.address(), true);
        assert!(plan.contains(PHRASE), "{plan}");
        assert!(plan.contains("nothing sent"), "{plan}");
        assert!(plan.contains("PID_DEVICE_CONTROL"), "{plan}");
        assert!(plan.contains("Verify Mode after backup"), "{plan}");
    }

    #[test]
    fn only_the_scopes_own_phrase_opens_the_write() {
        for wrong in [
            "I confirm individual-address write enable to 1.1.68",
            "I confirm individual-address programming to 1.1.67",
            "I confirm download to 1.1.67",
        ] {
            let parsed = parse_args(&args(&[
                "1.1.67",
                "--enable",
                "--gateway",
                GW,
                "--confirm",
                wrong,
            ]))
            .unwrap();
            assert!(
                check(&parsed).unwrap_err().contains("not written"),
                "{wrong}"
            );
        }
        let parsed = parse_args(&args(&[
            "1.1.67",
            "--disable",
            "--gateway",
            GW,
            "--confirm",
            PHRASE,
        ]))
        .unwrap();
        assert!(matches!(
            check(&parsed).unwrap().1,
            Mode::Write { enable: false, .. }
        ));
        let with_backup = parse_args(&args(&[
            "1.1.67",
            "--enable",
            "--gateway",
            GW,
            "--confirm",
            PHRASE,
            "--backup-dir",
            "safe-place",
        ]))
        .unwrap();
        assert_eq!(with_backup.backup_dir.as_deref(), Some("safe-place"));
    }

    #[test]
    fn malformed_arguments_are_refused() {
        for bad in [
            vec!["1.1.67", "--enable", "--disable"],
            vec!["1.1.67", "--gateway", GW, "--confirm", PHRASE],
            vec!["1.1.67", "--enable", "--confirm", PHRASE],
            vec!["1.1.67", "1.1.68", "--gateway", GW],
            vec!["1.1.67", "--gateway"],
            vec!["1.1.67", "--gateway", GW, "--force"],
        ] {
            assert!(parse_args(&args(&bad)).is_err(), "{bad:?}");
        }
        let parsed = parse_args(&args(&["1.1.300", "--gateway", GW])).unwrap();
        assert!(check(&parsed).is_err());
    }

    #[tokio::test]
    async fn read_then_enable_prints_what_the_device_holds() {
        let dir = tempfile::tempdir().unwrap();
        let device = SimulatedDevice::with_config(SimulatorConfig {
            serial_number_write_enabled: false,
            ..Default::default()
        });
        let address = device.address();
        device.preset_property(
            0,
            knx_core::commissioning::properties::PID_DEVICE_CONTROL,
            &[0x02],
        );
        let mut out = Vec::new();
        assert!(read(&device, address, AuthorisationPlan::Skip, fast(), &mut out).await);
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains("= 0000h"), "{text}");
        assert!(text.contains("disabled"), "{text}");

        let mut out = Vec::new();
        let authorisation =
            WriteAuthorisation::for_simulator(address, WriteScope::IndividualAddressWriteEnable)
                .unwrap();
        let (_history_dir, activity) = journal(device.address());
        assert!(
            execute(
                &device,
                AuthorisationPlan::Skip,
                fast(),
                authorisation,
                true,
                RecoveryAndHistory {
                    backup_dir: dir.path(),
                    activity: &activity
                },
                &mut out
            )
            .await
        );
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains("0000h -> 0004h"), "{text}");
        assert!(text.contains("written: yes"), "{text}");
        assert!(text.contains("pre-write property backup"), "{text}");
        let backups: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        assert_eq!(backups.len(), 1);
        let record: knx_app::service_control_backup::ServiceControlBackup =
            serde_json::from_slice(&std::fs::read(&backups[0]).unwrap()).unwrap();
        assert_eq!(record.format, 2);
        assert_eq!(record.octets, "0000");
        assert_eq!(record.device_control_octets, "02");
    }

    #[tokio::test]
    async fn a_blocked_backup_refuses_before_a_property_write() {
        let dir = tempfile::tempdir().unwrap();
        let blocker = dir.path().join("occupied");
        std::fs::write(&blocker, b"not a directory").unwrap();
        let device = SimulatedDevice::with_config(SimulatorConfig {
            serial_number_write_enabled: false,
            ..Default::default()
        });
        let authorisation = WriteAuthorisation::for_simulator(
            device.address(),
            WriteScope::IndividualAddressWriteEnable,
        )
        .unwrap();
        let mut out = Vec::new();
        let (_history_dir, activity) = journal(device.address());
        assert!(
            !execute(
                &device,
                AuthorisationPlan::Skip,
                fast(),
                authorisation,
                true,
                RecoveryAndHistory {
                    backup_dir: &blocker,
                    activity: &activity
                },
                &mut out,
            )
            .await
        );
        assert!(String::from_utf8(out).unwrap().contains("backup failed"));
        assert!(!device.seen().iter().any(|seen| matches!(
            seen,
            knx_net::commissioning::simulator::Seen::PropertyWrite { .. }
        )));
    }

    #[tokio::test]
    async fn an_already_set_bit_is_reported_and_not_written() {
        let dir = tempfile::tempdir().unwrap();
        let device = SimulatedDevice::with_config(SimulatorConfig::default());
        let authorisation = WriteAuthorisation::for_simulator(
            device.address(),
            WriteScope::IndividualAddressWriteEnable,
        )
        .unwrap();
        let mut out = Vec::new();
        let (_history_dir, activity) = journal(device.address());
        assert!(
            execute(
                &device,
                AuthorisationPlan::Skip,
                fast(),
                authorisation,
                true,
                RecoveryAndHistory {
                    backup_dir: dir.path(),
                    activity: &activity
                },
                &mut out
            )
            .await
        );
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains("written: no need"), "{text}");
        assert!(!text.contains("written: yes"), "{text}");
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
        assert!(!device.seen().iter().any(|seen| matches!(
            seen,
            knx_net::commissioning::simulator::Seen::PropertyWrite { .. }
        )));
    }

    #[tokio::test]
    async fn a_refusal_says_written_no() {
        let dir = tempfile::tempdir().unwrap();
        let device = SimulatedDevice::with_config(SimulatorConfig {
            mask_version: SERVICE_CONTROL_INVERTED_MASK,
            ..Default::default()
        });
        let authorisation = WriteAuthorisation::for_simulator(
            device.address(),
            WriteScope::IndividualAddressWriteEnable,
        )
        .unwrap();
        let mut out = Vec::new();
        let (_history_dir, activity) = journal(device.address());
        assert!(
            !execute(
                &device,
                AuthorisationPlan::Skip,
                fast(),
                authorisation,
                true,
                RecoveryAndHistory {
                    backup_dir: dir.path(),
                    activity: &activity
                },
                &mut out
            )
            .await
        );
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains("0021h"), "{text}");
        assert!(text.contains("written: no"), "{text}");
        assert!(device.seen().iter().all(|seen| !matches!(
            seen,
            knx_net::commissioning::simulator::Seen::PropertyWrite {
                property_id: PID_SERVICE_CONTROL,
                ..
            }
        )));
    }
}
