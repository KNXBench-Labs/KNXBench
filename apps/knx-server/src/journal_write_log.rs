//! Recent journal-write timings for the diagnostic bundle and a log line for slow writes.

use std::collections::VecDeque;
use std::time::Duration;

use knx_store::project_history::JournalWriteReport;
use serde_json::{json, Value};

/// How many recent journal writes the diagnostic bundle shows.
pub const RECENT_JOURNAL_WRITES: usize = 20;
/// A journal write at least this long is logged to standard error.
pub const SLOW_JOURNAL_WRITE: Duration = Duration::from_secs(2);

/// One finished journal write: when, how long, whether it succeeded and what it did.
#[derive(Debug, Clone)]
pub struct JournalWriteRecord {
    pub at: String,
    /// Wall time of the whole write, server preparation included.
    pub total: Duration,
    pub ok: bool,
    pub report: JournalWriteReport,
}

/// The last [`RECENT_JOURNAL_WRITES`] journal writes, oldest first.
#[derive(Debug, Default)]
pub struct RecentJournalWrites {
    records: VecDeque<JournalWriteRecord>,
}

impl RecentJournalWrites {
    /// Keeps `record`, dropping the oldest beyond the limit, and logs it when slow.
    pub fn record(&mut self, record: JournalWriteRecord) {
        if record.total >= SLOW_JOURNAL_WRITE {
            eprintln!("knx-server: {}", slow_line(&record));
        }
        if self.records.len() == RECENT_JOURNAL_WRITES {
            self.records.pop_front();
        }
        self.records.push_back(record);
    }

    pub fn records(&self) -> impl Iterator<Item = &JournalWriteRecord> {
        self.records.iter()
    }

    /// The diagnostic-bundle view: timings, counts, sizes and flags only.
    pub fn to_json(&self) -> Value {
        json!({
            "slowThresholdMs": millis(SLOW_JOURNAL_WRITE),
            "recent": self.records.iter().map(record_json).collect::<Vec<_>>(),
        })
    }
}

fn millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

fn record_json(record: &JournalWriteRecord) -> Value {
    let report = &record.report;
    let phases: serde_json::Map<String, Value> = report
        .phases
        .iter()
        .map(|(phase, time)| ((*phase).to_owned(), json!(millis(*time))))
        .collect();
    json!({
        "at": record.at,
        "totalMs": millis(record.total),
        "ok": record.ok,
        "phasesMs": phases,
        "undoStates": report.undo_states,
        "redoStates": report.redo_states,
        "keptRows": report.kept_rows,
        "copiedRows": report.copied_rows,
        "encodedRows": report.encoded_rows,
        "savedVersions": report.saved_versions,
        "contextBytes": report.context_bytes,
        "contextReused": report.context_reused,
        "baselineHashReused": report.baseline_hash_reused,
    })
}

/// One log line: total, every phase, counts and reuse flags; no paths or names.
fn slow_line(record: &JournalWriteRecord) -> String {
    let report = &record.report;
    let phases = report
        .phases
        .iter()
        .map(|(phase, time)| format!("{phase} {:.2}", time.as_secs_f64()))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "slow journal write {:.2} s{} ({phases}); undo {}, redo {}, kept {}, copied {}, encoded {}, versions {}, context {} B{}{}",
        record.total.as_secs_f64(),
        if record.ok { "" } else { ", failed" },
        report.undo_states,
        report.redo_states,
        report.kept_rows,
        report.copied_rows,
        report.encoded_rows,
        report.saved_versions,
        report.context_bytes,
        if report.context_reused { ", context reused" } else { "" },
        if report.baseline_hash_reused { ", baseline reused" } else { "" },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(total_ms: u64) -> JournalWriteRecord {
        JournalWriteRecord {
            at: "2026-11-08T22:00:00.000Z".into(),
            total: Duration::from_millis(total_ms),
            ok: true,
            report: JournalWriteReport {
                phases: vec![("admission", Duration::from_millis(total_ms))],
                undo_states: 3,
                ..JournalWriteReport::default()
            },
        }
    }

    #[test]
    fn keeps_only_the_most_recent_writes_oldest_first() {
        let mut recent = RecentJournalWrites::default();
        for total in 0..(RECENT_JOURNAL_WRITES as u64 + 5) {
            recent.record(record(total));
        }
        let totals: Vec<_> = recent.records().map(|r| millis(r.total)).collect();
        assert_eq!(totals.len(), RECENT_JOURNAL_WRITES);
        assert_eq!(totals[0], 5);
        assert_eq!(*totals.last().unwrap(), RECENT_JOURNAL_WRITES as u64 + 4);
    }

    #[test]
    fn the_view_holds_only_timings_counts_sizes_and_flags() {
        let mut recent = RecentJournalWrites::default();
        recent.record(record(1500));
        let view = recent.to_json();
        let entry = &view["recent"][0];
        let keys: Vec<_> = entry.as_object().unwrap().keys().cloned().collect();
        assert_eq!(
            keys,
            [
                "at",
                "baselineHashReused",
                "contextBytes",
                "contextReused",
                "copiedRows",
                "encodedRows",
                "keptRows",
                "ok",
                "phasesMs",
                "redoStates",
                "savedVersions",
                "totalMs",
                "undoStates"
            ]
        );
        assert_eq!(entry["phasesMs"]["admission"], 1500);
        assert_eq!(view["slowThresholdMs"], 2000);
    }

    #[test]
    fn the_slow_line_names_phases_and_counts_only() {
        let line = slow_line(&record(2500));
        assert!(
            line.starts_with("slow journal write 2.50 s (admission 2.50); undo 3,"),
            "{line}"
        );
    }
}
