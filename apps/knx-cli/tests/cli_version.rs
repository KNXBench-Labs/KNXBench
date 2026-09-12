//! Drives the built `knx` binary to check what `--version` says about itself.

use std::process::Command;

fn run_cli(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_knx"))
        .args(args)
        .output()
        .expect("failed to run the knx binary")
}

/// Asserts `line` is `knx <manifest version>` with, optionally, SemVer
/// build metadata `+g<hex>` naming the commit the binary was built from.
/// The metadata is optional because a source tarball or a Docker build has
/// no git to ask; the manifest version is not.
fn assert_version_line(line: &str) {
    let expected = format!("knx {}", env!("CARGO_PKG_VERSION"));
    let rest = line
        .strip_prefix(&expected)
        .unwrap_or_else(|| panic!("expected {expected:?} at the start of {line:?}"));
    if rest.is_empty() {
        return;
    }
    let sha = rest
        .strip_prefix("+g")
        .unwrap_or_else(|| panic!("expected `+g<sha>` build metadata, got {rest:?}"));
    assert!(
        sha.len() >= 7 && sha.chars().all(|c| c.is_ascii_hexdigit()),
        "build metadata is not an abbreviated commit hash: {sha:?}"
    );
}

#[test]
fn version_flag_prints_name_version_and_build_metadata() {
    let output = run_cli(&["--version"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_version_line(stdout.trim_end());
    assert!(output.stderr.is_empty());
}

#[test]
fn short_version_flag_says_the_same_thing() {
    let long = run_cli(&["--version"]);
    let short = run_cli(&["-V"]);
    assert!(short.status.success());
    assert_eq!(short.stdout, long.stdout);
}

#[test]
fn manifest_version_is_a_pre_release_while_the_project_is_alpha() {
    // The version string is the one honest statement about maturity the
    // binary makes about itself. `0.0.0` is not one, and neither is a
    // bare `0.1.0` before anything has been released.
    let version = env!("CARGO_PKG_VERSION");
    assert!(
        version.contains("-alpha.") || version.contains("-beta.") || version.contains("-rc."),
        "expected a SemVer pre-release, got {version:?}"
    );
}
