// apps/knx-server/src/session_log.rs
//! Server-side, in-memory record of import diagnostics and operational
//! feedback (T11 — session log) for a future frontend Log tab. Lives only in
//! `AppState`, never persisted to `.knxdb`: it explains what happened during
//! *this* server run, not project content.
//!
//! `reset()` only ever runs where `domain.rs` replaces the whole project
//! (`open_project`/`open_native_project` on success) — everything else
//! (save/export/edit/undo/redo, and any failed load) only ever appends, so a
//! failed operation never erases the trail that explains what state the
//! project is actually in.
//!
//! [`MAX_ENTRIES`] caps how many entries `SessionLog` ever holds at once.
//! Past that, `push()` evicts the oldest *real* entries and keeps a
//! synthetic warning entry pinned at index 0 that names the running total
//! of real entries dropped so far — the number in that message is always
//! exactly how many are actually missing, because CLAUDE.md's "never
//! silently discard information" rule applies to the log itself, not just
//! to import data, and a counter that undercounts its own losses would
//! violate that rule just as surely as dropping the entries silently
//! would. See `push`'s own doc comment for the exact mechanics.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    /// RFC3339, `chrono::Utc::now().to_rfc3339()`.
    pub timestamp: String,
    pub severity: Severity,
    /// e.g. "import", "open", "save", "export", "undo", "redo", or the
    /// command's own name for an edit.
    pub source: String,
    pub message: String,
    /// xpath, when the origin has one.
    pub location: Option<String>,
    pub detail: Option<String>,
}

/// Maximum number of [`LogEntry`] items [`SessionLog`] ever holds at once,
/// including the synthetic drop-notice entry once the cap has been
/// exceeded (see [`SessionLog::push`]). Not derived from any measured
/// resource limit — a generous ceiling for a single in-memory, per-process,
/// never-persisted session log, chosen to keep `GET /api/log` cheap to
/// re-fetch and re-serialize even across a very long session.
const MAX_ENTRIES: usize = 1000;

#[derive(Debug, Default)]
pub struct SessionLog {
    entries: Vec<LogEntry>,
    /// Running count of real entries dropped for exceeding [`MAX_ENTRIES`].
    /// Zero means the synthetic drop-notice entry does not exist yet.
    /// Cleared by [`SessionLog::reset`] along with everything else.
    dropped: usize,
}

impl SessionLog {
    pub fn reset(&mut self) {
        self.entries.clear();
        self.dropped = 0;
    }

    /// Appends `entry`, then enforces [`MAX_ENTRIES`].
    ///
    /// Below the cap this is a plain append. `dropped` counts real entries
    /// actually removed, not calls that overflowed, because those are not
    /// the same number: the push that first exceeds the cap removes *two*
    /// real entries in one call — the oldest one (which is what "at the
    /// cap" always means) and a second one to make room for the synthetic
    /// `Severity::Warning`/`source: "log"` entry that gets pinned at index
    /// 0 from that point on, naming how many real entries have been
    /// dropped so far. So `dropped` jumps by 2 on that first overflowing
    /// push. Every following push while still over capacity removes
    /// exactly one more real entry (index 1, since index 0 is the pinned
    /// synthetic entry), so `dropped` grows by 1 from there on, and
    /// refreshes the synthetic entry's message in place — it is never
    /// itself dropped, duplicated, or counted against `dropped`. The
    /// number the synthetic entry reports is therefore always exactly how
    /// many real entries are actually gone, not a rougher proxy for it.
    pub fn push(&mut self, entry: LogEntry) {
        self.entries.push(entry);
        if self.entries.len() <= MAX_ENTRIES {
            return;
        }
        if self.dropped == 0 {
            self.dropped += 2;
            self.entries.remove(0);
            self.entries.remove(0);
            self.entries.insert(0, self.synthetic_drop_notice());
        } else {
            self.dropped += 1;
            self.entries.remove(1);
            self.entries[0] = self.synthetic_drop_notice();
        }
    }

    fn synthetic_drop_notice(&self) -> LogEntry {
        // `dropped` is 0 (no synthetic entry yet) or >= 2 (see `push`) —
        // it is never 1, so "entries" is always grammatically correct here.
        LogEntry {
            timestamp: now(),
            severity: Severity::Warning,
            source: "log".to_string(),
            message: format!(
                "{} log entries dropped after exceeding the {MAX_ENTRIES}-entry session log cap",
                self.dropped
            ),
            location: None,
            detail: None,
        }
    }

    pub fn entries(&self) -> &[LogEntry] {
        &self.entries
    }
}

pub fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// Converts one `ImportReport` into its warning/error entries (success
/// notice NOT included here — the caller appends that separately, since
/// only the caller knows the operation actually succeeded end to end).
///
/// Order: `report.errors`, then `report.unknown`, then `report.opaque`,
/// then `report.conflicts`, then `report.unsupported` — the same order
/// `ImportReport`'s own fields are declared in, so a reader scanning the
/// log sees "what's actually wrong" before "what's merely unhandled".
///
/// `report.inferred` and `SourceInfo::namespace_disagreement` are
/// deliberately not mapped here — a residual for a later round, not
/// silently dropped (see `docs/KNOWN_LIMITATIONS.md`/`IMPLEMENTATION_STATUS.md`).
pub fn from_import_report(report: &knx_etsproj::ImportReport) -> Vec<LogEntry> {
    let mut entries = Vec::new();

    for error in &report.errors {
        entries.push(LogEntry {
            timestamp: now(),
            severity: match error.severity {
                knx_etsproj::report::Severity::Error => Severity::Error,
                knx_etsproj::report::Severity::Warning => Severity::Warning,
            },
            source: format!("import:{}", error.stage),
            message: error.detail.clone(),
            location: Some(error.xpath.clone()),
            detail: None,
        });
    }

    for unknown in &report.unknown {
        entries.push(LogEntry {
            timestamp: now(),
            severity: Severity::Warning,
            source: "import:unknown".to_string(),
            message: format!(
                "unknown {:?} '{}' seen {} time(s) in {}",
                unknown.kind, unknown.name, unknown.occurrences, unknown.source_path
            ),
            location: Some(unknown.xpath.clone()),
            detail: unknown.sample.clone(),
        });
    }

    for opaque in &report.opaque {
        entries.push(LogEntry {
            timestamp: now(),
            severity: Severity::Info,
            source: "import:opaque".to_string(),
            message: format!("{} preserved opaque: {}", opaque.kind, opaque.reason),
            location: Some(opaque.source_path.clone()),
            detail: Some(format!("{} bytes, sha256 {}", opaque.size, opaque.sha256)),
        });
    }

    for conflict in &report.conflicts {
        entries.push(LogEntry {
            timestamp: now(),
            severity: Severity::Warning,
            source: "import:conflict".to_string(),
            message: format!(
                "group address {} has {} candidate datapoint types",
                conflict.group_address,
                conflict.candidates.len()
            ),
            location: None,
            detail: None,
        });
    }

    for unsupported in &report.unsupported {
        entries.push(LogEntry {
            timestamp: now(),
            severity: Severity::Warning,
            source: "import:unsupported".to_string(),
            message: format!("{}: {}", unsupported.what, unsupported.consequence),
            location: None,
            detail: None,
        });
    }

    entries
}

/// Converts one `CsvImportReport` (T12 — CSV group-address import,
/// `knx_csv::plan_import`) into its problem/ignored-column entries.
/// Deliberately the same shape as `from_import_report` above: the success
/// summary is NOT included here — the caller (`domain.rs`) appends that
/// separately, once it knows whether the import as a whole was accepted
/// (design §4's all-or-nothing rule means "planned cleanly" and "actually
/// applied" can differ only in the row-error case, which the caller alone
/// resolves).
///
/// Order: `report.problems`, then `report.ignored_columns` — what might be
/// wrong before what was merely not applied, the same "wrong before
/// unhandled" order `from_import_report` uses for its own categories.
/// Neither list is ever dropped for being "only" a warning or "only" an
/// ignored column (CLAUDE.md: never silently discard information).
pub fn from_csv_import_report(report: &knx_csv::CsvImportReport) -> Vec<LogEntry> {
    let mut entries = Vec::new();

    for problem in &report.problems {
        let kind = match problem.severity {
            knx_csv::Severity::Error => "error",
            knx_csv::Severity::Warning => "warning",
        };
        entries.push(LogEntry {
            timestamp: now(),
            severity: match problem.severity {
                knx_csv::Severity::Error => Severity::Error,
                knx_csv::Severity::Warning => Severity::Warning,
            },
            source: format!("csv-import:{kind}"),
            message: problem.detail.clone(),
            location: problem.row.map(|row| format!("row {row}")),
            detail: None,
        });
    }

    for ignored in &report.ignored_columns {
        let reason = match ignored.reason {
            knx_csv::IgnoredColumnReason::ExportOnly => "export-only column, not applied on import",
            knx_csv::IgnoredColumnReason::Unknown => "unrecognized column",
        };
        entries.push(LogEntry {
            timestamp: now(),
            severity: Severity::Info,
            source: "csv-import:ignored-column".to_string(),
            message: format!("column '{}' ignored ({reason})", ignored.name),
            location: None,
            detail: None,
        });
    }

    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report_with(
        errors: Vec<knx_etsproj::report::ImportError>,
        unknown: Vec<knx_etsproj::parse::UnknownConstruct>,
        opaque: Vec<knx_etsproj::report::OpaqueSummary>,
        conflicts: Vec<knx_etsproj::infer::Conflict>,
        unsupported: Vec<knx_etsproj::report::UnsupportedFeature>,
    ) -> knx_etsproj::ImportReport {
        knx_etsproj::ImportReport {
            source: knx_etsproj::report::SourceInfo {
                file_name: "t.knxproj".into(),
                file_size: 0,
                schema_version: 11,
                namespace: "t".into(),
                created_by: None,
                tool_version: None,
                namespace_disagreement: None,
            },
            counts: knx_etsproj::report::EntityCounts { rows: vec![] },
            unknown,
            opaque,
            inferred: vec![],
            conflicts,
            unsupported,
            errors,
        }
    }

    #[test]
    fn each_report_category_maps_to_its_documented_severity_and_order() {
        let report = report_with(
            vec![
                knx_etsproj::report::ImportError {
                    stage: "validate",
                    severity: knx_etsproj::report::Severity::Error,
                    xpath: "/a".into(),
                    detail: "bad thing".into(),
                },
                knx_etsproj::report::ImportError {
                    stage: "map",
                    severity: knx_etsproj::report::Severity::Warning,
                    xpath: "/b".into(),
                    detail: "suspicious thing".into(),
                },
            ],
            vec![knx_etsproj::parse::UnknownConstruct {
                source_path: "P-0512/0.xml".into(),
                xpath: "/c".into(),
                kind: knx_etsproj::parse::UnknownKind::Element,
                name: "Foo".into(),
                occurrences: 3,
                sample: Some("<Foo bar=\"1\"/>".into()),
            }],
            vec![knx_etsproj::report::OpaqueSummary {
                source_path: "P-0512/1.xml".into(),
                xpath: String::new(),
                kind: "Baggage".into(),
                name: String::new(),
                size: 42,
                sha256: "deadbeef".into(),
                reason: "unrecognized manufacturer namespace".into(),
            }],
            vec![knx_etsproj::infer::Conflict {
                group_address: knx_core::GroupAddressId(7),
                candidates: vec![],
            }],
            vec![knx_etsproj::report::UnsupportedFeature {
                what: "some.dll".into(),
                consequence: "not executed".into(),
            }],
        );

        let entries = from_import_report(&report);
        assert_eq!(entries.len(), 6);

        assert_eq!(entries[0].severity, Severity::Error);
        assert_eq!(entries[0].source, "import:validate");
        assert_eq!(entries[0].message, "bad thing");
        assert_eq!(entries[0].location.as_deref(), Some("/a"));

        assert_eq!(entries[1].severity, Severity::Warning);
        assert_eq!(entries[1].source, "import:map");
        assert_eq!(entries[1].message, "suspicious thing");

        assert_eq!(entries[2].severity, Severity::Warning);
        assert_eq!(entries[2].source, "import:unknown");
        assert!(entries[2].message.contains("Foo"));
        assert!(entries[2].message.contains('3'));
        assert!(entries[2].message.contains("P-0512/0.xml"));
        assert_eq!(entries[2].location.as_deref(), Some("/c"));
        assert_eq!(entries[2].detail.as_deref(), Some("<Foo bar=\"1\"/>"));

        assert_eq!(entries[3].severity, Severity::Info);
        assert_eq!(entries[3].source, "import:opaque");
        assert!(entries[3].message.contains("Baggage"));
        assert!(entries[3]
            .message
            .contains("unrecognized manufacturer namespace"));
        assert_eq!(entries[3].location.as_deref(), Some("P-0512/1.xml"));
        assert_eq!(
            entries[3].detail.as_deref(),
            Some("42 bytes, sha256 deadbeef")
        );

        assert_eq!(entries[4].severity, Severity::Warning);
        assert_eq!(entries[4].source, "import:conflict");
        assert!(entries[4].message.contains('7'));
        assert!(entries[4].location.is_none());

        assert_eq!(entries[5].severity, Severity::Warning);
        assert_eq!(entries[5].source, "import:unsupported");
        assert_eq!(entries[5].message, "some.dll: not executed");
    }

    #[test]
    fn reset_clears_and_push_appends() {
        let mut log = SessionLog::default();
        assert!(log.entries().is_empty());
        log.push(LogEntry {
            timestamp: now(),
            severity: Severity::Info,
            source: "test".into(),
            message: "m1".into(),
            location: None,
            detail: None,
        });
        log.push(LogEntry {
            timestamp: now(),
            severity: Severity::Info,
            source: "test".into(),
            message: "m2".into(),
            location: None,
            detail: None,
        });
        assert_eq!(log.entries().len(), 2);
        log.reset();
        assert!(log.entries().is_empty());
    }

    fn numbered_entry(n: usize) -> LogEntry {
        LogEntry {
            timestamp: now(),
            severity: Severity::Info,
            source: "test".into(),
            message: format!("entry {n}"),
            location: None,
            detail: None,
        }
    }

    fn is_synthetic_drop_notice(entry: &LogEntry) -> bool {
        entry.severity == Severity::Warning && entry.source == "log"
    }

    #[test]
    fn under_the_cap_nothing_is_dropped() {
        let mut log = SessionLog::default();
        for n in 0..500 {
            log.push(numbered_entry(n));
        }
        assert_eq!(log.entries().len(), 500);
        assert!(!log.entries().iter().any(is_synthetic_drop_notice));
    }

    #[test]
    fn exactly_at_the_cap_still_no_synthetic_entry() {
        let mut log = SessionLog::default();
        for n in 0..MAX_ENTRIES {
            log.push(numbered_entry(n));
        }
        assert_eq!(log.entries().len(), MAX_ENTRIES);
        assert!(!log.entries().iter().any(is_synthetic_drop_notice));
    }

    #[test]
    fn one_past_the_cap_drops_two_real_entries_and_names_two_dropped() {
        let mut log = SessionLog::default();
        for n in 0..=MAX_ENTRIES {
            // MAX_ENTRIES + 1 pushes total.
            log.push(numbered_entry(n));
        }
        let entries = log.entries();
        assert_eq!(entries.len(), MAX_ENTRIES);

        assert!(is_synthetic_drop_notice(&entries[0]));
        assert!(entries[0].message.contains('2'));
        assert!(entries[0].message.contains(&MAX_ENTRIES.to_string()));

        assert!(
            !entries.iter().any(|e| e.message == "entry 0"),
            "oldest real entry should have been dropped: {entries:#?}"
        );
        assert!(
            !entries.iter().any(|e| e.message == "entry 1"),
            "second-oldest real entry should have been dropped too, to make room \
             for the synthetic entry: {entries:#?}"
        );
        assert!(
            entries
                .iter()
                .any(|e| e.message == format!("entry {MAX_ENTRIES}")),
            "newest real entry should still be present: {entries:#?}"
        );
    }

    #[test]
    fn well_past_the_cap_names_the_total_dropped_and_keeps_one_synthetic_entry() {
        let mut log = SessionLog::default();
        for n in 0..(MAX_ENTRIES + 250) {
            log.push(numbered_entry(n));
        }
        let entries = log.entries();
        assert_eq!(entries.len(), MAX_ENTRIES);

        assert!(is_synthetic_drop_notice(&entries[0]));
        assert!(entries[0].message.contains("251"));
        assert_eq!(
            entries
                .iter()
                .filter(|e| is_synthetic_drop_notice(e))
                .count(),
            1
        );
    }

    /// Pins the invariant directly instead of relying on hard-coded numbers
    /// that could happen to match more than one (wrong) accounting scheme —
    /// which is exactly how the very first version of this counter shipped
    /// with an off-by-one and still passed its own tests. Whatever the
    /// synthetic entry claims was dropped, plus however many real entries
    /// are actually still there, must equal every push that ever happened.
    #[test]
    fn dropped_plus_retained_always_equals_total_pushes_past_the_cap() {
        fn dropped_count_named_in(entry: &LogEntry) -> usize {
            entry
                .message
                .split_whitespace()
                .next()
                .expect("synthetic drop notice message starts with a number")
                .parse()
                .expect("synthetic drop notice message starts with a number")
        }

        for extra in [1usize, 250usize] {
            let mut log = SessionLog::default();
            let total = MAX_ENTRIES + extra;
            for n in 0..total {
                log.push(numbered_entry(n));
            }
            let entries = log.entries();
            let dropped = dropped_count_named_in(&entries[0]);
            let retained_real_entries = entries.len() - 1; // index 0 is synthetic
            assert_eq!(
                dropped + retained_real_entries,
                total,
                "extra={extra}: dropped ({dropped}) named in the synthetic entry \
                 plus real entries retained ({retained_real_entries}) should equal \
                 total pushes ({total})"
            );
        }
    }

    #[test]
    fn reset_after_a_drop_clears_the_synthetic_entry_and_the_dropped_count() {
        let mut log = SessionLog::default();
        for n in 0..(MAX_ENTRIES + 5) {
            log.push(numbered_entry(n));
        }
        assert!(log.entries().iter().any(is_synthetic_drop_notice));

        log.reset();
        assert!(log.entries().is_empty());

        log.push(numbered_entry(0));
        assert_eq!(log.entries().len(), 1);
        assert!(!log.entries().iter().any(is_synthetic_drop_notice));
    }

    #[test]
    fn severity_serializes_lowercase_and_entry_uses_camel_case_fields() {
        let entry = LogEntry {
            timestamp: "2026-09-10T00:00:00Z".into(),
            severity: Severity::Warning,
            source: "import:unknown".into(),
            message: "m".into(),
            location: Some("/x".into()),
            detail: None,
        };
        let json = serde_json::to_value(&entry).unwrap();
        assert_eq!(json["severity"], "warning");
        assert_eq!(json["timestamp"], "2026-09-10T00:00:00Z");
        assert_eq!(json["source"], "import:unknown");
        assert_eq!(json["message"], "m");
        assert_eq!(json["location"], "/x");
        assert!(json["detail"].is_null());
    }

    #[test]
    fn csv_import_report_maps_problems_then_ignored_columns_in_order() {
        let report = knx_csv::CsvImportReport {
            separator: ',',
            rows_read: 2,
            created: 1,
            updated: 0,
            unchanged: 1,
            ignored_columns: vec![knx_csv::IgnoredColumn {
                name: "DatapointType".into(),
                reason: knx_csv::IgnoredColumnReason::ExportOnly,
            }],
            problems: vec![
                knx_csv::CsvProblem {
                    row: Some(3),
                    severity: knx_csv::Severity::Error,
                    detail: "name missing".into(),
                },
                knx_csv::CsvProblem {
                    row: Some(4),
                    severity: knx_csv::Severity::Warning,
                    detail: "no range contains this address".into(),
                },
            ],
        };

        let entries = from_csv_import_report(&report);
        assert_eq!(entries.len(), 3);

        assert_eq!(entries[0].severity, Severity::Error);
        assert_eq!(entries[0].source, "csv-import:error");
        assert_eq!(entries[0].message, "name missing");
        assert_eq!(entries[0].location.as_deref(), Some("row 3"));

        assert_eq!(entries[1].severity, Severity::Warning);
        assert_eq!(entries[1].source, "csv-import:warning");
        assert_eq!(entries[1].message, "no range contains this address");
        assert_eq!(entries[1].location.as_deref(), Some("row 4"));

        assert_eq!(entries[2].severity, Severity::Info);
        assert_eq!(entries[2].source, "csv-import:ignored-column");
        assert!(entries[2].message.contains("DatapointType"));
        assert!(entries[2].location.is_none());
    }
}
