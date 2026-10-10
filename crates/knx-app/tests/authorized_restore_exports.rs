//! Opt-in regressions over authorized ETS exports, never raw internal restore points.

use serde::Deserialize;
use std::collections::BTreeSet;
use std::io::Read;
use std::path::{Component, Path};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Manifest {
    format_version: u8,
    authorized_ets_exports: bool,
    exports: Vec<Export>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Export {
    file: String,
    sha256: String,
    expected: Expected,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Expected {
    schema: u32,
    devices: usize,
    group_addresses: usize,
    communication_objects: usize,
    parameters: usize,
    modules: usize,
    unknown: usize,
    opaque: usize,
    unsupported: usize,
    errors: usize,
}

fn parse_manifest(bytes: &[u8]) -> Result<Manifest, &'static str> {
    if bytes.len() > 1024 * 1024 {
        return Err("manifest budget exceeded");
    }
    let manifest: Manifest =
        serde_json::from_slice(bytes).map_err(|_| "malformed authorized-export manifest")?;
    if manifest.format_version != 1 || !manifest.authorized_ets_exports {
        return Err("explicit authorized ETS export attestation required");
    }
    if !(2..=16).contains(&manifest.exports.len()) {
        return Err("between 2 and 16 exported revisions required");
    }
    let mut files = BTreeSet::new();
    let mut hashes = BTreeSet::new();
    for export in &manifest.exports {
        let path = Path::new(&export.file);
        if export.file.is_empty()
            || export.file.contains(['\\', ':', '\0'])
            || export
                .file
                .split('/')
                .any(|c| c.is_empty() || c == "." || c == "..")
            || path
                .components()
                .any(|c| !matches!(c, Component::Normal(_)))
            || !path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("knxproj"))
        {
            return Err("only relative .knxproj export paths are accepted");
        }
        if export.sha256.len() != 64
            || !export
                .sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err("expected SHA-256 must be 64 lowercase hexadecimal characters");
        }
        if !files.insert(&export.file) {
            return Err("duplicate export path");
        }
        hashes.insert(&export.sha256);
    }
    if hashes.len() < 2 {
        return Err("at least two distinct exported byte revisions required");
    }
    Ok(manifest)
}

#[cfg(target_os = "linux")]
fn open_manifest_directory(path: &Path) -> Result<std::fs::File, &'static str> {
    use rustix::fs::{openat2, Mode, OFlags, ResolveFlags, CWD};
    openat2(
        CWD,
        path,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
        ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
    )
    .map(std::fs::File::from)
    .map_err(|_| "manifest directory refused")
}
#[cfg(target_os = "linux")]
fn open_confined_regular(root: &std::fs::File, path: &Path) -> Result<std::fs::File, &'static str> {
    use rustix::fs::{openat2, Mode, OFlags, ResolveFlags};
    let file = std::fs::File::from(
        openat2(
            root,
            path,
            OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NONBLOCK,
            Mode::empty(),
            ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
        )
        .map_err(|_| "export path refused")?,
    );
    if !file
        .metadata()
        .map_err(|_| "file metadata unavailable")?
        .is_file()
    {
        return Err("regular export file required");
    }
    Ok(file)
}
#[cfg(not(target_os = "linux"))]
fn open_manifest_directory(_: &Path) -> Result<std::fs::File, &'static str> {
    Err("confined private export regression requires Linux")
}
#[cfg(not(target_os = "linux"))]
fn open_confined_regular(_: &std::fs::File, _: &Path) -> Result<std::fs::File, &'static str> {
    Err("confined private export regression requires Linux")
}

fn run_manifest(path: &Path) -> Result<usize, &'static str> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let directory = open_manifest_directory(parent).map_err(|_| "manifest unavailable")?;
    let name = path.file_name().ok_or("manifest unavailable")?;
    let mut bytes = vec![];
    open_confined_regular(&directory, Path::new(name))
        .map_err(|_| "manifest unavailable")?
        .take(1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "manifest read failed")?;
    let manifest = parse_manifest(&bytes)?;
    let mut project_id = None;
    for export in &manifest.exports {
        let path = Path::new(&export.file);
        let mut original = vec![];
        open_confined_regular(&directory, path)
            .map_err(|_| "export path refused")?
            .take(knx_app::selective_import::MAX_SOURCE_BYTES as u64 + 1)
            .read_to_end(&mut original)
            .map_err(|_| "export read failed")?;
        if original.len() > knx_app::selective_import::MAX_SOURCE_BYTES {
            return Err("export budget exceeded");
        }
        if knx_etsproj::opaque::sha256_hex(&original) != export.sha256 {
            return Err("authorized export identity drift");
        }
        let conn =
            knx_store::open_and_migrate_in_memory().map_err(|_| "disposable store setup failed")?;
        let imported = knx_app::import::import_ets_project_bytes(
            original.clone(),
            "authorized-export.knxproj",
            &conn,
            knx_app::ImportOptions::default(),
        )
        .map_err(|_| "project importer refused an authorized fixture")?;
        let p = &imported.project;
        let expected = &export.expected;
        if imported.report.source.schema_version != expected.schema
            || p.devices.iter().count() != expected.devices
            || p.installations
                .iter()
                .map(|i| i.group_addresses.len())
                .sum::<usize>()
                != expected.group_addresses
            || p.devices.com_objects().count() != expected.communication_objects
            || p.installations
                .iter()
                .map(|i| i.parameters.len())
                .sum::<usize>()
                != expected.parameters
            || p.devices.module_instances().count() != expected.modules
            || imported.report.unknown.len() != expected.unknown
            || imported.opaque_entries != expected.opaque
            || imported.report.unsupported.len() != expected.unsupported
            || imported.report.error_count() != expected.errors
        {
            return Err("authorized-export baseline drift");
        }
        if let Some(id) = &project_id {
            if id != &p.info.project_id {
                return Err("exports do not identify one project");
            }
        } else {
            project_id = Some(p.info.project_id.clone());
        }
        knx_store::save_project(&conn, p)
            .map_err(|_| "export model is not natively representable")?;
        let loaded = knx_store::load_project(&conn).map_err(|_| "native reload failed")?;
        if loaded != *p {
            return Err("native model roundtrip mismatch");
        }
        let opaque =
            knx_store::load_opaque(&conn).map_err(|_| "retained evidence reload failed")?;
        if opaque.len() != expected.opaque {
            return Err("retained evidence count mismatch");
        }
        for entry in &opaque {
            if knx_etsproj::opaque::sha256_hex(&entry.bytes) != entry.sha256 {
                return Err("retained-byte commitment mismatch");
            }
        }
        let mut after = vec![];
        open_confined_regular(&directory, path)
            .map_err(|_| "export readback failed")?
            .take(knx_app::selective_import::MAX_SOURCE_BYTES as u64 + 1)
            .read_to_end(&mut after)
            .map_err(|_| "export readback failed")?;
        if after != original {
            return Err("private source changed during regression");
        }
    }
    Ok(manifest.exports.len())
}

fn synthetic_manifest() -> serde_json::Value {
    let expected = serde_json::json!({"schema":11,"devices":1,"groupAddresses":1,"communicationObjects":1,"parameters":0,"modules":0,"unknown":0,"opaque":0,"unsupported":0,"errors":0});
    serde_json::json!({"formatVersion":1,"authorizedEtsExports":true,"exports":[
        {"file":"revision-a.knxproj","sha256":"a".repeat(64),"expected":expected},
        {"file":"revision-b.knxproj","sha256":"b".repeat(64),"expected":expected}]})
}

#[test]
fn manifest_requires_authorization_unique_exports_and_exact_expectations() {
    let valid = synthetic_manifest();
    assert!(parse_manifest(&serde_json::to_vec(&valid).unwrap()).is_ok());
    for change in ["authorization", "identity", "counts", "unknown"] {
        let mut v = valid.clone();
        match change {
            "authorization" => v["authorizedEtsExports"] = false.into(),
            "identity" => v["exports"][1]["sha256"] = v["exports"][0]["sha256"].clone(),
            "counts" => {
                v["exports"][0]["expected"]
                    .as_object_mut()
                    .unwrap()
                    .remove("parameters");
            }
            _ => v["unexpected"] = true.into(),
        }
        assert!(parse_manifest(&serde_json::to_vec(&v).unwrap()).is_err());
    }
}

#[test]
fn internal_restorepoint_absolute_and_traversing_paths_are_not_interchange_fixtures() {
    for path in [
        "live.restorepoint",
        "../revision.knxproj",
        "/private/revision.knxproj",
        "a\\revision.knxproj",
        "C:revision.knxproj",
        "a/./revision.knxproj",
        "a//revision.knxproj",
    ] {
        let mut v = synthetic_manifest();
        v["exports"][0]["file"] = path.into();
        assert!(parse_manifest(&serde_json::to_vec(&v).unwrap()).is_err());
    }
}

#[test]
fn explicitly_missing_manifest_is_a_refusal_not_a_successful_empty_regression() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(
        run_manifest(&dir.path().join("missing.json")),
        Err("manifest unavailable")
    );
}

#[test]
#[ignore = "requires authorized private ETS restore-point .knxproj exports and manifest"]
fn authorized_restore_point_exports_match_private_baselines_and_native_roundtrips() {
    let path = std::env::var_os("KNXBENCH_RESTORE_EXPORT_MANIFEST").expect(
        "KNXBENCH_RESTORE_EXPORT_MANIFEST is required; no authorized ETS export regression was run",
    );
    let result = run_manifest(Path::new(&path));
    assert!(
        result.is_ok(),
        "authorized-export regression refused; inspect private inputs locally (no source details printed)"
    );
    println!("authorized_restore_exports_verified={}", result.unwrap());
}

#[test]
#[cfg(target_os = "linux")]
fn authorized_export_paths_cannot_follow_leaf_or_ancestor_symlinks() {
    use std::os::unix::fs::symlink;
    let directory = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let data = b"not an ETS fixture; only a confinement witness";
    std::fs::write(outside.path().join("revision.knxproj"), data).unwrap();
    symlink(
        outside.path().join("revision.knxproj"),
        directory.path().join("leaf.knxproj"),
    )
    .unwrap();
    symlink(outside.path(), directory.path().join("ancestor")).unwrap();
    for relative in ["leaf.knxproj", "ancestor/revision.knxproj"] {
        let mut manifest = synthetic_manifest();
        manifest["exports"][0]["file"] = relative.into();
        manifest["exports"][0]["sha256"] = knx_etsproj::opaque::sha256_hex(data).into();
        std::fs::write(
            directory.path().join("manifest.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        assert_eq!(
            run_manifest(&directory.path().join("manifest.json")),
            Err("export path refused")
        );
    }
    assert_eq!(
        std::fs::read(outside.path().join("revision.knxproj")).unwrap(),
        data
    );
}
