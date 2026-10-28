//! Tests of private schema admission against actual SQLite transactions.
//!
//! Synthetic fixtures do not establish historical or external compatibility.

use super::*;
use crate::operation::PublicationGate;

fn fresh() -> (tempfile::TempDir, rusqlite::Connection) {
    let dir = tempfile::tempdir().unwrap();
    let conn = crate::open_and_migrate(&dir.path().join("catalogue.db")).unwrap();
    (dir, conn)
}

#[test]
fn version_and_object_counts_cannot_admit_substituted_schema() {
    let (_dir, mut conn) = fresh();
    let count: i64 = conn
        .query_row("SELECT count(*) FROM sqlite_schema", [], |r| r.get(0))
        .unwrap();
    conn.execute_batch("DROP INDEX application_program_source; CREATE INDEX unrelated ON application_program(name)").unwrap();
    assert_eq!(
        conn.query_row("SELECT count(*) FROM sqlite_schema", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        count
    );
    let tx = conn.transaction().unwrap();
    assert!(matches!(
        admit(&tx, &PublicationGate::new()),
        Err(AdmissionError::UnsupportedSchemaShape)
    ));
}

#[test]
fn empty_current_schema_refuses() {
    let mut conn = rusqlite::Connection::open_in_memory().unwrap();
    conn.pragma_update(None, "user_version", super::ADMITTED_VERSION)
        .unwrap();
    let tx = conn.transaction().unwrap();
    assert!(matches!(
        admit(&tx, &PublicationGate::new()),
        Err(AdmissionError::UnsupportedSchemaShape)
    ));
}

#[test]
fn fresh_chain_is_admitted() {
    let (_dir, mut conn) = fresh();
    let tx = conn.transaction().unwrap();
    assert!(admit(&tx, &PublicationGate::new()).is_ok());
}

#[test]
fn temp_and_attached_schemas_refuse() {
    for setup in [
        "CREATE TEMP TABLE app_temp(id INTEGER)",
        "ATTACH ':memory:' AS extra",
    ] {
        let (_dir, mut conn) = fresh();
        conn.execute_batch(setup).unwrap();
        let tx = conn.transaction().unwrap();
        assert!(
            matches!(
                admit(&tx, &PublicationGate::new()),
                Err(AdmissionError::UnsupportedSchemaShape)
            ),
            "{setup}"
        );
        assert!(!tx.is_autocommit());
    }
}

#[test]
fn utf16_refuses_even_with_identical_sql() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("utf16.db");
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute_batch(
        "PRAGMA encoding='UTF-16le'; CREATE TABLE encoding_marker(id); DROP TABLE encoding_marker",
    )
    .unwrap();
    drop(conn);
    let mut conn = crate::open_and_migrate(&path).unwrap();
    let encoding: String = conn.query_row("PRAGMA encoding", [], |r| r.get(0)).unwrap();
    assert_eq!(encoding, "UTF-16le");
    let tx = conn.transaction().unwrap();
    assert!(matches!(
        admit(&tx, &PublicationGate::new()),
        Err(AdmissionError::UnsupportedSchemaShape)
    ));
}

#[test]
fn precancelled_schema_scan_returns_typed_stop_before_sql() {
    let mut conn = rusqlite::Connection::open_in_memory().unwrap();
    let tx = conn.transaction().unwrap();
    let gate = PublicationGate::new();
    assert_eq!(gate.try_cancel(), crate::operation::CancelOutcome::Accepted);
    assert!(matches!(
        admit(&tx, &gate),
        Err(AdmissionError::Stop(crate::operation::Stop::CancelAccepted))
    ));
    assert!(!tx.is_autocommit());
}

#[test]
fn metadata_budget_helper_exact_next_reachable_and_overflow() {
    use rusqlite::types::ValueRef::{Blob, Integer, Null, Real, Text};
    // Framing8 + null1 + int9 + real9 + text(9+2 UTF8 bytes) + blob(9+3).
    let cells = [
        Null,
        Integer(-1),
        Real(1.0),
        Text("é".as_bytes()),
        Blob(b"abc"),
    ];
    let mut meter = Budget::default();
    meter.record(&cells).unwrap();
    assert_eq!(meter.used, 50);
    let mut exact = Budget {
        used: 2_097_152 - 50,
    };
    exact.record(&cells).unwrap();
    assert_eq!(exact.used, 2_097_152);
    // Empty record is the next reachable framing charge (+8), not fictitious +1.
    assert!(matches!(
        exact.record(&[]),
        Err(AdmissionError::WitnessSchemaAdmissionLimit(Limit::Metadata))
    ));
    assert_eq!(exact.used, 2_097_152);
    let mut overflow = Budget { used: u64::MAX - 1 };
    assert!(matches!(
        overflow.record(&[Null]),
        Err(AdmissionError::WitnessSchemaAdmissionLimit(Limit::Metadata))
    ));
    assert_eq!(overflow.used, u64::MAX - 1);
}

#[test]
fn actual_enumeration_limits_precede_unknown_shape() {
    for (setup, expected) in [
        (
            (0..137)
                .map(|n| format!("CREATE INDEX excess{n} ON schema_meta(value); "))
                .collect::<String>(),
            Limit::Objects,
        ),
        (
            (0..19)
                .map(|n| format!("CREATE TABLE excess{n}(id INTEGER PRIMARY KEY) STRICT; "))
                .collect(),
            Limit::Tables,
        ),
        (
            format!(
                "CREATE TABLE \"{}\"(id INTEGER PRIMARY KEY) STRICT",
                "x".repeat(257)
            ),
            Limit::Identifier,
        ),
        (
            format!(
                "CREATE INDEX over_sql ON schema_meta(value) /*{}*/",
                "x".repeat(262_145)
            ),
            Limit::Sql,
        ),
        (
            format!(
                "CREATE TABLE many_columns(id INTEGER PRIMARY KEY,{}) STRICT",
                (0..256)
                    .map(|n| format!("c{n} INTEGER"))
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            Limit::Columns,
        ),
    ] {
        let (_dir, mut conn) = fresh();
        conn.execute_batch(&setup).unwrap();
        let tx = conn.transaction().unwrap();
        match admit(&tx, &PublicationGate::new()) {
            Err(AdmissionError::WitnessSchemaAdmissionLimit(found)) => assert_eq!(found, expected),
            Err(other) => panic!("expected {expected:?}, observed {other:?}"),
            Ok(_) => panic!("expected enumeration limit"),
        }
        assert!(!tx.is_autocommit());
    }
}

fn fixture() -> serde_json::Value {
    serde_json::from_str(include_str!("schema28-fixture.json")).unwrap()
}

fn assert_records<const N: usize>(
    actual: &[Record<N>],
    expected: &[serde_json::Value],
    keys: [&str; N],
) {
    use rusqlite::types::Value;
    let expected: Vec<Record<N>> = expected
        .iter()
        .map(|row| {
            Record(keys.map(|key| match &row[key] {
                serde_json::Value::Null => Value::Null,
                serde_json::Value::Number(n) => Value::Integer(n.as_i64().unwrap()),
                serde_json::Value::String(s) => Value::Text(s.clone()),
                _ => panic!("unsupported oracle scalar"),
            }))
        })
        .collect();
    assert_eq!(actual, expected);
}

fn assert_full_fixture(admitted: &Admitted<'_, '_>) {
    let f = fixture();
    let d = &f["observedDescriptors"];
    let mut objects = d["storedObjects"].as_array().unwrap().clone();
    objects.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
    assert_records(
        &admitted.objects,
        &objects,
        [
            "schema_rowid",
            "type",
            "name",
            "tbl_name",
            "rootpage",
            "sql",
        ],
    );
    let tables = d["tablesIncludingSchemaRelation"].as_array().unwrap();
    assert_eq!(admitted.metadata_bytes, 93_143); // Independent fixture accounting, including repeats.
    assert_eq!(admitted.relations.len(), tables.len());
    for (actual, expected) in admitted.relations.iter().zip(tables) {
        assert_records(
            std::slice::from_ref(&actual.table),
            std::slice::from_ref(&expected["tableList"]),
            ["schema", "name", "type", "ncol", "wr", "strict"],
        );
        assert_records(
            &actual.columns,
            expected["tableXinfo"].as_array().unwrap(),
            [
                "cid",
                "name",
                "type",
                "notnull",
                "dflt_value",
                "pk",
                "hidden",
            ],
        );
        assert_records(
            &actual.foreign_keys,
            expected["foreignKeyList"].as_array().unwrap(),
            [
                "id",
                "seq",
                "table",
                "from",
                "to",
                "on_update",
                "on_delete",
                "match",
            ],
        );
        let indexes = expected["indexes"].as_array().unwrap();
        assert_eq!(actual.indexes.len(), indexes.len());
        for (index, expected) in actual.indexes.iter().zip(indexes) {
            assert_records(
                std::slice::from_ref(&index.list),
                std::slice::from_ref(&expected["indexList"]),
                ["seq", "name", "unique", "origin", "partial"],
            );
            assert_records(
                &index.xinfo,
                expected["indexXinfo"].as_array().unwrap(),
                ["seqno", "cid", "name", "desc", "coll", "key"],
            );
        }
    }
}

#[test]
fn schema28_fixture_matches_complete_runtime_descriptors_rw_and_true_ro() {
    let (dir, mut conn) = fresh();
    {
        let tx = conn.transaction().unwrap();
        let admitted = admit(&tx, &PublicationGate::new()).unwrap();
        assert_full_fixture(&admitted);
    }
    drop(conn);
    let mut ro = crate::open_read_only(&dir.path().join("catalogue.db")).unwrap();
    assert_eq!(ro.migrated_from, None);
    assert!(ro.conn.is_readonly("main").unwrap());
    let tx = ro.conn.transaction().unwrap();
    assert_full_fixture(&admit(&tx, &PublicationGate::new()).unwrap());
    let before = snapshot(&tx);
    let error = tx
        .execute(
            "INSERT INTO schema_meta(key,value) VALUES('refused','write')",
            [],
        )
        .unwrap_err();
    assert_eq!(
        error.sqlite_error_code(),
        Some(rusqlite::ErrorCode::ReadOnly)
    );
    assert_full_fixture(&admit(&tx, &PublicationGate::new()).unwrap());
    assert_eq!(before, snapshot(&tx));
    assert!(!tx.is_autocommit());
}

#[test]
fn borrowed_retention_seam_enforces_byte_limits_before_copy() {
    use rusqlite::types::ValueRef::{Integer, Null, Text};
    let exact_id = "é".repeat(128); // 256 bytes, not 256 characters.
    let over_id = format!("{exact_id}x");
    let exact_sql = "s".repeat(262_144);
    let over_sql = format!("{exact_sql}x");
    for (bytes, field, expected, charge) in [
        (exact_id.as_bytes(), Field::Identifier, None, 273),
        (
            over_id.as_bytes(),
            Field::Identifier,
            Some(Limit::Identifier),
            0,
        ),
        (exact_sql.as_bytes(), Field::Sql, None, 262_161),
        (over_sql.as_bytes(), Field::Sql, Some(Limit::Sql), 0),
    ] {
        let mut budget = Budget::default();
        let result = retain([Text(bytes)], [field], &mut budget);
        match (result, expected) {
            (Ok(record), None) => assert_eq!(ValueRef::from(&record.0[0]), Text(bytes)),
            (Err(AdmissionError::WitnessSchemaAdmissionLimit(found)), Some(kind)) => {
                assert_eq!(found, kind)
            }
            _ => panic!("wrong borrowed retention outcome"),
        }
        assert_eq!(budget.used, charge);
    }
    // Repeated fields in distinct records count repeatedly. This seam is not
    // a claim that an unfamiliar SQL schema is admitted by the full helper.
    let mut budget = Budget::default();
    for _ in 0..7 {
        retain([Text(exact_sql.as_bytes())], [Field::Sql], &mut budget).unwrap();
    }
    assert_eq!(budget.used, 1_835_127);
    let final_sql = "f".repeat(262_008);
    retain([Text(final_sql.as_bytes())], [Field::Sql], &mut budget).unwrap();
    assert_eq!(budget.used, 2_097_152);
    assert!(matches!(
        retain([Null], [Field::Sql], &mut budget),
        Err(AdmissionError::WitnessSchemaAdmissionLimit(Limit::Metadata))
    ));
    assert_eq!(budget.used, 2_097_152);
    assert!(matches!(
        retain([Integer(1)], [Field::Sql], &mut Budget::default()),
        Err(AdmissionError::UnsupportedSchemaShape)
    ));
}

// Independent test-only readback, deliberately not the production inspector.
fn rows(conn: &rusqlite::Connection, sql: &str) -> Vec<Vec<rusqlite::types::Value>> {
    let mut statement = conn.prepare(sql).unwrap();
    let count = statement.column_count();
    statement
        .query_map([], |row| {
            (0..count)
                .map(|i| row.get(i))
                .collect::<rusqlite::Result<Vec<_>>>()
        })
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap()
}

fn snapshot(conn: &rusqlite::Connection) -> Vec<Vec<rusqlite::types::Value>> {
    let mut result = rows(
        conn,
        "SELECT rowid,type,name,tbl_name,rootpage,sql FROM main.sqlite_schema ORDER BY rowid",
    );
    result.extend(rows(conn, "PRAGMA main.user_version"));
    result.extend(rows(conn, "PRAGMA main.schema_version"));
    result.extend(rows(conn, "SELECT total_changes()"));
    for table in rows(
        conn,
        "SELECT name FROM main.sqlite_schema WHERE type='table' ORDER BY name",
    ) {
        if let rusqlite::types::Value::Text(name) = &table[0] {
            result.extend(rows(
                conn,
                &format!("SELECT * FROM main.\"{}\"", name.replace('"', "\"\"")),
            ));
        }
    }
    result
}

fn shape_refusal_preserves(conn: &mut rusqlite::Connection) {
    let tx = conn.transaction().unwrap();
    let before = snapshot(&tx);
    assert!(matches!(
        admit(&tx, &PublicationGate::new()),
        Err(AdmissionError::UnsupportedSchemaShape)
    ));
    assert!(!tx.is_autocommit());
    assert_eq!(snapshot(&tx), before);
}

#[test]
fn column_pk_default_check_and_fk_drift_refuse() {
    for ddl in [
        "CREATE TABLE schema_meta(key BLOB PRIMARY KEY,value TEXT NOT NULL) STRICT",
        "CREATE TABLE schema_meta(key TEXT NOT NULL,value TEXT PRIMARY KEY) STRICT",
        "CREATE TABLE schema_meta(key TEXT PRIMARY KEY,value TEXT NOT NULL DEFAULT 'changed') STRICT",
        "CREATE TABLE schema_meta(key TEXT PRIMARY KEY,value TEXT NOT NULL CHECK(length(value)>9)) STRICT",
        "CREATE TABLE schema_meta(key TEXT PRIMARY KEY,value TEXT NOT NULL REFERENCES source_file(sha256)) STRICT",
        "CREATE TABLE schema_meta(key TEXT PRIMARY KEY,value TEXT) STRICT",
    ] {
        let (_dir, mut conn) = fresh();
        conn.execute_batch(&format!("DROP TABLE schema_meta; {ddl}")).unwrap();
        shape_refusal_preserves(&mut conn);
    }
}

#[test]
fn autoindex_expression_predicates_and_collations_drift_refuse() {
    for ddl in [
        "DROP TABLE schema_meta; CREATE TABLE schema_meta(key TEXT NOT NULL,value TEXT PRIMARY KEY) STRICT",
        "DROP INDEX application_program_source; CREATE INDEX application_program_source ON application_program(lower(name))",
        "DROP INDEX application_program_source; CREATE INDEX application_program_source ON application_program(source_sha256 COLLATE NOCASE)",
        "DROP INDEX application_program_source; CREATE INDEX application_program_source ON application_program(source_sha256 DESC)",
        "DROP INDEX application_program_source; CREATE UNIQUE INDEX application_program_source ON application_program(source_sha256)",
    ] {
        let (_dir, mut conn) = fresh();
        conn.execute_batch(ddl).unwrap();
        shape_refusal_preserves(&mut conn);
    }
    let f = fixture();
    let objects = f["observedDescriptors"]["storedObjects"]
        .as_array()
        .unwrap();
    let partial: Vec<_> = objects
        .iter()
        .filter(|o| o["sql"].as_str().is_some_and(|s| s.contains(" WHERE ")))
        .collect();
    assert_eq!(partial.len(), 2);
    for index in partial {
        let name = index["name"].as_str().unwrap();
        let sql = index["sql"].as_str().unwrap();
        let (definition, _) = sql.split_once(" WHERE ").unwrap();
        let (_dir, mut conn) = fresh();
        conn.execute_batch(&format!("DROP INDEX \"{name}\"; {definition} WHERE 0"))
            .unwrap();
        shape_refusal_preserves(&mut conn);
    }
}

#[test]
fn extra_internal_and_executable_objects_refuse_without_evaluation() {
    for ddl in [
        "ANALYZE",
        "CREATE TABLE sequence_owner(id INTEGER PRIMARY KEY AUTOINCREMENT) STRICT",
        "CREATE VIEW executable AS SELECT function_that_must_not_run()",
        "CREATE TRIGGER executable BEFORE INSERT ON schema_meta BEGIN SELECT RAISE(ABORT,'must not execute'); END",
        "CREATE VIRTUAL TABLE virtual_owner USING fts5(content)",
    ] {
        let (_dir, mut conn) = fresh();
        conn.execute_batch(ddl).unwrap();
        shape_refusal_preserves(&mut conn);
    }
}

#[test]
fn unsupported_rowid_strict_generated_pkless_and_alias_shapes_refuse() {
    for ddl in [
        "CREATE TABLE schema_meta(key TEXT PRIMARY KEY,value TEXT NOT NULL) STRICT, WITHOUT ROWID",
        "CREATE TABLE schema_meta(key TEXT PRIMARY KEY,value TEXT NOT NULL)",
        "CREATE TABLE schema_meta(key TEXT PRIMARY KEY,value TEXT NOT NULL,derived TEXT GENERATED ALWAYS AS (value) VIRTUAL) STRICT",
        "CREATE TABLE schema_meta(key TEXT PRIMARY KEY,value TEXT NOT NULL,derived TEXT GENERATED ALWAYS AS (value) STORED) STRICT",
        "CREATE TABLE schema_meta(key TEXT,value TEXT NOT NULL) STRICT",
        "CREATE TABLE schema_meta(key TEXT PRIMARY KEY,value TEXT NOT NULL,RoWiD INTEGER) STRICT",
        "CREATE TABLE schema_meta(key TEXT PRIMARY KEY,value TEXT NOT NULL,_RoWiD_ INTEGER) STRICT",
        "CREATE TABLE schema_meta(key TEXT PRIMARY KEY,value TEXT NOT NULL,OiD INTEGER) STRICT",
    ] {
        let (_dir, mut conn) = fresh();
        conn.execute_batch(&format!("DROP TABLE schema_meta; {ddl}")).unwrap();
        shape_refusal_preserves(&mut conn);
    }
}

#[test]
fn old_and_future_versions_preserve_caller_state() {
    for version in [1, 27, 28, 29, 31, i32::MAX] {
        let (_dir, mut conn) = fresh();
        conn.pragma_update(None, "user_version", version).unwrap();
        let tx = conn.transaction().unwrap();
        let before = snapshot(&tx);
        assert!(
            matches!(admit(&tx, &PublicationGate::new()), Err(AdmissionError::UnsupportedVersion(found)) if found == i64::from(version))
        );
        assert_eq!(snapshot(&tx), before);
        assert!(!tx.is_autocommit());
    }
}

#[test]
fn observation_error_keeps_original_sqlite_busy_cause() {
    let (dir, mut conn) = fresh();
    conn.busy_timeout(std::time::Duration::ZERO).unwrap();
    let contender = rusqlite::Connection::open(dir.path().join("catalogue.db")).unwrap();
    let tx = conn.transaction().unwrap();
    contender.execute_batch("BEGIN EXCLUSIVE").unwrap();
    let result = admit(&tx, &PublicationGate::new());
    contender.execute_batch("ROLLBACK").unwrap();
    match result {
        Err(AdmissionError::Observation(rusqlite::Error::SqliteFailure(cause, message))) => {
            assert_eq!(cause.code, rusqlite::ErrorCode::DatabaseBusy);
            assert_eq!(cause.extended_code, 5);
            assert_eq!(message.as_deref(), Some("database is locked"));
        }
        Err(other) => panic!("unexpected error {other:?}"),
        Ok(_) => panic!("expected original observation cause"),
    }
    assert!(!tx.is_autocommit());
    assert!(admit(&tx, &PublicationGate::new()).is_ok());
}

thread_local! {
    static OWNED_SCALAR_COPIES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

pub(super) fn owned_scalar_copy() {
    OWNED_SCALAR_COPIES.with(|count| count.set(count.get() + 1));
}

#[test]
fn metadata_refusal_precedes_actual_owned_scalar_copy_boundary() {
    let mut budget = Budget { used: 2_097_152 };
    OWNED_SCALAR_COPIES.with(|count| count.set(0));
    assert!(matches!(
        retain([ValueRef::Text(b"would-copy")], [Field::Sql], &mut budget),
        Err(AdmissionError::WitnessSchemaAdmissionLimit(Limit::Metadata))
    ));
    assert_eq!(OWNED_SCALAR_COPIES.with(|count| count.get()), 0);
    assert_eq!(budget.used, 2_097_152);
    let mut allowed = Budget::default();
    retain([ValueRef::Text(b"copy")], [Field::Sql], &mut allowed).unwrap();
    assert_eq!(OWNED_SCALAR_COPIES.with(|count| count.get()), 1);
    assert_eq!(allowed.used, 21);
}

struct ScanPause {
    remaining: usize,
    reached: std::sync::mpsc::SyncSender<()>,
    resume: std::sync::mpsc::Receiver<()>,
}

thread_local! {
    static SCAN_PAUSE: std::cell::RefCell<Option<ScanPause>> = const { std::cell::RefCell::new(None) };
}

pub(super) fn record_boundary() {
    SCAN_PAUSE.with(|slot| {
        let mut guard = slot.borrow_mut();
        if let Some(pause) = guard.as_mut() {
            pause.remaining -= 1;
            if pause.remaining == 0 {
                let pause = guard.take().unwrap();
                pause.reached.send(()).unwrap();
                pause.resume.recv().unwrap();
            }
        }
    });
}

#[test]
fn midscan_cancel_at_real_retained_record_boundary_returns_typed_stop() {
    let (_dir, mut conn) = fresh();
    let gate = std::sync::Arc::new(PublicationGate::new());
    let worker_gate = std::sync::Arc::clone(&gate);
    let (reached_tx, reached_rx) = std::sync::mpsc::sync_channel(0);
    let (resume_tx, resume_rx) = std::sync::mpsc::sync_channel(0);
    let worker = std::thread::spawn(move || {
        SCAN_PAUSE.with(|slot| {
            *slot.borrow_mut() = Some(ScanPause {
                remaining: 64,
                reached: reached_tx,
                resume: resume_rx,
            })
        });
        let tx = conn.transaction().unwrap();
        let before = snapshot(&tx);
        let result = admit(&tx, &worker_gate);
        let stopped = matches!(
            result,
            Err(AdmissionError::Stop(crate::operation::Stop::CancelAccepted))
        );
        let unchanged = before == snapshot(&tx);
        (stopped, unchanged, !tx.is_autocommit())
    });
    let reached = reached_rx.recv();
    let cancellation = gate.try_cancel();
    let resumed = resume_tx.send(());
    let joined = worker.join();
    // Join before assertions, including any failed channel/worker outcome.
    assert!(reached.is_ok());
    assert!(resumed.is_ok());
    assert_eq!(cancellation, crate::operation::CancelOutcome::Accepted);
    assert_eq!(joined.unwrap(), (true, true, true));
}

#[test]
fn admission_preserves_transaction_data_and_actual_layout_not_fixture_layout() {
    let (_dir, mut conn) = fresh();
    let original_layout = rows(
        &conn,
        "SELECT rowid,rootpage FROM sqlite_schema WHERE name='catalog_item_section'",
    );
    conn.execute_batch("DROP INDEX catalog_item_section; CREATE TABLE disposable(id INTEGER PRIMARY KEY) STRICT; CREATE INDEX catalog_item_section ON catalog_item (section_id); DROP TABLE disposable").unwrap();
    let actual_layout = rows(
        &conn,
        "SELECT rowid,rootpage FROM sqlite_schema WHERE name='catalog_item_section'",
    );
    assert_ne!(actual_layout, original_layout);
    conn.execute_batch(
        "INSERT INTO schema_meta(key,value) VALUES('synthetic','retained'); PRAGMA query_only=ON",
    )
    .unwrap();
    let tx = conn.transaction().unwrap();
    let before = snapshot(&tx);
    let first = admit(&tx, &PublicationGate::new()).unwrap();
    let second = admit(&tx, &PublicationGate::new()).unwrap();
    assert_eq!(first.objects, second.objects);
    assert_eq!(first.relations, second.relations);
    let independent = rows(&tx, "SELECT rowid,type,name,tbl_name,rootpage,sql FROM main.sqlite_schema ORDER BY name COLLATE BINARY");
    assert_eq!(
        first
            .objects
            .iter()
            .map(|r| r.0.to_vec())
            .collect::<Vec<_>>(),
        independent
    );
    for relation in &first.relations {
        let rusqlite::types::Value::Text(name) = &relation.table.0[1] else {
            panic!("expected name")
        };
        assert_eq!(
            relation
                .columns
                .iter()
                .map(|r| r.0.to_vec())
                .collect::<Vec<_>>(),
            rows(&tx, &format!("PRAGMA main.table_xinfo(\"{name}\")"))
        );
        let mut index_list = rows(&tx, &format!("PRAGMA main.index_list(\"{name}\")"));
        index_list.sort_by(|a, b| {
            ValueRef::from(&a[1])
                .as_str()
                .unwrap()
                .as_bytes()
                .cmp(ValueRef::from(&b[1]).as_str().unwrap().as_bytes())
        });
        assert_eq!(
            relation
                .indexes
                .iter()
                .map(|r| r.list.0.to_vec())
                .collect::<Vec<_>>(),
            index_list
        );
        for index in &relation.indexes {
            let rusqlite::types::Value::Text(name) = &index.list.0[1] else {
                panic!("expected name")
            };
            assert_eq!(
                index.xinfo.iter().map(|r| r.0.to_vec()).collect::<Vec<_>>(),
                rows(&tx, &format!("PRAGMA main.index_xinfo(\"{name}\")"))
            );
        }
        assert_eq!(
            relation
                .foreign_keys
                .iter()
                .map(|r| r.0.to_vec())
                .collect::<Vec<_>>(),
            rows(&tx, &format!("PRAGMA main.foreign_key_list(\"{name}\")"))
        );
    }
    assert_eq!(before, snapshot(&tx));
    assert!(!tx.is_autocommit());
}

// Synthetic cached-descriptor divergences isolate introspection from SQL-text
// matching. These are not historical DDL variants or admissible file fixtures.
fn restore_sql_text_only(conn: &rusqlite::Connection, name: &str, sql: &str) {
    conn.pragma_update(None, "writable_schema", true).unwrap();
    conn.execute(
        "UPDATE main.sqlite_schema SET sql=?1 WHERE name=?2",
        [sql, name],
    )
    .unwrap();
    conn.pragma_update(None, "writable_schema", false).unwrap();
    assert_eq!(
        conn.query_row("SELECT sql FROM sqlite_schema WHERE name=?1", [name], |r| r
            .get::<_, String>(0))
            .unwrap(),
        sql
    );
}

#[test]
fn introspection_columns_defaults_pk_and_foreign_keys_are_independently_checked() {
    for ddl in [
        "CREATE TABLE schema_meta(key TEXT PRIMARY KEY,value BLOB NOT NULL) STRICT",
        "CREATE TABLE schema_meta(key TEXT NOT NULL,value TEXT PRIMARY KEY) STRICT",
        "CREATE TABLE schema_meta(key TEXT PRIMARY KEY,value TEXT NOT NULL DEFAULT 'changed') STRICT",
        "CREATE TABLE schema_meta(key TEXT PRIMARY KEY,value TEXT NOT NULL REFERENCES source_file(sha256)) STRICT",
        "CREATE TABLE schema_meta(key TEXT PRIMARY KEY,value TEXT) STRICT",
        "CREATE TABLE schema_meta(key TEXT PRIMARY KEY,value TEXT NOT NULL) STRICT, WITHOUT ROWID",
        "CREATE TABLE schema_meta(key TEXT PRIMARY KEY,value TEXT NOT NULL)",
    ] {
        let (_dir, mut conn) = fresh();
        let original: String = conn.query_row("SELECT sql FROM sqlite_schema WHERE name='schema_meta'",[],|r|r.get(0)).unwrap();
        conn.execute_batch(&format!("DROP TABLE schema_meta; {ddl}")).unwrap();
        restore_sql_text_only(&conn,"schema_meta",&original);
        shape_refusal_preserves(&mut conn);
    }
}

#[test]
fn introspection_index_key_aux_direction_collation_uniqueness_partial_are_checked() {
    for tail in [
        "ON application_program(name)",
        "ON application_program(lower(name))",
        "ON application_program(source_sha256 COLLATE NOCASE)",
        "ON application_program(source_sha256 DESC)",
        "ON application_program(source_sha256) WHERE name IS NOT NULL",
    ] {
        let (_dir, mut conn) = fresh();
        let original: String = conn
            .query_row(
                "SELECT sql FROM sqlite_schema WHERE name='application_program_source'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        conn.execute_batch(&format!(
            "DROP INDEX application_program_source; CREATE INDEX application_program_source {tail}"
        ))
        .unwrap();
        restore_sql_text_only(&conn, "application_program_source", &original);
        shape_refusal_preserves(&mut conn);
    }
    let (_dir, mut conn) = fresh();
    let original: String = conn
        .query_row(
            "SELECT sql FROM sqlite_schema WHERE name='application_program_source'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    conn.execute_batch("DROP INDEX application_program_source; CREATE UNIQUE INDEX application_program_source ON application_program(source_sha256)").unwrap();
    restore_sql_text_only(&conn, "application_program_source", &original);
    shape_refusal_preserves(&mut conn);
}

#[test]
fn exact_native_metadata_ceilings_are_not_misclassified_as_limits() {
    let sql_prefix = "CREATE INDEX exact_sql ON schema_meta(value) /*";
    let exact_sql = format!(
        "{sql_prefix}{}*/",
        "x".repeat(262_144 - sql_prefix.len() - 2)
    );
    let exact_id = "é".repeat(128);
    for (setup, check, pinned) in [
        ((0..136).map(|n|format!("CREATE INDEX bound{n} ON schema_meta(value); ")).collect::<String>(), "SELECT count(*) FROM sqlite_schema", 256),
        ((0..18).map(|n|format!("CREATE TABLE bound{n}(id INTEGER PRIMARY KEY) STRICT; ")).collect(), "SELECT count(*) FROM pragma_table_list WHERE schema='main' AND name <> 'sqlite_schema'", 64),
        (format!("CREATE TABLE \"{exact_id}\"(id INTEGER PRIMARY KEY) STRICT"), "SELECT max(length(CAST(name AS BLOB))) FROM sqlite_schema", 256),
        (exact_sql, "SELECT length(CAST(sql AS BLOB)) FROM sqlite_schema WHERE name='exact_sql'", 262_144),
        (format!("CREATE TABLE bound_columns(id INTEGER PRIMARY KEY,{}) STRICT",(0..255).map(|n|format!("c{n} INTEGER")).collect::<Vec<_>>().join(",")), "SELECT ncol FROM pragma_table_list WHERE schema='main' AND name='bound_columns'", 256),
    ] {
        let (_dir, mut conn) = fresh();
        conn.execute_batch(&setup).unwrap();
        assert_eq!(conn.query_row(check,[],|r|r.get::<_,i64>(0)).unwrap(),pinned);
        // Exact bounded unknown shapes still refuse, but not as an excess.
        shape_refusal_preserves(&mut conn);
    }
}

#[test]
fn sqlite_storage_classes_and_invalid_utf8_are_not_coerced() {
    let mut budget = Budget::default();
    match retain([ValueRef::Text(b"\xff")], [Field::Identifier], &mut budget) {
        Err(AdmissionError::Observation(rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            cause,
        ))) => assert!(cause.to_string().contains("invalid utf-8")),
        _ => panic!("expected original UTF8 observation cause"),
    }
    for value in [
        ValueRef::Real(3.0),
        ValueRef::Blob(b"name"),
        ValueRef::Integer(3),
    ] {
        assert!(matches!(
            retain([value], [Field::Identifier], &mut Budget::default()),
            Err(AdmissionError::UnsupportedSchemaShape)
        ));
    }
}

#[test]
fn missing_version_is_typed_refusal() {
    let mut conn = rusqlite::Connection::open_in_memory().unwrap();
    let tx = conn.transaction().unwrap();
    let gate = PublicationGate::new();
    assert!(matches!(
        admit(&tx, &gate),
        Err(AdmissionError::UnsupportedVersion(0))
    ));
    assert!(!tx.is_autocommit());
}

fn introspection_collision_refuses_without_evaluation(ddl: &str) {
    let (_dir, mut conn) = fresh();
    let tx = conn.transaction().unwrap();
    assert!(admit(&tx, &PublicationGate::new()).is_ok());
    tx.execute_batch(
        "INSERT INTO schema_meta(key,value) VALUES('collision_sentinel','caller data')",
    )
    .unwrap();
    tx.execute_batch(ddl).unwrap();
    // The readback selects stored schema rows and ordinary table data only;
    // it never SELECTs from a view, including the overflow witness below.
    let before = snapshot(&tx);
    let scope_sql = if ddl.starts_with("ATTACH") {
        "SELECT rowid,type,name,tbl_name,rootpage,sql FROM extra.sqlite_schema ORDER BY rowid"
    } else {
        "SELECT rowid,type,name,tbl_name,rootpage,sql FROM temp.sqlite_schema ORDER BY rowid"
    };
    let scope_before = rows(&tx, scope_sql);
    let result = admit(&tx, &PublicationGate::new());
    assert!(!tx.is_autocommit());
    assert_eq!(snapshot(&tx), before);
    assert_eq!(rows(&tx, scope_sql), scope_before);
    match result {
        Err(AdmissionError::UnsupportedSchemaShape) => {}
        Err(error) => panic!("collision must refuse as shape without evaluation: {error:?}"),
        Ok(_) => panic!("collision must not be admitted"),
    }
}

#[test]
fn pragma_table_list_overflow_view_refuses_without_evaluation() {
    introspection_collision_refuses_without_evaluation(
        "CREATE VIEW pragma_table_list AS SELECT 'main' AS schema, 'schema_meta' AS name, abs(-9223372036854775808) AS ncol",
    );
}

#[test]
fn pragma_table_list_table_collision_refuses_as_shape() {
    introspection_collision_refuses_without_evaluation(
        "CREATE TABLE pragma_table_list(payload TEXT) STRICT; INSERT INTO pragma_table_list VALUES('retained collision data')",
    );
}

#[test]
fn sibling_introspection_collisions_refuse_before_queries() {
    // These are the four remaining table-valued introspection names in relation().
    // Exact stored-object admission must reject them before any such call.
    for name in [
        "pragma_table_xinfo",
        "pragma_foreign_key_list",
        "pragma_index_list",
        "pragma_index_xinfo",
    ] {
        for ddl in [
            format!("CREATE VIEW {name} AS SELECT abs(-9223372036854775808) AS witness"),
            format!("CREATE TABLE {name}(payload TEXT) STRICT"),
        ] {
            introspection_collision_refuses_without_evaluation(&ddl);
        }
    }
}

#[test]
fn scoped_introspection_collisions_refuse_before_queries() {
    for name in [
        "pragma_table_list",
        "pragma_table_xinfo",
        "pragma_foreign_key_list",
        "pragma_index_list",
        "pragma_index_xinfo",
    ] {
        for ddl in [
            format!("CREATE TEMP VIEW {name} AS SELECT abs(-9223372036854775808) AS witness"),
            format!("ATTACH ':memory:' AS extra; CREATE VIEW extra.{name} AS SELECT abs(-9223372036854775808) AS witness"),
        ] {
            introspection_collision_refuses_without_evaluation(&ddl);
        }
    }
}
