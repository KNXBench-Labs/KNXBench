//! Persistence for `knx_core::StringTable` — the `string_table_entry`
//! table, one row per `(key, language)` pair (DATA_MODEL §8).

use rusqlite::{params, Connection};

use knx_core::string_table::{Language, StringTable, TranslationKey};

use crate::StoreError;

pub fn upsert_string_table(conn: &Connection, table: &StringTable) -> Result<(), StoreError> {
    // A `SAVEPOINT` rather than `conn.unchecked_transaction()`: this is
    // called both standalone (this module's own tests) and nested inside
    // `knx_store::project::save_project`'s own transaction. `BEGIN` (which
    // `unchecked_transaction()` issues) fails with "cannot start a
    // transaction within a transaction" in the nested case; `SAVEPOINT` is
    // valid at any nesting depth, including with no transaction open yet.
    conn.execute_batch("SAVEPOINT knx_store_upsert_string_table")?;
    let result = (|| -> Result<(), StoreError> {
        conn.execute("DELETE FROM string_table_entry", [])?;
        let mut stmt = conn
            .prepare("INSERT INTO string_table_entry (key, language, value) VALUES (?1, ?2, ?3)")?;
        for (key, language, value) in table.iter() {
            stmt.execute(params![key.0, language.0, value])?;
        }
        Ok(())
    })();
    match result {
        Ok(()) => {
            conn.execute_batch("RELEASE knx_store_upsert_string_table")?;
            Ok(())
        }
        Err(e) => {
            // Best-effort unwind; the original error is what's reported.
            let _ = conn.execute_batch(
                "ROLLBACK TO knx_store_upsert_string_table; \
                 RELEASE knx_store_upsert_string_table",
            );
            Err(e)
        }
    }
}

pub fn load_string_table(
    conn: &Connection,
    default_language: Language,
) -> Result<StringTable, StoreError> {
    let mut table = StringTable::new(default_language);
    let mut stmt = conn.prepare("SELECT key, language, value FROM string_table_entry")?;
    let rows = stmt.query_map([], |row| {
        let key: String = row.get(0)?;
        let language: String = row.get(1)?;
        let value: String = row.get(2)?;
        Ok((key, language, value))
    })?;
    for row in rows {
        let (key, language, value) = row?;
        table.insert(TranslationKey(key), Language(language), value);
    }
    Ok(table)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_and_migrate_in_memory;

    #[test]
    fn a_string_table_round_trips() {
        let conn = open_and_migrate_in_memory().unwrap();
        let mut table = StringTable::new(Language("en".into()));
        table.insert(
            TranslationKey("k1".into()),
            Language("de".into()),
            "Licht".into(),
        );
        table.insert(
            TranslationKey("k1".into()),
            Language("en".into()),
            "Light".into(),
        );
        upsert_string_table(&conn, &table).unwrap();
        let loaded = load_string_table(&conn, Language("en".into())).unwrap();
        assert_eq!(loaded, table);
    }

    #[test]
    fn an_empty_string_table_round_trips() {
        let conn = open_and_migrate_in_memory().unwrap();
        let table = StringTable::new(Language("de-DE".into()));
        upsert_string_table(&conn, &table).unwrap();
        let loaded = load_string_table(&conn, Language("de-DE".into())).unwrap();
        assert_eq!(loaded, table);
    }

    #[test]
    fn upsert_replaces_rather_than_accumulates() {
        let conn = open_and_migrate_in_memory().unwrap();
        let mut table = StringTable::new(Language("en".into()));
        table.insert(
            TranslationKey("k1".into()),
            Language("en".into()),
            "A".into(),
        );
        upsert_string_table(&conn, &table).unwrap();
        table.insert(
            TranslationKey("k2".into()),
            Language("en".into()),
            "B".into(),
        );
        upsert_string_table(&conn, &table).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM string_table_entry", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 2);
    }
}
