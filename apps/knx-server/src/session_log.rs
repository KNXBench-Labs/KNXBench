// apps/knx-server/src/session_log.rs
//! Server-side, in-memory record of import diagnostics and operational
//! feedback (T11 — session log) for a future frontend Log tab. Lives only in
//! `AppState`, never persisted to `.knxdb`: it explains what happened during
//! *this* server run, not project content.
//!
//! `reset()` only ever runs where `domain.rs` replaces the whole project
//! (`open_project`/`open_native_project` on success) — everything else
//! (save/edit/undo/redo, and any failed load) only ever appends, so a
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SettingsQuarantineReasonDto {
    Unreadable,
    InvalidJson,
    NotObject,
    MissingSchemaVersion,
    SettingsNotObject,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SettingsDiagnostic {
    Migrated {
        from_version: u32,
        to_version: u32,
    },
    Adopted {
        from_version: u32,
        to_version: u32,
    },
    RefusedNewer {
        file_version: u32,
        current_version: u32,
    },
    Quarantined {
        reason: SettingsQuarantineReasonDto,
        moved_to: String,
    },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    /// RFC3339, `chrono::Utc::now().to_rfc3339()`.
    pub timestamp: String,
    pub severity: Severity,
    /// e.g. "import", "open", "save", "csv-export", "undo", "redo", or the
    /// command's own name for an edit.
    pub source: String,
    pub message: String,
    /// xpath, when the origin has one.
    pub location: Option<String>,
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostic: Option<SettingsDiagnostic>,
}

/// Maximum number of [`LogEntry`] items [`SessionLog`] ever holds at once,
/// including the synthetic drop-notice entry once the cap has been
/// exceeded (see [`SessionLog::push`]). Raised from 1000 after issue #7,
/// where one import alone produced thousands of entries. Not derived from a
/// resource limit: a generous ceiling for a single in-memory, per-process,
/// never-persisted log; `GET /api/log` cost at this size is recorded in the
/// KL-036 dossier.
pub(crate) const MAX_ENTRIES: usize = 16_000;

/// Real entries removed for exceeding [`MAX_ENTRIES`], per severity.
#[derive(Debug, Default, Clone, Copy)]
struct Dropped {
    info: usize,
    warning: usize,
    error: usize,
}

impl Dropped {
    fn total(self) -> usize {
        self.info + self.warning + self.error
    }

    fn count(&mut self, severity: Severity) {
        match severity {
            Severity::Info => self.info += 1,
            Severity::Warning => self.warning += 1,
            Severity::Error => self.error += 1,
        }
    }
}

#[derive(Debug, Default)]
pub struct SessionLog {
    entries: Vec<LogEntry>,
    /// Real entries dropped for exceeding [`MAX_ENTRIES`]. A zero total
    /// means the synthetic drop-notice entry does not exist yet. Cleared by
    /// [`SessionLog::reset`] along with everything else.
    dropped: Dropped,
}

impl SessionLog {
    pub fn reset(&mut self) {
        self.entries.clear();
        self.dropped = Dropped::default();
    }

    /// Appends `entry`, then enforces [`MAX_ENTRIES`].
    ///
    /// Below the cap this is a plain append. Above it, real entries are
    /// removed least severe first (every info before any warning, every
    /// warning before any error) and oldest first within a severity, so
    /// routine lines can never push an import's errors out (#7). The first
    /// overflowing push removes *two* real entries: one for the overflow
    /// and one to make room for the synthetic `Severity::Warning`/`source:
    /// "log"` entry pinned at index 0 from then on. Every later overflow
    /// removes one and refreshes that entry in place; it is never itself
    /// dropped, duplicated or counted. Its message therefore always names
    /// exactly how many real entries are gone, and of which severity.
    pub fn push(&mut self, entry: LogEntry) {
        self.entries.push(entry);
        if self.entries.len() <= MAX_ENTRIES {
            return;
        }
        let first_overflow = self.dropped.total() == 0;
        let removals = if first_overflow { 2 } else { 1 };
        for _ in 0..removals {
            let skip = usize::from(!first_overflow);
            let victim = self.least_valuable(skip);
            let removed = self.entries.remove(victim);
            self.dropped.count(removed.severity);
        }
        if first_overflow {
            self.entries.insert(0, self.synthetic_drop_notice());
        } else {
            self.entries[0] = self.synthetic_drop_notice();
        }
    }

    /// Index of the oldest entry of the lowest severity present, ignoring
    /// the first `skip` entries (the pinned drop notice).
    fn least_valuable(&self, skip: usize) -> usize {
        let mut oldest_warning = None;
        let mut oldest_error = None;
        for (index, entry) in self.entries.iter().enumerate().skip(skip) {
            match entry.severity {
                Severity::Info => return index,
                Severity::Warning => {
                    oldest_warning.get_or_insert(index);
                }
                Severity::Error => {
                    oldest_error.get_or_insert(index);
                }
            }
        }
        oldest_warning
            .or(oldest_error)
            .expect("an over-full log holds at least one real entry")
    }

    fn synthetic_drop_notice(&self) -> LogEntry {
        // The total is 0 (no synthetic entry yet) or >= 2 (see `push`), so
        // "entries" is always grammatically correct here. The leading
        // sentence is what the web client parses; keep it stable.
        let Dropped {
            info,
            warning,
            error,
        } = self.dropped;
        LogEntry {
            timestamp: now(),
            severity: Severity::Warning,
            source: "log".to_string(),
            message: format!(
                "{} log entries dropped after exceeding the {MAX_ENTRIES}-entry session log cap \
                 ({info} info, {warning} warning, {error} error; least severe and oldest first)",
                self.dropped.total()
            ),
            location: None,
            diagnostic: None,
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
///
/// Mostly one entry per finding. Three repetitive kinds are grouped (#7):
/// opaque entries sharing kind and reason, per-file retained-payload notices
/// sharing one wording, and unknown constructs in manufacturer data sharing
/// kind and name (`knx_app::import_findings`). A group of one keeps the single-entry
/// shape; a larger group is one entry whose `detail` lists every member,
/// one per line. The import report itself is never grouped.
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
            location: error
                .source_path
                .clone()
                .or_else(|| (!error.xpath.is_empty()).then(|| error.xpath.clone())),
            diagnostic: None,
            detail: None,
        });
    }

    if let Some(observations) = &report.source_observations {
        entries.push(LogEntry {
            timestamp: now(),
            severity: Severity::Info,
            source: "import:source-observations".into(),
            message: "XML element observations include retained subtrees; not semantic acceptance"
                .into(),
            location: None,
            diagnostic: None,
            detail: Some(
                serde_json::to_string(observations).expect("closed numeric observation contract"),
            ),
        });
    }

    // Unknown constructs in the project stay individual. The same unknown
    // attribute/element across many manufacturer application files is one
    // entry per (kind, name), its detail listing every file and path (#7).
    for unknown in report.unknown.iter().filter(|u| unknown_group(u).is_none()) {
        entries.push(unknown_entry(unknown));
    }
    let unknown_groups = grouped(
        report.unknown.iter().filter(|u| unknown_group(u).is_some()),
        |u| unknown_group(u).expect("filtered to grouped rows"),
    );
    for ((kind, name), members) in unknown_groups {
        if let [only] = members.as_slice() {
            entries.push(unknown_entry(only));
            continue;
        }
        let files: std::collections::BTreeSet<_> =
            members.iter().map(|u| u.source_path.as_str()).collect();
        let occurrences: u64 = members.iter().map(|u| u64::from(u.occurrences)).sum();
        entries.push(LogEntry {
            timestamp: now(),
            severity: Severity::Warning,
            source: "import:unknown".to_string(),
            message: format!(
                "unknown {kind} '{name}' in manufacturer data: {} places in {} files, \
                 {occurrences} occurrence(s); samples in the import report",
                members.len(),
                files.len()
            ),
            location: None,
            diagnostic: None,
            detail: Some(member_list(members.iter().map(|u| {
                format!("{} {} — {}×", u.source_path, u.xpath, u.occurrences)
            }))),
        });
    }

    // Opaque entries are grouped by (kind, reason): a project with hundreds
    // of retained icons, help archives or attributes would otherwise push
    // everything else out of the capped log (#7). A one-member group keeps
    // its original single-entry shape; a larger one becomes one entry whose
    // detail still lists every member with its size and hash, so the log
    // loses nothing. Raw-text candidates are always individual.
    let opaque_groups = grouped(
        report
            .opaque
            .iter()
            .filter(|o| o.raw_text_preview.is_none()),
        |opaque| (opaque.kind.as_str(), opaque.reason.as_str()),
    );
    for ((kind, reason), members) in &opaque_groups {
        if let [opaque] = members.as_slice() {
            entries.push(opaque_entry(opaque));
            continue;
        }
        entries.push(LogEntry {
            timestamp: now(),
            severity: Severity::Info,
            source: "import:opaque".to_string(),
            message: format!(
                "{} {kind} entries preserved opaque: {reason}",
                members.len()
            ),
            location: None,
            diagnostic: None,
            detail: Some(member_list(members.iter().map(|opaque| {
                let place = if opaque.xpath.is_empty() {
                    opaque.source_path.clone()
                } else {
                    format!("{} {}", opaque.source_path, opaque.xpath)
                };
                format!("{place} — {} bytes, sha256 {}", opaque.size, opaque.sha256)
            }))),
        });
    }
    for opaque in report
        .opaque
        .iter()
        .filter(|o| o.raw_text_preview.is_some())
    {
        entries.push(opaque_entry(opaque));
        if let Some(preview) = &opaque.raw_text_preview {
            entries.push(LogEntry {
                timestamp: now(),
                severity: Severity::Info,
                source: "import:opaque:raw-text".to_string(),
                message: format!(
                    "RTF-like raw text candidate ({}); not validated or interpreted; original retained{}",
                    opaque.name,
                    if preview.truncated { "; preview truncated" } else { "" },
                ),
                location: Some(format!("{} {}", opaque.source_path, opaque.xpath)),
                diagnostic: None,
                detail: Some(preview.value.clone()),
            });
        }
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
            diagnostic: None,
            detail: None,
        });
    }

    // Per-file "retained payload" notices share one wording per media class
    // (`knx_app::retained_payload_label`). They are grouped like the opaque
    // entries above; every other unsupported capability stays its own entry.
    for unsupported in report
        .unsupported
        .iter()
        .filter(|u| payload_label(u).is_none())
    {
        entries.push(unsupported_entry(unsupported));
    }
    let payload_groups = grouped(
        report
            .unsupported
            .iter()
            .filter(|u| payload_label(u).is_some()),
        |u| payload_label(u).expect("filtered to labelled rows"),
    );
    for (label, rows) in payload_groups {
        if let [only] = rows.as_slice() {
            entries.push(unsupported_entry(only));
            continue;
        }
        entries.push(LogEntry {
            timestamp: now(),
            severity: Severity::Warning,
            source: "import:unsupported".to_string(),
            message: format!("{} retained files: {label}", rows.len()),
            location: None,
            diagnostic: None,
            detail: Some(member_list(rows.iter().map(|row| row.what.clone()))),
        });
    }

    entries
}

fn unknown_group(unknown: &knx_etsproj::UnknownConstruct) -> Option<(&str, &str)> {
    knx_app::import_findings::manufacturer_unknown_group(
        &unknown.source_path,
        knx_app::import_findings::unknown_kind_name(unknown.kind),
        &unknown.name,
    )
}

fn unknown_entry(unknown: &knx_etsproj::UnknownConstruct) -> LogEntry {
    LogEntry {
        timestamp: now(),
        severity: Severity::Warning,
        source: "import:unknown".to_string(),
        message: format!(
            "unknown {:?} '{}' seen {} time(s) in {}",
            unknown.kind, unknown.name, unknown.occurrences, unknown.source_path
        ),
        location: Some(unknown.xpath.clone()),
        diagnostic: None,
        detail: unknown.sample.clone(),
    }
}

fn opaque_entry(opaque: &knx_etsproj::report::OpaqueSummary) -> LogEntry {
    LogEntry {
        timestamp: now(),
        severity: Severity::Info,
        source: "import:opaque".to_string(),
        message: format!("{} preserved opaque: {}", opaque.kind, opaque.reason),
        location: Some(opaque.source_path.clone()),
        diagnostic: None,
        detail: Some(format!("{} bytes, sha256 {}", opaque.size, opaque.sha256)),
    }
}

fn unsupported_entry(unsupported: &knx_etsproj::report::UnsupportedFeature) -> LogEntry {
    LogEntry {
        timestamp: now(),
        severity: Severity::Warning,
        source: "import:unsupported".to_string(),
        message: format!("{}: {}", unsupported.what, unsupported.consequence),
        location: None,
        diagnostic: None,
        detail: None,
    }
}

fn payload_label(unsupported: &knx_etsproj::report::UnsupportedFeature) -> Option<&str> {
    knx_app::retained_payload_label(&unsupported.what, &unsupported.consequence)
}

/// Groups `items` by `key`, groups and members both in first-seen order.
fn grouped<'a, T, K: Ord + Copy>(
    items: impl Iterator<Item = &'a T>,
    key: impl Fn(&'a T) -> K,
) -> Vec<(K, Vec<&'a T>)> {
    let mut index = std::collections::BTreeMap::new();
    let mut groups: Vec<(K, Vec<&'a T>)> = Vec::new();
    for item in items {
        let k = key(item);
        let slot = *index.entry(k).or_insert_with(|| {
            groups.push((k, Vec::new()));
            groups.len() - 1
        });
        groups[slot].1.push(item);
    }
    groups
}

/// One member per line, in report order — the grouped entry's lossless part.
fn member_list(members: impl Iterator<Item = String>) -> String {
    members.collect::<Vec<_>>().join("\n")
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
            diagnostic: None,
            detail: None,
        });
    }

    for ignored in &report.ignored_columns {
        let reason = match ignored.reason {
            knx_csv::IgnoredColumnReason::ReadOnly => {
                "read-only validation column, never applied on import"
            }
            knx_csv::IgnoredColumnReason::Unknown => "unrecognized column",
        };
        entries.push(LogEntry {
            timestamp: now(),
            severity: Severity::Info,
            source: "csv-import:ignored-column".to_string(),
            message: format!("column '{}' ignored ({reason})", ignored.name),
            location: None,
            diagnostic: None,
            detail: None,
        });
    }

    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexical_observations_are_projected_without_a_semantic_completeness_claim() {
        let mut report = report_with(vec![], vec![], vec![], vec![], vec![]);
        report.source_observations = Some(knx_etsproj::report::SourceObservations {
            meaning: "xml-element-occurrences-not-semantic-acceptance",
            topology: [("DeviceInstance".into(), 3)].into(),
            metadata: Default::default(),
        });
        let rows = from_import_report(&report);
        let row = rows
            .iter()
            .find(|r| r.source == "import:source-observations")
            .expect("source observations reach the diagnostics UI");
        assert_eq!(row.severity, Severity::Info);
        assert!(row.message.contains("not semantic acceptance"));
        assert!(row.detail.as_deref().unwrap().contains("DeviceInstance"));
    }

    fn report_with(
        errors: Vec<knx_etsproj::report::ImportError>,
        unknown: Vec<knx_etsproj::parse::UnknownConstruct>,
        opaque: Vec<knx_etsproj::report::OpaqueSummary>,
        conflicts: Vec<knx_etsproj::infer::Conflict>,
        unsupported: Vec<knx_etsproj::report::UnsupportedFeature>,
    ) -> knx_etsproj::ImportReport {
        knx_etsproj::ImportReport {
            source_observations: None,
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
                    source_path: None,
                    stage: "validate",
                    severity: knx_etsproj::report::Severity::Error,
                    xpath: "/a".into(),
                    detail: "bad thing".into(),
                },
                knx_etsproj::report::ImportError {
                    source_path: None,
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
                raw_text_preview: None,
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
        assert_eq!(counts(&report), (1, 4));
    }

    /// The status-count rule, asserted equal to what the projection shows.
    fn counts(report: &knx_etsproj::ImportReport) -> (usize, usize) {
        let entries = from_import_report(report);
        let shown = |severity| entries.iter().filter(|e| e.severity == severity).count();
        let rule = knx_app::import_finding_counts(report);
        assert_eq!(
            (shown(Severity::Error), shown(Severity::Warning)),
            rule,
            "the project status and the Log's import scope must count the same rows"
        );
        rule
    }

    const PNG_NOTICE: &str = "manufacturer payload (png); retained byte-exact, uninterpreted; not rendered or executed; contents not unpacked";
    const ZIP_NOTICE: &str = "manufacturer payload (zip); retained byte-exact, uninterpreted; not rendered or executed; contents not unpacked";

    fn payload_notice(path: &str, notice: &str) -> knx_etsproj::report::UnsupportedFeature {
        knx_etsproj::report::UnsupportedFeature {
            what: path.into(),
            consequence: notice.into(),
        }
    }

    fn baggage(path: &str, reason: &str) -> knx_etsproj::report::OpaqueSummary {
        knx_etsproj::report::OpaqueSummary {
            source_path: path.into(),
            xpath: String::new(),
            kind: "Baggage".into(),
            name: String::new(),
            size: 7,
            sha256: format!("sha-{path}"),
            reason: reason.into(),
            raw_text_preview: None,
        }
    }

    fn import_error() -> knx_etsproj::report::ImportError {
        knx_etsproj::report::ImportError {
            source_path: None,
            stage: "validate",
            severity: knx_etsproj::report::Severity::Error,
            xpath: "/a".into(),
            detail: "bad thing".into(),
        }
    }

    #[test]
    fn retained_payload_notices_are_grouped_per_wording_without_losing_a_path() {
        let report = report_with(
            vec![],
            vec![],
            vec![
                baggage("M-0001/Baggages/a.png", PNG_NOTICE),
                baggage("M-0001/Baggages/b.png", PNG_NOTICE),
                baggage("M-0001/Baggages/c.zip", ZIP_NOTICE),
            ],
            vec![],
            vec![
                payload_notice("M-0001/Baggages/a.png", PNG_NOTICE),
                knx_etsproj::report::UnsupportedFeature {
                    what: "ETS 5.7 project format (schema 20)".into(),
                    consequence: "evidenced against three samples".into(),
                },
                payload_notice("M-0001/Baggages/b.png", PNG_NOTICE),
                payload_notice("M-0001/Baggages/c.zip", ZIP_NOTICE),
                payload_notice("M-0001/Baggages/d.png", PNG_NOTICE),
            ],
        );
        let entries = from_import_report(&report);
        let rows: Vec<_> = entries
            .iter()
            .map(|e| (e.severity, e.source.as_str(), e.message.as_str()))
            .collect();
        assert_eq!(
            rows,
            vec![
                (
                    Severity::Info,
                    "import:opaque",
                    &*format!("2 Baggage entries preserved opaque: {PNG_NOTICE}")
                ),
                (
                    Severity::Info,
                    "import:opaque",
                    &*format!("Baggage preserved opaque: {ZIP_NOTICE}")
                ),
                (
                    Severity::Warning,
                    "import:unsupported",
                    "ETS 5.7 project format (schema 20): evidenced against three samples"
                ),
                (
                    Severity::Warning,
                    "import:unsupported",
                    &*format!("3 retained files: {PNG_NOTICE}")
                ),
                (
                    Severity::Warning,
                    "import:unsupported",
                    &*format!("M-0001/Baggages/c.zip: {ZIP_NOTICE}")
                ),
            ]
        );
        assert_eq!(
            entries[0].detail.as_deref(),
            Some(
                "M-0001/Baggages/a.png — 7 bytes, sha256 sha-M-0001/Baggages/a.png\n\
                 M-0001/Baggages/b.png — 7 bytes, sha256 sha-M-0001/Baggages/b.png"
            )
        );
        // A group of one keeps exactly the pre-grouping shape.
        assert_eq!(
            entries[1].location.as_deref(),
            Some("M-0001/Baggages/c.zip")
        );
        assert_eq!(
            entries[3].detail.as_deref(),
            Some("M-0001/Baggages/a.png\nM-0001/Baggages/b.png\nM-0001/Baggages/d.png")
        );
        assert_eq!(entries[4].detail, None);
        assert_eq!(counts(&report), (0, 3));
    }

    #[test]
    fn import_errors_survive_a_project_with_thousands_of_retained_files() {
        // Issue #7: ~1400 retained manufacturer files produced one opaque and
        // one unsupported entry each, the 1000-entry cap evicted the oldest,
        // and the genuine findings (written first) were the ones lost.
        let paths: Vec<String> = (0..1500)
            .map(|n| format!("M-0001/Baggages/{n}.png"))
            .collect();
        let report = report_with(
            vec![import_error()],
            vec![],
            paths.iter().map(|p| baggage(p, PNG_NOTICE)).collect(),
            vec![],
            paths
                .iter()
                .map(|p| payload_notice(p, PNG_NOTICE))
                .collect(),
        );
        let mut log = SessionLog::default();
        for entry in from_import_report(&report) {
            log.push(entry);
        }
        let entries = log.entries();
        assert_eq!(entries.len(), 3);
        assert!(entries.iter().all(|e| e.source != "log"), "nothing evicted");
        assert_eq!(entries[0].message, "bad thing");
        let grouped = entries[2].detail.as_deref().unwrap();
        assert_eq!(grouped.lines().count(), 1500);
        assert!(paths.iter().zip(grouped.lines()).all(|(p, line)| p == line));
        assert_eq!(counts(&report), (1, 1));
    }

    fn unknown(source_path: &str, name: &str, occurrences: u32) -> knx_etsproj::UnknownConstruct {
        knx_etsproj::UnknownConstruct {
            source_path: source_path.into(),
            xpath: format!("/KNX/ManufacturerData/{name}"),
            kind: knx_etsproj::UnknownKind::Attribute,
            name: name.into(),
            occurrences,
            sample: Some("sample".into()),
        }
    }

    #[test]
    fn manufacturer_unknowns_are_grouped_by_kind_and_name_project_ones_stay_single() {
        // Issue #7 with a product database: thousands of unknown attributes
        // from application programs, one entry per path, drowned the rest.
        let report = report_with(
            vec![],
            vec![
                unknown("M-0083/M-0083_A-01.xml", "Name", 53),
                unknown("P-0512/project.xml", "ProjectId", 1),
                unknown("M-0001/M-0001_A-02.xml", "Name", 87),
                unknown("M-0083/M-0083_A-01.xml", "Id", 4),
                unknown("M-0083/M-0083_A-03.xml", "Name", 1),
            ],
            vec![],
            vec![],
            vec![],
        );
        let entries = from_import_report(&report);
        let messages: Vec<_> = entries.iter().map(|e| e.message.as_str()).collect();
        assert_eq!(
            messages,
            [
                "unknown Attribute 'ProjectId' seen 1 time(s) in P-0512/project.xml",
                "unknown Attribute 'Name' in manufacturer data: 3 places in 3 files, \
                 141 occurrence(s); samples in the import report",
                "unknown Attribute 'Id' seen 4 time(s) in M-0083/M-0083_A-01.xml",
            ]
        );
        assert_eq!(
            entries[1].detail.as_deref(),
            Some(
                "M-0083/M-0083_A-01.xml /KNX/ManufacturerData/Name — 53×\n\
                 M-0001/M-0001_A-02.xml /KNX/ManufacturerData/Name — 87×\n\
                 M-0083/M-0083_A-03.xml /KNX/ManufacturerData/Name — 1×"
            )
        );
        assert_eq!(entries[2].detail.as_deref(), Some("sample"));
        assert_eq!(counts(&report), (0, 3));
    }

    #[test]
    fn raw_text_candidates_and_other_unsupported_rows_are_never_grouped() {
        let preview = |path: &str| knx_etsproj::report::OpaqueSummary {
            raw_text_preview: Some(knx_etsproj::report::RawTextPreview {
                format: "rtf-like",
                value: "{\\rtf1}".into(),
                truncated: false,
            }),
            xpath: "/x/@Text".into(),
            kind: "RetainedAttribute".into(),
            ..baggage(path, "same reason")
        };
        let same = |n: u8| knx_etsproj::report::UnsupportedFeature {
            what: format!("additional project part P-{n}"),
            consequence: "kept but not imported".into(),
        };
        let report = report_with(
            vec![],
            vec![],
            vec![preview("P-1/0.xml"), preview("P-1/0.xml")],
            vec![],
            vec![same(1), same(2)],
        );
        let entries = from_import_report(&report);
        let sources: Vec<_> = entries.iter().map(|e| e.source.as_str()).collect();
        assert_eq!(
            sources,
            [
                "import:opaque",
                "import:opaque:raw-text",
                "import:opaque",
                "import:opaque:raw-text",
                "import:unsupported",
                "import:unsupported",
            ]
        );
        assert_eq!(counts(&report), (0, 2));
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
            diagnostic: None,
            detail: None,
        });
        log.push(LogEntry {
            timestamp: now(),
            severity: Severity::Info,
            source: "test".into(),
            message: "m2".into(),
            location: None,
            diagnostic: None,
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
            diagnostic: None,
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
    fn a_full_log_drops_info_before_warnings_and_warnings_before_errors() {
        // Issue #7 follow-up: age alone used to decide, so a burst of
        // routine lines could push the import's errors out of the log.
        let with = |severity, n: usize| LogEntry {
            severity,
            ..numbered_entry(n)
        };
        let mut log = SessionLog::default();
        log.push(with(Severity::Error, 0));
        log.push(with(Severity::Warning, 1));
        for n in 2..MAX_ENTRIES {
            log.push(with(Severity::Info, n));
        }
        // Over the cap by 10: infos 2..=12 go (11: ten plus room for the notice).
        for n in MAX_ENTRIES..MAX_ENTRIES + 10 {
            log.push(with(Severity::Warning, n));
        }
        let messages: Vec<_> = log.entries().iter().map(|e| e.message.clone()).collect();
        assert_eq!(log.entries().len(), MAX_ENTRIES);
        assert!(messages.contains(&"entry 0".into()) && messages.contains(&"entry 1".into()));
        assert!(!messages.contains(&"entry 12".into()) && messages.contains(&"entry 13".into()));
        assert_eq!(
            log.entries()[0].message,
            format!(
                "11 log entries dropped after exceeding the {MAX_ENTRIES}-entry session log cap \
                 (11 info, 0 warning, 0 error; least severe and oldest first)"
            )
        );

        // With no info left, the oldest warning goes, never the error.
        let mut log = SessionLog::default();
        log.push(with(Severity::Error, 0));
        for n in 1..=MAX_ENTRIES + 1 {
            log.push(with(Severity::Warning, n));
        }
        let messages: Vec<_> = log.entries().iter().map(|e| e.message.clone()).collect();
        assert!(messages.contains(&"entry 0".into()));
        assert!(!messages.contains(&"entry 1".into()) && !messages.contains(&"entry 3".into()));
        assert!(messages.contains(&"entry 4".into()));
        assert!(log.entries()[0]
            .message
            .contains("(0 info, 3 warning, 0 error;"));
    }

    #[test]
    fn errors_alone_are_dropped_oldest_first_and_counted() {
        let mut log = SessionLog::default();
        for n in 0..MAX_ENTRIES + 3 {
            log.push(LogEntry {
                severity: Severity::Error,
                ..numbered_entry(n)
            });
        }
        let entries = log.entries();
        assert_eq!(entries.len(), MAX_ENTRIES);
        assert!(entries[0].message.starts_with("4 log entries dropped"));
        assert!(entries[0].message.contains("(0 info, 0 warning, 4 error;"));
        assert_eq!(entries[1].message, "entry 4");
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
            diagnostic: None,
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
            readdressed: 0,
            deleted: 0,
            unchanged: 1,
            destructive_changes: Vec::new(),
            ignored_columns: vec![knx_csv::IgnoredColumn {
                name: "DatapointType".into(),
                reason: knx_csv::IgnoredColumnReason::ReadOnly,
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
