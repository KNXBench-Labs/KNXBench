//! Public CLI individual-address programming needs durable recovery first.
use std::net::UdpSocket;
use std::process::Command;
use std::time::Duration;

const CONFIRM: &str = "I confirm individual-address programming to 1.1.30";

#[test]
fn confirmed_programming_refuses_without_durable_backup_before_any_socket() {
    let listener = UdpSocket::bind("127.0.0.1:0").unwrap();
    listener
        .set_read_timeout(Some(Duration::from_millis(200)))
        .unwrap();
    let gateway = listener.local_addr().unwrap().to_string();
    let output = Command::new(env!("CARGO_BIN_EXE_knx"))
        .args([
            "device",
            "program-address",
            "1.1.30",
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
fn programming_plan_remains_available_without_advertising_a_working_write() {
    let output = Command::new(env!("CARGO_BIN_EXE_knx"))
        .args(["device", "program-address", "1.1.30"])
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
fn invalid_address_or_phrase_fails_before_backup_gate() {
    for (address, confirmation) in [("1.1.300", CONFIRM), ("1.1.30", "wrong phrase")] {
        let output = Command::new(env!("CARGO_BIN_EXE_knx"))
            .args([
                "device",
                "program-address",
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
