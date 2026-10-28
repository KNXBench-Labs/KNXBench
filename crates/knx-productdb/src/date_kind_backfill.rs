//! v28 -> v29 (WP-08): re-kinds stored `TypeDate` types as `Date`, exactly like a fresh ingest.
//!
//! A v28 parser filed `TypeDate` under `Other` and reported the child as an
//! unknown element. Per stored program blob that still carries that element
//! row, `parse::program::backfill_type_child_kinds` re-kinds the winning rows
//! and swaps the element row for the attribute rows (`Encoding`) a v29 ingest
//! writes. Each blob's row change is then applied to every package that has
//! it as an `ApplicationProgram` member: `package.unknown_count` by the
//! distinct-row change, a measured install report by the occurrence change.
//! A blob that cannot be re-read keeps its rows and gets a
//! `ParameterKindBackfillError`; a report that cannot follow exactly becomes
//! `unavailable` with an `InstallReportBackfillError`. Nothing is guessed.

use std::collections::{BTreeMap, HashMap, HashSet};

use rusqlite::{params, Connection};

use crate::ingest::{classify, FileKind};
use crate::package::ReportNotUpgradable;
use crate::ProductDbError;

const CHILDREN: [&str; 1] = ["TypeDate"];
const SAVEPOINT: &str = "v29_date_kind";

/// `(xpath, kind, name)` of one `Element`/`Attribute` unknown row.
pub(crate) type Key = (String, String, String);
/// `occurrences` and `sample` of one row.
type Row = (i64, Option<String>);

/// What one blob's backfill changed in its own unknown rows.
#[derive(Debug, Default)]
pub(crate) struct BlobDelta {
    /// Rows whose occurrences changed, with the row's sample afterwards.
    pub(crate) changes: Vec<(Key, i64, Option<String>)>,
    /// Distinct rows after minus distinct rows before.
    pub(crate) distinct: i64,
    /// A row present before and after whose sample changed. A merged report
    /// row cannot tell whose sample it keeps, so such a report is downgraded.
    pub(crate) sample_changed: bool,
}

fn blob_rows(conn: &Connection, sha256: &str) -> Result<BTreeMap<Key, Row>, ProductDbError> {
    let mut stmt = conn.prepare(
        "SELECT xpath, kind, name, occurrences, sample FROM ingest_unknown
         WHERE source_sha256 = ?1 AND kind IN ('Element', 'Attribute')",
    )?;
    let rows = stmt
        .query_map([sha256], |r| {
            Ok(((r.get(0)?, r.get(1)?, r.get(2)?), (r.get(3)?, r.get(4)?)))
        })?
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    Ok(rows)
}

fn delta_between(before: &BTreeMap<Key, Row>, after: &BTreeMap<Key, Row>) -> BlobDelta {
    let mut delta = BlobDelta {
        distinct: after.len() as i64 - before.len() as i64,
        ..BlobDelta::default()
    };
    let keys: std::collections::BTreeSet<&Key> = before.keys().chain(after.keys()).collect();
    for key in keys {
        let (old, old_sample) = before.get(key).cloned().unwrap_or((0, None));
        let (new, new_sample) = after.get(key).cloned().unwrap_or((0, None));
        if before.contains_key(key) && after.contains_key(key) && old_sample != new_sample {
            delta.sample_changed = true;
        }
        if new != old {
            delta.changes.push((key.clone(), new - old, new_sample));
        }
    }
    delta
}

fn failure(source_path: &str, cause: impl std::fmt::Display) -> ProductDbError {
    ProductDbError::Xml {
        source_path: source_path.to_string(),
        cause: format!("v29 backfill: {cause}"),
    }
}

/// Re-derives one blob inside its own savepoint; `Ok(Err(_))` names a blob
/// that was not carried forward and whose rows were left untouched.
fn backfill_blob(
    conn: &Connection,
    sha256: &str,
    source_path: &str,
    extended: bool,
) -> Result<Result<BlobDelta, ProductDbError>, ProductDbError> {
    let Some(bytes) = crate::load_source_file(conn, sha256)? else {
        return Ok(Err(failure(
            source_path,
            "stored program bytes are missing",
        )));
    };
    if classify(&bytes) != FileKind::ApplicationProgram {
        return Ok(Err(failure(
            source_path,
            "stored program no longer classifies as an ApplicationProgram",
        )));
    }
    if crate::sha256_hex(&bytes) != sha256 {
        return Ok(Err(failure(
            source_path,
            "stored bytes do not match their SHA-256 key",
        )));
    }
    let before = blob_rows(conn, sha256)?;
    conn.execute_batch(&format!("SAVEPOINT {SAVEPOINT};"))?;
    let replay = |mode| {
        crate::parse::program::backfill_type_child_kinds(
            conn,
            sha256,
            source_path,
            &bytes,
            mode,
            &CHILDREN,
        )?;
        blob_rows(conn, sha256)
    };
    let result = (|| {
        // A shared source ledger does not authenticate prior operation modes.
        // Probe both modes from exactly the same original rows; do not union
        // their histories or reuse a mode-dependent delta for every package.
        let targeted = if extended {
            let rows = replay(false)?;
            conn.execute_batch(&format!("ROLLBACK TO SAVEPOINT {SAVEPOINT};"))?;
            Some(
                rows.into_iter()
                    .filter(|(key, _)| date_evidence(key))
                    .collect::<BTreeMap<_, _>>(),
            )
        } else {
            None
        };
        let after = replay(extended)?;
        if targeted.as_ref().is_some_and(|targeted| {
            !targeted
                .iter()
                .eq(after.iter().filter(|(key, _)| date_evidence(key)))
        }) {
            return Err(failure(source_path,
                "TypeDate evidence differs between targeted and package modes; historical mode ownership is unavailable"));
        }
        Ok(delta_between(&before, &after))
    })();
    match result {
        Ok(delta) => {
            conn.execute_batch(&format!("RELEASE SAVEPOINT {SAVEPOINT};"))?;
            Ok(Ok(delta))
        }
        Err(ProductDbError::Sqlite(error)) => Err(ProductDbError::Sqlite(error)),
        Err(error) => {
            conn.execute_batch(&format!(
                "ROLLBACK TO SAVEPOINT {SAVEPOINT}; RELEASE SAVEPOINT {SAVEPOINT};"
            ))?;
            Ok(Err(error))
        }
    }
}

/// Only the promoted child's element/attribute evidence participates in the
/// mode probe; unrelated reconciliation evidence cannot justify refusing Date.
fn date_evidence(key: &Key) -> bool {
    (key.1 == "Element" && key.2 == "TypeDate")
        || (key.1 == "Attribute" && key.0.ends_with("/TypeDate"))
}

fn record(
    conn: &Connection,
    sha256: &str,
    xpath: &str,
    kind: &str,
    name: &str,
    error: &ProductDbError,
) -> Result<(), ProductDbError> {
    conn.execute(
        "INSERT INTO ingest_unknown (source_sha256, program_id, xpath, kind, name, occurrences, sample)
         VALUES (?1, NULL, ?2, ?3, ?4, 1, ?5)
         ON CONFLICT DO NOTHING",
        params![sha256, xpath, kind, name, error.to_string()],
    )?;
    Ok(())
}

pub(crate) fn migrate_v28_to_v29(conn: &Connection) -> Result<(), ProductDbError> {
    // `extended`: ingest reconciles a blob some scheme-21/23 package carries
    // in package mode, so the backfill reconciles its new rows the same way.
    let sources = conn
        .prepare(
            "SELECT DISTINCT u.source_sha256, s.source_path,
                    EXISTS (SELECT 1 FROM package_member AS m JOIN package AS p
                            ON p.sha256 = m.package_sha256
                            WHERE m.source_sha256 = u.source_sha256 AND p.scheme IN (21, 23))
             FROM ingest_unknown AS u JOIN source_file AS s ON s.sha256 = u.source_sha256
             WHERE u.kind = 'Element' AND u.name = 'TypeDate'
             ORDER BY u.source_sha256",
        )?
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, bool>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut deltas = HashMap::new();
    let mut failed = HashSet::new();
    for (sha256, source_path, extended) in sources {
        match backfill_blob(conn, &sha256, &source_path, extended)? {
            Ok(delta) => {
                deltas.insert(sha256, delta);
            }
            Err(error) => {
                record(
                    conn,
                    &sha256,
                    &source_path,
                    "ParameterKindBackfillError",
                    "backfill_date_kind",
                    &error,
                )?;
                failed.insert(sha256);
            }
        }
    }
    follow_in_packages(conn, &deltas, &failed)
}

fn follow_in_packages(
    conn: &Connection,
    deltas: &HashMap<String, BlobDelta>,
    failed: &HashSet<String>,
) -> Result<(), ProductDbError> {
    let packages = conn
        .prepare("SELECT sha256 FROM package ORDER BY sha256")?
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    for package in packages {
        let members = conn
            .prepare(
                "SELECT source_sha256 FROM package_member
                 WHERE package_sha256 = ?1 AND role = 'ApplicationProgram'
                 ORDER BY ordinal",
            )?
            .query_map([&package], |r| r.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        if !members
            .iter()
            .any(|m| deltas.contains_key(m) || failed.contains(m))
        {
            continue;
        }
        let distinct = members
            .iter()
            .filter_map(|m| deltas.get(m))
            .try_fold(0i64, |total, d| total.checked_add(d.distinct))
            .ok_or_else(|| failure(&package, "unknown counter overflow"))?;
        let updated = conn.execute(
            "UPDATE package SET unknown_count = unknown_count + ?2
             WHERE sha256 = ?1 AND unknown_count + ?2 >= 0",
            params![package, distinct],
        )?;
        if updated != 1 {
            return Err(failure(&package, "package unknown counter would underflow"));
        }
        let measured: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM package_install_report
                           WHERE package_sha256 = ?1 AND status = 'measured')",
            [&package],
            |r| r.get(0),
        )?;
        if !measured {
            continue;
        }
        let refusal = if let Some(member) = members.iter().find(|m| failed.contains(*m)) {
            Some(format!("member {member} was not carried forward"))
        } else if members
            .iter()
            .filter_map(|m| deltas.get(m))
            .any(|d| d.sample_changed)
        {
            Some("a retired row's sample changed; the merged report row is ambiguous".into())
        } else {
            None
        };
        let changes: Vec<_> = members
            .iter()
            .filter_map(|m| deltas.get(m))
            .flat_map(|d| d.changes.iter().cloned())
            .collect();
        conn.execute_batch("SAVEPOINT v29_date_kind_report;")?;
        let outcome = match refusal {
            Some(why) => Ok(Err(ReportNotUpgradable(why))),
            None => crate::package::apply_report_unknown_deltas(conn, &package, &changes),
        };
        let why = match outcome {
            Ok(Ok(())) => None,
            Ok(Err(why)) => Some(why.0),
            Err(ProductDbError::Sqlite(error)) => return Err(ProductDbError::Sqlite(error)),
            Err(error) => Some(error.to_string()),
        };
        match why {
            None => conn.execute_batch("RELEASE SAVEPOINT v29_date_kind_report;")?,
            Some(why) => {
                conn.execute_batch(
                    "ROLLBACK TO SAVEPOINT v29_date_kind_report;
                     RELEASE SAVEPOINT v29_date_kind_report;",
                )?;
                crate::package::mark_report_unavailable(conn, &package)?;
                record(
                    conn,
                    &package,
                    &package,
                    "InstallReportBackfillError",
                    "date_kind_report_backfill",
                    &failure(&package, why),
                )?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(name: &str) -> Key {
        ("/x".into(), "Attribute".into(), name.into())
    }

    #[test]
    fn a_delta_counts_retired_added_and_changed_rows() {
        let before = BTreeMap::from([
            (
                ("/p".into(), "Element".into(), "TypeDate".into()),
                (2, None),
            ),
            (key("Kept"), (1, Some("a".into()))),
        ]);
        let after = BTreeMap::from([
            (key("Encoding"), (2, Some("DPT 11".into()))),
            (key("Kept"), (1, Some("a".into()))),
        ]);
        let delta = delta_between(&before, &after);
        assert_eq!(delta.distinct, 0);
        assert!(!delta.sample_changed);
        assert_eq!(
            delta.changes,
            vec![
                (("/p".into(), "Element".into(), "TypeDate".into()), -2, None),
                (key("Encoding"), 2, Some("DPT 11".into())),
            ]
        );
    }

    #[test]
    fn a_changed_sample_of_a_surviving_row_is_flagged() {
        let before = BTreeMap::from([(key("Encoding"), (1, Some("a".into())))]);
        let after = BTreeMap::from([(key("Encoding"), (2, Some("b".into())))]);
        let delta = delta_between(&before, &after);
        assert!(delta.sample_changed);
        assert_eq!(delta.changes, vec![(key("Encoding"), 1, Some("b".into()))]);
    }
}
