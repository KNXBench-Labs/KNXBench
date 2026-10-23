//! Server-side owner of the managed product catalogue for the HTTP routes.
//!
//! One mutex around the ProductDB [`ManagedCatalogue`]; a poisoned mutex is a
//! typed refusal, never an `expect`-panic. Construction is the only path into
//! a mutation; no production Ready/apply route is wired here.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use knx_productdb::managed_catalogue::{
    LegacyMutationKind, LegacySettlementHint, ManagedCatalogue, OwnerError, OwnerRestoreError,
    QuarantineReason, WriterHealth,
};
use knx_productdb::Connection;

use crate::errors::ApiError;

/// Maps an owner access failure to a server error. A quarantined or poisoned
/// owner means the catalogue is unusable for this request, not merely absent.
pub(crate) fn owner_error(error: OwnerError) -> ApiError {
    ApiError::internal(format!("catalogue owner unavailable: {error}"))
}

pub(crate) fn restore_error(error: OwnerRestoreError) -> ApiError {
    match error {
        OwnerRestoreError::InvalidBackup(why) => ApiError::refused(
            "invalidBackup",
            format!("not a usable product database backup: {why}"),
        ),
        other => ApiError::internal(format!("catalogue owner restore unavailable: {other}")),
    }
}

/// The server's protected managed catalogue: one owner, one lock.
pub struct CatalogueOwner {
    state: Mutex<ManagedCatalogue>,
    #[cfg(test)]
    panic_after_handoff: std::sync::atomic::AtomicBool,
}

#[derive(Debug)]
pub(crate) enum ControlledOwnerError {
    Busy,
    Stopped,
    Owner(OwnerError),
    Preview(knx_productdb::managed_apply::PreviewError),
    Bridge(knx_productdb::managed_apply::ApplyBridgeError),
}
impl std::fmt::Display for ControlledOwnerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Busy => f.write_str("catalogue writer busy"),
            Self::Stopped => f.write_str("catalogue operation stopped"),
            Self::Owner(error) => write!(f, "catalogue owner unavailable: {error}"),
            Self::Preview(error) => write!(f, "catalogue preview refused: {error}"),
            Self::Bridge(error) => write!(f, "catalogue receipt handoff refused: {error:?}"),
        }
    }
}

/// A poisoned owner mutex is reported as a quarantined, unusable owner.
fn poisoned() -> OwnerError {
    OwnerError::NotHealthy(WriterHealth::Quarantined(QuarantineReason::Poisoned))
}

fn poisoned_restore() -> OwnerRestoreError {
    OwnerRestoreError::NotHealthy(WriterHealth::Quarantined(QuarantineReason::Poisoned))
}

impl CatalogueOwner {
    /// Adopts an already-opened, already-migrated connection as the managed
    /// owner. Startup passes its completed `open_and_migrate` result here.
    pub fn adopt_opened(conn: Connection, configured_path: PathBuf) -> Result<Self, OwnerError> {
        let managed = ManagedCatalogue::adopt_opened(conn, configured_path)?;
        Ok(Self {
            state: Mutex::new(managed),
            #[cfg(test)]
            panic_after_handoff: std::sync::atomic::AtomicBool::new(false),
        })
    }

    pub(crate) fn try_prepare_controlled(
        &self,
        source: knx_productdb::managed_apply::AdmittedSource<'_>,
        gate: &knx_productdb::operation::PublicationGate,
        workspace: &mut knx_productdb::managed_apply::ApplyWorkspace,
    ) -> Result<knx_productdb::managed_apply::PreparedPreview, ControlledOwnerError> {
        gate.checkpoint()
            .map_err(|_| ControlledOwnerError::Stopped)?;
        let mut managed = match self.state.try_lock() {
            Ok(managed) => managed,
            Err(std::sync::TryLockError::WouldBlock) => return Err(ControlledOwnerError::Busy),
            Err(std::sync::TryLockError::Poisoned(_)) => {
                return Err(ControlledOwnerError::Owner(poisoned()));
            }
        };
        managed
            .prepare_controlled(source, gate, workspace)
            .map_err(ControlledOwnerError::Preview)
    }

    pub(crate) fn try_prepare_controlled_owned(
        &self,
        source: knx_productdb::managed_apply::AdmittedSource<'_>,
        gate: &knx_productdb::operation::PublicationGate,
        workspace: &mut knx_productdb::managed_apply::ApplyWorkspace,
        publisher: &mut dyn knx_productdb::managed_apply::RecoveryPublisher,
    ) -> Result<knx_productdb::managed_apply::PreparedPreview, ControlledOwnerError> {
        gate.checkpoint()
            .map_err(|_| ControlledOwnerError::Stopped)?;
        let mut managed = match self.state.try_lock() {
            Ok(m) => m,
            Err(std::sync::TryLockError::WouldBlock) => return Err(ControlledOwnerError::Busy),
            Err(std::sync::TryLockError::Poisoned(_)) => {
                return Err(ControlledOwnerError::Owner(poisoned()))
            }
        };
        managed
            .prepare_controlled_owned(source, gate, workspace, publisher)
            .map_err(ControlledOwnerError::Preview)
    }

    pub(crate) fn try_apply_controlled(
        &self,
        source: knx_productdb::managed_apply::AdmittedSource<'_>,
        shown: &knx_productdb::managed_apply::PreparedPreview,
        control: &dyn knx_productdb::operation::CommitArbitration,
        workspace: &mut knx_productdb::managed_apply::ApplyWorkspace,
        receipt: &mut knx_productdb::managed_apply::ApplyReceipt,
        publisher: &mut dyn knx_productdb::managed_apply::RecoveryPublisher,
    ) -> Result<knx_productdb::managed_apply::ApplyExecution, ControlledOwnerError> {
        control
            .gate()
            .checkpoint()
            .map_err(|_| ControlledOwnerError::Stopped)?;
        let mut managed = match self.state.try_lock() {
            Ok(managed) => managed,
            Err(std::sync::TryLockError::WouldBlock) => return Err(ControlledOwnerError::Busy),
            Err(std::sync::TryLockError::Poisoned(_)) => {
                return Err(ControlledOwnerError::Owner(poisoned()));
            }
        };
        let execution = managed
            .execute_controlled(source, shown, control, workspace, receipt, publisher)
            .map_err(ControlledOwnerError::Bridge)?;
        #[cfg(test)]
        if self
            .panic_after_handoff
            .load(std::sync::atomic::Ordering::SeqCst)
        {
            panic!("server-post-handoff-panic");
        }
        Ok(execution)
    }

    #[cfg(test)]
    pub(crate) fn panic_after_handoff_for_test(&self) {
        self.panic_after_handoff
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    /// A read lease: the `&Connection` never escapes the closure. A poisoned
    /// mutex or an unhealthy owner is a refusal.
    pub fn with_read<T, E>(
        &self,
        read: impl FnOnce(&Connection) -> Result<T, E>,
    ) -> Result<Result<T, E>, OwnerError> {
        let guard = self.state.lock().map_err(|_| poisoned())?;
        guard.with_read(read)
    }

    /// A non-blocking read lease for status projections: `Ok(None)` means the
    /// owner was busy, `Ok(Some(inner))` carries the read result, and `Err`
    /// means the owner is poisoned. Never blocks on the lock.
    pub(crate) fn try_read<T, E>(
        &self,
        read: impl FnOnce(&Connection) -> Result<T, E>,
    ) -> Result<Option<Result<T, E>>, OwnerError> {
        match self.state.try_lock() {
            Ok(guard) => guard.with_read(read).map(Some),
            Err(std::sync::TryLockError::WouldBlock) => Ok(None),
            Err(std::sync::TryLockError::Poisoned(_)) => Err(poisoned()),
        }
    }

    /// A managed mutation guard around one legacy mutator, delegated to the
    /// ProductDB owner. Generation increment, quarantine-on-unwind and the
    /// settlement hint classification stay inside `ManagedCatalogue`.
    pub(crate) fn with_legacy_mutation<T, E>(
        &self,
        kind: LegacyMutationKind,
        work: impl FnOnce(&mut Connection) -> Result<T, E>,
        classify: impl FnOnce(&Result<T, E>) -> LegacySettlementHint,
    ) -> Result<Result<T, E>, OwnerError> {
        let mut guard = self.state.lock().map_err(|_| poisoned())?;
        guard.with_legacy_mutation(kind, work, classify)
    }

    /// The dedicated restore adapter: retire and invalidate before reopen,
    /// then adopt a fresh incarnation only on success.
    pub(crate) fn restore_legacy(
        &self,
        live_path: &Path,
        backup_path: &Path,
    ) -> Result<(), OwnerRestoreError> {
        let mut guard = self.state.lock().map_err(|_| poisoned_restore())?;
        guard.restore_legacy(live_path, backup_path)
    }
}
