use super::*;
use crate::catalogue_jobs::{artifacts::ArtifactRoot, Handle, Incarnation};
use std::sync::Arc;

fn workspace() -> (tempfile::TempDir, JobWorkspace) {
    let tmp = tempfile::tempdir().unwrap();
    let root = Arc::new(ArtifactRoot::acquire(tmp.path()).unwrap());
    let incarnation = Incarnation::from_bytes([1; 16]);
    let dir = root.create_incarnation_dir(&incarnation).unwrap();
    let ws = JobWorkspace::create(
        root,
        &dir,
        Handle {
            incarnation,
            serial: U64(1),
            job: JobId::from_bytes([2; 16]),
        },
    )
    .unwrap();
    (tmp, ws)
}
fn summary() -> Value {
    serde_json::json!({"ordinal":"0","kind":"resultSummary","packages":"1","cataloguePageBytes":"4096","nextJobInputLimitExceeded":false})
}

#[test]
fn record_and_combined_artifact_limits_refuse_before_persistent_append() {
    let (_tmp, mut ws) = workspace();
    let dir = ws.directory().to_path_buf();
    ws.artifact_bytes = 67_108_863;
    let mut writer = ArtifactWriter::new(&mut ws, ArtifactKind::Result).unwrap();
    assert_eq!(writer.append(&summary()), Err(FailureKind::ResultLimit));
    assert_eq!(
        std::fs::metadata(dir.join("result.raw.part"))
            .unwrap()
            .len(),
        0
    );
    drop(writer);
    let (_tmp, mut ws) = workspace();
    let dir = ws.directory().to_path_buf();
    ws.artifact_records = 100_000;
    let mut writer = ArtifactWriter::new(&mut ws, ArtifactKind::Result).unwrap();
    assert_eq!(writer.append(&summary()), Err(FailureKind::ResultLimit));
    assert_eq!(
        std::fs::metadata(dir.join("result.public.part"))
            .unwrap()
            .len(),
        0
    );
    drop(writer);
    let (_tmp, mut ws) = workspace();
    let dir = ws.directory().to_path_buf();
    let mut writer = ArtifactWriter::new(&mut ws, ArtifactKind::Result).unwrap();
    writer.append(&summary()).unwrap();
    let before = std::fs::metadata(dir.join("result.raw.part"))
        .unwrap()
        .len();
    let oversized = serde_json::json!({"ordinal":"1","kind":"provenance","sourceKey":"0","sha256":"00".repeat(32),"alias":"x".repeat(65_536),"packageKey":"0"});
    assert_eq!(writer.append(&oversized), Err(FailureKind::RecordLimit));
    assert_eq!(
        std::fs::metadata(dir.join("result.raw.part"))
            .unwrap()
            .len(),
        before
    );
}

#[test]
fn baggage_projection_withholds_all_seven_lexemes_including_empty_values() {
    let (_tmp, mut ws) = workspace();
    let dir = ws.directory().to_path_buf();
    let mut writer = ArtifactWriter::new(&mut ws, ArtifactKind::Result).unwrap();
    writer.append(&summary()).unwrap();
    writer.append(&serde_json::json!({"ordinal":"1","kind":"baggageDeclaration","packageKey":"0","indexPath":"Baggages.xml","declarationOrdinal":"0","declarationId":"private","name":"","targetPath":"private","installOnImport":"private","timeInfo":"private","fileVersion":"private","detail":"private","resolution":"missing","memberPath":null})).unwrap();
    let artifact = writer.finish().unwrap();
    assert_eq!(artifact.withheld_fields, U64(7));
    assert_eq!(artifact.withheld_records, U64(1));
    assert_eq!(artifact.withheld_occurrences, U64(1));
    let public = std::fs::read_to_string(dir.join("result.public")).unwrap();
    assert!(!public.contains("private"));
    let row: Value = serde_json::from_str(public.lines().nth(1).unwrap()).unwrap();
    assert_eq!(row["withheld"][1]["rawUtf8Bytes"], "0");
    assert_eq!(row["withheld"][0]["field"], "declarationId");
}

#[test]
fn pages_are_complete_byte_bounded_maximal_prefixes_and_detect_changed_content() {
    let (_tmp, mut ws) = workspace();
    let dir = ws.directory().to_path_buf();
    let mut writer = ArtifactWriter::new(&mut ws, ArtifactKind::Result).unwrap();
    writer.append(&summary()).unwrap();
    for ordinal in 1..=40 {
        writer.append(&serde_json::json!({"ordinal":ordinal.to_string(),"kind":"wrapperOtherMember","path":"a".repeat(8000),"size":"0","sha256":"00".repeat(32),"memberKind":"file"})).unwrap();
    }
    let artifact = writer.finish().unwrap();
    let mut offset = U64(0);
    let mut ordinals = Vec::new();
    let mut first_len = 0;
    loop {
        let page = detail_page(&ws, &artifact, offset, 200).unwrap();
        assert!(page.len() <= 262_144);
        let value: Value = serde_json::from_slice(&page).unwrap();
        let rows = value["records"].as_array().unwrap();
        assert!(!rows.is_empty());
        if offset == U64(0) {
            first_len = rows.len();
            assert!(rows.len() < 41);
            let next_row_len = std::fs::read_to_string(dir.join("result.public"))
                .unwrap()
                .lines()
                .nth(rows.len())
                .unwrap()
                .len();
            assert!(page.len() + next_row_len + 1 > 262_144);
        }
        for row in rows {
            ordinals.push(row["ordinal"].as_str().unwrap().parse::<u64>().unwrap());
        }
        match value["nextOffset"].as_str() {
            Some(next) => offset = next.parse().unwrap(),
            None => break,
        }
    }
    assert!(first_len > 1);
    assert_eq!(ordinals, (0..41).collect::<Vec<_>>());
    let bytes = std::fs::read(dir.join("result.public")).unwrap();
    std::fs::write(dir.join("result.public"), &bytes).unwrap();
    assert!(matches!(
        detail_page(&ws, &artifact, U64(0), 200),
        Err(FailureKind::IoFailure)
    ));
}

// These exercise the real writer before publication, not only its leaf parser.
#[test]
fn master_declaration_selectors_preserve_identity_and_withhold_only_detail() {
    let policy: Value = serde_json::from_str(include_str!(
        "../../../../../docs/contracts/catalogue-jobs/wp043-protocol-policy.json"
    ))
    .unwrap();
    assert_eq!(
        policy["masterDeclarationIdentity"]["maximumOrdinal"],
        knx_productdb::master_provenance::MAX_MASTER_DECLARATIONS
    );
    let (_tmp, mut ws) = workspace();
    let dir = ws.directory().to_path_buf();
    let mut writer = ArtifactWriter::new(&mut ws, ArtifactKind::Result).unwrap();
    writer.append(&summary()).unwrap();
    let mut expected = Vec::new();
    for (index, element) in [
        "DatapointType",
        "DatapointSubtype",
        "FunctionType",
        "FunctionPoint",
        "SpaceUsage",
    ]
    .iter()
    .enumerate()
    {
        for (offset, (diagnostic_kind, ordinal)) in [
            ("master-identity-collision", 1),
            ("malformed-master-declaration", 1_000_000),
        ]
        .iter()
        .enumerate()
        {
            let path = format!("/KNX/MasterData/{element}[@master-ordinal={ordinal}]");
            let raw = serde_json::json!({"ordinal":(1 + index * 2 + offset).to_string(),"kind":"diagnostic","packageKey":"0","diagnosticKind":diagnostic_kind,"archivePath":"knx_master.xml","xmlPath":path,"detail":"/home/alice/private <source-text> \u{00e4}","occurrences":"1"});
            writer.append(&raw).unwrap();
            expected.push(raw);
        }
    }
    let artifact = writer.finish().unwrap();
    assert_eq!(artifact.records, U64(11));
    assert_eq!(artifact.withheld_fields, U64(10));
    assert_eq!(artifact.withheld_records, U64(10));
    assert_eq!(artifact.withheld_occurrences, U64(10));
    let raw = std::fs::read_to_string(dir.join("result.raw")).unwrap();
    let public = std::fs::read_to_string(dir.join("result.public")).unwrap();
    assert!(!public.contains("/home/alice"));
    assert!(!public.contains("source-text"));
    for ((raw, public), expected) in raw
        .lines()
        .skip(1)
        .zip(public.lines().skip(1))
        .zip(expected)
    {
        let raw: Value = serde_json::from_str(raw).unwrap();
        let mut public: Value = serde_json::from_str(public).unwrap();
        assert_eq!(raw, expected);
        assert_eq!(
            public["withheld"],
            serde_json::json!([{"field":"detail","reason":"untrustedFreeText","rawUtf8Bytes":raw["detail"].as_str().unwrap().len().to_string()}])
        );
        public.as_object_mut().unwrap().remove("withheld");
        public["detail"] = raw["detail"].clone();
        assert_eq!(
            public, raw,
            "projection must not remove or rewrite declaration identity"
        );
    }
}

#[test]
fn invalid_or_wrong_kind_master_selectors_refuse_without_appending() {
    let (_tmp, mut ws) = workspace();
    let dir = ws.directory().to_path_buf();
    let mut writer = ArtifactWriter::new(&mut ws, ArtifactKind::Result).unwrap();
    writer.append(&summary()).unwrap();
    let before: Vec<_> = ["result.raw.part", "result.public.part", "result.index.part"]
        .map(|name| std::fs::metadata(dir.join(name)).unwrap().len())
        .into();
    let base = "/KNX/MasterData/FunctionTypes/FunctionType";
    let valid = format!("{base}[@master-ordinal=1]");
    let row = serde_json::json!({"ordinal":"1","kind":"diagnostic","packageKey":"0","diagnosticKind":"master-identity-collision","archivePath":"knx_master.xml","xmlPath":valid,"detail":"private","occurrences":"1"});
    let mut invalid = vec![
        base.to_string(),
        format!("{base}[1]"),
        format!("{base}[@Id='private']"),
        format!("{valid}/FunctionPoint"),
        format!("{valid}[@master-ordinal=2]"),
        format!("{valid}\n"),
        format!("/{}FunctionType[@master-ordinal=1]", "Name/".repeat(1700)),
        "/KNX/Other[@master-ordinal=1]".into(),
        "/KNX/../FunctionType[@master-ordinal=1]".into(),
        "/KNX//FunctionType[@master-ordinal=1]".into(),
        "/KNX\n/FunctionType[@master-ordinal=1]".into(),
        "/KNX/FunctionType\u{200b}[@master-ordinal=1]".into(),
        format!("/{}/FunctionType[@master-ordinal=1]", "A".repeat(513)),
    ];
    for ordinal in [
        "0",
        "01",
        "+1",
        "-1",
        " 1",
        "1 ",
        "1\n",
        "１",
        "1.0",
        "1e1",
        "'1'",
        "1000001",
        "18446744073709551616",
        "1][@Id=private",
    ] {
        invalid.push(format!("{base}[@master-ordinal={ordinal}]"));
    }
    for kind in ["master-identity-collision", "malformed-master-declaration"] {
        for path in &invalid {
            let mut bad = row.clone();
            bad["diagnosticKind"] = kind.into();
            bad["xmlPath"] = path.clone().into();
            assert_eq!(
                writer.append(&bad),
                Err(FailureKind::PublicProjectionRefused)
            );
        }
    }
    for (field, value) in [
        ("diagnosticKind", "unsupported-master-section"),
        ("diagnosticKind", "unsupported-master-subtree"),
        ("diagnosticKind", "unresolved-baggage-declaration"),
        ("diagnosticKind", "undeclared-baggage-payload"),
        ("occurrences", "0"),
        ("occurrences", "2"),
        ("archivePath", "/home/alice/private"),
        ("archivePath", "../knx_master.xml"),
        ("archivePath", "C:\\private"),
        ("archivePath", "knx_master.xml\u{200b}"),
    ] {
        let mut bad = row.clone();
        bad[field] = value.into();
        assert_eq!(
            writer.append(&bad),
            Err(FailureKind::PublicProjectionRefused)
        );
    }
    let unknown = serde_json::json!({"ordinal":"1","kind":"unknownConstruct","packageKey":"0","xpath":valid,"unknownKind":"Element","name":"FunctionType","occurrences":"1","sample":"private"});
    assert_eq!(
        writer.append(&unknown),
        Err(FailureKind::PublicProjectionRefused)
    );
    for (index, name) in ["result.raw.part", "result.public.part", "result.index.part"]
        .iter()
        .enumerate()
    {
        assert_eq!(
            std::fs::metadata(dir.join(name)).unwrap().len(),
            before[index]
        );
    }
}

#[test]
fn manifest_conflict_keeps_existing_file_and_never_admits_descriptor() {
    let (_tmp, mut ws) = workspace();
    let dir = ws.directory().to_path_buf();
    let mut old = ws.create_file("result.manifest").unwrap();
    old.write_all(b"existing").unwrap();
    drop(old);
    let mut writer = ArtifactWriter::new(&mut ws, ArtifactKind::Result).unwrap();
    writer.append(&summary()).unwrap();
    assert!(matches!(writer.finish(), Err(FailureKind::IoFailure)));
    assert_eq!(
        std::fs::read(dir.join("result.manifest")).unwrap(),
        b"existing"
    );
    assert!(ws.published[1].is_none());
}
