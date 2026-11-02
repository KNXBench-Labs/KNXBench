//! Unit tests for the allowlisted session-log, bus-monitor and client-state projections.

use std::time::{Duration, Instant};

use serde_json::{json, Value};

use super::*;

fn row(seq: u64, observed_at: Instant) -> TelegramRow {
    TelegramRow {
        seq,
        timestamp: "2026-11-01T09:00:00+00:00".into(),
        source: "1.1.5".into(),
        destination: "1/2/3".into(),
        destination_name: Some("Light".into()),
        service: "GroupValueWrite".into(),
        raw_payload: Some("[0c, 1a]".into()),
        decoded: None,
        control: None,
        source_raw: Some(0x1105),
        destination_raw: Some(0x0a03),
        observed_at,
        flow_generation: Some(1),
    }
}

/// A fresh preview's projection of `snapshot`.
fn monitor_of(snapshot: &MonitorSnapshot) -> BundleSection {
    bus_monitor(snapshot, &monitor_window(snapshot)).expect("fresh window projects")
}

fn body(section: &BundleSection) -> Value {
    serde_json::from_slice(&section.files[0].bytes).unwrap()
}

#[test]
fn the_ten_minute_window_includes_its_boundary_and_counts_older_rows() {
    let taken = Instant::now();
    let at = |age: Duration| {
        taken
            .checked_sub(age)
            .expect("monotonic clock older than test")
    };
    let snapshot = MonitorSnapshot {
        session_id: 1,
        status: SessionStatus::Active,
        dropped_before: 3,
        next_seq: 7,
        rows: vec![
            row(4, at(MONITOR_WINDOW + Duration::from_millis(1))),
            row(5, at(MONITOR_WINDOW)),
            row(6, at(Duration::ZERO)),
        ],
        taken,
    };
    let section = monitor_of(&snapshot);
    let body = body(&section);
    let seqs: Vec<u64> = body["telegrams"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["seq"].as_u64().unwrap())
        .collect();
    assert_eq!(seqs, [5, 6]);
    assert_eq!(body["firstSeq"], 5);
    assert_eq!(body["endSeq"], 7);
    assert_eq!(body["droppedBeforeAtReview"], 3);
    assert_eq!(section.entries, 2);
    // Undecoded octets are not classified, whatever their length.
    assert_eq!(body["telegrams"][0]["payload"], Value::Null);
    assert_eq!(
        body["telegrams"][0]["timestamp"],
        "2026-11-01T09:00:00.000Z"
    );
    // The same snapshot and window project to the same bytes.
    let window = monitor_window(&snapshot);
    assert_eq!(
        bus_monitor(&snapshot, &window).unwrap().files[0].bytes,
        bus_monitor(&snapshot, &window).unwrap().files[0].bytes
    );
}

#[test]
fn text_dpts_long_payloads_and_codec_text_are_withheld() {
    let taken = Instant::now();
    let mut text = row(1, taken);
    text.decoded = Some(DecodedValue::Value {
        dpt: "DPST-16-0".into(),
        text: "Password123".into(),
    });
    text.raw_payload = Some("[50, 61]".into());
    let mut long = row(2, taken);
    long.raw_payload = Some("[01, 02, 03, 04, 05]".into());
    let mut error = row(3, taken);
    error.decoded = Some(DecodedValue::Error {
        dpt: "DPST-9-1".into(),
        reason: crate::bus::DecodeFailureReason::DecodeFailed,
        text: "bad 5a5ac3a5".into(),
        error: "bad 5a5ac3a5".into(),
    });
    let mut value = row(4, taken);
    value.decoded = Some(DecodedValue::Value {
        dpt: "DPST-9-1".into(),
        text: "21.5 °C".into(),
    });
    let mut odd = row(5, taken);
    odd.service = "Password123".into();
    odd.source = "Password123".into();
    let snapshot = MonitorSnapshot {
        session_id: 1,
        status: SessionStatus::Closed,
        dropped_before: 0,
        next_seq: 6,
        rows: vec![text, long, error, value, odd],
        taken,
    };
    let section = monitor_of(&snapshot);
    let raw = String::from_utf8(section.files[0].bytes.clone()).unwrap();
    assert!(
        !raw.contains("Password123") && !raw.contains("5a5ac3a5"),
        "{raw}"
    );
    let body = body(&section);
    let rows = body["telegrams"].as_array().unwrap();
    assert_eq!(rows[0]["payload"], Value::Null);
    assert_eq!(
        rows[0]["decoded"],
        json!({"kind": "value", "dpt": "DPST-16-0"})
    );
    assert_eq!(rows[1]["payload"], Value::Null);
    assert_eq!(rows[2]["decoded"]["reason"], "decodeFailed");
    assert_eq!(rows[3]["decoded"]["text"], "21.5 °C");
    assert_eq!(rows[4]["service"], Value::Null);
    assert_eq!(rows[4]["source"], Value::Null);
    assert_eq!(body["statusAtReview"], "closed");
    assert_eq!(body["withheld"]["decodedText"], 2);
    assert_eq!(body["withheld"]["payload"], 4);
    assert!(section.notes.contains(&NOTE_REDUCED.to_string()));
}

#[test]
fn message_forms_accept_only_their_placeholders() {
    let mut withheld = Withheld::default();
    let form = |m: &str, w: &mut Withheld| classify_message(m, w);
    assert_eq!(
        form(
            "refused to open: the open project has unsaved edits",
            &mut withheld
        )
        .as_deref(),
        Some("refused to open: the open project has unsaved edits")
    );
    assert_eq!(
        form(
            "refused to Password123: the open project has unsaved edits",
            &mut withheld
        ),
        None
    );
    let drop = "12 log entries dropped after exceeding the 16000-entry session log cap \
                (2 info, 10 warning, 0 error; least severe and oldest first)";
    assert_eq!(form(drop, &mut withheld).as_deref(), Some(drop));
    assert_eq!(
        form(&drop.replace("12 log", "0x5a log"), &mut withheld),
        None
    );
    assert_eq!(form("saved Password123", &mut withheld), None);
    assert_eq!(
        form("saved as /home/alice/x.knxdb", &mut withheld).as_deref(),
        Some("saved as [withheld]")
    );
    assert_eq!(withheld.0["message"], 3);
    assert_eq!(withheld.0["path"], 1);
}

#[test]
fn app_version_grammar_refuses_free_prerelease_text() {
    assert!(app_version("0.1.0-alpha.1"));
    assert!(app_version("1.2.3"));
    assert!(!app_version("0.1.0-Password123"));
    assert!(!app_version("0.1"));
    assert!(!app_version("v0.1.0"));
}

#[test]
fn wrong_types_and_oversized_error_lists_are_withheld_not_echoed() {
    let errors: Vec<Value> = (0..51).map(|_| json!({"message": "saved"})).collect();
    let client = json!({
        "shell": "Password123",
        "uiScale": "big",
        "viewport": {"width": -1, "height": 900},
        "recentErrors": errors,
        "omittedErrorCount": 2,
    });
    let section = client_state(client.as_object().unwrap());
    let raw = String::from_utf8(section.files[0].bytes.clone()).unwrap();
    assert!(
        !raw.contains("Password123") && !raw.contains("big"),
        "{raw}"
    );
    let body = body(&section);
    assert_eq!(body["shell"], Value::Null);
    assert_eq!(body["uiScale"], Value::Null);
    assert_eq!(body["viewport"], json!({"width": null, "height": 900}));
    assert_eq!(body["recentErrors"], Value::Null);
    assert_eq!(body["omittedErrorCount"], 2);
    assert_eq!(body["withheld"]["invalidFields"], 4);
}

fn snapshot_of(rows: Vec<TelegramRow>) -> MonitorSnapshot {
    let taken = Instant::now();
    MonitorSnapshot {
        session_id: 1,
        status: SessionStatus::Active,
        dropped_before: 0,
        next_seq: rows.len() as u64,
        rows,
        taken,
    }
}

fn group_row(seq: u64, value: knx_core::GroupValue, decoded: Option<DecodedValue>) -> TelegramRow {
    let mut row = row(seq, Instant::now());
    row.raw_payload = Some(crate::bus::format_group_value_payload(&value));
    row.decoded = decoded;
    row
}

fn dpt_ref(main: u16, sub: u16) -> knx_core::DptRef {
    knx_core::DptRef {
        main,
        sub: Some(sub),
    }
}

#[test]
fn dpt15_access_codes_from_the_real_decoder_never_leave() {
    let value = knx_core::GroupValue::Bytes(vec![0x12, 0x34, 0x56, 0x00]);
    let decoded = crate::bus::decode_single(dpt_ref(15, 0), &value);
    assert!(
        matches!(&decoded, DecodedValue::Value { text, .. } if text.contains("123456")),
        "producer must decode the access code: {decoded:?}"
    );
    let section = monitor_of(&snapshot_of(vec![group_row(0, value, Some(decoded))]));
    let raw = String::from_utf8(section.files[0].bytes.clone()).unwrap();
    assert!(
        !raw.contains("123456") && !raw.contains("12, 34, 56"),
        "{raw}"
    );
    let body = body(&section);
    assert_eq!(
        body["telegrams"][0]["decoded"],
        json!({"kind": "value", "dpt": "DPST-15-0"})
    );
    assert_eq!(body["telegrams"][0]["payload"], Value::Null);
    assert_eq!(body["withheld"]["decodedText"], 1);
    assert_eq!(body["withheld"]["payload"], 1);
}

#[test]
fn raw_octets_without_a_typed_non_text_decode_never_leave() {
    use knx_core::GroupValue::{Bytes, Short};
    let pin = Bytes(vec![0x50, 0x49, 0x4e, 0x31]);
    let malformed = crate::bus::decode_single(dpt_ref(16, 0), &pin);
    assert!(
        matches!(malformed, DecodedValue::Error { .. }),
        "{malformed:?}"
    );
    let rows = vec![
        group_row(
            0,
            Bytes(vec![0x50, 0x49]),
            Some(DecodedValue::Unresolved {
                reason: crate::bus::UnresolvedReason::NoProject,
                text: "no project".into(),
            }),
        ),
        group_row(
            1,
            Bytes(vec![0x61, 0x62, 0x63, 0x00]),
            Some(DecodedValue::Conflict {
                text: "conflicting DPTs: DPST-12-1, DPST-16-0 SENTINEL".into(),
            }),
        ),
        group_row(2, pin, Some(malformed)),
        group_row(3, Bytes(vec![0x12, 0x34]), None),
        group_row(4, Short(1), None),
    ];
    let section = monitor_of(&snapshot_of(rows));
    let raw = String::from_utf8(section.files[0].bytes.clone()).unwrap();
    for leaked in ["50, 49", "61, 62", "12, 34", "0x01", "SENTINEL"] {
        assert!(!raw.contains(leaked), "{leaked} in {raw}");
    }
    let body = body(&section);
    for row in body["telegrams"].as_array().unwrap() {
        assert_eq!(row["payload"], Value::Null, "{row}");
    }
    assert_eq!(body["withheld"]["payload"], 5);
}

#[test]
fn a_known_non_text_value_keeps_its_producer_octets() {
    let value = knx_core::GroupValue::Bytes(vec![0x0c, 0x1a]);
    let decoded = crate::bus::decode_single(dpt_ref(9, 1), &value);
    let section = monitor_of(&snapshot_of(vec![group_row(0, value, Some(decoded))]));
    let body = body(&section);
    assert_eq!(body["telegrams"][0]["payload"], "[0c, 1a]");
    assert_eq!(body["telegrams"][0]["decoded"]["dpt"], "DPST-9-1");
}

#[test]
fn decoded_values_leave_only_on_value_services_and_names_are_bounded() {
    let mut management = row(0, Instant::now());
    management.service = "MemoryResponse".into();
    management.decoded = Some(DecodedValue::Value {
        dpt: "DPST-9-1".into(),
        text: "SENTINEL".into(),
    });
    let mut named = row(1, Instant::now());
    named.destination_name = Some("n".repeat(500));
    let section = monitor_of(&snapshot_of(vec![management, named]));
    let raw = String::from_utf8(section.files[0].bytes.clone()).unwrap();
    assert!(!raw.contains("SENTINEL"), "{raw}");
    let body = body(&section);
    assert_eq!(body["telegrams"][0]["decoded"], Value::Null);
    let name = body["telegrams"][1]["destinationName"].as_str().unwrap();
    assert!(name.chars().count() <= 128, "{}", name.len());
    assert_eq!(body["withheld"]["destinationNameTruncated"], 1);
}

#[test]
fn shipped_language_packs_and_bundled_themes_are_known() {
    for (language, theme, expected_theme) in [
        (
            "bar",
            "user-modern-retro-green-crt",
            "user-modern-retro-green-crt",
        ),
        ("tlh", "user-imported-pack", "other"),
        ("de", "rail-dark", "rail-dark"),
    ] {
        let client = json!({"uiLanguage": language, "theme": theme});
        let body = body(&client_state(client.as_object().unwrap()));
        assert_eq!(body["uiLanguage"], language);
        assert_eq!(body["theme"], expected_theme);
    }
}

#[test]
fn undo_redo_import_counts_and_source_observations_keep_their_known_text() {
    let entry = |source: &str, message: &str| LogEntry {
        timestamp: "2026-11-01T09:00:00Z".into(),
        severity: crate::session_log::Severity::Info,
        source: source.into(),
        message: message.into(),
        location: None,
        detail: None,
        diagnostic: None,
    };
    let entries = [
        entry("undo", "undo"),
        entry("redo", "redo"),
        entry(
            "import",
            "imported Password123.knxproj (DeviceInstance: 3/3, GroupAddress: 10/12)",
        ),
        entry("import", "imported x.knxproj (Password123: 3/3)"),
        entry(
            "import:source-observations",
            "XML element observations include retained subtrees; not semantic acceptance",
        ),
        entry("AddressProgramming", "SENTINEL"),
    ];
    let section = session_log(&entries);
    let raw = String::from_utf8(section.files[0].bytes.clone()).unwrap();
    assert!(
        !raw.contains("Password123") && !raw.contains("SENTINEL"),
        "{raw}"
    );
    let body = body(&section);
    let rows = body["entries"].as_array().unwrap();
    assert_eq!(rows[0]["source"], "undo");
    assert_eq!(rows[0]["message"], "undo");
    assert_eq!(rows[1]["source"], "redo");
    assert_eq!(rows[1]["message"], "redo");
    assert_eq!(
        rows[2]["message"],
        "imported [withheld] (DeviceInstance: 3/3, GroupAddress: 10/12)"
    );
    assert_eq!(rows[3]["message"], "imported [withheld]");
    assert_eq!(
        rows[4]["message"],
        "XML element observations include retained subtrees; not semantic acceptance"
    );
    assert_eq!(rows[5]["source"], "AddressProgramming");
}

/// A new `knx_core::Command` variant must be listed here, not silently withheld.
/// This source-layout guard intentionally fails loudly if the enum is reformatted.
#[test]
fn command_sources_equal_the_command_enum() {
    let source = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../crates/knx-core/src/command.rs"
    ))
    .unwrap();
    let body = source
        .split("pub enum Command {")
        .nth(1)
        .and_then(|rest| rest.split("\n}").next())
        .expect("Command enum");
    let mut variants: Vec<&str> = body
        .lines()
        .filter_map(|line| line.strip_prefix("    "))
        .filter(|line| line.starts_with(|c: char| c.is_ascii_uppercase()))
        .map(|line| line.split([' ', '{', '(', ',']).next().unwrap())
        .collect();
    variants.sort_unstable();
    variants.dedup();
    let mut listed = COMMAND_SOURCES.to_vec();
    listed.sort_unstable();
    assert_eq!(listed, variants);
}

fn contiguous(seqs: &[u64]) -> MonitorSnapshot {
    let taken = Instant::now();
    MonitorSnapshot {
        session_id: 7,
        status: SessionStatus::Active,
        dropped_before: 0,
        next_seq: 10,
        rows: seqs.iter().map(|seq| row(*seq, taken)).collect(),
        taken,
    }
}

fn pinned(first_seq: u64, end_seq: u64) -> MonitorBinding {
    MonitorBinding {
        session_id: 7,
        first_seq,
        end_seq,
        dropped_before: 0,
        status_at_review: MonitorStatus::Active,
        captured_at: "2026-11-01T09:00:00.000Z".into(),
    }
}

#[test]
fn a_pinned_window_projects_exactly_its_rows_and_ignores_later_traffic() {
    let before = bus_monitor(&contiguous(&[5, 6, 7]), &pinned(5, 8)).unwrap();
    let later = bus_monitor(&contiguous(&[5, 6, 7, 8, 9]), &pinned(5, 8)).unwrap();
    assert_eq!(before.files[0].bytes, later.files[0].bytes);
    assert_eq!(later.entries, 3);
    let empty = bus_monitor(&contiguous(&[8, 9]), &pinned(8, 8)).unwrap();
    assert_eq!(body(&empty)["telegrams"], json!([]));
}

#[test]
fn evicted_duplicated_or_missing_pinned_rows_and_other_sessions_refuse() {
    // Oldest pinned row evicted by the cap.
    assert!(bus_monitor(&contiguous(&[6, 7, 8, 9]), &pinned(5, 8)).is_none());
    // A hole and a duplicate inside the interval.
    assert!(bus_monitor(&contiguous(&[5, 7, 8]), &pinned(5, 8)).is_none());
    assert!(bus_monitor(&contiguous(&[5, 6, 6, 7]), &pinned(5, 8)).is_none());
    // A window reaching past what the buffer has numbered.
    assert!(bus_monitor(&contiguous(&[5, 6, 7]), &pinned(5, 11)).is_none());
    // Same numbers, another session.
    let mut other = pinned(5, 8);
    other.session_id = 8;
    assert!(bus_monitor(&contiguous(&[5, 6, 7]), &other).is_none());
}

#[test]
fn malformed_bindings_are_not_well_formed() {
    assert!(pinned(5, 8).is_well_formed());
    assert!(!pinned(9, 8).is_well_formed());
    assert!(!pinned(0, crate::bus::MAX_TELEGRAMS as u64 + 1).is_well_formed());
    assert!(!pinned(9_007_199_254_740_992, 9_007_199_254_740_992).is_well_formed());
    let mut date = pinned(5, 8);
    date.captured_at = "2026-11-01T10:00:00+01:00".into();
    assert!(
        !date.is_well_formed(),
        "only the canonical rendering is accepted"
    );
    for raw in [
        json!({"sessionId": 1, "firstSeq": 0, "endSeq": 1, "droppedBefore": 0,
               "statusAtReview": "active", "capturedAt": "2026-11-01T09:00:00.000Z", "x": 1}),
        json!({"sessionId": 1, "firstSeq": 0.5, "endSeq": 1, "droppedBefore": 0,
               "statusAtReview": "active", "capturedAt": "2026-11-01T09:00:00.000Z"}),
        json!({"sessionId": 1, "firstSeq": 0, "endSeq": 1, "droppedBefore": 0,
               "statusAtReview": "gone", "capturedAt": "2026-11-01T09:00:00.000Z"}),
    ] {
        assert!(serde_json::from_value::<MonitorBinding>(raw).is_err());
    }
}
