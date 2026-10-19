//! Rewinds a v24 database to a genuine v23 one, for migration tests.
//!
//! v24 (ADR-0139) added `access` and `ui_hint` to `dynamic_node` and stopped
//! reporting `Access` on `ParameterBlock` and `ParameterSeparator`. `UIHint`
//! was recognized before and never reported. Rewinding:
//! - folds both values back into `extra` (`name=value` lines, sorted by
//!   attribute name, as the v23 parser wrote it);
//! - drops both columns;
//! - re-adds one `ingest_unknown` `Access` row per blob and host xpath, with
//!   the first value in node order as its sample;
//! - re-adds the matching `package_install_unknown` rows and counts, and raises
//!   `package.unknown_count` by the distinct rows each member adds.

use std::collections::BTreeMap;

use rusqlite::{params, Connection};

const PROGRAM: &str = "/KNX/ManufacturerData/Manufacturer/ApplicationPrograms/ApplicationProgram";

/// `(blob, xpath)` -> (occurrences, sample).
type Retired = BTreeMap<(String, String), (i64, String)>;

fn retired_rows(conn: &Connection) -> Retired {
    let mut out = Retired::new();
    let nodes: Vec<(String, String, bool, String)> = conn
        .prepare(
            "SELECT a.source_sha256, d.kind, d.module_def_id <> '', d.access
             FROM dynamic_node AS d JOIN application_program AS a ON a.id = d.program_id
             WHERE d.access IS NOT NULL
             ORDER BY d.program_id, d.module_def_id, d.node_id",
        )
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    for (sha, kind, module, value) in nodes {
        let xpath = if module {
            format!("{PROGRAM}/ModuleDefs/ModuleDef/Dynamic/{kind}")
        } else {
            format!("{PROGRAM}/Dynamic/{kind}")
        };
        out.entry((sha, xpath))
            .and_modify(|(n, _)| *n += 1)
            .or_insert((1, value));
    }
    out
}

#[allow(dead_code)]
pub fn rewind_to_v23(conn: &Connection) {
    let retired = retired_rows(conn);
    type Row = (
        String,
        String,
        i64,
        Option<String>,
        Option<String>,
        Option<String>,
    );
    let nodes: Vec<Row> = conn
        .prepare(
            "SELECT program_id, module_def_id, node_id, extra, access, ui_hint
             FROM dynamic_node WHERE access IS NOT NULL OR ui_hint IS NOT NULL",
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
    for (program, module, node, extra, access, ui_hint) in nodes {
        let mut lines: Vec<String> = extra
            .map(|e| e.split('\n').map(str::to_string).collect())
            .unwrap_or_default();
        if let Some(access) = access {
            lines.push(format!("Access={access}"));
        }
        if let Some(ui_hint) = ui_hint {
            lines.push(format!("UIHint={ui_hint}"));
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
    super::v20_rewind::drop_v24_objects(conn);

    for ((sha, xpath), (occurrences, sample)) in &retired {
        conn.execute(
            "INSERT INTO ingest_unknown (source_sha256, program_id, xpath, kind, name, occurrences, sample)
             VALUES (?1, NULL, ?2, 'Attribute', 'Access', ?3, ?4)",
            params![sha, xpath, occurrences, sample],
        )
        .unwrap();
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
        let mut merged: BTreeMap<String, (i64, String)> = BTreeMap::new();
        let mut distinct_per_member = 0i64;
        for member in &members {
            for ((sha, xpath), (occurrences, sample)) in &retired {
                if sha == member {
                    distinct_per_member += 1;
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
                "Access".into(),
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
    conn.execute_batch("PRAGMA user_version = 23;").unwrap();
}
