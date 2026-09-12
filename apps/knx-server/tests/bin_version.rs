//! Drives the built `knx-server` binary to check what `--version` says about itself.

use std::process::Command;

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_knx-server"))
        .args(args)
        .output()
        .expect("failed to run the knx-server binary")
}

#[test]
fn version_flag_prints_and_exits_without_binding_a_port() {
    let output = run(&["--version"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let line = stdout.trim_end();
    let expected = format!("knx-server {}", env!("CARGO_PKG_VERSION"));
    let rest = line
        .strip_prefix(&expected)
        .unwrap_or_else(|| panic!("expected {expected:?} at the start of {line:?}"));
    if !rest.is_empty() {
        let sha = rest
            .strip_prefix("+g")
            .unwrap_or_else(|| panic!("expected `+g<sha>` build metadata, got {rest:?}"));
        assert!(
            sha.len() >= 7 && sha.chars().all(|c| c.is_ascii_hexdigit()),
            "build metadata is not an abbreviated commit hash: {sha:?}"
        );
    }
    // The library-side formatter and the binary agree, so `knx-desktop`
    // (which links the library) can report the same thing if it ever wants to.
    assert_eq!(line, knx_server::version_line());
}

#[test]
fn short_version_flag_says_the_same_thing() {
    assert_eq!(run(&["-V"]).stdout, run(&["--version"]).stdout);
}

#[test]
fn manifest_version_is_a_pre_release_while_the_project_is_alpha() {
    // Same guard `apps/knx-cli/tests/cli_version.rs` keeps on `knx`: the
    // version string is the one honest statement about maturity the
    // binary makes about itself, and it must not lose its pre-release
    // identifier before anyone means it to.
    let version = env!("CARGO_PKG_VERSION");
    assert!(
        version.contains("-alpha.") || version.contains("-beta.") || version.contains("-rc."),
        "expected a SemVer pre-release, got {version:?}"
    );
}
