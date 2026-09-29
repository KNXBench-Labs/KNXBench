//! Native bus-monitor snapshot validation before an atomic dialog-owned write.
use std::path::Path;

use serde_json::Value;

use crate::native_json_export::{write_atomically, MAX_EXPORT_BYTES};

fn optional_text(value: Option<&Value>) -> bool {
    value.is_some_and(|value| value.is_null() || value.is_string())
}

fn valid_row(row: &Value) -> bool {
    let text = |key| row.get(key).and_then(Value::as_str).is_some();
    let decoded = row.get("decoded").is_some_and(|value| {
        value.is_null()
            || (value.get("kind").and_then(Value::as_str).is_some()
                && value.get("text").and_then(Value::as_str).is_some()
                && (value.get("dpt").is_none() || optional_text(value.get("dpt")))
                && (value.get("error").is_none() || optional_text(value.get("error")))
                && (value.get("reason").is_none() || optional_text(value.get("reason"))))
    });
    row.get("seq").and_then(Value::as_u64).is_some()
        && text("timestamp")
        && text("source")
        && text("destination")
        && text("service")
        && optional_text(row.get("destinationName"))
        && optional_text(row.get("rawPayload"))
        && decoded
}

/// No caller-supplied path reaches this function from the WebView. The
/// native command obtains one from the OS save dialog, then validates the
/// versioned, bounded document before touching any existing destination.
pub(crate) fn write_bus_capture(path: &Path, contents: &str) -> Result<(), String> {
    if contents.len() > MAX_EXPORT_BYTES {
        return Err("Bus monitor capture exceeds the 16 MiB desktop limit".into());
    }
    let document: Value = serde_json::from_str(contents)
        .map_err(|error| format!("Invalid bus monitor capture JSON: {error}"))?;
    let valid = document.get("format").and_then(Value::as_str) == Some("knxbench-bus-monitor")
        && document.get("version").and_then(Value::as_u64) == Some(1)
        && document.get("capacity").and_then(Value::as_u64) == Some(1000)
        && document.get("sessionId").and_then(Value::as_u64).is_some()
        && document
            .get("serverIncarnation")
            .and_then(Value::as_str)
            .is_some_and(|value| !value.is_empty())
        && matches!(
            document.get("status").and_then(Value::as_str),
            Some("active" | "closed")
        )
        && document
            .get("serverDroppedBefore")
            .and_then(Value::as_u64)
            .is_some()
        && document
            .get("clientPrunedCount")
            .and_then(Value::as_u64)
            .is_some()
        && document
            .get("exportedAt")
            .and_then(Value::as_str)
            .is_some_and(|value| !value.is_empty())
        && document.get("notice").and_then(Value::as_str).is_some()
        && document
            .get("rows")
            .and_then(Value::as_array)
            .is_some_and(|rows| rows.len() <= 1000 && rows.iter().all(valid_row));
    if !valid {
        return Err("Unsupported bus monitor capture document".into());
    }
    write_atomically(path, contents, "bus monitor capture")
}
