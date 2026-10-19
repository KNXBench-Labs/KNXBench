//! Rewinds a v18 database to a genuine v17 one, for migration tests.
//!
//! v18 (ADR-0052) added `dynamic_node.name`/`number` and stopped reporting
//! `Channel/@Number` as unknown. Rewinding therefore:
//! - folds both values back into `extra`, the way the v17 parser wrote it
//!   (`name=value` lines, sorted by attribute name);
//! - drops the two columns;
//! - re-adds one `ingest_unknown` row per blob and channel xpath;
//! - re-adds the matching `package_install_unknown` row and its counts, and
//!   raises `package.unknown_count` by the distinct rows each member adds.
//!
//! The `sample` of each re-added row is the first channel's `@Number` in
//! node order, which is the value the v17 collector kept.

use rusqlite::{params, Connection};

const PROGRAM_CHANNEL: &str =
    "/KNX/ManufacturerData/Manufacturer/ApplicationPrograms/ApplicationProgram/Dynamic/Channel";
const MODULE_CHANNEL: &str = "/KNX/ManufacturerData/Manufacturer/ApplicationPrograms/ApplicationProgram/ModuleDefs/ModuleDef/Dynamic/Channel";

/// One retired row, as v17 recorded it per blob: xpath, occurrences, sample.
type Retired = (String, i64, String);

fn retired_rows(conn: &Connection, sha: &str) -> Vec<Retired> {
    let mut out = Vec::new();
    for (xpath, module) in [(PROGRAM_CHANNEL, false), (MODULE_CHANNEL, true)] {
        let scope = if module {
            "module_def_id <> ''"
        } else {
            "module_def_id = ''"
        };
        let rows: Vec<String> = conn
            .prepare(&format!(
                "SELECT number FROM dynamic_node
                 WHERE kind = 'Channel' AND number IS NOT NULL AND {scope}
                   AND program_id IN (SELECT id FROM application_program WHERE source_sha256 = ?1)
                 ORDER BY program_id, module_def_id, node_id"
            ))
            .unwrap()
            .query_map([sha], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        if let Some(first) = rows.first() {
            out.push((xpath.to_string(), rows.len() as i64, first.clone()));
        }
    }
    out
}

#[allow(dead_code)]
pub fn rewind_to_v17(conn: &Connection) {
    super::v20_rewind::drop_v20_objects(conn);
    let blobs: Vec<String> = conn
        .prepare("SELECT DISTINCT source_sha256 FROM application_program ORDER BY 1")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    let retired: Vec<(String, Vec<Retired>)> = blobs
        .into_iter()
        .map(|sha| {
            let rows = retired_rows(conn, &sha);
            (sha, rows)
        })
        .collect();

    // `extra` held every attribute the v17 spec did not capture, sorted by
    // name; for a channel that adds `Name` and `Number` to what v18 keeps.
    type RewoundNode = (
        String,
        String,
        i64,
        Option<String>,
        Option<String>,
        Option<String>,
    );
    let nodes: Vec<RewoundNode> = conn
        .prepare(
            "SELECT program_id, module_def_id, node_id, extra, name, number FROM dynamic_node
             WHERE kind = 'Channel' AND (name IS NOT NULL OR number IS NOT NULL)",
        )
        .unwrap()
        .query_map([], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
            ))
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    for (program, module, node, extra, name, number) in nodes {
        let mut lines: Vec<String> = extra
            .map(|e| e.split('\n').map(str::to_string).collect())
            .unwrap_or_default();
        if let Some(name) = name {
            lines.push(format!("Name={name}"));
        }
        if let Some(number) = number {
            lines.push(format!("Number={number}"));
        }
        lines.sort_by(|a, b| {
            let key = |l: &str| l.split('=').next().unwrap_or_default().to_string();
            key(a).cmp(&key(b))
        });
        conn.execute(
            "UPDATE dynamic_node SET extra = ?4
             WHERE program_id = ?1 AND module_def_id = ?2 AND node_id = ?3",
            params![program, module, node, lines.join("\n")],
        )
        .unwrap();
    }
    conn.execute_batch(
        "ALTER TABLE dynamic_node DROP COLUMN name;
         ALTER TABLE dynamic_node DROP COLUMN number;",
    )
    .unwrap();

    for (sha, rows) in &retired {
        for (xpath, occurrences, sample) in rows {
            conn.execute(
                "INSERT INTO ingest_unknown (source_sha256, program_id, xpath, kind, name, occurrences, sample)
                 VALUES (?1, NULL, ?2, 'Attribute', 'Number', ?3, ?4)",
                params![sha, xpath, occurrences, sample],
            )
            .unwrap();
        }
    }

    let packages: Vec<String> = conn
        .prepare("SELECT package_sha256 FROM package_install_report WHERE status = 'measured'")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    for package in packages {
        let members: Vec<String> = conn
            .prepare(
                "SELECT source_sha256 FROM package_member
                 WHERE package_sha256 = ?1 AND role = 'ApplicationProgram'",
            )
            .unwrap()
            .query_map([&package], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        let mut merged: std::collections::BTreeMap<String, (i64, String)> = Default::default();
        let mut distinct_per_member = 0i64;
        for member in &members {
            if let Some((_, rows)) = retired.iter().find(|(sha, _)| sha == member) {
                distinct_per_member += rows.len() as i64;
                for (xpath, occurrences, sample) in rows {
                    merged
                        .entry(xpath.clone())
                        .and_modify(|(o, _)| *o += occurrences)
                        .or_insert((*occurrences, sample.clone()));
                }
            }
        }
        conn.execute(
            "UPDATE package SET unknown_count = unknown_count + ?2 WHERE sha256 = ?1",
            params![package, distinct_per_member],
        )
        .unwrap();
        if merged.is_empty() {
            continue;
        }
        // Rebuild the unknown rows in install's order (xpath, kind, name),
        // then the header and the two `unknown_construct` counts.
        let mut rows: Vec<(String, String, String, i64, Option<String>)> = conn
            .prepare(
                "SELECT xpath, kind, name, occurrences, sample FROM package_install_unknown
                 WHERE package_sha256 = ?1",
            )
            .unwrap()
            .query_map([&package], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
            })
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        for (xpath, (occurrences, sample)) in &merged {
            rows.push((
                xpath.clone(),
                "Attribute".into(),
                "Number".into(),
                *occurrences,
                Some(sample.clone()),
            ));
        }
        rows.sort_by(|a, b| (&a.0, &a.1, &a.2).cmp(&(&b.0, &b.1, &b.2)));
        conn.execute(
            "DELETE FROM package_install_unknown WHERE package_sha256 = ?1",
            [&package],
        )
        .unwrap();
        for (ordinal, (xpath, kind, name, occurrences, sample)) in rows.iter().enumerate() {
            conn.execute(
                "INSERT INTO package_install_unknown VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    package,
                    ordinal as i64,
                    xpath,
                    kind,
                    name,
                    occurrences,
                    sample
                ],
            )
            .unwrap();
        }
        let distinct = rows.len() as i64;
        let total: i64 = rows.iter().map(|r| r.3).sum();
        conn.execute(
            "UPDATE package_install_report SET unknown_distinct = ?2, unknown_occurrences = ?3
             WHERE package_sha256 = ?1",
            params![package, distinct, total],
        )
        .unwrap();
        for (disposition, count) in [("read", total), ("stored", distinct)] {
            conn.execute(
                "UPDATE package_install_count SET count = ?3
                 WHERE package_sha256 = ?1 AND category = 'unknown_construct' AND disposition = ?2",
                params![package, disposition, count],
            )
            .unwrap();
        }
    }
    conn.execute_batch("PRAGMA user_version = 17;").unwrap();
}
