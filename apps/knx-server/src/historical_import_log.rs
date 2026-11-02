//! Projects a stored import report into session-log entries without inventing missing facts.

use crate::session_log::{grouped, member_list, now, LogEntry, Severity};
use serde_json::Value;

/// Shown where the stored row does not say; never a zero, a `null` or an empty string.
pub(crate) const NOT_RECORDED: &str = "not recorded";

/// The error and warning entries of an admitted stored report, one per counted finding.
/// Fails, listing nothing, if the projection disagrees with the admitted counts; the
/// counters stay authoritative and the caller says why the list is missing.
pub(crate) fn project(report: &Value, admitted: (usize, usize)) -> Result<Vec<LogEntry>, String> {
    let entries = entries(report);
    let counted = (
        entries
            .iter()
            .filter(|e| e.severity == Severity::Error)
            .count(),
        entries
            .iter()
            .filter(|e| e.severity == Severity::Warning)
            .count(),
    );
    if counted == admitted {
        Ok(entries)
    } else {
        Err(format!(
            "stored findings project to {} errors and {} warnings, the admitted counts are {} and {}",
            counted.0, counted.1, admitted.0, admitted.1
        ))
    }
}

fn rows<'a>(report: &'a Value, key: &str) -> &'a [Value] {
    report[key]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
}

fn text<'a>(row: &'a Value, key: &str) -> Option<&'a str> {
    row[key].as_str()
}

fn shown(row: &Value, key: &str) -> String {
    text(row, key)
        .filter(|v| !v.is_empty())
        .unwrap_or(NOT_RECORDED)
        .to_string()
}

fn number(row: &Value, key: &str) -> String {
    row[key]
        .as_u64()
        .map_or_else(|| NOT_RECORDED.to_string(), |n| n.to_string())
}

/// The original row, kept in `detail` when a display field could not be read.
fn raw_if(row: &Value, incomplete: bool) -> Option<String> {
    incomplete.then(|| row.to_string())
}

fn lacks(row: &Value, keys: &[&str]) -> bool {
    keys.iter()
        .any(|key| text(row, key).is_none_or(str::is_empty))
}

fn warning(
    source: &str,
    message: String,
    location: Option<String>,
    detail: Option<String>,
) -> LogEntry {
    LogEntry {
        timestamp: now(),
        severity: Severity::Warning,
        source: source.into(),
        message,
        location,
        diagnostic: None,
        detail,
    }
}

fn entries(report: &Value) -> Vec<LogEntry> {
    let mut out = Vec::new();
    for row in rows(report, "errors") {
        let xpath = text(row, "xpath").unwrap_or_default();
        out.push(LogEntry {
            timestamp: now(),
            severity: if row["severity"] == "error" {
                Severity::Error
            } else {
                Severity::Warning
            },
            source: format!(
                "import:{}",
                text(row, "stage")
                    .filter(|v| !v.is_empty())
                    .unwrap_or("stage-not-recorded")
            ),
            message: shown(row, "detail"),
            location: text(row, "source_path")
                .map(str::to_string)
                .or_else(|| (!xpath.is_empty()).then(|| xpath.to_string())),
            diagnostic: None,
            detail: raw_if(row, lacks(row, &["stage", "detail"])),
        });
    }
    unknown(report, &mut out);
    for row in rows(report, "conflicts") {
        let candidates = row["candidates"]
            .as_array()
            .map_or_else(|| NOT_RECORDED.to_string(), |c| c.len().to_string());
        let address = row["group_address"]
            .as_u64()
            .map_or_else(|| NOT_RECORDED.to_string(), |a| a.to_string());
        let incomplete =
            row["group_address"].as_u64().is_none() || row["candidates"].as_array().is_none();
        out.push(warning(
            "import:conflict",
            format!("group address {address} has {candidates} candidate datapoint types"),
            None,
            raw_if(row, incomplete),
        ));
    }
    unsupported(report, &mut out);
    out
}

fn unknown_group(row: &Value) -> Option<(&str, &str)> {
    knx_app::import_findings::manufacturer_unknown_group(
        text(row, "source_path").unwrap_or_default(),
        text(row, "kind").unwrap_or_default(),
        text(row, "name").unwrap_or_default(),
    )
}

fn unknown_entry(row: &Value) -> LogEntry {
    let incomplete =
        lacks(row, &["kind", "name", "source_path"]) || row["occurrences"].as_u64().is_none();
    warning(
        "import:unknown",
        format!(
            "unknown {} '{}' seen {} time(s) in {}",
            shown(row, "kind"),
            shown(row, "name"),
            number(row, "occurrences"),
            shown(row, "source_path")
        ),
        text(row, "xpath").map(str::to_string),
        raw_if(row, incomplete).or_else(|| text(row, "sample").map(str::to_string)),
    )
}

fn unknown(report: &Value, out: &mut Vec<LogEntry>) {
    let all = rows(report, "unknown");
    out.extend(
        all.iter()
            .filter(|u| unknown_group(u).is_none())
            .map(unknown_entry),
    );
    for ((kind, name), members) in grouped(all.iter().filter(|u| unknown_group(u).is_some()), |u| {
        unknown_group(u).expect("filtered to grouped rows")
    }) {
        if let [only] = members.as_slice() {
            out.push(unknown_entry(only));
            continue;
        }
        let files: std::collections::BTreeSet<_> = members
            .iter()
            .map(|u| text(u, "source_path").unwrap_or_default())
            .collect();
        // Unbounded JSON integers: a missing, non-integer or overflowing total is withheld.
        let total = members
            .iter()
            .try_fold(0_u64, |sum, u| sum.checked_add(u["occurrences"].as_u64()?))
            .map_or_else(|| NOT_RECORDED.to_string(), |n| n.to_string());
        let shown_or = |value: &str| {
            if value.is_empty() {
                NOT_RECORDED.to_string()
            } else {
                value.to_string()
            }
        };
        out.push(warning(
            "import:unknown",
            format!(
                "unknown {} '{}' in manufacturer data: {} places in {} files, {total} occurrence(s); samples in the import report",
                shown_or(kind),
                shown_or(name),
                members.len(),
                files.len()
            ),
            None,
            Some(member_list(members.iter().map(|u| {
                format!("{} {} — {}×", shown(u, "source_path"), shown(u, "xpath"), number(u, "occurrences"))
            }))),
        ));
    }
}

fn payload_label(row: &Value) -> Option<&str> {
    knx_app::retained_payload_label(
        text(row, "what").unwrap_or_default(),
        text(row, "consequence").unwrap_or_default(),
    )
}

fn unsupported_entry(row: &Value) -> LogEntry {
    warning(
        "import:unsupported",
        format!("{}: {}", shown(row, "what"), shown(row, "consequence")),
        None,
        raw_if(row, lacks(row, &["what", "consequence"])),
    )
}

fn unsupported(report: &Value, out: &mut Vec<LogEntry>) {
    let all = rows(report, "unsupported");
    out.extend(
        all.iter()
            .filter(|u| payload_label(u).is_none())
            .map(unsupported_entry),
    );
    for (label, members) in grouped(all.iter().filter(|u| payload_label(u).is_some()), |u| {
        payload_label(u).expect("filtered to labelled rows")
    }) {
        if let [only] = members.as_slice() {
            out.push(unsupported_entry(only));
            continue;
        }
        out.push(warning(
            "import:unsupported",
            format!("{} retained files: {label}", members.len()),
            None,
            Some(member_list(members.iter().map(|row| shown(row, "what")))),
        ));
    }
}
