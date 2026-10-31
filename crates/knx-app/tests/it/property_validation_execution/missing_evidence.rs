//! Missing allocation bytes prove only absence, even for a newly written checkpoint.
use super::*;
use knx_app::property_validation::{FinalStepCategory, FinalStepRecord, FinalStepStage};
use serde_json::json;

#[tokio::test]
async fn a_current_missing_evidence_checkpoint_reports_not_recorded_before_and_after_reopen() {
    let (bundle, recovery) = executed_record().await;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("op");
    let mut store = store_at_step(&path, &bundle, recovery, 7);
    store
        .checkpoint(Checkpoint::Failed {
            code: app::FailureCode::IdentityOrRecovery,
            local_diagnostic: "synthetic current caller omitted evidence".into(),
            pointer_failure: None,
            final_step: Some(FinalStepRecord {
                category: FinalStepCategory::VerificationMismatch,
                stage: FinalStepStage::Allocation,
            }),
            allocation_evidence: None,
        })
        .unwrap();
    let expected = json!([{"run": "download", "evidence": {
        "availability": "unavailable", "reason": "notRecorded"
    }}]);
    // Freeze both observations before asserting, so even the original RED reads both.
    let immediate = serde_json::to_value(store.allocation_mismatch_evidence()).unwrap();
    drop(store);
    let reopened = OperationStore::open(&path, &bundle.binding).unwrap();
    let durable = serde_json::to_value(reopened.allocation_mismatch_evidence()).unwrap();
    assert_eq!((immediate, durable), (expected.clone(), expected));
}
