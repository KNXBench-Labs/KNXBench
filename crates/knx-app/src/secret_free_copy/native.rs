//! Copies a native project store with retained secrets blanked, history dropped, source untouched.
//!
//! The source is admitted by `knx_store::open_existing_read_only` (an older
//! schema is migrated in memory, never on disk) and copied with SQLite's
//! `VACUUM INTO` into a private scratch directory. Only that copy is
//! changed; a second `VACUUM INTO` then writes the result without free
//! pages, so no deleted or overwritten value survives in unused space.
//!
//! Where a native store keeps secret material, and what happens to it:
//!
//! - `opaque_entry` rows of kind `RetainedAttribute` named in
//!   [`super::SECRET_NAMES`]: value blanked, SHA-256 recomputed.
//! - `RetainedElement` rows (raw XML such as a device's `Security`,
//!   `BusAccess` or `DeviceCertificates`): redacted as XML; a fragment that
//!   cannot be inspected is withheld (row deleted).
//! - Whole-file rows (container entries, product files kept without a
//!   product database): XML is redacted, nested archives are rewritten,
//!   UTF-16 text is withheld, other binary payloads are kept.
//! - The import evidence row (`KnxBenchImportEvidence`): the digest and any
//!   sample of a secret-named row in the embedded report is blanked.
//! - `manufacturer_file` rows: keyed by their SHA-256, so they cannot be
//!   edited; one that names a secret or cannot be inspected is withheld.
//! - `project_history_*`: every snapshot embeds the retained rows above in
//!   compressed form, so the whole history is left out.

use std::path::Path;

use knx_store::Connection;
use serde_json::Value;

use super::archive;
use super::xml::{self, Base, Content};
use super::{
    io_error, is_secret_name, is_secret_value, Findings, SecretFreeCopy, SecretFreeCopyError,
    FREE_ACCESS_KEY,
};

const RETAINED_ATTRIBUTE: &str = "RetainedAttribute";
const RETAINED_ELEMENT: &str = "RetainedElement";
const IMPORT_EVIDENCE: &str = "KnxBenchImportEvidence";
/// A nested archive in a whole-file row is level 1, as in a `.knxproj`.
const ROW_ARCHIVE_DEPTH: usize = 1;

pub(super) fn copy(source: &Path) -> Result<SecretFreeCopy, SecretFreeCopyError> {
    let admitted = knx_store::open_existing_read_only(source)?;
    let scratch = tempfile::Builder::new()
        .prefix("knxbench-secret-free-")
        .tempdir()
        .map_err(io_error("creating a scratch directory"))?;
    let work = scratch.path().join("work.knxdb");
    // `query_only` also refuses `VACUUM INTO`, which writes only its target.
    // The file connection stays opened read-only, so the source still
    // cannot be written; an older schema's connection is an in-memory copy.
    admitted.conn.pragma_update(None, "query_only", false)?;
    vacuum_into(&admitted.conn, &work)?;
    drop(admitted);

    let conn = knx_store::open_existing_and_migrate(&work)?;
    let mut findings = Findings::default();
    let tx = conn.unchecked_transaction()?;
    redact_rows(&tx, &mut findings)?;
    withhold_carried_files(&tx, &mut findings)?;
    drop_history(&tx, &mut findings)?;
    withhold_residual_rows(&tx, &mut findings)?;
    tx.commit()?;

    let finished = scratch.path().join("secret-free.knxdb");
    vacuum_into(&conn, &finished)?;
    drop(conn);
    reopen(&finished)?;
    let bytes = std::fs::read(&finished).map_err(io_error("reading the redacted copy"))?;
    // Values inside SQLite overflow chains can straddle a page boundary, so
    // this scan is a last net for domain text, not the primary check.
    let occurrences = findings.needles().count_in(&bytes, usize::MAX);
    if occurrences > 0 {
        return Err(SecretFreeCopyError::ResidualSecret { occurrences });
    }
    Ok(findings.finish(bytes))
}

fn vacuum_into(conn: &Connection, target: &Path) -> Result<(), SecretFreeCopyError> {
    let target = target.to_str().ok_or(SecretFreeCopyError::Io {
        stage: "naming the scratch copy",
        kind: std::io::ErrorKind::InvalidInput,
    })?;
    conn.execute("VACUUM INTO ?1", [target])?;
    Ok(())
}

/// What becomes of one retained row.
enum Row {
    Keep,
    Replace(Vec<u8>),
    Withhold(String),
}

fn redact_rows(tx: &Connection, findings: &mut Findings) -> Result<(), SecretFreeCopyError> {
    let ids = tx
        .prepare("SELECT id FROM opaque_entry ORDER BY id")?
        .query_map([], |row| row.get::<_, i64>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    for id in ids {
        let (source_path, xpath, kind, name, bytes): (String, String, String, String, Vec<u8>) = tx
            .query_row(
                "SELECT source_path, xpath, kind, name, bytes FROM opaque_entry WHERE id = ?1",
                [id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )?;
        let location = format!("{source_path}{xpath}");
        let mut row = Findings::default();
        match redact_row(&kind, &name, &xpath, &location, bytes, &mut row)? {
            Row::Keep => findings.absorb(row),
            Row::Replace(bytes) => {
                let sha256 = knx_store::project_history::image_hash(&bytes);
                tx.execute(
                    "UPDATE opaque_entry SET bytes = ?1, sha256 = ?2 WHERE id = ?3",
                    (&bytes, &sha256, id),
                )?;
                findings.absorb(row);
            }
            Row::Withhold(reason) => {
                tx.execute("DELETE FROM opaque_entry WHERE id = ?1", [id])?;
                findings.values.append(&mut row.values);
                findings.withhold(&format!("retained {location}"), &reason);
            }
        }
    }
    Ok(())
}

fn redact_row(
    kind: &str,
    name: &str,
    xpath: &str,
    location: &str,
    bytes: Vec<u8>,
    findings: &mut Findings,
) -> Result<Row, SecretFreeCopyError> {
    match kind {
        RETAINED_ATTRIBUTE => Ok(redact_attribute(name, xpath, location, &bytes, findings)),
        RETAINED_ELEMENT => {
            let base = Base {
                text: location,
                root_in_base: true,
            };
            Ok(xml_row(&bytes, &base, findings))
        }
        IMPORT_EVIDENCE => Ok(match redact_evidence(&bytes, location, findings) {
            Ok(Some(redacted)) => Row::Replace(redacted),
            Ok(None) => Row::Keep,
            Err(reason) => Row::Withhold(reason.into()),
        }),
        _ => match xml::sniff(&bytes) {
            Content::Xml => {
                let base = format!("{location}:");
                let base = Base {
                    text: &base,
                    root_in_base: false,
                };
                Ok(xml_row(&bytes, &base, findings))
            }
            Content::Zip => {
                let mut inner = Findings::default();
                match archive::nested(bytes, location, ROW_ARCHIVE_DEPTH, &mut inner, None)? {
                    Ok(rewritten) => {
                        let changed = !inner.redactions.is_empty() || !inner.withheld.is_empty();
                        findings.absorb(inner);
                        Ok(if changed {
                            Row::Replace(rewritten)
                        } else {
                            Row::Keep
                        })
                    }
                    Err(reason) => {
                        findings.values.append(&mut inner.values);
                        Ok(Row::Withhold(reason))
                    }
                }
            }
            Content::WideText => Ok(Row::Withhold("UTF-16/32 text is not inspected".into())),
            Content::Other => Ok(Row::Keep),
        },
    }
}

fn xml_row(bytes: &[u8], base: &Base<'_>, findings: &mut Findings) -> Row {
    match xml::redact(bytes, base, findings) {
        Ok(Some(redacted)) => Row::Replace(redacted),
        Ok(None) => Row::Keep,
        Err(reason) => Row::Withhold(reason.into()),
    }
}

/// A retained attribute's stored value is the attribute's decoded text.
fn redact_attribute(
    name: &str,
    xpath: &str,
    location: &str,
    value: &[u8],
    findings: &mut Findings,
) -> Row {
    if is_secret_name(name.as_bytes()) && is_secret_value(name.as_bytes(), value) {
        findings.remember(value);
        findings.redact(format!("{location}/@{name}"), name.to_string());
        return Row::Replace(Vec::new());
    }
    if name == "Parameter" && xpath.ends_with("/BusAccess") {
        let Ok(text) = std::str::from_utf8(value) else {
            return Row::Withhold("connection string is not UTF-8".into());
        };
        let location = format!("{location}/@Parameter");
        if let Some(redacted) = xml::redact_connection_string(text, &location, findings) {
            return Row::Replace(redacted.into_bytes());
        }
    }
    Row::Keep
}

/// The import evidence envelope embeds the import report as a JSON string.
/// Rows of either that are named like a secret lose their digest, sample
/// and value: a SHA-256 of a four-octet key or a password is reversible by
/// search.
fn redact_evidence(
    bytes: &[u8],
    location: &str,
    findings: &mut Findings,
) -> Result<Option<Vec<u8>>, &'static str> {
    let mut envelope: Value =
        serde_json::from_slice(bytes).map_err(|_| "import evidence is not JSON")?;
    let mut changed = redact_json(&mut envelope, location, findings);
    if let Some(Value::String(embedded)) = envelope.get_mut("report_json") {
        let mut report: Value =
            serde_json::from_str(embedded).map_err(|_| "embedded import report is not JSON")?;
        if redact_json(&mut report, location, findings) {
            *embedded = serde_json::to_string(&report)
                .map_err(|_| "embedded import report could not be rewritten")?;
            changed = true;
        }
    }
    if !changed {
        return Ok(None);
    }
    serde_json::to_vec(&envelope)
        .map(Some)
        .map_err(|_| "import evidence could not be rewritten")
}

fn redact_json(value: &mut Value, location: &str, findings: &mut Findings) -> bool {
    match value {
        Value::Object(map) => {
            let mut changed = false;
            let secret = map
                .get("name")
                .and_then(Value::as_str)
                .filter(|name| is_secret_name(name.as_bytes()))
                .map(str::to_owned);
            if let Some(name) = secret {
                let free_access_digest = knx_store::project_history::image_hash(FREE_ACCESS_KEY);
                let text = |key: &str| map.get(key).and_then(Value::as_str).unwrap_or_default();
                let place = format!(
                    "{location}:{}{}/@{name}",
                    text("source_path"),
                    text("xpath")
                );
                for field in ["sha256", "sample", "value", "raw_text_preview"] {
                    let Some(slot) = map.get_mut(field) else {
                        continue;
                    };
                    if slot.is_null() || slot.as_str() == Some("") {
                        continue;
                    }
                    // The digest of the `BCUKey` free-access sentinel names
                    // no key; it stays, as the value itself does.
                    if field == "sha256"
                        && name == "BCUKey"
                        && slot.as_str() == Some(free_access_digest.as_str())
                    {
                        continue;
                    }
                    if field != "sha256" {
                        if let Some(plain) = slot.as_str() {
                            findings.remember(plain.as_bytes());
                        }
                    }
                    *slot = if field == "raw_text_preview" {
                        Value::Null
                    } else {
                        Value::String(String::new())
                    };
                    findings.redact(place.clone(), format!("{name} {field}"));
                    changed = true;
                }
            }
            for child in map.values_mut() {
                changed |= redact_json(child, location, findings);
            }
            changed
        }
        Value::Array(items) => items.iter_mut().fold(false, |changed, item| {
            redact_json(item, location, findings) | changed
        }),
        _ => false,
    }
}

fn withhold_carried_files(
    tx: &Connection,
    findings: &mut Findings,
) -> Result<(), SecretFreeCopyError> {
    let hashes = tx
        .prepare("SELECT sha256 FROM manufacturer_file ORDER BY sha256")?
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    for sha256 in hashes {
        let mut part = Findings::default();
        let (location, reason) = match knx_store::load_manufacturer_file(tx, &sha256) {
            Ok(Some(file)) => match inspect_carried(&file.bytes, &file.source_path, &mut part)? {
                Some(reason) => (file.source_path, reason),
                None => continue,
            },
            Ok(None) => continue,
            Err(_) => (sha256.clone(), "cannot be decoded".to_string()),
        };
        tx.execute("DELETE FROM manufacturer_file WHERE sha256 = ?1", [&sha256])?;
        findings.values.append(&mut part.values);
        findings.withhold(&format!("carried product file {location}"), &reason);
    }
    Ok(())
}

/// Why a carried product file must be left out, if it must.
fn inspect_carried(
    bytes: &[u8],
    location: &str,
    findings: &mut Findings,
) -> Result<Option<String>, SecretFreeCopyError> {
    const NAMES_A_SECRET: &str =
        "names a secret; its bytes are its identity, so it is left out rather than edited";
    match xml::sniff(bytes) {
        Content::Xml => {
            let base = format!("{location}:");
            let base = Base {
                text: &base,
                root_in_base: false,
            };
            Ok(match xml::redact(bytes, &base, findings) {
                Ok(Some(_)) => Some(NAMES_A_SECRET.into()),
                Ok(None) => None,
                Err(reason) => Some(reason.into()),
            })
        }
        Content::Zip => {
            let nested =
                archive::nested(bytes.to_vec(), location, ROW_ARCHIVE_DEPTH, findings, None)?;
            Ok(match nested {
                Err(reason) => Some(reason),
                Ok(_) if !findings.redactions.is_empty() || !findings.withheld.is_empty() => {
                    Some(NAMES_A_SECRET.into())
                }
                Ok(_) => None,
            })
        }
        Content::WideText => Ok(Some("UTF-16/32 text is not inspected".into())),
        Content::Other => Ok(None),
    }
}

fn drop_history(tx: &Connection, findings: &mut Findings) -> Result<(), SecretFreeCopyError> {
    let count = |table: &str| -> Result<i64, knx_store::SqlError> {
        tx.query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
            row.get(0)
        })
    };
    let states = count("project_history_stack")?;
    let versions = count("project_history_version")?;
    let workspace = count("project_history_state")?;
    let contexts = count("project_history_context")?;
    if states + versions + workspace + contexts == 0 {
        return Ok(());
    }
    tx.execute_batch(
        "DELETE FROM project_history_stack;
         DELETE FROM project_history_version;
         DELETE FROM project_history_state;
         DELETE FROM project_history_context;",
    )?;
    findings.withhold(
        "project history",
        &format!(
            "{states} undo/redo state(s), {versions} saved version(s) and the recovery \
             workspace left out; their snapshots embed retained source"
        ),
    );
    Ok(())
}

/// Second net: a retained row that still contains a removed value (the
/// same key copied into an unrelated payload, say) is withheld.
fn withhold_residual_rows(
    tx: &Connection,
    findings: &mut Findings,
) -> Result<(), SecretFreeCopyError> {
    let mut residual = Vec::new();
    {
        let needles = findings.needles();
        let mut stmt = tx.prepare("SELECT id, source_path, xpath, bytes FROM opaque_entry")?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let bytes: Vec<u8> = row.get(3)?;
            if needles.any_in(&bytes) {
                let location = format!("{}{}", row.get::<_, String>(1)?, row.get::<_, String>(2)?);
                residual.push((row.get::<_, i64>(0)?, location));
            }
        }
    }
    for (id, location) in residual {
        tx.execute("DELETE FROM opaque_entry WHERE id = ?1", [id])?;
        findings.withhold(
            &format!("retained {location}"),
            "still contains a removed secret value",
        );
    }
    Ok(())
}

/// The copy must open the way every reader opens a project.
fn reopen(path: &Path) -> Result<(), SecretFreeCopyError> {
    let reopen = |error: &dyn std::fmt::Display| SecretFreeCopyError::Reopen(error.to_string());
    let store = knx_store::open_existing_read_only(path).map_err(|e| reopen(&e))?;
    knx_store::load_project(&store.conn).map_err(|e| reopen(&e))?;
    knx_store::load_opaque(&store.conn).map_err(|e| reopen(&e))?;
    knx_store::project_history::load_editor(&store.conn).map_err(|e| reopen(&e))?;
    Ok(())
}
