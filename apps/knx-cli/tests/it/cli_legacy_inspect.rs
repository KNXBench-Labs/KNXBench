//! `knx products inspect-legacy`: reads a legacy EX-IM file without touching any database.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

const PASSWORD: &str = "marvin-synthetic";

fn fixture(name: &str) -> PathBuf {
    knx_testsupport::workspace_root()
        .join("crates/knx-productdb/fixtures/legacy")
        .join(name)
}

/// Runs the binary with an isolated data/config home so a stray default
/// product database would be visible in `home`.
fn run(args: &[&str], stdin: Option<&str>, home: &std::path::Path) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_knx"));
    command
        .args(args)
        .env("HOME", home)
        .env("XDG_DATA_HOME", home.join("data"))
        .env("XDG_CONFIG_HOME", home.join("config"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().expect("failed to run the knx binary");
    if let Some(input) = stdin {
        child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(input.as_bytes())
            .unwrap();
    }
    drop(child.stdin.take());
    child.wait_with_output().unwrap()
}

fn text(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

fn files_under(dir: &std::path::Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(next) = stack.pop() {
        for entry in std::fs::read_dir(&next).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else {
                found.push(path);
            }
        }
    }
    found
}

#[test]
fn the_password_on_stdin_opens_the_file_and_prints_its_contents() {
    let home = tempfile::tempdir().unwrap();
    let file = fixture("marvin-encrypted.vd4");
    let out = run(
        &[
            "products",
            "inspect-legacy",
            file.to_str().unwrap(),
            "--password-stdin",
        ],
        Some(&format!("{PASSWORD}\n")),
        home.path(),
    );
    let all = text(&out);
    assert_eq!(out.status.code(), Some(0), "{all}");
    for expected in [
        "legacy ETS3 product database",
        "format version 6.2",
        "7 tables, 10 rows",
        "MT-42-B",
        "Heart of Gold Sensor b\u{e9}ta",
        "Improbability Drive",
        "MV-0701",
        "windows-1252 (assumed)",
        "not imported",
    ] {
        assert!(all.contains(expected), "missing {expected:?} in:\n{all}");
    }
    assert!(!all.contains(PASSWORD));
    assert!(files_under(home.path()).is_empty(), "inspect created files");
}

#[test]
fn a_password_file_works_too() {
    let home = tempfile::tempdir().unwrap();
    let secret = home.path().join("secret.txt");
    std::fs::write(&secret, format!("{PASSWORD}\n")).unwrap();
    let file = fixture("marvin-encrypted.vd4");
    let out = run(
        &[
            "products",
            "inspect-legacy",
            file.to_str().unwrap(),
            "--password-file",
            secret.to_str().unwrap(),
        ],
        None,
        home.path(),
    );
    let all = text(&out);
    assert_eq!(out.status.code(), Some(0), "{all}");
    assert!(all.contains("MT-42"), "{all}");
    assert!(!all.contains(PASSWORD));
}

#[test]
fn a_wrong_password_fails_without_repeating_it() {
    let home = tempfile::tempdir().unwrap();
    let file = fixture("marvin-encrypted.vd4");
    let out = run(
        &[
            "products",
            "inspect-legacy",
            file.to_str().unwrap(),
            "--password-stdin",
        ],
        Some("canary-wrong-password\n"),
        home.path(),
    );
    let all = text(&out);
    assert_ne!(out.status.code(), Some(0));
    assert!(all.contains("wrong password"), "{all}");
    assert!(!all.contains("canary-wrong-password"));
}

#[test]
fn a_protected_file_without_a_password_names_both_options() {
    let home = tempfile::tempdir().unwrap();
    let file = fixture("marvin-encrypted.vd4");
    let out = run(
        &["products", "inspect-legacy", file.to_str().unwrap()],
        None,
        home.path(),
    );
    let all = text(&out);
    assert_ne!(out.status.code(), Some(0));
    assert!(all.contains("--password-stdin"), "{all}");
    assert!(all.contains("--password-file"), "{all}");
}

#[test]
fn a_password_on_the_command_line_is_refused_and_not_echoed() {
    let home = tempfile::tempdir().unwrap();
    let file = fixture("marvin-encrypted.vd4");
    for args in [
        vec!["--password", "canary-argv-password"],
        vec!["--password=canary-argv-password"],
    ] {
        let mut full = vec!["products", "inspect-legacy", file.to_str().unwrap()];
        full.extend(args);
        let out = run(&full, None, home.path());
        let all = text(&out);
        assert_ne!(out.status.code(), Some(0));
        assert!(!all.contains("canary-argv-password"), "{all}");
        assert!(all.contains("never accepted on the command line"), "{all}");
    }
}

#[test]
fn an_empty_password_on_stdin_is_refused() {
    let home = tempfile::tempdir().unwrap();
    let file = fixture("marvin-encrypted.vd4");
    let out = run(
        &[
            "products",
            "inspect-legacy",
            file.to_str().unwrap(),
            "--password-stdin",
        ],
        Some("\n"),
        home.path(),
    );
    assert_ne!(out.status.code(), Some(0));
    assert!(text(&out).contains("empty password"), "{}", text(&out));
}

#[test]
fn an_unprotected_file_needs_no_password_and_a_project_export_is_named() {
    let home = tempfile::tempdir().unwrap();
    let plain = fixture("marvin-plain.vd4");
    let out = run(
        &["products", "inspect-legacy", plain.to_str().unwrap()],
        None,
        home.path(),
    );
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    assert!(text(&out).contains("not encrypted"), "{}", text(&out));

    let project = fixture("marvin-project.pr5");
    let out = run(
        &[
            "products",
            "inspect-legacy",
            project.to_str().unwrap(),
            "--password-stdin",
        ],
        Some(&format!("{PASSWORD}\n")),
        home.path(),
    );
    let all = text(&out);
    assert_eq!(out.status.code(), Some(0), "{all}");
    assert!(all.contains("legacy ETS3 project export"), "{all}");
}

#[test]
fn a_file_that_is_not_a_legacy_container_is_refused_by_name() {
    let home = tempfile::tempdir().unwrap();
    let other = home.path().join("random.vd4");
    std::fs::write(&other, b"definitely not a zip").unwrap();
    let out = run(
        &["products", "inspect-legacy", other.to_str().unwrap()],
        None,
        home.path(),
    );
    assert_ne!(out.status.code(), Some(0));
    assert!(
        text(&out).contains("not a legacy EX-IM container"),
        "{}",
        text(&out)
    );
}

#[test]
fn an_endless_password_file_is_read_only_up_to_a_bound() {
    let home = tempfile::tempdir().unwrap();
    let file = fixture("marvin-encrypted.vd4");
    // /dev/zero never ends; an unbounded read would hang. Its first "line"
    // is 4 KiB of NUL bytes, which is simply a wrong password.
    let out = run(
        &[
            "products",
            "inspect-legacy",
            file.to_str().unwrap(),
            "--password-file",
            "/dev/zero",
        ],
        None,
        home.path(),
    );
    assert_ne!(out.status.code(), Some(0));
    assert!(text(&out).contains("wrong password"), "{}", text(&out));
}
