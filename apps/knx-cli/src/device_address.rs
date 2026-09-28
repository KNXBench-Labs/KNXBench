//! `knx device program-address`: gives the one device in programming mode its new address.
//!
//! MP §2.3 `NM_IndividualAddress_Write`, driven by
//! [`program_individual_address`]: wait, round by round, until exactly one
//! device is in programming mode (the Standard's step 2 `repeat`, lifted out
//! of the library, KNOWN_LIMITATIONS §116), then the four steps. This is
//! programming the individual address, not a download (docs/GLOSSARY.md).
//!
//! Without `--confirm` it prints what would happen and opens no socket. The
//! phrase is [`required_confirmation_phrase`] for the new address and
//! [`WriteScope::IndividualAddressProgramming`]. It covers the whole
//! procedure, including step 4's restart: MP §2.3 makes that restart part
//! of the procedure ("shall deactivate the Programming Mode by executing a
//! restart"), so the restart authorisation is derived here, and only after
//! the programming phrase matched.

use std::fmt::Write as _;
use std::io::Write;
use std::ops::ControlFlow;
use std::time::Duration;

use knx_core::commissioning::authorisation::AuthorisationPlan;
use knx_core::commissioning::mutation::{
    required_confirmation_phrase, WriteAuthorisation, WriteScope,
};
use knx_core::{ContactableAddress, IndividualAddress};
use knx_net::commissioning::individual_address_write::{IndividualAddressWriteError, Occupancy};
use knx_net::commissioning::programming_button_wait::{
    program_individual_address, write_count, AddressProgrammingAuthorisation, ButtonEvent,
    ButtonProgrammingError, ButtonProgrammingReport, ButtonWait,
};
use knx_net::{ManagementTransport, SessionTiming};

/// How long to wait for a button when `--wait` is not given.
pub const DEFAULT_WAIT: Duration = Duration::from_secs(120);
/// The longest `--wait` accepted. `[A]` RES §4.26.1 lets a device leave
/// programming mode by itself after four minutes; waiting ten covers a
/// walk across a building without making a forgotten terminal wait all day.
pub const MAX_WAIT: Duration = Duration::from_secs(600);

/// Raw `knx device program-address` arguments.
#[derive(Debug, PartialEq, Eq)]
pub struct ProgramAddressArgs {
    pub new_address: String,
    pub gateway: Option<String>,
    pub confirm: Option<String>,
    pub wait: Duration,
}

pub fn parse_program_address_args(args: &[String]) -> Result<ProgramAddressArgs, String> {
    let mut new_address = None;
    let mut gateway = None;
    let mut confirm = None;
    let mut wait = DEFAULT_WAIT;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--gateway" => {
                gateway = Some(crate::take_value(args, i + 1, "--gateway")?);
                i += 1;
            }
            "--confirm" => {
                confirm = Some(crate::take_value(args, i + 1, "--confirm")?);
                i += 1;
            }
            "--wait" => {
                let value = crate::take_value(args, i + 1, "--wait")?;
                let seconds: u64 = value
                    .parse()
                    .map_err(|_| format!("--wait takes whole seconds, not {value:?}"))?;
                wait = Duration::from_secs(seconds);
                if wait.is_zero() || wait > MAX_WAIT {
                    return Err(format!(
                        "--wait must be between 1 and {} seconds",
                        MAX_WAIT.as_secs()
                    ));
                }
                i += 1;
            }
            flag if flag.starts_with("--") => return Err(format!("unknown flag {flag}")),
            positional => {
                if new_address.replace(positional.to_string()).is_some() {
                    return Err("give exactly one new address".to_string());
                }
            }
        }
        i += 1;
    }
    let new_address = new_address.ok_or("missing the new address, e.g. 1.1.30")?;
    if confirm.is_some() && gateway.is_none() {
        return Err("--confirm writes to a device and needs --gateway <host:port>".to_string());
    }
    Ok(ProgramAddressArgs {
        new_address,
        gateway,
        confirm,
        wait,
    })
}

/// What the operator asked for, after the checks that need no socket.
#[derive(Debug)]
pub enum Mode {
    /// Print what would happen.
    Plan,
    /// Program, through this gateway.
    Program {
        gateway: std::net::SocketAddrV4,
        authorisation: AddressProgrammingAuthorisation,
    },
}

/// Parses the address, refuses an excluded one, and checks the phrase.
pub fn check(args: &ProgramAddressArgs) -> Result<(ContactableAddress, Mode), String> {
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
            let programming = WriteAuthorisation::for_hardware(
                address,
                WriteScope::IndividualAddressProgramming,
                confirmation,
            )
            .map_err(|e| format!("not written: {e}"))?;
            // Only reached with the programming phrase matched: step 4's
            // restart is part of the procedure the operator confirmed.
            let restart = WriteAuthorisation::for_hardware(
                address,
                WriteScope::Restart,
                &required_confirmation_phrase(address, WriteScope::Restart),
            )
            .map_err(|e| format!("not written: {e}"))?;
            Mode::Program {
                gateway,
                authorisation: AddressProgrammingAuthorisation {
                    programming,
                    restart,
                },
            }
        }
        _ => Mode::Plan,
    };
    Ok((target, mode))
}

/// What the command will do, printed before anything is sent.
pub fn format_plan(new_address: IndividualAddress, wait: Duration) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "== program individual address {new_address}: plan (nothing sent yet) =="
    );
    let _ = writeln!(
        out,
        "wait up to {} s for exactly one device in programming mode (asks once about every 2 s)",
        wait.as_secs()
    );
    let _ = writeln!(out, "then MP §2.3 NM_IndividualAddress_Write:");
    let _ = writeln!(
        out,
        "  1: check whether {new_address} is already taken (stops if a different device holds it)"
    );
    let _ = writeln!(
        out,
        "  2: count devices in programming mode again (exactly one)"
    );
    let _ = writeln!(
        out,
        "  3: write {new_address} to that device (skipped if it already has it)"
    );
    let _ = writeln!(
        out,
        "  4: connect to {new_address}, read it back, restart it (ends programming mode)"
    );
    out
}

/// Runs the wait and the procedure, printing each change the operator
/// must act on, then the outcome. Returns whether the device now has the
/// address.
pub async fn execute<T: ManagementTransport>(
    transport: &T,
    new_address: IndividualAddress,
    authorisation: AddressProgrammingAuthorisation,
    timing: SessionTiming,
    wait: ButtonWait,
    out: &mut impl Write,
) -> bool {
    let _ = writeln!(
        out,
        "== program individual address {new_address}: waiting for a programming button =="
    );
    let mut last: Option<Vec<IndividualAddress>> = None;
    let result = program_individual_address(
        transport,
        AuthorisationPlan::Skip,
        timing,
        new_address,
        authorisation,
        wait,
        |event| {
            match event {
                // Only when the answer changes: a waiting operator needs the
                // instruction, not a line every two seconds.
                ButtonEvent::Round {
                    number,
                    in_programming_mode,
                } if last.as_ref() != Some(in_programming_mode) => {
                    let mut line = String::new();
                    let _ = write_count(&mut line, in_programming_mode);
                    let hint = match in_programming_mode.len() {
                        0 => " — press the programming button on the device to address",
                        1 => "",
                        _ => " — release all but one",
                    };
                    let _ = writeln!(out, "  [round {number}] {line}{hint}");
                    last = Some(in_programming_mode.clone());
                }
                ButtonEvent::Round { .. } => {}
                ButtonEvent::Found { current_address } => {
                    let _ = writeln!(
                        out,
                        "  found {current_address} in programming mode; programming {new_address}"
                    );
                }
            }
            ControlFlow::Continue(())
        },
    )
    .await;
    match result {
        Ok(report) => {
            summarise(out, new_address, &report);
            true
        }
        Err(err) => {
            let _ = writeln!(
                out,
                "== program individual address {new_address}: FAILED: {err} =="
            );
            let _ = writeln!(out, "{}", written_line(&err));
            false
        }
    }
}

fn summarise(
    out: &mut impl Write,
    new_address: IndividualAddress,
    report: &ButtonProgrammingReport,
) {
    let _ = writeln!(
        out,
        "== program individual address {new_address}: finished =="
    );
    let occupancy = match report.procedure.occupancy {
        Occupancy::NotOccupied => "was free".to_string(),
        Occupancy::OccupiedWithResponse | Occupancy::OccupiedAfterDisconnect => {
            format!(
                "was already held by this device ({})",
                report.previous_address
            )
        }
    };
    let _ = writeln!(out, "{new_address} {occupancy} before");
    if report.procedure.wrote {
        let _ = writeln!(
            out,
            "address written: yes, {} -> {new_address}; the device answered at {new_address} and was restarted",
            report.previous_address
        );
    } else {
        let _ = writeln!(
            out,
            "address written: no need, the device already had {new_address}; it answered and was restarted"
        );
    }
}

/// The line that says whether the device changed, for every failure.
/// KNOWN_LIMITATIONS §7 item 2: a failed step 4 after a write must not
/// read as "nothing happened".
fn written_line(err: &ButtonProgrammingError) -> String {
    match err {
        ButtonProgrammingError::Procedure(IndividualAddressWriteError::Session {
            step,
            report,
            ..
        }) if report.wrote => format!(
            "address written: yes, but NOT confirmed: the device did not answer at the new \
             address in step {step}. Check it with a read before anything else; recovery is \
             another programming-mode session"
        ),
        _ => "address written: no".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use knx_net::commissioning::simulator::{SimulatedDevice, SimulatorConfig};

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

    fn simulator_authorisations(address: IndividualAddress) -> AddressProgrammingAuthorisation {
        AddressProgrammingAuthorisation {
            programming: WriteAuthorisation::for_simulator(
                address,
                WriteScope::IndividualAddressProgramming,
            )
            .unwrap(),
            restart: WriteAuthorisation::for_simulator(address, WriteScope::Restart).unwrap(),
        }
    }

    #[test]
    fn the_default_is_a_plan() {
        let parsed = parse_program_address_args(&args(&["1.1.30"])).unwrap();
        assert_eq!(parsed.wait, DEFAULT_WAIT);
        assert!(matches!(check(&parsed).unwrap().1, Mode::Plan));
    }

    #[test]
    fn a_gateway_without_the_phrase_is_still_a_plan() {
        let parsed =
            parse_program_address_args(&args(&["1.1.30", "--gateway", "192.0.2.1:3671"])).unwrap();
        assert!(matches!(check(&parsed).unwrap().1, Mode::Plan));
    }

    #[test]
    fn the_phrase_must_name_the_new_address_and_this_procedure() {
        for wrong in [
            "I confirm individual-address programming to 1.1.31",
            "I confirm download to 1.1.30",
            "I confirm restart to 1.1.30",
            "",
        ] {
            let parsed = parse_program_address_args(&args(&[
                "1.1.30",
                "--gateway",
                "192.0.2.1:3671",
                "--confirm",
                wrong,
            ]));
            let refused = match parsed {
                Ok(parsed) => check(&parsed).expect_err(wrong),
                Err(e) => e,
            };
            assert!(
                refused.contains("not written") || refused.contains("needs a value"),
                "{wrong:?}: {refused}"
            );
        }
        let parsed = parse_program_address_args(&args(&[
            "1.1.30",
            "--gateway",
            "192.0.2.1:3671",
            "--confirm",
            "I confirm individual-address programming to 1.1.30",
        ]))
        .unwrap();
        let (_, mode) = check(&parsed).unwrap();
        let Mode::Program { authorisation, .. } = mode else {
            panic!("the right phrase must program");
        };
        assert_eq!(
            authorisation.programming.scope(),
            WriteScope::IndividualAddressProgramming
        );
        assert_eq!(authorisation.restart.scope(), WriteScope::Restart);
    }

    #[test]
    fn an_excluded_address_is_refused_before_the_phrase_is_read() {
        let excluded = knx_core::EXCLUDED_INDIVIDUAL_ADDRESSES[0];
        let parsed = parse_program_address_args(&args(&[&excluded.to_string()])).unwrap();
        assert!(check(&parsed).is_err());
    }

    #[test]
    fn bad_arguments_are_refused() {
        assert!(parse_program_address_args(&args(&[])).is_err());
        assert!(parse_program_address_args(&args(&["1.1.30", "1.1.31"])).is_err());
        assert!(parse_program_address_args(&args(&["1.1.30", "--wait", "0"])).is_err());
        assert!(parse_program_address_args(&args(&["1.1.30", "--wait", "601"])).is_err());
        assert!(parse_program_address_args(&args(&["1.1.30", "--wait", "soon"])).is_err());
        assert!(parse_program_address_args(&args(&["1.1.30", "--confirm", "x"])).is_err());
        assert!(parse_program_address_args(&args(&["1.1.30", "--frobnicate"])).is_err());
        assert!(check(&parse_program_address_args(&args(&["1.1.300"])).unwrap()).is_err());
    }

    #[test]
    fn the_plan_names_the_address_and_all_four_steps() {
        let plan = format_plan("1.1.30".parse().unwrap(), Duration::from_secs(90));
        assert!(plan.contains("nothing sent yet"));
        assert!(plan.contains("90 s"));
        for step in ["  1:", "  2:", "  3:", "  4:"] {
            assert!(plan.contains(step), "{plan}");
        }
    }

    /// The whole path against the simulator: the operator presses the
    /// button during the wait, and the output tells them when to.
    #[tokio::test]
    async fn waits_for_the_button_then_programs_and_says_so() {
        let device = std::sync::Arc::new(SimulatedDevice::new());
        let original = device.address();
        let new_address: IndividualAddress = "1.1.30".parse().unwrap();
        let authorisation = simulator_authorisations(new_address);
        let presser = std::sync::Arc::clone(&device);
        let press = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(80)).await;
            presser.set_programming_mode(true);
        });
        let mut out = Vec::new();
        let ok = execute(
            device.as_ref(),
            new_address,
            authorisation,
            fast(),
            ButtonWait {
                give_up_after: Duration::from_secs(5),
                pause_between_rounds: Duration::from_millis(5),
            },
            &mut out,
        )
        .await;
        press.await.unwrap();
        let out = String::from_utf8(out).unwrap();
        assert!(ok, "{out}");
        assert_eq!(device.address(), new_address);
        assert!(
            out.contains("no device is in programming mode — press the programming button"),
            "{out}"
        );
        assert_eq!(
            out.matches("no device is in programming mode").count(),
            1,
            "an unchanged answer is printed once: {out}"
        );
        assert!(
            out.contains(&format!("found {original} in programming mode")),
            "{out}"
        );
        assert!(
            out.contains(&format!(
                "address written: yes, {original} -> {new_address}"
            )),
            "{out}"
        );
    }

    #[tokio::test]
    async fn nobody_pressing_ends_with_nothing_written() {
        let device = SimulatedDevice::new();
        let new_address: IndividualAddress = "1.1.31".parse().unwrap();
        let authorisation = simulator_authorisations(new_address);
        let mut out = Vec::new();
        let ok = execute(
            &device,
            new_address,
            authorisation,
            fast(),
            ButtonWait {
                give_up_after: Duration::from_millis(100),
                pause_between_rounds: Duration::from_millis(5),
            },
            &mut out,
        )
        .await;
        let out = String::from_utf8(out).unwrap();
        assert!(!ok);
        assert!(out.contains("gave up after"), "{out}");
        assert!(out.ends_with("address written: no\n"), "{out}");
    }

    /// KNOWN_LIMITATIONS §7 item 2: written, then silent at the new
    /// address, must not read as "nothing happened".
    #[tokio::test]
    async fn written_but_unconfirmed_says_so() {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            programming_mode: true,
            unanswered_connects: Some(1..3),
            ..Default::default()
        });
        let new_address: IndividualAddress = "1.1.32".parse().unwrap();
        let authorisation = simulator_authorisations(new_address);
        let mut out = Vec::new();
        let ok = execute(
            &device,
            new_address,
            authorisation,
            fast(),
            ButtonWait {
                give_up_after: Duration::from_secs(5),
                pause_between_rounds: Duration::from_millis(5),
            },
            &mut out,
        )
        .await;
        let out = String::from_utf8(out).unwrap();
        assert!(!ok, "{out}");
        assert_eq!(device.address(), new_address, "the write itself landed");
        assert!(
            out.contains("address written: yes, but NOT confirmed"),
            "{out}"
        );
    }

    #[tokio::test]
    async fn another_device_on_the_new_address_stops_before_writing() {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            programming_mode: true,
            ..Default::default()
        });
        let original = device.address();
        // Somebody else answers at the new address: the simulator itself,
        // renamed, is not it, so point the new address at the simulator's
        // own address and put the programming-mode witness elsewhere.
        let other: IndividualAddress = "1.1.40".parse().unwrap();
        device.set_programming_mode(false);
        device.set_other_programming_mode_devices(vec![other]);
        let authorisation = simulator_authorisations(original);
        let mut out = Vec::new();
        let ok = execute(
            &device,
            original,
            authorisation,
            fast(),
            ButtonWait {
                give_up_after: Duration::from_secs(5),
                pause_between_rounds: Duration::from_millis(5),
            },
            &mut out,
        )
        .await;
        let out = String::from_utf8(out).unwrap();
        assert!(!ok, "{out}");
        assert!(out.contains("occupied by a device other than"), "{out}");
        assert!(out.ends_with("address written: no\n"), "{out}");
        assert_eq!(device.address(), original);
    }
}
