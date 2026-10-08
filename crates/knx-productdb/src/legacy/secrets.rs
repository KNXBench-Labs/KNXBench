//! Withholds secret-class column values from a legacy payload before it is stored.
//!
//! A column whose name contains `PASSWORD` (any case) is secret-class
//! (design 2026-09-26 §6.4, decision B-3; ADR-0094). Observed names are
//! `PROJECT_PASSWORD`, `PROJECT_BCU_PASSWORD` and `DEVICE_BCU_PASSWORD`. The
//! stored copy of a payload keeps every byte except those values: each
//! non-empty one, continuation lines included, becomes an empty value line,
//! which the grammar accepts. Only the table, the column and the number of
//! withheld values are reported, never a value.

use super::exim::parse_exim;
use super::LegacyError;

/// One secret-class column whose values were withheld.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretColumn {
    pub table: String,
    pub column: String,
    /// Rows whose value was non-empty and is now blank.
    pub rows: usize,
}

/// A payload with its secret-class values withheld.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WithheldPayload {
    /// The payload to store. Equal to the input when nothing was withheld.
    pub bytes: Vec<u8>,
    /// In table order, then column order; only columns with withheld values.
    pub columns: Vec<SecretColumn>,
}

/// `true` for a secret-class column name.
pub fn is_secret_column(name: &str) -> bool {
    name.to_ascii_uppercase().contains("PASSWORD")
}

/// Blanks every non-empty secret-class value of `payload`. A payload the
/// grammar refuses is refused here too: nothing is half-withheld.
pub fn withhold_secret_values(payload: &[u8]) -> Result<WithheldPayload, LegacyError> {
    let document = parse_exim(payload)?;
    let mut ranges = Vec::new();
    let mut columns = Vec::new();
    for table in document.tables() {
        for (index, column) in table.columns().iter().enumerate() {
            if !is_secret_column(&column.name) {
                continue;
            }
            let mut rows = 0;
            for row in 0..table.row_count() {
                let range = table.source_range(row, index);
                if !range.is_empty() {
                    ranges.push(range);
                    rows += 1;
                }
            }
            if rows > 0 {
                columns.push(SecretColumn {
                    table: table.name().to_string(),
                    column: column.name.clone(),
                    rows,
                });
            }
        }
    }
    if ranges.is_empty() {
        return Ok(WithheldPayload {
            bytes: payload.to_vec(),
            columns,
        });
    }
    ranges.sort_by_key(|r| r.start);
    let mut bytes = Vec::with_capacity(payload.len());
    let mut at = 0;
    for range in ranges {
        bytes.extend_from_slice(&payload[at..range.start]);
        at = range.end;
    }
    bytes.extend_from_slice(&payload[at..]);
    Ok(WithheldPayload { bytes, columns })
}
