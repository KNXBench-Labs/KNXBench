//! Exercise the ordinary read-only byte-order probe through the built CLI and local sockets.

mod admission;
pub(super) mod fixture;
mod outcomes;
pub(super) mod peer;
mod recording;

use super::{stderr, stdout, KillOnDrop};
use std::{
    io::Read,
    process::{Command, Output, Stdio},
    thread,
    time::{Duration, Instant},
};

const WAIT: Duration = Duration::from_secs(35);

pub(super) struct Joined<T>(Option<thread::JoinHandle<T>>);
impl<T> Joined<T> {
    pub(super) fn new(handle: thread::JoinHandle<T>) -> Self {
        Self(Some(handle))
    }
    pub(super) fn join(mut self) -> thread::Result<T> {
        self.0.take().unwrap().join()
    }
}
impl<T> Drop for Joined<T> {
    fn drop(&mut self) {
        if let Some(handle) = self.0.take() {
            let _ = handle.join();
        }
    }
}

/// Drain both pipes while waiting; even a failing assertion kills and waits for the child.
pub(super) fn collect(cmd: &mut Command) -> Output {
    let argv = std::iter::once(cmd.get_program())
        .chain(cmd.get_args())
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let mut child = KillOnDrop(
        cmd.stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let mut out = child.0.stdout.take().unwrap();
    let mut err = child.0.stderr.take().unwrap();
    let stdout = thread::spawn(move || {
        let mut bytes = Vec::new();
        out.read_to_end(&mut bytes).unwrap();
        bytes
    });
    let stderr = thread::spawn(move || {
        let mut bytes = Vec::new();
        err.read_to_end(&mut bytes).unwrap();
        bytes
    });
    let deadline = Instant::now() + WAIT;
    let timed_out = loop {
        if child.0.try_wait().unwrap().is_some() {
            break false;
        }
        if Instant::now() >= deadline {
            child.0.kill().unwrap();
            break true;
        }
        thread::sleep(Duration::from_millis(10));
    };
    let output = Output {
        status: child.0.wait().unwrap(),
        stdout: stdout.join().unwrap(),
        stderr: stderr.join().unwrap(),
    };
    eprintln!(
        "probe-cli-receipt {}",
        serde_json::json!({"argv": argv, "exit": output.status.code(), "status": output.status.to_string(), "stdout": String::from_utf8_lossy(&output.stdout), "stderr": String::from_utf8_lossy(&output.stderr)})
    );
    assert!(
        !timed_out,
        "probe did not settle: {}\n{}",
        self::stdout(&output),
        self::stderr(&output)
    );
    output
}

fn failed(output: &Output, expected: &str) {
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}\n{}",
        stdout(output),
        stderr(output)
    );
    assert!(
        format!("{}{}", stdout(output), stderr(output)).contains(expected),
        "missing {expected}: {}\n{}",
        stdout(output),
        stderr(output)
    );
    assert!(
        !stdout(output).contains("verdict:"),
        "no invented tally after refusal"
    );
}

fn result(output: &Output, counts: &str, controls: &str, transport: &str) {
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}\n{}",
        stdout(output),
        stderr(output)
    );
    let out = stdout(output);
    for expected in [
        counts,
        controls,
        "written to the device: no (read only)",
        "Z_R: its value reads the same in both orders",
        "T_R: a text, not a number",
        "5 octets in 2 runs",
    ] {
        assert!(out.contains(expected), "missing {expected}: {out}");
    }
    assert!(
        stderr(output).contains(&format!("over {transport}, assigned individual address")),
        "{}",
        stderr(output)
    );
}
