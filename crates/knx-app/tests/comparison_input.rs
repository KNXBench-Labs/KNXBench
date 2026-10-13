//! Verifies comparison-input normalization for native stores and raw ETS archives.

use knx_app::comparison::{load_comparison_input, ComparisonInputError};
use knx_core::{Language, Project};

#[test]
fn raw_knxproj_returns_a_normalized_project_and_its_import_report() {
    let dir = tempfile::tempdir().unwrap();
    let path = knx_testsupport::write_minimal_knxproj(dir.path());

    let input = load_comparison_input(&path).expect("minimal ETS fixture imports");

    assert_eq!(input.project.info.name, "Minimal");
    let report = input
        .import_report
        .expect("raw ETS input must retain its compatibility report");
    assert_eq!(report.source.schema_version, 11);
}

#[test]
fn native_store_returns_a_project_without_an_import_report() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("project.knxdb");
    let mut project = Project::new(Language("en".into()));
    project.info.name = "Native".into();
    let connection = knx_store::open_and_migrate(&path).unwrap();
    knx_store::save_project(&connection, &project).unwrap();
    drop(connection);

    let input = load_comparison_input(&path).expect("native store loads");

    assert_eq!(input.project.info.name, "Native");
    assert!(input.import_report.is_none());
}

#[test]
fn missing_and_unsupported_inputs_are_explicit_and_create_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("missing.knxdb");
    assert!(matches!(
        load_comparison_input(&missing),
        Err(ComparisonInputError::NotFound(path)) if path == missing
    ));
    assert!(!missing.exists());

    let unsupported = dir.path().join("project.txt");
    std::fs::write(&unsupported, b"not a project").unwrap();
    assert!(matches!(
        load_comparison_input(&unsupported),
        Err(ComparisonInputError::UnsupportedFormat(path)) if path == unsupported
    ));
}

#[test]
fn the_input_kind_is_named_by_the_extension_alone_ignoring_case() {
    use knx_app::comparison::ComparisonInputKind;
    use std::path::Path;

    assert_eq!(
        ComparisonInputKind::of_path(Path::new("a/b.knxdb")),
        Some(ComparisonInputKind::NativeStore)
    );
    assert_eq!(
        ComparisonInputKind::of_path(Path::new("B.KnxProj")),
        Some(ComparisonInputKind::EtsProject)
    );
    assert_eq!(
        ComparisonInputKind::of_path(Path::new("b.knxproj.zip")),
        None
    );
    assert_eq!(ComparisonInputKind::of_path(Path::new("knxproj")), None);
}

/// Every file in `dir` with its bytes, sorted by name: a SQLite journal or
/// WAL sidecar left behind counts as a change just like rewritten bytes.
fn directory_bytes(dir: &std::path::Path) -> Vec<(String, Vec<u8>)> {
    let mut files: Vec<(String, Vec<u8>)> = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            (
                entry.file_name().to_string_lossy().into_owned(),
                std::fs::read(entry.path()).unwrap(),
            )
        })
        .collect();
    files.sort();
    files
}

/// KL-157: comparing against a project saved by an older KNXBench reads it
/// through an in-memory upgrade; the file keeps its bytes, its schema
/// version, its working history and its retained opaque entries.
#[test]
fn an_older_native_store_is_compared_without_upgrading_the_file() {
    for fixture in [
        include_bytes!("../../knx-store/tests/fixtures/native13-before-notes-saved.sqlite")
            .as_slice(),
        include_bytes!("../../knx-store/tests/fixtures/native13-before-notes-working.sqlite")
            .as_slice(),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("older.knxdb");
        std::fs::write(&path, fixture).unwrap();
        let before = directory_bytes(dir.path());

        let input = load_comparison_input(&path).expect("an older native store loads");

        assert!(input.import_report.is_none());
        assert!(!input.project.info.name.is_empty());
        assert_eq!(directory_bytes(dir.path()), before);
    }
}
