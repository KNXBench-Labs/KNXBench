//! Tests of the typed full-current-schema streaming witness.

use super::super::AdmissionError;
use super::*;
use crate::operation::PublicationGate;
use rusqlite::types::ValueRef;
use sha2::{Digest, Sha256};

fn fresh() -> (tempfile::TempDir, rusqlite::Connection) {
    let dir = tempfile::tempdir().unwrap();
    let conn = crate::open_and_migrate(&dir.path().join("catalogue.db")).unwrap();
    (dir, conn)
}

fn witness_of(conn: &mut rusqlite::Connection) -> CatalogueWitness {
    let tx = conn.transaction().unwrap();
    witness_current(&tx, &PublicationGate::new()).unwrap()
}

#[test]
fn empty_fresh_catalogue_witnesses_complete_counts() {
    let (_dir, mut conn) = fresh();
    let witness = witness_of(&mut conn);
    assert_eq!(witness.encoding_version, 1);
    assert_eq!(witness.table_count, 47);
    assert_eq!(witness.schema_object_count, 120);
    // 120 stored schema objects plus the one `schema_meta` seed row migration
    // writes (created_by).
    assert_eq!(witness.row_count, 121);
    assert_ne!(witness.sha256, [0u8; 32]);
    assert!(witness.encoded_bytes > 0);
}

#[test]
fn witness_is_deterministic_across_transactions() {
    let (_dir, mut conn) = fresh();
    conn.execute(
        "INSERT INTO manufacturer (id, name) VALUES ('m1', 'Example Co')",
        [],
    )
    .unwrap();
    let first = witness_of(&mut conn);
    let second = witness_of(&mut conn);
    assert_eq!(first.sha256, second.sha256);
    assert_eq!(first.row_count, second.row_count);
    assert_eq!(first.encoded_bytes, second.encoded_bytes);
}

#[test]
fn witness_changes_on_insert_update_and_delete() {
    let (_dir, mut conn) = fresh();
    let empty = witness_of(&mut conn);

    conn.execute(
        "INSERT INTO manufacturer (id, name) VALUES ('m1', 'Example Co')",
        [],
    )
    .unwrap();
    let one = witness_of(&mut conn);
    assert_eq!(one.row_count, 122);
    assert_ne!(one.sha256, empty.sha256);
    // Data rows do not change schema accounting.
    assert_eq!(one.table_count, empty.table_count);
    assert_eq!(one.schema_object_count, empty.schema_object_count);

    conn.execute(
        "INSERT INTO manufacturer (id, name) VALUES ('m2', 'Other Co')",
        [],
    )
    .unwrap();
    let two = witness_of(&mut conn);
    assert_eq!(two.row_count, 123);
    assert_ne!(two.sha256, one.sha256);

    conn.execute(
        "UPDATE manufacturer SET name = 'Renamed Co' WHERE id = 'm1'",
        [],
    )
    .unwrap();
    let updated = witness_of(&mut conn);
    assert_eq!(updated.row_count, 123);
    assert_ne!(updated.sha256, two.sha256);

    conn.execute("DELETE FROM manufacturer WHERE id = 'm2'", [])
        .unwrap();
    let deleted = witness_of(&mut conn);
    assert_eq!(deleted.row_count, 122);
    assert_ne!(deleted.sha256, updated.sha256);
    assert_ne!(deleted.sha256, one.sha256);
}

#[test]
fn witness_distinguishes_null_empty_text_and_a_value() {
    let digests: Vec<[u8; 32]> = [None, Some(""), Some("x")]
        .into_iter()
        .map(|name: Option<&str>| {
            let (_dir, mut conn) = fresh();
            conn.execute(
                "INSERT INTO manufacturer (id, name) VALUES ('m1', ?1)",
                rusqlite::params![name],
            )
            .unwrap();
            witness_of(&mut conn).sha256
        })
        .collect();
    assert_ne!(digests[0], digests[1]);
    assert_ne!(digests[1], digests[2]);
    assert_ne!(digests[0], digests[2]);
}

#[test]
fn cell_codec_encodes_storage_classes_exactly() {
    let cases: &[(&[u8], ValueRef<'_>)] = &[
        (&[0x00], ValueRef::Null),
        (&[0x01, 0, 0, 0, 0, 0, 0, 0, 1], ValueRef::Integer(1)),
        (
            &[0x01, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
            ValueRef::Integer(-1),
        ),
        (&[0x02, 0x3F, 0xF0, 0, 0, 0, 0, 0, 0], ValueRef::Real(1.0)),
        (&[0x03, 0, 0, 0, 0, 0, 0, 0, 1, 0x61], ValueRef::Text(b"a")),
        (
            &[0x04, 0, 0, 0, 0, 0, 0, 0, 2, 0x00, 0xFF],
            ValueRef::Blob(&[0x00, 0xFF]),
        ),
    ];
    for (expected, value) in cases {
        let mut writer = Writer::new();
        encode_cell(&mut writer, *value).unwrap();
        assert_eq!(writer.bytes, expected.len() as u64);
        let digest: [u8; 32] = Sha256::digest(expected).into();
        assert_eq!(writer.finish(), digest);
    }
}

#[test]
fn embedded_nul_text_is_encoded_as_raw_bytes() {
    let mut writer = Writer::new();
    encode_cell(&mut writer, ValueRef::Text(b"a\0b")).unwrap();
    let expected = [0x03, 0, 0, 0, 0, 0, 0, 0, 3, b'a', 0x00, b'b'];
    assert_eq!(writer.bytes, expected.len() as u64);
    let digest: [u8; 32] = Sha256::digest(expected).into();
    assert_eq!(writer.finish(), digest);
}

#[test]
fn large_blob_streams_in_chunks_and_is_sensitive_to_middle_bytes() {
    let sha = "a".repeat(64);
    let build = |bytes: Vec<u8>| -> [u8; 32] {
        let (_dir, mut conn) = fresh();
        conn.execute(
            "INSERT INTO source_file (sha256, source_path, len, bytes) VALUES (?1, '/x', ?2, ?3)",
            rusqlite::params![sha, bytes.len() as i64, bytes],
        )
        .unwrap();
        witness_of(&mut conn).sha256
    };
    let blob = vec![0xABu8; 200_000];
    let first = build(blob.clone());
    let mut changed = blob;
    changed[100_000] ^= 0xFF;
    let second = build(changed);
    assert_ne!(first, second);
}

#[test]
fn admission_refusals_propagate_typed_errors() {
    // Empty schema at the admitted user_version cannot be admitted.
    let mut conn = rusqlite::Connection::open_in_memory().unwrap();
    conn.pragma_update(
        None,
        "user_version",
        crate::migration::CURRENT_PRODUCTDB_VERSION,
    )
    .unwrap();
    let tx = conn.transaction().unwrap();
    assert!(matches!(
        witness_current(&tx, &PublicationGate::new()),
        Err(WitnessError::Admission(
            AdmissionError::UnsupportedSchemaShape
        ))
    ));

    // Wrong version refuses before any data streaming.
    let mut conn = rusqlite::Connection::open_in_memory().unwrap();
    conn.pragma_update(None, "user_version", 27).unwrap();
    let tx = conn.transaction().unwrap();
    assert!(matches!(
        witness_current(&tx, &PublicationGate::new()),
        Err(WitnessError::Admission(AdmissionError::UnsupportedVersion(
            27
        )))
    ));
}

#[test]
fn precancelled_witness_returns_typed_stop() {
    let (_dir, mut conn) = fresh();
    let tx = conn.transaction().unwrap();
    let gate = PublicationGate::new();
    assert_eq!(gate.try_cancel(), crate::operation::CancelOutcome::Accepted);
    assert!(matches!(
        witness_current(&tx, &gate),
        Err(WitnessError::Stop(crate::operation::Stop::CancelAccepted))
    ));
    assert!(!tx.is_autocommit());
}

#[test]
fn cell_length_bound_helper_rejects_oversized_and_negative() {
    assert!(matches!(
        admitted_cell_length(None),
        Err(WitnessError::LengthMissing)
    ));
    assert!(matches!(
        admitted_cell_length(Some(-1)),
        Err(WitnessError::NegativeLength(-1))
    ));
    assert_eq!(admitted_cell_length(Some(0)).unwrap(), 0);
    assert_eq!(admitted_cell_length(Some(1)).unwrap(), 1);
    assert_eq!(
        admitted_cell_length(Some(CELL_BYTE_BOUND as i64)).unwrap(),
        CELL_BYTE_BOUND
    );
    assert!(matches!(
        admitted_cell_length(Some(CELL_BYTE_BOUND as i64 + 1)),
        Err(WitnessError::CellTooLarge(len)) if len == CELL_BYTE_BOUND + 1
    ));
}

#[test]
fn copy_witness_omits_schema_version_while_the_live_witness_tracks_it() {
    let (_dir, conn) = fresh();
    conn.execute(
        "INSERT INTO manufacturer (id, name) VALUES ('m1', 'Example Co')",
        [],
    )
    .unwrap();

    let first = {
        let tx = conn.unchecked_transaction().unwrap();
        let live = witness_current(&tx, &PublicationGate::new()).unwrap();
        let copy = copy_content_witness(&tx, &PublicationGate::new()).unwrap();
        (live, copy)
    };

    // A no-op DDL round-trip bumps the schema cookie but leaves the 120
    // objects and their rowids intact.
    conn.execute_batch("CREATE TABLE scratch(x); DROP TABLE scratch;")
        .unwrap();

    let second = {
        let tx = conn.unchecked_transaction().unwrap();
        let live = witness_current(&tx, &PublicationGate::new()).unwrap();
        let copy = copy_content_witness(&tx, &PublicationGate::new()).unwrap();
        (live, copy)
    };

    assert_ne!(first.0.sha256, second.0.sha256);
    assert_eq!(first.1.sha256, second.1.sha256);
    assert_eq!(first.1.table_count, first.0.table_count);
    assert_eq!(first.1.schema_object_count, first.0.schema_object_count);
    assert_eq!(first.1.row_count, first.0.row_count);
    // The copy witness is a distinct domain from the live witness.
    assert_ne!(first.1.sha256, first.0.sha256);
}

#[test]
fn copied_destination_matches_the_pinned_source_content() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("catalogue.db");
    let conn = crate::open_and_migrate(&path).unwrap();
    conn.execute(
        "INSERT INTO manufacturer (id, name) VALUES ('m1', 'Example Co')",
        [],
    )
    .unwrap();
    drop(conn);

    let ro = crate::open_read_only(&path).unwrap();
    let page_size: i64 = ro
        .conn
        .query_row("PRAGMA page_size", [], |r| r.get(0))
        .unwrap();
    let tx = ro.conn.unchecked_transaction().unwrap();
    let n: i64 = tx
        .query_row("SELECT count(*) FROM sqlite_schema", [], |r| r.get(0))
        .unwrap();
    assert!(n > 0);
    let source_witness = copy_content_witness(&tx, &PublicationGate::new()).unwrap();

    let copy_path = dir.path().join("copy.db");
    let mut dest = rusqlite::Connection::open(&copy_path).unwrap();
    dest.pragma_update(None, "page_size", page_size).unwrap();
    crate::catalogue_copy::copy_pinned_current(&ro.conn, &mut dest, &PublicationGate::new())
        .unwrap();
    drop(dest);

    let dro = crate::open_read_only(&copy_path).unwrap();
    let dtx = dro.conn.unchecked_transaction().unwrap();
    let destination_witness = copy_content_witness(&dtx, &PublicationGate::new()).unwrap();

    assert_eq!(source_witness.sha256, destination_witness.sha256);
    assert_eq!(source_witness.table_count, destination_witness.table_count);
    assert_eq!(source_witness.row_count, destination_witness.row_count);
    assert_eq!(
        source_witness.schema_object_count,
        destination_witness.schema_object_count
    );
}
