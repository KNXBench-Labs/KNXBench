//! Stage 1: unpacking a `.knxproj` file into its ZIP entries.
//!
//! Reads only — writing a container back is Task 19. Password-protected
//! projects (the nested `<project part>.zip` payload) are detected and
//! refused, not decrypted: both decryption schemes in IMPORT_EXPORT §2 are
//! unverified against a real protected project, and shipping an untested
//! decryption path would claim support this repository cannot demonstrate.

use std::io::{Cursor, Read};

use zip::ZipArchive;

/// Refuses to read an entry whose declared uncompressed size exceeds this
/// limit, before allocating anything for it (Task 21). Without this guard,
/// `Vec::with_capacity(entry.size())` allocates the *declared* size
/// upfront — a zip-bomb-shaped entry that lies about its size (e.g.
/// declaring gigabytes while containing only a few real bytes) does not
/// just run slowly, an allocation failure that large calls Rust's global
/// allocator error handler, which aborts the process outright rather than
/// returning a recoverable error. The largest entry in either reference
/// project is 5.7 MB; the whole uncompressed reference container is 22 MB
/// (RESEARCH). 64 MB leaves ample headroom for a legitimate project.
const MAX_ENTRY_SIZE: u64 = 64 * 1024 * 1024;

/// One entry's name and uncompressed size, snapshotted from the ZIP central
/// directory at open time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryInfo {
    pub path: String,
    pub size: u64,
}

/// An opened `.knxproj` archive: a ZIP over an in-memory byte buffer, plus an
/// inventory snapshot so lookups don't have to re-scan the central directory.
pub struct Container {
    archive: ZipArchive<Cursor<Vec<u8>>>,
    entries: Vec<EntryInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContainerError {
    NotAZip(String),
    EntryNotFound(String),
    Read { path: String, cause: String },
    NoProjectPart,
    PasswordProtected { nested_entry: String },
}

impl std::fmt::Display for ContainerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContainerError::NotAZip(cause) => write!(f, "not a zip archive: {cause}"),
            ContainerError::EntryNotFound(path) => write!(f, "entry not found: {path}"),
            ContainerError::Read { path, cause } => write!(f, "failed to read {path}: {cause}"),
            ContainerError::NoProjectPart => {
                write!(
                    f,
                    "no P-*.signature entry; cannot determine the project part"
                )
            }
            ContainerError::PasswordProtected { nested_entry } => write!(
                f,
                "project is password-protected (nested payload {nested_entry}); \
                 decryption is not implemented"
            ),
        }
    }
}

impl std::error::Error for ContainerError {}

impl Container {
    /// Opens `bytes` as a `.knxproj` archive and snapshots its inventory.
    ///
    /// Directory entries (which ETS6 writes and ETS4 does not) are skipped:
    /// they carry no content and every other operation here works by path,
    /// not by directory listing.
    pub fn open(bytes: Vec<u8>) -> Result<Self, ContainerError> {
        let mut archive = ZipArchive::new(Cursor::new(bytes))
            .map_err(|e| ContainerError::NotAZip(e.to_string()))?;

        let mut entries = Vec::with_capacity(archive.len());
        for i in 0..archive.len() {
            let entry = archive
                .by_index(i)
                .map_err(|e| ContainerError::NotAZip(e.to_string()))?;
            if entry.is_dir() {
                continue;
            }
            entries.push(EntryInfo {
                path: entry.name().to_string(),
                size: entry.size(),
            });
        }

        let container = Self { archive, entries };

        if let Some(project_part) = container.project_part_opt() {
            let nested = format!("{project_part}.zip");
            if container.find(&nested).is_some() {
                return Err(ContainerError::PasswordProtected {
                    nested_entry: nested,
                });
            }
        }

        Ok(container)
    }

    pub fn entries(&self) -> &[EntryInfo] {
        &self.entries
    }

    /// Reads one entry's bytes. Looked up case-insensitively via `find`, so
    /// callers never have to know the generation's filename casing.
    pub fn read(&mut self, path: &str) -> Result<Vec<u8>, ContainerError> {
        let found = self
            .find(path)
            .ok_or_else(|| ContainerError::EntryNotFound(path.to_string()))?;
        if found.size > MAX_ENTRY_SIZE {
            return Err(ContainerError::Read {
                path: found.path.clone(),
                cause: format!(
                    "declared uncompressed size {} bytes exceeds the {MAX_ENTRY_SIZE}-byte limit",
                    found.size
                ),
            });
        }
        let real_path = found.path.clone();
        let mut entry = self
            .archive
            .by_name(&real_path)
            .map_err(|e| ContainerError::Read {
                path: real_path.clone(),
                cause: e.to_string(),
            })?;
        let mut buf = Vec::with_capacity(entry.size() as usize);
        entry
            .read_to_end(&mut buf)
            .map_err(|e| ContainerError::Read {
                path: real_path,
                cause: e.to_string(),
            })?;
        Ok(buf)
    }

    /// Case-insensitive over the inventory; `Project.xml` and `project.xml`
    /// both resolve.
    pub fn find(&self, path: &str) -> Option<&EntryInfo> {
        self.entries
            .iter()
            .find(|e| e.path.eq_ignore_ascii_case(path))
    }

    /// The `P-xxxx` part, recovered from the `P-*.signature` entry name.
    pub fn project_part(&self) -> Result<&str, ContainerError> {
        self.project_part_opt().ok_or(ContainerError::NoProjectPart)
    }

    fn project_part_opt(&self) -> Option<&str> {
        self.entries.iter().find_map(|e| {
            let name = e.path.rsplit('/').next().unwrap_or(&e.path);
            name.strip_suffix(".signature")
                .filter(|stem| stem.starts_with("P-"))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::io::Write;
    use std::path::PathBuf;

    fn workspace_root() -> PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(std::path::Path::parent)
            .expect("crate lives at <root>/crates/<name>")
            .to_path_buf()
    }

    fn reference_ets4_bytes() -> Vec<u8> {
        std::fs::read(
            workspace_root()
                .join("OriginalData/DemoProjects/Unser Zuhause ets4 - 2025-12-15.knxproj"),
        )
        .expect("reference ETS4 project is committed at the workspace root")
    }

    fn zip_with_entries(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        for (name, bytes) in entries {
            writer.start_file(*name, options).unwrap();
            writer.write_all(bytes).unwrap();
        }
        writer.finish().unwrap().into_inner()
    }

    #[test]
    fn the_reference_project_inventory_has_thirty_eight_entries() {
        let mut c = Container::open(reference_ets4_bytes()).unwrap();
        assert_eq!(c.entries().len(), 38);
        // Measured: unlike the per-installation `0.xml` files, ETS4 writes
        // `knx_master.xml` with no UTF-8 BOM and no XML declaration — it
        // starts directly with the root element.
        assert!(c.read("knx_master.xml").unwrap().starts_with(b"<KNX"));
    }

    #[test]
    fn entry_lookup_is_case_insensitive() {
        let c = Container::open(reference_ets4_bytes()).unwrap();
        assert_eq!(
            c.find("p-0512/project.xml").map(|e| e.path.as_str()),
            Some("P-0512/Project.xml")
        );
    }

    #[test]
    fn the_project_part_comes_from_the_signature_filename() {
        let c = Container::open(reference_ets4_bytes()).unwrap();
        assert_eq!(c.project_part().unwrap(), "P-0512");
    }

    #[test]
    fn a_file_that_is_not_a_zip_is_rejected_with_its_own_error() {
        assert!(matches!(
            Container::open(b"not a zip at all".to_vec()),
            Err(ContainerError::NotAZip(_))
        ));
    }

    #[test]
    fn a_password_protected_project_is_detected_and_named() {
        // A container whose only project-part entry is `P-0512.zip` is the
        // nested-payload form ETS writes for a protected project.
        let bytes = zip_with_entries(&[("P-0512.signature", b"x"), ("P-0512.zip", b"PK\x03\x04")]);
        assert!(matches!(
            Container::open(bytes),
            Err(ContainerError::PasswordProtected { .. })
        ));
    }
}
