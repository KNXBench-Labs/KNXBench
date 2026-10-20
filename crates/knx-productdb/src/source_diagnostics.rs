//! Source-bound persisted ingest diagnostics, shared by first import and retry.
use crate::ProductDbError;
use rusqlite::{Connection, OptionalExtension};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceDiagnostic {
    pub xpath: String,
    pub kind: String,
    pub name: String,
    pub occurrences: u32,
    pub sample: Option<String>,
}
/// None is unavailable/unmeasured. An empty vector is explicitly measured zero.
/// Conflict rows are keyed by the retained winner; their sample identifies the
/// incoming source, so both directions must be included without leaking hashes.
pub fn source_diagnostics(
    conn: &Connection,
    sha256: &str,
) -> Result<Option<Vec<SourceDiagnostic>>, ProductDbError> {
    let measured: Option<i64> = conn
        .query_row(
            "SELECT 1 FROM source_parse_evidence WHERE sha256=?1",
            [sha256],
            |r| r.get(0),
        )
        .optional()?;
    // None must continue to mean unmeasured. A named repair refusal is an
    // error, not a parse marker or a synthetic measured diagnostic inventory.
    if measured.is_none() {
        let failure: Option<(String,String,i64,Option<String>)> = conn.query_row(
            "SELECT xpath,name,occurrences,sample FROM ingest_unknown WHERE source_sha256=?1 AND kind='PromotionRepairError' ORDER BY id LIMIT 1",
            [sha256],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional()?;
        if let Some((path, name, count, sample)) = failure {
            if path.is_empty()
                || name != "promotion_repair"
                || count != 1
                || sample.as_deref().is_none_or(str::is_empty)
            {
                return Err(ProductDbError::Xml {
                    source_path: "stored ingest diagnostics".into(),
                    cause: "invalid persisted diagnostic kind or count".into(),
                });
            }
            return Err(ProductDbError::Xml {
                source_path: path,
                cause: format!(
                    "unmeasured source: promotion repair failed: {}",
                    sample.unwrap_or_default()
                ),
            });
        }
        return Ok(None);
    }
    let mut stmt = conn.prepare("SELECT xpath,kind,name,occurrences,sample FROM ingest_unknown WHERE (source_sha256=?1 AND kind != 'IdConflict') OR (kind='IdConflict' AND (sample=?1 OR source_sha256=?1)) ORDER BY xpath,kind,name,occurrences,sample")?;
    let rows = stmt.query_map([sha256], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, i64>(3)?,
            r.get::<_, Option<String>>(4)?,
        ))
    })?;
    let mut findings = Vec::new();
    for row in rows {
        let (xpath, kind, name, count, sample) = row?;
        let invalid = || ProductDbError::Xml {
            source_path: "stored ingest diagnostics".into(),
            cause: "invalid persisted diagnostic kind or count".into(),
        };
        if !matches!(
            kind.as_str(),
            "Element"
                | "Attribute"
                | "IdConflict"
                | "BaggageIndexParseError"
                | "PromotionRepairError"
        ) {
            return Err(invalid());
        }
        let occurrences = u32::try_from(count).map_err(|_| invalid())?;
        if occurrences == 0
            || (kind == "PromotionRepairError"
                && (occurrences != 1
                    || name != "promotion_repair"
                    || xpath.is_empty()
                    || sample.as_deref().is_none_or(str::is_empty)))
        {
            return Err(invalid());
        }
        findings.push(SourceDiagnostic {
            xpath,
            name,
            occurrences,
            sample: if kind == "IdConflict" { None } else { sample },
            kind,
        });
    }
    Ok(Some(findings))
}
