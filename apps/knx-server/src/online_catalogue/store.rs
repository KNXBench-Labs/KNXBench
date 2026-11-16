//! On-disk state of one catalogue source: configured address, index document and its metadata.

use std::fs::DirBuilder;
use std::io::{Read, Write};
use std::os::unix::fs::DirBuilderExt;
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::index::{self, Snapshot, MAX_DECODED_BYTES};

pub const CONFIG_FILE: &str = "config.json";
pub const META_FILE: &str = "index.meta.json";
const DATA_PREFIX: &str = "index-";
const DATA_SUFFIX: &str = ".json";
const MAX_SMALL_FILE: u64 = 64 * 1024;
const STATE_SCHEMA: u32 = 1;

/// What is known about the stored index document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Meta {
    pub schema: u32,
    /// The canonical base address this index was fetched from; it must match to be used.
    pub origin: String,
    pub etag: Option<String>,
    pub version: String,
    pub generated_at: String,
    pub entry_count: u64,
    pub skipped: u64,
    pub data_file: String,
    pub data_sha256: String,
    /// When this document was last accepted from a 200 response, even if its bytes were unchanged.
    pub fetched_at: String,
    /// When the server last confirmed it (a new document or a 304).
    pub last_checked_at: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Config {
    schema: u32,
    base_url: Option<String>,
}

/// The outcome of loading the stored index for a given configured address.
pub enum Loaded {
    Empty,
    /// An index exists but was fetched from another address; it is not used.
    OtherOrigin,
    /// The files do not match their metadata or do not parse.
    Damaged,
    Ready(Box<Meta>, Box<Snapshot>),
}

pub fn private_dir(dir: &Path) -> std::io::Result<()> {
    DirBuilder::new().recursive(true).mode(0o700).create(dir)
}

pub fn write_atomic(dir: &Path, name: &str, bytes: &[u8]) -> std::io::Result<()> {
    private_dir(dir)?;
    let mut temp = tempfile::NamedTempFile::new_in(dir)?;
    temp.write_all(bytes)?;
    temp.as_file().sync_all()?;
    temp.persist(dir.join(name)).map_err(|e| e.error)?;
    std::fs::File::open(dir)?.sync_all()
}

/// Opens a bounded private regular file without following a final symlink.
pub(super) fn open_regular(path: &Path, cap: u64) -> std::io::Result<std::fs::File> {
    use rustix::fs::{open, Mode, OFlags};
    use std::os::unix::fs::MetadataExt;
    let file = std::fs::File::from(open(
        path,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )?);
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.nlink() != 1 || metadata.len() > cap {
        return Err(std::io::Error::other(
            "private file is not a bounded regular file",
        ));
    }
    Ok(file)
}

fn read_limited(path: &Path, cap: u64) -> std::io::Result<Option<Vec<u8>>> {
    let file = match open_regular(path, cap) {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
    };
    let mut bytes = Vec::new();
    file.take(cap + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > cap {
        return Err(std::io::Error::other("file larger than its limit"));
    }
    Ok(Some(bytes))
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// The user's configured base address, when one was saved.
pub fn read_base_url(dir: &Path) -> Option<String> {
    let bytes = read_limited(&dir.join(CONFIG_FILE), MAX_SMALL_FILE).ok()??;
    let config: Config = serde_json::from_slice(&bytes).ok()?;
    (config.schema == STATE_SCHEMA)
        .then_some(config.base_url)
        .flatten()
}

pub fn write_base_url(dir: &Path, base: Option<&str>) -> std::io::Result<()> {
    let body = serde_json::to_vec(&Config {
        schema: STATE_SCHEMA,
        base_url: base.map(str::to_string),
    })
    .map_err(std::io::Error::other)?;
    write_atomic(dir, CONFIG_FILE, &body)
}

fn read_meta(dir: &Path) -> Result<Option<Meta>, ()> {
    let Some(bytes) = read_limited(&dir.join(META_FILE), MAX_SMALL_FILE).map_err(|_| ())? else {
        return Ok(None);
    };
    let meta: Meta = serde_json::from_slice(&bytes).map_err(|_| ())?;
    let name_ok = meta.data_file.starts_with(DATA_PREFIX)
        && meta.data_file.ends_with(DATA_SUFFIX)
        && !meta.data_file.contains(['/', '\\']);
    if meta.schema != STATE_SCHEMA || !name_ok {
        return Err(());
    }
    Ok(Some(meta))
}

/// Loads and verifies the stored index for the configured origin.
pub fn load_index(dir: &Path, origin: &str) -> Loaded {
    let meta = match read_meta(dir) {
        Ok(Some(meta)) => meta,
        Ok(None) => return Loaded::Empty,
        Err(()) => return Loaded::Damaged,
    };
    if meta.origin != origin {
        return Loaded::OtherOrigin;
    }
    let Ok(Some(data)) = read_limited(&dir.join(&meta.data_file), MAX_DECODED_BYTES as u64) else {
        return Loaded::Damaged;
    };
    if sha256_hex(&data) != meta.data_sha256 {
        return Loaded::Damaged;
    }
    match index::parse(&data) {
        Ok(parsed) if parsed.entries.len() as u64 + parsed.skipped == meta.entry_count => {
            Loaded::Ready(Box::new(meta), Box::new(Snapshot::new(parsed)))
        }
        _ => Loaded::Damaged,
    }
}

/// The stored metadata without parsing the document (cheap; used for status).
pub fn peek_meta(dir: &Path) -> Option<Meta> {
    read_meta(dir).ok().flatten()
}

/// Writes the new document, then its metadata, then removes superseded documents: a crash at any
/// point leaves either the old pair or the new pair readable.
pub fn save_index(dir: &Path, meta: &Meta, decoded: &[u8]) -> std::io::Result<()> {
    write_atomic(dir, &meta.data_file, decoded)?;
    let body = serde_json::to_vec(meta).map_err(std::io::Error::other)?;
    write_atomic(dir, META_FILE, &body)?;
    remove_superseded(dir, &meta.data_file);
    Ok(())
}

/// Updates only the confirmation time (a 304 answer).
pub fn save_meta(dir: &Path, meta: &Meta) -> std::io::Result<()> {
    let body = serde_json::to_vec(meta).map_err(std::io::Error::other)?;
    write_atomic(dir, META_FILE, &body)
}

fn remove_superseded(dir: &Path, keep: &str) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with(DATA_PREFIX) && name.ends_with(DATA_SUFFIX) && name != keep {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

/// The data file name for a document, derived from its digest.
pub fn data_file_name(sha256_hex: &str) -> String {
    format!("{DATA_PREFIX}{sha256_hex}{DATA_SUFFIX}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::online_catalogue::index::fixtures::{document, entry};

    fn meta_for(doc: &[u8], origin: &str) -> Meta {
        let sha = sha256_hex(doc);
        Meta {
            schema: STATE_SCHEMA,
            origin: origin.into(),
            etag: Some("\"e1\"".into()),
            version: "abc123".into(),
            generated_at: "2026-01-01T00:00:00+00:00".into(),
            entry_count: 1,
            skipped: 0,
            data_file: data_file_name(&sha),
            data_sha256: sha,
            fetched_at: "2026-01-02T00:00:00Z".into(),
            last_checked_at: "2026-01-02T00:00:00Z".into(),
        }
    }

    #[test]
    fn a_saved_index_loads_and_a_changed_origin_is_not_used() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(matches!(
            load_index(tmp.path(), "https://a.example"),
            Loaded::Empty
        ));
        let doc = document(vec![entry("a1", "knxprod", "A")]);
        let meta = meta_for(&doc, "https://a.example");
        save_index(tmp.path(), &meta, &doc).unwrap();
        let Loaded::Ready(loaded, snapshot) = load_index(tmp.path(), "https://a.example") else {
            panic!("expected a ready index");
        };
        assert_eq!(*loaded, meta);
        assert_eq!(snapshot.parsed.entries.len(), 1);
        assert!(matches!(
            load_index(tmp.path(), "https://b.example"),
            Loaded::OtherOrigin
        ));
    }

    #[test]
    fn a_damaged_or_mismatched_document_is_reported_not_trusted() {
        let tmp = tempfile::tempdir().unwrap();
        let doc = document(vec![entry("a1", "knxprod", "A")]);
        let meta = meta_for(&doc, "https://a.example");
        save_index(tmp.path(), &meta, &doc).unwrap();
        std::fs::write(tmp.path().join(&meta.data_file), b"{}").unwrap();
        assert!(matches!(
            load_index(tmp.path(), "https://a.example"),
            Loaded::Damaged
        ));
        std::fs::write(tmp.path().join(META_FILE), b"nonsense").unwrap();
        assert!(matches!(
            load_index(tmp.path(), "https://a.example"),
            Loaded::Damaged
        ));
    }

    #[test]
    fn a_newer_document_replaces_the_older_one_and_leaves_one_data_file() {
        let tmp = tempfile::tempdir().unwrap();
        let first = document(vec![entry("a1", "knxprod", "A")]);
        save_index(tmp.path(), &meta_for(&first, "https://a.example"), &first).unwrap();
        let second = document(vec![entry("a1", "knxprod", "A changed")]);
        let meta = meta_for(&second, "https://a.example");
        save_index(tmp.path(), &meta, &second).unwrap();
        let files: Vec<_> = std::fs::read_dir(tmp.path())
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| n.starts_with(DATA_PREFIX))
            .collect();
        assert_eq!(files, vec![meta.data_file]);
    }

    #[test]
    fn the_configured_address_round_trips_and_can_be_cleared() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(read_base_url(tmp.path()), None);
        write_base_url(tmp.path(), Some("https://x.example")).unwrap();
        assert_eq!(
            read_base_url(tmp.path()).as_deref(),
            Some("https://x.example")
        );
        write_base_url(tmp.path(), None).unwrap();
        assert_eq!(read_base_url(tmp.path()), None);
    }
}
