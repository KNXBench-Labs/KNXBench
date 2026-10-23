//! Owned, fixed D/B inventory with retained lifecycle connections.
//!
//! Failed lifecycle transitions never own the only copy of a connection:
//! the inventory retains it, including failed native close.

use std::fs::{File, OpenOptions};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::Path;
mod publication;
pub use publication::{PublishedRecovery, RecoveryPublicationError, ValidatedRecovery};
use std::path::PathBuf;

use rusqlite::{Connection, DropBehavior, OpenFlags, Transaction};

use super::{copy_pinned_current, is_supported_page_size, scalar, CopyError, COPY_PAGE_BYTE_BOUND};
use crate::catalogue_schema::{copy_content_witness, CopyContentWitness, WitnessError};
use crate::operation::{PublicationGate, Stop};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DestinationRole {
    Rehearsal,
    Recovery,
}
impl DestinationRole {
    fn index(self) -> usize {
        match self {
            Self::Rehearsal => 0,
            Self::Recovery => 1,
        }
    }
}

// Non-clone, owner-bound receipts; only the corresponding transition mints one.
#[derive(Debug)]
pub(crate) struct FreshDestination {
    role: DestinationRole,
    id: [u8; 16],
}
#[derive(Debug)]
pub(crate) struct ClosedDestination {
    role: DestinationRole,
    id: [u8; 16],
}
#[derive(Debug)]
pub(crate) struct ValidatedDestination {
    role: DestinationRole,
    id: [u8; 16],
}
#[derive(Debug)]
pub(crate) struct RehearsalDestination {
    id: [u8; 16],
}

#[derive(Debug)]
pub(crate) enum DestinationError {
    Occupied,
    InvalidReceipt,
    ContentMismatch,
    Identity,
    ContentChanged,
    Growth(crate::catalogue_growth::GrowthError),
    HeaderFacts,
    Copy(Box<CopyError>),
    Sqlite(rusqlite::Error),
    Io(std::io::Error),
    Witness(WitnessError),
    Stop(Stop),
    Publication(RecoveryPublicationError),
}
#[derive(Debug)]
pub(crate) enum DestinationCleanup {
    WriterClose(rusqlite::Error),
    ReadEnd(rusqlite::Error),
    ReadNotAutocommit,
    ReadClose(rusqlite::Error),
    RehearsalClose(rusqlite::Error),
}
#[derive(Debug)]
pub(crate) struct DestinationFailure {
    pub(crate) primary: Option<DestinationError>,
    pub(crate) cleanup: Vec<DestinationCleanup>,
}
impl From<DestinationError> for CopyError {
    fn from(primary: DestinationError) -> Self {
        Self::Destination(Box::new(DestinationFailure {
            primary: Some(primary),
            cleanup: Vec::new(),
        }))
    }
}
impl From<rusqlite::Error> for DestinationError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Sqlite(e)
    }
}
impl From<std::io::Error> for DestinationError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
impl From<WitnessError> for DestinationError {
    fn from(e: WitnessError) -> Self {
        Self::Witness(e)
    }
}
impl From<Stop> for DestinationError {
    fn from(e: Stop) -> Self {
        Self::Stop(e)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Incomplete,
    Fresh,
    Closed,
    Validated,
    Rehearsal,
    Published,
    PublicationUncertain,
}
#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DestinationStage {
    Allocated,
    WriterClose,
    ReadOpen,
    ReadOpened,
    ReadEnd,
    WriteOpen,
    WriteOpened,
    WriteReady,
    RehearsalClose,
}
#[cfg(test)]
type Hook = Box<dyn FnMut(DestinationStage, &Path, Option<&Connection>)>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ContentStamp {
    len: u64,
    mtime: (i64, i64),
    ctime: (i64, i64),
}
impl ContentStamp {
    fn of(meta: &std::fs::Metadata) -> Self {
        Self {
            len: meta.len(),
            mtime: (meta.mtime(), meta.mtime_nsec()),
            ctime: (meta.ctime(), meta.ctime_nsec()),
        }
    }
}
/// Header and checked physical extent admitted from the reserved source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CopyFacts {
    application_id: i64,
    user_version: i64,
    page_size: i64,
    page_count: i64,
}
impl CopyFacts {
    fn extent(&self) -> Option<u64> {
        u64::try_from(self.page_count)
            .ok()?
            .checked_mul(u64::try_from(self.page_size).ok()?)
    }
    pub(crate) fn read(conn: &Connection) -> Result<Self, CopyError> {
        let result = (|| -> Result<Self, DestinationError> {
            let facts = Self {
                application_id: scalar(conn, "PRAGMA main.application_id")?,
                user_version: scalar(conn, "PRAGMA main.user_version")?,
                page_size: scalar(conn, "PRAGMA main.page_size")?,
                page_count: scalar(conn, "PRAGMA main.page_count")?,
            };
            let encoding: String = conn.query_row("PRAGMA main.encoding", [], |r| r.get(0))?;
            if encoding != "UTF-8"
                || !is_supported_page_size(facts.page_size)
                || facts.page_count <= 0
                || facts.extent().is_none_or(|n| n > COPY_PAGE_BYTE_BOUND)
            {
                return Err(DestinationError::HeaderFacts);
            }
            Ok(facts)
        })();
        result.map_err(CopyError::from)
    }
}
struct Destination {
    id: [u8; 16],
    path: PathBuf,
    anchor: File,
    parent_identity: Option<(u64, u64)>,
    frozen: Option<ContentStamp>,
    connection: Option<Connection>,
    witness: Option<CopyContentWitness>,
    configuration: Option<(i64, i64)>,
    facts: Option<CopyFacts>,
    state: State,
    #[cfg(test)]
    hook: Option<Hook>,
}

/// Allocation survives initialization/validation failure; no Drop removes files.
#[derive(Default)]
pub(crate) struct ArtifactInventory {
    slots: [Option<Destination>; 2],
    #[cfg(test)]
    hooks: [Option<Hook>; 2],
}
impl Destination {
    fn check_frozen(&self) -> Result<(), DestinationError> {
        self.check_identity()?;
        if self.frozen != Some(ContentStamp::of(&self.anchor.metadata()?)) {
            return Err(DestinationError::ContentChanged);
        }
        Ok(())
    }
    fn check_identity(&self) -> Result<(), DestinationError> {
        let main = std::fs::symlink_metadata(&self.path)?;
        let anchor = self.anchor.metadata()?;
        let parent =
            std::fs::symlink_metadata(self.path.parent().ok_or(DestinationError::Identity)?)?;
        if !main.is_file()
            || main.file_type().is_symlink()
            || !anchor.is_file()
            || main.nlink() != 1
            || anchor.nlink() != 1
            || (main.dev(), main.ino()) != (anchor.dev(), anchor.ino())
            || !parent.is_dir()
            || parent.file_type().is_symlink()
            || self.parent_identity != Some((parent.dev(), parent.ino()))
        {
            return Err(DestinationError::Identity);
        }
        Ok(())
    }
    #[cfg(test)]
    fn seam(&mut self, stage: DestinationStage) {
        if let Some(mut hook) = self.hook.take() {
            hook(stage, &self.path, self.connection.as_ref());
            self.hook = Some(hook);
        }
    }
}
impl ArtifactInventory {
    #[cfg(test)]
    pub(crate) fn set_hook(&mut self, role: DestinationRole, hook: Hook) {
        self.hooks[role.index()] = Some(hook);
    }
    #[cfg(test)]
    pub(crate) fn retained_connection(&self, role: DestinationRole) -> Option<&Connection> {
        self.slots[role.index()].as_ref()?.connection.as_ref()
    }

    pub(crate) fn allocated(&self, role: DestinationRole) -> bool {
        self.slots[role.index()].is_some()
    }

    pub(crate) fn create_new(
        &mut self,
        role: DestinationRole,
        path: PathBuf,
        page_size: i64,
    ) -> Result<FreshDestination, CopyError> {
        self.create_new_observed(role, path, page_size, &mut |_, _| Ok(()))
    }

    pub(crate) fn create_new_observed(
        &mut self,
        role: DestinationRole,
        path: PathBuf,
        page_size: i64,
        observer: &mut dyn FnMut(&Path, &File) -> Result<(), RecoveryPublicationError>,
    ) -> Result<FreshDestination, CopyError> {
        if self.allocated(role) {
            return Err(DestinationError::Occupied.into());
        }
        let mut id = [0; 16];
        getrandom::fill(&mut id).expect("destination receipt randomness");
        let anchor = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)
            .map_err(DestinationError::from)?;
        // Register immediately, before metadata, SQLite open or configuration.
        self.slots[role.index()] = Some(Destination {
            id,
            path,
            anchor,
            parent_identity: None,
            frozen: None,
            connection: None,
            witness: None,
            configuration: None,
            facts: None,
            state: State::Incomplete,
            #[cfg(test)]
            hook: self.hooks[role.index()].take(),
        });
        let slot = self.slots[role.index()].as_mut().unwrap();
        #[cfg(test)]
        slot.seam(DestinationStage::Allocated);
        let parent =
            std::fs::symlink_metadata(slot.path.parent().ok_or(DestinationError::Identity)?)
                .map_err(DestinationError::from)?;
        slot.parent_identity = Some((parent.dev(), parent.ino()));
        slot.check_identity()?;
        observer(&slot.path, &slot.anchor).map_err(DestinationError::Publication)?;
        slot.connection = Some(
            Connection::open_with_flags(&slot.path, rw_flags()).map_err(DestinationError::from)?,
        );
        slot.check_identity()?;
        slot.connection
            .as_ref()
            .unwrap()
            .pragma_update(None, "page_size", page_size)
            .map_err(DestinationError::from)?;
        slot.state = State::Fresh;
        Ok(FreshDestination { role, id })
    }

    fn slot(
        &mut self,
        role: DestinationRole,
        id: [u8; 16],
        state: State,
    ) -> Result<&mut Destination, CopyError> {
        self.slots[role.index()]
            .as_mut()
            .filter(|s| s.id == id && s.state == state)
            .ok_or_else(|| DestinationError::InvalidReceipt.into())
    }
    #[cfg(test)]
    pub(crate) fn closed_path(&self, closed: &ClosedDestination) -> &Path {
        &self.slots[closed.role.index()].as_ref().unwrap().path
    }
    pub(crate) fn copy_and_close(
        &mut self,
        fresh: FreshDestination,
        expected_role: DestinationRole,
        source: &Transaction<'_>,
        gate: &PublicationGate,
    ) -> Result<ClosedDestination, CopyError> {
        gate.checkpoint()?;
        if fresh.role != expected_role {
            return Err(DestinationError::InvalidReceipt.into());
        }
        let slot = self.slot(fresh.role, fresh.id, State::Fresh)?;
        slot.state = State::Incomplete;
        slot.check_identity()?;
        slot.facts = Some(CopyFacts::read(source)?);
        slot.witness = Some(copy_content_witness(source, gate).map_err(DestinationError::from)?);
        let writer = slot.connection.as_ref().unwrap();
        slot.configuration = Some((
            scalar(writer, "PRAGMA trusted_schema").map_err(DestinationError::from)?,
            scalar(writer, "PRAGMA recursive_triggers").map_err(DestinationError::from)?,
        ));
        let copy = copy_pinned_current(source, slot.connection.as_mut().unwrap(), gate);
        #[cfg(test)]
        slot.seam(DestinationStage::WriterClose);
        let mut primary = copy.err().map(|e| DestinationError::Copy(Box::new(e)));
        if let Err(stop) = gate.checkpoint() {
            primary.get_or_insert(DestinationError::Stop(stop));
        }
        if let Err(error) = slot.check_identity() {
            primary.get_or_insert(error);
        }
        let mut cleanup = Vec::new();
        if let Err((conn, error)) = slot.connection.take().unwrap().close() {
            slot.connection = Some(conn);
            cleanup.push(DestinationCleanup::WriterClose(error));
        }
        if primary.is_some() || !cleanup.is_empty() {
            return Err(CopyError::Destination(Box::new(DestinationFailure {
                primary,
                cleanup,
            })));
        }
        slot.check_identity()?;
        slot.frozen = Some(ContentStamp::of(
            &slot.anchor.metadata().map_err(DestinationError::from)?,
        ));
        if slot.facts.as_ref().and_then(CopyFacts::extent) != slot.frozen.map(|s| s.len) {
            return Err(DestinationError::HeaderFacts.into());
        }
        slot.state = State::Closed;
        Ok(ClosedDestination {
            role: fresh.role,
            id: fresh.id,
        })
    }
    pub(crate) fn validate(
        &mut self,
        closed: ClosedDestination,
        gate: &PublicationGate,
    ) -> Result<ValidatedDestination, CopyError> {
        let slot = self.slot(closed.role, closed.id, State::Closed)?;
        slot.state = State::Incomplete;
        #[cfg(test)]
        slot.seam(DestinationStage::ReadOpen);
        slot.check_frozen()?;
        slot.connection = Some(
            Connection::open_with_flags(&slot.path, ro_flags()).map_err(DestinationError::from)?,
        );
        #[cfg(test)]
        slot.seam(DestinationStage::ReadOpened);
        let conn = slot.connection.as_ref().unwrap();
        let result = (|| -> Result<(), DestinationError> {
            gate.checkpoint()?;
            slot.check_frozen()?;
            if !conn.is_readonly("main")? {
                return Err(DestinationError::InvalidReceipt);
            }
            conn.pragma_update(None, "query_only", true)?;
            let mut tx = conn.unchecked_transaction()?;
            tx.set_drop_behavior(DropBehavior::Ignore);
            let _: i64 = scalar(&tx, "SELECT count(*) FROM main.sqlite_schema")?;
            if Some(CopyFacts::read(&tx).map_err(|error| DestinationError::Copy(Box::new(error)))?)
                != slot.facts
            {
                return Err(DestinationError::HeaderFacts);
            }
            if scalar(&tx, "PRAGMA schema_version")? != 1 {
                return Err(DestinationError::ContentMismatch);
            }
            if Some(copy_content_witness(&tx, gate)?) != slot.witness {
                return Err(DestinationError::ContentMismatch);
            }
            Ok(())
        })();
        #[cfg(test)]
        slot.seam(DestinationStage::ReadEnd);
        let result = result.and_then(|()| gate.checkpoint().map_err(DestinationError::from));
        let conn = slot.connection.as_ref().unwrap();
        let mut cleanup = Vec::new();
        if !conn.is_autocommit() {
            if let Err(error) = conn.execute_batch("ROLLBACK") {
                cleanup.push(DestinationCleanup::ReadEnd(error));
            }
            if !conn.is_autocommit() {
                cleanup.push(DestinationCleanup::ReadNotAutocommit);
            }
        }
        if let Err((conn, error)) = slot.connection.take().unwrap().close() {
            slot.connection = Some(conn);
            cleanup.push(DestinationCleanup::ReadClose(error));
        }
        if result.is_err() || !cleanup.is_empty() {
            return Err(CopyError::Destination(Box::new(DestinationFailure {
                primary: result.err(),
                cleanup,
            })));
        }
        gate.checkpoint().map_err(DestinationError::from)?;
        slot.check_frozen()?;
        slot.state = State::Validated;
        Ok(ValidatedDestination {
            role: closed.role,
            id: closed.id,
        })
    }
    pub(crate) fn open_rehearsal(
        &mut self,
        validated: ValidatedDestination,
        gate: &PublicationGate,
    ) -> Result<RehearsalDestination, CopyError> {
        if validated.role != DestinationRole::Rehearsal {
            return Err(DestinationError::InvalidReceipt.into());
        }
        let slot = self.slot(validated.role, validated.id, State::Validated)?;
        slot.state = State::Incomplete;
        gate.checkpoint().map_err(DestinationError::from)?;
        #[cfg(test)]
        slot.seam(DestinationStage::WriteOpen);
        slot.check_frozen()?;
        slot.connection = Some(
            Connection::open_with_flags(&slot.path, rw_flags()).map_err(DestinationError::from)?,
        );
        #[cfg(test)]
        slot.seam(DestinationStage::WriteOpened);
        slot.check_frozen()?;
        let conn = slot.connection.as_ref().unwrap();
        if conn.is_readonly("main").map_err(DestinationError::from)? {
            return Err(DestinationError::InvalidReceipt.into());
        }
        if Some(CopyFacts::read(conn)?) != slot.facts
            || scalar(conn, "PRAGMA schema_version").map_err(DestinationError::from)? != 1
        {
            return Err(DestinationError::HeaderFacts.into());
        }
        let (trusted, recursive) = slot.configuration.ok_or(DestinationError::InvalidReceipt)?;
        for (pragma, value) in [
            ("foreign_keys", 1),
            ("query_only", 0),
            ("ignore_check_constraints", 0),
            ("writable_schema", 0),
            ("defer_foreign_keys", 0),
            ("trusted_schema", trusted),
            ("recursive_triggers", recursive),
        ] {
            conn.pragma_update(None, pragma, value)
                .map_err(DestinationError::from)?;
            if scalar(conn, &format!("PRAGMA {pragma}")).map_err(DestinationError::from)? != value {
                return Err(DestinationError::InvalidReceipt.into());
            }
        }
        crate::catalogue_growth::enforce(conn).map_err(DestinationError::Growth)?;
        #[cfg(test)]
        slot.seam(DestinationStage::WriteReady);
        gate.checkpoint().map_err(DestinationError::from)?;
        slot.check_frozen()?;
        slot.state = State::Rehearsal;
        Ok(RehearsalDestination { id: validated.id })
    }
    pub(crate) fn rehearsal(
        &mut self,
        receipt: &RehearsalDestination,
    ) -> Result<&Connection, CopyError> {
        let slot = self.slot(DestinationRole::Rehearsal, receipt.id, State::Rehearsal)?;
        slot.check_frozen()?;
        Ok(slot.connection.as_ref().unwrap())
    }
    pub(crate) fn close_rehearsal(
        &mut self,
        receipt: RehearsalDestination,
    ) -> Result<(), CopyError> {
        let slot = self.slot(DestinationRole::Rehearsal, receipt.id, State::Rehearsal)?;
        #[cfg(test)]
        slot.seam(DestinationStage::RehearsalClose);
        slot.state = State::Incomplete;
        if let Err((conn, error)) = slot.connection.take().unwrap().close() {
            slot.connection = Some(conn);
            return Err(CopyError::Destination(Box::new(DestinationFailure {
                primary: None,
                cleanup: vec![DestinationCleanup::RehearsalClose(error)],
            })));
        }
        Ok(())
    }
    #[cfg(test)]
    pub(crate) fn test_writer(&mut self, receipt: &FreshDestination) -> &mut Connection {
        self.slot(receipt.role, receipt.id, State::Fresh)
            .unwrap()
            .connection
            .as_mut()
            .unwrap()
    }
}
fn rw_flags() -> OpenFlags {
    OpenFlags::SQLITE_OPEN_READ_WRITE
        | OpenFlags::SQLITE_OPEN_PRIVATE_CACHE
        | OpenFlags::SQLITE_OPEN_NO_MUTEX
}
fn ro_flags() -> OpenFlags {
    OpenFlags::SQLITE_OPEN_READ_ONLY
        | OpenFlags::SQLITE_OPEN_PRIVATE_CACHE
        | OpenFlags::SQLITE_OPEN_NO_MUTEX
}
