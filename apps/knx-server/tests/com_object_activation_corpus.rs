//! Pins the corpus projects' evaluated com-object activation and channel counts (ISSUE-08).
//!
//! Imports each reference project through the HTTP API into a fresh product
//! database (the import installs the embedded products), then reads every
//! device's detail with `language=de-DE` and counts `activation` states and
//! `channel` ownership. Aggregates only: no names, ids or texts of the
//! private corpus are stored here.
//!
//! What the counts say, per project:
//!
//! - Every object the project stores is `Active` and owned by a channel
//!   element; the evaluation agrees with ETS's own stored activity.
//! - No channel text is invented: an element with an empty `@Text` stays
//!   `None`, which is every channel of the two house exports.
//! - A module-based object is attributed to its own module instance.

use std::collections::{BTreeMap, HashSet};
use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

/// Aggregate counts over every communication object of one project.
async fn counts(path: std::path::PathBuf) -> BTreeMap<&'static str, usize> {
    let dir = tempfile::tempdir().unwrap();
    let products = knx_productdb::open_and_migrate(&dir.path().join("p.sqlite")).unwrap();
    let state = Arc::new(knx_server::AppState {
        product_db: Some(Mutex::new(products)),
        ..Default::default()
    });
    let app = knx_server::app(Arc::clone(&state), None);

    let imported = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/project/import")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "path": path.to_string_lossy() }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(imported.status(), StatusCode::OK);

    let device_ids: Vec<u32> = {
        let project = state.project.lock().unwrap();
        project
            .as_ref()
            .unwrap()
            .devices
            .iter()
            .map(|d| d.id.0)
            .collect()
    };

    let mut c: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut inc = |k: &'static str| *c.entry(k).or_insert(0) += 1;
    for id in device_ids {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/device/{id}?language=de-DE"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let detail = body_json(response).await;
        let mut channel_keys: HashSet<String> = HashSet::new();
        for com in detail["com_objects"].as_array().unwrap() {
            inc("objects");
            match com["activation"].as_str().unwrap() {
                "Active" => inc("active"),
                "Inactive" => inc("inactive"),
                "Undetermined" => inc("undetermined"),
                "NotEvaluated" => inc("not_evaluated"),
                other => panic!("unknown activation {other}"),
            }
            let channel = &com["channel"];
            if channel.is_null() {
                if com["activation"] == "Active" {
                    inc("active_without_channel");
                }
                continue;
            }
            assert_eq!(
                com["activation"], "Active",
                "only an active object may carry a channel"
            );
            inc("with_channel");
            match channel["kind"].as_str().unwrap() {
                "Channel" => inc("in_channel"),
                "ChannelIndependentBlock" => inc("in_channel_independent_block"),
                other => panic!("unknown channel kind {other}"),
            }
            match channel["text"].as_str() {
                None => inc("channel_without_text"),
                Some(text) if text.contains("{{") => inc("channel_text_with_placeholder"),
                Some(_) => {}
            }
            channel_keys.insert(channel["key"].as_str().unwrap().to_string());
        }
        for _ in 0..channel_keys.len() {
            inc("channels");
        }
    }
    c
}

#[tokio::test]
#[ignore = "requires the gitignored OriginalData/ corpus; run with --ignored"]
async fn corpus_com_object_activation_and_channels() {
    assert!(
        knx_testsupport::corpus_available(),
        "OriginalData/ corpus not present (gitignored, local-only); this test is #[ignore]d and must be run explicitly on a machine that has it"
    );
    let map = |pairs: &[(&'static str, usize)]| pairs.iter().copied().collect::<BTreeMap<_, _>>();

    // Every object the house exports store is one ETS itself stored as
    // active, and the evaluation agrees on each (907 + 867). Every one sits
    // in a `Channel` element, and every such element in these MDT programs
    // has an empty `@Text`, so there is no channel text to show: the UI
    // needs its generic fallback here, not a heuristic.
    assert_eq!(
        counts(knx_testsupport::reference_ets4_path()).await,
        map(&[
            ("active", 907),
            ("channel_without_text", 907),
            ("channels", 34),
            ("in_channel", 907),
            ("objects", 907),
            ("with_channel", 907),
        ])
    );
    assert_eq!(
        counts(knx_testsupport::reference_ets6_path()).await,
        map(&[
            ("active", 867),
            ("channel_without_text", 867),
            ("channels", 33),
            ("in_channel", 867),
            ("objects", 867),
            ("with_channel", 867),
        ])
    );
    // KV: all 75 objects are module-based, every module instance is told
    // apart, and every channel text is present and fully substituted.
    assert_eq!(
        counts(knx_testsupport::reference_kv_schema21_path()).await,
        map(&[
            ("active", 75),
            ("channels", 32),
            ("in_channel", 75),
            ("objects", 75),
            ("with_channel", 75),
        ])
    );
}
