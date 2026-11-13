//! Drive the real `knx` binary through optional TCP tunnelling against loopback gateways.

mod diagnostics;
mod find_serial;
mod probe;
mod scan;

use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream, UdpSocket},
    process::{Child, Command, Output, Stdio},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

const CHANNEL: u8 = 0x2a;
/// CONNECT_REQUEST for a TCP tunnel: Route Back control and data HPAIs, link layer CRI.
const TCP_CONNECT_REQUEST: [u8; 26] = [
    0x06, 0x10, 0x02, 0x05, 0x00, 0x1a, 0x08, 0x02, 0, 0, 0, 0, 0, 0, 0x08, 0x02, 0, 0, 0, 0, 0, 0,
    0x04, 0x04, 0x02, 0x00,
];
/// CONNECT_RESPONSE: channel 0x2a, no error, Route Back data HPAI, CRD for IA 1.1.250.
const TCP_CONNECT_RESPONSE: [u8; 20] = [
    0x06, 0x10, 0x02, 0x06, 0x00, 0x14, CHANNEL, 0x00, 0x08, 0x02, 0, 0, 0, 0, 0, 0, 0x04, 0x04,
    0x11, 0xfa,
];
/// L_Data.ind 1.1.1 -> 0/0/1, GroupValueWrite 1, as a TUNNELLING_REQUEST with sequence 7.
const TCP_TELEGRAM: [u8; 21] = [
    0x06, 0x10, 0x04, 0x20, 0x00, 0x15, 0x04, CHANNEL, 0x07, 0x00, 0x29, 0x00, 0xbc, 0xe0, 0x11,
    0x01, 0x00, 0x01, 0x01, 0x00, 0x81,
];
const DISCONNECT_RESPONSE: [u8; 8] = [0x06, 0x10, 0x02, 0x0a, 0x00, 0x08, CHANNEL, 0x00];
const WAIT: Duration = Duration::from_secs(10);

fn knx() -> Command {
    Command::new(env!("CARGO_BIN_EXE_knx"))
}
fn run(args: &[&str]) -> Output {
    knx().args(args).output().unwrap()
}
fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn read_frame(stream: &mut TcpStream) -> Vec<u8> {
    let mut header = [0u8; 6];
    stream.read_exact(&mut header).unwrap();
    let total = u16::from_be_bytes([header[4], header[5]]) as usize;
    let mut frame = header.to_vec();
    frame.resize(total, 0);
    stream.read_exact(&mut frame[6..]).unwrap();
    frame
}

/// Accepts one connection and plays the handshake; `rest` plays the tunnel.
fn tcp_gateway(rest: impl FnOnce(&mut TcpStream) + Send + 'static) -> (SocketAddr, JoinHandle<()>) {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let address = listener.local_addr().unwrap();
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream.set_read_timeout(Some(WAIT)).unwrap();
        assert_eq!(read_frame(&mut stream), TCP_CONNECT_REQUEST);
        stream.write_all(&TCP_CONNECT_RESPONSE).unwrap();
        rest(&mut stream);
    });
    (address, handle)
}

/// The client's orderly end: DISCONNECT_REQUEST, our response, then the close.
fn expect_disconnect_and_close(stream: &mut TcpStream) {
    let disconnect = read_frame(stream);
    assert_eq!(&disconnect[2..4], [0x02, 0x09], "DISCONNECT_REQUEST");
    assert_eq!(disconnect[6], CHANNEL);
    // The client may close as soon as it has sent its request, so answering can fail.
    let _ = stream.write_all(&DISCONNECT_RESPONSE);
    let mut rest = Vec::new();
    if stream.read_to_end(&mut rest).is_ok() {
        assert!(rest.is_empty(), "nothing may follow the disconnect");
    }
}

/// A port nothing listens on, with a UDP socket on the same number to catch a fallback.
fn refused_tcp_with_udp_trap() -> (u16, UdpSocket) {
    let port = {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        listener.local_addr().unwrap().port()
    };
    let trap = UdpSocket::bind((Ipv4Addr::LOCALHOST, port)).unwrap();
    trap.set_read_timeout(Some(Duration::from_millis(400)))
        .unwrap();
    (port, trap)
}
fn assert_no_datagram(trap: &UdpSocket) {
    let mut buffer = [0u8; 64];
    assert!(
        trap.recv_from(&mut buffer).is_err(),
        "a UDP datagram followed an explicit TCP request"
    );
}

#[test]
fn invalid_duplicate_and_serial_combinations_fail_before_any_connection() {
    let dir = tempfile::tempdir().unwrap();
    let serial = dir.path().join("absent-serial");
    let serial = serial.to_str().unwrap();
    // Anything that connected would hit this listener.
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    listener.set_nonblocking(true).unwrap();
    let gateway = listener.local_addr().unwrap().to_string();
    let cases: Vec<(Vec<&str>, &str)> = vec![
        (
            vec!["--gateway", &gateway, "--tunnel-transport", "tls"],
            "must be udp or tcp",
        ),
        (
            vec!["--gateway", &gateway, "--tunnel-transport", "TCP"],
            "must be udp or tcp",
        ),
        (
            vec!["--gateway", &gateway, "--tunnel-transport", ""],
            "must be udp or tcp",
        ),
        (
            vec!["--gateway", &gateway, "--tunnel-transport"],
            "needs a value",
        ),
        (
            vec![
                "--gateway",
                &gateway,
                "--tunnel-transport",
                "tcp",
                "--tunnel-transport",
                "tcp",
            ],
            "duplicate --tunnel-transport",
        ),
        (
            vec![
                "--gateway",
                &gateway,
                "--tunnel-transport",
                "tcp",
                "--tunnel-transport",
                "udp",
            ],
            "duplicate --tunnel-transport",
        ),
        (
            vec![
                "--serial",
                serial,
                "--source",
                "1.1.1",
                "--tunnel-transport",
                "tcp",
            ],
            "only valid with --gateway",
        ),
        (
            vec![
                "--serial",
                serial,
                "--source",
                "1.1.1",
                "--tunnel-transport",
                "udp",
            ],
            "only valid with --gateway",
        ),
        (
            vec!["--tunnel-transport", "tcp"],
            "exactly one of --gateway",
        ),
        (
            vec!["--gateway", "0.0.0.0:3671", "--tunnel-transport", "tcp"],
            "unicast",
        ),
        (
            vec!["--gateway", "192.0.2.1:0", "--tunnel-transport", "tcp"],
            "nonzero port",
        ),
    ];
    for command in ["monitor", "write"] {
        for (flags, expected) in &cases {
            let mut args = vec!["bus", command];
            if command == "write" {
                args.extend(["1/1/1", "1"]);
            }
            // Keep a missing final option value missing, not a consumed positional address.
            args.extend(flags.iter().copied());
            let output = run(&args);
            let error = stderr(&output);
            assert!(!output.status.success(), "{args:?}");
            assert!(error.contains(expected), "{args:?}: {error}");
            assert!(!error.contains("could not connect"), "{args:?}: {error}");
            assert!(!error.contains("panicked"), "{args:?}");
        }
    }
    assert!(!std::path::Path::new(serial).exists());
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock,
        "a rejected command connected anyway"
    );
}

#[test]
fn help_names_the_option_its_default_and_its_scope() {
    // A command with no endpoint is refused and prints the usage text.
    let output = run(&["bus", "monitor"]);
    assert!(!output.status.success());
    let text = stderr(&output);
    assert!(text.contains("--tunnel-transport <udp|tcp>"), "{text}");
    assert!(text.contains("defaults to udp"), "{text}");
    assert!(text.contains("never falls back to udp"), "{text}");
    assert!(text.contains("TCP socket accepted the frame"), "{text}");
}

#[test]
fn tcp_dry_run_encodes_without_opening_anything() {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    listener.set_nonblocking(true).unwrap();
    let gateway = listener.local_addr().unwrap().to_string();
    let output = run(&[
        "bus",
        "write",
        "--gateway",
        &gateway,
        "--tunnel-transport",
        "tcp",
        "--dry-run",
        "1/1/1",
        "1",
    ]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stdout(&output).contains("0x01"));
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}

#[test]
fn write_over_tcp_sends_the_exact_handshake_and_says_the_socket_accepted_it() {
    let (address, gateway) = tcp_gateway(|stream| {
        let request = read_frame(stream);
        assert_eq!(
            &request[..4],
            [0x06, 0x10, 0x04, 0x20],
            "TUNNELLING_REQUEST"
        );
        assert_eq!(request[6], 0x04, "connection header length");
        assert_eq!(request[7], CHANNEL);
        assert_eq!(request[10], 0x11, "L_Data.req");
        // 1/1/1 is 0x0901 on the wire: a group write of the 6-bit value 1.
        assert_eq!(
            &request[request.len() - 5..],
            [0x09, 0x01, 0x01, 0x00, 0x81]
        );
        // TCP has no TUNNELLING_ACK; the CLI must not wait for or expect one.
        expect_disconnect_and_close(stream);
    });
    let output = run(&[
        "bus",
        "write",
        "--gateway",
        &address.to_string(),
        "--tunnel-transport",
        "tcp",
        "1/1/1",
        "1",
    ]);
    gateway.join().expect("the gateway script failed");
    assert!(output.status.success(), "{}", stderr(&output));
    let out = stdout(&output);
    assert_eq!(
        out,
        "sent to 1/1/1 over TCP; the TCP socket accepted it, device delivery is not confirmed\n"
    );
    assert!(!out.contains("wrote to"));
}

/// What one interrupted `bus monitor --tunnel-transport tcp` run produced.
struct MonitorRun {
    address: SocketAddr,
    status: std::process::ExitStatus,
    stderr: String,
    first_stdout_line: String,
}

/// Runs the monitor against a gateway that sends `frames` after the handshake. The interrupt is
/// sent only after the CLI has printed its ready line, which it does after registering its
/// signal handler, and after the first telegram line reached stdout.
fn monitor_session(frames: Vec<Vec<u8>>) -> MonitorRun {
    let (seen_tx, seen_rx) = std::sync::mpsc::channel::<()>();
    let (address, gateway) = tcp_gateway(move |stream| {
        for frame in &frames {
            stream.write_all(frame).unwrap();
        }
        // Wait until the CLI printed the telegram, then it is interrupted and disconnects.
        seen_rx
            .recv_timeout(WAIT)
            .expect("the CLI never printed the telegram");
        expect_disconnect_and_close(stream);
    });
    let mut child = KillOnDrop(
        knx()
            .args([
                "bus",
                "monitor",
                "--gateway",
                &address.to_string(),
                "--tunnel-transport",
                "tcp",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let stderr_pipe = child.0.stderr.take().unwrap();
    let (line_tx, line_rx) = std::sync::mpsc::channel::<String>();
    let reader = thread::spawn(move || {
        for line in BufReader::new(stderr_pipe).lines().map_while(Result::ok) {
            let _ = line_tx.send(line);
        }
    });
    let mut stderr_lines = Vec::new();
    loop {
        let line = line_rx
            .recv_timeout(WAIT)
            .expect("the CLI never announced that it is ready");
        let ready = line.contains("Ctrl-C to stop");
        stderr_lines.push(line);
        if ready {
            break;
        }
    }
    let mut lines = BufReader::new(child.0.stdout.take().unwrap()).lines();
    let first_stdout_line = lines.next().expect("no stdout line").unwrap();
    seen_tx.send(()).unwrap();
    // The CLI's own Ctrl-C path, via the signal the terminal would send.
    assert!(Command::new("kill")
        .args(["-INT", &child.0.id().to_string()])
        .status()
        .unwrap()
        .success());
    let status = wait_for(&mut child.0);
    gateway.join().expect("the gateway script failed");
    reader.join().unwrap();
    stderr_lines.extend(line_rx.try_iter());
    MonitorRun {
        address,
        status,
        stderr: stderr_lines.join("\n"),
        first_stdout_line,
    }
}

#[test]
fn monitor_over_tcp_prints_telegrams_says_tcp_and_closes_orderly_on_interrupt() {
    let run = monitor_session(vec![TCP_TELEGRAM.to_vec()]);
    assert!(
        run.first_stdout_line
            .starts_with("1.1.1 -> 0/0/1: GroupValueWrite"),
        "{}",
        run.first_stdout_line
    );
    assert!(run.first_stdout_line.contains("0x01"));
    assert!(run.status.success(), "{}", run.stderr);
    assert!(
        run.stderr.contains(&format!(
            "connected to {} over TCP, assigned individual address 1.1.250",
            run.address
        )),
        "{}",
        run.stderr
    );
    assert!(run.stderr.contains("disconnecting..."), "{}", run.stderr);
    // Negative control: nothing was skipped, so no skip line appears.
    assert!(!run.stderr.contains("skipped"), "{}", run.stderr);
}

#[test]
fn monitor_over_tcp_reports_skipped_frames_once_and_still_prints_the_next_telegram() {
    // An unknown service, a TUNNELLING_REQUEST above any plausible size, then a valid telegram.
    let mut unknown = vec![0x06, 0x10, 0x09, 0x99, 0x00, 0x10];
    unknown.extend_from_slice(&[0xee; 10]);
    let mut oversize = vec![0x06, 0x10, 0x04, 0x20];
    oversize.extend_from_slice(&1500u16.to_be_bytes());
    oversize.resize(1500, 0x00);
    let run = monitor_session(vec![unknown, oversize, TCP_TELEGRAM.to_vec()]);
    assert!(
        run.first_stdout_line
            .starts_with("1.1.1 -> 0/0/1: GroupValueWrite"),
        "{}",
        run.first_stdout_line
    );
    assert!(run.status.success(), "{}", run.stderr);
    let summary: Vec<&str> = run
        .stderr
        .lines()
        .filter(|line| line.contains("skipped"))
        .collect();
    assert_eq!(summary.len(), 1, "{}", run.stderr);
    assert!(
        summary[0].starts_with("warning: 2 TCP frame(s) skipped before disconnecting"),
        "{}",
        summary[0]
    );
    // The summary comes before the disconnect cleanup ran, and says what it counted.
    assert!(
        run.stderr.find("skipped").unwrap() > run.stderr.find("disconnecting...").unwrap(),
        "{}",
        run.stderr
    );
    assert!(!summary[0].contains("dropped") && !summary[0].contains("lost"));
}

#[test]
fn monitor_over_tcp_reports_the_actual_protocol_closure_instead_of_blame() {
    let (ready_tx, ready_rx) = std::sync::mpsc::channel();
    let (address, gateway) = tcp_gateway(move |stream| {
        ready_rx
            .recv_timeout(WAIT)
            .expect("the CLI never completed its handshake");
        stream
            .write_all(&[0x06, 0x20, 0x04, 0x20, 0x00, 0x06])
            .unwrap();
        let mut probe = [0u8; 1];
        match stream.read(&mut probe) {
            Ok(0) => {}
            Err(error) if error.kind() == std::io::ErrorKind::ConnectionReset => {}
            other => panic!("the malformed stream must be released: {other:?}"),
        }
    });
    let mut child = KillOnDrop(
        knx()
            .args([
                "bus",
                "monitor",
                "--gateway",
                &address.to_string(),
                "--tunnel-transport",
                "tcp",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let mut lines = BufReader::new(child.0.stderr.take().unwrap()).lines();
    let mut stderr = String::new();
    let mut ready = false;
    for line in lines.by_ref() {
        let line = line.unwrap();
        stderr.push_str(&line);
        stderr.push('\n');
        if line.contains("Ctrl-C to stop") {
            ready_tx.send(()).unwrap();
            ready = true;
            break;
        }
    }
    assert!(
        ready,
        "the CLI must report ready before the established-stream fault: {stderr}"
    );
    let status = wait_for(&mut child.0);
    for line in lines {
        stderr.push_str(&line.unwrap());
        stderr.push('\n');
    }
    gateway
        .join()
        .expect("the malformed-stream gateway script failed");
    assert!(status.success(), "{stderr}");
    assert!(stderr.contains("TCP tunnel closed"), "{stderr}");
    assert!(stderr.contains("protocol version 0x20"), "{stderr}");
    assert!(!stderr.contains("gateway closed"), "{stderr}");
}

#[test]
fn refused_tcp_is_an_error_and_never_falls_back_to_udp() {
    let (port, trap) = refused_tcp_with_udp_trap();
    let gateway = format!("127.0.0.1:{port}");
    for args in [
        vec![
            "bus",
            "write",
            "--gateway",
            &gateway,
            "--tunnel-transport",
            "tcp",
            "1/1/1",
            "1",
        ],
        vec![
            "bus",
            "monitor",
            "--gateway",
            &gateway,
            "--tunnel-transport",
            "tcp",
        ],
    ] {
        let output = run(&args);
        assert!(!output.status.success(), "{args:?}");
        let error = stderr(&output);
        assert!(
            error.contains(&format!("could not connect to {gateway}")),
            "{error}"
        );
        assert!(stdout(&output).is_empty(), "{args:?}");
        assert_no_datagram(&trap);
    }
}

#[test]
fn default_and_explicit_udp_still_open_a_udp_tunnel_and_never_touch_tcp() {
    for extra in [vec![], vec!["--tunnel-transport", "udp"]] {
        let gateway = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        gateway.set_read_timeout(Some(WAIT)).unwrap();
        let address = gateway.local_addr().unwrap();
        // Anything TCP would be refused: the same port has no TCP listener.
        let script = thread::spawn(move || {
            let mut buffer = [0u8; 128];
            let (length, client) = gateway.recv_from(&mut buffer).unwrap();
            let frame = &buffer[..length];
            assert_eq!(&frame[2..4], [0x02, 0x05], "CONNECT_REQUEST");
            assert_eq!(frame[7], 0x01, "control HPAI names UDP");
            assert_eq!(frame[15], 0x01, "data HPAI names UDP");
            // Refuse (E_NO_MORE_CONNECTIONS) so the command ends at once.
            gateway
                .send_to(&[0x06, 0x10, 0x02, 0x06, 0x00, 0x08, 0x00, 0x24], client)
                .unwrap();
        });
        let mut args = vec!["bus", "write", "--gateway"];
        let address = address.to_string();
        args.push(&address);
        args.extend(extra.iter().copied());
        args.extend(["1/1/1", "1"]);
        let output = run(&args);
        script.join().expect("the UDP script failed");
        assert!(!output.status.success(), "{args:?}");
        assert!(stderr(&output).contains("could not connect to"), "{args:?}");
    }
}

struct KillOnDrop(Child);
impl Drop for KillOnDrop {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn wait_for(child: &mut Child) -> std::process::ExitStatus {
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            return status;
        }
        assert!(
            start.elapsed() < WAIT,
            "the CLI did not exit after the interrupt"
        );
        thread::sleep(Duration::from_millis(20));
    }
}
