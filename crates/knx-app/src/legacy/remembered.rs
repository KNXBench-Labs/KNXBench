//! The one legacy password a user asked to remember: a private file, never a list (ADR-0094, L3).
//!
//! The user decided (grilling Q3/Q12) that exactly one password may be kept,
//! by explicit request, in the server's or user's configuration directory:
//! `$XDG_CONFIG_HOME/knx/legacy-vd-password`, else
//! `$HOME/.config/knx/legacy-vd-password`. It is a plain file with mode
//! 0600 in a directory created 0700, written atomically. A file that group
//! or others may read is refused, never used. It is never a candidate list:
//! a password the user gives wins, the remembered one is tried only when
//! none was given, and nothing else is ever tried.

use std::ffi::OsString;
use std::fmt;
use std::io::{ErrorKind, Read, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use knx_productdb::legacy::{read_legacy_member, LegacyError, LegacyPayload};

use super::{open_legacy_file, LegacyPassword};

/// File name of the remembered password inside `<config>/knx/`.
pub const REMEMBERED_PASSWORD_FILE: &str = "legacy-vd-password";

/// Distinguishes the temporary files of concurrent [`RememberedPassword::store`] calls.
static TEMP_SERIAL: AtomicU64 = AtomicU64::new(0);

/// A remembered password is one short line; a bounded read keeps a
/// replaced special file from exhausting memory.
const MAX_REMEMBERED_READ: u64 = 4096;

/// Where the remembered password lives, from the two environment values.
pub fn remembered_password_path_from(
    xdg_config_home: Option<OsString>,
    home: Option<OsString>,
) -> Option<PathBuf> {
    let base = match xdg_config_home.filter(|v| !v.is_empty()) {
        Some(dir) => PathBuf::from(dir),
        None => PathBuf::from(home.filter(|v| !v.is_empty())?).join(".config"),
    };
    Some(base.join("knx").join(REMEMBERED_PASSWORD_FILE))
}

/// [`remembered_password_path_from`] for this process's environment.
pub fn default_remembered_password_path() -> Option<PathBuf> {
    remembered_password_path_from(
        std::env::var_os("XDG_CONFIG_HOME"),
        std::env::var_os("HOME"),
    )
}

/// Why the remembered password could not be read, written or removed.
/// No variant carries the password.
#[derive(Debug)]
pub enum RememberError {
    /// An empty password is never stored.
    Empty,
    /// Group or others may access the file; it is not used.
    TooOpen {
        path: PathBuf,
        mode: u32,
    },
    NotUtf8 {
        path: PathBuf,
    },
    /// The file exists, but its first line is empty.
    EmptyFile {
        path: PathBuf,
    },
    Io {
        path: PathBuf,
        message: String,
    },
}

impl fmt::Display for RememberError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("an empty password is not remembered"),
            Self::TooOpen { path, mode } => write!(
                f,
                "the remembered password file {} has mode {mode:04o}; it must be 0600 \
                 (only the owner may read it), so it is not used",
                path.display()
            ),
            Self::NotUtf8 { path } => write!(
                f,
                "the remembered password file {} is not UTF-8 text",
                path.display()
            ),
            Self::EmptyFile { path } => write!(
                f,
                "the remembered password file {} is empty",
                path.display()
            ),
            Self::Io { path, message } => write!(f, "{}: {message}", path.display()),
        }
    }
}

impl std::error::Error for RememberError {}

fn io(path: &Path, error: std::io::Error) -> RememberError {
    RememberError::Io {
        path: path.to_path_buf(),
        message: error.to_string(),
    }
}

/// The remembered password's file. Holds only its path.
#[derive(Debug, Clone)]
pub struct RememberedPassword {
    path: PathBuf,
}

impl RememberedPassword {
    pub fn at(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Whether a password is remembered (the file exists).
    pub fn is_set(&self) -> bool {
        self.path.is_file()
    }

    /// The remembered password, `None` when there is none.
    pub fn load(&self) -> Result<Option<LegacyPassword>, RememberError> {
        let file = match std::fs::File::open(&self.path) {
            Ok(file) => file,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(io(&self.path, e)),
        };
        let mode = file
            .metadata()
            .map_err(|e| io(&self.path, e))?
            .permissions()
            .mode()
            & 0o777;
        if mode & 0o077 != 0 {
            return Err(RememberError::TooOpen {
                path: self.path.clone(),
                mode,
            });
        }
        let mut bytes = Vec::new();
        file.take(MAX_REMEMBERED_READ)
            .read_to_end(&mut bytes)
            .map_err(|e| io(&self.path, e))?;
        let text = String::from_utf8(bytes).map_err(|_| RememberError::NotUtf8 {
            path: self.path.clone(),
        })?;
        let line = text.split('\n').next().unwrap_or_default();
        let line = line.strip_suffix('\r').unwrap_or(line);
        if line.is_empty() {
            return Err(RememberError::EmptyFile {
                path: self.path.clone(),
            });
        }
        Ok(Some(LegacyPassword::new(line)))
    }

    /// Remembers `password`, replacing the one remembered before. Written
    /// to a 0600 temporary file beside the target and renamed over it.
    pub fn store(&self, password: &LegacyPassword) -> Result<(), RememberError> {
        if password.0.is_empty() {
            return Err(RememberError::Empty);
        }
        let parent = self.path.parent().ok_or_else(|| RememberError::Io {
            path: self.path.clone(),
            message: "no parent directory".into(),
        })?;
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(parent)
            .map_err(|e| io(parent, e))?;
        let name = self
            .path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| REMEMBERED_PASSWORD_FILE.into());
        // Unique per process and per call: two concurrent stores (two
        // server requests) never share or delete each other's temporary
        // file; `create_new` refuses rather than reuse a leftover.
        let serial = TEMP_SERIAL.fetch_add(1, Ordering::Relaxed);
        let temp = parent.join(format!(".{name}.tmp-{}-{serial}", std::process::id()));
        let written = (|| {
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&temp)?;
            file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
            file.write_all(password.0.as_bytes())?;
            file.write_all(b"\n")?;
            file.sync_all()?;
            std::fs::rename(&temp, &self.path)
        })();
        if let Err(e) = written {
            let _ = std::fs::remove_file(&temp);
            return Err(io(&self.path, e));
        }
        if let Ok(dir) = std::fs::File::open(parent) {
            let _ = dir.sync_all();
        }
        Ok(())
    }

    /// Forgets the remembered password. `true` when there was one.
    pub fn forget(&self) -> Result<bool, RememberError> {
        match std::fs::remove_file(&self.path) {
            Ok(()) => Ok(true),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(false),
            Err(e) => Err(io(&self.path, e)),
        }
    }
}

/// Which password opened a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasswordUsed {
    /// The file is not encrypted.
    None,
    Given,
    Remembered,
}

/// Why a legacy file could not be opened under the password policy.
#[derive(Debug)]
pub enum LegacyOpenError {
    Legacy(LegacyError),
    /// No password was given, and the remembered one does not open this
    /// file (or the file is damaged).
    RememberedDoesNotFit,
    /// No password was given, and the remembered one cannot be read.
    Remembered(RememberError),
}

impl fmt::Display for LegacyOpenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Legacy(e) => e.fmt(f),
            Self::RememberedDoesNotFit => f.write_str(
                "the remembered password does not open this file (or the file is damaged); \
                 give its password",
            ),
            Self::Remembered(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for LegacyOpenError {}

/// Opens a legacy file: with the given password when there is a non-empty
/// one, otherwise with the remembered one, otherwise asks for a password
/// (`PasswordRequired`). An unencrypted file needs none. Nothing else is
/// ever tried.
pub fn open_with_password_policy(
    bytes: &[u8],
    given: Option<&LegacyPassword>,
    remembered: Option<&RememberedPassword>,
) -> Result<(LegacyPayload, PasswordUsed), LegacyOpenError> {
    let member = read_legacy_member(bytes).map_err(LegacyOpenError::Legacy)?;
    if member.encrypted_stream().is_none() {
        return open_legacy_file(bytes, None)
            .map(|payload| (payload, PasswordUsed::None))
            .map_err(LegacyOpenError::Legacy);
    }
    if let Some(given) = given.filter(|p| !p.0.is_empty()) {
        return open_legacy_file(bytes, Some(given))
            .map(|payload| (payload, PasswordUsed::Given))
            .map_err(LegacyOpenError::Legacy);
    }
    let Some(store) = remembered else {
        return Err(LegacyOpenError::Legacy(LegacyError::PasswordRequired));
    };
    let Some(password) = store.load().map_err(LegacyOpenError::Remembered)? else {
        return Err(LegacyOpenError::Legacy(LegacyError::PasswordRequired));
    };
    match open_legacy_file(bytes, Some(&password)) {
        Ok(payload) => Ok((payload, PasswordUsed::Remembered)),
        Err(LegacyError::WrongPassword | LegacyError::WrongPasswordOrCorrupt) => {
            Err(LegacyOpenError::RememberedDoesNotFit)
        }
        Err(e) => Err(LegacyOpenError::Legacy(e)),
    }
}
