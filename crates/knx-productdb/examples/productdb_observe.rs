//! Opt-in Linux target observer; never use the outer Python wrapper's RSS.
//!
//! `productdb_observe --receipt ABS_NEW_FILE -- ABS_EXECUTABLE ARGS...`
//! The caller owns the receipt directory and excludes replacement races. The
//! outer observer owns session/group cancellation; this helper never detaches.
//! Linux `wait4` reports absolute target peak RSS in KiB, not an allocation delta.
#[cfg(not(target_os = "linux"))]
compile_error!("productdb_observe requires Linux wait4 RSS semantics");

use std::{
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::{self, Seek, SeekFrom, Write},
    mem::MaybeUninit,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
    process::{Child, Command, Stdio},
    time::Instant,
};

use serde_json::{json, Value};

type Reaped = (libc::pid_t, libc::c_int, libc::rusage);

// `Child` does not kill/reap on drop. Keep ownership until wait4 really reaps it,
// including checkpoint and wait failures. Never kill a PID after a valid reap.
struct OwnedChild(Child, bool);
impl Drop for OwnedChild {
    fn drop(&mut self) {
        if self.1 {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}

fn write_receipt(file: &mut File, value: &Value) -> io::Result<()> {
    let bytes = serde_json::to_vec(value)?;
    file.seek(SeekFrom::Start(0))?;
    file.set_len(0)?;
    file.write_all(&bytes)?;
    file.write_all(b"\n")?;
    file.flush()?;
    file.sync_data()
}

fn wait_once(pid: libc::pid_t) -> io::Result<Reaped> {
    let mut status = 0;
    let mut usage = MaybeUninit::<libc::rusage>::uninit();
    // SAFETY: `pid` is our live, positive child PID; both writable pointers refer
    // to correctly aligned, live storage. libc supplies the platform ABI. With
    // options=0, a positive return initializes both outputs; no other thread or
    // Child::wait competes for this child in this single-threaded executable.
    let reaped = unsafe { libc::wait4(pid, &mut status, 0, usage.as_mut_ptr()) };
    if reaped < 0 {
        return Err(io::Error::last_os_error());
    }
    if reaped != pid {
        return Err(io::Error::other("wait4 did not reap the exact target PID"));
    }
    // SAFETY: the successful blocking wait4 above initialized `usage`.
    Ok((reaped, status, unsafe { usage.assume_init() }))
}

fn cpu_time(value: libc::timeval) -> io::Result<f64> {
    if value.tv_sec < 0 || !(0..1_000_000).contains(&value.tv_usec) {
        return Err(io::Error::other("invalid target CPU resource observation"));
    }
    Ok(value.tv_sec as f64 + value.tv_usec as f64 / 1_000_000.0)
}

fn observe(receipt: &Path, executable: &Path, args: &[OsString]) -> io::Result<i32> {
    observe_with(receipt, executable, args, wait_once, write_receipt)
}

// Private seams permit real child cleanup tests with injected EINTR, wait/proof
// and disk-write failures. The CLI has no fault-injection mode or environment.
fn observe_with(
    receipt: &Path,
    executable: &Path,
    args: &[OsString],
    mut wait: impl FnMut(libc::pid_t) -> io::Result<Reaped>,
    mut write: impl FnMut(&mut File, &Value) -> io::Result<()>,
) -> io::Result<i32> {
    if !receipt.is_absolute() || !executable.is_absolute() {
        return Err(io::Error::other(
            "receipt and executable must be absolute paths",
        ));
    }
    // Atomic reservation rejects existing regular files, directories, FIFOs and
    // both live/dangling symlinks. No existing file or sidecar is overwritten.
    let mut checkpoint = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(receipt)?;
    let reserved = checkpoint.metadata()?;
    if !reserved.is_file() {
        return Err(io::Error::other("receipt must be a new regular file"));
    }
    write(
        &mut checkpoint,
        &json!({"format":1,"complete":false,"pid":null}),
    )?;
    let started = Instant::now();
    let child = Command::new(executable)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()?;
    let mut owned = OwnedChild(child, true);
    let pid = libc::pid_t::try_from(owned.0.id())
        .map_err(|_| io::Error::other("target PID is out of range"))?;
    write(
        &mut checkpoint,
        &json!({"format":1,"complete":false,"pid":pid}),
    )?;
    let (reaped, status, usage) = loop {
        match wait(pid) {
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            other => break other?,
        }
    };
    if reaped != pid {
        return Err(io::Error::other("wait4 did not reap the exact target PID"));
    }
    owned.1 = false;
    let elapsed = started.elapsed().as_secs_f64();
    let (exit, signal, helper_exit) = if libc::WIFEXITED(status) {
        let exit = libc::WEXITSTATUS(status);
        (exit, None, exit)
    } else if libc::WIFSIGNALED(status) {
        let signal = libc::WTERMSIG(status);
        (-signal, Some(signal), 128 + signal)
    } else {
        return Err(io::Error::other(
            "wait4 did not return a terminal target status",
        ));
    };
    let cpu = cpu_time(usage.ru_utime)? + cpu_time(usage.ru_stime)?;
    if usage.ru_maxrss <= 0 || !cpu.is_finite() || !elapsed.is_finite() {
        return Err(io::Error::other(
            "missing or invalid target resource observation",
        ));
    }
    let complete = json!({
        "format":1,"complete":true,"pid":pid,"waitStatus":status,
        "exit":exit,"signal":signal,"peakRssKiB":usage.ru_maxrss,
        "cpuSeconds":cpu,"elapsedSeconds":elapsed,
    });
    // Prepare completion separately: a short write/flush failure must leave the
    // flushed incomplete checkpoint, not a partly overwritten success receipt.
    // tempfile is an existing dev dependency; it atomically reserves a new,
    // private sibling and removes it on failure. Never use a fixed .tmp sidecar.
    let parent = receipt
        .parent()
        .ok_or_else(|| io::Error::other("missing receipt parent"))?;
    let mut final_file = tempfile::NamedTempFile::new_in(parent)?;
    write(final_file.as_file_mut(), &complete)?;
    let current = fs::symlink_metadata(receipt)?;
    if !current.is_file() || current.dev() != reserved.dev() || current.ino() != reserved.ino() {
        return Err(io::Error::other(
            "receipt reservation changed during observation",
        ));
    }
    // This replaces only our own checkpoint in the exclusively owned directory,
    // never a pre-existing caller file. Completed JSON becomes visible at once.
    final_file.persist(receipt).map_err(|error| error.error)?;
    Ok(helper_exit)
}

fn cli(args: &[OsString]) -> io::Result<i32> {
    if args.len() < 4 || args[0] != "--receipt" || args[2] != "--" {
        return Err(io::Error::other(
            "usage: productdb_observe --receipt ABS_NEW_FILE -- ABS_EXECUTABLE ARGS...",
        ));
    }
    observe(Path::new(&args[1]), Path::new(&args[3]), &args[4..])
}

fn main() {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let exit = match cli(&args) {
        Ok(exit) => exit,
        Err(_) => {
            // Do not echo private paths or target arguments into outer logs.
            eprintln!("native productdb observation failed; require a complete target receipt");
            125
        }
    };
    std::process::exit(exit);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn short_success_has_exact_reap_receipt() {
        let root = tempfile::tempdir().unwrap();
        let receipt = root.path().join("receipt.json");
        let result = observe(&receipt, Path::new("/usr/bin/true"), &[]);
        assert_eq!(result.ok(), Some(0), "short target must be observed");
        let value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(receipt).unwrap()).unwrap();
        assert_eq!(value["format"], 1);
        assert_eq!(value["complete"], true);
        assert_eq!(value["exit"], 0);
        assert_eq!(value["signal"], serde_json::Value::Null);
        assert_eq!(value["waitStatus"], 0);
        assert!(value["pid"].as_u64().unwrap() > 0);
        assert!(value["peakRssKiB"].as_i64().unwrap() > 0);
        assert!(value["cpuSeconds"].as_f64().unwrap() >= 0.0);
        assert!(value["elapsedSeconds"].as_f64().unwrap() >= 0.0);
        assert_eq!(value.as_object().unwrap().len(), 9);
    }

    fn read(path: &Path) -> Value {
        serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
    }

    fn assert_incomplete(path: &Path, pid: Option<libc::pid_t>) {
        assert_eq!(read(path), json!({"format":1,"complete":false,"pid":pid}));
    }

    fn assert_no_child(pid: libc::pid_t) {
        assert!(
            !Path::new(&format!("/proc/{pid}")).exists(),
            "target must be reaped"
        );
    }

    #[test]
    fn ordinary_nonzero_exit_is_raw_observation() {
        let root = tempfile::tempdir().unwrap();
        let receipt = root.path().join("receipt.json");
        let args = [OsString::from("-c"), OsString::from("exit 7")];
        assert_eq!(
            observe(&receipt, Path::new("/usr/bin/sh"), &args).unwrap(),
            7
        );
        let value = read(&receipt);
        assert_eq!(value["complete"], true);
        assert_eq!(value["exit"], 7);
        assert_eq!(value["waitStatus"], 7 << 8);
        assert_eq!(value["signal"], Value::Null);
    }

    #[test]
    fn signal_is_negative_target_exit_and_128_plus_signal_helper_exit() {
        let root = tempfile::tempdir().unwrap();
        let receipt = root.path().join("receipt.json");
        let args = [OsString::from("-c"), OsString::from("kill -TERM $$")];
        assert_eq!(
            observe(&receipt, Path::new("/usr/bin/sh"), &args).unwrap(),
            143
        );
        let value = read(&receipt);
        assert_eq!(value["complete"], true);
        assert_eq!(value["exit"], -15);
        assert_eq!(value["signal"], 15);
        assert_eq!(value["waitStatus"], 15);
    }

    #[test]
    fn spawn_missing_keeps_incomplete_null_pid_without_metrics() {
        let root = tempfile::tempdir().unwrap();
        let receipt = root.path().join("receipt.json");
        assert!(observe(&receipt, &root.path().join("missing"), &[]).is_err());
        assert_incomplete(&receipt, None);
    }

    #[test]
    fn relative_paths_and_bad_cli_do_not_reserve_receipt() {
        let root = tempfile::tempdir().unwrap();
        let receipt = root.path().join("receipt.json");
        assert!(observe(&receipt, Path::new("true"), &[]).is_err());
        assert!(!receipt.exists());
        assert!(observe(Path::new("relative.json"), Path::new("/usr/bin/true"), &[]).is_err());
        assert!(cli(&[]).is_err());
        assert!(cli(&[
            "--receipt".into(),
            receipt.into_os_string(),
            "bad".into(),
            "/usr/bin/true".into()
        ])
        .is_err());
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
    }

    #[test]
    fn existing_receipt_and_symlinks_never_spawn_or_overwrite() {
        use std::os::unix::fs::symlink;
        let root = tempfile::tempdir().unwrap();
        let original = root.path().join("original");
        fs::write(&original, b"unchanged").unwrap();
        let link = root.path().join("link");
        let dangling = root.path().join("dangling");
        symlink(&original, &link).unwrap();
        symlink(root.path().join("absent"), &dangling).unwrap();
        let marker = root.path().join("must-not-exist");
        for receipt in [&original, &link, &dangling, &root.path().to_path_buf()] {
            assert!(observe(
                receipt,
                Path::new("/usr/bin/touch"),
                &[marker.clone().into_os_string()]
            )
            .is_err());
            assert!(!marker.exists());
        }
        assert_eq!(fs::read(&original).unwrap(), b"unchanged");
        assert!(fs::symlink_metadata(link).unwrap().file_type().is_symlink());
        assert!(fs::symlink_metadata(dangling)
            .unwrap()
            .file_type()
            .is_symlink());
    }

    #[test]
    fn existing_fixed_sidecar_is_untouched_and_no_temporary_siblings_remain() {
        let root = tempfile::tempdir().unwrap();
        let receipt = root.path().join("receipt.json");
        let sidecar = root.path().join("receipt.json.tmp");
        fs::write(&sidecar, b"not-owned").unwrap();
        assert_eq!(
            observe(&receipt, Path::new("/usr/bin/true"), &[]).unwrap(),
            0
        );
        assert_eq!(fs::read(sidecar).unwrap(), b"not-owned");
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 2);
        assert_eq!(
            fs::metadata(receipt).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }

    #[test]
    fn interrupted_wait_retries_exact_child_without_reaping_unrelated_child() {
        let root = tempfile::tempdir().unwrap();
        let receipt = root.path().join("receipt.json");
        let mut unrelated = OwnedChild(
            Command::new("/usr/bin/sleep").arg("30").spawn().unwrap(),
            true,
        );
        let mut attempts = 0;
        let result = observe_with(
            &receipt,
            Path::new("/usr/bin/true"),
            &[],
            |pid| {
                attempts += 1;
                if attempts == 1 {
                    Err(io::Error::from(io::ErrorKind::Interrupted))
                } else {
                    wait_once(pid)
                }
            },
            write_receipt,
        );
        assert_eq!(result.unwrap(), 0);
        assert_eq!(attempts, 2);
        assert_ne!(read(&receipt)["pid"], unrelated.0.id());
        assert!(unrelated.0.try_wait().unwrap().is_none());
    }

    #[test]
    fn wait_error_kills_and_reaps_target_leaving_checkpoint() {
        let root = tempfile::tempdir().unwrap();
        let receipt = root.path().join("receipt.json");
        let mut target_pid = 0;
        let result = observe_with(
            &receipt,
            Path::new("/usr/bin/sleep"),
            &["30".into()],
            |pid| {
                target_pid = pid;
                Err(io::Error::from_raw_os_error(libc::ECHILD))
            },
            write_receipt,
        );
        assert!(result.is_err());
        assert!(target_pid > 0);
        assert_incomplete(&receipt, Some(target_pid));
        assert_no_child(target_pid);
    }

    #[test]
    fn pid_checkpoint_write_failure_kills_and_reaps_target() {
        let root = tempfile::tempdir().unwrap();
        let receipt = root.path().join("receipt.json");
        let mut target_pid = 0;
        let result = observe_with(
            &receipt,
            Path::new("/usr/bin/sleep"),
            &["30".into()],
            wait_once,
            |file, value| {
                if let Some(pid) = value["pid"].as_i64() {
                    target_pid = pid as libc::pid_t;
                    Err(io::Error::from_raw_os_error(libc::ENOSPC))
                } else {
                    write_receipt(file, value)
                }
            },
        );
        assert!(result.is_err());
        assert!(target_pid > 0);
        assert_incomplete(&receipt, None);
        assert_no_child(target_pid);
    }

    #[test]
    fn final_partial_write_failure_leaves_valid_incomplete_checkpoint() {
        let root = tempfile::tempdir().unwrap();
        let receipt = root.path().join("receipt.json");
        let result = observe_with(
            &receipt,
            Path::new("/usr/bin/true"),
            &[],
            wait_once,
            |file, value| {
                if value["complete"] == true {
                    file.write_all(b"{\"complete\":true,")?;
                    Err(io::Error::from_raw_os_error(libc::ENOSPC))
                } else {
                    write_receipt(file, value)
                }
            },
        );
        assert!(result.is_err());
        let pid = read(&receipt)["pid"].as_i64().unwrap() as libc::pid_t;
        assert_incomplete(&receipt, Some(pid));
        assert_no_child(pid);
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
    }

    #[test]
    fn zero_rss_proof_failure_never_publishes_fake_metrics() {
        let root = tempfile::tempdir().unwrap();
        let receipt = root.path().join("receipt.json");
        let result = observe_with(
            &receipt,
            Path::new("/usr/bin/true"),
            &[],
            |pid| {
                let (reaped, status, mut usage) = wait_once(pid)?;
                usage.ru_maxrss = 0;
                Ok((reaped, status, usage))
            },
            write_receipt,
        );
        assert!(result.is_err());
        let pid = read(&receipt)["pid"].as_i64().unwrap() as libc::pid_t;
        assert_incomplete(&receipt, Some(pid));
        assert_no_child(pid);
    }

    #[test]
    fn invalid_cpu_proof_never_publishes_metrics() {
        let root = tempfile::tempdir().unwrap();
        let receipt = root.path().join("receipt.json");
        let result = observe_with(
            &receipt,
            Path::new("/usr/bin/true"),
            &[],
            |pid| {
                let (reaped, status, mut usage) = wait_once(pid)?;
                usage.ru_utime.tv_usec = 1_000_000;
                Ok((reaped, status, usage))
            },
            write_receipt,
        );
        assert!(result.is_err());
        let pid = read(&receipt)["pid"].as_i64().unwrap() as libc::pid_t;
        assert_incomplete(&receipt, Some(pid));
        assert_no_child(pid);
    }
}
