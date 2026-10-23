//! Sealed upload spool: bounded write, SHA-256 hashing and a no-replace seal.

use std::fs::{DirBuilder, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

use rustix::cstr;
use rustix::fs::{renameat_with, RenameFlags};
use sha2::{Digest, Sha256};

use super::strict::Sha256 as WireSha256;

/// The fixed owned I/O buffer bound (policy `caps.ioChunkBytes`).
pub const IO_CHUNK_BYTES: usize = 65_536;

/// A spool admission/IO failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpoolError {
    /// An underlying filesystem or write error.
    Io,
    /// The write would exceed the declared byte length.
    Oversize,
    /// The spool finished with fewer bytes than declared.
    Short,
    /// The sealed destination already exists; no replacement or resume.
    AlreadySealed,
}

/// The immutable sealed spool identity: observed digest and byte length.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpoolSealed {
    pub sha256: WireSha256,
    pub bytes: u64,
}

/// An in-progress source spool writing `source.part` until an exact-EOF seal.
pub struct Spool {
    declared: u64,
    written: u64,
    hasher: Sha256,
    file: File,
    job_dir: PathBuf,
    sealed: bool,
    file_identity: (u64, u64),
    directory_identity: (u64, u64),
}

impl Spool {
    /// Creates the private job directory (0700) and a fresh `source.part`.
    pub fn create(job_dir: &Path, declared: u64) -> Result<Self, SpoolError> {
        DirBuilder::new()
            .recursive(false)
            .mode(0o700)
            .create(job_dir)
            .map_err(|_| SpoolError::Io)?;
        let part = job_dir.join("source.part");
        let file = OpenOptions::new()
            .read(true)
            .mode(0o600)
            .write(true)
            .create_new(true)
            .open(&part)
            .map_err(|_| SpoolError::Io)?;
        Self::from_owned(job_dir, declared, file)
    }

    pub(crate) fn from_owned(
        job_dir: &Path,
        declared: u64,
        file: File,
    ) -> Result<Self, SpoolError> {
        let meta = file.metadata().map_err(|_| SpoolError::Io)?;
        let directory = std::fs::symlink_metadata(job_dir).map_err(|_| SpoolError::Io)?;
        Ok(Spool {
            file_identity: (meta.dev(), meta.ino()),
            directory_identity: (directory.dev(), directory.ino()),
            declared,
            written: 0,
            hasher: Sha256::new(),
            file,
            job_dir: job_dir.to_path_buf(),
            sealed: false,
        })
    }

    /// Writes a borrowed chunk, counting and hashing the actual consumed bytes.
    pub fn write(&mut self, bytes: &[u8]) -> Result<(), SpoolError> {
        if self.sealed {
            return Err(SpoolError::AlreadySealed);
        }
        let len = bytes.len() as u64;
        let next = self.written.checked_add(len).ok_or(SpoolError::Oversize)?;
        if next > self.declared {
            return Err(SpoolError::Oversize);
        }
        self.file.write_all(bytes).map_err(|_| SpoolError::Io)?;
        self.hasher.update(bytes);
        self.written = next;
        Ok(())
    }

    /// Bytes written so far.
    pub fn written(&self) -> u64 {
        self.written
    }

    /// Seals only on exact EOF: flushes/syncs, closes and no-replace renames
    /// `source.part` -> `source.sealed`.
    pub fn finish(mut self) -> Result<SpoolSealed, SpoolError> {
        if self.written != self.declared {
            return Err(SpoolError::Short);
        }
        self.file.flush().map_err(|_| SpoolError::Io)?;
        self.file.sync_all().map_err(|_| SpoolError::Io)?;
        let meta = self.file.metadata().map_err(|_| SpoolError::Io)?;
        let path = std::fs::symlink_metadata(self.job_dir.join("source.part"))
            .map_err(|_| SpoolError::Io)?;
        let directory = std::fs::symlink_metadata(&self.job_dir).map_err(|_| SpoolError::Io)?;
        if !path.is_file()
            || path.file_type().is_symlink()
            || path.nlink() != 1
            || path.mode() & 0o777 != 0o600
            || (path.dev(), path.ino()) != self.file_identity
            || (meta.dev(), meta.ino()) != self.file_identity
            || path.len() != self.written
            || !directory.is_dir()
            || directory.file_type().is_symlink()
            || (directory.dev(), directory.ino()) != self.directory_identity
        {
            return Err(SpoolError::Io);
        }
        let mut readback =
            File::open(self.job_dir.join("source.part")).map_err(|_| SpoolError::Io)?;
        let mut hash = Sha256::new();
        let mut buffer = [0u8; IO_CHUNK_BYTES];
        let mut observed = 0u64;
        loop {
            let n = readback.read(&mut buffer).map_err(|_| SpoolError::Io)?;
            if n == 0 {
                break;
            }
            observed = observed.checked_add(n as u64).ok_or(SpoolError::Oversize)?;
            if observed > self.declared {
                return Err(SpoolError::Oversize);
            }
            hash.update(&buffer[..n]);
        }
        if observed != self.written || hash.finalize()[..] != self.hasher.clone().finalize()[..] {
            return Err(SpoolError::Io);
        }
        drop(readback);
        drop(self.file);

        let dir = File::open(&self.job_dir).map_err(|_| SpoolError::Io)?;
        renameat_with(
            &dir,
            cstr!("source.part"),
            &dir,
            cstr!("source.sealed"),
            RenameFlags::NOREPLACE,
        )
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                SpoolError::AlreadySealed
            } else {
                SpoolError::Io
            }
        })?;

        dir.sync_all().map_err(|_| SpoolError::Io)?;
        let digest = self.hasher.finalize();
        Ok(SpoolSealed {
            sha256: WireSha256::from_bytes(digest.into()),
            bytes: self.written,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job_dir() -> (tempfile::TempDir, PathBuf) {
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = tmp.path().join("j-abcdef");
        (tmp, dir)
    }

    #[test]
    fn seals_exact_eof_with_observed_digest() {
        let (_tmp, dir) = job_dir();
        let mut spool = Spool::create(&dir, 4).unwrap();
        spool.write(b"abcd").unwrap();
        let sealed = spool.finish().unwrap();
        assert_eq!(sealed.bytes, 4);
        // SHA-256 of "abcd".
        let expected = Sha256::digest(b"abcd");
        assert_eq!(sealed.sha256.as_bytes(), expected.as_slice());
        assert!(dir.join("source.sealed").exists());
        assert!(!dir.join("source.part").exists());
    }

    #[test]
    fn rejects_short_finish() {
        let (_tmp, dir) = job_dir();
        let mut spool = Spool::create(&dir, 4).unwrap();
        spool.write(b"ab").unwrap();
        assert_eq!(spool.finish(), Err(SpoolError::Short));
    }

    #[test]
    fn rejects_oversize_write() {
        let (_tmp, dir) = job_dir();
        let mut spool = Spool::create(&dir, 3).unwrap();
        spool.write(b"ab").unwrap();
        assert_eq!(spool.write(b"cd"), Err(SpoolError::Oversize));
        assert_eq!(spool.written(), 2);
    }

    #[test]
    fn refuses_replace_when_sealed_already_present() {
        let (_tmp, dir) = job_dir();
        let mut spool = Spool::create(&dir, 1).unwrap();
        spool.write(b"x").unwrap();
        // Simulate a conflicting sealed file appearing before the seal.
        std::fs::write(dir.join("source.sealed"), b"existing").unwrap();
        assert_eq!(spool.finish(), Err(SpoolError::AlreadySealed));
        assert_eq!(
            std::fs::read(dir.join("source.sealed")).unwrap(),
            b"existing"
        );
    }
}
