//! R4 route witnesses bind selected recordings while ambient producer state changes.

#[path = "r4_review_binding/support.rs"]
mod support;

use axum::http::StatusCode;
use knx_app::operation_recording::RecordedOutcome;
use serde_json::json;

use support::*;

#[tokio::test]
async fn preexisting_unselected_b_growth_and_finish_preserve_pinned_a_c_review() {
    let fixture = PinnedFixture::new();
    let b = fixture.start_ambient("B");
    let b_id = b.id().unwrap().to_string();
    let (b_initial, b_initial_bytes) =
        wait_for_recording(&fixture.state, &b_id, |summary, bytes| {
            summary.active && summary.outcome.is_none() && !bytes.is_empty()
        })
        .await;

    let initial = preview_ok(&fixture.state, &fixture.query).await;
    assert_chooser_contains(&initial, &b_id);
    let pinned = pinned_projection(&initial, &fixture);
    assert_reviewed_section_texts(&fixture, &initial).await;
    let (status, body) = export_response(&fixture, &initial).await;
    let baseline_export = exported_snapshot(status, body, &initial, &fixture, &[&b_id]);

    let growth_marker = "R4_UNSELECTED_B_APPEND_MARKER";
    b.note(
        "ambient-growth",
        json!({"marker": growth_marker, "payload": "b".repeat(2048)}),
    );
    let (b_grown, b_grown_bytes) = wait_for_recording(&fixture.state, &b_id, |summary, bytes| {
        summary.active
            && summary.outcome.is_none()
            && summary.size_bytes > b_initial.size_bytes
            && bytes
                .windows(growth_marker.len())
                .any(|window| window == growth_marker.as_bytes())
    })
    .await;
    assert!(b_grown.size_bytes > b_initial.size_bytes);
    assert!(b_grown_bytes.len() > b_initial_bytes.len());

    let after_growth = preview_ok(&fixture.state, &fixture.query).await;
    assert_eq!(fingerprint(&after_growth), fingerprint(&initial));
    assert_eq!(pinned_projection(&after_growth, &fixture), pinned);
    assert_reviewed_section_texts(&fixture, &initial).await;
    let (status, body) = export_response(&fixture, &initial).await;
    let grown_export = exported_snapshot(status, body, &after_growth, &fixture, &[&b_id]);
    assert_same_selected_export(&baseline_export, &grown_export);

    b.finish(RecordedOutcome::succeeded(json!({"ambient": "B"})));
    let (b_finished, b_finished_bytes) =
        wait_for_recording(&fixture.state, &b_id, |summary, bytes| {
            !summary.active
                && summary.outcome.as_deref() == Some("succeeded")
                && summary.size_bytes > b_grown.size_bytes
                && bytes
                    .windows(b"\"type\":\"end\"".len())
                    .any(|window| window == b"\"type\":\"end\"")
        })
        .await;
    assert!(b_finished.size_bytes > b_grown.size_bytes);
    assert!(b_finished_bytes.len() > b_grown_bytes.len());

    let after_finish = preview_ok(&fixture.state, &fixture.query).await;
    assert_eq!(fingerprint(&after_finish), fingerprint(&initial));
    assert_eq!(pinned_projection(&after_finish, &fixture), pinned);
    assert_reviewed_section_texts(&fixture, &initial).await;
    let (status, body) = export_response(&fixture, &initial).await;
    let finished_export = exported_snapshot(status, body, &after_finish, &fixture, &[&b_id]);
    assert_same_selected_export(&baseline_export, &finished_export);

    eprintln!(
        "R4 old-B control: id={b_id} sizes={}->{}->{} active={}->{} outcome={:?} timestamps={:?}/{:?}/{:?}",
        b_initial.size_bytes,
        b_grown.size_bytes,
        b_finished.size_bytes,
        b_initial.active,
        b_finished.active,
        b_finished.outcome,
        baseline_export.timestamps,
        grown_export.timestamps,
        finished_export.timestamps,
    );
}

#[tokio::test]
async fn new_unselected_d_preserves_pinned_a_c_review_section_text_and_export() {
    let fixture = PinnedFixture::new();
    let b_id = fixture.complete_ambient("B");
    let initial = preview_ok(&fixture.state, &fixture.query).await;
    assert_chooser_contains(&initial, &b_id);
    let pinned = pinned_projection(&initial, &fixture);
    assert_reviewed_section_texts(&fixture, &initial).await;
    let (baseline_status, baseline_body) = export_response(&fixture, &initial).await;
    let baseline_export =
        exported_snapshot(baseline_status, baseline_body, &initial, &fixture, &[&b_id]);

    let d_id = fixture.complete_ambient("D");
    assert!(!chooser_ids(&initial).contains(&d_id));
    let after_d = preview_ok(&fixture.state, &fixture.query).await;
    assert_chooser_contains(&after_d, &b_id);
    assert_chooser_contains(&after_d, &d_id);
    assert_eq!(pinned_projection(&after_d, &fixture), pinned);

    let (after_status, after_body) = export_response(&fixture, &initial).await;
    let changed_marker = String::from_utf8_lossy(&after_body).contains("bundlePreviewChanged");
    eprintln!(
        "R4 new-D discriminator: before={} after={} export_status={} marker=bundlePreviewChanged:{}",
        fingerprint(&initial),
        fingerprint(&after_d),
        after_status,
        changed_marker,
    );

    assert_eq!(
        fingerprint(&after_d),
        fingerprint(&initial),
        "an unselected newly ambient D must not change the selected A/C review"
    );
    assert_eq!(
        after_status,
        StatusCode::OK,
        "original review was refused after D appeared: {}",
        super::json_of(&after_body)
    );

    assert_reviewed_section_texts(&fixture, &initial).await;
    let after_export = exported_snapshot(
        after_status,
        after_body,
        &after_d,
        &fixture,
        &[&b_id, &d_id],
    );
    assert_same_selected_export(&baseline_export, &after_export);
    eprintln!(
        "R4 new-D timestamps observed without normalization: before={:?} after={:?}",
        baseline_export.timestamps, after_export.timestamps
    );
}

#[tokio::test]
async fn selecting_ambient_b_or_d_changes_the_real_review_and_exports_fresh_runs() {
    let fixture = PinnedFixture::new();
    let b_id = fixture.complete_ambient("B");
    let d_id = fixture.complete_ambient("D");
    let baseline = preview_ok(&fixture.state, &fixture.query).await;

    for selected in [&b_id, &d_id] {
        let query = pinned_query(selected, &fixture.c_id);
        let selected_preview = preview_ok(&fixture.state, &query).await;
        assert_ne!(fingerprint(&selected_preview), fingerprint(&baseline));
        let (stale_status, stale_body) =
            export_response_for(&fixture.state, &query, &baseline).await;
        assert_eq!(stale_status, StatusCode::CONFLICT);
        assert!(String::from_utf8_lossy(&stale_body).contains("bundlePreviewChanged"));
        let (status, body) = export_response_for(&fixture.state, &query, &selected_preview).await;
        assert_eq!(
            status,
            StatusCode::OK,
            "fresh valid selected run {selected}: {}",
            super::json_of(&body)
        );
        assert!(body.starts_with(b"PK"), "fresh selected run is a ZIP");
    }
}
