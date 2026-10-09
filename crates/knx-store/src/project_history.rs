//! Versioned native snapshots, durable editor recovery and atomic project time travel.

use std::fmt;

use crate::StoreError;
use knx_core::{CommandStack, Project};
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};

/// Read-only admission precedes a non-creating writable connection. Schema
/// upgrades are deferred to the history operation's own transaction.
pub fn open_editor_store(
    path: &std::path::Path,
    allow_empty: bool,
) -> Result<Connection, HistoryError> {
    match crate::open_existing_read_only(path) {
        Ok(admitted) => {
            load_editor(&admitted.conn)?;
            list_versions(&admitted.conn)?;
        }
        Err(crate::MigrationError::NothingSaved) if allow_empty => {}
        Err(error) => return Err(error.into()),
    }
    let conn = Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    Ok(conn)
}

pub const FORMAT_VERSION: i64 = 1;
pub const MAX_IMAGE_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_HISTORY_BYTES: i64 = 512 * 1024 * 1024;
pub const MAX_STACK_STATES: usize = 256;
pub const MAX_VERSIONS: usize = 256;

#[derive(Debug)]
pub enum HistoryError {
    Migration(crate::MigrationError),
    Sqlite(rusqlite::Error),
    Store(StoreError),
    Invalid(&'static str),
    Limit(&'static str),
    Stale,
}

impl fmt::Display for HistoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Migration(error) => write!(f, "{error}"),
            Self::Sqlite(error) => write!(f, "project history storage: {error}"),
            Self::Store(error) => write!(f, "project history snapshot: {error}"),
            Self::Invalid(reason) => write!(f, "project history is unavailable: {reason}; no history was discarded"),
            Self::Limit(reason) => write!(f, "project history limit: {reason}; explicitly delete versions or clear undo history before retrying"),
            Self::Stale => write!(f, "project history changed in another editor; reopen before editing or restoring"),
        }
    }
}

impl std::error::Error for HistoryError {}
impl From<crate::MigrationError> for HistoryError {
    fn from(error: crate::MigrationError) -> Self {
        Self::Migration(error)
    }
}
impl From<rusqlite::Error> for HistoryError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Sqlite(value)
    }
}
impl From<StoreError> for HistoryError {
    fn from(value: StoreError) -> Self {
        Self::Store(value)
    }
}

mod context;
mod snapshot;
pub use snapshot::{image_hash, NativeSnapshot};
#[derive(Debug)]
pub struct EditorHistory {
    pub generation: i64,
    pub baseline: NativeSnapshot,
    pub working: NativeSnapshot,
    pub undo: Vec<Project>,
    pub redo: Vec<Project>,
}

#[derive(Debug, Clone)]
pub struct ProjectVersion {
    pub id: i64,
    pub created_at: String,
    pub reason: String,
    pub label: String,
    pub bytes: i64,
    pub image_hash: String,
}

pub fn stored_history_bytes(conn: &Connection) -> Result<i64, HistoryError> {
    Ok(conn.query_row(
        "SELECT coalesce((SELECT sum(length(working)) FROM project_history_state), 0)
         + coalesce((SELECT sum(length(image)) FROM project_history_stack), 0)
         + coalesce((SELECT sum(length(image)) FROM project_history_version), 0)
         + coalesce((SELECT sum(length(image)) FROM project_history_context), 0)",
        [],
        |row| row.get(0),
    )?)
}

fn check_limits(conn: &Connection) -> Result<(), HistoryError> {
    let oversized: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM project_history_state WHERE length(working) > ?1)
          OR EXISTS(SELECT 1 FROM project_history_stack WHERE length(image) > ?1)
          OR EXISTS(SELECT 1 FROM project_history_version WHERE length(image) > ?1)
          OR EXISTS(SELECT 1 FROM project_history_context WHERE length(image) > ?1)",
        [MAX_IMAGE_BYTES as i64],
        |r| r.get(0),
    )?;
    if oversized {
        return Err(HistoryError::Limit("one native snapshot exceeds 64 MiB"));
    }
    let (contexts, unsupported): (i64, i64) = conn.query_row(
        "SELECT count(*), coalesce(sum(format_version != 1 OR length(content_hash) != 64 OR length(image_hash) != 64
         OR content_hash GLOB '*[^0-9a-f]*' OR image_hash GLOB '*[^0-9a-f]*'), 0) FROM project_history_context",
        [], |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    if contexts > (MAX_STACK_STATES + MAX_VERSIONS + 1) as i64 || unsupported != 0 {
        return Err(HistoryError::Invalid(
            "unsupported or excessive retained-context records",
        ));
    }
    let (states, wrong_ids): (i64, i64) = conn.query_row(
        "SELECT count(*), coalesce(sum(id != 0), 0) FROM project_history_state",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    if states > 1 || wrong_ids != 0 {
        return Err(HistoryError::Invalid("unexpected workspace row identity"));
    }
    if stored_history_bytes(conn)? > MAX_HISTORY_BYTES {
        return Err(HistoryError::Limit("total history exceeds 512 MiB"));
    }
    let steps: i64 = conn.query_row("SELECT count(*) FROM project_history_stack", [], |r| {
        r.get(0)
    })?;
    let versions: i64 =
        conn.query_row("SELECT count(*) FROM project_history_version", [], |r| {
            r.get(0)
        })?;
    if steps > MAX_STACK_STATES as i64 {
        return Err(HistoryError::Limit("undo and redo exceed 256 states"));
    }
    if versions > MAX_VERSIONS as i64 {
        return Err(HistoryError::Limit("more than 256 project versions"));
    }
    Ok(())
}

fn load_editor_inner(conn: &Connection) -> Result<Option<EditorHistory>, HistoryError> {
    snapshot::check_schema(conn)?;
    check_limits(conn)?;
    let state = conn.query_row(
        "SELECT format_version, generation, baseline_hash, working, working_hash, context_hash FROM project_history_state WHERE id = 0",
        [], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?, r.get::<_, String>(2)?, r.get::<_, Vec<u8>>(3)?, r.get::<_, String>(4)?, r.get::<_, String>(5)?)),
    ).optional()?;
    let Some((format, generation, baseline_hash, image, hash, context_hash)) = state else {
        let steps: i64 = conn.query_row("SELECT count(*) FROM project_history_stack", [], |r| {
            r.get(0)
        })?;
        if steps != 0 {
            return Err(HistoryError::Invalid("stack without a workspace"));
        }
        return Ok(None);
    };
    if format != FORMAT_VERSION || generation <= 0 || generation > 9_007_199_254_740_991 {
        return Err(HistoryError::Invalid(
            "unknown workspace version or generation",
        ));
    }
    let baseline = NativeSnapshot::read(conn)?;
    if baseline.semantic_hash()? != baseline_hash {
        return Err(HistoryError::Stale);
    }
    let retained = context::read(conn, &context_hash)?;
    let working = context::attach(&image, &hash, &retained)?;
    if working.project.info.project_id != baseline.project.info.project_id {
        return Err(HistoryError::Invalid(
            "workspace belongs to another project",
        ));
    }
    let mut undo = Vec::new();
    let mut redo = Vec::new();
    let mut stmt = conn.prepare("SELECT side, position, format_version, image, image_hash, context_hash FROM project_history_stack ORDER BY side, position")?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let side: String = row.get(0)?;
        let position: i64 = row.get(1)?;
        let format: i64 = row.get(2)?;
        let image: Vec<u8> = row.get(3)?;
        let hash: String = row.get(4)?;
        let stack_context: String = row.get(5)?;
        if stack_context != context_hash {
            return Err(HistoryError::Invalid(
                "stack retained context differs from the working project",
            ));
        }
        let target = match side.as_str() {
            "undo" => &mut undo,
            "redo" => &mut redo,
            _ => return Err(HistoryError::Invalid("unknown stack side")),
        };
        if position != target.len() as i64 || format != FORMAT_VERSION {
            return Err(HistoryError::Invalid(
                "unknown stack version or position gap",
            ));
        }
        let snapshot = context::attach(&image, &hash, &retained)?;
        if snapshot.project.info.project_id != working.project.info.project_id
            || snapshot.opaque != working.opaque
            || snapshot.manufacturer_refs != working.manufacturer_refs
        {
            return Err(HistoryError::Invalid(
                "stack snapshot belongs to another project or retained context",
            ));
        }
        target.push(snapshot.project);
    }
    Ok(Some(EditorHistory {
        generation,
        baseline,
        working,
        undo,
        redo,
    }))
}

/// A consistent read transaction: root, journal and stack come from one revision.
pub fn load_editor(conn: &Connection) -> Result<Option<EditorHistory>, HistoryError> {
    if !conn.is_autocommit() {
        return load_editor_inner(conn);
    }
    let tx = conn.unchecked_transaction()?;
    let history = load_editor_inner(&tx)?;
    tx.commit()?;
    Ok(history)
}

fn admitted_generation(conn: &Connection, expected: Option<i64>) -> Result<i64, HistoryError> {
    let current = load_editor_inner(conn)?.map_or(0, |h| h.generation);
    if expected.is_some_and(|e| e != current) {
        return Err(HistoryError::Stale);
    }
    Ok(current)
}

fn next_generation(current: i64) -> Result<i64, HistoryError> {
    current
        .checked_add(1)
        .filter(|next| *next <= 9_007_199_254_740_991)
        .ok_or(HistoryError::Invalid("generation exhausted"))
}

fn write_workspace(
    conn: &Connection,
    baseline: &NativeSnapshot,
    working: &NativeSnapshot,
    stack: &CommandStack,
    generation: i64,
) -> Result<(), HistoryError> {
    let (undo_steps, redo_steps) = stack.history_lengths();
    if undo_steps.saturating_add(redo_steps) > MAX_STACK_STATES {
        return Err(HistoryError::Limit("undo and redo exceed 256 states"));
    }
    let baseline_hash = baseline.semantic_hash()?;
    let context_hash = context::remember(conn, working)?;
    let image = context::model_image(&working.project)?;
    conn.execute("DELETE FROM project_history_stack", [])?;
    conn.execute(
        "INSERT INTO project_history_state (id, format_version, generation, baseline_hash, working, working_hash, context_hash)
         VALUES (0, ?1, ?2, ?3, ?4, ?5, ?6) ON CONFLICT(id) DO UPDATE SET
         format_version=excluded.format_version, generation=excluded.generation, baseline_hash=excluded.baseline_hash,
         working=excluded.working, working_hash=excluded.working_hash, context_hash=excluded.context_hash",
        params![FORMAT_VERSION, generation, baseline_hash, image, image_hash(&image), context_hash],
    )?;
    context::collect_unreferenced(conn)?;
    let mut bytes = stored_history_bytes(conn)?;
    if bytes > MAX_HISTORY_BYTES {
        return Err(HistoryError::Limit("total history exceeds 512 MiB"));
    }
    let mut stmt = conn.prepare("INSERT INTO project_history_stack (side, position, format_version, image, image_hash, context_hash) VALUES (?1, ?2, ?3, ?4, ?5, ?6)")?;
    let mut refusal = None;
    let complete = stack
        .visit_snapshot_states(&working.project, |side, position, project| {
            let result = (|| -> Result<(), HistoryError> {
                if project.info.project_id != working.project.info.project_id {
                    return Err(HistoryError::Invalid("cross-project stack state"));
                }
                let image = context::model_image(project)?;
                bytes = bytes
                    .checked_add(image.len() as i64)
                    .ok_or(HistoryError::Limit("history byte accounting overflow"))?;
                if bytes > MAX_HISTORY_BYTES {
                    return Err(HistoryError::Limit("total history exceeds 512 MiB"));
                }
                let side = match side {
                    knx_core::command::HistorySide::Undo => "undo",
                    knx_core::command::HistorySide::Redo => "redo",
                };
                stmt.execute(params![
                    side,
                    position as i64,
                    FORMAT_VERSION,
                    image,
                    image_hash(&image),
                    context_hash
                ])?;
                Ok(())
            })();
            match result {
                Ok(()) => true,
                Err(error) => {
                    refusal = Some(error);
                    false
                }
            }
        })
        .map_err(|_| {
            HistoryError::Invalid("an inverse cannot replay; history was not discarded")
        })?;
    if !complete {
        return Err(refusal.unwrap_or(HistoryError::Invalid("history admission stopped")));
    }
    check_limits(conn)
}

fn append_version(
    conn: &Connection,
    snapshot: &NativeSnapshot,
    reason: &str,
    label: &str,
) -> Result<(), HistoryError> {
    if label.trim().is_empty() || label.chars().count() > 120 {
        return Err(HistoryError::Invalid(
            "version label must contain 1 to 120 characters",
        ));
    }
    let context_hash = context::remember(conn, snapshot)?;
    let image = context::model_image(&snapshot.project)?;
    conn.execute(
        "INSERT INTO project_history_version (format_version, created_at, reason, label, image, image_hash, context_hash) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![FORMAT_VERSION, chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true), reason, label, image, image_hash(&image), context_hash],
    )?;
    check_limits(conn)
}

fn write_root(tx: &Transaction<'_>, snapshot: &NativeSnapshot) -> Result<(), HistoryError> {
    crate::project::write_project(tx, &snapshot.project)?;
    crate::opaque::write_opaque(tx, &snapshot.opaque)?;
    crate::manifest::write_manufacturer_refs(tx, &snapshot.manufacturer_refs)?;
    Ok(())
}

/// Saves the working journal, optionally promoting it to the explicit saved
/// baseline. Generation/baseline admission and all rows share one write lock.
pub fn save_editor(
    conn: &Connection,
    working: &NativeSnapshot,
    stack: &CommandStack,
    expected_generation: Option<i64>,
    save_root: bool,
) -> Result<i64, HistoryError> {
    let (undo_steps, redo_steps) = stack.history_lengths();
    if undo_steps.saturating_add(redo_steps) > MAX_STACK_STATES {
        return Err(HistoryError::Limit("undo and redo exceed 256 states"));
    }
    let tx = Transaction::new_unchecked(conn, TransactionBehavior::Immediate)?;
    crate::migration::migrate_in_transaction(&tx)?;
    snapshot::check_schema(&tx)?;
    let current = admitted_generation(&tx, expected_generation)?;
    list_versions_inner(&tx)?;
    let generation = next_generation(current)?;
    let old = match NativeSnapshot::read(&tx) {
        Ok(snapshot) => Some(snapshot),
        Err(HistoryError::Store(StoreError::NotSaved)) => None,
        Err(error) => return Err(error),
    };
    if save_root {
        if expected_generation.is_none() {
            if let Some(previous) = load_editor_inner(&tx)? {
                if previous.working != previous.baseline && previous.working != *working {
                    append_version(
                        &tx,
                        &previous.working,
                        "replaced_workspace",
                        "Recovered work before replacement",
                    )?;
                }
            }
        }
        if let Some(old) = old.as_ref().filter(|old| *old != working) {
            append_version(&tx, old, "save", "Previous saved state")?;
        }
        write_root(&tx, working)?;
    }
    let baseline = if save_root {
        working
    } else {
        old.as_ref().ok_or(HistoryError::Invalid(
            "workspace requires a saved native project",
        ))?
    };
    write_workspace(&tx, baseline, working, stack, generation)?;
    tx.commit()?;
    Ok(generation)
}

fn list_versions_inner(conn: &Connection) -> Result<Vec<ProjectVersion>, HistoryError> {
    check_limits(conn)?;
    let mut stmt = conn.prepare("SELECT id, format_version, created_at, reason, label, image, image_hash, context_hash FROM project_history_version ORDER BY id DESC")?;
    let mut rows = stmt.query([])?;
    let mut result = Vec::new();
    let mut contexts = std::collections::HashMap::new();
    while let Some(row) = rows.next()? {
        let id: i64 = row.get(0)?;
        let format: i64 = row.get(1)?;
        let created_at: String = row.get(2)?;
        let reason: String = row.get(3)?;
        let label: String = row.get(4)?;
        let image: Vec<u8> = row.get(5)?;
        let hash: String = row.get(6)?;
        let context_hash: String = row.get(7)?;
        let context_bytes = if let Some(bytes) = contexts.get(&context_hash) {
            *bytes
        } else {
            context::read(conn, &context_hash)?;
            let bytes: i64 = conn.query_row(
                "SELECT length(image) FROM project_history_context WHERE content_hash = ?1",
                [&context_hash],
                |r| r.get(0),
            )?;
            contexts.insert(context_hash, bytes);
            bytes
        };
        if id <= 0
            || id > 9_007_199_254_740_991
            || format != FORMAT_VERSION
            || !["save", "named", "pre_restore", "replaced_workspace"].contains(&reason.as_str())
            || label.trim().is_empty()
            || label.chars().count() > 120
            || chrono::DateTime::parse_from_rfc3339(&created_at).is_err()
            || image.len() > MAX_IMAGE_BYTES
            || image.len() < 100
            || image_hash(&image) != hash
        {
            return Err(HistoryError::Invalid(
                "invalid version metadata, length or SHA-256",
            ));
        }
        result.push(ProjectVersion {
            id,
            created_at,
            reason,
            label,
            bytes: image.len() as i64 + context_bytes,
            image_hash: hash,
        });
    }
    Ok(result)
}

pub fn list_versions(conn: &Connection) -> Result<Vec<ProjectVersion>, HistoryError> {
    if !conn.is_autocommit() {
        return list_versions_inner(conn);
    }
    let tx = conn.unchecked_transaction()?;
    let result = list_versions_inner(&tx)?;
    tx.commit()?;
    Ok(result)
}

fn read_version_inner(conn: &Connection, id: i64) -> Result<NativeSnapshot, HistoryError> {
    let (format, image, hash, context_hash): (i64, Vec<u8>, String, String) = conn.query_row(
        "SELECT format_version, image, image_hash, context_hash FROM project_history_version WHERE id = ?1", [id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
    ).optional()?.ok_or(HistoryError::Invalid("version does not exist"))?;
    if format != FORMAT_VERSION {
        return Err(HistoryError::Invalid("unsupported version envelope"));
    }
    context::attach(&image, &hash, &context::read(conn, &context_hash)?)
}

pub fn read_version(conn: &Connection, id: i64) -> Result<NativeSnapshot, HistoryError> {
    let tx = conn.unchecked_transaction()?;
    check_limits(&tx)?;
    let result = read_version_inner(&tx, id)?;
    tx.commit()?;
    Ok(result)
}

pub enum HistoryAction<'a> {
    Create(&'a str),
    Delete(i64),
    Restore(i64),
    ClearUndo,
}

/// All response evidence is assembled while the write transaction is held,
/// before commit. Adapters never need a fallible read after publishing state.
pub struct HistoryChange {
    pub generation: i64,
    pub versions: Vec<ProjectVersion>,
    pub total_bytes: i64,
    pub restored: Option<EditorHistory>,
}

pub fn change_history(
    conn: &Connection,
    current_working: &NativeSnapshot,
    expected: i64,
    action: HistoryAction<'_>,
) -> Result<HistoryChange, HistoryError> {
    let tx = Transaction::new_unchecked(conn, TransactionBehavior::Immediate)?;
    crate::migration::migrate_in_transaction(&tx)?;
    snapshot::check_schema(&tx)?;
    let generation = next_generation(admitted_generation(&tx, Some(expected))?)?;
    list_versions_inner(&tx)?;
    let baseline = NativeSnapshot::read(&tx)?;
    if current_working.project.info.project_id != baseline.project.info.project_id {
        return Err(HistoryError::Invalid(
            "current working snapshot belongs to another project",
        ));
    }
    // A migrated legacy store may have versions but no editor workspace yet.
    if load_editor_inner(&tx)?.is_none() {
        write_workspace(
            &tx,
            &baseline,
            current_working,
            &CommandStack::new(),
            generation,
        )?;
    }
    let restored = match action {
        HistoryAction::Create(label) => {
            append_version(&tx, current_working, "named", label)?;
            None
        }
        HistoryAction::Delete(id) => {
            if tx.execute("DELETE FROM project_history_version WHERE id = ?1", [id])? != 1 {
                return Err(HistoryError::Invalid("version does not exist"));
            }
            None
        }
        HistoryAction::ClearUndo => {
            write_workspace(
                &tx,
                &baseline,
                current_working,
                &CommandStack::new(),
                generation,
            )?;
            None
        }
        HistoryAction::Restore(id) => {
            let mut replacement = read_version_inner(&tx, id)?;
            replacement
                .project
                .ids
                .raise_to(&current_working.project.ids);
            append_version(
                &tx,
                current_working,
                "pre_restore",
                "Before restoring a project version",
            )?;
            write_root(&tx, &replacement)?;
            write_workspace(
                &tx,
                &replacement,
                &replacement,
                &CommandStack::new(),
                generation,
            )?;
            Some(EditorHistory {
                generation,
                baseline: replacement.clone(),
                working: replacement,
                undo: Vec::new(),
                redo: Vec::new(),
            })
        }
    };
    if tx.execute(
        "UPDATE project_history_state SET generation = ?1 WHERE id = 0",
        [generation],
    )? != 1
    {
        return Err(HistoryError::Invalid("native workspace not initialized"));
    }
    context::collect_unreferenced(&tx)?;
    check_limits(&tx)?;
    let versions = list_versions_inner(&tx)?;
    let total_bytes = stored_history_bytes(&tx)?;
    tx.commit()?;
    Ok(HistoryChange {
        generation,
        versions,
        total_bytes,
        restored,
    })
}

pub fn create_version(
    conn: &Connection,
    snapshot: &NativeSnapshot,
    label: &str,
    expected: i64,
) -> Result<i64, HistoryError> {
    change_history(conn, snapshot, expected, HistoryAction::Create(label))
        .map(|result| result.generation)
}

pub fn delete_version(conn: &Connection, id: i64, expected: i64) -> Result<i64, HistoryError> {
    let current =
        load_editor(conn)?.map_or_else(|| NativeSnapshot::read(conn), |h| Ok(h.working))?;
    change_history(conn, &current, expected, HistoryAction::Delete(id))
        .map(|result| result.generation)
}

/// Preserve the exact current project before restoring the chosen version.
pub fn restore_version(
    conn: &Connection,
    id: i64,
    current_working: &NativeSnapshot,
    expected: i64,
) -> Result<EditorHistory, HistoryError> {
    change_history(conn, current_working, expected, HistoryAction::Restore(id))?
        .restored
        .ok_or(HistoryError::Invalid("restore did not yield a project"))
}

/// A plain store writer must not silently discard a recovered workspace. This
/// hook runs inside its existing project/passthrough transaction, before writes.
pub(crate) fn before_plain_save(
    tx: &Transaction<'_>,
    replacement: &NativeSnapshot,
) -> Result<(), HistoryError> {
    snapshot::check_schema(tx)?;
    let old = match NativeSnapshot::read(tx) {
        Ok(snapshot) => snapshot,
        Err(HistoryError::Store(StoreError::NotSaved)) => return Ok(()),
        Err(error) => return Err(error),
    };
    let previous = load_editor_inner(tx)?;
    list_versions_inner(tx)?;
    if let Some(previous) = &previous {
        if previous.working != previous.baseline && previous.working != *replacement {
            append_version(
                tx,
                &previous.working,
                "replaced_workspace",
                "Recovered work before replacement",
            )?;
        }
    }
    if old != *replacement {
        append_version(tx, &old, "save", "Previous saved state")?;
    }
    if previous.is_some() {
        let generation = next_generation(previous.map_or(0, |h| h.generation))?;
        write_workspace(
            tx,
            replacement,
            replacement,
            &CommandStack::new(),
            generation,
        )?;
    }
    Ok(())
}
