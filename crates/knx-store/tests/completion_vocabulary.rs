//! Scalar completion vocabulary and corruption refusal, without private inputs.
use knx_core::{CompletionStatus, Language, Project};
#[test]
fn documented_completion_statuses_roundtrip_without_substitution() {
    for completion in [
        CompletionStatus::FinishedCommissioning,
        CompletionStatus::Tested,
        CompletionStatus::Locked,
    ] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("native.knxdb");
        let conn = knx_store::open_and_migrate(&path).unwrap();
        let mut project = Project::new(Language("en".into()));
        project.info.completion = completion;
        knx_store::save_project(&conn, &project).unwrap();
        drop(conn);
        let conn = knx_store::open_existing_and_migrate(&path).unwrap();
        assert!(knx_store::load_project(&conn).unwrap() == project);
        knx_store::save_project(&conn, &project).unwrap();
        assert_eq!(
            conn.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
                .unwrap(),
            12
        );
    }
}
#[test]
fn unknown_native_completion_is_refused_instead_of_rewritten() {
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_store::open_and_migrate(&dir.path().join("native.knxdb")).unwrap();
    knx_store::save_project(&conn, &Project::new(Language("en".into()))).unwrap();
    conn.execute(
        "UPDATE project_info SET completion='synthetic-future-status'",
        [],
    )
    .unwrap();
    assert!(knx_store::load_project(&conn).is_err());
    assert_eq!(
        conn.query_row("SELECT completion FROM project_info", [], |r| r
            .get::<_, String>(0))
            .unwrap(),
        "synthetic-future-status"
    );
}
