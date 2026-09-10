//! Drives the built `knx` binary to test the export subcommand.
//! Tests read a `.knxdb` file (via import) and round-trip it back out
//! to a `.knxproj` file.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn reference_ets4_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .join("OriginalData/DemoProjects/Unser Zuhause ets4 - 2025-12-15.knxproj")
}

fn run_cli(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_knx"))
        .args(args)
        .output()
        .expect("failed to run the knx binary")
}

#[test]
fn export_after_import_succeeds_and_warns_about_unsigned() {
    if !reference_ets4_path().exists() {
        eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("project.knxdb");
    let output = dir.path().join("exported.knxproj");

    // Step 1: Import the reference project into a .knxdb.
    let import_out = run_cli(&[
        "import",
        reference_ets4_path().to_str().unwrap(),
        "--store",
        store.to_str().unwrap(),
        "--no-product-db",
    ]);
    assert_eq!(
        import_out.status.code(),
        Some(0),
        "import failed: {}",
        String::from_utf8_lossy(&import_out.stderr)
    );

    // Step 2: Export the project back out.
    let export_out = run_cli(&[
        "export",
        store.to_str().unwrap(),
        output.to_str().unwrap(),
        "--no-product-db",
    ]);
    assert_eq!(
        export_out.status.code(),
        Some(0),
        "export failed: {}",
        String::from_utf8_lossy(&export_out.stderr)
    );

    // Step 3: Verify the output file exists and is non-empty.
    assert!(
        output.exists(),
        "exported file does not exist at {}",
        output.display()
    );
    let size = std::fs::metadata(&output)
        .expect("failed to get metadata for exported file")
        .len();
    assert!(size > 0, "exported file is empty");

    // Step 4: Verify stderr contains the unsigned-export warning.
    let stderr = String::from_utf8(export_out.stderr).unwrap();
    assert!(
        stderr.contains("Unsigned"),
        "stderr did not mention unsigned warning; stderr: {}",
        stderr
    );
}

#[test]
fn export_against_nonexistent_store_fails() {
    if !reference_ets4_path().exists() {
        eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("exported.knxproj");

    let out = run_cli(&[
        "export",
        "/nonexistent/store.knxdb",
        output.to_str().unwrap(),
        "--no-product-db",
    ]);
    assert_ne!(
        out.status.code(),
        Some(0),
        "export should have failed but succeeded"
    );
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(!stderr.is_empty(), "stderr should be non-empty");
}
