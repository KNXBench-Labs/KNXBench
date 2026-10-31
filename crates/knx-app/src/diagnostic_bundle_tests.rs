//! Tests for the diagnostic bundle format: O1 defaults, checks, manifest, ZIP and mail draft.

use std::io::Read;

use super::*;

fn writer() -> WriterIdentity {
    WriterIdentity {
        program: "knx-server".into(),
        version: "0.1.0-alpha.18+gtest".into(),
        commit: Some("0123".into()),
        build_profile: "debug".into(),
    }
}

fn request(problem: ProblemType) -> BundleRequest {
    BundleRequest {
        problem,
        writer: writer(),
        recording_enabled: true,
        consent_at: "2026-10-29T10:00:00Z".into(),
        description: None,
    }
}

fn environment() -> BundleSection {
    let mut section = BundleSection::new(SectionKind::Environment);
    section.files.push(BundleFile::json(
        "environment.json",
        &json!({"os": "linux"}),
    ));
    section
}

fn operation(id: &str, payload: &[u8]) -> BundleSection {
    let mut section = BundleSection::new(SectionKind::Operation);
    section.label = Some(id.into());
    section.entries = 3;
    section.markers.addresses = true;
    section
        .files
        .push(BundleFile::new("recording.jsonl", payload.to_vec()));
    section
}

/// The owner's table O1, transcribed independently of the implementation's
/// layout: (section, [programming, reading, import, catalogue, ui]).
#[test]
fn defaults_follow_owner_table_o1() {
    use Offer::*;
    let table: [(SectionKind, [Offer; 5]); 12] = [
        (SectionKind::Environment, [Fixed; 5]),
        (SectionKind::SessionLog, [Preselected; 5]),
        (
            SectionKind::Operation,
            [Preselected, Preselected, NotOffered, NotOffered, Offered],
        ),
        (
            SectionKind::EarlierOperation,
            [Offered, Offered, NotOffered, NotOffered, Offered],
        ),
        (
            SectionKind::Device,
            [Preselected, Preselected, NotOffered, Offered, Offered],
        ),
        (
            SectionKind::LegacyReports,
            [Preselected, Preselected, NotOffered, NotOffered, NotOffered],
        ),
        (
            SectionKind::ImportReport,
            [NotOffered, NotOffered, Preselected, NotOffered, Offered],
        ),
        (
            SectionKind::ImportFile,
            [NotOffered, NotOffered, Offered, NotOffered, NotOffered],
        ),
        (
            SectionKind::Catalogue,
            [Offered, Offered, NotOffered, Preselected, NotOffered],
        ),
        (
            SectionKind::Project,
            [Offered, Offered, Offered, NotOffered, Offered],
        ),
        (
            SectionKind::BusMonitor,
            [Offered, Offered, NotOffered, NotOffered, Offered],
        ),
        (
            SectionKind::ClientState,
            [NotOffered, NotOffered, NotOffered, NotOffered, Preselected],
        ),
    ];
    for (kind, row) in table {
        for (problem, expected) in ProblemType::ALL.into_iter().zip(row) {
            assert_eq!(
                default_offer(problem, kind),
                expected,
                "{kind:?} for {problem:?}"
            );
        }
    }
    // The whole project is never preselected (decision 4).
    for problem in ProblemType::ALL {
        assert_ne!(
            default_offer(problem, SectionKind::Project),
            Offer::Preselected
        );
    }
}

#[test]
fn the_zip_holds_a_manifest_with_every_file_hash_and_the_files() {
    let sections = vec![environment(), operation("op-1", b"{\"type\":\"header\"}\n")];
    let bytes = build_zip(
        &request(ProblemType::DeviceProgramming),
        &sections,
        "2026-10-29T10:01:00Z",
    )
    .unwrap();
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let names: Vec<String> = archive.file_names().map(str::to_string).collect();
    assert_eq!(
        names,
        [
            "manifest.json",
            "environment/environment.json",
            "operations/op-1/recording.jsonl"
        ]
    );
    let mut manifest = String::new();
    archive
        .by_name("manifest.json")
        .unwrap()
        .read_to_string(&mut manifest)
        .unwrap();
    let manifest: Value = serde_json::from_str(&manifest).unwrap();
    assert_eq!(manifest["format"], BUNDLE_FORMAT);
    assert_eq!(manifest["writer"]["version"], "0.1.0-alpha.18+gtest");
    assert_eq!(manifest["problemType"], "deviceProgramming");
    assert_eq!(manifest["consentAt"], "2026-10-29T10:00:00Z");
    assert_eq!(manifest["recording"]["enabled"], true);
    let file = &manifest["sections"][1]["files"][0];
    assert_eq!(file["path"], "operations/op-1/recording.jsonl");
    assert_eq!(file["bytes"], 18);
    assert_eq!(
        file["sha256"],
        sha256_hex(b"{\"type\":\"header\"}\n").as_str()
    );
    assert_eq!(manifest["sections"][1]["markers"]["addresses"], true);
    let mut recording = Vec::new();
    archive
        .by_name("operations/op-1/recording.jsonl")
        .unwrap()
        .read_to_end(&mut recording)
        .unwrap();
    assert_eq!(recording, b"{\"type\":\"header\"}\n");
}

#[test]
fn a_disabled_recorder_is_stated_in_the_manifest() {
    let mut req = request(ProblemType::DeviceReading);
    req.recording_enabled = false;
    let manifest = manifest(&req, &[environment()], "t");
    assert_eq!(manifest["recording"]["enabled"], false);
    assert!(manifest["recording"]["statement"]
        .as_str()
        .unwrap()
        .contains("disabled"));
}

#[test]
fn sections_outside_the_offer_and_malformed_sets_are_refused() {
    let req = request(ProblemType::ProjectImport);
    // Import problems offer no operation recording.
    assert_eq!(
        build_zip(&req, &[environment(), operation("op-1", b"x")], "t"),
        Err(BundleError::NotOffered(SectionKind::Operation))
    );
    assert_eq!(
        build_zip(&req, &[], "t"),
        Err(BundleError::MissingEnvironment)
    );
    let req = request(ProblemType::DeviceProgramming);
    assert_eq!(
        build_zip(
            &req,
            &[
                environment(),
                operation("op-1", b"x"),
                operation("op-1", b"y")
            ],
            "t"
        ),
        Err(BundleError::Duplicate("operation:op-1".into()))
    );
    for bad in ["../x", "a/b", "..", "", "x y"] {
        assert_eq!(
            build_zip(&req, &[environment(), operation(bad, b"x")], "t"),
            Err(BundleError::BadName(bad.into())),
            "{bad:?}"
        );
    }
    let mut evil = environment();
    evil.files.push(BundleFile::new("../../etc", vec![]));
    assert!(matches!(
        build_zip(&req, &[evil], "t"),
        Err(BundleError::BadName(_))
    ));
}

#[test]
fn preview_counts_sizes_and_shows_text() {
    let section = operation("op-2", b"line one\n");
    let shown = preview(&section);
    assert_eq!(shown.key, "operation:op-2");
    assert_eq!(shown.bytes, 9);
    assert_eq!(shown.entries, 3);
    assert!(shown.markers.addresses && !shown.markers.names);
    let text = section_text(&section);
    assert!(text.contains("operations/op-2/recording.jsonl"));
    assert!(text.contains("line one"));
    let mut binary = BundleSection::new(SectionKind::Project);
    binary
        .files
        .push(BundleFile::new("project.knxdb", vec![0xff, 0xfe, 0x00]));
    assert!(section_text(&binary).contains("<3 octets, not text>"));
}

#[test]
fn the_mail_draft_names_version_and_operation_and_carries_no_content() {
    let draft = mail_draft(
        "0.1.0-alpha.18+g80386a154",
        ProblemType::DeviceProgramming,
        Some("op-20261029T100000000Z-0a1b2c3d"),
        "knxbench-diagnostics.zip",
    );
    assert!(draft.starts_with("mailto:issue@knxbench.com?subject="));
    assert!(draft.contains("0.1.0-alpha.18%2Bg80386a154"));
    assert!(draft.contains("op-20261029T100000000Z-0a1b2c3d"));
    assert!(draft.contains("not%20encrypted"));
    // The draft must not promise less than a bundle with raw recordings holds.
    assert!(draft.contains("possible%20passwords"));
    assert!(draft.contains("Keep%20it%20private"));
    assert!(!draft.contains("never%20included"));
    assert!(!draft.contains("attachment="));
    // Nothing but unreserved characters and escapes after the scheme.
    assert!(draft["mailto:".len()..]
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "-_.~%?=&@".contains(c)));
}

#[test]
fn file_names_are_safe() {
    assert_eq!(
        file_name("0.1.0+gabc", Some("op-1"), "2026-10-29T10:00:00Z"),
        "knxbench-diagnostics-0-1-0-gabc-op-1.zip"
    );
    assert_eq!(
        file_name("0.1.0", None, "2026-10-29T10:00:00Z"),
        "knxbench-diagnostics-0-1-0-20261029100000.zip"
    );
}
