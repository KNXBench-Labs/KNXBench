//! Exercises repository gates against runtime targets rather than their build tree.

use std::process::Command;
use std::{fs, path::Path};

fn write(root: &Path, relative: &str, source: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, source).unwrap();
}

fn workspace() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    write(
        root.path(),
        "Cargo.toml",
        "[workspace]\nmembers = [\"xtask\", \"crates/knx-core\"]\nresolver = \"2\"\n",
    );
    for (name, path) in [("xtask", "xtask"), ("knx-core", "crates/knx-core")] {
        write(
            root.path(),
            &format!("{path}/Cargo.toml"),
            &format!("[package]\nname = \"{name}\"\nversion = \"0.0.0\"\nedition = \"2021\"\n"),
        );
        write(
            root.path(),
            &format!("{path}/src/lib.rs"),
            "//! A fixture.\n",
        );
    }
    write(root.path(), "apps/demo/src/lib.rs", "//! An app.\n");
    write(root.path(), "docs/guide.md", "# Guide\n[here](#guide)\n");
    root
}

fn run(root: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(args)
        .current_dir(root)
        .output()
        .unwrap()
}

fn layering_workspace() -> tempfile::TempDir {
    let root = workspace();
    write(root.path(), "Cargo.toml", "[workspace]\nmembers = [\"xtask\", \"crates/*\"]\nexclude = [\"outside/knx-projection\", \"outside/rusqlite\"]\nresolver = \"2\"\n");
    for name in [
        "knx-etsproj",
        "knx-productdb",
        "knx-projection",
        "knx-csv",
        "knx-report",
        "knx-diff",
        "knx-secure",
    ] {
        write(
            root.path(),
            &format!("crates/{name}/Cargo.toml"),
            &format!("[package]\nname = \"{name}\"\nversion = \"0.0.0\"\nedition = \"2021\"\n"),
        );
        write(
            root.path(),
            &format!("crates/{name}/src/lib.rs"),
            "//! A fixture.\n",
        );
    }
    root
}

#[test]
fn layering_requires_policy_roots_to_be_workspace_members() {
    let root = layering_workspace();
    fs::create_dir(root.path().join("outside")).unwrap();
    fs::rename(
        root.path().join("crates/knx-projection"),
        root.path().join("outside/knx-projection"),
    )
    .unwrap();
    write(root.path(), "crates/knx-core/Cargo.toml", "[package]\nname = \"knx-core\"\nversion = \"0.0.0\"\nedition = \"2021\"\n[dependencies]\nknx-projection = { path = \"../../outside/knx-projection\" }\n");
    let metadata = cargo_metadata::MetadataCommand::new()
        .manifest_path(root.path().join("Cargo.toml"))
        .exec()
        .unwrap();
    let projection = metadata
        .packages
        .iter()
        .find(|package| package.name == "knx-projection")
        .unwrap();
    assert!(
        !metadata.workspace_members.contains(&projection.id),
        "fixture dependency must really be external to the workspace"
    );
    let output = run(root.path(), &["check-layering"]);
    assert!(
        !output.status.success(),
        "a transitive package must not replace a checked workspace root"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("missing layering root knx-projection")
    );
}

#[test]
fn layering_checks_a_complete_runtime_fixture_graph() {
    let root = layering_workspace();
    let cwd = tempfile::tempdir().unwrap();
    let output = run(
        cwd.path(),
        &["--root", root.path().to_str().unwrap(), "check-layering"],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("layering scope: 9 resolved packages"));
    write(
        root.path(),
        "outside/rusqlite/Cargo.toml",
        "[package]\nname = \"rusqlite\"\nversion = \"0.0.0\"\n",
    );
    write(root.path(), "outside/rusqlite/src/lib.rs", "");
    write(root.path(), "crates/knx-core/Cargo.toml", "[package]\nname = \"knx-core\"\nversion = \"0.0.0\"\nedition = \"2021\"\n[dependencies]\nrusqlite = { path = \"../../outside/rusqlite\" }\n");
    let output = run(
        cwd.path(),
        &["--root", root.path().to_str().unwrap(), "check-layering"],
    );
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("layering violation: knx-core reaches forbidden package rusqlite"));
}

#[test]
fn headers_checks_the_selected_runtime_fixture() {
    let root = workspace();
    write(root.path(), "apps/demo/src/lib.rs", "//! Missing period\n");
    let output = run(root.path(), &["check-headers"]);
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("header violation: apps/demo/src/lib.rs")
    );
}

#[test]
fn explicit_root_selects_a_valid_fixture_from_an_unrelated_directory() {
    let root = workspace();
    let cwd = tempfile::tempdir().unwrap();
    for (gate, coverage) in [
        ("check-headers", "3 files with a well-formed header"),
        ("check-anchors", "1 links checked across 1 markdown files"),
        ("check-corpus-gates", "2 Rust files scanned"),
    ] {
        let output = run(cwd.path(), &["--root", root.path().to_str().unwrap(), gate]);
        assert!(
            output.status.success(),
            "{gate}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains(&format!(
                "gate target: {}",
                root.path().canonicalize().unwrap().display()
            )),
            "{stdout}"
        );
        assert!(stdout.contains(coverage), "{gate}: {stdout}");
    }
}

#[test]
fn gates_reject_deleted_explicit_targets() {
    let root = workspace();
    let path = root.path().to_path_buf();
    root.close().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    for gate in [
        "check-headers",
        "check-anchors",
        "check-corpus-gates",
        "check-layering",
        "check-appimage",
    ] {
        let output = run(cwd.path(), &["--root", path.to_str().unwrap(), gate]);
        assert!(!output.status.success(), "{gate} accepted a deleted target");
        assert!(String::from_utf8_lossy(&output.stderr).contains("is unavailable"));
    }
}

#[test]
fn gates_reject_a_workspace_member_instead_of_climbing_to_its_parent() {
    let root = workspace();
    let output = run(&root.path().join("xtask"), &["check-headers"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("is not the workspace root"));
}

#[test]
fn gates_reject_an_unrelated_rust_workspace() {
    let root = tempfile::tempdir().unwrap();
    write(
        root.path(),
        "Cargo.toml",
        "[workspace]\nmembers = [\"other\"]\n",
    );
    write(
        root.path(),
        "other/Cargo.toml",
        "[package]\nname = \"other\"\nversion = \"0.0.0\"\n",
    );
    write(root.path(), "other/src/lib.rs", "");
    let output = run(root.path(), &["check-headers"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("lacks xtask/Cargo.toml"));
}

#[test]
fn expected_directories_do_not_substitute_for_the_named_workspace_members() {
    let root = workspace();
    write(
        root.path(),
        "crates/knx-core/Cargo.toml",
        "[package]\nname = \"not-knx-core\"\nversion = \"0.0.0\"\nedition = \"2021\"\n",
    );
    let output = run(root.path(), &["check-headers"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("lacks workspace member knx-core"));
}

#[test]
fn headers_rejects_generated_only_coverage() {
    let root = workspace();
    fs::remove_dir_all(root.path().join("apps")).unwrap();
    write(
        root.path(),
        "apps/knx-web/src/bindings/Foo.ts",
        "// generated\n",
    );
    let output = run(root.path(), &["check-headers"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("empty header scan"));
}

#[test]
fn layering_refuses_missing_policy_roots() {
    let root = workspace();
    let output = run(root.path(), &["check-layering"]);
    assert!(
        !output.status.success(),
        "missing checked crates must not pass: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("missing layering root"));
}

#[test]
fn misplaced_root_option_is_not_silently_ignored() {
    let root = workspace();
    let output = run(root.path(), &["check-headers", "--root", "not-a-target"]);
    assert!(
        !output.status.success(),
        "a misplaced target option must fail, not audit the cwd"
    );
}

#[test]
fn headers_rejects_an_unrelated_runtime_directory() {
    let unrelated = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .arg("check-headers")
        .current_dir(unrelated.path())
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "an unrelated runtime directory must not audit the baked-in source tree: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}
