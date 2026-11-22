//! Keeps every Cargo package and the web frontend on the one workspace version (ADR-0190).

use std::fs;
use std::path::{Path, PathBuf};

use cargo_metadata::MetadataCommand;
use serde_json::Value;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask lives one level below the workspace root")
        .to_path_buf()
}

/// Reads `[workspace.package] version` from the root manifest's text.
fn workspace_version(root: &Path) -> String {
    let manifest = fs::read_to_string(root.join("Cargo.toml")).unwrap();
    let section = manifest
        .split("[workspace.package]")
        .nth(1)
        .expect("root Cargo.toml has a [workspace.package] table");
    let section = section.split("\n[").next().unwrap();
    section
        .lines()
        .find_map(|line| line.strip_prefix("version = \""))
        .and_then(|rest| rest.strip_suffix('"'))
        .expect("[workspace.package] declares version = \"...\"")
        .to_string()
}

/// Accepts `version.workspace = true` with any spacing and an optional trailing comment.
fn inherits_workspace_version(line: &str) -> bool {
    let code = line.split('#').next().unwrap_or_default();
    let compact: String = code.chars().filter(|c| !c.is_whitespace()).collect();
    compact == "version.workspace=true" || compact == "version={workspace=true}"
}

#[test]
fn every_cargo_package_inherits_the_workspace_version() {
    let root = repo_root();
    let expected = workspace_version(&root);
    let metadata = MetadataCommand::new()
        .manifest_path(root.join("Cargo.toml"))
        .no_deps()
        .exec()
        .unwrap();
    let members = metadata.workspace_packages();
    assert!(members.len() >= 2, "workspace members not found");
    for package in members {
        assert_eq!(
            package.version.to_string(),
            expected,
            "{} must use the workspace version",
            package.name
        );
        let manifest = fs::read_to_string(&package.manifest_path).unwrap();
        assert!(
            manifest.lines().any(inherits_workspace_version),
            "{} must declare `version.workspace = true`, not its own number",
            package.manifest_path
        );
    }
}

#[test]
fn web_frontend_carries_the_workspace_version() {
    let root = repo_root();
    let expected = workspace_version(&root);
    let read = |relative: &str| -> Value {
        serde_json::from_str(&fs::read_to_string(root.join(relative)).unwrap()).unwrap()
    };
    let package = read("apps/knx-web/package.json");
    let lock = read("apps/knx-web/package-lock.json");
    for (place, value) in [
        ("package.json version", &package["version"]),
        ("package-lock.json version", &lock["version"]),
        (
            "package-lock.json packages[\"\"].version",
            &lock["packages"][""]["version"],
        ),
    ] {
        assert_eq!(value.as_str(), Some(expected.as_str()), "{place}");
    }
}
