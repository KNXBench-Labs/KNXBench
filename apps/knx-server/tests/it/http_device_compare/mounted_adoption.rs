//! Real observation → selection → native history → undo at the production mount.
//!
//! Opt-in characterization, not a replacement for the accepted read-only corpus
//! cases. Values come only from simulator memory read by the real App service.

use super::*;
use knx_store::project_history::{load_editor, NativeSnapshot};

fn snapshot(h: &Harness) -> NativeSnapshot {
    let project = h.state.project.lock().unwrap().clone().unwrap();
    let opaque = h.state.opaque.lock().unwrap().clone();
    let manufacturer_refs = h.state.manufacturer_refs.lock().unwrap().clone();
    NativeSnapshot {
        project,
        opaque,
        manufacturer_refs,
    }
}

fn editor(h: &Harness) -> knx_store::project_history::EditorHistory {
    let path = h.state.store_path.lock().unwrap().clone().unwrap();
    let read = knx_store::open_existing_read_only(&path).unwrap();
    load_editor(&read.conn)
        .unwrap()
        .expect("durable editor journal")
}

async fn assert_refused_unchanged(h: &Harness, cookie: &str, payload: Value, kind: &str) {
    let before = snapshot(h);
    let lengths = h.state.command_stack.lock().unwrap().history_lengths();
    let (_, tree) = send(&h.app, get("/api/project")).await;
    let path = h.state.store_path.lock().unwrap().clone().unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let (status, body) = send(
        &h.app,
        browser_post("/api/device-compare/apply", payload, cookie),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["kind"], kind);
    assert_eq!(snapshot(h), before);
    assert_eq!(
        h.state.command_stack.lock().unwrap().history_lengths(),
        lengths
    );
    assert_eq!(send(&h.app, get("/api/project")).await.1, tree);
    assert_eq!(std::fs::read(path).unwrap(), bytes);
}

async fn preview(h: &Harness, cookie: &str, observed: &Value, selected: &Value) -> Value {
    let before = snapshot(h);
    let path = h.state.store_path.lock().unwrap().clone().unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let lengths = h.state.command_stack.lock().unwrap().history_lengths();
    let (status, shown) = send(
        &h.app,
        browser_post(
            "/api/device-compare/preview",
            json!({"observationToken": observed["observationToken"], "selected": [selected]}),
            cookie,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{shown}");
    assert_eq!(shown["changed"], true);
    assert_eq!(shown["selected"], json!([selected]));
    // The entire real observation, not client-authored row/value JSON.
    assert_eq!(shown["rows"], observed["decoded"]);
    assert_eq!(shown["previewToken"].as_str().unwrap().len(), 64);
    assert_eq!(snapshot(h), before);
    assert_eq!(
        h.state.command_stack.lock().unwrap().history_lengths(),
        lengths
    );
    assert_eq!(std::fs::read(path).unwrap(), bytes);
    shown
}

#[tokio::test]
#[ignore = "requires private product corpus and saved K3 project; finite mounted adoption acceptance"]
async fn mounted_observation_adopts_one_row_persists_undo_and_refuses_foreign_consumed_stale() {
    let h = harness(SimulatorConfig::default()).await;
    // Materialize the current source context through the normal save path before
    // testing native compare-and-swap; never mutate the original corpus project.
    let (status, body) = send(&h.app, post("/api/project/save", json!({}))).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (cookie, initial) = compare_session(&h.app).await;
    let expected_address: u32 = std::env::var("KNXBENCH_COMPARISON_BYTE_ADDRESS")
        .expect("supply the independently inspected corpus fixture's unsigned byte field address")
        .parse()
        .unwrap();
    let row = initial["decoded"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| {
            r["kind"] == "parameter"
                && r["address"] == expected_address
                && r["adoptable"] == true
                && r["label"]
                    .as_str()
                    .is_some_and(|s| s.ends_with("bit 0 / 8"))
                && r["project"]
                    .as_str()
                    .and_then(|s| s.parse::<u64>().ok())
                    .is_some_and(|n| n != 37)
                && r["device"] == "0"
        })
        .expect("the corpus fixture's independently readable byte parameter");
    let label = row["label"].clone();
    let address = row["address"].as_u64().unwrap() as u32;
    let original_value = row["project"].clone();
    h.device.preset_memory(address, &[37]);
    // Cookie-bearing request retains the same browser, rather than borrowing a
    // fabricated token or promoting a JSON comparison into an observation.
    let (status, observed) = send(
        &h.app,
        browser_post(
            "/api/device-compare",
            json!({"address": "1.1.67", "gateway": GATEWAY}),
            &cookie,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{observed}");
    let rows: Vec<_> = observed["decoded"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["label"] == label && r["address"] == address)
        .collect();
    assert_eq!(
        rows.len(),
        1,
        "selection must identify exactly one retained row"
    );
    let row = rows[0];
    assert_eq!(row["project"], original_value);
    assert_eq!(row["device"], "37");
    assert_eq!(row["adoptable"], true);
    let selected = row["index"].clone();
    let before = snapshot(&h);
    let lengths = h.state.command_stack.lock().unwrap().history_lengths();
    let generation = editor(&h).generation;
    let shown = preview(&h, &cookie, &observed, &selected).await;
    let payload =
        json!({"previewToken": shown["previewToken"], "confirmation": "AdoptDeviceComparison"});
    let (foreign_cookie, _) = compare_session(&h.app).await;
    assert_ne!(foreign_cookie, cookie);
    assert_refused_unchanged(
        &h,
        &foreign_cookie,
        payload.clone(),
        "deviceComparisonExpired",
    )
    .await;
    let (status, applied) = send(
        &h.app,
        browser_post("/api/device-compare/apply", payload.clone(), &cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{applied}");
    assert_eq!(applied["changed"], true);
    assert_eq!(
        h.state.command_stack.lock().unwrap().history_lengths(),
        (lengths.0 + 1, 0)
    );
    let after = snapshot(&h);
    assert_ne!(after.project, before.project);
    assert_eq!(after.opaque, before.opaque);
    assert_eq!(after.manufacturer_refs, before.manufacturer_refs);
    let saved = editor(&h);
    assert_eq!(
        saved.working, after,
        "native working state exists without a second Save"
    );
    assert!(saved.generation > generation);
    assert_eq!(saved.undo.len(), lengths.0 + 1);
    assert!(saved
        .undo
        .last()
        .unwrap()
        .same_user_content_as(&before.project));
    assert!(saved.redo.is_empty());
    assert_refused_unchanged(&h, &cookie, payload, "deviceComparisonExpired").await;
    let (_, adopted) = compare_session(&h.app).await;
    let adopted_row = adopted["decoded"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["label"] == label && r["address"] == address)
        .unwrap();
    assert_eq!(adopted_row["project"], "37");
    assert_eq!(adopted_row["device"], "37");
    let before_parameters: Vec<_> = observed["decoded"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["kind"] == "parameter")
        .collect();
    let after_parameters: Vec<_> = adopted["decoded"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["kind"] == "parameter")
        .collect();
    assert_eq!(before_parameters.len(), after_parameters.len());
    for (old, new) in before_parameters.into_iter().zip(after_parameters) {
        assert_eq!(old["label"], new["label"]);
        assert_eq!(old["address"], new["address"]);
        if old["index"] != selected {
            assert_eq!(
                old["project"], new["project"],
                "unselected named field changed"
            );
        }
    }
    // Other unread/unselected/raw bytes need not match; do not assert same=true.
    let (status, body) = send(&h.app, post("/api/undo", json!({}))).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let undone = snapshot(&h);
    assert!(undone.project.same_user_content_as(&before.project));
    let mut expected = before.clone();
    expected.project.ids.raise_to(&after.project.ids);
    assert_eq!(
        undone, expected,
        "only allocator high-water marks may remain advanced"
    );
    let saved = editor(&h);
    assert_eq!(saved.working, undone);
    assert_eq!(saved.undo.len(), lengths.0);
    assert_eq!(saved.redo.len(), 1);
    let (cookie, restored) = compare_session(&h.app).await;
    let restored_row = restored["decoded"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["label"] == label && r["address"] == address)
        .unwrap();
    assert_eq!(restored_row["project"], original_value);
    assert_eq!(restored_row["device"], "37");
    let stale = preview(&h, &cookie, &restored, &restored_row["index"]).await;
    let style = if undone.project.info.group_address_style == knx_core::GroupAddressStyle::Free {
        "TwoLevel"
    } else {
        "Free"
    };
    let (status, body) = send(
        &h.app,
        post(
            "/api/project/group-address-style",
            json!({"groupAddressStyle": style}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_ne!(
        snapshot(&h),
        undone,
        "real intervening editor command must execute"
    );
    assert_refused_unchanged(
        &h,
        &cookie,
        json!({"previewToken": stale["previewToken"], "confirmation": "AdoptDeviceComparison"}),
        "deviceCompareStale",
    )
    .await;
    assert!(no_writes(&h.device));
    assert!(!h.device.memory_was_written());
    h.state.catalogue_jobs.close_admission_and_request_stop();
    assert_eq!(
        h.state
            .catalogue_jobs
            .drain(Duration::from_secs(5), std::future::pending())
            .await
            .end,
        knx_server::DrainEnd::Finished
    );
}
