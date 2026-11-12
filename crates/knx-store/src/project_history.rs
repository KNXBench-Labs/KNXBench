//! Versioned native snapshots, durable editor recovery and atomic project time travel.

use std::collections::{BTreeMap, HashSet};
use std::fmt;
use std::time::Instant;

use crate::parameter_baseline::PruneBasis;
use crate::StoreError;
use knx_core::commissioning::parameter_baseline::{BaselineEdit, ParameterBaselines};
use knx_core::commissioning::pending::{settle_restored_version, ProgrammingParts};
use knx_core::{CommandStack, DeviceId, Project};
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};

/// Read-only admission precedes a non-creating writable connection. Schema
/// upgrades are deferred to the history operation's own transaction.
pub fn open_editor_store(
    path: &std::path::Path,
    allow_empty: bool,
) -> Result<Connection, HistoryError> {
    open_editor_store_trusting(path, allow_empty, &JournalTrust::default())
}

/// [`open_editor_store`] whose read-only admission checks what `trusted`
/// knows by byte hash or value equality instead of decoding it again
/// ([`StackRead::Admit`]).
pub fn open_editor_store_trusting(
    path: &std::path::Path,
    allow_empty: bool,
    trusted: &JournalTrust,
) -> Result<Connection, HistoryError> {
    match crate::open_existing_read_only(path) {
        Ok(admitted) => {
            let tx = admitted.conn.unchecked_transaction()?;
            let context = admit_editor(&tx, trusted)?.map(|(_, context)| context);
            list_versions_checked(&tx, trusted, context.as_ref())?;
            tx.commit()?;
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
/// One encoded model image: the storage ceiling, as each image is one stored
/// value (ADR-0142; formerly 64 MiB).
pub const MAX_IMAGE_BYTES: usize = knx_core::limits::MAX_RETAINED_BYTES_USIZE;
/// One shared source context, not a per-edit model image. It carries the
/// project's opaque data, embedded product data included, so it gets the
/// storage ceiling (ADR-0142; formerly 256 MiB, below one measured program).
pub const MAX_CONTEXT_BYTES: usize = knx_core::limits::MAX_RETAINED_BYTES_USIZE;
/// Independent decoding budget; model and shared-context storage remain separately bounded.
pub const MAX_DECODED_IMAGE_BYTES: usize = HISTORY_BUDGET_BYTES;
/// All stored history images together (ADR-0142; formerly 512 MiB).
pub const MAX_HISTORY_BYTES: i64 = HISTORY_BUDGET_BYTES as i64;
/// Cumulative decoded charge for all physical images, counting shared context once.
pub const MAX_DECODED_HISTORY_BYTES: usize = HISTORY_BUDGET_BYTES;
/// The cumulative history budgets: the 4 GiB expansion budget
/// (`knx_core::limits::MAX_EXPANDED_ARCHIVE_BYTES`), which holds a maximal
/// context several times over.
const HISTORY_BUDGET_BYTES: usize = knx_core::limits::MAX_EXPANDED_ARCHIVE_BYTES as usize;
pub const MAX_STACK_STATES: usize = 256;
pub const MAX_VERSIONS: usize = 256;

#[derive(Debug)]
pub enum HistoryError {
    Migration(crate::MigrationError),
    Sqlite(rusqlite::Error),
    Store(StoreError),
    Invalid(&'static str),
    Limit(&'static str),
    /// Image-admission budget refusal; deleting history is not assumed to repair it.
    ImageLimit(&'static str),
    Carried(crate::ManufacturerFileError),
    Stale,
}

impl fmt::Display for HistoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Migration(error) => write!(f, "{error}"),
            Self::Sqlite(error) => write!(f, "project history storage: {error}"),
            Self::Store(error) => write!(f, "project history snapshot: {error}"),
            Self::Invalid(reason) => write!(
                f,
                "project history is unavailable: {reason}; no history was discarded"
            ),
            Self::Limit(reason) => write!(
                f,
                "project history limit: {reason}; explicitly delete versions or clear undo history before retrying"
            ),
            Self::ImageLimit(reason) => write!(f, "project history image limit: {reason}; project data was not discarded; this context requires a supported image budget"),
            Self::Carried(error) => write!(f, "project retained source: {error}"),
            Self::Stale => write!(
                f,
                "project history changed in another editor; reopen before editing or restoring"
            ),
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
#[cfg(test)]
mod decoded_budget_tests;
mod image_codec;
mod snapshot;
mod trust;
#[cfg(test)]
mod trust_tests;
mod write_report;
pub use snapshot::{image_hash, NativeSnapshot};
pub use trust::JournalTrust;
pub use write_report::JournalWriteReport;
#[derive(Debug)]
pub struct EditorHistory {
    pub generation: i64,
    pub baseline: NativeSnapshot,
    pub working: NativeSnapshot,
    pub undo: Vec<Project>,
    pub redo: Vec<Project>,
    /// The stored image hash of every `undo`/`redo` state, in the same order.
    pub rows: StackRows,
    /// What the read verified beyond `rows`.
    verified: Option<VerifiedHashes>,
    /// The knowledge a [`load_editor_with_trust`] read verified, captured with
    /// the values it read so that later changes to the public fields cannot
    /// alter it (see [`Self::take_trust`]). Other reads leave it empty.
    opened: Option<JournalTrust>,
}

/// The identities one journal read checked: the retained context's semantic
/// and stored-image hashes, the baseline's semantic hash and the working
/// state's stored-image hash.
#[derive(Debug, Clone)]
struct VerifiedHashes {
    context: String,
    context_image: String,
    baseline: String,
    working_image: String,
}

impl EditorHistory {
    /// Moves out what this read verified, for a writer that keeps working on
    /// it: the rows and, after [`load_editor_with_trust`], the retained context
    /// of `working`, the saved baseline and the working state, with the values
    /// read from the file. Changing `working`, `baseline`, `undo` or `redo`
    /// afterwards does not change it. A second call knows only the rows.
    pub fn take_trust(&mut self) -> JournalTrust {
        self.opened
            .take()
            .unwrap_or_else(|| JournalTrust::from_rows(self.rows.clone()))
    }
}

/// The knowledge of a full read from what it verified and the values it read.
fn opened_trust(
    rows: &StackRows,
    verified: &VerifiedHashes,
    baseline: &NativeSnapshot,
    working: &NativeSnapshot,
) -> JournalTrust {
    let context = trust::KnownContext {
        opaque: working.opaque.clone(),
        manufacturer_refs: working.manufacturer_refs.clone(),
        content_hash: verified.context.clone(),
        image_hash: verified.context_image.clone(),
    };
    JournalTrust::knowing(
        rows.clone(),
        context,
        baseline,
        &verified.baseline,
        (working.project.clone(), verified.working_image.clone()),
    )
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
          OR EXISTS(SELECT 1 FROM project_history_context WHERE length(image) > ?2)",
        [MAX_IMAGE_BYTES as i64, MAX_CONTEXT_BYTES as i64],
        |r| r.get(0),
    )?;
    if oversized {
        return Err(HistoryError::ImageLimit(
            "encoded native snapshot exceeds its model/context budget",
        ));
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
        return Err(HistoryError::Limit("total history exceeds 4 GiB"));
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
    // Return only fixed-size prefixes after row/stored-byte caps, before inflation.
    // SQLite may materialize a bounded stored blob internally for substr().
    // Do not trust compressed length declarations
    // as integrity evidence: exact length, hash and stream checks still follow.
    let mut stmt = conn.prepare(
        "SELECT substr(working,1,16), length(working) FROM project_history_state
        UNION ALL SELECT substr(image,1,16), length(image) FROM project_history_stack
        UNION ALL SELECT substr(image,1,16), length(image) FROM project_history_version
        UNION ALL SELECT substr(image,1,16), length(image) FROM project_history_context",
    )?;
    let mut rows = stmt.query([])?;
    let mut total = 0usize;
    while let Some(row) = rows.next()? {
        let prefix: Vec<u8> = row.get(0)?;
        let stored = usize::try_from(row.get::<_, i64>(1)?)
            .map_err(|_| HistoryError::Invalid("invalid stored image length"))?;
        total = total
            .checked_add(image_codec::decoded_size(&prefix, stored)?)
            .filter(|total| *total <= MAX_DECODED_HISTORY_BYTES)
            .ok_or(HistoryError::ImageLimit(
                "total decoded history exceeds 4 GiB",
            ))?;
    }
    Ok(())
}

/// What one pass over the stored journal keeps of each stack row.
enum StackRead<'a> {
    /// Decode and check every row and return its project state.
    States,
    /// As `States`, and also capture the read's [`JournalTrust`].
    StatesWithTrust,
    /// Check only, returning no states. A row whose stored hash the trust's
    /// `rows` name is verified by the SHA-256 of its bytes instead of being
    /// decoded: the caller already holds that image's state, encoded by itself
    /// or decoded and admitted when the file was opened. So is a retained
    /// context the trust knows, and a saved baseline equal to the known one
    /// takes the known semantic hash. Everything else is decoded and checked as
    /// in `States`.
    Admit(&'a JournalTrust),
}

fn load_editor_inner(conn: &Connection) -> Result<Option<EditorHistory>, HistoryError> {
    read_editor(conn, StackRead::States)
}

/// [`load_editor`] that also captures what the read verified, for a writer
/// that keeps editing the journal ([`EditorHistory::take_trust`]). The capture
/// copies the retained context and the baseline and working models once.
pub fn load_editor_with_trust(conn: &Connection) -> Result<Option<EditorHistory>, HistoryError> {
    if !conn.is_autocommit() {
        return read_editor(conn, StackRead::StatesWithTrust);
    }
    let tx = conn.unchecked_transaction()?;
    let history = read_editor(conn, StackRead::StatesWithTrust)?;
    tx.commit()?;
    Ok(history)
}

/// A retained context row whose bytes were checked: its semantic identity and
/// the recorded SHA-256 of its stored bytes.
type ContextIdentity = (String, String);

/// Admission without materializing states; see [`StackRead::Admit`].
/// Answers the admitted generation, or `None` without a workspace, and the
/// working state's retained context, which it checked.
fn admit_editor(
    conn: &Connection,
    trusted: &JournalTrust,
) -> Result<Option<(i64, ContextIdentity)>, HistoryError> {
    Ok(
        read_editor(conn, StackRead::Admit(trusted))?.and_then(|history| {
            let verified = history.verified?;
            Some((
                history.generation,
                (verified.context, verified.context_image),
            ))
        }),
    )
}

fn read_editor(
    conn: &Connection,
    read: StackRead<'_>,
) -> Result<Option<EditorHistory>, HistoryError> {
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
    let actual_baseline_hash = match read {
        StackRead::Admit(trust) => trust.semantic_hash(&baseline)?,
        StackRead::States | StackRead::StatesWithTrust => baseline.semantic_hash()?,
    };
    if actual_baseline_hash != baseline_hash {
        return Err(HistoryError::Stale);
    }
    let known_context = match read {
        StackRead::Admit(trust) => trust
            .context()
            .filter(|known| known.content_hash == context_hash),
        StackRead::States | StackRead::StatesWithTrust => None,
    };
    // Admission returns no states, so a verified known context needs no value:
    // every state below attaches the same one.
    let (retained, context_image) = match known_context {
        Some(known) if context::verify_known(conn, known)? => {
            (context::unvalued(), known.image_hash.clone())
        }
        _ => context::read_with_image_hash(conn, &context_hash)?,
    };
    let working = context::attach(&image, &hash, &retained)?;
    let trusted_rows: HashSet<&str> = match read {
        StackRead::Admit(trust) => trust.rows.hashes(),
        StackRead::States | StackRead::StatesWithTrust => HashSet::new(),
    };
    if working.project.info.project_id != baseline.project.info.project_id {
        return Err(HistoryError::Invalid(
            "workspace belongs to another project",
        ));
    }
    let mut undo = Vec::new();
    let mut redo = Vec::new();
    let mut rows_read = StackRows::default();
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
        let (target, hashes) = match side.as_str() {
            "undo" => (&mut undo, &mut rows_read.undo),
            "redo" => (&mut redo, &mut rows_read.redo),
            _ => return Err(HistoryError::Invalid("unknown stack side")),
        };
        if position != hashes.len() as i64 || format != FORMAT_VERSION {
            return Err(HistoryError::Invalid(
                "unknown stack version or position gap",
            ));
        }
        // Only admission trusts rows; `States` decodes all of them.
        if trusted_rows.contains(hash.as_str()) {
            if image_hash(&image) != hash {
                return Err(HistoryError::Invalid("stack image hash mismatch"));
            }
            hashes.push(hash);
            continue;
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
        if matches!(read, StackRead::States | StackRead::StatesWithTrust) {
            target.push(snapshot.project);
        }
        hashes.push(hash);
    }
    let verified = VerifiedHashes {
        context: context_hash,
        context_image,
        baseline: baseline_hash,
        working_image: hash,
    };
    // Admission attached no context value, so only a full read knows one.
    let opened = matches!(read, StackRead::StatesWithTrust)
        .then(|| opened_trust(&rows_read, &verified, &baseline, &working));
    Ok(Some(EditorHistory {
        generation,
        baseline,
        working,
        undo,
        redo,
        rows: rows_read,
        verified: Some(verified),
        opened,
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

/// The stored generation of a file only this writer committed to since its own
/// last commit, refused unless it is `expected`.
fn own_generation(conn: &Connection, expected: Option<i64>) -> Result<i64, HistoryError> {
    let current: i64 = conn
        .query_row(
            "SELECT generation FROM project_history_state WHERE id = 0",
            [],
            |r| r.get(0),
        )
        .optional()?
        .unwrap_or(0);
    if expected.is_some_and(|e| e != current) {
        return Err(HistoryError::Stale);
    }
    Ok(current)
}

/// The admitted generation, refused unless it is `expected`, and the retained
/// context the admission checked.
fn admitted_generation(
    conn: &Connection,
    expected: Option<i64>,
    trusted: &JournalTrust,
) -> Result<(i64, Option<ContextIdentity>), HistoryError> {
    let (current, context) = match admit_editor(conn, trusted)? {
        Some((generation, context)) => (generation, Some(context)),
        None => (0, None),
    };
    if expected.is_some_and(|e| e != current) {
        return Err(HistoryError::Stale);
    }
    Ok((current, context))
}

fn next_generation(current: i64) -> Result<i64, HistoryError> {
    current
        .checked_add(1)
        .filter(|next| *next <= 9_007_199_254_740_991)
        .ok_or(HistoryError::Invalid("generation exhausted"))
}

/// Image hashes of the stored undo and redo rows, bottom to top per side.
/// As a reuse request it names the bottom rows a caller proved unchanged.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StackRows {
    pub undo: Vec<String>,
    pub redo: Vec<String>,
}

impl StackRows {
    fn hashes(&self) -> HashSet<&str> {
        self.undo
            .iter()
            .chain(&self.redo)
            .map(String::as_str)
            .collect()
    }
}

/// Leading rows of `side` whose stored image hash still equals `expected`.
/// Hashes were verified against their images by the admission before this,
/// or, on the ADR-0166 own-commit path, written by this writer itself.
fn kept_rows(conn: &Connection, side: &str, expected: &[String]) -> Result<usize, HistoryError> {
    let mut stmt = conn.prepare(
        "SELECT image_hash FROM project_history_stack WHERE side = ?1 AND position = ?2 AND format_version = ?3",
    )?;
    let mut kept = 0;
    for (position, hash) in expected.iter().enumerate() {
        let stored: Option<String> = stmt
            .query_row(params![side, position as i64, FORMAT_VERSION], |r| r.get(0))
            .optional()?;
        if stored.as_deref() != Some(hash.as_str()) {
            break;
        }
        kept += 1;
    }
    Ok(kept)
}

/// What one workspace write may take from its writer's knowledge.
#[derive(Clone, Copy)]
struct Reuse<'a> {
    trust: &'a JournalTrust,
    /// Whether this transaction's admission already verified the retained
    /// context `trust` knows.
    context_verified: bool,
}

impl Reuse<'static> {
    const NOTHING: Self = Self {
        trust: &JournalTrust::EMPTY,
        context_verified: false,
    };
}

/// Stored working-image bytes whose state the writer knows.
struct KnownWorkingImage<'a> {
    project: &'a Project,
    image: Vec<u8>,
    hash: String,
}

/// The stored working image while it still holds the working model `trust`
/// knows: the next stack row is usually that state, so its bytes serve as is.
fn known_working_image<'a>(
    conn: &Connection,
    trust: &'a JournalTrust,
) -> Result<Option<KnownWorkingImage<'a>>, HistoryError> {
    let Some((project, hash)) = trust.working() else {
        return Ok(None);
    };
    let row: Option<(i64, Vec<u8>, String)> = conn
        .query_row(
            "SELECT format_version, working, working_hash FROM project_history_state WHERE id = 0",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    match row {
        Some((format, image, stored)) if format == FORMAT_VERSION && stored == *hash => {
            if image_hash(&image) != stored {
                return Err(HistoryError::Invalid("working image hash mismatch"));
            }
            Ok(Some(KnownWorkingImage {
                project,
                image,
                hash: stored,
            }))
        }
        _ => Ok(None),
    }
}

fn write_workspace(
    conn: &Connection,
    baseline: &NativeSnapshot,
    working: &NativeSnapshot,
    stack: &CommandStack,
    generation: i64,
    reuse: Reuse<'_>,
    report: &mut JournalWriteReport,
) -> Result<JournalTrust, HistoryError> {
    let (undo_steps, redo_steps) = stack.history_lengths();
    if undo_steps.saturating_add(redo_steps) > MAX_STACK_STATES {
        return Err(HistoryError::Limit("undo and redo exceed 256 states"));
    }
    report.undo_states = undo_steps;
    report.redo_states = redo_steps;
    let trust = reuse.trust;
    let started = Instant::now();
    let baseline_hash = match trust.known_semantic_hash(baseline) {
        Some(hash) => {
            report.baseline_hash_reused = true;
            hash
        }
        None => baseline.semantic_hash()?,
    };
    report.phase("baselineHash", started);
    let started = Instant::now();
    let (context, context_reused) =
        context::remember(conn, working, trust.context(), reuse.context_verified)?;
    report.context_reused = context_reused;
    report.context_bytes = conn
        .query_row(
            "SELECT length(image) FROM project_history_context WHERE content_hash = ?1",
            [&context.content_hash],
            |r| r.get::<_, i64>(0),
        )
        .map_or(0, |bytes| u64::try_from(bytes).unwrap_or(0));
    report.phase("context", started);
    let context_hash = context.content_hash.clone();
    let started = Instant::now();
    let previous_working = known_working_image(conn, trust)?;
    let image = context::model_image(&working.project)?;
    let working_image_hash = image_hash(&image);
    report.phase("workingImage", started);
    let started = Instant::now();
    let reuse = &trust.rows;
    // Kept rows hold exactly the states a full rewrite would produce; every
    // stack row pairs with the working context, as a full rewrite stores it.
    let keep_undo = kept_rows(conn, "undo", &reuse.undo)?.min(undo_steps);
    let keep_redo = kept_rows(conn, "redo", &reuse.redo)?.min(redo_steps);
    report.kept_rows = keep_undo + keep_redo;
    conn.execute(
        "DELETE FROM project_history_stack WHERE (side = 'undo' AND position >= ?1) OR (side = 'redo' AND position >= ?2)",
        params![keep_undo as i64, keep_redo as i64],
    )?;
    conn.execute(
        "UPDATE project_history_stack SET context_hash = ?1",
        [&context_hash],
    )?;
    let mut rows = StackRows {
        undo: reuse.undo[..keep_undo].to_vec(),
        redo: reuse.redo[..keep_redo].to_vec(),
    };
    rows.undo.resize(undo_steps, String::new());
    rows.redo.resize(redo_steps, String::new());
    conn.execute(
        "INSERT INTO project_history_state (id, format_version, generation, baseline_hash, working, working_hash, context_hash)
         VALUES (0, ?1, ?2, ?3, ?4, ?5, ?6) ON CONFLICT(id) DO UPDATE SET
         format_version=excluded.format_version, generation=excluded.generation, baseline_hash=excluded.baseline_hash,
         working=excluded.working, working_hash=excluded.working_hash, context_hash=excluded.context_hash",
        params![FORMAT_VERSION, generation, baseline_hash, image, working_image_hash, context_hash],
    )?;
    context::collect_unreferenced(conn)?;
    let mut bytes = stored_history_bytes(conn)?;
    if bytes > MAX_HISTORY_BYTES {
        return Err(HistoryError::Limit("total history exceeds 4 GiB"));
    }
    let mut stmt = conn.prepare("INSERT INTO project_history_stack (side, position, format_version, image, image_hash, context_hash) VALUES (?1, ?2, ?3, ?4, ?5, ?6)")?;
    let mut refusal = None;
    let floor = knx_core::command::SharedHistoryPrefix {
        undo: keep_undo,
        redo: keep_redo,
    };
    let complete = stack
        .visit_snapshot_states_above(&working.project, floor, |side, position, project| {
            let result = (|| -> Result<(), HistoryError> {
                if project.info.project_id != working.project.info.project_id {
                    return Err(HistoryError::Invalid("cross-project stack state"));
                }
                let side_rows = match side {
                    knx_core::command::HistorySide::Undo => &mut rows.undo,
                    knx_core::command::HistorySide::Redo => &mut rows.redo,
                };
                let (image, hash) = match &previous_working {
                    Some(known) if known.project == project => {
                        report.copied_rows += 1;
                        (known.image.clone(), known.hash.clone())
                    }
                    _ => {
                        report.encoded_rows += 1;
                        let image = context::model_image(project)?;
                        let hash = image_hash(&image);
                        (image, hash)
                    }
                };
                bytes = bytes
                    .checked_add(image.len() as i64)
                    .ok_or(HistoryError::Limit("history byte accounting overflow"))?;
                if bytes > MAX_HISTORY_BYTES {
                    return Err(HistoryError::Limit("total history exceeds 4 GiB"));
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
                    hash,
                    context_hash
                ])?;
                side_rows[position] = hash;
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
    report.phase("stackRows", started);
    let started = Instant::now();
    check_limits(conn)?;
    report.phase("limits", started);
    Ok(JournalTrust::knowing(
        rows,
        context,
        baseline,
        &baseline_hash,
        (working.project.clone(), working_image_hash),
    ))
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
    let context_hash = context::remember(conn, snapshot, None, false)?
        .0
        .content_hash;
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
    save_editor_inner(
        conn,
        working,
        stack,
        expected_generation,
        save_root,
        SaveExtras::default(),
    )
    .map(|(generation, _)| generation)
}

/// [`save_editor`] of the working journal alone that keeps the bottom stack
/// rows whose stored image hashes still equal `reuse`. The caller must have
/// proven those states unchanged (`CommandStack::shared_snapshot_prefix`);
/// a row whose hash differs, or any row above them, is written afresh.
/// Returns the new generation and the image hash of every stored row.
pub fn save_editor_reusing(
    conn: &Connection,
    working: &NativeSnapshot,
    stack: &CommandStack,
    expected_generation: i64,
    reuse: &StackRows,
) -> Result<(i64, StackRows), HistoryError> {
    save_editor_trusting(
        conn,
        working,
        stack,
        expected_generation,
        &JournalTrust::from_rows(reuse.clone()),
        &mut JournalWriteReport::default(),
    )
    .map(|(generation, trust)| (generation, trust.rows))
}

/// [`save_editor_reusing`] that also checks the retained context and saved
/// baseline `trust` knows by byte hash or value equality instead of decoding,
/// encoding or hashing them again (ADR-0165). Returns the new generation and
/// what the write verified, for the next one. `report` receives the phase
/// times and size facts, also of a write that fails.
pub fn save_editor_trusting(
    conn: &Connection,
    working: &NativeSnapshot,
    stack: &CommandStack,
    expected_generation: i64,
    trust: &JournalTrust,
    report: &mut JournalWriteReport,
) -> Result<(i64, JournalTrust), HistoryError> {
    save_editor_inner(
        conn,
        working,
        stack,
        Some(expected_generation),
        false,
        SaveExtras {
            reuse: Some(trust),
            report: Some(report),
            ..SaveExtras::default()
        },
    )
}

/// [`save_editor_trusting`] for a writer that proved no other connection
/// committed to the file since its own last commit at `expected_generation`
/// (ADR-0166 D: `PRAGMA data_version` unchanged on its kept connection). It
/// skips the admission and version listing that re-read what it wrote itself;
/// the stored generation is still compared, the retained context still checked
/// by the SHA-256 of its bytes and the limits still enforced.
pub fn save_editor_after_own_commit(
    conn: &Connection,
    working: &NativeSnapshot,
    stack: &CommandStack,
    expected_generation: i64,
    trust: &JournalTrust,
    report: &mut JournalWriteReport,
) -> Result<(i64, JournalTrust), HistoryError> {
    save_editor_inner(
        conn,
        working,
        stack,
        Some(expected_generation),
        false,
        SaveExtras {
            reuse: Some(trust),
            report: Some(report),
            own_commit: true,
            ..SaveExtras::default()
        },
    )
}

/// [`save_editor_trusting`] for a programming settlement: `edits` change the
/// file's parameter baselines in the same transaction as the settled flags,
/// so no crash can pair newer flags with an older baseline (ADR-0173 §7).
pub fn save_editor_settling(
    conn: &Connection,
    working: &NativeSnapshot,
    stack: &CommandStack,
    expected_generation: i64,
    trust: &JournalTrust,
    report: &mut JournalWriteReport,
    edits: &[BaselineEdit],
) -> Result<(i64, JournalTrust), HistoryError> {
    save_editor_inner(
        conn,
        working,
        stack,
        Some(expected_generation),
        false,
        SaveExtras {
            reuse: Some(trust),
            report: Some(report),
            baselines: BaselineWrite::Settle(edits),
            ..SaveExtras::default()
        },
    )
}

/// [`save_editor_with_sources`] whose file then holds exactly the baselines
/// of `baselines` that belong to devices of `working` (a native save).
pub fn save_editor_with_baselines(
    conn: &Connection,
    working: &NativeSnapshot,
    stack: &CommandStack,
    expected_generation: Option<i64>,
    sources: &[crate::CarriedManufacturerFile],
    baselines: &ParameterBaselines,
) -> Result<i64, HistoryError> {
    save_editor_inner(
        conn,
        working,
        stack,
        expected_generation,
        true,
        SaveExtras {
            sources,
            baselines: BaselineWrite::Replace(baselines),
            ..SaveExtras::default()
        },
    )
    .map(|(generation, _)| generation)
}

/// Changes only the file's parameter baselines, in one transaction that
/// first brings an older file to the current schema. For tests and
/// maintenance tools: the server never calls it, it writes every baseline
/// edit in the journal transaction of its settlement or mark (ADR-0173 §3/§7).
pub fn write_parameter_baselines(
    conn: &Connection,
    edits: &[BaselineEdit],
) -> Result<(), HistoryError> {
    let tx = Transaction::new_unchecked(conn, TransactionBehavior::Immediate)?;
    crate::migration::migrate_in_transaction(&tx)?;
    snapshot::check_schema(&tx)?;
    crate::parameter_baseline::apply_edits(&tx, edits)?;
    tx.commit()?;
    Ok(())
}

/// Every parameter baseline the file holds, read in one transaction.
pub fn load_parameter_baselines(conn: &Connection) -> Result<ParameterBaselines, HistoryError> {
    let tx = conn.unchecked_transaction()?;
    let baselines = crate::parameter_baseline::load_all(&tx)?;
    tx.commit()?;
    Ok(baselines)
}

/// Saves the explicit root and its newly acquired retained sources atomically.
/// Payloads are file-owned and never enter editor history images.
pub fn save_editor_with_sources(
    conn: &Connection,
    working: &NativeSnapshot,
    stack: &CommandStack,
    expected_generation: Option<i64>,
    sources: &[crate::CarriedManufacturerFile],
) -> Result<i64, HistoryError> {
    save_editor_inner(
        conn,
        working,
        stack,
        expected_generation,
        true,
        SaveExtras {
            sources,
            ..SaveExtras::default()
        },
    )
    .map(|(generation, _)| generation)
}

/// Compare the reviewed working snapshot under the same write lock as all writes.
/// Generation zero alone cannot detect a plain save that cleared the journal.
pub fn save_editor_if_unchanged(
    conn: &Connection,
    working: &NativeSnapshot,
    stack: &CommandStack,
    expected_generation: i64,
    expected_working_hash: &str,
    save_root: bool,
) -> Result<i64, HistoryError> {
    save_editor_inner(
        conn,
        working,
        stack,
        Some(expected_generation),
        save_root,
        SaveExtras {
            expected_working_hash: Some(expected_working_hash),
            ..SaveExtras::default()
        },
    )
    .map(|(generation, _)| generation)
}

/// What one editor write does with the file's parameter baselines
/// (ADR-0173 §7). They are file-level rows, never part of an image.
#[derive(Debug, Clone, Copy, Default)]
pub enum BaselineWrite<'a> {
    /// Leaves them, except that a root save drops the rows of devices the
    /// saved project lacks, all of them when the file held another project,
    /// and, for a writer not admitted at the journal's generation, every row
    /// whose device is not the same device (source, product, program) as in
    /// the project the rows were written for. Writes no row otherwise.
    #[default]
    Prune,
    /// A settlement's edits, in the transaction that persists its flags.
    Settle(&'a [BaselineEdit]),
    /// A native save of a server that holds the complete set: the file's rows
    /// become exactly the baselines of devices the saved project contains.
    Replace(&'a ParameterBaselines),
}

/// The optional inputs of one editor save; the default asks for none.
#[derive(Default)]
struct SaveExtras<'a> {
    expected_working_hash: Option<&'a str>,
    sources: &'a [crate::CarriedManufacturerFile],
    reuse: Option<&'a JournalTrust>,
    report: Option<&'a mut JournalWriteReport>,
    /// See [`save_editor_after_own_commit`].
    own_commit: bool,
    baselines: BaselineWrite<'a>,
}

fn save_editor_inner(
    conn: &Connection,
    working: &NativeSnapshot,
    stack: &CommandStack,
    expected_generation: Option<i64>,
    save_root: bool,
    extras: SaveExtras<'_>,
) -> Result<(i64, JournalTrust), HistoryError> {
    let SaveExtras {
        expected_working_hash,
        sources,
        reuse,
        report,
        own_commit,
        baselines,
    } = extras;
    let mut unreported = JournalWriteReport::default();
    let report = report.unwrap_or(&mut unreported);
    let (undo_steps, redo_steps) = stack.history_lengths();
    if undo_steps.saturating_add(redo_steps) > MAX_STACK_STATES {
        return Err(HistoryError::Limit("undo and redo exceed 256 states"));
    }
    // Each phase is recorded before its error propagates, so a busy lock or a
    // refusal still shows the step that cost the time.
    let started = Instant::now();
    let tx = Transaction::new_unchecked(conn, TransactionBehavior::Immediate);
    report.phase("lock", started);
    let tx = tx?;
    let started = Instant::now();
    let migrated = crate::migration::migrate_in_transaction(&tx).map_err(HistoryError::from);
    let checked = migrated.and_then(|()| snapshot::check_schema(&tx));
    report.phase("schema", started);
    checked?;
    // What this writer itself encoded or admitted is re-hashed, not decoded again.
    let untrusted = JournalTrust::default();
    let trusted = reuse.unwrap_or(&untrusted);
    let started = Instant::now();
    let admitted = if own_commit {
        own_generation(&tx, expected_generation).map(|current| (current, None))
    } else {
        admitted_generation(&tx, expected_generation, trusted)
    };
    report.phase("admission", started);
    let (current, admitted_context) = admitted?;
    let started = Instant::now();
    let listed = if own_commit {
        // Nothing but this writer committed since it listed them itself.
        tx.query_row("SELECT count(*) FROM project_history_version", [], |r| {
            r.get::<_, i64>(0)
        })
        .map_err(HistoryError::from)
        .map(|count| {
            (
                usize::try_from(count).unwrap_or(0),
                trusted.checked_contexts().clone(),
            )
        })
    } else {
        list_versions_checked(&tx, trusted, admitted_context.as_ref())
            .map(|(versions, checked)| (versions.len(), checked))
    };
    report.phase("versions", started);
    let (versions, checked_contexts) = listed?;
    report.saved_versions = versions;
    // The admission checked the working context; `remember` need not again.
    let context_verified = trusted.context().is_some_and(|known| {
        admitted_context.as_ref().is_some_and(|(content, image)| {
            known.content_hash == *content && known.image_hash == *image
        })
    });
    let generation = next_generation(current)?;
    let started = Instant::now();
    let old = match NativeSnapshot::read(&tx) {
        Ok(snapshot) => Some(snapshot),
        Err(HistoryError::Store(StoreError::NotSaved)) => None,
        Err(error) => return Err(error),
    };
    report.phase("rootRead", started);
    if let Some(expected) = expected_working_hash {
        let current = load_editor_inner(&tx)?
            .map(|editor| editor.working)
            .or_else(|| old.clone())
            .ok_or(HistoryError::Stale)?;
        if current.semantic_hash()? != expected {
            return Err(HistoryError::Stale);
        }
    }
    // The journal's working state an unadmitted root save replaces: the
    // project the file's parameter baselines were written for (ADR-0173 §2).
    let mut replaced_working = None;
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
                replaced_working = Some(previous.working);
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
    let reuse = Reuse {
        trust: trusted,
        context_verified,
    };
    let written = write_workspace(&tx, baseline, working, stack, generation, reuse, report)?
        .with_checked_contexts(checked_contexts);
    // After the journal rows: a baseline write that fails takes the settled
    // flags down with it in the same rollback (ADR-0173 §7).
    let started = Instant::now();
    let basis = if expected_generation.is_some() || own_commit {
        PruneBasis::Continued {
            previous_project_id: old.as_ref().map(|old| old.project.info.project_id.as_str()),
        }
    } else {
        PruneBasis::Replaced {
            described: replaced_working
                .as_ref()
                .or(old.as_ref())
                .map(|described| &described.project),
        }
    };
    match baselines {
        // Replacing already keeps only devices of the saved project.
        BaselineWrite::Replace(all) => {
            crate::parameter_baseline::replace_all(&tx, all, &working.project)?
        }
        BaselineWrite::Settle(edits) => crate::parameter_baseline::apply_edits(&tx, edits)?,
        BaselineWrite::Prune => {}
    }
    if save_root && !matches!(baselines, BaselineWrite::Replace(_)) {
        crate::parameter_baseline::prune(&tx, basis, &working.project)?;
    }
    // Only a write that touched them reports the phase.
    let touched = match baselines {
        BaselineWrite::Prune => save_root,
        BaselineWrite::Settle(edits) => save_root || !edits.is_empty(),
        BaselineWrite::Replace(_) => true,
    };
    if touched {
        report.phase("parameterBaselines", started);
    }
    for source in sources {
        let sha = crate::manufacturer_file_hash(&source.bytes);
        if !working.manufacturer_refs.iter().any(|r| {
            r.sha256 == sha
                && r.len == source.bytes.len() as i64
                && r.source_path == source.source_path
        }) {
            return Err(HistoryError::Invalid(
                "acquired source is absent from the saved manifest",
            ));
        }
        crate::store_manufacturer_file(&tx, &source.source_path, &source.bytes)
            .map_err(HistoryError::Carried)?;
        crate::load_manufacturer_file(&tx, &sha).map_err(HistoryError::Carried)?;
    }
    let started = Instant::now();
    let committed = tx.commit();
    report.phase("commit", started);
    committed?;
    Ok((generation, written))
}

fn list_versions_inner(conn: &Connection) -> Result<Vec<ProjectVersion>, HistoryError> {
    list_versions_checked(conn, &JournalTrust::EMPTY, None).map(|(versions, _)| versions)
}

/// [`list_versions_inner`] that checks a retained context `trusted` knows by
/// the SHA-256 of its bytes instead of decoding it, and skips the one this
/// transaction already checked (`checked`). Also answers every context it
/// checked, by semantic identity, with the SHA-256 recorded for its bytes.
fn list_versions_checked(
    conn: &Connection,
    trusted: &JournalTrust,
    checked: Option<&ContextIdentity>,
) -> Result<
    (
        Vec<ProjectVersion>,
        std::collections::BTreeMap<String, String>,
    ),
    HistoryError,
> {
    check_limits(conn)?;
    let mut checked_contexts = std::collections::BTreeMap::new();
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
            let (bytes, stored_image): (i64, String) = conn
                .query_row(
                    "SELECT length(image), image_hash FROM project_history_context WHERE content_hash = ?1",
                    [&context_hash],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()?
                .ok_or(HistoryError::Invalid("referenced retained context is missing"))?;
            let identity = (context_hash.clone(), stored_image);
            let already = checked == Some(&identity)
                || (trusted.knows_context(&identity.0, &identity.1)
                    && context::verify_stored(conn, &identity.0, &identity.1)?);
            if !already {
                context::read(conn, &context_hash)?;
            }
            checked_contexts.insert(identity.0, identity.1);
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
    Ok((result, checked_contexts))
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
    /// After a restore: per device, the parts the restored version claimed
    /// programmed that are pending against the live project (ADR-0173 §6).
    pub restored_pending: BTreeMap<DeviceId, ProgrammingParts>,
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
    let generation =
        next_generation(admitted_generation(&tx, Some(expected), &JournalTrust::default())?.0)?;
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
            Reuse::NOTHING,
            &mut JournalWriteReport::default(),
        )?;
    }
    let mut restored_pending = BTreeMap::new();
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
                Reuse::NOTHING,
                &mut JournalWriteReport::default(),
            )?;
            None
        }
        HistoryAction::Restore(id) => {
            let mut replacement = read_version_inner(&tx, id)?;
            replacement
                .project
                .ids
                .raise_to(&current_working.project.ids);
            // A version's programming flags describe the devices when it was
            // stored; settle them against the live project before it lands.
            restored_pending =
                settle_restored_version(&current_working.project, &mut replacement.project);
            // Baselines stay outside the version, but an id in it may name
            // another device than the live one (a re-import under the same
            // project id): keep only rows of the same devices (ADR-0173 §2).
            crate::parameter_baseline::prune(
                &tx,
                crate::parameter_baseline::PruneBasis::Replaced {
                    described: Some(&current_working.project),
                },
                &replacement.project,
            )?;
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
                Reuse::NOTHING,
                &mut JournalWriteReport::default(),
            )?;
            Some(EditorHistory {
                generation,
                baseline: replacement.clone(),
                working: replacement,
                undo: Vec::new(),
                redo: Vec::new(),
                rows: StackRows::default(),
                verified: None,
                opened: None,
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
        restored_pending,
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
    // A plain save brings no baselines and may hold a freshly imported
    // project under the same project id (`knx import --replace`): a row stays
    // only where the saved device is the one it was written for, judged
    // against the journal's working state, else the root (ADR-0173 §2).
    let described = previous.as_ref().map_or(&old, |previous| &previous.working);
    crate::parameter_baseline::prune(
        tx,
        PruneBasis::Replaced {
            described: Some(&described.project),
        },
        &replacement.project,
    )?;
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
            Reuse::NOTHING,
            &mut JournalWriteReport::default(),
        )?;
    }
    Ok(())
}
