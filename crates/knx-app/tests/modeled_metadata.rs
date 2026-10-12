//! Existing modeled project information is inspectable without guessing raw source semantics.

use knx_app::modeled_metadata;
use knx_core::commissioning::CompletionStatus;

#[test]
fn exact_modeled_fields_keep_number_absence_distinct_from_explicit_empty() {
    let mut project = knx_core::Project::new(knx_core::Language("en".into()));
    project.info.project_id = "P-FICTION".into();
    project.info.name = "  Fictional <project>  ".into();
    project.info.completion = CompletionStatus::FinishedCommissioning;
    project.info.ets_schema_version = 23;
    project.info.last_modified = Some("2010-01-02T03:04:05Z".parse().unwrap());
    project.info.project_start = Some("2010-01-01T00:00:00Z".parse().unwrap());
    project.info.unlifted_group_address_dpt_declarations = 3;
    let before = project.clone();
    let view =
        modeled_metadata::project(&project).expect("normalized project metadata not inspectable");
    assert_eq!(view.view_version, 1);
    assert_eq!(view.scope, "normalizedProject");
    assert_eq!(view.model_schema_version, project.schema_version);
    assert_eq!(view.name, project.info.name);
    assert_eq!(view.project_id, "P-FICTION");
    assert_eq!(view.project_number, None);
    assert_eq!(view.completion, "FinishedCommissioning");
    assert_eq!(view.ets_schema_version, 23);
    assert_eq!(
        view.last_modified.as_deref(),
        Some("2010-01-02T03:04:05+00:00")
    );
    assert_eq!(
        view.project_start.as_deref(),
        Some("2010-01-01T00:00:00+00:00")
    );
    assert_eq!(view.unlifted_group_address_dpt_declarations, 3);
    assert_eq!(project, before);
    project.info.project_number = Some(String::new());
    assert_eq!(
        modeled_metadata::project(&project).unwrap().project_number,
        Some(String::new())
    );
}
#[test]
fn exact_text_boundary_is_admitted_and_combined_fields_charge_the_same_budget() {
    let mut project = knx_core::Project::new(knx_core::Language("en".into()));
    project.info.name = "x".repeat(modeled_metadata::MAX_TEXT_BYTES);
    assert_eq!(
        modeled_metadata::project(&project).unwrap().name,
        project.info.name
    );
    project.info.project_number = Some("x".into());
    assert!(modeled_metadata::project(&project).is_err());
}

#[test]
fn excessive_text_refuses_without_truncating_or_mutating_the_project() {
    let mut project = knx_core::Project::new(knx_core::Language("en".into()));
    project.info.name = "x".repeat(modeled_metadata::MAX_TEXT_BYTES + 1);
    let before = project.clone();
    assert!(modeled_metadata::project(&project).is_err());
    assert_eq!(project, before);
}
#[test]
fn modeled_information_roundtrips_through_actual_native_storage_without_becoming_a_save_time() {
    let mut project = knx_core::Project::new(knx_core::Language("en".into()));
    project.info.name = "Fictional stored name".into();
    project.info.project_number = Some("".into());
    project.info.last_modified = Some("2010-01-02T03:04:05Z".parse().unwrap());
    let expected = serde_json::to_value(modeled_metadata::project(&project).unwrap()).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fictional.knxdb");
    let conn = knx_store::open_and_migrate(&path).unwrap();
    knx_store::save_project(&conn, &project).unwrap();
    drop(conn);
    let conn = knx_store::open_existing_and_migrate(&path).unwrap();
    let reopened = knx_store::load_project(&conn).unwrap();
    assert_eq!(reopened, project);
    assert_eq!(
        serde_json::to_value(modeled_metadata::project(&reopened).unwrap()).unwrap(),
        expected
    );
}
