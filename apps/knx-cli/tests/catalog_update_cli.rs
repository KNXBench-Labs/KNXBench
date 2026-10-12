//! `knx products update|restore`: previewed, token-confirmed catalogue updates (ADR-0117).
//!
//! Synthetic manufacturer files only; no network, bus or private corpus.

use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;
use zip::write::SimpleFileOptions;

const MASTER: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/20"><MasterData><Manufacturers><Manufacturer Id="M-0001" Name="Example"/></Manufacturers></MasterData></KNX>"#;

fn package(dir: &Path, version: u8) -> PathBuf {
    let program = format!(
        r#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0001"><ApplicationPrograms><ApplicationProgram Id="M-0001_A-0001-{version:02X}-0001" Name="Dimmer" ApplicationNumber="1" ApplicationVersion="{version}" ProgramType="ApplicationProgram" MaskVersion="MV-07B0" PeiType="0" LoadProcedureStyle="MergedProcedure" DefaultLanguage="en-US"><Static/></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#
    );
    let hardware = format!(
        r#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0001"><Hardware><Hardware Id="M-0001_H-1" Name="Dimmer hardware"><Products><Product Id="M-0001_H-1_P-1" Text="Dimmer" OrderNumber="DIM-1"/></Products><Hardware2Programs><Hardware2Program Id="M-0001_H-1_HP-0001-{version:02X}-0001" MediumTypes="MT-0"><ApplicationProgramRef RefId="M-0001_A-0001-{version:02X}-0001"/></Hardware2Program></Hardware2Programs></Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#
    );
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in [
        ("knx_master.xml", MASTER),
        ("M-0001/M-0001_A-0001.xml", program.as_bytes()),
        ("M-0001/Hardware.xml", hardware.as_bytes()),
    ] {
        zip.start_file(name, SimpleFileOptions::default()).unwrap();
        zip.write_all(bytes).unwrap();
    }
    let path = dir.join(format!("dimmer-v{version}.knxprod"));
    std::fs::write(&path, zip.finish().unwrap().into_inner()).unwrap();
    path
}

fn knx(args: &[&str], home: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_knx"))
        .args(args)
        .env("XDG_DATA_HOME", home)
        .env("HOME", home)
        .output()
        .expect("run the real CLI")
}

fn json(output: &Output) -> Value {
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap_or_else(|e| {
        panic!(
            "not JSON ({e}): {}",
            String::from_utf8_lossy(&output.stdout)
        )
    })
}

fn backups(dir: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.to_string_lossy().contains(".backup-"))
        .collect();
    found.sort();
    found
}

fn program_count(db: &Path, home: &Path) -> usize {
    let list = knx(
        &["products", "list", "--product-db", db.to_str().unwrap()],
        home,
    );
    assert!(
        list.status.success(),
        "{}",
        String::from_utf8_lossy(&list.stderr)
    );
    String::from_utf8_lossy(&list.stdout)
        .lines()
        .filter(|line| line.contains("M-0001_A-0001-"))
        .count()
}

#[test]
fn preview_confirm_and_restore_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    std::fs::create_dir(&home).unwrap();
    let db = dir.path().join("products.sqlite");
    let dbs = db.to_str().unwrap();
    let v16 = package(dir.path(), 0x10);
    let v17 = package(dir.path(), 0x11);
    let seeded = knx(
        &[
            "products",
            "ingest",
            v16.to_str().unwrap(),
            "--product-db",
            dbs,
        ],
        &home,
    );
    assert!(
        seeded.status.success(),
        "{}",
        String::from_utf8_lossy(&seeded.stderr)
    );
    let before = std::fs::read(&db).unwrap();

    // Preview: JSON, nothing written, no backup.
    let preview = json(&knx(
        &[
            "products",
            "update",
            v17.to_str().unwrap(),
            "--product-db",
            dbs,
        ],
        &home,
    ));
    assert_eq!(preview["applied"], false, "{preview}");
    assert_eq!(preview["preview"]["programs"][0]["status"], "newVersion");
    assert_eq!(
        preview["preview"]["programs"][0]["installedVersions"][0]["id"],
        "M-0001_A-0001-10-0001"
    );
    assert_eq!(preview["preview"]["sourceName"], "dimmer-v17.knxprod");
    assert_eq!(
        preview["preview"]["packageSha256"].as_str().unwrap().len(),
        64
    );
    let token = preview["preview"]["token"].as_str().unwrap().to_string();
    assert_eq!(std::fs::read(&db).unwrap(), before);
    assert!(backups(dir.path()).is_empty());

    // A malformed token is refused before anything happens.
    let malformed = knx(
        &[
            "products",
            "update",
            v17.to_str().unwrap(),
            "--product-db",
            dbs,
            "--confirm",
            "nope",
        ],
        &home,
    );
    assert!(!malformed.status.success());
    assert_eq!(std::fs::read(&db).unwrap(), before);

    // Confirm: backup first, then the install.
    let applied = json(&knx(
        &[
            "products",
            "update",
            v17.to_str().unwrap(),
            "--product-db",
            dbs,
            "--confirm",
            &token,
        ],
        &home,
    ));
    assert_eq!(applied["applied"], true, "{applied}");
    let backup = PathBuf::from(applied["backup"].as_str().unwrap());
    assert!(backup.exists());
    assert_eq!(backups(dir.path()), vec![backup.clone()]);
    assert_eq!(program_count(&db, &home), 2);

    // The same token again is stale now.
    let stale = knx(
        &[
            "products",
            "update",
            v17.to_str().unwrap(),
            "--product-db",
            dbs,
            "--confirm",
            &token,
        ],
        &home,
    );
    assert!(!stale.status.success());
    assert!(String::from_utf8_lossy(&stale.stderr).contains("preview again"));

    // Restore the backup: back to one program.
    let restored = knx(
        &[
            "products",
            "restore",
            backup.to_str().unwrap(),
            "--product-db",
            dbs,
        ],
        &home,
    );
    assert!(
        restored.status.success(),
        "{}",
        String::from_utf8_lossy(&restored.stderr)
    );
    assert_eq!(program_count(&db, &home), 1);

    // A file that is not a product database is refused; the live file stays.
    let live = std::fs::read(&db).unwrap();
    let bogus = dir.path().join("bogus.sqlite");
    std::fs::write(&bogus, b"not a database").unwrap();
    let refused = knx(
        &[
            "products",
            "restore",
            bogus.to_str().unwrap(),
            "--product-db",
            dbs,
        ],
        &home,
    );
    assert!(!refused.status.success());
    assert_eq!(std::fs::read(&db).unwrap(), live);
}
