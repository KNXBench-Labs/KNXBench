//! `knx device reset-address`: gives the pressed devices back the default address `15.15.255`.
//!
//! MP §2.18 `NM_IndividualAddress_Reset`, driven by
//! [`individual_address_reset`]. The procedure resets *every* device in
//! programming mode, so the operator names the devices whose buttons they
//! pressed, by their current address, and the first broadcast read must
//! find exactly those; anything else writes nothing.
//!
//! Without `--confirm` it prints what the protocol procedure would do and
//! opens no socket. A confirmed production CLI call currently fails closed
//! before opening a tunnel because it lacks a complete durable pre-write
//! backup (ADR-0058). The phrase is `required_confirmation_phrase` for
//! `15.15.255` and `WriteScope::IndividualAddressReset`; it is not itself
//! recovery evidence. Reprogramming the address with the button pressed
//! again is only an address-recovery step, not a whole-device backup.

use std::fmt::Write as _;
use std::io::Write;

use knx_core::commissioning::mutation::{WriteAuthorisation, WriteScope};
use knx_core::{ContactableAddress, IndividualAddress};
use knx_net::commissioning::individual_address_reset::{
    individual_address_reset, IndividualAddressResetError, DEFAULT_INDIVIDUAL_ADDRESS, MAX_ROUNDS,
};
use knx_net::{ManagementTransport, SessionTiming};

/// Raw `knx device reset-address` arguments.
#[derive(Debug, PartialEq, Eq)]
pub struct ResetAddressArgs {
    /// The devices the operator expects in programming mode.
    pub expected: Vec<String>,
    pub gateway: Option<String>,
    pub confirm: Option<String>,
}

/// Parses `<a.l.d>... [--gateway <host:port> --confirm <phrase>]`.
pub fn parse_reset_address_args(args: &[String]) -> Result<ResetAddressArgs, String> {
    let mut expected = Vec::new();
    let mut gateway = None;
    let mut confirm = None;
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
            flag if flag.starts_with("--") => return Err(format!("unknown flag {flag}")),
            positional => expected.push(positional.to_string()),
        }
        i += 1;
    }
    if expected.is_empty() {
        return Err(
            "name the devices whose programming button is pressed, by their current address, \
             e.g. 1.1.67"
                .to_string(),
        );
    }
    if confirm.is_some() && gateway.is_none() {
        return Err("--confirm writes to devices and needs --gateway <host:port>".to_string());
    }
    Ok(ResetAddressArgs {
        expected,
        gateway,
        confirm,
    })
}

/// What the operator asked for, after the checks that need no socket.
#[derive(Debug)]
pub enum Mode {
    /// Print what would happen.
    Plan,
    /// Reset, through this gateway.
    Reset {
        gateway: std::net::SocketAddrV4,
        authorisation: WriteAuthorisation,
    },
}

/// Parses the named devices (none excluded, none already `15.15.255`) and
/// checks the phrase.
pub fn check(args: &ResetAddressArgs) -> Result<(Vec<IndividualAddress>, Mode), String> {
    let mut expected = Vec::new();
    for text in &args.expected {
        let address: IndividualAddress = text
            .parse()
            .map_err(|e| format!("invalid individual address {text}: {e}"))?;
        ContactableAddress::new(address).map_err(|e| e.to_string())?;
        if address == DEFAULT_INDIVIDUAL_ADDRESS {
            return Err(format!(
                "{address} is already the default address; name the devices by the address \
                 they have now"
            ));
        }
        if !expected.contains(&address) {
            expected.push(address);
        }
    }
    let mode = match (&args.gateway, &args.confirm) {
        (Some(gateway), Some(confirmation)) => {
            let gateway = gateway
                .parse()
                .map_err(|_| "--gateway must be host:port, e.g. 192.0.2.1:3671".to_string())?;
            let authorisation = WriteAuthorisation::for_hardware(
                DEFAULT_INDIVIDUAL_ADDRESS,
                WriteScope::IndividualAddressReset,
                confirmation,
            )
            .map_err(|e| format!("not written: {e}"))?;
            Mode::Reset {
                gateway,
                authorisation,
            }
        }
        _ => Mode::Plan,
    };
    Ok((expected, mode))
}

/// What the command will do, printed before anything is sent.
pub fn format_plan(expected: &[IndividualAddress]) -> String {
    let names: Vec<_> = expected.iter().map(ToString::to_string).collect();
    let mut out = String::new();
    let _ = writeln!(
        out,
        "== reset individual address to {DEFAULT_INDIVIDUAL_ADDRESS}: plan (nothing sent yet) =="
    );
    let _ = writeln!(
        out,
        "expected in programming mode: {} (press their buttons first)",
        names.join(", ")
    );
    let _ = writeln!(out, "then MP §2.18 NM_IndividualAddress_Reset:");
    let _ = writeln!(
        out,
        "  0: ask who is in programming mode; stop unless it is exactly the devices above"
    );
    let _ = writeln!(
        out,
        "  1: write {DEFAULT_INDIVIDUAL_ADDRESS} to every device in programming mode (broadcast)"
    );
    let _ = writeln!(
        out,
        "  2: connect, restart and disconnect {DEFAULT_INDIVIDUAL_ADDRESS} (ends programming mode)"
    );
    let _ = writeln!(
        out,
        "  3: ask again; repeat 1-3 up to {MAX_ROUNDS} times until nobody answers"
    );
    let _ = writeln!(
        out,
        "recovery: `knx device program-address <address>` with the button pressed again"
    );
    out
}

/// Runs the reset and prints the outcome. Returns whether the reset ran and
/// nobody answered its closing read; whether programming mode really ended
/// is the operator's LED to judge.
pub async fn execute<T: ManagementTransport>(
    transport: &T,
    expected: &[IndividualAddress],
    authorisation: WriteAuthorisation,
    timing: SessionTiming,
    out: &mut impl Write,
) -> bool {
    let _ = writeln!(
        out,
        "== reset individual address to {DEFAULT_INDIVIDUAL_ADDRESS}: running =="
    );
    match individual_address_reset(transport, timing, authorisation, expected).await {
        Ok(report) if report.in_programming_mode.is_empty() => {
            let _ = writeln!(out, "no device is in programming mode; address written: no");
            false
        }
        Ok(report) => {
            let names: Vec<_> = report
                .in_programming_mode
                .iter()
                .map(ToString::to_string)
                .collect();
            let _ = writeln!(
                out,
                "== reset individual address to {DEFAULT_INDIVIDUAL_ADDRESS}: finished in {} \
                 round(s) ==",
                report.rounds
            );
            let _ = writeln!(
                out,
                "address written: yes, {} -> {DEFAULT_INDIVIDUAL_ADDRESS}; nobody answered the \
                 closing read",
                names.join(", ")
            );
            // MP §2.18 sends the restart unevaluated, and the closing read
            // is no proof it happened: live on 2026-09-30 the MDT push button
            // kept its LED on and answered in programming mode again later
            // (RESEARCH §19.16).
            let _ = writeln!(
                out,
                "restart: not confirmed (MP §2.18 does not listen for it). If a programming LED \
                 is still on, that device is still in programming mode: give it an address \
                 with `knx device program-address` or press its button once"
            );
            true
        }
        Err(err) => {
            let _ = writeln!(
                out,
                "== reset individual address to {DEFAULT_INDIVIDUAL_ADDRESS}: FAILED: {err} =="
            );
            let _ = writeln!(out, "{}", written_line(&err));
            false
        }
    }
}

/// Whether anything changed, for every failure. The guard and the first
/// read fail before the first write; a failure inside a round may come
/// after it.
fn written_line(err: &IndividualAddressResetError) -> String {
    match err {
        IndividualAddressResetError::NotTheExpectedDevices { .. }
        | IndividualAddressResetError::ExcludedDeviceInProgrammingMode(_) => {
            "address written: no".to_string()
        }
        IndividualAddressResetError::Session { step, .. }
            if *step == "authorisation" || *step == "who is in programming mode" =>
        {
            "address written: no".to_string()
        }
        IndividualAddressResetError::StillInProgrammingMode { .. }
        | IndividualAddressResetError::Session { .. } => format!(
            "address written: UNKNOWN: the devices may already be at \
             {DEFAULT_INDIVIDUAL_ADDRESS}. Scan the line before anything else; recovery is \
             `knx device program-address` with the button pressed"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use knx_net::commissioning::simulator::{SimulatedDevice, SimulatorConfig};
    use std::time::Duration;

    fn phrase() -> String {
        knx_core::commissioning::mutation::required_confirmation_phrase(
            DEFAULT_INDIVIDUAL_ADDRESS,
            WriteScope::IndividualAddressReset,
        )
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
            post_restart_disconnect_wait: Duration::from_millis(60),
            programming_mode_broadcast_timeout: Duration::from_millis(20),
        }
    }

    fn simulator_authorisation() -> WriteAuthorisation {
        WriteAuthorisation::for_simulator(
            DEFAULT_INDIVIDUAL_ADDRESS,
            WriteScope::IndividualAddressReset,
        )
        .unwrap()
    }

    #[test]
    fn the_default_is_a_plan() {
        let parsed = parse_reset_address_args(&args(&["1.1.67"])).unwrap();
        let (expected, mode) = check(&parsed).unwrap();
        assert_eq!(expected, vec!["1.1.67".parse().unwrap()]);
        assert!(matches!(mode, Mode::Plan));
        let parsed =
            parse_reset_address_args(&args(&["1.1.67", "--gateway", "192.0.2.1:3671"])).unwrap();
        assert!(matches!(check(&parsed).unwrap().1, Mode::Plan));
    }

    #[test]
    fn the_phrase_names_the_default_address_and_the_reset() {
        assert_eq!(phrase(), "I confirm individual-address reset to 15.15.255");
        for wrong in [
            "I confirm individual-address reset to 1.1.67",
            "I confirm individual-address programming to 15.15.255",
            "I confirm download to 1.1.67",
        ] {
            let parsed = parse_reset_address_args(&args(&[
                "1.1.67",
                "--gateway",
                "192.0.2.1:3671",
                "--confirm",
                wrong,
            ]))
            .unwrap();
            let refused = check(&parsed).expect_err(wrong);
            assert!(refused.contains("not written"), "{wrong:?}: {refused}");
        }
        let parsed = parse_reset_address_args(&args(&[
            "1.1.67",
            "--gateway",
            "192.0.2.1:3671",
            "--confirm",
            &phrase(),
        ]))
        .unwrap();
        let Mode::Reset { authorisation, .. } = check(&parsed).unwrap().1 else {
            panic!("the right phrase must reset");
        };
        assert_eq!(authorisation.scope(), WriteScope::IndividualAddressReset);
        assert_eq!(authorisation.target().address(), DEFAULT_INDIVIDUAL_ADDRESS);
    }

    #[test]
    fn bad_arguments_are_refused() {
        assert!(parse_reset_address_args(&args(&[])).is_err());
        assert!(parse_reset_address_args(&args(&["1.1.67", "--confirm", "x"])).is_err());
        assert!(parse_reset_address_args(&args(&["1.1.67", "--wait", "5"])).is_err());
        assert!(check(&parse_reset_address_args(&args(&["1.1.300"])).unwrap()).is_err());
        assert!(check(&parse_reset_address_args(&args(&["15.15.255"])).unwrap()).is_err());
        let excluded = knx_core::EXCLUDED_INDIVIDUAL_ADDRESSES[0].to_string();
        assert!(check(&parse_reset_address_args(&args(&[&excluded])).unwrap()).is_err());
    }

    #[test]
    fn repeats_are_named_once() {
        let parsed = parse_reset_address_args(&args(&["1.1.67", "1.1.67"])).unwrap();
        assert_eq!(check(&parsed).unwrap().0.len(), 1);
    }

    #[test]
    fn the_plan_names_the_devices_the_steps_and_the_recovery() {
        let plan = format_plan(&["1.1.67".parse().unwrap()]);
        assert!(plan.contains("nothing sent yet"), "{plan}");
        assert!(
            plan.contains("expected in programming mode: 1.1.67"),
            "{plan}"
        );
        for step in ["  0:", "  1:", "  2:", "  3:", "recovery:"] {
            assert!(plan.contains(step), "{plan}");
        }
    }

    #[tokio::test]
    async fn the_named_device_is_reset_and_it_says_so() {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            programming_mode: true,
            ..Default::default()
        });
        let before = device.address();
        let mut out = Vec::new();
        let ok = execute(
            &device,
            &[before],
            simulator_authorisation(),
            fast(),
            &mut out,
        )
        .await;
        let out = String::from_utf8(out).unwrap();
        assert!(ok, "{out}");
        assert_eq!(device.address(), DEFAULT_INDIVIDUAL_ADDRESS);
        assert!(
            out.contains(&format!("address written: yes, {before} -> 15.15.255")),
            "{out}"
        );
        // Live, 2026-09-30: the check read went unanswered while 1.1.67's
        // LED stayed on, and the device answered in programming mode again
        // later. The closing read is not proof that programming mode ended.
        assert!(!out.contains("any more"), "{out}");
        assert!(out.contains("programming LED"), "{out}");
        assert!(out.contains("program-address"), "{out}");
    }

    #[tokio::test]
    async fn a_stranger_in_programming_mode_writes_nothing() {
        let stranger: IndividualAddress = "1.1.40".parse().unwrap();
        let device = SimulatedDevice::with_config(SimulatorConfig {
            programming_mode: true,
            other_programming_mode_devices: vec![stranger],
            ..Default::default()
        });
        let before = device.address();
        let mut out = Vec::new();
        let ok = execute(
            &device,
            &[before],
            simulator_authorisation(),
            fast(),
            &mut out,
        )
        .await;
        let out = String::from_utf8(out).unwrap();
        assert!(!ok);
        assert_eq!(device.address(), before);
        assert!(out.contains("1.1.40"), "{out}");
        assert!(out.ends_with("address written: no\n"), "{out}");
    }

    #[tokio::test]
    async fn nobody_pressed_writes_nothing() {
        let device = SimulatedDevice::new();
        let before = device.address();
        let mut out = Vec::new();
        let ok = execute(
            &device,
            &[before],
            simulator_authorisation(),
            fast(),
            &mut out,
        )
        .await;
        let out = String::from_utf8(out).unwrap();
        assert!(!ok);
        assert_eq!(device.address(), before);
        assert!(out.contains("expected: "), "{out}");
        assert!(out.ends_with("address written: no\n"), "{out}");
    }

    #[tokio::test]
    async fn a_device_that_stays_pressed_is_reported_as_unknown() {
        let stuck: IndividualAddress = "1.1.40".parse().unwrap();
        let device = SimulatedDevice::with_config(SimulatorConfig {
            programming_mode: true,
            other_programming_mode_devices: vec![stuck],
            other_programming_mode_devices_ignore_restart: true,
            ..Default::default()
        });
        let before = device.address();
        let mut out = Vec::new();
        let ok = execute(
            &device,
            &[before, stuck],
            simulator_authorisation(),
            fast(),
            &mut out,
        )
        .await;
        let out = String::from_utf8(out).unwrap();
        assert!(!ok);
        assert!(out.contains("address written: UNKNOWN"), "{out}");
    }
}
