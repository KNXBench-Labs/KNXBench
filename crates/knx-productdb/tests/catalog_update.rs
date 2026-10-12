//! Previewed, recoverable catalogue updates from manufacturer files (ADR-0117).
//!
//! The preview installs into an in-memory copy and must leave the product
//! database file byte-identical; adoption needs the preview's token, writes a
//! backup first, and a restore brings the exact previous catalogue back.

use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};

use knx_productdb::query::resolve_program;
use knx_productdb::{
    apply_package_update, install_package, open_and_migrate, preview_package_update,
    restore_product_db_backup, PackageLimits, ProgramUpdateStatus, UpdateError,
};
use zip::write::SimpleFileOptions;

const MASTER: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/20"><MasterData><Manufacturers><Manufacturer Id="M-0001" Name="Example"/></Manufacturers></MasterData></KNX>"#;

fn program(version: u8) -> String {
    format!(
        r#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0001"><ApplicationPrograms><ApplicationProgram Id="M-0001_A-0001-{version:02X}-0001" Name="Dimmer" ApplicationNumber="1" ApplicationVersion="{version}" ProgramType="ApplicationProgram" MaskVersion="MV-07B0" PeiType="0" LoadProcedureStyle="MergedProcedure" DefaultLanguage="en-US"><Static/></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#
    )
}

fn hardware(version: u8) -> String {
    format!(
        r#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0001"><Hardware><Hardware Id="M-0001_H-1" Name="Dimmer hardware"><Products><Product Id="M-0001_H-1_P-1" Text="Dimmer" OrderNumber="DIM-1"/></Products><Hardware2Programs><Hardware2Program Id="M-0001_H-1_HP-0001-{version:02X}-0001" MediumTypes="MT-0"><ApplicationProgramRef RefId="M-0001_A-0001-{version:02X}-0001"/></Hardware2Program></Hardware2Programs></Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#
    )
}

fn package(version: u8) -> Vec<u8> {
    let program = program(version);
    let hardware = hardware(version);
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in [
        ("knx_master.xml", MASTER),
        ("M-0001/M-0001_A-0001.xml", program.as_bytes()),
        ("M-0001/Hardware.xml", hardware.as_bytes()),
    ] {
        zip.start_file(name, SimpleFileOptions::default()).unwrap();
        zip.write_all(bytes).unwrap();
    }
    zip.finish().unwrap().into_inner()
}

fn database_with(dir: &Path, versions: &[u8]) -> PathBuf {
    let path = dir.join("products.sqlite");
    let conn = open_and_migrate(&path).unwrap();
    for version in versions {
        install_package(
            &conn,
            &format!("dimmer-v{version}.knxprod"),
            &package(*version),
        )
        .unwrap();
    }
    drop(conn);
    path
}

fn program_ids(path: &Path) -> Vec<String> {
    let conn = open_and_migrate(path).unwrap();
    let mut statement = conn
        .prepare("SELECT id FROM application_program ORDER BY id")
        .unwrap();
    let ids = statement
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<String>, _>>()
        .unwrap();
    ids
}

#[test]
fn preview_reports_new_and_newer_programs_without_touching_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = database_with(dir.path(), &[0x10]);
    let before = std::fs::read(&path).unwrap();

    let preview = preview_package_update(
        &path,
        "dimmer-v17.knxprod",
        &package(0x11),
        PackageLimits::STANDARD,
    )
    .unwrap();
    assert_eq!(
        std::fs::read(&path).unwrap(),
        before,
        "a preview never writes"
    );
    assert_eq!(preview.source_name, "dimmer-v17.knxprod");
    assert_eq!(preview.package_sha256.len(), 64);
    assert!(!preview.package_already_installed);
    assert_eq!(preview.programs.len(), 1, "{preview:?}");
    let change = &preview.programs[0];
    assert_eq!(change.id, "M-0001_A-0001-11-0001");
    assert_eq!(change.application_version.as_deref(), Some("17"));
    assert_eq!(change.status, ProgramUpdateStatus::NewVersion);
    assert_eq!(change.installed_versions.len(), 1);
    assert_eq!(change.installed_versions[0].id, "M-0001_A-0001-10-0001");
    assert_eq!(
        change.installed_versions[0].application_version.as_deref(),
        Some("16")
    );
    assert_eq!(preview.token.len(), 64);

    let fresh = database_with(&dir.path().join("fresh"), &[]);
    let first = preview_package_update(
        &fresh,
        "dimmer-v16.knxprod",
        &package(0x10),
        PackageLimits::STANDARD,
    )
    .unwrap();
    assert_eq!(first.programs[0].status, ProgramUpdateStatus::New);
    assert!(first.programs[0].installed_versions.is_empty());

    let same = preview_package_update(
        &path,
        "renamed.knxprod",
        &package(0x10),
        PackageLimits::STANDARD,
    )
    .unwrap();
    assert!(same.package_already_installed);
    assert_eq!(
        same.programs[0].status,
        ProgramUpdateStatus::AlreadyInstalled
    );
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn adoption_needs_the_current_token_and_keeps_existing_bindings() {
    let dir = tempfile::tempdir().unwrap();
    let path = database_with(dir.path(), &[0x10]);
    let preview = preview_package_update(
        &path,
        "dimmer-v17.knxprod",
        &package(0x11),
        PackageLimits::STANDARD,
    )
    .unwrap();
    let before = std::fs::read(&path).unwrap();

    let stale = apply_package_update(
        &path,
        "dimmer-v17.knxprod",
        &package(0x11),
        PackageLimits::STANDARD,
        &"0".repeat(64),
    );
    assert!(matches!(stale, Err(UpdateError::StaleToken)), "{stale:?}");
    assert_eq!(
        std::fs::read(&path).unwrap(),
        before,
        "a refused adoption writes nothing"
    );

    let applied = apply_package_update(
        &path,
        "dimmer-v17.knxprod",
        &package(0x11),
        PackageLimits::STANDARD,
        &preview.token,
    )
    .unwrap();
    assert_eq!(applied.preview, preview);
    assert!(applied.backup.exists());
    assert!(!std::fs::read(&applied.backup).unwrap().is_empty());
    assert_eq!(
        program_ids(&path),
        vec![
            "M-0001_A-0001-10-0001".to_string(),
            "M-0001_A-0001-11-0001".to_string()
        ]
    );

    // The device bound to the old Hardware2Program still resolves to v16.
    let conn = open_and_migrate(&path).unwrap();
    assert_eq!(
        resolve_program(&conn, "M-0001_H-1_HP-0001-10-0001")
            .unwrap()
            .as_deref(),
        Some("M-0001_A-0001-10-0001")
    );
    drop(conn);

    // The token is spent: the catalogue changed, so it no longer matches.
    let again = apply_package_update(
        &path,
        "dimmer-v17.knxprod",
        &package(0x11),
        PackageLimits::STANDARD,
        &preview.token,
    );
    assert!(matches!(again, Err(UpdateError::StaleToken)), "{again:?}");
}

#[test]
fn a_restore_brings_back_the_exact_previous_catalogue() {
    let dir = tempfile::tempdir().unwrap();
    let path = database_with(dir.path(), &[0x10]);
    let preview = preview_package_update(
        &path,
        "dimmer-v17.knxprod",
        &package(0x11),
        PackageLimits::STANDARD,
    )
    .unwrap();
    let applied = apply_package_update(
        &path,
        "dimmer-v17.knxprod",
        &package(0x11),
        PackageLimits::STANDARD,
        &preview.token,
    )
    .unwrap();
    assert_eq!(program_ids(&path).len(), 2);

    restore_product_db_backup(&path, &applied.backup).unwrap();
    assert_eq!(
        program_ids(&path),
        vec!["M-0001_A-0001-10-0001".to_string()]
    );
    // A new preview of the same file sees the restored state.
    let after = preview_package_update(
        &path,
        "dimmer-v17.knxprod",
        &package(0x11),
        PackageLimits::STANDARD,
    )
    .unwrap();
    assert_eq!(after.token, preview.token, "restore is exact");

    // Something that is not a product database is refused; the live file stays.
    let live = std::fs::read(&path).unwrap();
    let bogus = dir.path().join("not-a-backup.sqlite");
    std::fs::write(&bogus, b"definitely not sqlite").unwrap();
    assert!(restore_product_db_backup(&path, &bogus).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), live);
}

#[test]
fn malformed_and_legacy_packages_are_refused_before_anything_is_written() {
    let dir = tempfile::tempdir().unwrap();
    let path = database_with(dir.path(), &[0x10]);
    let before = std::fs::read(&path).unwrap();
    for (name, bytes) in [
        ("broken.knxprod", b"PK\x03\x04 not really a zip".to_vec()),
        ("old.vd5", package(0x11)),
    ] {
        let preview = preview_package_update(&path, name, &bytes, PackageLimits::STANDARD);
        assert!(
            matches!(preview, Err(UpdateError::Package(_))),
            "{name}: {preview:?}"
        );
        let apply = apply_package_update(
            &path,
            name,
            &bytes,
            PackageLimits::STANDARD,
            &"0".repeat(64),
        );
        assert!(apply.is_err(), "{name}");
    }
    assert_eq!(std::fs::read(&path).unwrap(), before);
    let backups = std::fs::read_dir(dir.path())
        .unwrap()
        .filter(|entry| {
            entry
                .as_ref()
                .unwrap()
                .file_name()
                .to_string_lossy()
                .contains("backup")
        })
        .count();
    assert_eq!(backups, 0, "no backup for a refused update");
}
