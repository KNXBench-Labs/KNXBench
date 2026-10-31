//! Exercises native redaction through admitted ordinary and selective import producers.

use super::*;

fn produced(dir: &Path, selective: bool) -> PathBuf {
    let xml = topology(false)
        .replace(
            "http://knx.org/xml/project/21",
            "http://knx.org/xml/project/23",
        )
        .replace(
            "BCUKey=\"4294967295\"",
            "BCUKey=\"123\" IPRoutingBackboneKey=\"789\"",
        )
        .replace(
            "<DeviceInstance ",
            "<DeviceInstance xmlns:s=\"urn:s\" s:Key=\"456\" DiagnosticWitness=\"safe sample\" ",
        );
    let info = project_info(false).replace(
        "http://knx.org/xml/project/21",
        "http://knx.org/xml/project/23",
    );
    let bytes = knx_testsupport::zip_with_entries(&[
        ("P-0001.signature", b"synthetic signature"),
        ("P-0001/0.xml", xml.as_bytes()),
        ("P-0001/Project.xml", info.as_bytes()),
    ]);
    let path = dir.join("source.knxproj");
    std::fs::write(&path, &bytes).unwrap();
    let (project, opaque, refs) = if selective {
        let source = crate::selective_import::source_from_bytes(bytes, None).unwrap();
        (source.project, source.opaque, source.manufacturer_refs)
    } else {
        let conn = knx_store::open_and_migrate_in_memory().unwrap();
        let imported = crate::import_ets_project(&path, &conn).unwrap();
        let mut opaque = knx_store::load_opaque(&conn).unwrap();
        crate::import_evidence::capture_primary(&mut opaque, &imported.report).unwrap();
        (
            imported.project,
            opaque,
            knx_store::load_manufacturer_refs(&conn).unwrap(),
        )
    };
    let path = dir.join("project.knxdb");
    let conn = knx_store::open_and_migrate(&path).unwrap();
    knx_store::save_project_with_passthrough(&conn, &project, &opaque, &refs).unwrap();
    path
}

fn attribute_dialect(selective: bool) {
    let dir = tempfile::tempdir().unwrap();
    let path = produced(dir.path(), selective);
    let source = knx_store::open_existing_read_only(&path).unwrap();
    let rows = knx_store::load_opaque(&source.conn).unwrap();
    let kind = if selective {
        "SelectiveImportRetainedAttribute"
    } else {
        "RetainedAttribute"
    };
    for (name, value) in [("BCUKey", "123"), ("s:Key", "456")] {
        assert!(
            rows.iter()
                .any(|r| r.kind == kind && r.name == name && r.bytes == value.as_bytes()),
            "producer missing {name}"
        );
    }
    drop(source);
    let before = std::fs::read(&path).unwrap();
    let copy = native_project_without_secrets(&path).unwrap();
    let out = reopened(&copy.bytes, dir.path());
    let rows = knx_store::load_opaque(&out.conn).unwrap();
    for name in ["BCUKey", "s:Key"] {
        let row = rows
            .iter()
            .find(|r| r.kind == kind && r.name == name)
            .unwrap();
        assert!(
            row.bytes.is_empty(),
            "direct identified key {name} survived in {kind}"
        );
        assert_eq!(
            row.sha256,
            knx_store::project_history::image_hash(&row.bytes)
        );
    }
    assert_eq!(std::fs::read(path).unwrap(), before);
}

#[test]
fn secret_free_review_r2_selective_short_key_dialect() {
    attribute_dialect(true);
}
#[test]
fn secret_free_review_r2_ordinary_qualified_key_dialect() {
    attribute_dialect(false);
}

#[test]
fn secret_free_review_r3_selective_report_samples_and_key_digests() {
    let dir = tempfile::tempdir().unwrap();
    let path = produced(dir.path(), true);
    let source = knx_store::open_existing_read_only(&path).unwrap();
    let rows = knx_store::load_opaque(&source.conn).unwrap();
    let catalogue = crate::import_evidence::source_reports::catalogue(&rows).unwrap();
    let id = &catalogue[0].id;
    let original = crate::import_evidence::source_reports::read(&rows, id)
        .unwrap()
        .report_json;
    let report: serde_json::Value = serde_json::from_str(&original).unwrap();
    let digest = knx_store::project_history::image_hash(b"123");
    assert!(
        original.contains(&digest),
        "BCUKey must have a retained digest"
    );
    assert!(report["unknown"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["name"] == "s:Key" && r["sample"] == "456"));
    assert!(
        !report["unknown"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["name"] == "BCUKey"),
        "schema-known BCUKey has no unknown sample"
    );
    drop(source);
    let copy = native_project_without_secrets(&path).unwrap();
    let out = reopened(&copy.bytes, dir.path());
    let rows = knx_store::load_opaque(&out.conn).unwrap();
    let clean = crate::import_evidence::source_reports::read(&rows, id)
        .unwrap()
        .report_json;
    assert!(
        !clean.contains(&digest),
        "selective report leaked direct key digest"
    );
    let clean: serde_json::Value = serde_json::from_str(&clean).unwrap();
    assert_eq!(clean["counts"], report["counts"]);
    assert_eq!(
        clean["unknown"].as_array().unwrap().len(),
        report["unknown"].as_array().unwrap().len()
    );
    assert!(clean["unknown"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["name"] == "s:Key" && r["sample"] == ""));
    assert!(clean["unknown"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["name"] == "DiagnosticWitness" && r["sample"] == "safe sample"));
    let source = knx_store::open_existing_read_only(&path).unwrap();
    assert_eq!(
        crate::import_evidence::source_reports::read(
            &knx_store::load_opaque(&source.conn).unwrap(),
            id
        )
        .unwrap()
        .report_json,
        original
    );
}
