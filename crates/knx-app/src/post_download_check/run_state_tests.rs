//! Run-state evidence in the shared post-download record: mapping, admission and refusal.

use knx_core::commissioning::load_state::LoadState;
use knx_core::commissioning::properties::PID_OBJECT_TYPE;
use knx_core::commissioning::properties::PID_RUN_STATE_CONTROL;
use knx_core::commissioning::run_state::RunState;
use knx_core::IndividualAddress;
use knx_net::commissioning::post_download::{
    CandidateMismatch, LoadStateObservation, LoadStateReading, MatchedIdentity, RunStateCause,
    RunStateNotRead, RunStateObservation, RunStateStep,
};

use super::*;

fn observation(
    loads: Vec<LoadStateObservation>,
    run_state: RunStateObservation,
) -> PostDownloadObservation {
    PostDownloadObservation {
        target: IndividualAddress::from_raw(0x1105),
        outcome: ObservationOutcome::Observed {
            identity: MatchedIdentity {
                mask: MaskVersion(0x0701),
                manufacturer: 0x0083,
                properties: vec![],
                application: APPLICATION,
            },
            load_states: loads
                .into_iter()
                .zip([AddressTable, ApplicationProgram])
                .map(|(state, machine)| LoadStateReading {
                    machine,
                    expected: Some(LoadState::Loaded),
                    state,
                })
                .collect(),
            run_state,
        },
    }
}

fn read_loads() -> Vec<LoadStateObservation> {
    vec![
        LoadStateObservation::Observed(LoadState::Loaded),
        LoadStateObservation::Observed(LoadState::Loaded),
    ]
}

#[test]
fn exact0700_description_vocabulary_requires_provenance_and_strict_pdt() {
    let base = json_of(&PostDownloadCheck::from_observation(&observation(
        read_loads(),
        RunStateObservation::UndefinedValue,
    )));
    let with = |record: Value, marker: Option<Value>| {
        let mut value = base.clone();
        let matched = value["observation"]["reach"]["identity"]
            .as_object_mut()
            .unwrap();
        matched.insert("runState".into(), record);
        if let Some(marker) = marker {
            matched.insert("runStateMask".into(), marker);
        }
        value
    };
    let admit = |value: Value| {
        let check: PostDownloadCheck = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(serde_json::to_value(check).unwrap(), value);
    };
    let refuse = |value: Value| {
        assert!(
            serde_json::from_value::<PostDownloadCheck>(value.clone()).is_err(),
            "{value}"
        );
    };
    for reason in [
        "descriptionProblem",
        "indeterminateMetadata",
        "unqualifiedElements",
    ] {
        let record = json!({"status":"notQualified", "reason":reason});
        refuse(with(record.clone(), None));
        admit(with(record.clone(), Some(json!("0700"))));
        for pdt in [json!(0), json!(1), json!(63), Value::Null] {
            let mut extra = record.clone();
            extra["pdt"] = pdt;
            refuse(with(extra, Some(json!("0700"))));
        }
    }
    for pdt in [1, 2, 63] {
        admit(with(
            json!({"status":"notQualified", "reason":"unsupportedDatatype", "pdt":pdt}),
            Some(json!("0700")),
        ));
    }
    for pdt in [
        json!(0),
        json!(64),
        json!(1.5),
        json!("2"),
        Value::Null,
        json!(-1),
    ] {
        refuse(with(
            json!({"status":"notQualified", "reason":"unsupportedDatatype", "pdt":pdt}),
            Some(json!("0700")),
        ));
    }
    refuse(with(
        json!({"status":"notQualified", "reason":"unsupportedDatatype"}),
        Some(json!("0700")),
    ));
    for marker in [Value::Null, json!(700), json!("0701"), json!("700")] {
        refuse(with(json!({"status":"undefinedValue"}), Some(marker)));
    }
    refuse(with(
        json!({"status":"notRead", "reason":"unsupportedMask"}),
        Some(json!("0700")),
    ));
    for cause in ["noCorrelatedAnswer", "connectionLost", "readFailed"] {
        let record = json!({"status":"unavailable", "step":"description", "cause":cause});
        refuse(with(record.clone(), None));
        admit(with(record, Some(json!("0700"))));
    }
    // A parser cannot infer the originating fixture's mask from a legacy-shaped state.
    let mut known = with(
        json!({"status":"observed", "objectIndex":3, "state":"running"}),
        Some(json!("0700")),
    );
    known["runtime"] = json!("running");
    admit(known);
}

fn json_of(check: &PostDownloadCheck) -> Value {
    serde_json::to_value(check).unwrap()
}

#[test]
fn every_observed_state_is_the_summary_beside_matching_evidence() {
    for (state, name) in [
        (RunState::Halted, "halted"),
        (RunState::Running, "running"),
        (RunState::Ready, "ready"),
        (RunState::Terminated, "terminated"),
        (RunState::Starting, "starting"),
        (RunState::ShuttingDown, "shuttingDown"),
    ] {
        let check = PostDownloadCheck::from_observation(&observation(
            read_loads(),
            RunStateObservation::Observed(state),
        ));
        let value = json_of(&check);
        assert_eq!(value["runtime"], json!(name));
        assert_eq!(
            value["observation"]["reach"]["identity"]["runState"],
            json!({ "status": "observed", "objectIndex": 3, "state": name })
        );
        let back: PostDownloadCheck = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(back, check);
    }
}

#[test]
fn every_other_run_state_result_keeps_the_summary_unknown_with_closed_evidence() {
    let cases = [
        (
            RunStateObservation::NotRead(RunStateNotRead::UnsupportedMask),
            json!({ "status": "notRead", "reason": "unsupportedMask" }),
        ),
        (
            RunStateObservation::CandidateMismatch(CandidateMismatch::ObjectType),
            json!({ "status": "candidateMismatch", "kind": "objectType" }),
        ),
        (
            RunStateObservation::CandidateMismatch(CandidateMismatch::Application),
            json!({ "status": "candidateMismatch", "kind": "application" }),
        ),
        (
            RunStateObservation::UndefinedValue,
            json!({ "status": "undefinedValue" }),
        ),
    ];
    for (found, expected) in cases {
        let value = json_of(&PostDownloadCheck::from_observation(&observation(
            read_loads(),
            found,
        )));
        assert_eq!(value["runtime"], json!("unknown"));
        assert_eq!(
            value["observation"]["reach"]["identity"]["runState"],
            expected
        );
    }
    for (step, step_name) in [
        (RunStateStep::ObjectType, "objectType"),
        (RunStateStep::Application, "application"),
        (RunStateStep::RunState, "runState"),
    ] {
        for (cause, cause_name) in [
            (RunStateCause::Refused, "refused"),
            (RunStateCause::Malformed, "malformed"),
            (RunStateCause::NoCorrelatedAnswer, "noCorrelatedAnswer"),
            (RunStateCause::ConnectionLost, "connectionLost"),
            (RunStateCause::ReadFailed, "readFailed"),
        ] {
            let value = json_of(&PostDownloadCheck::from_observation(&observation(
                read_loads(),
                RunStateObservation::Unavailable { step, cause },
            )));
            assert_eq!(value["runtime"], json!("unknown"));
            assert_eq!(
                value["observation"]["reach"]["identity"]["runState"],
                json!({ "status": "unavailable", "step": step_name, "cause": cause_name })
            );
        }
    }
}

#[test]
fn a_failed_load_read_is_recorded_as_not_read_for_that_reason() {
    let loads = vec![
        LoadStateObservation::Observed(LoadState::Loaded),
        LoadStateObservation::NotRead,
    ];
    let value = json_of(&PostDownloadCheck::from_observation(&observation(
        loads,
        RunStateObservation::NotRead(RunStateNotRead::LoadReadFailed),
    )));
    assert_eq!(
        value["observation"]["reach"]["identity"]["runState"],
        json!({ "status": "notRead", "reason": "loadReadFailed" })
    );
    assert_eq!(value["runtime"], json!("unknown"));
}

/// A stored matching record: both machines loaded, with the given run evidence.
fn stored(run_state: Option<Value>, runtime: &str) -> Value {
    let mut value = matched(
        vec![
            machine("addressTable", json!("loaded"), loaded()),
            machine("applicationProgram", json!("loaded"), loaded()),
        ],
        "matchesPlan",
    );
    value["runtime"] = json!(runtime);
    let identity = &mut value["observation"]["reach"]["identity"];
    match run_state {
        Some(run_state) => identity["runState"] = run_state,
        None => {
            identity.as_object_mut().unwrap().remove("runState");
        }
    }
    value
}

fn running() -> Value {
    json!({ "status": "observed", "objectIndex": 3, "state": "running" })
}

#[test]
fn legacy_records_without_run_state_reserialize_unchanged_and_null_is_refused() {
    let legacy = stored(None, "unknown");
    let check: PostDownloadCheck = serde_json::from_value(legacy.clone()).unwrap();
    assert_eq!(json_of(&check), legacy, "no field is invented");
    let mut null = legacy;
    null["observation"]["reach"]["identity"]["runState"] = Value::Null;
    assert!(serde_json::from_value::<PostDownloadCheck>(null).is_err());
}

#[test]
fn matching_summary_and_evidence_are_admitted_and_everything_contradictory_refused() {
    let admitted = |value: Value| serde_json::from_value::<PostDownloadCheck>(value).is_ok();
    assert!(admitted(stored(Some(running()), "running")));
    let one = |status: Value| stored(Some(status), "unknown");
    assert!(admitted(one(json!({ "status": "undefinedValue" }))));
    assert!(admitted(one(
        json!({ "status": "notRead", "reason": "unsupportedMask" })
    )));
    // Known summary without evidence, or evidence without summary.
    assert!(!admitted(stored(None, "running")));
    assert!(!admitted(stored(Some(running()), "unknown")));
    // Summary disagrees with the evidence.
    assert!(!admitted(stored(Some(running()), "halted")));
    // Known summary beside non-observed evidence.
    assert!(!admitted(stored(
        Some(json!({ "status": "undefinedValue" })),
        "running"
    )));
    // Wrong index, state unknown, unknown fields, null members.
    for bad in [
        json!({ "status": "observed", "objectIndex": 4, "state": "running" }),
        json!({ "status": "observed", "objectIndex": 0, "state": "running" }),
        json!({ "status": "observed", "objectIndex": 3, "state": "unknown" }),
        json!({ "status": "observed", "objectIndex": 3, "state": "healthy" }),
        json!({ "status": "observed", "objectIndex": 3, "state": "running", "octet": 1 }),
        json!({ "status": "observed", "objectIndex": 3 }),
        json!({ "status": "undefinedValue", "octet": 129 }),
        json!({ "status": "notRead", "reason": "other" }),
        json!({ "status": "candidateMismatch", "kind": "index" }),
        json!({ "status": "unavailable", "step": "scan", "cause": "refused" }),
        json!({ "status": "unavailable", "step": "runState", "cause": "timeout" }),
        json!({ "status": "unavailable", "step": "runState", "cause": "refused", "error": "x" }),
        json!({ "status": "notObserved" }),
    ] {
        assert!(!admitted(stored(Some(bad.clone()), "unknown")), "{bad}");
        assert!(!admitted(stored(Some(bad.clone()), "running")), "{bad}");
    }
}

#[test]
fn non_notread_evidence_needs_every_load_read_to_have_succeeded() {
    let failed_load = |run_state: Value, runtime: &str| {
        let mut value = matched(
            vec![
                machine("addressTable", json!("loaded"), loaded()),
                machine(
                    "applicationProgram",
                    json!("loaded"),
                    json!({ "result": "unavailable" }),
                ),
            ],
            "unknown",
        );
        value["runtime"] = json!(runtime);
        value["observation"]["reach"]["identity"]["runState"] = run_state;
        serde_json::from_value::<PostDownloadCheck>(value).is_ok()
    };
    assert!(failed_load(
        json!({ "status": "notRead", "reason": "loadReadFailed" }),
        "unknown"
    ));
    assert!(!failed_load(running(), "running"));
    assert!(!failed_load(
        json!({ "status": "undefinedValue" }),
        "unknown"
    ));
    assert!(!failed_load(
        json!({ "status": "candidateMismatch", "kind": "objectType" }),
        "unknown"
    ));
    assert!(!failed_load(
        json!({ "status": "unavailable", "step": "runState", "cause": "refused" }),
        "unknown"
    ));
    // `loadReadFailed` is contradicted by loads that all succeeded.
    let all_read = stored(
        Some(json!({ "status": "notRead", "reason": "loadReadFailed" })),
        "unknown",
    );
    assert!(serde_json::from_value::<PostDownloadCheck>(all_read).is_err());
}

#[test]
fn a_known_state_without_an_identity_match_is_refused() {
    let mut value = json!({
        "observation": { "status": "attempted", "reach": { "result": "reached",
            "identity": { "result": "mismatch", "part": { "kind": "application" },
                "kind": "differs" } } },
        "runtime": "running"
    });
    assert!(serde_json::from_value::<PostDownloadCheck>(value.clone()).is_err());
    value["runtime"] = json!("unknown");
    assert!(serde_json::from_value::<PostDownloadCheck>(value).is_ok());
    for runtime in ["running", "halted"] {
        let unreached = json!({ "observation": { "status": "notChecked" }, "runtime": runtime });
        assert!(serde_json::from_value::<PostDownloadCheck>(unreached).is_err());
    }
}

#[tokio::test]
async fn a_real_transfer_then_check_carries_the_observed_state_end_to_end() {
    let device = device(APPLICATION, SimulatorConfig::default());
    device.preset_property(3, PID_OBJECT_TYPE, &[0x00, 0x03]);
    device.preset_property(3, PID_RUN_STATE_CONTROL, &[0x02]);
    let value = transfer_then_check(&device, &plan()).await;
    assert_eq!(value["runtime"], json!("ready"));
    assert_eq!(
        value["observation"]["reach"]["identity"]["runState"],
        json!({ "status": "observed", "objectIndex": 3, "state": "ready" })
    );
}

#[test]
fn the_lines_name_the_object_and_deny_health_download_and_function_claims() {
    let known = PostDownloadCheck::from_observation(&observation(
        read_loads(),
        RunStateObservation::Observed(RunState::Running),
    ))
    .lines();
    let line = known.last().unwrap();
    for part in [
        "run state: running",
        "validated plan-matching",
        "Applicationprogram object 3",
        "not device health",
        "not a successful download",
        "not a unique program instance",
        "not installation function",
    ] {
        assert!(line.contains(part), "{part}: {line}");
    }
    assert!(!known
        .iter()
        .any(|line| line.contains("application running")));
    let ready = PostDownloadCheck::from_observation(&observation(
        read_loads(),
        RunStateObservation::Observed(RunState::Ready),
    ))
    .lines();
    assert!(ready.last().unwrap().contains("run state: ready"));

    let undefined = PostDownloadCheck::from_observation(&observation(
        read_loads(),
        RunStateObservation::UndefinedValue,
    ))
    .lines();
    assert!(undefined
        .contains(&"application running: unknown (no defined run state observed)".to_string()));
    assert!(
        undefined.iter().any(|line| line.contains("undefined")),
        "{undefined:?}"
    );
    assert!(!undefined.contains(&"application running: unknown (not observed)".to_string()));
}

#[test]
fn every_present_run_state_record_without_a_defined_state_says_no_defined_state_was_observed() {
    let records = [
        RunStateObservation::NotRead(RunStateNotRead::UnsupportedMask),
        RunStateObservation::CandidateMismatch(CandidateMismatch::ObjectType),
        RunStateObservation::CandidateMismatch(CandidateMismatch::Application),
        RunStateObservation::UndefinedValue,
        RunStateObservation::Unavailable {
            step: RunStateStep::RunState,
            cause: RunStateCause::NoCorrelatedAnswer,
        },
    ];
    for found in records {
        let lines = PostDownloadCheck::from_observation(&observation(read_loads(), found)).lines();
        assert!(
            lines.contains(
                &"application running: unknown (no defined run state observed)".to_string()
            ),
            "{found:?}: {lines:?}"
        );
        assert!(
            lines
                .iter()
                .any(|line| line.starts_with("  run-state read: ")),
            "{found:?}: the record line stays: {lines:?}"
        );
        assert!(
            !lines.iter().any(|line| line.contains("(not observed)")),
            "{found:?}: {lines:?}"
        );
    }
}

#[test]
fn a_record_with_no_run_state_keeps_its_legacy_unknown_text_and_shape() {
    let legacy: PostDownloadCheck = serde_json::from_value(stored(None, "unknown")).unwrap();
    let lines = legacy.lines();
    assert!(lines.contains(&"application running: unknown (not observed)".to_string()));
    assert!(!lines
        .iter()
        .any(|line| line.contains("no defined run state")));
    assert!(!lines
        .iter()
        .any(|line| line.starts_with("  run-state read")));
    assert_eq!(legacy.runtime_summary(), "application running: unknown");
}

/// Characterization: an observed state beside a machine that differs from the plan.
#[test]
fn an_observed_state_beside_a_differing_machine_is_produced_and_admitted() {
    let produced = json_of(&PostDownloadCheck::from_observation(&observation(
        vec![
            LoadStateObservation::Observed(LoadState::Loaded),
            LoadStateObservation::Observed(LoadState::Unloaded),
        ],
        RunStateObservation::Observed(RunState::Running),
    )));
    let identity = &produced["observation"]["reach"]["identity"];
    assert_eq!(produced["runtime"], json!("running"));
    assert_eq!(identity["planLoadVerdict"], json!("differsFromPlan"));
    assert_eq!(
        identity["runState"],
        json!({ "status": "observed", "objectIndex": 3, "state": "running" })
    );
    assert_eq!(identity["loadStates"][1]["expected"], json!("loaded"));
    assert_eq!(
        identity["loadStates"][1]["observed"],
        json!({ "result": "observed", "loadState": "unloaded" })
    );
    let readmitted: PostDownloadCheck = serde_json::from_value(produced.clone()).unwrap();
    assert_eq!(json_of(&readmitted), produced);

    // The same shape from the shared table: a `runStateRunning` clone whose
    // application machine was read as another state than the plan expects.
    let fixtures: Value = serde_json::from_str(include_str!(
        "../../../../apps/knx-web/src/postDownloadCheck.fixtures.json"
    ))
    .unwrap();
    let cases: Value = serde_json::from_str(include_str!(
        "../../../../apps/knx-web/src/postDownloadCheck.cases.json"
    ))
    .unwrap();
    let case = cases["accepted"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["name"] == "differingLoadWithObservedState")
        .expect("the shared table names the case");
    assert_eq!(case["fixture"], json!("runStateRunning"));
    let shared = edited(&fixtures, case);
    let identity = &shared["observation"]["reach"]["identity"];
    assert_eq!(shared["runtime"], json!("running"));
    assert_eq!(identity["planLoadVerdict"], json!("differsFromPlan"));
    let machine = &identity["loadStates"][2];
    assert_eq!(machine["machine"], json!("applicationProgram"));
    assert_ne!(machine["expected"], machine["observed"]["loadState"]);
    let admitted: PostDownloadCheck = serde_json::from_value(shared.clone()).unwrap();
    assert_eq!(json_of(&admitted), shared);
}
