//! Copies a native project store with retained secrets blanked, history dropped, source untouched.
//!
//! One bounded byte snapshot is placed in private scratch before any SQLite
//! admission; WAL and journal-bearing sources are refused without opening them.
//! The private snapshot is admitted read-only (older schemas migrate in memory)
//! and copied with SQLite's `VACUUM INTO`. Only that private copy is
//! changed; a second `VACUUM INTO` then writes the result without free
//! pages, so no deleted or overwritten value survives in unused space.
//!
//! Where a native store keeps secret material, and what happens to it:
//!
//! - Ordinary/selective retained attributes named in
//!   [`super::SECRET_NAMES`]: value blanked, SHA-256 recomputed.
//! - `RetainedElement` rows (raw XML such as a device's `Security`,
//!   `BusAccess` or `DeviceCertificates`): redacted as XML; a fragment that
//!   cannot be inspected is withheld (row deleted).
//! - Whole-file rows (container entries, product files kept without a
//!   product database): XML is redacted, nested archives are rewritten,
//!   UTF-16 text is withheld, other binary payloads are kept.
//! - Primary evidence and selective reports share semantic sample/digest redaction.
//!   Secret-bearing historical row identities make that report unavailable.
//! - `manufacturer_file` rows: keyed by their SHA-256, so they cannot be
//!   edited; one that names a secret or cannot be inspected is withheld.
//! - `project_history_*`: every snapshot embeds the retained rows above in
//!   compressed form, so the whole history is left out.

use std::path::Path;

use knx_store::Connection;

use super::archive;
use super::xml::{self, Base, Content};
use super::{
    io_error, is_secret_name, is_secret_value, Findings, SecretFreeCopy, SecretFreeCopyError,
};

const RETAINED_ATTRIBUTE: &str = "RetainedAttribute";
const RETAINED_ELEMENT: &str = "RetainedElement";
const IMPORT_EVIDENCE: &str = "KnxBenchImportEvidence";
/// A nested archive in a whole-file row is level 1, as in a `.knxproj`.
const ROW_ARCHIVE_DEPTH: usize = 1;

pub(super) fn copy(source: &Path) -> Result<SecretFreeCopy, SecretFreeCopyError> {
    copy_observed(source, |_| {})
}

pub(super) fn copy_observed(
    source: &Path,
    allocated: impl FnOnce(&Path),
) -> Result<SecretFreeCopy, SecretFreeCopyError> {
    copy_with_budget(source, allocated, super::budget::Budget::default())
}

pub(super) fn copy_with_budget(
    source: &Path,
    allocated: impl FnOnce(&Path),
    budget: super::budget::Budget,
) -> Result<SecretFreeCopy, SecretFreeCopyError> {
    refuse_journals(source)?;
    let before = std::fs::metadata(source).map_err(io_error("reading source identity"))?;
    let modified = before
        .modified()
        .map_err(io_error("reading source modification time"))?;
    let original = archive::read_source_with_limit(source, super::budget::NATIVE_BYTES)?;
    let after = std::fs::metadata(source).map_err(io_error("rechecking source identity"))?;
    if before.len() != after.len()
        || modified
            != after
                .modified()
                .map_err(io_error("rechecking source modification time"))?
    {
        return Err(SecretFreeCopyError::SourceUnavailable {
            reason: "source changed during snapshot acquisition",
        });
    }
    refuse_journals(source)?;
    if original
        .get(18..20)
        .is_some_and(|version| version.contains(&2))
    {
        return Err(SecretFreeCopyError::SourceUnavailable {
            reason: "WAL-mode source requires a quiescent saved-file checkpoint",
        });
    }
    let scratch = tempfile::Builder::new()
        .prefix("knxbench-secret-free-")
        .tempdir()
        .map_err(io_error("creating a scratch directory"))?;
    allocated(scratch.path());
    let snapshot = scratch.path().join("source.knxdb");
    std::fs::write(&snapshot, &original).map_err(io_error("writing private source snapshot"))?;
    drop(original);
    let admitted = knx_store::open_existing_read_only(&snapshot)?;
    let work = scratch.path().join("work.knxdb");
    // `query_only` also refuses `VACUUM INTO`, which writes only its target.
    // The file connection stays opened read-only, so the source still
    // cannot be written; an older schema's connection is an in-memory copy.
    admitted.conn.pragma_update(None, "query_only", false)?;
    vacuum_into(&admitted.conn, &work)?;
    drop(admitted);

    let work_size = std::fs::metadata(&work)
        .map_err(io_error("reading snapshot size"))?
        .len();
    if work_size > super::budget::NATIVE_BYTES {
        return Err(SecretFreeCopyError::TooLarge {
            bytes: work_size,
            limit: super::budget::NATIVE_BYTES,
        });
    }
    let conn = knx_store::open_existing_and_migrate(&work)?;
    // Discover from the complete immutable scratch snapshot, then roll back
    // discovery edits before enforcing the global set on every logical surface.
    let mut learned = Findings {
        budget,
        ..Findings::default()
    };
    let discovery = conn.unchecked_transaction()?;
    redact_rows(&discovery, &mut learned, None)?;
    withhold_carried_files(&discovery, &mut learned, None)?;
    discovery.rollback()?;
    let needles = learned.needles();
    let mut findings = learned.child();
    let tx = conn.unchecked_transaction()?;
    redact_rows(&tx, &mut findings, Some(&needles))?;
    withhold_carried_files(&tx, &mut findings, Some(&needles))?;
    drop_history(&tx, &mut findings)?;
    withhold_residual_rows(&tx, &mut findings)?;
    check_logical_values(&tx, &needles)?;
    tx.commit()?;

    let finished = scratch.path().join("secret-free.knxdb");
    vacuum_into(&conn, &finished)?;
    drop(conn);
    reopen(&finished)?;
    let size = std::fs::metadata(&finished)
        .map_err(io_error("reading redacted size"))?
        .len();
    if size > super::budget::NATIVE_BYTES {
        return Err(SecretFreeCopyError::TooLarge {
            bytes: size,
            limit: super::budget::NATIVE_BYTES,
        });
    }
    let bytes = std::fs::read(&finished).map_err(io_error("reading the redacted copy"))?;
    // Values inside SQLite overflow chains can straddle a page boundary, so
    // this scan is a last net for domain text, not the primary check.
    let occurrences = findings.needles().count_in(&bytes, usize::MAX);
    findings.ensure()?;
    if occurrences > 0 {
        return Err(SecretFreeCopyError::ResidualSecret { occurrences });
    }
    findings.finish(bytes)
}

fn refuse_journals(source: &Path) -> Result<(), SecretFreeCopyError> {
    for suffix in ["-wal", "-shm", "-journal"] {
        let mut path = source.as_os_str().to_os_string();
        path.push(suffix);
        match std::fs::symlink_metadata(Path::new(&path)) {
            Ok(_) => {
                return Err(SecretFreeCopyError::SourceUnavailable {
                    reason: "SQLite journal/sidecar present; a quiescent saved file is required",
                })
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(io_error("checking source journals")(error)),
        }
    }
    Ok(())
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

fn redact_rows(
    tx: &Connection,
    findings: &mut Findings,
    residual: Option<&super::Needles<'_>>,
) -> Result<(), SecretFreeCopyError> {
    let (count, residency): (i64,i64) = tx.query_row("SELECT count(*),coalesce(sum(length(bytes)+length(source_path)+length(xpath)+length(kind)+length(name)),0) FROM opaque_entry",[],|row|Ok((row.get(0)?,row.get(1)?)))?;
    if count > 262144 {
        return Err(SecretFreeCopyError::BudgetExceeded {
            resource: "native retained row count budget exceeded",
        });
    }
    findings
        .budget
        .decoded(usize::try_from(residency).unwrap_or(usize::MAX))
        .map_err(|resource| SecretFreeCopyError::BudgetExceeded { resource })?;
    let ids = tx
        .prepare("SELECT id FROM opaque_entry ORDER BY id")?
        .query_map([], |row| row.get::<_, i64>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    for id in ids {
        findings.ensure()?;
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
        let mut row = findings.child();
        match redact_row((&kind, &name, &xpath, &location), bytes, &mut row, residual)? {
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
    identity: (&str, &str, &str, &str),
    bytes: Vec<u8>,
    findings: &mut Findings,
    residual: Option<&super::Needles<'_>>,
) -> Result<Row, SecretFreeCopyError> {
    let (kind, name, xpath, location) = identity;
    match kind {
        RETAINED_ATTRIBUTE | crate::selective_import::SELECTIVE_ATTRIBUTE_KIND => {
            Ok(redact_attribute(name, xpath, location, &bytes, findings))
        }
        RETAINED_ELEMENT => {
            let base = Base {
                text: location,
                root_in_base: true,
            };
            Ok(xml_row(&bytes, &base, findings))
        }
        IMPORT_EVIDENCE | "SelectiveImportReport" => Ok(
            match super::report::redact_evidence(&bytes, location, findings, residual) {
                Ok(Some(redacted)) => Row::Replace(redacted),
                Ok(None) => Row::Keep,
                Err(reason) => Row::Withhold(reason.into()),
            },
        ),
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
                let mut inner = findings.child();
                match archive::nested(bytes, location, ROW_ARCHIVE_DEPTH, &mut inner, residual)? {
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
    if super::local_name(name.as_bytes()) == b"Parameter"
        && xpath.rsplit('/').next().is_some_and(|part| {
            super::local_name(part.split('[').next().unwrap_or(part).as_bytes()) == b"BusAccess"
        })
    {
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

fn withhold_carried_files(
    tx: &Connection,
    findings: &mut Findings,
    residual: Option<&super::Needles<'_>>,
) -> Result<(), SecretFreeCopyError> {
    let hashes = tx
        .prepare("SELECT sha256 FROM manufacturer_file ORDER BY sha256")?
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    for sha256 in hashes {
        findings.ensure()?;
        let size: i64 = tx.query_row(
            "SELECT len FROM manufacturer_file WHERE sha256=?1",
            [&sha256],
            |r| r.get(0),
        )?;
        findings
            .budget
            .decoded(usize::try_from(size).unwrap_or(usize::MAX))
            .map_err(|resource| SecretFreeCopyError::BudgetExceeded { resource })?;
        let mut part = findings.child();
        let (location, reason) = match knx_store::load_manufacturer_file(tx, &sha256) {
            Ok(Some(file)) => {
                let reason = inspect_carried(&file.bytes, &file.source_path, &mut part, residual)?;
                match reason {
                    Some(reason) => (file.source_path, reason),
                    None if residual.is_some_and(|n| n.any_in(&file.bytes)) => (
                        file.source_path,
                        "decoded file still contains a removed secret value".into(),
                    ),
                    None => continue,
                }
            }
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
    residual: Option<&super::Needles<'_>>,
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
            let nested = archive::nested(
                bytes.to_vec(),
                location,
                ROW_ARCHIVE_DEPTH,
                findings,
                residual,
            )?;
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

/// Inspect live logical TEXT/BLOB values, never relying on SQLite physical page layout.
fn check_logical_values(
    tx: &Connection,
    needles: &super::Needles<'_>,
) -> Result<(), SecretFreeCopyError> {
    let tables = tx
        .prepare("SELECT name FROM sqlite_schema WHERE type='table' ORDER BY name")?
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    for table in tables {
        // These payloads have dedicated decoded inspections; still check their identities.
        let sql = match table.as_str() {
            "opaque_entry" => "SELECT source_path,xpath,kind,name FROM opaque_entry".to_string(),
            "manufacturer_file" => "SELECT source_path FROM manufacturer_file".to_string(),
            _ => format!("SELECT * FROM \"{}\"", table.replace('"', "\"\"")),
        };
        let mut statement = tx.prepare(&sql)?;
        let columns = statement.column_count();
        let mut rows = statement.query([])?;
        while let Some(row) = rows.next()? {
            for column in 0..columns {
                if let Ok(bytes) = row.get_ref(column)?.as_bytes() {
                    if needles.any_in(bytes) {
                        needles
                            .budget
                            .check()
                            .map_err(|resource| SecretFreeCopyError::BudgetExceeded { resource })?;
                        return Err(SecretFreeCopyError::ResidualSecret { occurrences: 1 });
                    }
                }
            }
        }
    }
    Ok(())
}

/// The copy must open the way every reader opens a project.
fn reopen(path: &Path) -> Result<(), SecretFreeCopyError> {
    let reopen = |error: &dyn std::fmt::Display| SecretFreeCopyError::Reopen(error.to_string());
    let store = knx_store::open_existing_read_only(path).map_err(|e| reopen(&e))?;
    knx_store::load_project(&store.conn).map_err(|e| reopen(&e))?;
    let opaque = knx_store::load_opaque(&store.conn).map_err(|e| reopen(&e))?;
    crate::import_evidence::read(&opaque).map_err(|e| reopen(&e))?;
    crate::import_evidence::source_reports::catalogue(&opaque).map_err(|e| reopen(&e))?;
    knx_store::project_history::load_editor(&store.conn).map_err(|e| reopen(&e))?;
    Ok(())
}
