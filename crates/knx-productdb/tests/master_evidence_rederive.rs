//! Tests atomic and idempotent master-evidence rebuilding from retained bytes.
//!
//! Covers exact-key idempotence,
//! corrupted/missing ownership evidence, original install availability and
//! all-or-nothing SQL failure. Fixtures are synthetic, offline and intentionally
//! use content identity rather than source filenames to select master data.

use knx_productdb::{
    open_and_migrate, rederive_master_language_evidence, sha256_hex, store_source_file, Connection,
    SourceFile,
};

const MASTER: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/11"><MasterData>
<Manufacturers><Manufacturer Id="M-0001" Name="Do not replay"/></Manufacturers>
<Languages><Language Identifier="en-US"><TranslationUnit RefId="unit" Version="uninterpreted"/></Language></Languages>
</MasterData></KNX>"#;

fn store(conn: &Connection, path: &str, bytes: &[u8]) {
    store_source_file(
        conn,
        &SourceFile {
            source_path: path.into(),
            manufacturer_id: None,
            bytes: bytes.to_vec(),
        },
    )
    .unwrap();
}

#[test]
fn explicit_rebuild_is_idempotent_and_does_not_replay_normalized_master_rows() {
    let directory = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&directory.path().join("products.sqlite")).unwrap();
    store(&conn, "not-a-master-filename.xml", MASTER);
    conn.execute_batch("CREATE TRIGGER never_replay_master BEFORE INSERT ON manufacturer BEGIN SELECT RAISE(ABORT,'normalized master replay forbidden'); END;").unwrap();
    let first = rederive_master_language_evidence(&conn).unwrap();
    assert_eq!(first.master_sources, 1);
    assert_eq!(first.failed_sources, 0);
    assert_eq!(first.derived_distinct_keys, 2);
    assert_eq!(first.unexamined_sources, 0);
    let second = rederive_master_language_evidence(&conn).unwrap();
    assert_eq!(first, second);
    let actual: Vec<(String, i64, String)> = conn
        .prepare("SELECT name,occurrences,sample FROM ingest_unknown ORDER BY name")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(
        actual,
        vec![
            ("RefId".into(), 1, "unit".into()),
            ("Version".into(), 1, "uninterpreted".into())
        ]
    );
    let normalized: i64 = conn
        .query_row("SELECT COUNT(*) FROM manufacturer", [], |row| row.get(0))
        .unwrap();
    assert_eq!(normalized, 0);
    assert_eq!(
        knx_productdb::load_source_file(&conn, &sha256_hex(MASTER))
            .unwrap()
            .unwrap(),
        MASTER
    );
}

#[test]
fn malformed_master_is_named_without_blocking_a_good_source_or_publishing_partial_fields() {
    let directory = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&directory.path().join("products.sqlite")).unwrap();
    let broken = br#"<KNX><MasterData><Languages Extra="must not publish"><Language>"#;
    store(&conn, "malformed.xml", broken);
    store(&conn, "good.xml", MASTER);
    let report = rederive_master_language_evidence(&conn).unwrap();
    assert_eq!(report.master_sources, 2);
    assert_eq!(report.failed_sources, 1);
    assert_eq!(report.derived_distinct_keys, 2);
    let failed_rows: Vec<(String, String)> = conn
        .prepare("SELECT kind,name FROM ingest_unknown WHERE source_sha256=?1")
        .unwrap()
        .query_map([sha256_hex(broken)], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(
        failed_rows,
        vec![(
            "MasterLanguageEvidenceError".into(),
            "master_language_unknowns".into()
        )]
    );
    rederive_master_language_evidence(&conn).unwrap();
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM ingest_unknown WHERE source_sha256=?1",
            [sha256_hex(broken)],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);
}

#[test]
fn missing_retained_master_is_not_silently_skipped_when_member_ownership_survives() {
    let directory = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&directory.path().join("products.sqlite")).unwrap();
    conn.pragma_update(None, "foreign_keys", "OFF").unwrap();
    conn.execute("INSERT INTO package (sha256,source_name,scheme,size,bytes,unknown_count) VALUES ('synthetic-package','synthetic',11,0,x'',0)",[]).unwrap();
    conn.execute("INSERT INTO package_member (package_sha256,ordinal,path,role,source_sha256,size) VALUES ('synthetic-package',0,'knx_master.xml','Master',?1,0)",[sha256_hex(MASTER)]).unwrap();
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    let report = rederive_master_language_evidence(&conn).unwrap();
    assert_eq!(report.master_sources, 1);
    assert_eq!(report.failed_sources, 1);
    assert_eq!(report.derived_distinct_keys, 0);
    let count:i64=conn.query_row("SELECT COUNT(*) FROM ingest_unknown WHERE source_sha256=?1 AND kind='MasterLanguageEvidenceError'",[sha256_hex(MASTER)],|row|row.get(0)).unwrap();
    assert_eq!(count, 1);
}

#[test]
fn a_late_sql_error_rolls_back_the_explicit_rebuild_and_propagates() {
    let directory = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&directory.path().join("products.sqlite")).unwrap();
    store(&conn, "good.xml", MASTER);
    conn.execute_batch("CREATE TRIGGER refuse_second_field BEFORE INSERT ON ingest_unknown WHEN NEW.name='Version' BEGIN SELECT RAISE(ABORT,'synthetic evidence SQL failure'); END;").unwrap();
    let error = rederive_master_language_evidence(&conn).unwrap_err();
    assert!(matches!(error, knx_productdb::ProductDbError::Sqlite(_)));
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM ingest_unknown", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 0);
    assert!(conn.is_autocommit());
}

#[test]
fn a_failed_rebuild_preserves_the_callers_outer_transaction() {
    let directory = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&directory.path().join("products.sqlite")).unwrap();
    store(&conn, "good.xml", MASTER);
    conn.execute_batch("CREATE TABLE caller_state(id INTEGER PRIMARY KEY);
        CREATE TRIGGER refuse_second_field BEFORE INSERT ON ingest_unknown WHEN NEW.name='Version' BEGIN SELECT RAISE(ABORT,'synthetic evidence SQL failure'); END;
        BEGIN; INSERT INTO caller_state VALUES(1);").unwrap();
    assert!(rederive_master_language_evidence(&conn).is_err());
    assert!(
        !conn.is_autocommit(),
        "the caller still owns its transaction"
    );
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM ingest_unknown", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 0);
    let caller_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM caller_state", [], |row| row.get(0))
        .unwrap();
    assert_eq!(caller_count, 1);
    conn.execute_batch("ROLLBACK").unwrap();
    let caller_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM caller_state", [], |row| row.get(0))
        .unwrap();
    assert_eq!(caller_count, 0);
}

#[test]
fn a_deferred_commit_error_rolls_back_the_rebuild_and_releases_its_transaction() {
    let directory = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&directory.path().join("products.sqlite")).unwrap();
    store(&conn, "good.xml", MASTER);
    conn.execute_batch(
        "CREATE TABLE evidence_parent(id INTEGER PRIMARY KEY);
         CREATE TABLE evidence_child(parent_id INTEGER REFERENCES evidence_parent(id) DEFERRABLE INITIALLY DEFERRED);
         CREATE TRIGGER deferred_evidence_failure AFTER INSERT ON ingest_unknown
         BEGIN INSERT INTO evidence_child(parent_id) VALUES(1); END;",
    ).unwrap();
    let error = rederive_master_language_evidence(&conn).unwrap_err();
    assert!(matches!(error, knx_productdb::ProductDbError::Sqlite(_)));
    for table in ["ingest_unknown", "evidence_child"] {
        let count: i64 = conn
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 0, "failed commit retained changes in {table}");
    }
    assert!(
        conn.is_autocommit(),
        "a failed RELEASE must not leak the owned savepoint"
    );
}

#[test]
fn oversized_unclassified_sources_are_explicitly_unexamined_not_clean() {
    let directory = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&directory.path().join("products.sqlite")).unwrap();
    // One byte above the private scanner's 64 MiB input cap; do not expose
    // production internals solely to construct a boundary fixture.
    let size: i64 = 64 * 1024 * 1024 + 1;
    conn.execute("INSERT INTO source_file(sha256,source_path,bytes,len) VALUES('synthetic-oversized-source','unknown.bin',zeroblob(?1),?1)", [size]).unwrap();
    let report = rederive_master_language_evidence(&conn).unwrap();
    assert_eq!(
        (
            report.master_sources,
            report.failed_sources,
            report.unexamined_sources
        ),
        (0, 0, 1)
    );
    assert_eq!(conn.query_row("SELECT COUNT(*) FROM ingest_unknown WHERE kind='MasterLanguageEvidenceUnexaminedSource'", [], |r| r.get::<_, i64>(0)).unwrap(), 1);
}

#[test]
fn a_repaired_master_replaces_its_unexamined_marker_with_current_evidence() {
    let directory = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&directory.path().join("products.sqlite")).unwrap();
    store(&conn, "retained.xml", MASTER);
    let sha = sha256_hex(MASTER);
    let size: i64 = 64 * 1024 * 1024 + 1;
    // Simulate retained-byte damage, then a repair to the original content.
    conn.execute(
        "UPDATE source_file SET bytes=zeroblob(?2),len=?2 WHERE sha256=?1",
        rusqlite::params![sha, size],
    )
    .unwrap();
    let damaged = rederive_master_language_evidence(&conn).unwrap();
    assert_eq!(damaged.unexamined_sources, 1);
    conn.execute(
        "UPDATE source_file SET bytes=?2,len=?3 WHERE sha256=?1",
        rusqlite::params![sha, MASTER, MASTER.len() as i64],
    )
    .unwrap();
    conn.execute("INSERT INTO ingest_unknown(source_sha256,program_id,xpath,kind,name,occurrences,sample) VALUES(?1,NULL,'/','MasterLanguageEvidenceUnexaminedSource','another_owner',1,'keep this finding')", [&sha]).unwrap();
    let repaired = rederive_master_language_evidence(&conn).unwrap();
    assert_eq!(
        (
            repaired.master_sources,
            repaired.failed_sources,
            repaired.unexamined_sources,
            repaired.derived_distinct_keys
        ),
        (1, 0, 0, 2)
    );
    let owned: i64 = conn.query_row("SELECT count(*) FROM ingest_unknown WHERE source_sha256=?1 AND xpath='/' AND name='master_language_unknowns' AND kind IN ('MasterLanguageEvidenceError','MasterLanguageEvidenceUnexaminedSource')", [&sha], |row| row.get(0)).unwrap();
    assert_eq!(
        owned, 0,
        "successful inspection left stale owned issue evidence"
    );
    let foreign: i64 = conn
        .query_row(
            "SELECT count(*) FROM ingest_unknown WHERE name='another_owner'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(foreign, 1);
    assert_eq!(rederive_master_language_evidence(&conn).unwrap(), repaired);
    assert_eq!(
        knx_productdb::load_source_file(&conn, &sha)
            .unwrap()
            .unwrap(),
        MASTER
    );
}

#[test]
fn a_repaired_nonmaster_clears_only_its_owned_classification_marker() {
    let directory = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&directory.path().join("products.sqlite")).unwrap();
    let original = b"opaque payload, not XML";
    store(&conn, "payload.bin", original);
    let sha = sha256_hex(original);
    let size: i64 = 64 * 1024 * 1024 + 1;
    conn.execute(
        "UPDATE source_file SET bytes=zeroblob(?2),len=?2 WHERE sha256=?1",
        rusqlite::params![sha, size],
    )
    .unwrap();
    assert_eq!(
        rederive_master_language_evidence(&conn)
            .unwrap()
            .unexamined_sources,
        1
    );
    // Bounded but still identity-invalid bytes must not retire this marker.
    conn.execute(
        "UPDATE source_file SET bytes=?2,len=?3 WHERE sha256=?1",
        rusqlite::params![sha, b"damaged", 7_i64],
    )
    .unwrap();
    let invalid = rederive_master_language_evidence(&conn).unwrap();
    assert_eq!(invalid.unexamined_sources, 1);
    let marker: i64 = conn.query_row("SELECT count(*) FROM ingest_unknown WHERE kind='MasterLanguageEvidenceUnexaminedSource'", [], |row| row.get(0)).unwrap();
    assert_eq!(marker, 1);
    // Corruption can also impersonate a master before the original opaque
    // bytes are recovered; its failed inspection marker is current, not history.
    conn.execute(
        "UPDATE source_file SET bytes=?2,len=?3 WHERE sha256=?1",
        rusqlite::params![sha, MASTER, MASTER.len() as i64],
    )
    .unwrap();
    let false_master = rederive_master_language_evidence(&conn).unwrap();
    assert_eq!(
        (
            false_master.master_sources,
            false_master.failed_sources,
            false_master.unexamined_sources
        ),
        (1, 1, 0)
    );
    let failed: i64 = conn
        .query_row(
            "SELECT count(*) FROM ingest_unknown WHERE kind='MasterLanguageEvidenceError'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(failed, 1);
    conn.execute(
        "UPDATE source_file SET bytes=?2,len=?3 WHERE sha256=?1",
        rusqlite::params![sha, original.as_slice(), original.len() as i64],
    )
    .unwrap();
    let repaired = rederive_master_language_evidence(&conn).unwrap();
    assert_eq!(
        (
            repaired.master_sources,
            repaired.failed_sources,
            repaired.unexamined_sources,
            repaired.derived_distinct_keys
        ),
        (0, 0, 0, 0)
    );
    assert_eq!(
        conn.query_row("SELECT count(*) FROM ingest_unknown", [], |row| row
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        knx_productdb::load_source_file(&conn, &sha)
            .unwrap()
            .unwrap(),
        original
    );
}
