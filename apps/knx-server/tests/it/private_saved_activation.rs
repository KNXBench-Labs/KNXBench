//! Opt-in private saved-state versus evaluated-activation differential (KL-146).
//!
//! Synthetic controls run in ordinary CI. The original gate requires explicit
//! opt-in and emits only boolean assertions; its receipt remains ignored/local.
//! Source ChannelId is a program-channel reference, not a ChannelInstance Id.
//! Missing channel observations do not establish an inactive channel.

#[path = "../common/saved_activation.rs"]
mod saved_activation;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use saved_activation::{
    channel_claim_matches, compare_activity, diagnostic_observed_agreement, diagnostic_report,
    saved_devices, Comparison, SavedDevice,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tower::ServiceExt;

// Harness-only compressed-input budget; not the selective-import limit or a
// change to production admission. XML and response limits remain separate.
const SOURCE_INPUT_LIMIT: u64 = 1024 * 1024 * 1024;
const RESPONSE_LIMIT: usize = 64 * 1024 * 1024;

fn read_source(path: &Path) -> Vec<u8> {
    let file = std::fs::File::open(path)
        .unwrap_or_else(|_| panic!("refusal: configured private source is unreadable"));
    let mut bytes = Vec::new();
    file.take(SOURCE_INPUT_LIMIT + 1)
        .read_to_end(&mut bytes)
        .unwrap_or_else(|_| panic!("refusal: configured private source read failed"));
    assert!(
        bytes.len() as u64 <= SOURCE_INPUT_LIMIT,
        "refusal: private input budget exceeded"
    );
    bytes
}

fn source_devices(bytes: Vec<u8>) -> BTreeMap<String, SavedDevice> {
    let mut container = knx_etsproj::Container::open(bytes)
        .unwrap_or_else(|_| panic!("refusal: private container validation failed"));
    let names: Vec<_> = container
        .entries()
        .iter()
        .filter_map(|e| {
            let (top, rest) = e.path.split_once('/')?;
            let stem = rest.strip_suffix(".xml")?;
            (top.starts_with("P-") && !stem.is_empty() && stem.bytes().all(|b| b.is_ascii_digit()))
                .then(|| e.path.clone())
        })
        .collect();
    assert!(
        !names.is_empty(),
        "refusal: no installation XML was selected"
    );
    let mut result = BTreeMap::new();
    for name in names {
        let xml = container
            .read(&name)
            .unwrap_or_else(|_| panic!("refusal: private installation member read failed"));
        let devices = saved_devices(&xml)
            .unwrap_or_else(|_| panic!("refusal: private saved-state source census failed"));
        for (id, device) in devices {
            assert!(
                result.insert(id, device).is_none(),
                "duplicate source device across installations"
            );
        }
    }
    assert!(
        !result.is_empty(),
        "refusal: no source device was exercised"
    );
    result
}

async fn get_json(app: &axum::Router, uri: &str) -> Value {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(uri)
                .body(Body::empty())
                .unwrap_or_else(|_| panic!("private request construction failed")),
        )
        .await
        .unwrap_or_else(|_| panic!("private request execution failed"));
    assert!(
        response.status() == StatusCode::OK,
        "private read projection was refused"
    );
    let body = axum::body::to_bytes(response.into_body(), RESPONSE_LIMIT)
        .await
        .unwrap_or_else(|_| panic!("private response budget/read failed"));
    serde_json::from_slice(&body)
        .unwrap_or_else(|_| panic!("private response is not structured JSON"))
}

fn mark(report: &mut BTreeMap<&'static str, bool>, prefix: bool, outcome: Comparison) {
    let key = match (prefix, outcome) {
        (false, Comparison::Agreement) => "object_agreement_seen",
        (false, Comparison::Difference) => "object_difference_seen",
        (false, Comparison::Undetermined) => "object_undetermined_seen",
        (false, Comparison::NotEvaluated) => "object_not_evaluated_seen",
        (true, Comparison::Agreement) => "channel_activity_agreement_seen",
        (true, Comparison::Difference) => "channel_activity_difference_seen",
        (true, Comparison::Undetermined) => "channel_activity_undetermined_seen",
        (true, Comparison::NotEvaluated) => "channel_activity_not_evaluated_seen",
    };
    report.insert(key, true);
}

#[tokio::test]
#[ignore = "requires KNXBENCH_PRIVATE_SCHEMA23_PROJECT; private read-only differential"]
async fn private_saved_activation_compares_every_imported_tree_object_without_rounding_unknown_states(
) {
    let evidence = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../.ai/logs/private-import-analysis/kl146-final");
    std::fs::create_dir_all(&evidence)
        .unwrap_or_else(|_| panic!("private evidence directory is unavailable"));
    let receipt = evidence.join("differential.private.json");
    for stale in [&receipt, &evidence.join("phase.private.json")] {
        if stale.exists() {
            std::fs::remove_file(stale)
                .unwrap_or_else(|_| panic!("stale private evidence cannot be invalidated"));
        }
    }
    let path = std::env::var_os("KNXBENCH_PRIVATE_SCHEMA23_PROJECT")
        .map(PathBuf::from)
        .unwrap_or_else(|| panic!("refusal: private-project opt-in variable is required"));
    assert!(
        path.is_absolute() && path.is_file(),
        "refusal: configured absolute private project is unavailable"
    );
    let original = read_source(&path);
    let input_digest = Sha256::digest(&original);
    let source = source_devices(original);
    std::fs::write(
        evidence.join("phase.private.json"),
        b"{\"source_census_complete\":true}",
    )
    .unwrap_or_else(|_| panic!("private phase receipt failed"));
    let dir = tempfile::Builder::new()
        .prefix("activation-")
        .tempdir_in(&evidence)
        .unwrap_or_else(|_| panic!("private database workspace is unavailable"));
    let products = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite"))
        .unwrap_or_else(|_| panic!("private product database cannot be initialized"));
    let state = Arc::new(knx_server::AppState {
        product_db: Some(
            knx_server::CatalogueOwner::adopt_opened(products, std::path::PathBuf::new()).unwrap(),
        ),
        ..Default::default()
    });
    let app = knx_server::app(Arc::clone(&state), None);
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/project/import")
                .header("content-type", "application/json")
                .body(Body::from(json!({"path": path}).to_string()))
                .unwrap_or_else(|_| panic!("private import request construction failed")),
        )
        .await
        .unwrap_or_else(|_| panic!("private import request execution failed"));
    assert!(
        response.status() == StatusCode::OK,
        "private production import was refused"
    );
    std::fs::write(
        evidence.join("phase.private.json"),
        b"{\"production_import_complete\":true}",
    )
    .unwrap_or_else(|_| panic!("private phase receipt failed"));
    let before = get_json(&app, "/api/project").await;
    let project_rows: Vec<_> = {
        let guard = state
            .project
            .lock()
            .unwrap_or_else(|_| panic!("private project lock failed"));
        let project = guard
            .as_ref()
            .unwrap_or_else(|| panic!("private import published no project"));
        project
            .devices
            .iter()
            .map(|device| {
                let objects: Vec<_> = device
                    .com_objects
                    .iter()
                    .map(|id| {
                        let object = project
                            .devices
                            .com_object(*id)
                            .unwrap_or_else(|| panic!("private device lists an absent object"));
                        (
                            id.0,
                            object.source.ets_id.clone(),
                            object.is_active,
                            object.module_instance.is_some(),
                        )
                    })
                    .collect();
                (
                    device.id,
                    device.source.ets_id.clone(),
                    device.program_ref.clone(),
                    objects,
                )
            })
            .collect()
    };
    assert!(
        project_rows.len() == source.len(),
        "imported device identity is not unique"
    );
    assert!(
        project_rows
            .iter()
            .map(|r| r.1.clone())
            .collect::<BTreeSet<_>>()
            == source.keys().cloned().collect(),
        "source and imported devices differ"
    );
    let mut report = diagnostic_report();
    let mut exercised = false;
    for (id, source_id, program_ref, objects) in project_rows {
        let saved = source
            .get(&source_id)
            .unwrap_or_else(|| panic!("imported device has no source owner"));
        assert!(
            objects.iter().map(|o| o.1.clone()).collect::<BTreeSet<_>>() == saved.objects,
            "source and imported tree object sets differ"
        );
        assert!(
            objects.len() == saved.objects.len(),
            "imported object identity is not unique"
        );
        if saved.channel_ids.keys().any(|r| !saved.objects.contains(r)) {
            report.insert("source_override_outside_tree_seen", true);
        }
        let detail = get_json(&app, &format!("/api/device/{}?language=de-DE", id.0)).await;
        let rows = detail["com_objects"]
            .as_array()
            .unwrap_or_else(|| panic!("private object projection is absent"));
        let mut observed = BTreeSet::new();
        let mut active_unscoped = Vec::new();
        for row in rows {
            let internal = row["id"]
                .as_u64()
                .and_then(|n| u32::try_from(n).ok())
                .unwrap_or_else(|| panic!("private projected identity is invalid"));
            assert!(
                observed.insert(internal),
                "duplicate private projected object"
            );
            let object = objects.iter().find(|o| o.0 == internal).unwrap_or_else(|| {
                panic!("private projection has an object outside its source set")
            });
            assert!(
                row["is_active"].as_bool() == Some(object.2) && object.2,
                "stored tree activity was replaced or misread"
            );
            let activation = row["activation"]
                .as_str()
                .unwrap_or_else(|| panic!("private evaluated activity is absent"));
            let outcome = compare_activity(true, activation)
                .unwrap_or_else(|_| panic!("private evaluated activity token is unknown"));
            mark(&mut report, false, outcome);
            exercised = true;
            if let Some(channel_id) = saved.channel_ids.get(&object.1) {
                report.insert("channel_id_claim_seen", true);
                if !object.3 && outcome == Comparison::Agreement && activation == "Active" {
                    active_unscoped.push((object.1.clone(), channel_id.clone()));
                } else {
                    report.insert("channel_id_not_adjudicated_seen", true);
                }
            }
        }
        assert!(
            observed == objects.iter().map(|o| o.0).collect(),
            "not every imported object was compared"
        );
        let guard = state
            .project
            .lock()
            .unwrap_or_else(|_| panic!("private project lock failed"));
        let project = guard
            .as_ref()
            .unwrap_or_else(|| panic!("private project vanished"));
        let instances: Vec<_> = project
            .devices
            .module_instances()
            .filter(|m| m.device == id)
            .cloned()
            .collect();
        let stored: Vec<_> = project
            .installations
            .iter()
            .flat_map(|i| &i.parameters)
            .filter(|p| p.device == id)
            .map(|p| (p.source.ets_id.clone(), p.raw.clone()))
            .collect();
        let resolved = state
            .product_db
            .as_ref()
            .unwrap_or_else(|| panic!("private products vanished"))
            .with_read(|products| {
                let program = knx_productdb::query::resolve_program(products, &program_ref)
                    .unwrap_or_else(|_| panic!("private program resolution failed"));
                Ok::<_, std::convert::Infallible>(program.map(|program| {
                    let evaluated = knx_productdb::device_evaluation::evaluate_device(
                        products, &program, stored, &instances,
                    )
                    .unwrap_or_else(|_| panic!("private channel evaluation was refused"));
                    (program, evaluated)
                }))
            })
            .unwrap_or_else(|_| panic!("private product database lock failed"))
            .unwrap_or_else(|_| panic!("private product database lock failed"));
        if let Some((program, evaluated)) = resolved {
            if saved.tree_present {
                let expected_unscoped: BTreeSet<_> = objects
                    .iter()
                    .filter(|o| !o.3)
                    .map(|o| knx_productdb::com_object_lookup_id(&program, &o.1, false))
                    .collect();
                if evaluated
                    .activation
                    .com_object_refs
                    .iter()
                    .any(|r| r.scope.is_none() && !expected_unscoped.contains(&r.ref_id))
                {
                    report.insert("extra_unscoped_activation_seen", true);
                }
            } else {
                report.insert("source_tree_absent_seen", true);
            }
            if evaluated
                .activation
                .com_object_refs
                .iter()
                .any(|r| r.scope.is_some())
                || objects.iter().any(|o| o.3)
            {
                report.insert("scoped_set_not_adjudicated_seen", true);
            }
            for (object, expected_channel) in active_unscoped {
                let lookup = knx_productdb::com_object_lookup_id(&program, &object, false);
                let hit = evaluated
                    .activation
                    .com_object_refs
                    .iter()
                    .find(|r| r.scope.is_none() && r.ref_id == lookup)
                    .unwrap_or_else(|| {
                        panic!("HTTP and shared evaluator active-object claims differ")
                    });
                let actual = hit.channel.as_ref().and_then(|c| c.element_id.as_deref());
                report.insert(
                    if channel_claim_matches(&program, &expected_channel, actual) {
                        "channel_id_agreement_seen"
                    } else {
                        "channel_id_difference_seen"
                    },
                    true,
                );
            }
            for (_, reference, stated) in &saved.channels {
                if let Some(stated) = stated {
                    report.insert("explicit_channel_activity_seen", true);
                    let qualified = reference.as_ref().map(|r| {
                        if r.starts_with(&format!("{program}_")) {
                            r.clone()
                        } else {
                            format!("{program}_{r}")
                        }
                    });
                    let witnessed = qualified.as_ref().is_some_and(|r| {
                        evaluated
                            .activation
                            .parameter_refs
                            .iter()
                            .chain(&evaluated.activation.com_object_refs)
                            .filter_map(|a| a.channel.as_ref())
                            .any(|c| c.scope.is_none() && c.element_id.as_ref() == Some(r))
                    });
                    // A positive channel owner is observable. Absence never proves inactivity.
                    mark(
                        &mut report,
                        true,
                        if witnessed {
                            compare_activity(*stated, "Active")
                                .unwrap_or_else(|_| panic!("channel comparison failed"))
                        } else {
                            Comparison::NotEvaluated
                        },
                    );
                } else {
                    mark(&mut report, true, Comparison::NotEvaluated);
                }
            }
        } else if !saved.channels.is_empty() {
            mark(&mut report, true, Comparison::NotEvaluated);
        }
    }
    std::fs::write(
        evidence.join("phase.private.json"),
        b"{\"comparison_complete\":true}",
    )
    .unwrap_or_else(|_| panic!("private phase receipt failed"));
    let after = get_json(&app, "/api/project").await;
    report.insert("project_snapshot_unchanged", before == after);
    report.insert(
        "input_unchanged",
        input_digest == Sha256::digest(read_source(&path)),
    );
    report.insert("nonempty_complete_object_comparison", exercised);
    assert!(exercised, "refusal: no saved-tree object was compared");
    assert!(
        report["project_snapshot_unchanged"] && report["input_unchanged"],
        "read-only input/project invariant failed"
    );
    assert!(
        report["object_agreement_seen"]
            || report["object_undetermined_seen"]
            || report["object_difference_seen"],
        "refusal: no actual evaluation was observed"
    );
    let observed_agreement = diagnostic_observed_agreement(&report)
        .unwrap_or_else(|_| panic!("diagnostic completeness/integrity validation failed"));
    // Owner-approved diagnostic delivery. Differences remain in the receipt;
    // they are not a test failure and do not change production activation.
    report.insert("comparison_completed", true);
    report.insert("observed_scope_agrees", observed_agreement);
    report.insert("parity_accepted", false);
    let encoded = serde_json::to_vec_pretty(&report)
        .unwrap_or_else(|_| panic!("private receipt serialization failed"));
    std::fs::write(receipt, encoded)
        .unwrap_or_else(|_| panic!("private receipt publication failed"));
}
