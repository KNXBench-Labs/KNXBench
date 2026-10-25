//! Receipt bounds: untrusted metadata becomes unavailable, and every legal row fits the store.

use knx_productdb::query::ProgramIdentity;

use super::*;
use crate::commissioning_activity::{
    DownloadCleanup, DownloadEvidence, OneShotActivity, OneShotLog, Restart, WriteEvidence,
};
use crate::post_download_check::PostDownloadCheck;

const SHA: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn identity(id: &str, version: Option<&str>, sha: &str) -> ProgramIdentity {
    ProgramIdentity {
        id: id.into(),
        name: Some("a name that is never stored".into()),
        application_version: version.map(Into::into),
        source_sha256: sha.into(),
    }
}

#[test]
fn only_a_matching_bounded_identity_is_stored_and_no_name_is() {
    let good = identity("M-0001_A-0001-01-0000", Some("1.2"), SHA);
    let stored = program_receipt("M-0001_A-0001-01-0000", Some(&good));
    assert_eq!(
        stored,
        ProgramReceipt::Identified {
            id: "M-0001_A-0001-01-0000".into(),
            application_version: Some("1.2".into()),
            source_sha256: SHA.into()
        }
    );
    assert!(!serde_json::to_string(&stored)
        .unwrap()
        .contains("never stored"));
    let no_version = identity("P", None, SHA);
    assert!(matches!(
        program_receipt("P", Some(&no_version)),
        ProgramReceipt::Identified {
            application_version: None,
            ..
        }
    ));
    let quote = identity("a\"b", None, SHA);
    let slash = identity("a\\b", None, SHA);
    let long = identity(&"x".repeat(MAX_PROGRAM_ID + 1), None, SHA);
    let edge = identity(&"x".repeat(MAX_PROGRAM_ID), None, SHA);
    let non_ascii = identity("Ä", None, SHA);
    let empty = identity("", None, SHA);
    let space = identity("a b", None, SHA);
    let version_long = identity("P", Some(&"v".repeat(MAX_APPLICATION_VERSION + 1)), SHA);
    let version_empty = identity("P", Some(""), SHA);
    let version_quote = identity("P", Some("1\""), SHA);
    let upper = identity("P", None, &SHA.to_uppercase());
    let short = identity("P", None, &SHA[1..]);
    for (id, bad) in [
        ("a\"b", &quote),
        ("a\\b", &slash),
        (&"x".repeat(MAX_PROGRAM_ID + 1), &long),
        ("Ä", &non_ascii),
        ("", &empty),
        ("a b", &space),
        ("P", &version_long),
        ("P", &version_empty),
        ("P", &version_quote),
        ("P", &upper),
        ("P", &short),
    ] {
        assert_eq!(
            program_receipt(id, Some(bad)),
            ProgramReceipt::Unavailable,
            "{id:?}"
        );
    }
    assert!(matches!(
        program_receipt(&"x".repeat(MAX_PROGRAM_ID), Some(&edge)),
        ProgramReceipt::Identified { .. }
    ));
    // Another program than the request named, and no identity at all.
    assert_eq!(
        program_receipt("Q", Some(&good)),
        ProgramReceipt::Unavailable
    );
    assert_eq!(program_receipt("Q", None), ProgramReceipt::Unavailable);
}

#[test]
fn counts_beyond_u32_make_the_summary_unavailable_not_truncated() {
    assert_eq!(plan_counts(3, [4, 5].into_iter()), Some((3, 9)));
    assert_eq!(plan_counts(0, [].into_iter()), Some((0, 0)));
    let max = u32::MAX as usize;
    assert_eq!(
        plan_counts(max, [max].into_iter()),
        Some((u32::MAX, u32::MAX))
    );
    assert_eq!(plan_counts(max + 1, [].into_iter()), None);
    assert_eq!(plan_counts(1, [max + 1].into_iter()), None);
    assert_eq!(
        plan_counts(1, [max, 1].into_iter()),
        None,
        "the sum overflows"
    );
    assert_eq!(plan_counts(usize::MAX, [usize::MAX].into_iter()), None);
}

#[test]
fn the_settlement_instant_is_one_canonical_millisecond_utc_string() {
    let at: DateTime<Utc> = "2026-10-25T07:08:09.123456Z".parse().unwrap();
    assert_eq!(canonical_time(at).unwrap(), "2026-10-25T07:08:09.123Z");
    use chrono::TimeZone;
    let far = Utc.with_ymd_and_hms(10_000, 1, 1, 0, 0, 0).unwrap();
    assert_eq!(canonical_time(far), None, "no 24-byte form exists");
    let early = Utc.with_ymd_and_hms(-1, 1, 1, 0, 0, 0).unwrap();
    assert_eq!(canonical_time(early), None);
    for bad in [
        "2026-10-25T07:08:09Z",
        "2026-10-25T07:08:09.123+00:00",
        "2026-10-25t07:08:09.123Z",
        "2026-10-25T07:08:09.123z",
        "2026-10-25T07:08:09.12Z",
        "2026-13-25T07:08:09.123Z",
        "2026-10-25T07:08:60.123Z",
        "2026-10-25T07:08:61.123Z",
        "",
    ] {
        assert!(!canonical_time_text(bad), "{bad}");
    }
    let unavailable = DownloadSettlement::computed(
        ProjectOutcome::SessionOnly,
        SourceVerdict::Unchanged,
        CurrentRequest::Readable,
        &Settlement::default(),
        far,
    );
    assert_eq!(
        unavailable,
        DownloadSettlement::Unavailable { format: Format1 }
    );
}

// ---- size ---------------------------------------------------------------

fn id_of(length: usize) -> String {
    // Every ASCII graphic byte but quote and backslash, `<>&'` among them.
    let alphabet: Vec<char> = (0x21_u8..0x7f)
        .map(char::from)
        .filter(|c| *c != '"' && *c != '\\')
        .collect();
    assert!("<>&'".chars().all(|c| alphabet.contains(&c)));
    (0..length).map(|i| alphabet[i % alphabet.len()]).collect()
}

fn maximal_check() -> PostDownloadCheck {
    let machines = [
        "addressTable",
        "associationTable",
        "applicationProgram",
        "peiProgram",
    ];
    let mut longest: Option<(usize, PostDownloadCheck)> = None;
    for verdict in ["matchesPlan", "differsFromPlan", "unknown"] {
        for expected in ["unloaded", "loadCompleting", "unloading"] {
            for observed in ["unloaded", "loadCompleting", "unloading"] {
                let readings: Vec<_> = machines
                    .iter()
                    .map(|machine| {
                        serde_json::json!({
                            "machine": machine,
                            "expected": expected,
                            "observed": {"result": "observed", "loadState": observed},
                        })
                    })
                    .collect();
                let document = serde_json::json!({
                    "observation": {"status": "attempted", "reach": {
                        "result": "reached", "identity": {"result": "match",
                            "exemplar": "notVerified",
                            "loadStates": readings, "planLoadVerdict": verdict}}},
                    "runtime": "unknown",
                });
                // Admitted through the check's own deserialisation, or skipped.
                let Ok(check) = serde_json::from_value::<PostDownloadCheck>(document) else {
                    continue;
                };
                let size = serde_json::to_string(&check).unwrap().len();
                if longest.as_ref().is_none_or(|(best, _)| size > *best) {
                    longest = Some((size, check));
                }
            }
        }
    }
    let (_, check) = longest.expect("some four-machine check is admitted");
    let json = serde_json::to_value(&check).unwrap();
    assert_eq!(
        json["observation"]["reach"]["identity"]["loadStates"]
            .as_array()
            .unwrap()
            .len(),
        4
    );
    check
}

fn bits(values: [bool; 4]) -> PartBits {
    PartBits {
        address: values[0],
        application: values[1],
        parameters: values[2],
        group_addresses: values[3],
    }
}

/// Every programmed/pending/untouched assignment of the four parts.
fn all_parts() -> Vec<SettledParts> {
    (0..81_u32)
        .map(|n| {
            let mut programmed = [false; 4];
            let mut pending = [false; 4];
            let mut touched = [false; 4];
            for (i, part) in (0..4).map(|i| (i, (n / 3_u32.pow(i as u32)) % 3)) {
                programmed[i] = part == 1;
                pending[i] = part == 2;
                touched[i] = part != 0;
            }
            SettledParts {
                programmed: bits(programmed),
                pending: bits(pending),
                touched: bits(touched),
            }
        })
        .collect()
}

fn settlements() -> Vec<DownloadSettlement> {
    let mut all = vec![
        DownloadSettlement::NotAttempted { format: Format1 },
        DownloadSettlement::NoProject { format: Format1 },
        DownloadSettlement::OtherProject { format: Format1 },
        DownloadSettlement::DeviceMissing { format: Format1 },
        DownloadSettlement::Unavailable { format: Format1 },
    ];
    let settled_at = "2026-10-25T07:08:09.123Z".to_string();
    for parts in all_parts() {
        for source in [
            SourceVerdict::Unchanged,
            SourceVerdict::ChangedOrUnknown,
            SourceVerdict::NotChecked,
        ] {
            for current_request in [CurrentRequest::Readable, CurrentRequest::Unavailable] {
                all.push(DownloadSettlement::Persisted {
                    format: Format1,
                    source,
                    current_request,
                    applied: parts,
                    settled_at: settled_at.clone(),
                });
                all.push(DownloadSettlement::SessionOnly {
                    format: Format1,
                    source,
                    current_request,
                    applied: parts,
                    settled_at: settled_at.clone(),
                });
                all.push(DownloadSettlement::PersistenceFailed {
                    format: Format1,
                    source,
                    current_request,
                    computed: parts,
                });
            }
        }
    }
    all
}

fn preparations(scope: Scope) -> Vec<DownloadPreparation> {
    let mut all = vec![DownloadPreparation::Unavailable {
        format: Format1,
        scope,
    }];
    for program in [
        ProgramReceipt::Unavailable,
        ProgramReceipt::Identified {
            id: id_of(MAX_PROGRAM_ID),
            application_version: Some(id_of(MAX_APPLICATION_VERSION)),
            source_sha256: SHA.into(),
        },
        ProgramReceipt::Identified {
            id: id_of(MAX_PROGRAM_ID),
            application_version: None,
            source_sha256: SHA.into(),
        },
    ] {
        all.push(DownloadPreparation::Available {
            format: Format1,
            scope,
            program,
            planned_mask: "FFFF".into(),
            planned_manufacturer: "FFFF".into(),
            steps: u32::MAX,
            data_octets: u32::MAX,
        });
    }
    all
}

const SCOPES: [Scope; 4] = [
    Scope::Complete,
    Scope::PartialParameters,
    Scope::PartialGroupAddresses,
    Scope::PartialBoth,
];

fn row(
    id: u64,
    state: &str,
    written: Written,
    restart: Option<Restart>,
    cleanup: DownloadCleanup,
    evidence: (bool, bool),
    check: Option<PostDownloadCheck>,
) -> OneShotActivity {
    OneShotActivity {
        id,
        kind: "deviceDownload".into(),
        address: Some("15.15.255".into()),
        state: state.into(),
        started_at: "2026-10-25T07:00:00.000Z".into(),
        finished_at: Some("2026-10-25T07:08:09.123Z".into()),
        write_evidence: Some(WriteEvidence {
            backup_recorded: evidence.0,
            send_possible: evidence.1,
        }),
        download_evidence: Some(DownloadEvidence {
            session_id: u64::MAX,
            written: Some(written),
            restart,
            cleanup,
            post_download_check: check,
            preparation: None,
            settlement: None,
        }),
    }
}

fn with_receipts(
    mut entry: OneShotActivity,
    preparation: &DownloadPreparation,
    settlement: &DownloadSettlement,
) -> OneShotActivity {
    let evidence = entry.download_evidence.as_mut().unwrap();
    evidence.preparation = Some(preparation.clone());
    evidence.settlement = Some(settlement.clone());
    entry
}

fn size(entry: &OneShotActivity) -> usize {
    serde_json::to_string(entry).unwrap().len()
}

/// Every legal full row, worst case per dimension, receipts and the largest
/// post-download check together. Receipts depend on a row only through its
/// state, written value and scope, so the longest legal settlement and
/// preparation are chosen per such triple and then composed with every
/// legal write evidence, cleanup, restart and check. Returns the longest.
fn longest_legal_row(id: u64) -> OneShotActivity {
    let settlements = settlements();
    let checks = [
        None,
        Some(PostDownloadCheck::interrupted()),
        Some(maximal_check()),
    ];
    let mut longest: Option<OneShotActivity> = None;
    let mut legal = 0_u32;
    for (state, written, restart) in [
        ("finished", Written::Yes, Some(Restart::Unconfirmed)),
        ("finished", Written::Yes, Some(Restart::Acknowledged)),
        ("finished", Written::Yes, Some(Restart::NotInPlan)),
        ("finished", Written::Partially, Some(Restart::Unconfirmed)),
        ("finished", Written::No, Some(Restart::NotInPlan)),
        ("failed", Written::Yes, None),
        ("failed", Written::Partially, None),
        ("failed", Written::No, None),
    ] {
        for scope in SCOPES {
            let preparation = preparations(scope)
                .into_iter()
                .max_by_key(|p| serde_json::to_string(p).unwrap().len())
                .unwrap();
            let settlement = settlements
                .iter()
                .filter(|settlement| {
                    valid_receipts(state, Some(written), Some(&preparation), Some(settlement))
                })
                .max_by_key(|settlement| serde_json::to_string(settlement).unwrap().len())
                .expect("every row state has a legal settlement");
            for cleanup in [
                DownloadCleanup::Pending,
                DownloadCleanup::ReturnedOk,
                DownloadCleanup::ReturnedError,
                DownloadCleanup::Unknown,
            ] {
                for evidence in [(false, false), (true, false), (true, true)] {
                    for check in &checks {
                        let entry = row(
                            id,
                            state,
                            written,
                            restart,
                            cleanup,
                            evidence,
                            check.clone(),
                        );
                        let entry = with_receipts(entry, &preparation, settlement);
                        if entry.validate(id).is_err() {
                            continue;
                        }
                        legal += 1;
                        if longest
                            .as_ref()
                            .is_none_or(|best| size(&entry) > size(best))
                        {
                            longest = Some(entry);
                        }
                    }
                }
            }
        }
    }
    assert!(legal > 100, "the sweep must reach many legal rows: {legal}");
    longest.expect("a legal row exists")
}

#[test]
fn the_longest_legal_row_with_both_receipts_and_the_largest_check_fits_the_store() {
    // Serialization alone: ids beyond the store's range are a private proof.
    let widest = longest_legal_row(u64::MAX);
    let bytes = size(&widest);
    assert!(
        bytes <= knx_store::activity_history::MAX_DOCUMENT_BYTES,
        "longest legal row {bytes} bytes"
    );
    let evidence = widest.download_evidence.as_ref().unwrap();
    assert!(evidence.post_download_check.is_some(), "largest check kept");
    assert!(evidence.preparation.is_some() && evidence.settlement.is_some());
    // The same shape through the real writer and strict reader, at the
    // largest operation id the store accepts.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("history.sqlite");
    let log = OneShotLog::persistent(path, "i".repeat(64));
    let mut entry = longest_legal_row(i64::MAX as u64);
    entry.address = Some("15.15.255".into());
    log.persist(&entry)
        .expect("the longest legal row is stored");
    let (rows, more) = log.history_page(0, 10).expect("and read back strictly");
    assert!(!more && rows.len() == 1);
    let reread = serde_json::to_string(&rows[0].activity).unwrap();
    assert_eq!(reread, serde_json::to_string(&entry).unwrap());
    println!("longest legal row: {bytes} bytes (private proof), store ceiling 4096");
}

#[test]
fn quote_backslash_and_oversize_never_widen_a_stored_document() {
    // One serialised byte per allowed byte: no escape can grow an id.
    let id = id_of(MAX_PROGRAM_ID);
    assert_eq!(
        serde_json::to_string(&id).unwrap().len(),
        MAX_PROGRAM_ID + 2
    );
    for text in ["a\"b", "a\\b", "a\nb", "a\u{7f}b"] {
        assert!(!graphic(text, 128), "{text:?}");
    }
}
