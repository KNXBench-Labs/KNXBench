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
