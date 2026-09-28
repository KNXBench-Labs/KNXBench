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
