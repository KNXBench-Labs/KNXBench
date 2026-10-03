//! Independent, versioned storage for bounded-size activity metadata.
//!
//! This database is not a project file or a recovery image. The application
//! owns the closed metadata vocabulary; storage preserves its document intact.
use std::fs::{File, OpenOptions};
use std::io::{self, Read};
use std::path::Path;
use std::time::Duration;

use rusqlite::{params, Connection, OpenFlags};

pub const FORMAT: u32 = 1;
const APPLICATION_ID: i32 = 0x4B4E5841;
pub const MAX_DOCUMENT_BYTES: usize = 4096;
pub const MAX_PAGE_SIZE: usize = 100;
// SQLite file-format §1.3: bytes 18/19 select rollback (1) versus WAL (2).
const SQLITE_HEADER_PREFIX_BYTES: usize = 20;
const SQLITE_JOURNAL_VERSION_RANGE: std::ops::Range<usize> = 18..20;

const ACTIVITY_TABLE: &str = "CREATE TABLE activity (
                    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
                    incarnation TEXT NOT NULL,
                    operation_id INTEGER NOT NULL CHECK(operation_id > 0),
                    document TEXT NOT NULL CHECK(length(CAST(document AS BLOB)) <= 4096),
                    UNIQUE(incarnation, operation_id)
                )";

pub struct ActivityHistory {
    connection: Connection,
}

pub struct StoredActivity {
    pub sequence: u64,
    pub operation_id: u64,
    pub incarnation: String,
    pub document: String,
}

fn invalid() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "activity history is invalid or unsupported",
    )
}

fn database_error(_: rusqlite::Error) -> io::Error {
    io::Error::other("activity history storage is unavailable")
}

impl ActivityHistory {
    /// Refuse nonempty foreign/versioned databases before changing their schema.
    /// New files are owner-only; existing files are never replaced or truncated.
    pub fn open(path: &Path) -> io::Result<Self> {
        Self::open_inner(path, true)
    }

    /// Refuse missing or blank storage after the application's first admission.
    pub fn open_existing(path: &Path) -> io::Result<Self> {
        Self::open_inner(path, false)
    }

    fn open_inner(path: &Path, allow_initialize: bool) -> io::Result<Self> {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let created = if allow_initialize {
            match options.open(path) {
                Ok(file) => {
                    file.sync_all()?;
                    true
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => false,
                Err(error) => return Err(error),
            }
        } else {
            false
        };
        if !created {
            // Even read-only WAL queries can create/write a -shm sidecar.
            // Version 1 supports rollback mode only; inspect before SQLite.
            let mut header = Vec::with_capacity(SQLITE_HEADER_PREFIX_BYTES);
            File::open(path)?
                .take(SQLITE_HEADER_PREFIX_BYTES as u64)
                .read_to_end(&mut header)?;
            if !header.is_empty()
                && (header.len() != SQLITE_HEADER_PREFIX_BYTES
                    || !header.starts_with(b"SQLite format 3\0")
                    || header[SQLITE_JOURNAL_VERSION_RANGE] != [1, 1])
            {
                return Err(invalid());
            }
            // Writable reads can recover a hot journal before admission.
            let readonly = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
                .map_err(database_error)?;
            readonly
                .busy_timeout(Duration::from_secs(1))
                .map_err(database_error)?;
            Self::initialization_needed(&readonly, allow_initialize)?;
        }
        let mut connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_WRITE)
            .map_err(database_error)?;
        connection
            .busy_timeout(Duration::from_secs(1))
            .map_err(database_error)?;
        if Self::initialization_needed(&connection, allow_initialize)? {
            let transaction = connection.transaction().map_err(database_error)?;
            transaction
                .execute_batch(ACTIVITY_TABLE)
                .map_err(database_error)?;
            transaction
                .pragma_update(None, "application_id", APPLICATION_ID)
                .map_err(database_error)?;
            transaction
                .pragma_update(None, "user_version", FORMAT)
                .map_err(database_error)?;
            transaction.commit().map_err(database_error)?;
        }
        Self::validate_schema(&connection)?;
        // FULL is a storage policy, not a KNX timing or recovery guarantee.
        connection
            .pragma_update(None, "synchronous", "FULL")
            .map_err(database_error)?;
        connection
            .execute_batch(
                "SELECT sequence, incarnation, operation_id, document FROM activity LIMIT 0;",
            )
            .map_err(database_error)?;
        if created {
            if let Some(parent) = path.parent() {
                File::open(parent)?.sync_all()?;
            }
        }
        Ok(Self { connection })
    }

    fn initialization_needed(connection: &Connection, allow_initialize: bool) -> io::Result<bool> {
        let application_id: i32 = connection
            .pragma_query_value(None, "application_id", |row| row.get(0))
            .map_err(database_error)?;
        let version: u32 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(database_error)?;
        if application_id == 0 && version == 0 && allow_initialize {
            let objects: i64 = connection
                .query_row("SELECT count(*) FROM sqlite_master", [], |row| row.get(0))
                .map_err(database_error)?;
            if objects == 0 {
                return Ok(true);
            }
        }
        if application_id != APPLICATION_ID || version != FORMAT {
            return Err(invalid());
        }
        Self::validate_schema(connection)?;
        Ok(false)
    }

    fn validate_schema(connection: &Connection) -> io::Result<()> {
        let schema: String = connection
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'activity'",
                [],
                |row| row.get(0),
            )
            .map_err(database_error)?;
        let foreign_objects: i64 = connection.query_row(
            "SELECT count(*) FROM sqlite_master WHERE name NOT IN ('activity', 'sqlite_sequence', 'sqlite_autoindex_activity_1')", [], |row| row.get(0),
        ).map_err(database_error)?;
        if schema != ACTIVITY_TABLE || foreign_objects != 0 {
            return Err(invalid());
        }
        Ok(())
    }

    pub fn record(&self, incarnation: &str, operation_id: u64, document: &str) -> io::Result<()> {
        if incarnation.is_empty() || incarnation.len() > 64 || document.len() > MAX_DOCUMENT_BYTES {
            return Err(invalid());
        }
        let id = i64::try_from(operation_id).map_err(|_| invalid())?;
        if id <= 0 {
            return Err(invalid());
        }
        self.connection
            .execute(
                "INSERT INTO activity (incarnation, operation_id, document) VALUES (?1, ?2, ?3)
             ON CONFLICT(incarnation, operation_id) DO UPDATE SET document = excluded.document",
                params![incarnation, id, document],
            )
            .map_err(database_error)?;
        Ok(())
    }

    /// Ascending operation-start order; a cursor is not an incremental update feed.
    /// The extra row determines pagination without an unbounded COUNT or read.
    pub fn page(&self, after: u64, limit: usize) -> io::Result<(Vec<StoredActivity>, bool)> {
        if limit == 0 || limit > MAX_PAGE_SIZE {
            return Err(invalid());
        }
        let after = i64::try_from(after).map_err(|_| invalid())?;
        let mut statement = self.connection.prepare(
            "SELECT sequence, operation_id, substr(incarnation, 1, 65), substr(document, 1, 4097)
             FROM activity WHERE sequence > ?1 ORDER BY sequence LIMIT ?2",
        ).map_err(database_error)?;
        let rows = statement
            .query_map(params![after, (limit + 1) as i64], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .map_err(database_error)?;
        let mut entries = Vec::new();
        for row in rows {
            let (sequence, operation_id, incarnation, document) = row.map_err(database_error)?;
            let sequence = u64::try_from(sequence).map_err(|_| invalid())?;
            let operation_id = u64::try_from(operation_id).map_err(|_| invalid())?;
            if sequence == 0 || operation_id == 0 {
                return Err(invalid());
            }
            let row = StoredActivity {
                sequence,
                operation_id,
                incarnation,
                document,
            };
            if row.incarnation.is_empty()
                || row.incarnation.len() > 64
                || row.document.len() > MAX_DOCUMENT_BYTES
            {
                return Err(invalid());
            }
            entries.push(row);
        }
        let has_more = entries.len() > limit;
        entries.truncate(limit);
        Ok((entries, has_more))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wal_history_is_refused_before_sidecar_creation_or_checkpoint() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source.sqlite");
        drop(ActivityHistory::open(&source).unwrap());
        let connection = Connection::open(&source).unwrap();
        let mode: String = connection
            .query_row("PRAGMA journal_mode=WAL", [], |row| row.get(0))
            .unwrap();
        assert_eq!(mode, "wal");
        connection.execute("INSERT INTO activity(incarnation, operation_id, document) VALUES ('synthetic', 1, '{}')", []).unwrap();
        let path = dir.path().join("activity.sqlite");
        let wal = path.with_extension("sqlite-wal");
        let shm = path.with_extension("sqlite-shm");
        std::fs::copy(&source, &path).unwrap();
        std::fs::copy(source.with_extension("sqlite-wal"), &wal).unwrap();
        let before = std::fs::read(&path).unwrap();
        let wal_before = std::fs::read(&wal).unwrap();
        assert!(!shm.exists());
        for existing_only in [false, true] {
            let result = if existing_only {
                ActivityHistory::open_existing(&path)
            } else {
                ActivityHistory::open(&path)
            };
            assert!(result.is_err(), "WAL mode must remain unsupported");
            assert!(!shm.exists(), "refusal created a shared-memory sidecar");
            assert!(std::fs::read(&path).unwrap() == before);
            assert!(std::fs::read(&wal).unwrap() == wal_before);
        }
    }

    #[test]
    fn hot_journal_refusal_preserves_database_and_journal() {
        const CHILD_PATH: &str = "KNXBENCH_TEST_ACTIVITY_HOT_JOURNAL";
        if let Some(path) = std::env::var_os(CHILD_PATH) {
            let connection = Connection::open(path).unwrap();
            connection.execute_batch("PRAGMA cache_size=5; BEGIN IMMEDIATE; UPDATE foreign_data SET value=randomblob(2097152);").unwrap();
            // Intentionally bypass Drop: leave a real interrupted transaction.
            std::process::exit(0);
        }
        for version in [0, 1, 2] {
            for existing_only in [false, true] {
                let dir = tempfile::tempdir().unwrap();
                let path = dir.path().join("activity.sqlite");
                let connection = Connection::open(&path).unwrap();
                connection.execute_batch("CREATE TABLE foreign_data(value BLOB); INSERT INTO foreign_data VALUES(zeroblob(2097152));").unwrap();
                if version != 0 {
                    connection
                        .pragma_update(None, "application_id", APPLICATION_ID)
                        .unwrap();
                    connection
                        .pragma_update(None, "user_version", version)
                        .unwrap();
                }
                drop(connection);
                let child = std::process::Command::new(std::env::current_exe().unwrap())
                    .args(["--exact", "activity_history::tests::hot_journal_refusal_preserves_database_and_journal", "--nocapture"])
                    .env(CHILD_PATH, &path)
                    .output()
                    .unwrap();
                assert!(child.status.success(), "fixture child failed");
                let journal = path.with_extension("sqlite-journal");
                let before = std::fs::read(&path).unwrap();
                let journal_before = std::fs::read(&journal).unwrap();
                assert!(journal_before.len() > 512 && journal_before[..8].iter().any(|b| *b != 0));
                let result = if existing_only {
                    ActivityHistory::open_existing(&path)
                } else {
                    ActivityHistory::open(&path)
                };
                assert!(result.is_err());
                assert!(
                    std::fs::read(&path).unwrap() == before,
                    "database changed on refusal"
                );
                assert!(journal.exists(), "journal removed on refusal");
                assert!(
                    std::fs::read(journal).unwrap() == journal_before,
                    "journal changed on refusal"
                );
            }
        }
    }

    #[test]
    fn claimed_format_with_a_foreign_schema_is_preserved_and_refused() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("activity.sqlite");
        drop(ActivityHistory::open(&path).unwrap());
        let connection = Connection::open(&path).unwrap();
        connection.execute_batch("DROP TABLE activity; CREATE TABLE activity(sequence INTEGER, incarnation TEXT, operation_id INTEGER, document TEXT);").unwrap();
        drop(connection);
        let before = std::fs::read(&path).unwrap();
        assert!(ActivityHistory::open(&path).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }

    #[test]
    fn records_reopen_and_updates_do_not_duplicate_an_operation() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("activity.sqlite");
        let history = ActivityHistory::open(&path).unwrap();
        history.record("first", 1, "running").unwrap();
        history.record("first", 1, "unknown").unwrap();
        history.record("second", 1, "finished").unwrap();
        drop(history);
        let reopened = ActivityHistory::open(&path).unwrap();
        let (rows, more) = reopened.page(0, 1).unwrap();
        assert!(more);
        assert_eq!(rows[0].document, "unknown");
        let (last, more) = reopened.page(rows[0].sequence, 1).unwrap();
        assert!(!more);
        assert_eq!(last.len(), 1);
        assert_eq!(last[0].incarnation, "second");
        assert_eq!(last[0].document, "finished");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }

    #[test]
    fn foreign_and_future_databases_are_preserved_on_refusal() {
        for future in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("activity.sqlite");
            if future {
                drop(ActivityHistory::open(&path).unwrap());
                let connection = Connection::open(&path).unwrap();
                connection.pragma_update(None, "user_version", 2).unwrap();
            } else {
                let connection = Connection::open(&path).unwrap();
                connection.execute_batch("CREATE TABLE unrelated(value TEXT); INSERT INTO unrelated VALUES ('retained');").unwrap();
            }
            let before = std::fs::read(&path).unwrap();
            assert!(ActivityHistory::open(&path).is_err());
            assert_eq!(std::fs::read(path).unwrap(), before);
        }
    }

    #[test]
    fn invalid_bounds_and_ids_do_not_change_history() {
        let dir = tempfile::tempdir().unwrap();
        let history = ActivityHistory::open(&dir.path().join("activity.sqlite")).unwrap();
        assert!(history.record("", 1, "bad").is_err());
        assert!(history.record(&"x".repeat(65), 1, "bad").is_err());
        assert!(history.record("one", 0, "bad").is_err());
        assert!(history.record("one", u64::MAX, "bad").is_err());
        assert!(history
            .record("one", 1, &"x".repeat(MAX_DOCUMENT_BYTES + 1))
            .is_err());
        assert!(history.page(0, 0).is_err());
        assert!(history.page(0, MAX_PAGE_SIZE + 1).is_err());
        assert!(history.page(u64::MAX, 1).is_err());
        assert!(history.page(0, 100).unwrap().0.is_empty());
    }
}
