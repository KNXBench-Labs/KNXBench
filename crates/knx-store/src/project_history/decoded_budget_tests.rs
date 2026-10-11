//! Tiny resource-preflight witnesses; these are not valid native snapshots.

use super::*;

#[test]
fn cumulative_declared_expansion_is_refused_before_inflating_any_table() {
    // Untrusted headers deliberately have no backing large allocation or valid
    // zlib stream. Excessive declared expansion must fail before inflation.
    for second_table in ["state", "stack", "version", "context"] {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE project_history_state(id INTEGER, working BLOB);
            CREATE TABLE project_history_stack(image BLOB);
            CREATE TABLE project_history_version(image BLOB);
            CREATE TABLE project_history_context(format_version INTEGER, content_hash TEXT, image_hash TEXT, image BLOB);").unwrap();
        let mut header = b"KNXHST\0\x01".to_vec();
        header.extend_from_slice(&((MAX_DECODED_IMAGE_BYTES / 2 + 1) as u64).to_le_bytes());
        header.push(0);
        conn.execute(
            "INSERT INTO project_history_context VALUES(1, ?1, ?1, ?2)",
            params!["a".repeat(64), header],
        )
        .unwrap();
        match second_table {
            "state" => {
                conn.execute("INSERT INTO project_history_state VALUES(0, ?1)", [&header])
                    .unwrap();
            }
            "stack" => {
                conn.execute("INSERT INTO project_history_stack VALUES(?1)", [&header])
                    .unwrap();
            }
            "version" => {
                conn.execute("INSERT INTO project_history_version VALUES(?1)", [&header])
                    .unwrap();
            }
            _ => {
                conn.execute(
                    "INSERT INTO project_history_context VALUES(1, ?1, ?1, ?2)",
                    params!["b".repeat(64), header],
                )
                .unwrap();
            }
        }
        let before = stored_history_bytes(&conn).unwrap();
        assert!(before < 4096);
        assert!(
            matches!(check_limits(&conn), Err(HistoryError::ImageLimit(reason)) if reason.contains("total decoded history")),
            "cumulative preflight must refuse before decoding"
        );
        assert_eq!(stored_history_bytes(&conn).unwrap(), before);
        let retained: Vec<u8> = conn
            .query_row(
                "SELECT image FROM project_history_context LIMIT 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(retained, header);
    }
}
