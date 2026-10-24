//! Cargo invoked inside the checkout runs with scratch knx homes by default.
//!
//! `.cargo/config.toml` forces `XDG_DATA_HOME`/`XDG_CONFIG_HOME` below
//! `target/dev-home/`, because desktop sessions and CI runner images export
//! the real homes. Only an explicit `--config env.<VAR>.force=false` lets an
//! exported value through (the package gate does so for its empty scratch
//! homes). That opt-out makes the exported locations the caller's
//! responsibility. Cargo discovers the config from its invocation directory,
//! not from `--manifest-path`, so a command started outside the checkout also
//! remains the caller's responsibility. A spawned `knx` once migrated the
//! owner's product database.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// `(variable, real fallback below $HOME, leaf below target/dev-home)`.
const HOMES: [(&str, &str, &str); 2] = [
    ("XDG_DATA_HOME", ".local/share", "data"),
    ("XDG_CONFIG_HOME", ".config", "config"),
];
const UNSET_TOOL_HOMES_PROBE: &str = "KNXBENCH_UNSET_TOOL_HOMES_PROBE";

fn workspace_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("apps/knx-cli sits two levels below the workspace root")
}

fn home_value(variable: &str) -> PathBuf {
    let value = std::env::var_os(variable)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| {
            panic!(
                "{variable} is unset: `.cargo/config.toml` [env] did not reach this test, \
                 so the apps would fall back to the real home"
            )
        });
    PathBuf::from(value)
}

/// The same directory, also when one spelling goes through a symlink.
fn same_directory(a: &Path, b: &Path) -> bool {
    a == b
        || matches!(
            (a.canonicalize(), b.canonicalize()),
            (Ok(a), Ok(b)) if a == b
        )
}

fn cargo() -> PathBuf {
    std::env::var_os("CARGO")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("cargo"))
}

fn write_environment_probe(root: &Path) -> PathBuf {
    let package = root.join("probe");
    fs::create_dir_all(package.join("src")).unwrap();
    fs::write(
        package.join("Cargo.toml"),
        "[package]\nname = \"cargo-dev-home-probe\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[workspace]\n",
    )
    .unwrap();
    fs::write(
        package.join("src/lib.rs"),
        r#"#[test]
fn cargo_passes_the_expected_homes() {
    for variable in [
        "XDG_DATA_HOME",
        "XDG_CONFIG_HOME",
        "CARGO_HOME",
        "RUSTUP_HOME",
    ] {
        let actual = std::env::var_os(variable).expect("probed home is set");
        let expected = std::env::var_os(format!("EXPECTED_{variable}"))
            .expect("expected home sentinel is set");
        assert_eq!(actual, expected, "unexpected {variable}");
    }
}
"#,
    )
    .unwrap();
    package.join("Cargo.toml")
}

fn checkout_probe() -> tempfile::TempDir {
    let target = workspace_root().join("target");
    fs::create_dir_all(&target).unwrap();
    tempfile::Builder::new()
        .prefix("cargo-dev-home-probe-")
        .tempdir_in(target)
        .unwrap()
}

fn original_tool_home(variable: &str, original_home: &Path, fallback: &str) -> PathBuf {
    std::env::var_os(variable)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| original_home.join(fallback))
}

fn run_environment_probe(
    cwd: &Path,
    manifest: &Path,
    scratch: &Path,
    data_home: &Path,
    config_home: &Path,
    cargo_arguments: &[&str],
) {
    let original_home = std::env::var_os("HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .expect("HOME is set for the Cargo environment probe");
    let cargo_home = original_tool_home("CARGO_HOME", &original_home, ".cargo");
    let rustup_home = original_tool_home("RUSTUP_HOME", &original_home, ".rustup");
    let home = scratch.join("home");
    fs::create_dir_all(&home).unwrap();
    let output = Command::new(cargo())
        .args(cargo_arguments)
        .args(["test", "--offline", "--manifest-path"])
        .arg(manifest)
        .current_dir(cwd)
        .env("HOME", home)
        .env("CARGO_HOME", &cargo_home)
        .env("RUSTUP_HOME", &rustup_home)
        .env("XDG_DATA_HOME", data_home)
        .env("XDG_CONFIG_HOME", config_home)
        .env("EXPECTED_XDG_DATA_HOME", data_home)
        .env("EXPECTED_XDG_CONFIG_HOME", config_home)
        .env("EXPECTED_CARGO_HOME", cargo_home)
        .env("EXPECTED_RUSTUP_HOME", rustup_home)
        .env("CARGO_TARGET_DIR", scratch.join("target"))
        .output()
        .expect("failed to run the Cargo environment probe");
    assert!(
        output.status.success(),
        "Cargo environment probe failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn cargo_test_runs_with_scratch_knx_homes() {
    let real_home = std::env::var_os("HOME").map(PathBuf::from);
    for (variable, fallback, leaf) in HOMES {
        let value = home_value(variable);
        assert!(
            value.is_absolute(),
            "{variable}={} is relative",
            value.display()
        );
        if let Some(home) = &real_home {
            let real = home.join(fallback);
            assert!(
                !same_directory(&value, &real),
                "{variable} is the real {}; tests must not run against real user data",
                real.display()
            );
        }
        let default = workspace_root().join("target").join("dev-home").join(leaf);
        if !same_directory(&value, &default) {
            // Let through on purpose (`--config env.<VAR>.force=false`). The
            // caller remains responsible for choosing an isolated location.
            eprintln!("{variable} chosen by the caller: {}", value.display());
        }
    }
}

#[test]
fn manifest_path_from_external_cwd_bypasses_checkout_config() {
    let probe = checkout_probe();
    let scratch = tempfile::tempdir().unwrap();
    let manifest = write_environment_probe(probe.path());
    let external_cwd = scratch.path().join("external-cwd");
    let data_home = scratch.path().join("exported-data");
    let config_home = scratch.path().join("exported-config");
    fs::create_dir_all(&external_cwd).unwrap();

    run_environment_probe(
        &external_cwd,
        &manifest,
        scratch.path(),
        &data_home,
        &config_home,
        &[],
    );
}

#[test]
fn force_false_accepts_caller_supplied_scratch_homes() {
    let probe = checkout_probe();
    let scratch = tempfile::tempdir().unwrap();
    let manifest = write_environment_probe(probe.path());
    let data_home = scratch.path().join("exported-data");
    let config_home = scratch.path().join("exported-config");
    let cargo_arguments = [
        "--config",
        "env.XDG_DATA_HOME.force=false",
        "--config",
        "env.XDG_CONFIG_HOME.force=false",
    ];

    run_environment_probe(
        workspace_root(),
        &manifest,
        scratch.path(),
        &data_home,
        &config_home,
        &cargo_arguments,
    );
}

#[test]
fn nested_cargo_uses_original_tool_homes_when_parent_vars_are_unset() {
    if std::env::var_os(UNSET_TOOL_HOMES_PROBE).is_some() {
        let probe = checkout_probe();
        let scratch = tempfile::tempdir().unwrap();
        let manifest = write_environment_probe(probe.path());
        let external_cwd = scratch.path().join("external-cwd");
        let data_home = scratch.path().join("exported-data");
        let config_home = scratch.path().join("exported-config");
        fs::create_dir_all(&external_cwd).unwrap();
        run_environment_probe(
            &external_cwd,
            &manifest,
            scratch.path(),
            &data_home,
            &config_home,
            &[],
        );
        return;
    }

    let output = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "cargo_dev_homes::nested_cargo_uses_original_tool_homes_when_parent_vars_are_unset",
            "--nocapture",
        ])
        .env(UNSET_TOOL_HOMES_PROBE, "1")
        .env_remove("CARGO_HOME")
        .env_remove("RUSTUP_HOME")
        .output()
        .expect("failed to run the unset-tool-homes probe");
    assert!(
        output.status.success(),
        "unset-tool-homes probe failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn a_spawned_knx_binary_inherits_the_scratch_config_home() {
    // Checked first, so a missing default fails here instead of letting the
    // child look at the real config directory.
    let config_home = home_value("XDG_CONFIG_HOME");
    let output = Command::new(env!("CARGO_BIN_EXE_knx"))
        .args(["products", "legacy-password", "status"])
        .output()
        .expect("failed to run the knx binary");
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).unwrap();
    let expected = config_home.join("knx").join("legacy-vd-password");
    assert!(
        stdout.contains(&*expected.to_string_lossy()),
        "the child resolved another config home: {stdout}"
    );
}
