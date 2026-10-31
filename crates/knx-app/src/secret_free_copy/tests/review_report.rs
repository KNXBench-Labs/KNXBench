//! Regresses the public import-report sanitizer without mutating historical evidence.

use super::*;

#[test]
fn secret_free_report_refuses_secret_row_identity_instead_of_collapsing_counts() {
    let report = serde_json::json!({"unknown":[
        {"name":"Password","sample":"IdentityPasswordWitness"},
        {"source_path":"IdentityPasswordWitness-a.xml","xpath":"/a","kind":"attribute","name":"Unrelated","sample":"safe"},
        {"source_path":"IdentityPasswordWitness-b.xml","xpath":"/a","kind":"attribute","name":"Unrelated","sample":"safe"}]});
    let before = report.clone();
    assert!(
        import_report_without_secrets(&report).is_err(),
        "redacted identities silently collapsed distinct historical rows"
    );
    assert_eq!(report, before);
}

#[test]
fn secret_free_report_refuses_nonobject_input() {
    assert!(import_report_without_secrets(&serde_json::json!("not a report")).is_err());
}

#[test]
fn secret_free_report_refuses_overdeep_input() {
    let mut nested = serde_json::json!({"name":"safe"});
    for _ in 0..65 {
        nested = serde_json::json!({"child":nested});
    }
    assert!(import_report_without_secrets(&nested).is_err());
}

#[test]
fn secret_free_report_refuses_secret_in_json_object_identity() {
    let report = serde_json::json!({"unknown":[{"name":"Password","sample":"IdentityPasswordWitness"}],
        "other":{"IdentityPasswordWitness":"safe"}});
    assert!(
        import_report_without_secrets(&report).is_err(),
        "known secret survived in JSON member identity"
    );
}

#[test]
fn secret_free_review_r3_shared_report_connection_digest() {
    let digest = knx_store::project_history::image_hash(b"Host=example;UserPassword=123;Port=3671");
    let report = serde_json::json!({"counts":{"rows":[{"entity":"device","read":1,"mapped":1}]},
        "unknown":[{"source_path":"0.xml","xpath":"/s:BusAccess","name":"s:Parameter",
        "sample":"Host=example;UserPassword=123;Port=3671","sha256":digest},
        {"name":"s:Key","sample":"456","sha256":knx_store::project_history::image_hash(b"456")},
        {"name":"DiagnosticWitness","sample":"safe sample"}]});
    let before = report.clone();
    let copy = import_report_without_secrets(&report).unwrap();
    assert_eq!(report, before);
    let out: serde_json::Value = serde_json::from_slice(&copy.bytes).unwrap();
    assert_eq!(out["counts"], report["counts"]);
    assert_eq!(out["unknown"].as_array().unwrap().len(), 3);
    for (before, after) in report["unknown"]
        .as_array()
        .unwrap()
        .iter()
        .zip(out["unknown"].as_array().unwrap())
    {
        for field in ["source_path", "xpath", "kind", "name"] {
            assert_eq!(before[field], after[field]);
        }
    }
    assert_eq!(
        out["unknown"][0]["sample"], "Host=example;UserPassword=;Port=3671",
        "connection credential survived"
    );
    assert_eq!(
        out["unknown"][0]["sha256"], "",
        "credential-containing field digest survived"
    );
    assert_eq!(out["unknown"][1]["sample"], "");
    assert_eq!(out["unknown"][1]["sha256"], "");
    assert_eq!(out["unknown"][2], report["unknown"][2]);
}

#[test]
fn secret_free_review_r6_report_metadata_and_connection_part_names() {
    let report = serde_json::json!({"unknown":[
        {"name":"Password","source_path":"0.xml","xpath":"/a","detail":"MetaPasswordWitness","sample":"MetaPasswordWitness"},
        {"name":"Parameter","xpath":"/BusAccess","sample":"Password-MetaPasswordWitness=MetaPasswordWitness;Host=example"},
        {"name":"DiagnosticWitness","sample":"safe sample"}]});
    let before = report.clone();
    let copy = import_report_without_secrets(&report).unwrap();
    assert_eq!(report, before);
    let decoded: serde_json::Value = serde_json::from_slice(&copy.bytes).unwrap();
    assert!(
        !decoded.to_string().contains("MetaPasswordWitness"),
        "report metadata leaked removed password"
    );
    assert!(!serde_json::to_string(&copy)
        .unwrap()
        .contains("MetaPasswordWitness"));
    assert!(!format!("{copy:?}").contains("MetaPasswordWitness"));
    assert_eq!(decoded["unknown"][2], report["unknown"][2]);
}
