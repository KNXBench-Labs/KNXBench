//! A plain `cargo test` runs with scratch knx homes, never the developer's real ones.
//!
//! `.cargo/config.toml` forces `XDG_DATA_HOME`/`XDG_CONFIG_HOME` below
//! `target/dev-home/`, because desktop sessions and CI runner images export
//! the real homes. Only an explicit `--config env.<VAR>.force=false` lets an
//! exported value through (the package gate does so for its empty scratch
//! homes). Either way the homes a test, and every binary it spawns, sees must
//! not be the real `~/.local/share` or `~/.config`: a spawned `knx` once
//! migrated the owner's product database.

use std::path::{Path, PathBuf};
use std::process::Command;

/// `(variable, real fallback below $HOME, leaf below target/dev-home)`.
const HOMES: [(&str, &str, &str); 2] = [
    ("XDG_DATA_HOME", ".local/share", "data"),
    ("XDG_CONFIG_HOME", ".config", "config"),
];

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
            // Let through on purpose (`--config env.<VAR>.force=false`), and
            // still not the real home.
            eprintln!("{variable} chosen by the caller: {}", value.display());
        }
    }
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
