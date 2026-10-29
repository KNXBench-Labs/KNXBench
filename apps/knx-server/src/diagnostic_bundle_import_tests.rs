//! HTTP regression tests keep imported secret samples and digests out of diagnostic bundles.

use super::{call, json_of, state};
use axum::http::StatusCode;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::io::Read;

const BCU_KEY: &str = "3141592653";
const BACKBONE_KEY: &str = "SNTLdiagnosticBackboneKey012345";

#[tokio::test]
async fn import_report_preview_and_zip_remove_secrets_without_changing_historical_evidence() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source.knxproj");
    let topology = format!(
        r#"<KNX xmlns="http://knx.org/xml/project/21"><Project Id="P-0001">
<Installations><Installation InstallationId="0" Name="Main" BCUKey="{BCU_KEY}"
 IPRoutingBackboneKey="{BACKBONE_KEY}" DiagnosticWitness="safe sample">
<Topology/></Installation></Installations></Project></KNX>"#,
    );
    let info = br#"<KNX xmlns="http://knx.org/xml/project/21"><Project Id="P-0001">
<ProjectInformation Name="Synthetic diagnostic fixture" GroupAddressStyle="ThreeLevel"/>
</Project></KNX>"#;
    let original = knx_testsupport::zip_with_entries(&[
        ("P-0001.signature", b"synthetic signature"),
        ("P-0001/0.xml", topology.as_bytes()),
        ("P-0001/Project.xml", info),
    ]);
    std::fs::write(&source, &original).unwrap();
    let store = knx_store::open_and_migrate_in_memory().unwrap();
    let imported = knx_app::import_ets_project(&source, &store).unwrap();
    let original_report = serde_json::to_value(&imported.report).unwrap();
    // These are actual importer-produced samples/digests, not a mocked report.
    let original_text = original_report.to_string();
    assert!(
        original_report["unknown"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| { row["name"] == "IPRoutingBackboneKey" && row["sample"] == BACKBONE_KEY }),
        "fixture must carry an importer-produced backbone key sample"
    );
    assert!(original_text.contains(BACKBONE_KEY));
    let digest = |secret: &str| format!("{:x}", Sha256::digest(secret.as_bytes()));
    let key_digest = digest(BCU_KEY);
    assert!(
        original_text.contains(&key_digest),
        "fixture must carry its digest"
    );
    let mut opaque = knx_store::load_opaque(&store).unwrap();
    knx_app::import_evidence::capture_primary(&mut opaque, &imported.report).unwrap();
    let evidence_before = knx_app::import_evidence::read(&opaque)
        .unwrap()
        .unwrap()
        .report_json;
    let state = state(dir.path(), true);
    *state.project.lock().unwrap() = Some(imported.project);
    *state.opaque.lock().unwrap() = opaque;

    let (status, body, _) = call(
        &state,
        "POST",
        "/api/diagnostics/bundle/section",
        Some(json!({"problemType": "projectImport", "key": "importReport"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let view = json_of(&body);
    let text = view["text"].as_str().unwrap();
    let evidence_after = knx_app::import_evidence::read(&state.opaque.lock().unwrap())
        .unwrap()
        .unwrap()
        .report_json;
    assert_eq!(
        evidence_after, evidence_before,
        "export cannot rewrite history"
    );
    assert_eq!(std::fs::read(&source).unwrap(), original);
    for secret in [
        BCU_KEY.to_string(),
        BACKBONE_KEY.to_string(),
        key_digest,
        digest(BACKBONE_KEY),
    ] {
        assert!(
            !text.contains(&secret),
            "secret sample or digest leaked into section text"
        );
    }

    let (status, bytes, _) = call(
        &state,
        "POST",
        "/api/diagnostics/bundle",
        Some(json!({"problemType": "projectImport", "consent": true})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut report = String::new();
    archive
        .by_name("import/import-report.json")
        .unwrap()
        .read_to_string(&mut report)
        .unwrap();
    let exported: Value = serde_json::from_str(&report).unwrap();
    assert_eq!(exported["counts"], original_report["counts"]);
    assert_eq!(
        exported["unknown"].as_array().unwrap().len(),
        original_report["unknown"].as_array().unwrap().len()
    );
    assert!(
        report.contains("safe sample"),
        "unrelated diagnostic values remain useful"
    );
    let mut replay = knx_app::import_evidence::encode(&imported.report).unwrap();
    let original_counts = knx_app::import_evidence::read(std::slice::from_ref(&replay))
        .unwrap()
        .unwrap()
        .counts();
    let mut envelope: Value = serde_json::from_slice(&replay.bytes).unwrap();
    envelope["report_json"] = json!(report);
    replay.bytes = serde_json::to_vec(&envelope).unwrap();
    replay.sha256 = knx_store::project_history::image_hash(&replay.bytes);
    assert_eq!(
        knx_app::import_evidence::read(&[replay])
            .unwrap()
            .unwrap()
            .counts(),
        original_counts,
        "the actual historical reader must derive identical error/warning totals"
    );
    let mut manifest = String::new();
    archive
        .by_name("manifest.json")
        .unwrap()
        .read_to_string(&mut manifest)
        .unwrap();
    let manifest: Value = serde_json::from_str(&manifest).unwrap();
    let section = manifest["sections"]
        .as_array()
        .unwrap()
        .iter()
        .find(|section| section["kind"] == "importReport")
        .unwrap();
    assert!(
        section["notes"].as_array().unwrap().iter().any(|note| note
            .as_str()
            .is_some_and(|note| note.contains("IPRoutingBackboneKey"))),
        "the exported manifest must disclose the known-key redaction"
    );
    for index in 0..archive.len() {
        let mut payload = Vec::new();
        archive
            .by_index(index)
            .unwrap()
            .read_to_end(&mut payload)
            .unwrap();
        let text = String::from_utf8_lossy(&payload);
        for secret in [
            BCU_KEY.to_string(),
            BACKBONE_KEY.to_string(),
            digest(BCU_KEY),
            digest(BACKBONE_KEY),
        ] {
            assert!(
                !text.contains(&secret),
                "secret sample or digest leaked into ZIP entry"
            );
        }
    }
    assert_eq!(
        knx_app::import_evidence::read(&state.opaque.lock().unwrap())
            .unwrap()
            .unwrap()
            .report_json,
        evidence_before
    );
    assert_eq!(std::fs::read(&source).unwrap(), original);
}

#[tokio::test]
async fn import_report_identity_collision_is_visibly_unavailable_not_rewritten() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("identity-collision.knxproj");
    let xml = br#"<KNX xmlns="http://knx.org/xml/project/21"><Project Id="P-0001">
<Installations><Installation InstallationId="0" Name="Main"
 Password="payload" DiagnosticWitness="safe sample">
<Topology/></Installation></Installations></Project></KNX>"#;
    let info = br#"<KNX xmlns="http://knx.org/xml/project/21"><Project Id="P-0001">
<ProjectInformation Name="Synthetic identity collision" GroupAddressStyle="ThreeLevel"/>
</Project></KNX>"#;
    let original = knx_testsupport::zip_with_entries(&[
        ("P-0001.signature", b"synthetic signature"),
        ("P-0001/0.xml", xml),
        ("P-0001/Project.xml", info),
        ("M-TEST/Baggages/a.bin", b"safe a"),
        ("M-TEST/Baggages/b.bin", b"safe b"),
    ]);
    std::fs::write(&source, &original).unwrap();
    let conn = knx_store::open_and_migrate_in_memory().unwrap();
    let imported = knx_app::import_ets_project(&source, &conn).unwrap();
    let mut opaque = knx_store::load_opaque(&conn).unwrap();
    knx_app::import_evidence::capture_primary(&mut opaque, &imported.report).unwrap();
    let historical = knx_app::import_evidence::read(&opaque).unwrap().unwrap();
    assert_eq!(
        imported
            .report
            .unsupported
            .iter()
            .filter(|row| row.what.starts_with("M-TEST/Baggages/"))
            .count(),
        2,
        "the real importer must produce the grouping-sensitive retained notices"
    );
    assert!(
        historical.counts().1 > 0,
        "calibrate historical warning counts"
    );
    let refusal = knx_app::secret_free_copy::import_report_without_secrets(&historical.report)
        .unwrap_err()
        .to_string();
    assert!(
        refusal.contains("report unavailable"),
        "calibrate identity collision"
    );
    let before = opaque.clone();
    let state = state(dir.path(), true);
    *state.project.lock().unwrap() = Some(imported.project);
    *state.opaque.lock().unwrap() = opaque;
    let query = json!({ "problemType": "projectImport", "sections": ["importReport"] });
    let (status, body, _) = call(
        &state,
        "POST",
        "/api/diagnostics/bundle/preview",
        Some(query.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let preview = json_of(&body);
    let offer = preview["offers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|offer| offer["kind"] == "importReport")
        .unwrap();
    assert!(
        offer["unavailable"]
            .as_str()
            .is_some_and(|reason| reason.contains("report unavailable")),
        "the real HTTP preview must visibly refuse a meaning-changing report projection"
    );
    assert!(!preview["sections"]
        .as_array()
        .unwrap()
        .iter()
        .any(|section| section["kind"] == "importReport"));
    let mut text_query = query.clone();
    text_query["key"] = json!("importReport");
    let (status, body, _) = call(
        &state,
        "POST",
        "/api/diagnostics/bundle/section",
        Some(text_query),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(String::from_utf8_lossy(&body).contains("bundleSectionUnknown"));
    let mut export_query = query;
    export_query["consent"] = json!(true);
    let (status, body, _) = call(
        &state,
        "POST",
        "/api/diagnostics/bundle",
        Some(export_query),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(body)).unwrap();
    assert!(archive.by_name("import/import-report.json").is_err());
    assert_eq!(*state.opaque.lock().unwrap(), before);
    assert_eq!(std::fs::read(source).unwrap(), original);
}
