//! The achievements record in the data directory: unlocks and counters, merged and never shrunk.
//!
//! One JSON document, `achievements.json`, next to `settings.json` under
//! `AppState::data_dir` (ADR-0089):
//!
//! ```json
//! { "schemaVersion": 1,
//!   "unlocked": { "foundation": "2026-10-07T21:30:00Z" },
//!   "progress": { "time-traveller": 42 } }
//! ```
//!
//! Deliberately its own file rather than a key in `settings.json`: an
//! unlock is a record of something that happened, not a preference, so
//! resetting or quarantining preferences must not take it along.
//!
//! The server owns structure, not meaning. Which achievements exist, what
//! unlocks them and how they are named is the frontend's catalogue
//! (`apps/knx-web/src/achievements/`); this module stores ids it has never
//! heard of exactly like ids it has, which is also what lets a newer
//! build's unlocks survive a round trip through an older one.
//!
//! The record only ever grows. A write is a *delta* merged into what is
//! on disk — an unlock keeps its earliest timestamp, a counter keeps its
//! highest value — so two windows reporting at the same moment cannot
//! undo each other. The one way back is an explicit reset, and even that
//! moves the old file aside instead of deleting it.
//!
//! KL-164: a "one more" counter cannot be a maximum — two windows that
//! both saw 5 would both report 6 and one step was lost. Such steps arrive
//! as additive `increments` under a client `requestId`. The ids of the
//! last [`MAX_APPLIED_REQUESTS`] applied batches are kept in the file
//! (`appliedRequests`), so a duplicated, retried or reordered batch counts
//! once. Older builds keep that member verbatim like any unknown one; the
//! schema version does not change.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::{Map, Value};

/// The file, relative to `AppState::data_dir`.
pub const ACHIEVEMENTS_FILE_NAME: &str = "achievements.json";

/// The schema version this build reads and writes. The first one: the
/// day it changes, a migration chain shaped like `crate::settings`'s
/// arrives with it. Until then a file at any other version is either from
/// the future (refused, untouched) or not one of ours (moved aside).
pub const ACHIEVEMENTS_SCHEMA_VERSION: u32 = 1;

/// The most ids one record may hold, unlocks and counters together. The
/// shipped catalogue has a few dozen; the ceiling only exists so a client
/// cannot grow the file without bound.
pub const MAX_RECORD_ENTRIES: usize = 1024;

/// The most ids one delta may name.
pub const MAX_DELTA_ENTRIES: usize = 256;

/// The largest counter value accepted: JavaScript's
/// `Number.MAX_SAFE_INTEGER`, because the frontend that reads it back is
/// JavaScript and anything larger would not survive the trip exactly.
pub const MAX_PROGRESS: u64 = 9_007_199_254_740_991;

/// The longest id accepted.
const MAX_ID_LEN: usize = 64;

/// How many applied request ids the record remembers. A batch replayed
/// after this many newer ones would count again; real clients retry within
/// seconds, so the window is generous, and it bounds the file.
pub const MAX_APPLIED_REQUESTS: usize = 1024;

/// The largest single increment accepted: a batch is a handful of events.
pub const MAX_INCREMENT: u64 = 10_000;

const REQUEST_ID_MIN_LEN: usize = 8;
const REQUEST_ID_MAX_LEN: usize = 64;

const SCHEMA_VERSION_FIELD: &str = "schemaVersion";
const UNLOCKED_FIELD: &str = "unlocked";
const PROGRESS_FIELD: &str = "progress";
const APPLIED_REQUESTS_FIELD: &str = "appliedRequests";

/// The record in memory.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AchievementRecord {
    /// Achievement id -> RFC 3339 instant it was first unlocked.
    pub unlocked: BTreeMap<String, String>,
    /// Achievement id -> highest counter value reported.
    pub progress: BTreeMap<String, u64>,
    /// Request ids of applied increment batches, oldest first, at most
    /// [`MAX_APPLIED_REQUESTS`].
    pub applied_requests: Vec<String>,
    /// Top-level members this build does not know, carried verbatim so a
    /// newer minor addition is not lost by a write from this one.
    pub extra: Map<String, Value>,
}

/// What a client asks to be merged in: maxima (`unlocked`, `progress`)
/// and, for "one more" counters, additive `increments` under a request id.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AchievementDelta {
    #[serde(default)]
    pub unlocked: BTreeMap<String, String>,
    #[serde(default)]
    pub progress: BTreeMap<String, u64>,
    #[serde(default)]
    pub increments: BTreeMap<String, u64>,
    #[serde(default, rename = "requestId")]
    pub request_id: Option<String>,
}

/// What [`load`] found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AchievementsLoad {
    /// No file yet: nothing unlocked. Not a reason to write one.
    Absent,
    /// A readable file at this build's version.
    Current(AchievementRecord),
    /// A file from a newer build. Left untouched; writes are refused.
    RefusedNewer { file_version: u32 },
    /// An unreadable or malformed file, moved aside to `moved_to`.
    Quarantined { moved_to: PathBuf, detail: String },
}

impl AchievementsLoad {
    /// The record this session should show: whatever was loadable, an
    /// empty one otherwise.
    pub fn record(&self) -> AchievementRecord {
        match self {
            AchievementsLoad::Current(record) => record.clone(),
            AchievementsLoad::Absent
            | AchievementsLoad::RefusedNewer { .. }
            | AchievementsLoad::Quarantined { .. } => AchievementRecord::default(),
        }
    }

    /// Whether a write would overwrite a file this build cannot read.
    pub fn blocks_writes(&self) -> bool {
        matches!(self, AchievementsLoad::RefusedNewer { .. })
    }
}

pub fn achievements_path(data_dir: &Path) -> PathBuf {
    data_dir.join(ACHIEVEMENTS_FILE_NAME)
}

/// Reads the record, moving a damaged file aside. A newer file is neither
/// read nor touched. The `io::Error` this can return is the filesystem
/// refusing the move itself, never a complaint about the contents.
pub fn load(data_dir: &Path) -> std::io::Result<AchievementsLoad> {
    let path = achievements_path(data_dir);
    let raw = match std::fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(AchievementsLoad::Absent),
        Err(e) => return quarantine(data_dir, &path, &format!("could not be read: {e}")),
    };
    match parse(&raw) {
        Ok(Parsed::Current(record)) => Ok(AchievementsLoad::Current(record)),
        Ok(Parsed::Newer(file_version)) => Ok(AchievementsLoad::RefusedNewer { file_version }),
        Err(detail) => quarantine(data_dir, &path, detail),
    }
}

enum Parsed {
    Current(AchievementRecord),
    Newer(u32),
}

/// Reads a document, or says in a few words why it is not one of ours.
/// The version is checked before the maps, so a newer file is refused
/// whatever shape its maps have taken in the meantime.
fn parse(raw: &str) -> Result<Parsed, &'static str> {
    let Ok(Value::Object(mut root)) = serde_json::from_str::<Value>(raw) else {
        return Err("is not a JSON object");
    };
    let version = root
        .remove(SCHEMA_VERSION_FIELD)
        .and_then(|v| v.as_u64())
        .and_then(|v| u32::try_from(v).ok())
        .ok_or("has no readable \"schemaVersion\" (a whole number is required)")?;
    if version > ACHIEVEMENTS_SCHEMA_VERSION {
        return Ok(Parsed::Newer(version));
    }
    if version < ACHIEVEMENTS_SCHEMA_VERSION {
        return Err("has a \"schemaVersion\" no build of this application ever wrote");
    }
    let unlocked = match root.remove(UNLOCKED_FIELD) {
        None | Some(Value::Null) => BTreeMap::new(),
        Some(value) => serde_json::from_value::<BTreeMap<String, String>>(value)
            .map_err(|_| "has an \"unlocked\" member that is not a map of timestamps")?,
    };
    let progress = match root.remove(PROGRESS_FIELD) {
        None | Some(Value::Null) => BTreeMap::new(),
        Some(value) => serde_json::from_value::<BTreeMap<String, u64>>(value)
            .map_err(|_| "has a \"progress\" member that is not a map of counters")?,
    };
    let applied_requests = match root.remove(APPLIED_REQUESTS_FIELD) {
        None | Some(Value::Null) => Vec::new(),
        Some(value) => serde_json::from_value::<Vec<String>>(value)
            .ok()
            .filter(|ids| {
                ids.len() <= MAX_APPLIED_REQUESTS && ids.iter().all(|id| is_valid_request_id(id))
            })
            .ok_or("has an \"appliedRequests\" member that is not a short list of request ids")?,
    };
    Ok(Parsed::Current(AchievementRecord {
        unlocked,
        progress,
        applied_requests,
        extra: root,
    }))
}

/// Moves a damaged file aside as `achievements.damaged-<stamp>.json`.
fn quarantine(data_dir: &Path, path: &Path, detail: &str) -> std::io::Result<AchievementsLoad> {
    let stamp = crate::data_file::utc_stamp();
    let moved_to = crate::data_file::move_aside(data_dir, path, "achievements", "damaged", &stamp)?;
    Ok(AchievementsLoad::Quarantined {
        moved_to,
        detail: format!("{ACHIEVEMENTS_FILE_NAME} {detail}"),
    })
}

/// Writes `record` atomically (`crate::data_file::write_atomically`).
/// Every caller holds `AppState::achievements_lock`.
pub fn store(data_dir: &Path, record: &AchievementRecord) -> std::io::Result<()> {
    let mut root = record.extra.clone();
    root.insert(
        SCHEMA_VERSION_FIELD.to_string(),
        Value::from(ACHIEVEMENTS_SCHEMA_VERSION),
    );
    root.insert(
        UNLOCKED_FIELD.to_string(),
        serde_json::to_value(&record.unlocked)?,
    );
    root.insert(
        PROGRESS_FIELD.to_string(),
        serde_json::to_value(&record.progress)?,
    );
    if !record.applied_requests.is_empty() {
        root.insert(
            APPLIED_REQUESTS_FIELD.to_string(),
            serde_json::to_value(&record.applied_requests)?,
        );
    }
    let mut body = serde_json::to_string_pretty(&Value::Object(root))?;
    body.push('\n');
    crate::data_file::write_atomically(data_dir, ACHIEVEMENTS_FILE_NAME, &body)
}

/// Checks a delta before anything is merged: ids are short lowercase
/// slugs, timestamps are RFC 3339, counters fit [`MAX_PROGRESS`], and the
/// delta names at most [`MAX_DELTA_ENTRIES`] ids.
pub fn validate(delta: &AchievementDelta) -> Result<(), String> {
    let named = delta.unlocked.len() + delta.progress.len() + delta.increments.len();
    if named > MAX_DELTA_ENTRIES {
        return Err(format!(
            "a report may name at most {MAX_DELTA_ENTRIES} achievements, this one names {named}"
        ));
    }
    for id in delta
        .unlocked
        .keys()
        .chain(delta.progress.keys())
        .chain(delta.increments.keys())
    {
        if !is_valid_id(id) {
            return Err(format!(
                "\"{id}\" is not an achievement id (lowercase letters, digits and dashes, \
                 at most {MAX_ID_LEN} characters, starting with a letter or digit)"
            ));
        }
    }
    for (id, at) in &delta.unlocked {
        if chrono::DateTime::parse_from_rfc3339(at).is_err() {
            return Err(format!(
                "the unlock time of \"{id}\" is not an RFC 3339 timestamp"
            ));
        }
    }
    for (id, value) in &delta.progress {
        if *value > MAX_PROGRESS {
            return Err(format!(
                "the counter of \"{id}\" is larger than {MAX_PROGRESS}"
            ));
        }
    }
    for (id, by) in &delta.increments {
        if *by == 0 || *by > MAX_INCREMENT {
            return Err(format!(
                "the increment of \"{id}\" must be between 1 and {MAX_INCREMENT}"
            ));
        }
    }
    match &delta.request_id {
        Some(id) if !is_valid_request_id(id) => Err(format!(
            "a request id has {REQUEST_ID_MIN_LEN} to {REQUEST_ID_MAX_LEN} letters, digits, \
             dashes or underscores"
        )),
        None if !delta.increments.is_empty() => {
            Err("increments need a request id, so a retry cannot count twice".to_string())
        }
        _ => Ok(()),
    }
}

fn is_valid_request_id(id: &str) -> bool {
    (REQUEST_ID_MIN_LEN..=REQUEST_ID_MAX_LEN).contains(&id.len())
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

fn is_valid_id(id: &str) -> bool {
    let bytes = id.as_bytes();
    !bytes.is_empty()
        && bytes.len() <= MAX_ID_LEN
        && bytes[0] != b'-'
        && bytes
            .iter()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'-')
}

/// Whether `candidate` is a strictly earlier instant than `existing`.
/// An existing value this build cannot parse is never displaced: it may
/// be the only record of something, written by a build that knew better.
fn is_earlier(candidate: &str, existing: &str) -> bool {
    match (
        chrono::DateTime::parse_from_rfc3339(candidate),
        chrono::DateTime::parse_from_rfc3339(existing),
    ) {
        (Ok(candidate), Ok(existing)) => candidate < existing,
        _ => false,
    }
}

/// Merges a validated delta into `record`. An unlock keeps its earliest
/// timestamp, a counter its highest value; nothing is ever removed.
/// Increments add up, once per request id while that id is remembered.
/// Refuses, leaving `record` unchanged, when the result would exceed
/// [`MAX_RECORD_ENTRIES`].
pub fn merge(record: &mut AchievementRecord, delta: AchievementDelta) -> Result<(), String> {
    let new_counters: std::collections::BTreeSet<&String> = delta
        .progress
        .keys()
        .chain(delta.increments.keys())
        .filter(|id| !record.progress.contains_key(*id))
        .collect();
    let added = delta
        .unlocked
        .keys()
        .filter(|id| !record.unlocked.contains_key(*id))
        .count()
        + new_counters.len();
    let total = record.unlocked.len() + record.progress.len() + added;
    if total > MAX_RECORD_ENTRIES {
        return Err(format!(
            "the achievements record may hold at most {MAX_RECORD_ENTRIES} entries"
        ));
    }
    for (id, at) in delta.unlocked {
        match record.unlocked.get(&id) {
            Some(existing) if !is_earlier(&at, existing) => {}
            _ => {
                record.unlocked.insert(id, at);
            }
        }
    }
    for (id, value) in delta.progress {
        let entry = record.progress.entry(id).or_insert(0);
        *entry = (*entry).max(value);
    }
    let applied = delta
        .request_id
        .as_ref()
        .is_some_and(|id| record.applied_requests.contains(id));
    if let (false, Some(request_id)) = (applied || delta.increments.is_empty(), delta.request_id) {
        for (id, by) in delta.increments {
            let entry = record.progress.entry(id).or_insert(0);
            *entry = entry.saturating_add(by).min(MAX_PROGRESS);
        }
        record.applied_requests.push(request_id);
        let excess = record
            .applied_requests
            .len()
            .saturating_sub(MAX_APPLIED_REQUESTS);
        record.applied_requests.drain(..excess);
    }
    Ok(())
}

/// Starts over: moves the current file (if any) aside as
/// `achievements.reset-<stamp>.json` and returns where it went. The old
/// record stays on disk for anyone who wants it back.
pub fn reset(data_dir: &Path) -> std::io::Result<Option<PathBuf>> {
    let path = achievements_path(data_dir);
    if !path.exists() {
        return Ok(None);
    }
    let stamp = crate::data_file::utc_stamp();
    crate::data_file::move_aside(data_dir, &path, "achievements", "reset", &stamp).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn data_dir() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    fn write(dir: &tempfile::TempDir, value: &Value) {
        std::fs::write(achievements_path(dir.path()), value.to_string()).unwrap();
    }

    fn delta(unlocked: &[(&str, &str)], progress: &[(&str, u64)]) -> AchievementDelta {
        AchievementDelta {
            unlocked: unlocked
                .iter()
                .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                .collect(),
            progress: progress
                .iter()
                .map(|(k, v)| ((*k).to_string(), *v))
                .collect(),
            ..AchievementDelta::default()
        }
    }

    fn increments(request_id: &str, by: &[(&str, u64)]) -> AchievementDelta {
        AchievementDelta {
            increments: by.iter().map(|(k, v)| ((*k).to_string(), *v)).collect(),
            request_id: Some(request_id.to_string()),
            ..AchievementDelta::default()
        }
    }

    #[test]
    fn increments_from_two_windows_both_count() {
        let mut record = AchievementRecord::default();
        merge(&mut record, delta(&[], &[("power-user", 5)])).unwrap();
        // Both windows saw 5 and each adds one step.
        merge(
            &mut record,
            increments("window-a-0001", &[("power-user", 1)]),
        )
        .unwrap();
        merge(
            &mut record,
            increments("window-b-0001", &[("power-user", 1)]),
        )
        .unwrap();
        assert_eq!(record.progress["power-user"], 7);
    }

    #[test]
    fn a_repeated_or_reordered_batch_counts_once() {
        let mut record = AchievementRecord::default();
        let first = increments("batch-0000001", &[("power-user", 2)]);
        let second = increments("batch-0000002", &[("power-user", 1), ("night-owl", 1)]);
        merge(&mut record, second.clone()).unwrap();
        merge(&mut record, first.clone()).unwrap();
        merge(&mut record, first).unwrap();
        merge(&mut record, second).unwrap();
        assert_eq!(record.progress["power-user"], 3);
        assert_eq!(record.progress["night-owl"], 1);
        assert_eq!(record.applied_requests, ["batch-0000002", "batch-0000001"]);
    }

    #[test]
    fn the_dedup_log_is_bounded_and_survives_the_file() {
        let dir = data_dir();
        let mut record = AchievementRecord::default();
        for n in 0..=MAX_APPLIED_REQUESTS {
            merge(
                &mut record,
                increments(&format!("request-{n:06}"), &[("power-user", 1)]),
            )
            .unwrap();
        }
        assert_eq!(record.applied_requests.len(), MAX_APPLIED_REQUESTS);
        assert_eq!(
            record.applied_requests[0], "request-000001",
            "the oldest id is forgotten first"
        );
        store(dir.path(), &record).unwrap();
        let mut reloaded = load(dir.path()).unwrap().record();
        assert_eq!(reloaded, record);
        merge(
            &mut reloaded,
            increments("request-000500", &[("power-user", 1)]),
        )
        .unwrap();
        assert_eq!(
            reloaded.progress["power-user"], record.progress["power-user"],
            "remembered after a restart"
        );
        merge(
            &mut reloaded,
            increments("request-000000", &[("power-user", 1)]),
        )
        .unwrap();
        assert_eq!(
            reloaded.progress["power-user"],
            record.progress["power-user"] + 1,
            "an id older than the window counts again: the documented bound"
        );
    }

    #[test]
    fn increments_saturate_and_need_a_valid_request_id() {
        let mut record = AchievementRecord::default();
        merge(&mut record, delta(&[], &[("power-user", MAX_PROGRESS - 1)])).unwrap();
        merge(
            &mut record,
            increments("saturate-001", &[("power-user", 5)]),
        )
        .unwrap();
        assert_eq!(record.progress["power-user"], MAX_PROGRESS);
        let mut anonymous = increments("x", &[("power-user", 1)]);
        anonymous.request_id = None;
        assert!(validate(&anonymous).unwrap_err().contains("request id"));
        for bad in [
            "short",
            "has space-in-it",
            "ümlaut-request",
            &"x".repeat(65),
        ] {
            assert!(
                validate(&increments(bad, &[("power-user", 1)])).is_err(),
                "{bad}"
            );
        }
        assert!(validate(&increments("fine-request_01", &[("power-user", 0)])).is_err());
        assert!(validate(&increments(
            "fine-request_01",
            &[("power-user", MAX_INCREMENT + 1)]
        ))
        .is_err());
        assert!(validate(&increments("fine-request_01", &[("Power", 1)])).is_err());
        assert_eq!(
            validate(&increments(
                "fine-request_01",
                &[("power-user", MAX_INCREMENT)]
            )),
            Ok(())
        );
    }

    #[test]
    fn a_damaged_dedup_log_is_moved_aside_like_any_damage() {
        for applied in [json!("one"), json!([1, 2]), json!(["no spaces allowed"])] {
            let dir = data_dir();
            write(
                &dir,
                &json!({ "schemaVersion": 1, "appliedRequests": applied }),
            );
            assert!(
                matches!(
                    load(dir.path()).unwrap(),
                    AchievementsLoad::Quarantined { .. }
                ),
                "{applied}"
            );
        }
    }

    #[test]
    fn an_absent_file_is_an_empty_record_and_no_reason_to_write() {
        let dir = data_dir();
        let load = load(dir.path()).unwrap();
        assert_eq!(load, AchievementsLoad::Absent);
        assert_eq!(load.record(), AchievementRecord::default());
        assert!(!load.blocks_writes());
        assert!(!achievements_path(dir.path()).exists());
    }

    #[test]
    fn a_record_survives_a_round_trip_through_the_file() {
        let dir = data_dir();
        let mut record = AchievementRecord::default();
        record
            .unlocked
            .insert("foundation".into(), "2026-10-07T21:30:00Z".into());
        record.progress.insert("time-traveller".into(), 42);
        store(dir.path(), &record).unwrap();

        assert_eq!(load(dir.path()).unwrap(), AchievementsLoad::Current(record));
    }

    #[test]
    fn the_file_states_its_version_and_ends_with_a_newline() {
        let dir = data_dir();
        store(dir.path(), &AchievementRecord::default()).unwrap();
        let raw = std::fs::read_to_string(achievements_path(dir.path())).unwrap();
        assert!(raw.ends_with('\n'));
        let parsed: Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(parsed[SCHEMA_VERSION_FIELD], ACHIEVEMENTS_SCHEMA_VERSION);
    }

    #[test]
    fn unknown_top_level_members_and_unknown_ids_survive_a_write() {
        let dir = data_dir();
        write(
            &dir,
            &json!({
                "schemaVersion": 1,
                "unlocked": { "from-a-newer-build": "2026-01-01T00:00:00Z" },
                "progress": {},
                "rarityHints": { "kept": true }
            }),
        );
        let mut record = load(dir.path()).unwrap().record();
        merge(
            &mut record,
            delta(&[("foundation", "2026-10-07T21:30:00Z")], &[]),
        )
        .unwrap();
        store(dir.path(), &record).unwrap();

        let raw: Value =
            serde_json::from_str(&std::fs::read_to_string(achievements_path(dir.path())).unwrap())
                .unwrap();
        assert_eq!(raw["rarityHints"], json!({ "kept": true }));
        assert_eq!(
            raw["unlocked"]["from-a-newer-build"],
            "2026-01-01T00:00:00Z"
        );
        assert_eq!(raw["unlocked"]["foundation"], "2026-10-07T21:30:00Z");
    }

    #[test]
    fn a_newer_file_is_refused_and_left_byte_for_byte_as_it_was() {
        let dir = data_dir();
        let body = r#"{"schemaVersion": 7, "unlocked": {"x": "y"}}"#;
        std::fs::write(achievements_path(dir.path()), body).unwrap();

        let load = load(dir.path()).unwrap();
        assert_eq!(load, AchievementsLoad::RefusedNewer { file_version: 7 });
        assert!(load.blocks_writes());
        assert_eq!(load.record(), AchievementRecord::default());
        assert_eq!(
            std::fs::read_to_string(achievements_path(dir.path())).unwrap(),
            body
        );
    }

    #[test]
    fn damaged_files_are_moved_aside_never_deleted() {
        let cases = [
            "not JSON",
            "[1, 2]",
            r#"{"unlocked": {}}"#,
            r#"{"schemaVersion": 0}"#,
            r#"{"schemaVersion": 1.5}"#,
            r#"{"schemaVersion": 1, "unlocked": []}"#,
            r#"{"schemaVersion": 1, "unlocked": {"a": 3}}"#,
            r#"{"schemaVersion": 1, "progress": {"a": "three"}}"#,
            r#"{"schemaVersion": 1, "progress": {"a": -1}}"#,
        ];
        for body in cases {
            let dir = data_dir();
            std::fs::write(achievements_path(dir.path()), body).unwrap();
            let load = load(dir.path()).unwrap();
            let AchievementsLoad::Quarantined { moved_to, detail } = &load else {
                panic!("{body}: expected a quarantine, got {load:?}");
            };
            assert!(!achievements_path(dir.path()).exists(), "{body}");
            assert_eq!(std::fs::read_to_string(moved_to).unwrap(), body);
            let name = moved_to.file_name().unwrap().to_string_lossy().into_owned();
            assert!(name.starts_with("achievements.damaged-"), "{name}");
            assert!(detail.starts_with(ACHIEVEMENTS_FILE_NAME), "{detail}");
            assert!(!load.blocks_writes());
            assert_eq!(load.record(), AchievementRecord::default());
        }
    }

    #[test]
    fn missing_maps_read_as_empty() {
        let dir = data_dir();
        write(&dir, &json!({ "schemaVersion": 1 }));
        assert_eq!(
            load(dir.path()).unwrap(),
            AchievementsLoad::Current(AchievementRecord::default())
        );
    }

    #[test]
    fn an_unlock_keeps_its_earliest_timestamp() {
        let mut record = AchievementRecord::default();
        merge(
            &mut record,
            delta(&[("foundation", "2026-10-07T21:30:00+02:00")], &[]),
        )
        .unwrap();
        merge(
            &mut record,
            delta(&[("foundation", "2026-10-08T08:00:00Z")], &[]),
        )
        .unwrap();
        assert_eq!(record.unlocked["foundation"], "2026-10-07T21:30:00+02:00");
        merge(
            &mut record,
            delta(&[("foundation", "2026-10-07T19:00:00Z")], &[]),
        )
        .unwrap();
        assert_eq!(record.unlocked["foundation"], "2026-10-07T19:00:00Z");
    }

    #[test]
    fn an_existing_timestamp_this_build_cannot_parse_is_never_overwritten() {
        let mut record = AchievementRecord::default();
        record
            .unlocked
            .insert("foundation".into(), "the day the bus woke".into());
        merge(
            &mut record,
            delta(&[("foundation", "2026-10-07T19:00:00Z")], &[]),
        )
        .unwrap();
        assert_eq!(record.unlocked["foundation"], "the day the bus woke");
    }

    #[test]
    fn a_counter_only_ever_rises() {
        let mut record = AchievementRecord::default();
        merge(&mut record, delta(&[], &[("time-traveller", 10)])).unwrap();
        merge(&mut record, delta(&[], &[("time-traveller", 4)])).unwrap();
        assert_eq!(record.progress["time-traveller"], 10);
        merge(&mut record, delta(&[], &[("time-traveller", 11)])).unwrap();
        assert_eq!(record.progress["time-traveller"], 11);
    }

    #[test]
    fn validation_accepts_a_well_formed_delta() {
        assert_eq!(
            validate(&delta(
                &[("night-shift", "2026-10-07T03:00:00Z")],
                &[("palette-pro", 25)]
            )),
            Ok(())
        );
        assert_eq!(validate(&AchievementDelta::default()), Ok(()));
    }

    #[test]
    fn validation_refuses_bad_ids_timestamps_counters_and_sizes() {
        let long = "a".repeat(MAX_ID_LEN + 1);
        let bad: Vec<AchievementDelta> = vec![
            delta(&[("", "2026-10-07T03:00:00Z")], &[]),
            delta(&[("Upper", "2026-10-07T03:00:00Z")], &[]),
            delta(&[("-leading", "2026-10-07T03:00:00Z")], &[]),
            delta(&[("has space", "2026-10-07T03:00:00Z")], &[]),
            delta(&[("../escape", "2026-10-07T03:00:00Z")], &[]),
            delta(&[(long.as_str(), "2026-10-07T03:00:00Z")], &[]),
            delta(&[("night-shift", "yesterday")], &[]),
            delta(&[], &[("palette-pro", MAX_PROGRESS + 1)]),
            delta(&[], &[("Bad", 1)]),
        ];
        for d in bad {
            assert!(validate(&d).is_err(), "{d:?} should be refused");
        }
        let too_many = AchievementDelta {
            unlocked: BTreeMap::new(),
            progress: (0..=MAX_DELTA_ENTRIES)
                .map(|i| (format!("c{i}"), 1))
                .collect(),
            ..AchievementDelta::default()
        };
        assert!(validate(&too_many).is_err());
        let at_limit = AchievementDelta {
            unlocked: BTreeMap::new(),
            progress: (0..MAX_DELTA_ENTRIES)
                .map(|i| (format!("c{i}"), 1))
                .collect(),
            ..AchievementDelta::default()
        };
        assert_eq!(validate(&at_limit), Ok(()));
    }

    #[test]
    fn a_merge_that_would_exceed_the_record_ceiling_changes_nothing() {
        let mut record = AchievementRecord::default();
        for i in 0..MAX_RECORD_ENTRIES {
            record.progress.insert(format!("c{i}"), 1);
        }
        let before = record.clone();
        assert!(merge(&mut record, delta(&[], &[("one-more", 1)])).is_err());
        assert_eq!(record, before);
        // Raising an id that is already there adds no entry, so it is fine.
        merge(&mut record, delta(&[], &[("c0", 5)])).unwrap();
        assert_eq!(record.progress["c0"], 5);
    }

    #[test]
    fn reset_moves_the_record_aside_and_reports_where() {
        let dir = data_dir();
        let mut record = AchievementRecord::default();
        record.progress.insert("time-traveller".into(), 3);
        store(dir.path(), &record).unwrap();
        let before = std::fs::read_to_string(achievements_path(dir.path())).unwrap();

        let moved = reset(dir.path()).unwrap().expect("a file was there");
        let name = moved.file_name().unwrap().to_string_lossy().into_owned();
        assert!(name.starts_with("achievements.reset-"), "{name}");
        assert_eq!(std::fs::read_to_string(&moved).unwrap(), before);
        assert_eq!(load(dir.path()).unwrap(), AchievementsLoad::Absent);
    }

    #[test]
    fn reset_with_nothing_on_disk_is_a_quiet_no_op() {
        let dir = data_dir();
        assert_eq!(reset(dir.path()).unwrap(), None);
    }
}
