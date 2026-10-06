//! `knx device address-by-serial` / `find-serial`: MP §2.5 and §2.4, no programming button.
//!
//! The serial number comes from the operator (`--serial MMMM:NNNNNNNN`) or
//! from the project (`--project <p.knxdb> --device <DeviceInstance Id>`,
//! Project Schema `DeviceInstance/@SerialNumber`). Never both, never a
//! guess: a project that records none for the device is an error that asks
//! for `--serial`.
//!
//! Without `--confirm` it prints what would happen and opens no socket. The
//! With `--confirm` the CLI still validates the phrase, then fails closed
//! before a socket opens: there is no verified durable pre-write backup for
//! this procedure yet. The simulator procedure remains available for tests.

use std::fmt::Write as _;
use std::io::Write;
use std::path::Path;

use knx_core::commissioning::mutation::{WriteAuthorisation, WriteScope};
use knx_core::commissioning::serial_number::SerialNumber;
use knx_core::{ContactableAddress, IndividualAddress};
use knx_net::commissioning::individual_address_write::Occupancy;
use knx_net::commissioning::serial_number_write::{
    serial_number_write, SerialNumberWriteError, SerialNumberWriteReport,
};
use knx_net::{ManagementTransport, SessionTiming};

/// Where the serial number comes from, as given.
#[derive(Debug, PartialEq, Eq)]
pub enum SerialSource {
    /// `--serial`.
    Given(String),
    /// `--project` and `--device`.
    Project { project: String, device: String },
}

/// Raw `knx device address-by-serial` arguments.
#[derive(Debug, PartialEq, Eq)]
pub struct AddressBySerialArgs {
    pub new_address: String,
    pub source: SerialSource,
    pub gateway: Option<String>,
    pub confirm: Option<String>,
}

pub fn parse_address_by_serial_args(args: &[String]) -> Result<AddressBySerialArgs, String> {
    let mut new_address = None;
    let mut serial = None;
    let mut project = None;
    let mut device = None;
    let mut gateway = None;
    let mut confirm = None;
    let mut i = 0;
    while i < args.len() {
        let slot = match args[i].as_str() {
            "--serial" => &mut serial,
            "--project" => &mut project,
            "--device" => &mut device,
            "--gateway" => &mut gateway,
            "--confirm" => &mut confirm,
            flag if flag.starts_with("--") => return Err(format!("unknown flag {flag}")),
            positional => {
                if new_address.replace(positional.to_string()).is_some() {
                    return Err("give exactly one new address".to_string());
                }
                i += 1;
                continue;
            }
        };
        *slot = Some(crate::take_value(args, i + 1, &args[i])?);
        i += 2;
    }
    let new_address = new_address.ok_or("missing the new address, e.g. 1.1.30")?;
    let source = match (serial, project, device) {
        (Some(serial), None, None) => SerialSource::Given(serial),
        (None, Some(project), Some(device)) => SerialSource::Project { project, device },
        (None, None, None) => {
            return Err(
                "name the device: --serial MMMM:NNNNNNNN, or --project <p.knxdb> --device <id>"
                    .to_string(),
            )
        }
        (Some(_), _, _) => {
            return Err("--serial and --project/--device name the device twice".to_string())
        }
        _ => return Err("--project and --device go together".to_string()),
    };
    if confirm.is_some() && gateway.is_none() {
        return Err("--confirm writes to a device and needs --gateway <host:port>".to_string());
    }
    Ok(AddressBySerialArgs {
        new_address,
        source,
        gateway,
        confirm,
    })
}

/// The serial number, from wherever `source` says.
pub fn resolve_serial(source: &SerialSource) -> Result<SerialNumber, String> {
    match source {
        SerialSource::Given(text) => text.parse().map_err(|e| format!("--serial {text:?}: {e}")),
        SerialSource::Project { project, device } => {
            // `open_and_migrate` creates a missing file; a typo must not
            // become an empty project.
            if !Path::new(project).exists() {
                return Err(format!("project not found: {project}"));
            }
            let conn = knx_store::open_existing_and_migrate(Path::new(project))
                .map_err(|e| format!("could not read project {project}: {e}"))?;
            let opaque = knx_store::load_opaque(&conn)
                .map_err(|e| format!("could not read project {project}: {e}"))?;
            knx_app::serial_number::project_serial_number(&opaque, device)
                .map_err(|e| format!("project {project}, device {device}: {e}"))?
                .ok_or_else(|| {
                    format!(
                        "project {project} records no serial number for device {device}; \
                         give it with --serial MMMM:NNNNNNNN (from the device label)"
                    )
                })
        }
    }
}

/// What the operator asked for, after the checks that need no socket.
#[derive(Debug)]
pub enum Mode {
    Plan,
    Write {
        gateway: std::net::SocketAddrV4,
        authorisation: WriteAuthorisation,
    },
}

/// Parses the address, refuses an excluded one, and checks the phrase.
pub fn check(args: &AddressBySerialArgs) -> Result<(ContactableAddress, Mode), String> {
    let address: IndividualAddress = args
        .new_address
        .parse()
        .map_err(|e| format!("invalid individual address {}: {e}", args.new_address))?;
    let target = ContactableAddress::new(address).map_err(|e| e.to_string())?;
    let mode = match (&args.gateway, &args.confirm) {
        (Some(gateway), Some(confirmation)) => {
            let gateway = gateway
                .parse()
                .map_err(|_| "--gateway must be host:port, e.g. 192.0.2.1:3671".to_string())?;
            let authorisation = WriteAuthorisation::for_hardware(
                address,
                WriteScope::IndividualAddressProgramming,
                confirmation,
            )
            .map_err(|e| format!("not written: {e}"))?;
            knx_app::serial_address_recovery::require_persistent_pre_write_recovery()
                .map_err(str::to_string)?;
            Mode::Write {
                gateway,
                authorisation,
            }
        }
        _ => Mode::Plan,
    };
    Ok((target, mode))
}

/// What the command will do, printed before anything is sent.
pub fn format_plan(serial: SerialNumber, new_address: IndividualAddress) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "== address device {serial} as {new_address}: plan (nothing sent yet) =="
    );
    let _ = writeln!(
        out,
        "MP §2.5 NM_IndividualAddress_SerialNumber_Write, no programming button:"
    );
    let _ = writeln!(
        out,
        "  1: ask the bus who has serial number {serial} (stops if nobody answers)"
    );
    let _ = writeln!(
        out,
        "  2: check whether {new_address} is already taken (stops if a different device holds it)"
    );
    let _ = writeln!(
        out,
        "  3: write {new_address} to the device with {serial} (skipped if it already has it)"
    );
    let _ = writeln!(
        out,
        "  4: ask again and expect the answer from {new_address}; no restart follows (MP §2.5)"
    );
    out
}

/// Runs MP §2.5 and prints the outcome. Returns whether the device answers
/// from the new address.
pub async fn execute<T: ManagementTransport>(
    transport: &T,
    serial: SerialNumber,
    authorisation: WriteAuthorisation,
    timing: SessionTiming,
    out: &mut impl Write,
) -> bool {
    let new_address = authorisation.target().address();
    match serial_number_write(transport, timing, serial, authorisation).await {
        Ok(report) => {
            summarise(out, &report);
            true
        }
        Err(err) => {
            let _ = writeln!(
                out,
                "== address device {serial} as {new_address}: FAILED: {err} =="
            );
            let _ = writeln!(out, "{}", written_line(&err));
            false
        }
    }
}

fn summarise(out: &mut impl Write, report: &SerialNumberWriteReport) {
    let SerialNumberWriteReport {
        serial_number,
        previous_address,
        occupancy,
        wrote,
        verified_address,
    } = report;
    let _ = writeln!(
        out,
        "== address device {serial_number} as {verified_address}: finished =="
    );
    let _ = writeln!(out, "found at {previous_address}");
    if *occupancy == Occupancy::NotOccupied {
        let _ = writeln!(out, "{verified_address} was free before");
    }
    if *wrote {
        let _ = writeln!(
            out,
            "address written: yes, {previous_address} -> {verified_address}; the device answers \
             from {verified_address}"
        );
    } else {
        let _ = writeln!(
            out,
            "address written: no need, the device already had {verified_address}"
        );
    }
}

/// Whether the device may have changed, for every failure: only a failure
/// after step 3 can follow a write.
fn written_line(err: &SerialNumberWriteError) -> String {
    match err {
        SerialNumberWriteError::NotVerified { .. }
        | SerialNumberWriteError::Session { step: 4, .. } => "address written: sent, but NOT \
             confirmed. Ask again with `knx device find-serial` before anything else"
            .to_string(),
        SerialNumberWriteError::Session { step: 3, .. } => {
            "address written: unknown, the write failed to send".to_string()
        }
        _ => "address written: no".to_string(),
    }
}

/// What `knx device find-serial` asks.
#[derive(Debug, PartialEq, Eq)]
pub enum FindSerial {
    /// MP §2.4: who has this serial number (broadcast).
    Address(SerialNumber),
    /// RES §4.2.11: which serial number the device at this address has
    /// (`PID_SERIAL_NUMBER` over a connection).
    SerialNumber(ContactableAddress),
}

/// `knx device find-serial (<serial> | --address <a.l.d>) --gateway <host:port>`.
/// Read-only either way.
pub struct FindSerialArgs {
    pub query: FindSerial,
    pub gateway: String,
    pub activity_history: Option<String>,
}

pub fn parse_find_serial_args(args: &[String]) -> Result<FindSerialArgs, String> {
    let mut serial = None;
    let mut address = None;
    let mut gateway = None;
    let mut activity_history = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--gateway" => {
                gateway = Some(crate::take_value(args, i + 1, "--gateway")?);
                i += 2;
            }
            "--activity-history" => {
                if activity_history.is_some() {
                    return Err("give --activity-history only once".into());
                }
                activity_history = Some(crate::take_value(args, i + 1, "--activity-history")?);
                i += 2;
            }
            "--address" => {
                address = Some(crate::take_value(args, i + 1, "--address")?);
                i += 2;
            }
            flag if flag.starts_with("--") => return Err(format!("unknown flag {flag}")),
            positional => {
                if serial.replace(positional.to_string()).is_some() {
                    return Err("give exactly one serial number".to_string());
                }
                i += 1;
            }
        }
    }
    let query = match (serial, address) {
        (Some(serial), None) => FindSerial::Address(resolve_serial(&SerialSource::Given(serial))?),
        (None, Some(address)) => {
            let parsed: IndividualAddress = address
                .parse()
                .map_err(|e| format!("invalid individual address {address}: {e}"))?;
            FindSerial::SerialNumber(ContactableAddress::new(parsed).map_err(|e| e.to_string())?)
        }
        (None, None) => {
            return Err("give a serial number (0083:12345678) or --address <a.l.d>".to_string())
        }
        (Some(_), Some(_)) => return Err("give a serial number or --address, not both".to_string()),
    };
    let gateway = gateway.ok_or("find-serial asks the bus and needs --gateway <host:port>")?;
    Ok(FindSerialArgs {
        query,
        gateway,
        activity_history,
    })
}

/// Answers a [`FindSerial`] against `transport`, as the line to print.
/// `Ok(None)` is MP §2.4's "no such device".
pub async fn find<T: ManagementTransport>(
    transport: &T,
    query: &FindSerial,
    timing: SessionTiming,
) -> Result<Option<String>, String> {
    match query {
        FindSerial::Address(serial) => {
            knx_net::commissioning::serial_number_write::serial_number_read(
                transport, *serial, timing,
            )
            .await
            .map(|found| found.map(|address| format!("{serial}: {address}")))
            .map_err(|e| format!("{serial}: {e}"))
        }
        FindSerial::SerialNumber(address) => {
            let address = address.address();
            let mut session = knx_net::ManagementSession::read_only(
                transport,
                address,
                knx_core::commissioning::authorisation::AuthorisationPlan::Skip,
                timing,
            )
            .map_err(|e| format!("{address}: {e}"))?;
            session
                .connect()
                .await
                .map_err(|e| format!("{address}: {e}"))?;
            let read = session.read_serial_number().await;
            session.disconnect().await;
            read.map(|serial| Some(format!("{address}: {serial}")))
                .map_err(|e| format!("{address}: {e}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use knx_net::commissioning::simulator::{SimulatedDevice, SimulatorConfig};

    const SERIAL: &str = "0083:12345678";

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
            post_restart_disconnect_wait: Duration::from_millis(60),
            programming_mode_broadcast_timeout: Duration::from_millis(20),
        }
    }

    fn device() -> SimulatedDevice {
        SimulatedDevice::with_config(SimulatorConfig {
            serial_number: Some(SERIAL.parse::<SerialNumber>().unwrap().octets()),
            programming_mode: false,
            ..Default::default()
        })
    }

    #[test]
    fn the_default_is_a_plan_and_the_phrase_must_name_the_new_address() {
        let parsed = parse_address_by_serial_args(&args(&["1.1.30", "--serial", SERIAL])).unwrap();
        assert!(matches!(check(&parsed).unwrap().1, Mode::Plan));
        for wrong in [
            "I confirm individual-address programming to 1.1.31",
            "I confirm restart to 1.1.30",
        ] {
            let parsed = parse_address_by_serial_args(&args(&[
                "1.1.30",
                "--serial",
                SERIAL,
                "--gateway",
                "192.0.2.1:3671",
                "--confirm",
                wrong,
            ]))
            .unwrap();
            assert!(
                check(&parsed).unwrap_err().contains("not written"),
                "{wrong}"
            );
        }
    }

    #[test]
    fn confirmed_write_refuses_without_durable_backup_before_any_socket() {
        let parsed = parse_address_by_serial_args(&args(&[
            "1.1.30",
            "--serial",
            SERIAL,
            "--gateway",
            "192.0.2.1:3671",
            "--confirm",
            "I confirm individual-address programming to 1.1.30",
        ]))
        .unwrap();
        let error = check(&parsed).unwrap_err();
        assert!(error.contains("backup"), "{error}");
    }

    #[test]
    fn the_device_is_named_exactly_once() {
        for bad in [
            vec!["1.1.30"],
            vec![
                "1.1.30",
                "--serial",
                SERIAL,
                "--project",
                "p.knxdb",
                "--device",
                "D",
            ],
            vec!["1.1.30", "--project", "p.knxdb"],
            vec!["1.1.30", "--device", "D"],
            vec!["1.1.30", "--serial", SERIAL, "--confirm", "x"],
            vec!["1.1.30", "1.1.31", "--serial", SERIAL],
            vec!["1.1.30", "--serial"],
        ] {
            assert!(
                parse_address_by_serial_args(&args(&bad)).is_err(),
                "{bad:?}"
            );
        }
    }

    #[test]
    fn a_bad_serial_is_refused_not_padded() {
        for bad in ["0083:1234567", "83:12345678", "0083:1234567X"] {
            assert!(
                resolve_serial(&SerialSource::Given(bad.to_string())).is_err(),
                "{bad}"
            );
        }
    }

    #[test]
    fn a_project_without_the_serial_asks_for_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty.knxdb");
        let conn = knx_store::open_and_migrate(&path).unwrap();
        knx_store::save_project(
            &conn,
            &knx_core::Project::new(knx_core::Language("en".into())),
        )
        .unwrap();
        let err = resolve_serial(&SerialSource::Project {
            project: path.display().to_string(),
            device: "P-0001-0_DI-1".to_string(),
        })
        .unwrap_err();
        assert!(err.contains("records no serial number"), "{err}");
        assert!(err.contains("--serial"), "{err}");
        let missing = resolve_serial(&SerialSource::Project {
            project: dir.path().join("typo.knxdb").display().to_string(),
            device: "D".to_string(),
        })
        .unwrap_err();
        assert!(missing.contains("project not found"), "{missing}");
        assert!(
            !dir.path().join("typo.knxdb").exists(),
            "a typo must not create a project"
        );
    }

    #[test]
    fn the_plan_names_the_serial_the_address_and_no_restart() {
        let plan = format_plan(SERIAL.parse().unwrap(), "1.1.30".parse().unwrap());
        assert!(plan.contains("nothing sent yet"), "{plan}");
        assert!(plan.contains(SERIAL), "{plan}");
        assert!(plan.contains("no restart"), "{plan}");
        for step in ["  1:", "  2:", "  3:", "  4:"] {
            assert!(plan.contains(step), "{plan}");
        }
    }

    #[tokio::test]
    async fn writes_and_reports_against_the_simulator() {
        let device = device();
        let before = device.address();
        let new_address: IndividualAddress = "1.1.30".parse().unwrap();
        let authorisation = WriteAuthorisation::for_simulator(
            new_address,
            WriteScope::IndividualAddressProgramming,
        )
        .unwrap();
        let mut out = Vec::new();
        let ok = execute(
            &device,
            SERIAL.parse().unwrap(),
            authorisation,
            fast(),
            &mut out,
        )
        .await;
        let out = String::from_utf8(out).unwrap();
        assert!(ok, "{out}");
        assert_eq!(device.address(), new_address);
        assert!(
            out.contains(&format!("address written: yes, {before} -> {new_address}")),
            "{out}"
        );
    }

    #[tokio::test]
    async fn a_refused_write_says_sent_but_not_confirmed() {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            serial_number: Some(SERIAL.parse::<SerialNumber>().unwrap().octets()),
            serial_number_write_enabled: false,
            programming_mode: false,
            ..Default::default()
        });
        let new_address: IndividualAddress = "1.1.30".parse().unwrap();
        let authorisation = WriteAuthorisation::for_simulator(
            new_address,
            WriteScope::IndividualAddressProgramming,
        )
        .unwrap();
        let mut out = Vec::new();
        let ok = execute(
            &device,
            SERIAL.parse().unwrap(),
            authorisation,
            fast(),
            &mut out,
        )
        .await;
        let out = String::from_utf8(out).unwrap();
        assert!(!ok, "{out}");
        assert!(out.contains("sent, but NOT confirmed"), "{out}");
    }

    #[test]
    fn find_serial_accepts_explicit_activity_history_without_io() {
        for question in [vec![SERIAL], vec!["--address", "1.1.67"]] {
            let mut input = question;
            input.extend([
                "--gateway",
                "192.0.2.1:3671",
                "--activity-history",
                "not-opened-history.sqlite",
            ]);
            let parsed = parse_find_serial_args(&args(&input))
                .expect("find-serial must accept an explicit metadata history path");
            assert_eq!(
                parsed.activity_history.as_deref(),
                Some("not-opened-history.sqlite")
            );
            assert_eq!(parsed.gateway, "192.0.2.1:3671");
        }
    }

    #[test]
    fn find_serial_takes_exactly_one_question() {
        let FindSerialArgs {
            query,
            activity_history,
            ..
        } = parse_find_serial_args(&args(&[SERIAL, "--gateway", "192.0.2.1:3671"])).unwrap();
        assert!(activity_history.is_none());
        assert_eq!(query, FindSerial::Address(SERIAL.parse().unwrap()));
        let FindSerialArgs { query, .. } = parse_find_serial_args(&args(&[
            "--address",
            "1.1.67",
            "--gateway",
            "192.0.2.1:3671",
        ]))
        .unwrap();
        assert!(matches!(query, FindSerial::SerialNumber(_)));
        for bad in [
            vec!["--gateway", "192.0.2.1:3671"],
            vec![SERIAL, "--address", "1.1.67", "--gateway", "192.0.2.1:3671"],
            vec![SERIAL],
            vec!["--address", "1.1.300", "--gateway", "192.0.2.1:3671"],
        ] {
            assert!(parse_find_serial_args(&args(&bad)).is_err(), "{bad:?}");
        }
        let excluded = knx_core::EXCLUDED_INDIVIDUAL_ADDRESSES[0].to_string();
        assert!(parse_find_serial_args(&args(&[
            "--address",
            &excluded,
            "--gateway",
            "192.0.2.1:3671"
        ]))
        .is_err());
    }

    #[tokio::test]
    async fn find_answers_both_ways_against_the_simulator() {
        let device = device();
        let by_serial = find(
            &device,
            &FindSerial::Address(SERIAL.parse().unwrap()),
            fast(),
        )
        .await
        .unwrap();
        assert_eq!(by_serial, Some(format!("{SERIAL}: {}", device.address())));
        let by_address = find(
            &device,
            &FindSerial::SerialNumber(ContactableAddress::new(device.address()).unwrap()),
            fast(),
        )
        .await
        .unwrap();
        assert_eq!(by_address, Some(format!("{}: {SERIAL}", device.address())));
        let nobody = find(
            &device,
            &FindSerial::Address("0083:00000001".parse().unwrap()),
            fast(),
        )
        .await
        .unwrap();
        assert_eq!(nobody, None);
    }

    #[tokio::test]
    async fn nobody_with_that_serial_writes_nothing() {
        let device = device();
        let new_address: IndividualAddress = "1.1.30".parse().unwrap();
        let authorisation = WriteAuthorisation::for_simulator(
            new_address,
            WriteScope::IndividualAddressProgramming,
        )
        .unwrap();
        let mut out = Vec::new();
        let ok = execute(
            &device,
            "0083:00000001".parse().unwrap(),
            authorisation,
            fast(),
            &mut out,
        )
        .await;
        let out = String::from_utf8(out).unwrap();
        assert!(!ok);
        assert!(out.ends_with("address written: no\n"), "{out}");
        assert_eq!(device.serial_number_writes(), 0);
    }
}
