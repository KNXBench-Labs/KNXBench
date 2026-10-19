//! Audits retained-byte corpus migration without replaying historical reports.
//!
//! Input databases are task-local
//! copies made by the baseline/candidate matrix, never original corpus files.
//! Only aggregate pass/fail escapes; normalized data and historic reports must
//! remain byte-value equal while current master-language evidence is derived.

use std::collections::BTreeMap;

use knx_productdb::{open_and_migrate, rederive_master_language_evidence, Connection};
use rusqlite::types::ValueRef;
use sha2::{Digest, Sha256};

fn table_commitments(conn: &Connection) -> BTreeMap<String, Vec<u8>> {
    let tables = conn
        .prepare("SELECT name FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name != 'ingest_unknown' ORDER BY name")
        .unwrap()
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let mut result = BTreeMap::new();
    for table in tables {
        let quoted = format!("\"{}\"", table.replace('"', "\"\""));
        let column_count = conn
            .prepare(&format!("SELECT * FROM {quoted}"))
            .unwrap()
            .column_count();
        let order = (1..=column_count)
            .map(|column| column.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let mut statement = conn
            .prepare(&format!("SELECT * FROM {quoted} ORDER BY {order}"))
            .unwrap();
        let mut rows = statement.query([]).unwrap();
        let mut hash = Sha256::new();
        while let Some(row) = rows.next().unwrap() {
            hash.update([255]);
            for column in 0..column_count {
                match row.get_ref(column).unwrap() {
                    ValueRef::Null => hash.update([0]),
                    ValueRef::Integer(value) => {
                        hash.update([1]);
                        hash.update(value.to_le_bytes());
                    }
                    ValueRef::Real(value) => {
                        hash.update([2]);
                        hash.update(value.to_bits().to_le_bytes());
                    }
                    ValueRef::Text(bytes) | ValueRef::Blob(bytes) => {
                        hash.update([
                            if matches!(row.get_ref(column).unwrap(), ValueRef::Text(_)) {
                                3
                            } else {
                                4
                            },
                        ]);
                        hash.update((bytes.len() as u64).to_le_bytes());
                        hash.update(bytes);
                    }
                }
            }
        }
        result.insert(table, hash.finalize().to_vec());
    }
    result
}

fn current_language_evidence(conn: &Connection) -> Vec<(String, String, String, i64, String)> {
    // DISTINCT intentionally ignores historical repeated per-install rows.
    // The current byte-derived value of an exact source/key must agree.
    conn.prepare("SELECT DISTINCT source_sha256,xpath,name,occurrences,COALESCE(sample,'') FROM ingest_unknown WHERE program_id IS NULL AND kind='Attribute' AND (xpath='/KNX/MasterData/Languages' OR xpath LIKE '/KNX/MasterData/Languages/%') ORDER BY source_sha256,xpath,name,occurrences,sample")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

#[test]
#[ignore = "explicit task-local baseline/fresh corpus databases required; offline only"]
fn real_v18_upgrade_preserves_normalized_and_historical_values() {
    let source = std::env::var_os("KNXBENCH_AR05_BASELINE_DB")
        .expect("explicit v18 corpus database required");
    let fresh = std::env::var_os("KNXBENCH_AR05_FRESH_DB")
        .expect("explicit fresh corpus database required");
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("upgraded.sqlite");
    std::fs::copy(source, &path).expect("copy task-local baseline database");
    let old = Connection::open(&path).unwrap();
    let version: i64 = old
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap();
    assert_eq!(
        version, 18,
        "fixture must come from the actual v18 baseline"
    );
    let before = table_commitments(&old);
    drop(old);

    let upgraded = open_and_migrate(&path).unwrap();
    assert!(
        table_commitments(&upgraded) == before,
        "migration changed normalized, retained-source or historical installation values"
    );
    let first_evidence = current_language_evidence(&upgraded);
    let first = rederive_master_language_evidence(&upgraded).unwrap();
    assert_eq!(first.master_sources, 67);
    assert_eq!(first.failed_sources, 0);
    assert_eq!(first.unexamined_sources, 0);
    assert_eq!(first.derived_distinct_keys, first.master_sources * 2);
    assert!(
        current_language_evidence(&upgraded) == first_evidence,
        "explicit rebuild changed upgraded exact-key evidence"
    );
    assert!(
        table_commitments(&upgraded) == before,
        "explicit rebuild changed non-evidence values"
    );
    let fresh =
        Connection::open_with_flags(fresh, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    assert!(
        current_language_evidence(&fresh) == first_evidence,
        "fresh and upgraded current master evidence disagree"
    );
}

#[test]
fn evidence_comparison_includes_the_wrapper_but_not_lookalike_prefixes() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("CREATE TABLE ingest_unknown(source_sha256 TEXT, program_id TEXT, xpath TEXT, kind TEXT, name TEXT, occurrences INTEGER, sample TEXT);
        INSERT INTO ingest_unknown VALUES
        ('synthetic',NULL,'/KNX/MasterData/Languages','Attribute','Wrapper',1,'wrapper'),
        ('synthetic',NULL,'/KNX/MasterData/Languages/Language','Attribute','Extra',1,'child'),
        ('synthetic',NULL,'/KNX/MasterData/LanguagesOther','Attribute','Foreign',1,'foreign');").unwrap();
    let values = current_language_evidence(&conn);
    assert_eq!(
        values.len(),
        2,
        "wrapper evidence was omitted or foreign prefix admitted"
    );
    assert!(values
        .iter()
        .any(|value| value.1 == "/KNX/MasterData/Languages"));
    assert!(!values.iter().any(|value| value.4 == "foreign"));
}
