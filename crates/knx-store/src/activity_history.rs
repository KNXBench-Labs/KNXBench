//! Independent, versioned storage for bounded-size activity metadata.
//!
//! This database is not a project file or a recovery image. The application
//! owns the closed metadata vocabulary; storage preserves its document intact.
use std::fs::{File, OpenOptions};
use std::io::{self, Read};
use std::path::Path;
use std::time::Duration;

use rusqlite::{params, Connection, OpenFlags};

pub const FORMAT: u32 = 2;
const LEGACY_FORMAT: u32 = 1;
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

enum Admission {
    Initialize,
    Upgrade,
    Current,
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
        // Owned stores use DELETE journaling. Unknown sidecar entries must not be
        // tidied by SQLite during opening or even initial sentinel creation.
        for suffix in ["-wal", "-shm", "-journal"] {
            let mut sidecar = path.as_os_str().to_os_string();
            sidecar.push(suffix);
            match std::fs::symlink_metadata(Path::new(&sidecar)) {
                Ok(_) => return Err(invalid()),
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }
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
            // Supported history formats are rollback-only; inspect before SQLite.
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
            Self::admission(&readonly, allow_initialize)?;
        }
        let mut connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_WRITE)
            .map_err(database_error)?;
        connection
            .busy_timeout(Duration::from_secs(1))
            .map_err(database_error)?;
        let admission = Self::admission(&connection, allow_initialize)?;
        // EXTRA also syncs the directory after a DELETE journal is unlinked.
        // Configure before the first transaction; this is storage policy,
        // not a KNX timing or universal power-loss recovery guarantee.
        connection
            .pragma_update(None, "synchronous", "EXTRA")
            .map_err(database_error)?;
        if !matches!(admission, Admission::Current) {
            let transaction = connection.transaction().map_err(database_error)?;
            if matches!(admission, Admission::Initialize) {
                transaction
                    .execute_batch(ACTIVITY_TABLE)
                    .map_err(database_error)?;
                transaction
                    .pragma_update(None, "application_id", APPLICATION_ID)
                    .map_err(database_error)?;
            }
            // A positively admitted legacy database keeps every document,
            // operation key and sequence. Only the compatibility marker changes.
            transaction
                .pragma_update(None, "user_version", FORMAT)
                .map_err(database_error)?;
            transaction.commit().map_err(database_error)?;
        }
        Self::validate_schema(&connection)?;
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

    fn admission(connection: &Connection, allow_initialize: bool) -> io::Result<Admission> {
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
                return Ok(Admission::Initialize);
            }
        }
        if application_id != APPLICATION_ID || !matches!(version, LEGACY_FORMAT | FORMAT) {
            return Err(invalid());
        }
        Self::validate_schema(connection)?;
        Ok(if version == LEGACY_FORMAT {
            Admission::Upgrade
        } else {
            Admission::Current
        })
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
    fn orphan_sqlite_sidecar_is_refused_before_any_history_open() {
        for suffix in ["-wal", "-shm", "-journal"] {
            for existing_only in [false, true] {
                for main_exists in [true, false] {
                    for payload in [&b""[..], &b"synthetic unknown sidecar"[..]] {
                        let dir = tempfile::tempdir().unwrap();
                        let path = dir.path().join("activity.sqlite");
                        let before = if main_exists {
                            let history = ActivityHistory::open(&path).unwrap();
                            history
                                .record("synthetic-orphan", 1, r#"{"retained":true}"#)
                                .unwrap();
                            let (rows, more) = history.page(0, 1).unwrap();
                            assert!(!more && rows.len() == 1);
                            assert_eq!(rows[0].document, r#"{"retained":true}"#);
                            drop(history);
                            let bytes = std::fs::read(&path).unwrap();
                            assert!(bytes.get(18..20) == Some(&[1, 1][..]));
                            Some(bytes)
                        } else {
                            None
                        };
                        let mut name = path.as_os_str().to_os_string();
                        name.push(suffix);
                        let sidecar = std::path::PathBuf::from(name);
                        std::fs::write(&sidecar, payload).unwrap();
                        let result = if existing_only {
                            ActivityHistory::open_existing(&path)
                        } else {
                            ActivityHistory::open(&path)
                        };
                        assert!(result.is_err(), "orphan sidecar was admitted");
                        match before {
                            Some(bytes) => assert!(std::fs::read(&path).unwrap() == bytes),
                            None => assert!(!path.exists(), "refusal created a main file"),
                        }
                        assert!(std::fs::read(&sidecar).unwrap() == payload);
                        for other in ["-wal", "-shm", "-journal"] {
                            if other != suffix {
                                let mut name = path.as_os_str().to_os_string();
                                name.push(other);
                                assert!(!std::path::PathBuf::from(name).exists());
                            }
                        }
                    }
                }
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn orphan_sidecar_entries_are_not_followed_or_cleaned() {
        for suffix in ["-wal", "-shm", "-journal"] {
            for existing_only in [false, true] {
                for kind in ["directory", "symlink", "dangling-symlink"] {
                    let dir = tempfile::tempdir().unwrap();
                    let path = dir.path().join("activity.sqlite");
                    let history = ActivityHistory::open(&path).unwrap();
                    history.record("synthetic-entry", 1, "retained").unwrap();
                    drop(history);
                    let before = std::fs::read(&path).unwrap();
                    let target = dir.path().join("retained-target");
                    let mut name = path.as_os_str().to_os_string();
                    name.push(suffix);
                    let sidecar = std::path::PathBuf::from(name);
                    match kind {
                        "directory" => {
                            std::fs::create_dir(&sidecar).unwrap();
                            std::fs::write(sidecar.join("sentinel"), b"retained child").unwrap();
                        }
                        "symlink" => {
                            std::fs::write(&target, b"retained target").unwrap();
                            std::os::unix::fs::symlink(&target, &sidecar).unwrap();
                        }
                        "dangling-symlink" => {
                            std::os::unix::fs::symlink(&target, &sidecar).unwrap();
                        }
                        _ => unreachable!(),
                    }
                    let result = if existing_only {
                        ActivityHistory::open_existing(&path)
                    } else {
                        ActivityHistory::open(&path)
                    };
                    assert!(result.is_err(), "sidecar entry was admitted");
                    assert!(std::fs::read(&path).unwrap() == before);
                    match kind {
                        "directory" => {
                            assert!(std::fs::symlink_metadata(&sidecar).unwrap().is_dir());
                            assert!(
                                std::fs::read(sidecar.join("sentinel")).unwrap()
                                    == b"retained child"
                            );
                            assert_eq!(std::fs::read_dir(&sidecar).unwrap().count(), 1);
                        }
                        "symlink" | "dangling-symlink" => {
                            assert!(std::fs::symlink_metadata(&sidecar).unwrap().is_symlink());
                            assert_eq!(std::fs::read_link(&sidecar).unwrap(), target);
                            if kind == "symlink" {
                                assert!(std::fs::read(&target).unwrap() == b"retained target");
                            } else {
                                assert!(!target.exists(), "refusal created the link target");
                            }
                        }
                        _ => unreachable!(),
                    }
                }
            }
        }
    }

    fn seed_legacy_history(path: &Path) -> (&'static str, &'static str) {
        let first = r#"{"id":1,"kind":"serviceControlRead","address":"1.1.1","state":"finished","startedAt":"2026-10-01T00:00:00Z","finishedAt":"2026-10-01T00:00:01Z"}"#;
        let second = r#"{"id":2,"kind":"serviceControlRead","address":"1.1.1","state":"running","startedAt":"2026-10-01T00:00:02Z","finishedAt":null}"#;
        let connection = Connection::open(path).unwrap();
        // Published v1 schema, independent of the production schema constant.
        connection
            .execute_batch(
                "CREATE TABLE activity (
                    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
                    incarnation TEXT NOT NULL,
                    operation_id INTEGER NOT NULL CHECK(operation_id > 0),
                    document TEXT NOT NULL CHECK(length(CAST(document AS BLOB)) <= 4096),
                    UNIQUE(incarnation, operation_id)
                )",
            )
            .unwrap();
        connection
            .pragma_update(None, "application_id", 0x4B4E5841_i32)
            .unwrap();
        connection.pragma_update(None, "user_version", 1).unwrap();
        for (sequence, id, document) in [(3, 1, first), (11, 2, second)] {
            connection
                .execute(
                    "INSERT INTO activity(sequence, incarnation, operation_id, document) VALUES (?1, 'synthetic-legacy', ?2, ?3)",
                    params![sequence, id, document],
                )
                .unwrap();
        }
        let count: i64 = connection
            .query_row("SELECT count(*) FROM activity", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 2, "nonempty legacy fixture");
        drop(connection);
        (first, second)
    }

    #[test]
    fn legacy_upgrade_preserves_documents_keys_and_cursor_sequence() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("activity.sqlite");
        let (first, second) = seed_legacy_history(&path);

        let history = ActivityHistory::open_existing(&path).unwrap();
        let version: u32 = history
            .connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, 2);
        let (rows, more) = history.page(0, 100).unwrap();
        assert!(!more);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].sequence, 3);
        assert_eq!(rows[0].operation_id, 1);
        assert_eq!(rows[0].incarnation, "synthetic-legacy");
        assert_eq!(rows[0].document, first);
        assert_eq!(rows[1].sequence, 11);
        assert_eq!(rows[1].operation_id, 2);
        assert_eq!(rows[1].incarnation, "synthetic-legacy");
        assert_eq!(rows[1].document, second);
        let sequence: i64 = history
            .connection
            .query_row(
                "SELECT seq FROM sqlite_sequence WHERE name = 'activity'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(sequence, 11);
        drop(history);

        let reopened = ActivityHistory::open_existing(&path).unwrap();
        let (rows, more) = reopened.page(3, 1).unwrap();
        assert!(!more);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].sequence, 11);
        assert_eq!(rows[0].document, second);
        reopened.record("synthetic-new", 1, "new receipt").unwrap();
        let (rows, more) = reopened.page(11, 1).unwrap();
        assert!(!more);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].sequence, 12);
        assert_eq!(rows[0].incarnation, "synthetic-new");
        assert_eq!(rows[0].operation_id, 1);
        assert_eq!(rows[0].document, "new receipt");
    }

    #[test]
    fn legacy_upgrade_write_failure_preserves_version_and_nonempty_history() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("activity.sqlite");
        let (first, second) = seed_legacy_history(&path);
        let blocker = Connection::open(&path).unwrap();
        blocker.execute_batch("BEGIN IMMEDIATE").unwrap();
        let before = std::fs::read(&path).unwrap();
        assert!(ActivityHistory::open_existing(&path).is_err());
        assert!(std::fs::read(&path).unwrap() == before);
        let version: u32 = blocker
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, 1);
        let documents: Vec<String> = blocker
            .prepare("SELECT document FROM activity ORDER BY sequence")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(documents, [first, second]);
        assert!(!path.with_extension("sqlite-wal").exists());
        assert!(!path.with_extension("sqlite-shm").exists());
        blocker.execute_batch("ROLLBACK").unwrap();
        drop(blocker);
        // Explicitly resolving the fixture's competing transaction permits
        // a later open. The application failure latch is a separate contract.
        let history = ActivityHistory::open_existing(&path).unwrap();
        let (rows, more) = history.page(0, 100).unwrap();
        assert!(!more);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].document, first);
        assert_eq!(rows[1].document, second);
    }

    #[test]
    fn legacy_upgrade_reader_blocked_commit_preserves_nonempty_history() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("activity.sqlite");
        let (first, second) = seed_legacy_history(&path);
        let before = std::fs::read(&path).unwrap();
        let journal = path.with_extension("sqlite-journal");
        assert!(!journal.exists(), "late-migration/journal-precondition");

        // A reader permits a RESERVED writer but prevents the final exclusive lock.
        let reader = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        reader.execute_batch("BEGIN DEFERRED").unwrap();
        let count: i64 = reader
            .query_row("SELECT count(*) FROM activity", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 2, "late-migration/nonempty-reader");

        let (sender, receiver) = std::sync::mpsc::channel();
        let worker_path = path.clone();
        let (rejected, journal_seen) = std::thread::scope(|scope| {
            let worker = scope.spawn(move || {
                let rejected = ActivityHistory::open_existing(&worker_path).is_err();
                sender.send(rejected).expect("late-migration/worker-result");
            });
            let deadline = std::time::Instant::now() + Duration::from_secs(6);
            let mut journal_seen = false;
            let rejected = loop {
                match std::fs::metadata(&journal) {
                    Ok(metadata) => journal_seen |= metadata.len() > 0,
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                    Err(_) => panic!("late-migration/journal-observation"),
                }
                match receiver.try_recv() {
                    Ok(rejected) => break rejected,
                    Err(std::sync::mpsc::TryRecvError::Empty) => {}
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        panic!("late-migration/worker-disconnected");
                    }
                }
                assert!(
                    std::time::Instant::now() < deadline,
                    "late-migration/worker-deadline"
                );
                std::thread::yield_now();
            };
            worker.join().expect("late-migration/worker-join");
            (rejected, journal_seen)
        });
        // Require a real dirty rollback journal, not just an early admission error.
        assert!(journal_seen, "late-migration/started-write-witness");
        assert!(rejected, "late-migration/finalization-refused");
        assert!(
            std::fs::read(&path).unwrap() == before,
            "late-migration/main-preserved"
        );
        let version: u32 = reader
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, 1, "late-migration/version-preserved");
        let rows: Vec<(i64, String, i64, String)> = reader
            .prepare("SELECT sequence, incarnation, operation_id, document FROM activity ORDER BY sequence")
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(
            rows,
            [
                (3, "synthetic-legacy".to_owned(), 1, first.to_owned()),
                (11, "synthetic-legacy".to_owned(), 2, second.to_owned()),
            ],
            "late-migration/rows-preserved"
        );
        let sequence: i64 = reader
            .query_row(
                "SELECT seq FROM sqlite_sequence WHERE name = 'activity'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(sequence, 11, "late-migration/counter-preserved");
        for suffix in ["sqlite-wal", "sqlite-shm", "sqlite-journal"] {
            assert!(
                !path.with_extension(suffix).exists(),
                "late-migration/no-retained-sidecar"
            );
        }
        // Only the test operator resolves its own reader; no application auto-retry.
        reader.execute_batch("ROLLBACK").unwrap();
        drop(reader);
        let history = ActivityHistory::open_existing(&path).unwrap();
        let (rows, more) = history.page(0, 100).unwrap();
        assert!(!more);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].document, first);
        assert_eq!(rows[1].document, second);
        history
            .record("synthetic-new", 1, "after explicit reader release")
            .unwrap();
        let (rows, more) = history.page(11, 1).unwrap();
        assert!(!more);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].sequence, 12, "late-migration/cursor-preserved");
    }

    #[test]
    fn legacy_foreign_schema_is_refused_before_upgrade_by_both_openers() {
        for existing_only in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("activity.sqlite");
            seed_legacy_history(&path);
            let connection = Connection::open(&path).unwrap();
            connection.execute_batch("CREATE TABLE unrelated(value TEXT); INSERT INTO unrelated VALUES ('retained');").unwrap();
            drop(connection);
            let before = std::fs::read(&path).unwrap();
            let result = if existing_only {
                ActivityHistory::open_existing(&path)
            } else {
                ActivityHistory::open(&path)
            };
            assert!(result.is_err());
            assert!(std::fs::read(&path).unwrap() == before);
            assert!(!path.with_extension("sqlite-wal").exists());
            assert!(!path.with_extension("sqlite-shm").exists());
        }
    }

    #[test]
    fn admitted_connections_use_extra_synchronization() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("activity.sqlite");
        let history = ActivityHistory::open(&path).unwrap();
        let synchronization: i64 = history
            .connection
            .pragma_query_value(None, "synchronous", |row| row.get(0))
            .unwrap();
        assert_eq!(synchronization, 3);
        let journal: String = history
            .connection
            .pragma_query_value(None, "journal_mode", |row| row.get(0))
            .unwrap();
        assert_eq!(journal, "delete");
        history.record("synthetic", 1, "retained").unwrap();
        drop(history);
        let reopened = ActivityHistory::open_existing(&path).unwrap();
        let synchronization: i64 = reopened
            .connection
            .pragma_query_value(None, "synchronous", |row| row.get(0))
            .unwrap();
        assert_eq!(synchronization, 3);
        let (rows, more) = reopened.page(0, 1).unwrap();
        assert!(!more);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].document, "retained");
    }

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
        for version in [0, LEGACY_FORMAT, FORMAT, FORMAT + 1] {
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
                connection
                    .pragma_update(None, "user_version", FORMAT + 1)
                    .unwrap();
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
