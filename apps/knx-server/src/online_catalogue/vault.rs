//! The credential file: AES-256-GCM under a separately stored random key, bound to one server.

use std::fs::{DirBuilder, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM, NONCE_LEN};
use serde::{Deserialize, Serialize};

use super::error::VaultError;

const FORMAT_VERSION: u32 = 1;
const KEY_LEN: usize = 32;
/// A credential record is a few hundred bytes; a bounded read keeps a swapped file harmless.
const MAX_FILE_BYTES: u64 = 16 * 1024;
const AAD_PREFIX: &[u8] = b"knxbench-online-catalogue-credentials-v1:";
const MAX_FIELD_BYTES: usize = 4096;

/// What the user stored: a login for renewable tokens, or a token alone.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "camelCase")]
pub enum Credentials {
    Password { user: String, password: String },
    Token { token: String },
}

impl std::fmt::Debug for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Password { user, .. } => write!(f, "Credentials::Password({user}, [redacted])"),
            Self::Token { .. } => write!(f, "Credentials::Token([redacted])"),
        }
    }
}

impl Credentials {
    /// Refuses empty, oversized or control-character fields before anything is stored.
    pub fn validate(&self) -> Result<(), &'static str> {
        let fields: Vec<&str> = match self {
            Self::Password { user, password } => vec![user, password],
            Self::Token { token } => vec![token],
        };
        for field in fields {
            if field.is_empty() || field.len() > MAX_FIELD_BYTES {
                return Err("a credential field is empty or too long");
            }
            if field.chars().any(char::is_control) {
                return Err("a credential field has control characters");
            }
        }
        if let Self::Token { token } = self {
            if token.chars().any(char::is_whitespace) {
                return Err("a token must not contain whitespace");
            }
        }
        Ok(())
    }

    pub fn user_name(&self) -> Option<&str> {
        match self {
            Self::Password { user, .. } => Some(user),
            Self::Token { .. } => None,
        }
    }
}

/// The plaintext inside the sealed file.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Sealed {
    origin: String,
    credentials: Credentials,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileBody {
    version: u32,
    alg: String,
    nonce: String,
    ciphertext: String,
}

/// Where the sealed file and its key live; they are deliberately different directories.
#[derive(Debug, Clone)]
pub struct Vault {
    file: PathBuf,
    key: PathBuf,
    source: &'static str,
}

/// A non-secret description of what is stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VaultState {
    Empty,
    Stored {
        mode: &'static str,
        user: Option<String>,
    },
    Problem(VaultError),
}

impl Vault {
    pub fn new(file: PathBuf, key: PathBuf, source: &'static str) -> Self {
        Self { file, key, source }
    }

    /// Reads the stored account for `origin`; `Ok(None)` when nothing is stored.
    pub fn load(&self, origin: &str) -> Result<Option<Credentials>, VaultError> {
        let body = match read_bounded(&self.file) {
            Ok(Some(bytes)) => bytes,
            Ok(None) => return Ok(None),
            Err(error) => return Err(error),
        };
        let key = self.read_key()?.ok_or(VaultError::KeyMissing)?;
        let body: FileBody = serde_json::from_slice(&body).map_err(|_| VaultError::Corrupt)?;
        if body.version != FORMAT_VERSION || body.alg != "AES-256-GCM" {
            return Err(VaultError::Corrupt);
        }
        let nonce: [u8; NONCE_LEN] = B64
            .decode(&body.nonce)
            .ok()
            .and_then(|v| v.try_into().ok())
            .ok_or(VaultError::Corrupt)?;
        let mut data = B64
            .decode(&body.ciphertext)
            .map_err(|_| VaultError::Corrupt)?;
        // The origin sits inside the sealed plaintext, so it cannot be altered without the key;
        // a file copied to another server address opens but is refused by the check below.
        let stored = self.open(&key, nonce, &mut data)?;
        if stored.origin != origin {
            return Err(VaultError::OriginMismatch);
        }
        Ok(Some(stored.credentials))
    }

    fn open(
        &self,
        key: &[u8; KEY_LEN],
        nonce: [u8; NONCE_LEN],
        data: &mut [u8],
    ) -> Result<Sealed, VaultError> {
        let opened = LessSafeKey::new(
            UnboundKey::new(&AES_256_GCM, key).map_err(|_| VaultError::KeyInvalid)?,
        )
        .open_in_place(
            Nonce::assume_unique_for_key(nonce),
            Aad::from(self.aad()),
            data,
        )
        .map_err(|_| VaultError::Corrupt)?;
        serde_json::from_slice(opened).map_err(|_| VaultError::Corrupt)
    }

    fn aad(&self) -> Vec<u8> {
        let mut aad = AAD_PREFIX.to_vec();
        aad.extend_from_slice(self.source.as_bytes());
        aad
    }

    /// Seals and atomically replaces the stored account. Refuses to create a new key while an
    /// existing credential file has none, and refuses to overwrite an existing record that the
    /// current key cannot authenticate: explicit `forget` is the only way to replace it.
    pub fn store(&self, origin: &str, credentials: &Credentials) -> Result<(), VaultError> {
        credentials.validate().map_err(|_| VaultError::Corrupt)?;
        let existing_key = self.read_key()?;
        if existing_key.is_none() && self.file.exists() {
            return Err(VaultError::KeyMissing);
        }
        if existing_key.is_some() && self.file.exists() {
            match self.load(origin) {
                Ok(_) => {}
                Err(VaultError::OriginMismatch) => {}
                Err(_) => return Err(VaultError::Corrupt),
            }
        }
        let plain = serde_json::to_vec(&Sealed {
            origin: origin.to_string(),
            credentials: credentials.clone(),
        })
        .map_err(|_| VaultError::Io)?;
        // Seal with an in-memory key first; the key file is only created once the
        // envelope is known to fit the bounded read cap.
        let key: [u8; KEY_LEN] = match existing_key {
            Some(key) => key,
            None => {
                let mut fresh = [0u8; KEY_LEN];
                getrandom::fill(&mut fresh).map_err(|_| VaultError::Io)?;
                fresh
            }
        };
        let mut nonce = [0u8; NONCE_LEN];
        getrandom::fill(&mut nonce).map_err(|_| VaultError::Io)?;
        let mut data = plain;
        LessSafeKey::new(UnboundKey::new(&AES_256_GCM, &key).map_err(|_| VaultError::KeyInvalid)?)
            .seal_in_place_append_tag(
                Nonce::assume_unique_for_key(nonce),
                Aad::from(self.aad()),
                &mut data,
            )
            .map_err(|_| VaultError::Io)?;
        let body = serde_json::to_vec(&FileBody {
            version: FORMAT_VERSION,
            alg: "AES-256-GCM".into(),
            nonce: B64.encode(nonce),
            ciphertext: B64.encode(&data),
        })
        .map_err(|_| VaultError::Io)?;
        // The sealed envelope must stay readable under the bounded read cap.
        if body.len() as u64 > MAX_FILE_BYTES {
            return Err(VaultError::Corrupt);
        }
        if existing_key.is_none() {
            self.write_key(&key)?;
        }
        write_private(&self.file, &body)
    }

    /// Removes the stored account (the key stays: it protects nothing else, and a later store reuses it).
    pub fn forget(&self) -> Result<(), VaultError> {
        match std::fs::remove_file(&self.file) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err(VaultError::Io),
        }
    }

    /// What is stored, without exposing a secret.
    pub fn state(&self, origin: &str) -> VaultState {
        match self.load(origin) {
            Ok(None) => VaultState::Empty,
            Ok(Some(Credentials::Password { user, .. })) => VaultState::Stored {
                mode: "password",
                user: Some(user),
            },
            Ok(Some(Credentials::Token { .. })) => VaultState::Stored {
                mode: "token",
                user: None,
            },
            Err(error) => VaultState::Problem(error),
        }
    }

    fn read_key(&self) -> Result<Option<[u8; KEY_LEN]>, VaultError> {
        let meta = match std::fs::symlink_metadata(&self.key) {
            Ok(meta) => meta,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(VaultError::Io),
        };
        if !meta.is_file() || meta.nlink() != 1 || meta.len() != KEY_LEN as u64 {
            return Err(VaultError::KeyInvalid);
        }
        if meta.mode() & 0o077 != 0 {
            return Err(VaultError::KeyTooOpen);
        }
        let mut bytes = Vec::new();
        super::store::open_regular(&self.key, KEY_LEN as u64)
            .and_then(|f| f.take(KEY_LEN as u64 + 1).read_to_end(&mut bytes))
            .map_err(|_| VaultError::Io)?;
        bytes
            .try_into()
            .map(Some)
            .map_err(|_| VaultError::KeyInvalid)
    }

    /// Persists a freshly generated key; only called after the sealed envelope fit the cap.
    fn write_key(&self, key: &[u8; KEY_LEN]) -> Result<(), VaultError> {
        if let Some(dir) = self.key.parent() {
            private_dir(dir)?;
        }
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&self.key)
            .map_err(|_| VaultError::Io)?;
        file.write_all(key).map_err(|_| VaultError::Io)?;
        file.sync_all().map_err(|_| VaultError::Io)?;
        File::open(self.key.parent().ok_or(VaultError::Io)?)
            .and_then(|dir| dir.sync_all())
            .map_err(|_| VaultError::Io)?;
        Ok(())
    }
}

fn private_dir(dir: &Path) -> Result<(), VaultError> {
    DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)
        .map_err(|_| VaultError::Io)
}

fn read_bounded(path: &Path) -> Result<Option<Vec<u8>>, VaultError> {
    let file = match super::store::open_regular(path, MAX_FILE_BYTES) {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(VaultError::Io),
    };
    let mut bytes = Vec::new();
    file.take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| VaultError::Io)?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err(VaultError::Corrupt);
    }
    Ok(Some(bytes))
}

fn write_private(path: &Path, body: &[u8]) -> Result<(), VaultError> {
    let dir = path.parent().ok_or(VaultError::Io)?;
    private_dir(dir)?;
    let mut temp = tempfile::NamedTempFile::new_in(dir).map_err(|_| VaultError::Io)?;
    temp.write_all(body).map_err(|_| VaultError::Io)?;
    temp.as_file().sync_all().map_err(|_| VaultError::Io)?;
    temp.persist(path).map_err(|_| VaultError::Io)?;
    File::open(dir)
        .and_then(|f| f.sync_all())
        .map_err(|_| VaultError::Io)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vault(dir: &Path) -> Vault {
        Vault::new(
            dir.join("data/credentials.enc"),
            dir.join("config/key"),
            "reko",
        )
    }

    fn login() -> Credentials {
        Credentials::Password {
            user: "alice".into(),
            password: "correct horse battery staple".into(),
        }
    }

    #[test]
    fn stored_credentials_round_trip_and_are_not_plaintext_on_disk() {
        let tmp = tempfile::tempdir().unwrap();
        let v = vault(tmp.path());
        assert_eq!(v.state("https://reko.example"), VaultState::Empty);
        v.store("https://reko.example", &login()).unwrap();
        assert_eq!(v.load("https://reko.example").unwrap(), Some(login()));
        let on_disk = std::fs::read_to_string(tmp.path().join("data/credentials.enc")).unwrap();
        assert!(!on_disk.contains("alice") && !on_disk.contains("horse"));
        assert_eq!(
            v.state("https://reko.example"),
            VaultState::Stored {
                mode: "password",
                user: Some("alice".into())
            }
        );
        // Mode of the key and the credential file: private to the owner.
        for p in ["config/key", "data/credentials.enc"] {
            let mode = std::fs::metadata(tmp.path().join(p)).unwrap().mode() & 0o777;
            assert_eq!(mode, 0o600, "{p}");
        }
    }

    #[test]
    fn a_new_save_replaces_and_forget_removes_without_touching_the_key() {
        let tmp = tempfile::tempdir().unwrap();
        let v = vault(tmp.path());
        v.store("https://a.example", &login()).unwrap();
        let token = Credentials::Token {
            token: "tok123".into(),
        };
        v.store("https://a.example", &token).unwrap();
        assert_eq!(v.load("https://a.example").unwrap(), Some(token));
        v.forget().unwrap();
        assert_eq!(v.load("https://a.example").unwrap(), None);
        v.forget().unwrap();
        assert!(tmp.path().join("config/key").exists());
        v.store("https://a.example", &login()).unwrap();
    }

    #[test]
    fn another_origin_never_receives_the_stored_account() {
        let tmp = tempfile::tempdir().unwrap();
        let v = vault(tmp.path());
        v.store("https://a.example", &login()).unwrap();
        assert_eq!(v.load("https://b.example"), Err(VaultError::OriginMismatch));
    }

    #[test]
    fn a_missing_key_never_creates_a_replacement_for_an_existing_file() {
        let tmp = tempfile::tempdir().unwrap();
        let v = vault(tmp.path());
        v.store("https://a.example", &login()).unwrap();
        std::fs::remove_file(tmp.path().join("config/key")).unwrap();
        assert_eq!(v.load("https://a.example"), Err(VaultError::KeyMissing));
        assert_eq!(
            v.store("https://a.example", &login()),
            Err(VaultError::KeyMissing)
        );
        assert!(!tmp.path().join("config/key").exists());
        // An explicit forget is the way out; the next store starts a fresh pair.
        v.forget().unwrap();
        v.store("https://a.example", &login()).unwrap();
        assert!(v.load("https://a.example").unwrap().is_some());
    }

    #[test]
    fn tampering_a_wrong_key_and_an_open_key_are_all_refused() {
        let tmp = tempfile::tempdir().unwrap();
        let v = vault(tmp.path());
        v.store("https://a.example", &login()).unwrap();
        let path = tmp.path().join("data/credentials.enc");
        let mut body: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        let ct = body["ciphertext"].as_str().unwrap().to_string();
        let mut raw = B64.decode(&ct).unwrap();
        raw[0] ^= 1;
        body["ciphertext"] = B64.encode(raw).into();
        std::fs::write(&path, serde_json::to_vec(&body).unwrap()).unwrap();
        assert_eq!(v.load("https://a.example"), Err(VaultError::Corrupt));

        v.forget().unwrap();
        v.store("https://a.example", &login()).unwrap();
        let key_path = tmp.path().join("config/key");
        std::fs::write(&key_path, [7u8; 32]).unwrap();
        assert_eq!(v.load("https://a.example"), Err(VaultError::Corrupt));
        std::fs::write(&key_path, [7u8; 5]).unwrap();
        assert_eq!(v.load("https://a.example"), Err(VaultError::KeyInvalid));
        // Isolate the permission refusal from the preceding wrong-length-key case.
        std::fs::write(&key_path, [7u8; 32]).unwrap();
        let mut perms = std::fs::metadata(&key_path).unwrap().permissions();
        std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o644);
        std::fs::set_permissions(&key_path, perms).unwrap();
        assert_eq!(v.load("https://a.example"), Err(VaultError::KeyTooOpen));
    }

    #[test]
    fn debug_output_never_shows_a_secret_and_bad_fields_are_refused() {
        let text = format!(
            "{:?} {:?}",
            login(),
            Credentials::Token {
                token: "sekret".into()
            }
        );
        assert!(!text.contains("horse") && !text.contains("sekret"));
        for bad in [
            Credentials::Password {
                user: "".into(),
                password: "x".into(),
            },
            Credentials::Password {
                user: "a".into(),
                password: "x\n".into(),
            },
            Credentials::Token {
                token: "a b".into(),
            },
            Credentials::Token {
                token: "x".repeat(MAX_FIELD_BYTES + 1),
            },
        ] {
            assert!(bad.validate().is_err());
        }
    }
}
