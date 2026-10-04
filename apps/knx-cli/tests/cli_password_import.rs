//! `knx import` of a password-protected project: the password comes on stdin, never argv (AR08).

use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

use knx_testsupport::{write_zipcrypto_minimal_knxproj, ZIPCRYPTO_MINIMAL_PASSWORD};

fn run(args: &[&str], stdin: Option<&str>) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_knx"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to run the knx binary");
    {
        let mut pipe = child.stdin.take().unwrap();
        if let Some(text) = stdin {
            pipe.write_all(text.as_bytes()).unwrap();
        }
    }
    child.wait_with_output().unwrap()
}

fn text(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

fn import_args<'a>(source: &'a Path, store: &'a Path) -> Vec<&'a str> {
    vec![
        "import",
        source.to_str().unwrap(),
        "--store",
        store.to_str().unwrap(),
        "--no-product-db",
    ]
}

#[test]
fn the_password_on_stdin_imports_the_project_and_is_never_echoed() {
    let dir = tempfile::tempdir().unwrap();
    let source = write_zipcrypto_minimal_knxproj(dir.path());
    let store = dir.path().join("out.knxdb");
    let mut args = import_args(&source, &store);
    args.push("--password-stdin");
    let out = run(&args, Some(&format!("{ZIPCRYPTO_MINIMAL_PASSWORD}\n")));
    let all = text(&out);
    assert_eq!(out.status.code(), Some(0), "{all}");
    assert!(
        all.contains("1 devices") || all.contains("1 device"),
        "{all}"
    );
    assert!(!all.contains(ZIPCRYPTO_MINIMAL_PASSWORD));
    let saved = std::fs::read(&store).unwrap();
    assert!(!saved
        .windows(ZIPCRYPTO_MINIMAL_PASSWORD.len())
        .any(|w| w == ZIPCRYPTO_MINIMAL_PASSWORD.as_bytes()));
}

#[test]
fn a_wrong_password_fails_without_repeating_it() {
    let dir = tempfile::tempdir().unwrap();
    let source = write_zipcrypto_minimal_knxproj(dir.path());
    let store = dir.path().join("out.knxdb");
    let mut args = import_args(&source, &store);
    args.push("--password-stdin");
    let out = run(&args, Some("canary-wrong-password\n"));
    let all = text(&out);
    assert_ne!(out.status.code(), Some(0));
    assert!(all.contains("wrong password"), "{all}");
    assert!(!all.contains("canary-wrong-password"));
}

#[test]
fn a_protected_project_without_a_password_names_the_flag() {
    let dir = tempfile::tempdir().unwrap();
    let source = write_zipcrypto_minimal_knxproj(dir.path());
    let store = dir.path().join("out.knxdb");
    let out = run(&import_args(&source, &store), None);
    let all = text(&out);
    assert_ne!(out.status.code(), Some(0));
    assert!(all.contains("password-protected"), "{all}");
    assert!(all.contains("--password-stdin"), "{all}");
}

#[test]
fn a_password_on_the_command_line_is_refused_and_not_echoed() {
    let dir = tempfile::tempdir().unwrap();
    let source = write_zipcrypto_minimal_knxproj(dir.path());
    let store = dir.path().join("out.knxdb");
    let mut args = import_args(&source, &store);
    args.extend(["--password", "canary-argv-password"]);
    let out = run(&args, None);
    let all = text(&out);
    assert_ne!(out.status.code(), Some(0));
    assert!(all.contains("--password-stdin"), "{all}");
    assert!(!all.contains("canary-argv-password"), "{all}");
    assert!(!store.exists(), "a refused command line opens no store");
}

#[test]
fn an_empty_password_on_stdin_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let source = write_zipcrypto_minimal_knxproj(dir.path());
    let store = dir.path().join("out.knxdb");
    let mut args = import_args(&source, &store);
    args.push("--password-stdin");
    let out = run(&args, Some("\n"));
    let all = text(&out);
    assert_ne!(out.status.code(), Some(0));
    assert!(all.contains("empty password"), "{all}");
}
