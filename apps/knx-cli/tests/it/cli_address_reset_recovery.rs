//! Public CLI reset must not open a tunnel without a complete durable backup.
use std::net::UdpSocket;
use std::process::Command;
use std::time::Duration;

const CONFIRM: &str = "I confirm individual-address reset to 15.15.255";

#[test]
fn confirmed_reset_refuses_without_durable_recovery_before_any_socket() {
    let listener = UdpSocket::bind("127.0.0.1:0").unwrap();
    listener
        .set_read_timeout(Some(Duration::from_millis(200)))
        .unwrap();
    let gateway = listener.local_addr().unwrap().to_string();
    let output = Command::new(env!("CARGO_BIN_EXE_knx"))
        .args([
            "device",
            "reset-address",
            "1.1.67",
            "--gateway",
            &gateway,
            "--confirm",
            CONFIRM,
        ])
        .output()
        .unwrap();
    assert!(!output.status.success(), "{:?}", output);
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("no verified durable pre-write backup"),
        "{stderr}"
    );
    let mut packet = [0; 1];
    let error = listener.recv(&mut packet).unwrap_err();
    assert!(
        matches!(
            error.kind(),
            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
        ),
        "unexpected socket result: {error}"
    );
}

#[test]
fn reset_plan_still_works_but_does_not_offer_a_working_write_flag() {
    let output = Command::new(env!("CARGO_BIN_EXE_knx"))
        .args(["device", "reset-address", "1.1.67"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{:?}", output);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("plan (nothing sent yet)"), "{stdout}");
    assert!(
        stdout.contains("confirmed writes currently fail closed"),
        "{stdout}"
    );
}

#[test]
fn invalid_address_or_confirmation_is_rejected_before_recovery_gate() {
    for (address, confirmation) in [("1.1.300", CONFIRM), ("1.1.67", "wrong phrase")] {
        let output = Command::new(env!("CARGO_BIN_EXE_knx"))
            .args([
                "device",
                "reset-address",
                address,
                "--gateway",
                "127.0.0.1:3671",
                "--confirm",
                confirmation,
            ])
            .output()
            .unwrap();
        assert!(!output.status.success());
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(!stderr.contains("durable pre-write backup"), "{stderr}");
    }
}
