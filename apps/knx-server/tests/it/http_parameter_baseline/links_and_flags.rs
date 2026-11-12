//! HTTP manual marks remove durable link-only evidence in both directions outside undo.

use super::*;
use knx_core::commissioning::parameter_baseline::{
    BaselineChange, LinkBaseline, LinkBaselineChange,
};

fn link_only(state: &knx_server::AppState) {
    state
        .parameter_baselines
        .lock()
        .unwrap()
        .apply(&BaselineEdit {
            device: DeviceId(1),
            parameter: BaselineChange::Remove,
            links: LinkBaselineChange::Record(LinkBaseline {
                program_id: "A-1".into(),
                links: vec![],
                at: chrono::DateTime::UNIX_EPOCH,
            }),
        });
}

#[tokio::test]
async fn both_manual_mark_directions_remove_link_only_evidence_durably_and_undo_never_restores_it()
{
    for programmed in [true, false] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("link-only.knxdb");
        let state = state(None, &[]);
        link_only(&state);
        let (status, body) = call(
            &state,
            "POST",
            "/api/project/save-as",
            Some(json!({"path":path})),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let store = knx_store::open_existing_read_only(&path).unwrap();
        let all = knx_store::project_history::load_parameter_baselines(&store.conn).unwrap();
        assert!(all.get(DeviceId(1)).is_none());
        assert!(all.get_links(DeviceId(1)).is_some());
        drop(store);
        let shown = tree(&state).await;
        let (status, body) = call(
            &state,
            "POST",
            "/api/device/1/programming",
            Some(mark(&shown, programmed)),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert!(state.parameter_baselines.lock().unwrap().is_empty());
        let (status, body) = call(&state, "POST", "/api/undo", None).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert!(state.parameter_baselines.lock().unwrap().is_empty());
        drop(state);
        let again = Arc::new(knx_server::AppState::default());
        let (status, body) = call(
            &again,
            "POST",
            "/api/project/open",
            Some(json!({"path":path})),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert!(again.parameter_baselines.lock().unwrap().is_empty());
        let (status, body) = call(&again, "GET", "/api/device/1", None).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["last_download"]["links"]["state"], "absent");
        assert_eq!(body["last_download"]["flags"]["state"], "absent");
    }
}
