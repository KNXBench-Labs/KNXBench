//! Stage 1: unpacking a `.knxproj` file into its ZIP entries.
//!
//! Reads only — writing a container back is Task 19, and nothing here
//! ever writes a ZipCrypto- or AES-encrypted entry either way: both
//! protection schemes are read-only support for a project the caller
//! already has the password to, never a way to produce one.
//!
//! Password-protected projects (the nested `<project part>.zip` payload,
//! IMPORT_EXPORT §2) split by scheme. Schema < 21 (ETS4/ETS5) ZipCrypto is
//! decrypted, via [`Container::open_with_password`] — see `knx-secure`'s
//! `zipcrypto` module for the algorithm and why its 1990s cipher is not
//! security. Schema ≥ 21 (ETS6) AES stays refused
//! (`ContainerError::UnsupportedEncryption`): its key derivation lives in
//! `knx-secure` too, but container decryption is unverified against a
//! real protected project, and shipping an untested decryption path would
//! claim support this repository cannot demonstrate. [`Container::open`]
//! (no password) refuses either scheme outright, unchanged.

use std::collections::BTreeMap;
use std::io::{Cursor, Read};

use flate2::read::DeflateDecoder;
use knx_secure::zipcrypto::{self, CheckBytes, ZipCryptoError};
use zip::{CompressionMethod, ZipArchive};

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
///
/// For a ZipCrypto-protected project opened via
/// [`Container::open_with_password`], `decrypted` holds the nested
/// `<project part>.zip` payload's entries in plaintext, keyed by their
/// full path (e.g. `P-0001/Project.xml`) — [`read`](Container::read)
/// checks it before falling back to `archive`, so a caller never has to
/// know which entries came from which layer.
pub struct Container {
    archive: ZipArchive<Cursor<Vec<u8>>>,
    entries: Vec<EntryInfo>,
    decrypted: BTreeMap<String, Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContainerError {
    NotAZip(String),
    EntryNotFound(String),
    Read {
        path: String,
        cause: String,
    },
    NoProjectPart,
    /// [`Container::open`] found a nested `<project part>.zip` payload but
    /// was given no password to try. [`Container::open_with_password`]
    /// never returns this variant — with a password in hand it either
    /// decrypts, refuses by name (`UnsupportedEncryption`), or reports
    /// `WrongPassword`.
    PasswordProtected {
        nested_entry: String,
    },
    /// A password was supplied but the nested payload's ZipCrypto check
    /// byte (APPNOTE.TXT §6.1.6) matched neither known convention —
    /// see `knx_secure::zipcrypto`. For all practical purposes this
    /// means the password was wrong; see that module's docs for the one
    /// case (a false accept) this cannot rule out, which does not raise
    /// this error.
    WrongPassword {
        nested_entry: String,
    },
    /// The nested payload is protected with a scheme this crate does not
    /// decrypt yet — currently only AES (schema ≥ 21 / ETS6; see this
    /// module's header). Distinct from `PasswordProtected` so a caller
    /// that *did* supply a password can tell "wrong scheme" from "no
    /// password given" apart.
    UnsupportedEncryption {
        nested_entry: String,
        scheme: String,
    },
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
                 no password was supplied"
            ),
            ContainerError::WrongPassword { nested_entry } => write!(
                f,
                "wrong password for password-protected project (nested payload {nested_entry})"
            ),
            ContainerError::UnsupportedEncryption {
                nested_entry,
                scheme,
            } => write!(
                f,
                "project is protected with {scheme}, which this build does not decrypt \
                 (nested payload {nested_entry})"
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
        let container = Self::open_raw(bytes)?;

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

    /// Opens `bytes` as a `.knxproj` archive, decrypting the nested
    /// `<project part>.zip` payload with `password` if one is present.
    ///
    /// Behaves exactly like [`Container::open`] for a project that is not
    /// password-protected — a caller who does not yet know whether a
    /// project is protected can call this unconditionally instead of
    /// trying `open` first. Only ZipCrypto (schema < 21 / ETS4-ETS5) is
    /// decrypted; an AES-protected (schema ≥ 21 / ETS6) payload is
    /// refused with [`ContainerError::UnsupportedEncryption`], same as
    /// this module's header explains.
    pub fn open_with_password(bytes: Vec<u8>, password: &str) -> Result<Self, ContainerError> {
        let mut container = Self::open_raw(bytes)?;

        let Some(project_part) = container.project_part_opt().map(str::to_owned) else {
            return Ok(container);
        };
        let nested = format!("{project_part}.zip");
        let Some(nested_info) = container.find(&nested).cloned() else {
            return Ok(container);
        };

        if nested_info.size > MAX_ENTRY_SIZE {
            return Err(ContainerError::Read {
                path: nested_info.path.clone(),
                cause: format!(
                    "declared uncompressed size {} bytes exceeds the {MAX_ENTRY_SIZE}-byte limit",
                    nested_info.size
                ),
            });
        }

        let mut payload =
            container
                .archive
                .by_name(&nested_info.path)
                .map_err(|e| ContainerError::Read {
                    path: nested_info.path.clone(),
                    cause: e.to_string(),
                })?;
        let mut payload_bytes = Vec::with_capacity(payload.size() as usize);
        payload
            .read_to_end(&mut payload_bytes)
            .map_err(|e| ContainerError::Read {
                path: nested_info.path.clone(),
                cause: e.to_string(),
            })?;
        drop(payload);

        let mut inner =
            ZipArchive::new(Cursor::new(payload_bytes)).map_err(|e| ContainerError::Read {
                path: nested_info.path.clone(),
                cause: e.to_string(),
            })?;

        let mut decrypted_entries = Vec::new();
        for i in 0..inner.len() {
            // `by_index_raw` returns the entry's bytes exactly as stored —
            // still encrypted, still compressed. The `zip` crate's own
            // decrypt-and-decompress path is not usable here: it is fused
            // to its own crypto (which for AES needs a feature this
            // workspace does not enable, and for ZipCrypto this task
            // deliberately bypasses in favour of `knx_secure::zipcrypto`),
            // so container mechanics (this loop) and cryptography
            // (`knx_secure`) stay on their own sides of the boundary.
            let mut entry = inner.by_index_raw(i).map_err(|e| ContainerError::Read {
                path: nested_info.path.clone(),
                cause: e.to_string(),
            })?;
            if entry.is_dir() {
                continue;
            }
            // The nested payload already stores paths rooted the same way
            // the outer archive would for an unprotected project (e.g.
            // `P-0001/Project.xml`, not bare `Project.xml`) — matches
            // both this crate's fixture and real ETS output structure.
            let entry_path = entry.name().to_string();

            if !entry.encrypted() {
                // Not every entry in a protected payload need be
                // encrypted in principle; none observed so far are not,
                // but nothing here assumes otherwise. Read it plain.
                let size = entry.size();
                if size > MAX_ENTRY_SIZE {
                    return Err(ContainerError::Read {
                        path: entry_path,
                        cause: format!(
                            "declared uncompressed size {size} bytes exceeds the \
                             {MAX_ENTRY_SIZE}-byte limit"
                        ),
                    });
                }
                let mut raw =
                    Vec::with_capacity(entry.compressed_size().min(MAX_ENTRY_SIZE) as usize);
                entry
                    .read_to_end(&mut raw)
                    .map_err(|e| ContainerError::Read {
                        path: entry_path.clone(),
                        cause: e.to_string(),
                    })?;
                let plain = decompress(entry.compression(), &raw, entry_path.clone())?;
                container.entries.push(EntryInfo {
                    path: entry_path.clone(),
                    size: plain.len() as u64,
                });
                container.decrypted.insert(entry_path, plain);
                continue;
            }

            if entry.compression() == CompressionMethod::AES {
                return Err(ContainerError::UnsupportedEncryption {
                    nested_entry: nested.clone(),
                    scheme: "AES (WinZip AES / schema ≥ 21)".to_string(),
                });
            }

            let check = CheckBytes {
                crc32_high_byte: (entry.crc32() >> 24) as u8,
                dos_time_high_byte: entry
                    .last_modified()
                    .map(|dt| (dt.timepart() >> 8) as u8)
                    .unwrap_or(0),
            };
            let declared_size = entry.size();

            let mut raw = Vec::with_capacity(entry.compressed_size().min(MAX_ENTRY_SIZE) as usize);
            entry
                .read_to_end(&mut raw)
                .map_err(|e| ContainerError::Read {
                    path: entry_path.clone(),
                    cause: e.to_string(),
                })?;

            if declared_size > MAX_ENTRY_SIZE {
                return Err(ContainerError::Read {
                    path: entry_path,
                    cause: format!(
                        "declared uncompressed size {declared_size} bytes exceeds the \
                         {MAX_ENTRY_SIZE}-byte limit"
                    ),
                });
            }

            let compressed =
                zipcrypto::decrypt(password.as_bytes(), &raw, check).map_err(|e| match e {
                    ZipCryptoError::WrongPassword => ContainerError::WrongPassword {
                        nested_entry: nested.clone(),
                    },
                    ZipCryptoError::TruncatedHeader { len } => ContainerError::Read {
                        path: entry_path.clone(),
                        cause: format!(
                            "ZipCrypto-encrypted entry is only {len} bytes, \
                             shorter than the 12-byte encryption header"
                        ),
                    },
                })?;

            let plain = decompress(entry.compression(), &compressed, entry_path.clone())?;
            decrypted_entries.push((entry_path, plain));
        }

        // The opaque nested-zip blob is replaced by the entries it
        // actually contains, so `Container::entries`/`read` treat a
        // decrypted project exactly like an unprotected one.
        container.entries.retain(|e| e.path != nested_info.path);
        for (path, plain) in decrypted_entries {
            container.entries.push(EntryInfo {
                path: path.clone(),
                size: plain.len() as u64,
            });
            container.decrypted.insert(path, plain);
        }

        Ok(container)
    }

    /// Shared by [`Container::open`] and [`Container::open_with_password`]:
    /// parses the outer archive and its inventory, without deciding what to
    /// do about a password-protected nested payload — that decision is
    /// each caller's alone.
    fn open_raw(bytes: Vec<u8>) -> Result<Self, ContainerError> {
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

        Ok(Self {
            archive,
            entries,
            decrypted: BTreeMap::new(),
        })
    }

    pub fn entries(&self) -> &[EntryInfo] {
        &self.entries
    }

    /// Reads one entry's bytes. Looked up case-insensitively via `find`, so
    /// callers never have to know the generation's filename casing.
    ///
    /// An entry that came from a ZipCrypto-decrypted payload
    /// ([`Container::open_with_password`]) is already plaintext in
    /// `decrypted` by the time this runs — decryption happens once, at
    /// open time, never per-read.
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

        if let Some(plain) = self.decrypted.get(&real_path) {
            return Ok(plain.clone());
        }

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

/// Decompresses one already-decrypted ZipCrypto entry's bytes.
///
/// The `zip` crate's own decompression is fused to its own decryption
/// (see the note in [`Container::open_with_password`]), so once
/// `knx_secure::zipcrypto` has stripped the encryption layer, this crate
/// has to finish the job itself. Every entry seen in either reference
/// project and both fixtures is Stored or Deflated; anything else is
/// reported, not guessed at.
fn decompress(
    method: CompressionMethod,
    bytes: &[u8],
    path: String,
) -> Result<Vec<u8>, ContainerError> {
    match method {
        CompressionMethod::Stored => Ok(bytes.to_vec()),
        CompressionMethod::Deflated => {
            let mut out = Vec::new();
            DeflateDecoder::new(bytes)
                .read_to_end(&mut out)
                .map_err(|e| ContainerError::Read {
                    path,
                    cause: e.to_string(),
                })?;
            Ok(out)
        }
        other => Err(ContainerError::Read {
            path,
            cause: format!("unsupported compression method for a decrypted entry: {other:?}"),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::io::Write;

    fn reference_ets4_bytes() -> Vec<u8> {
        std::fs::read(knx_testsupport::reference_ets4_path())
            .expect("reference ETS4 project lives under OriginalData/ (gitignored); see knx_testsupport::corpus_available")
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
        if !crate::testutil::corpus_available() {
            eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
            return;
        }
        let mut c = Container::open(reference_ets4_bytes()).unwrap();
        assert_eq!(c.entries().len(), 38);
        // Measured: unlike the per-installation `0.xml` files, ETS4 writes
        // `knx_master.xml` with no UTF-8 BOM and no XML declaration — it
        // starts directly with the root element.
        assert!(c.read("knx_master.xml").unwrap().starts_with(b"<KNX"));
    }

    #[test]
    fn entry_lookup_is_case_insensitive() {
        if !crate::testutil::corpus_available() {
            eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
            return;
        }
        let c = Container::open(reference_ets4_bytes()).unwrap();
        assert_eq!(
            c.find("p-0512/project.xml").map(|e| e.path.as_str()),
            Some("P-0512/Project.xml")
        );
    }

    #[test]
    fn the_project_part_comes_from_the_signature_filename() {
        if !crate::testutil::corpus_available() {
            eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
            return;
        }
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

    // Fixture: `fixtures/zipcrypto-protected.knxproj`, generated with the
    // Info-ZIP `zip` CLI (never by any code path in this repository — see
    // this crate's fixture-generation notes) as a nested-container-shaped
    // stand-in for a real ETS4/ETS5 password-protected `.knxproj`: an
    // outer unencrypted zip containing `knx_master.xml`,
    // `P-0001.signature`, and a ZipCrypto-encrypted `P-0001.zip` (itself
    // containing `P-0001/Project.xml` and `P-0001/0.xml`, Deflate
    // compressed), password `hunter2knx`. Synthetic, not a real project
    // export: the ZipCrypto algorithm is fully specified by APPNOTE.TXT
    // and does not vary by the tool that wrote it, so a fixture generated
    // by a standard, independent ZIP tool tests the algorithm this crate
    // implements, not any one vendor's quirks — see `knx_secure::zipcrypto`
    // for the one quirk (the check-byte convention) that *does* vary by
    // writer, and how this crate handles it either way.
    fn zipcrypto_protected_fixture_bytes() -> Vec<u8> {
        include_bytes!("../fixtures/zipcrypto-protected.knxproj").to_vec()
    }

    #[test]
    fn the_right_password_decrypts_the_zipcrypto_fixture() {
        let mut c =
            Container::open_with_password(zipcrypto_protected_fixture_bytes(), "hunter2knx")
                .expect("known-good password must decrypt the fixture");
        assert_eq!(c.project_part().unwrap(), "P-0001");
        let project_xml = c.read("P-0001/Project.xml").unwrap();
        assert!(project_xml.starts_with(b"<?xml"));
        assert!(String::from_utf8_lossy(&project_xml).contains("<Project"));
        assert!(
            c.find("P-0001.zip").is_none(),
            "the opaque nested payload must be replaced by its decrypted entries"
        );
    }

    #[test]
    fn a_wrong_password_is_a_typed_error_not_a_panic_or_garbage() {
        let result = Container::open_with_password(zipcrypto_protected_fixture_bytes(), "not it");
        assert!(
            matches!(result, Err(ContainerError::WrongPassword { .. })),
            "wrong password must not silently succeed"
        );
    }

    #[test]
    fn open_with_password_on_an_unprotected_project_behaves_like_open() {
        // Not-encrypted case: a password-aware open on a plain project
        // must succeed exactly as `Container::open` does, so a caller
        // that does not yet know whether a project is protected can call
        // `open_with_password` unconditionally.
        let bytes = zip_with_entries(&[
            ("P-0512.signature", b"x"),
            ("P-0512/Project.xml", b"<KNX/>"),
        ]);
        let mut c = Container::open_with_password(bytes, "irrelevant")
            .expect("an unprotected project must open regardless of the password given");
        assert_eq!(c.project_part().unwrap(), "P-0512");
        assert_eq!(c.read("P-0512/Project.xml").unwrap(), b"<KNX/>");
    }
}
