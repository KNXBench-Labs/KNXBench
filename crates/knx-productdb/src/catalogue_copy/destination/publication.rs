//! Opaque OS-acknowledged publication of the actual closed validated B.
use super::*;
use rustix::fs::{openat, renameat_with, Mode, OFlags, RenameFlags, CWD};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryPublicationError {
    Refused,
    AlreadyExists,
    Io,
}

/// Not client-constructible. This borrows the exact native destination owner;
/// no connection or arbitrary SQL callback crosses the facade.
pub struct ValidatedRecovery<'a> {
    destination: &'a mut Destination,
}

/// Minted ONLY after file and both directory sync acknowledgements. The opaque
/// destination id is consumed/checked by orchestration before its first W DML.
#[derive(Debug)]
pub struct PublishedRecovery {
    id: [u8; 16],
}

impl ValidatedRecovery<'_> {
    pub fn path(&self) -> &std::path::Path {
        &self.destination.path
    }
    pub fn descriptor(&self) -> &File {
        &self.destination.anchor
    }
    pub fn moved(&self) -> bool {
        matches!(
            self.destination.state,
            State::Published | State::PublicationUncertain
        )
    }

    /// No replacement and no copy fallback. A failure after rename retains the
    /// actual moved pathname/anchor in the original inventory, not a lost file.
    pub fn publish_to(
        &mut self,
        parent: &std::path::Path,
        name: &str,
    ) -> Result<PublishedRecovery, RecoveryPublicationError> {
        use RecoveryPublicationError as E;
        let slot = &mut self.destination;
        if slot.state != State::Validated
            || slot.connection.is_some()
            || name.is_empty()
            || name.len() > 512
            || name == "."
            || name == ".."
            || name.contains(['/', '\\', ':'])
            || name.chars().any(char::is_control)
        {
            return Err(E::Refused);
        }
        slot.check_frozen().map_err(|_| E::Refused)?;
        for suffix in ["-journal", "-wal", "-shm"] {
            let mut native = slot.path.as_os_str().to_os_string();
            native.push(suffix);
            let path = std::path::PathBuf::from(native);
            match std::fs::symlink_metadata(path) {
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                _ => return Err(E::Refused),
            }
        }
        let open_dir = |path: &std::path::Path| -> Result<File, E> {
            openat(
                CWD,
                path,
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map(File::from)
            .map_err(|_| E::Io)
        };
        let source_parent = slot.path.parent().ok_or(E::Refused)?;
        let source = open_dir(source_parent)?;
        let target = open_dir(parent)?;
        let sm = source.metadata().map_err(|_| E::Io)?;
        if slot.parent_identity != Some((sm.dev(), sm.ino())) {
            return Err(E::Refused);
        }
        let tm = target.metadata().map_err(|_| E::Io)?;
        slot.anchor.sync_all().map_err(|_| E::Io)?;
        let basename = slot.path.file_name().ok_or(E::Refused)?;
        renameat_with(&source, basename, &target, name, RenameFlags::NOREPLACE).map_err(|e| {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                E::AlreadyExists
            } else {
                E::Io
            }
        })?;
        // Retain moved ownership BEFORE the next fallible observation or sync.
        slot.path = parent.join(name);
        slot.parent_identity = Some((tm.dev(), tm.ino()));
        slot.state = State::PublicationUncertain;
        slot.frozen = None;
        slot.check_identity().map_err(|_| E::Refused)?;
        slot.anchor
            .sync_all()
            .and_then(|_| target.sync_all())
            .and_then(|_| source.sync_all())
            .map_err(|_| E::Io)?;
        slot.frozen = Some(ContentStamp::of(
            &slot.anchor.metadata().map_err(|_| E::Io)?,
        ));
        slot.state = State::Published;
        Ok(PublishedRecovery { id: slot.id })
    }
}
impl ArtifactInventory {
    pub(crate) fn validated_recovery(&mut self) -> Result<ValidatedRecovery<'_>, CopyError> {
        let destination = self.slots[DestinationRole::Recovery.index()]
            .as_mut()
            .filter(|s| s.state == State::Validated && s.connection.is_none())
            .ok_or(DestinationError::InvalidReceipt)?;
        destination.check_frozen()?;
        Ok(ValidatedRecovery { destination })
    }
    pub(crate) fn require_published(
        &mut self,
        receipt: PublishedRecovery,
    ) -> Result<(), CopyError> {
        let slot = self.slot(DestinationRole::Recovery, receipt.id, State::Published)?;
        slot.check_frozen()?;
        Ok(())
    }
    pub(crate) fn native_handles_closed(&self) -> bool {
        self.slots
            .iter()
            .flatten()
            .all(|slot| slot.connection.is_none())
    }
}
