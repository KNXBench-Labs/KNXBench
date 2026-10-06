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
//!
//! AES is detected from the entry's raw on-disk compression-method field
//! (`raw_compression_method`), not from `zip`'s own parsed
//! `CompressionMethod` — the `zip` crate overwrites that field with the
//! entry's real underlying method the moment it parses a WinZip AES extra
//! field (0x9901), so checking the parsed value can never see an AES
//! entry at all.

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

/// The most uncompressed bytes one archive may declare across all its
/// members (AR18 review F3). Every member is read with at most its declared
/// size, so this bounds what an import holds in memory from the archive.
/// The reference projects declare at most 22 MiB in total; 512 MiB leaves
/// room for large real projects while keeping a crafted archive of a few
/// megabytes from driving the process to gigabytes.
pub const MAX_ARCHIVE_UNCOMPRESSED: u64 = 512 * 1024 * 1024;

/// The on-disk compression-method value APPNOTE §4.4.5 reserves to mean
/// "see the WinZip AES extra field (0x9901) for the entry's real method".
/// `zip` calls this `CompressionMethod::Unsupported(99)` internally but,
/// per this module's header, overwrites it with the *real* underlying
/// method the moment it parses that extra field — so this constant is only
/// ever checked against the raw bytes below, never against
/// [`zip::read::ZipFile::compression`] (finding 1 of the T15 branch
/// review).
const AES_RAW_COMPRESSION_METHOD: u16 = 99;

/// Reads the raw on-disk compression-method field (APPNOTE §4.4.5,
/// local-file-header offset 8, a `u16` little-endian) straight out of
/// `payload`, bypassing `zip`'s own parsed [`CompressionMethod`], which a
/// WinZip AES extra field (0x9901) overwrites with the real underlying
/// method the instant `zip` parses it. `header_start` is
/// [`zip::read::ZipFile::header_start`]; `payload` must be the exact bytes
/// the enclosing `ZipArchive` was built from. `None` if the offset falls
/// outside `payload` — treated as "not AES" by the caller, same as any
/// other value that isn't 99.
fn raw_compression_method(payload: &[u8], header_start: u64) -> Option<u16> {
    let offset = usize::try_from(header_start).ok()?.checked_add(8)?;
    let bytes = payload.get(offset..offset.checked_add(2)?)?;
    Some(u16::from_le_bytes([bytes[0], bytes[1]]))
}

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
    /// The nested payload `decrypted` came from (e.g. `P-0001.zip`), set
    /// together with it by [`Container::open_with_password`].
    decrypted_from: Option<String>,
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
        scheme: EncryptionScheme,
    },
    /// A nested payload entry's path already exists elsewhere in the
    /// container's inventory — either the outer archive or an entry
    /// already committed from this same payload. Never silently shadowed
    /// (finding 6 of the T15 branch review): whichever of the two a
    /// caller reads by path, the other one's bytes would vanish with no
    /// warning.
    DuplicateEntry {
        path: String,
    },
    /// The archive has more central-directory records than the `zip` reader
    /// keeps members: two records decode to one name (an Info-ZIP Unicode
    /// Path field, or CP437 against UTF-8), so one would stand in for the
    /// other (AR18 re-check N1).
    NameCollision {
        records: usize,
        members: usize,
    },
    /// The members together declare more uncompressed bytes than
    /// [`MAX_ARCHIVE_UNCOMPRESSED`] (AR18 review F3).
    TooLarge {
        total: u64,
        limit: u64,
    },
}

/// Which encryption scheme a nested payload turned out to be protected
/// with, once its raw on-disk compression-method field (not `zip`'s parsed
/// [`CompressionMethod`], which a WinZip AES extra field overwrites — see
/// this module's header) named it. A type, not a string, so a caller can
/// `match` it instead of comparing English prose (finding 12 of the T15
/// branch review) — even though there is, for now, exactly one variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncryptionScheme {
    /// WinZip AES (`AE-x`, APPNOTE's compression-method 99 + extra field
    /// 0x9901) — the scheme schema ≥ 21 (ETS6) `.knxproj` files use.
    Aes,
}

impl std::fmt::Display for EncryptionScheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EncryptionScheme::Aes => write!(f, "AES (WinZip AES / schema ≥ 21)"),
        }
    }
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
                "wrong password for password-protected project (nested payload {nested_entry}); \
                 a damaged encrypted entry is reported the same way"
            ),
            ContainerError::UnsupportedEncryption {
                nested_entry,
                scheme,
            } => write!(
                f,
                "project is protected with {scheme}, which this build does not decrypt \
                 (nested payload {nested_entry})"
            ),
            ContainerError::DuplicateEntry { path } => {
                write!(f, "duplicate entry path across the container: {path}")
            }
            ContainerError::TooLarge { total, limit } => write!(
                f,
                "the archive declares {total} uncompressed bytes, more than the \
                 {limit}-byte limit"
            ),
            ContainerError::NameCollision { records, members } => write!(
                f,
                "the archive has {records} member records, but they decode to only \
                 {members} distinct names; one member would stand in for another"
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
        let payload_size = payload.size();
        let payload_bytes = read_declared(&mut payload, payload_size, nested_info.path.clone())?;
        drop(payload);

        // Borrows `payload_bytes` rather than consuming it, so the raw
        // on-disk local-file-header bytes stay available below for the AES
        // check `zip`'s own parsed `CompressionMethod` cannot answer (see
        // `raw_compression_method` and this module's header) — `inner`
        // never outlives this function, so a borrow is enough; no need to
        // clone a buffer that can be up to `MAX_ENTRY_SIZE` bytes.
        let mut inner = ZipArchive::new(Cursor::new(payload_bytes.as_slice())).map_err(|e| {
            ContainerError::Read {
                path: nested_info.path.clone(),
                cause: e.to_string(),
            }
        })?;
        // Same identity rule as the outer archive (AR18 review F2): the
        // reader above has already collapsed an exact duplicate.
        let records = check_member_names(&payload_bytes, inner.central_directory_start())?;
        check_decoded_names(&mut inner, records)?;

        // The opaque nested-zip blob is about to be replaced by the
        // entries it actually contains. Removed here, before the loop
        // below, rather than after: each entry's collision check
        // (`container.find`, just below) then sees every entry already
        // committed for this payload as well as the outer archive's own
        // inventory, including entries pushed by earlier iterations of
        // this same loop (finding 6 of the T15 branch review).
        container.entries.retain(|e| e.path != nested_info.path);

        // The budget is judged on what the payload *declares*, before any
        // member is unpacked (AR18 re-check N2): checked only after the
        // loop, a small file could make it unpack gigabytes first. Each
        // member is then read with at most its declared size, so the
        // declarations are binding.
        let mut declared = container.entries.clone();
        for i in 0..inner.len() {
            let entry = inner.by_index_raw(i).map_err(|e| ContainerError::Read {
                path: nested_info.path.clone(),
                cause: e.to_string(),
            })?;
            if !entry.is_dir() {
                declared.push(EntryInfo {
                    path: entry.name().to_string(),
                    size: entry.size(),
                });
            }
        }
        check_total_size(&declared)?;
        drop(declared);

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

            if container.find(&entry_path).is_some() {
                return Err(ContainerError::DuplicateEntry { path: entry_path });
            }

            if !entry.encrypted() {
                // Not every entry in a protected payload need be
                // encrypted in principle; none observed so far are not,
                // but nothing here assumes otherwise. Read it plain.
                let declared_size = entry.size();
                if declared_size > MAX_ENTRY_SIZE {
                    return Err(ContainerError::Read {
                        path: entry_path,
                        cause: format!(
                            "declared uncompressed size {declared_size} bytes exceeds the \
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
                let plain =
                    decompress(entry.compression(), &raw, declared_size, entry_path.clone())?;
                // Which of the two failed is diagnostic, not decoration:
                // a length mismatch points at a truncated or padded
                // archive, a CRC mismatch at bytes that were altered
                // while keeping their length. Say which.
                if plain.len() as u64 != declared_size {
                    return Err(ContainerError::Read {
                        path: entry_path,
                        cause: format!(
                            "decompressed to {} bytes, but the archive declares {declared_size}",
                            plain.len()
                        ),
                    });
                }
                let actual_crc = zipcrypto::crc32(&plain);
                if actual_crc != entry.crc32() {
                    return Err(ContainerError::Read {
                        path: entry_path,
                        cause: format!(
                            "decompressed bytes have CRC-32 {actual_crc:#010x}, but the archive \
                             declares {:#010x}",
                            entry.crc32()
                        ),
                    });
                }
                container.entries.push(EntryInfo {
                    path: entry_path.clone(),
                    size: plain.len() as u64,
                });
                container.decrypted.insert(entry_path, plain);
                continue;
            }

            // `zip` overwrites `entry.compression()` with the real
            // underlying method the instant it parses a WinZip AES extra
            // field (0x9901), so checking the parsed `CompressionMethod`
            // for `AES` here can never fire (finding 1 of the T15 branch
            // review) — the raw on-disk field at the local file header's
            // compression-method offset still says 99 either way.
            if raw_compression_method(&payload_bytes, entry.header_start())
                == Some(AES_RAW_COMPRESSION_METHOD)
            {
                return Err(ContainerError::UnsupportedEncryption {
                    nested_entry: nested.clone(),
                    scheme: EncryptionScheme::Aes,
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
            if declared_size > MAX_ENTRY_SIZE {
                return Err(ContainerError::Read {
                    path: entry_path,
                    cause: format!(
                        "declared uncompressed size {declared_size} bytes exceeds the \
                         {MAX_ENTRY_SIZE}-byte limit"
                    ),
                });
            }

            let mut raw = Vec::with_capacity(entry.compressed_size().min(MAX_ENTRY_SIZE) as usize);
            entry
                .read_to_end(&mut raw)
                .map_err(|e| ContainerError::Read {
                    path: entry_path.clone(),
                    cause: e.to_string(),
                })?;

            let compressed =
                zipcrypto::decrypt(password.as_bytes(), &raw, check).map_err(|e| match e {
                    ZipCryptoError::WrongPassword => ContainerError::WrongPassword {
                        nested_entry: nested.clone(),
                    },
                    ZipCryptoError::TruncatedHeader { len } => ContainerError::Read {
                        path: entry_path.clone(),
                        cause: format!(
                            "ZipCrypto-encrypted entry is only {len} bytes, \
                             shorter than the {}-byte encryption header",
                            zipcrypto::HEADER_LEN
                        ),
                    },
                })?;

            // A wrong password that slipped past the check byte decrypts
            // to noise, and noise is almost never a valid deflate stream:
            // inflating fails before the CRC below is ever reached. That
            // failure is the same false accept the CRC check catches, so
            // it is reported the same way, as a wrong password, not as a
            // damaged archive (AR08). A method this reader does not
            // support is still its own error, whatever the password.
            let method = entry.compression();
            let plain = match decompress(method, &compressed, declared_size, entry_path.clone()) {
                Ok(plain) => plain,
                Err(_) if method == CompressionMethod::Deflated => {
                    return Err(ContainerError::WrongPassword {
                        nested_entry: nested.clone(),
                    });
                }
                Err(other) => return Err(other),
            };

            // The check byte (APPNOTE §6.1.6) only rules out about 255 of
            // 256 wrong passwords per convention it is tried against — see
            // `knx_secure::zipcrypto`'s module docs. The entry's own
            // CRC-32, already in hand from the central directory, rules
            // out essentially all the rest: a mismatch here, immediately
            // after a check byte that already passed, is the false accept
            // that check byte cannot catch (finding 2 of the T15 branch
            // review) — the wrong password proceeded far enough to
            // decompress cleanly but produced the wrong bytes. Reported
            // the same way an outright-wrong check byte is, not as
            // ordinary data corruption.
            if plain.len() as u64 != declared_size || zipcrypto::crc32(&plain) != entry.crc32() {
                return Err(ContainerError::WrongPassword {
                    nested_entry: nested.clone(),
                });
            }

            container.entries.push(EntryInfo {
                path: entry_path.clone(),
                size: plain.len() as u64,
            });
            container.decrypted.insert(entry_path, plain);
        }

        if !container.decrypted.is_empty() {
            container.decrypted_from = Some(nested);
        }
        check_total_size(&container.entries)?;
        Ok(container)
    }

    /// Shared by [`Container::open`] and [`Container::open_with_password`]:
    /// parses the outer archive and its inventory, without deciding what to
    /// do about a password-protected nested payload — that decision is
    /// each caller's alone.
    fn open_raw(bytes: Vec<u8>) -> Result<Self, ContainerError> {
        // Names are checked on the raw central directory first: the `zip`
        // crate keys members by name, so an exact duplicate is already
        // collapsed into one slot by the time its index can be asked.
        let records = {
            let probe = ZipArchive::new(Cursor::new(bytes.as_slice()))
                .map_err(|e| ContainerError::NotAZip(e.to_string()))?;
            check_member_names(&bytes, probe.central_directory_start())?
        };
        let mut archive = ZipArchive::new(Cursor::new(bytes))
            .map_err(|e| ContainerError::NotAZip(e.to_string()))?;
        check_decoded_names(&mut archive, records)?;

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
        check_total_size(&entries)?;

        Ok(Self {
            archive,
            entries,
            decrypted: BTreeMap::new(),
            decrypted_from: None,
        })
    }

    pub fn entries(&self) -> &[EntryInfo] {
        &self.entries
    }

    /// Whether any entry in this container came from a decrypted nested
    /// payload ([`Container::open_with_password`]), rather than straight
    /// from the outer archive. Always `false` for [`Container::open`] and
    /// for an unprotected project opened via `open_with_password`.
    ///
    /// A decrypted project has no roundtrip claim yet
    /// (`KNOWN_LIMITATIONS.md` §13): the opaque passthrough store
    /// (ADR-0006) reads every entry back through
    /// [`entries`](Container::entries)/[`read`](Container::read), which
    /// cannot tell a decrypted entry from one the caller wrote plaintext,
    /// and the original ZipCrypto ciphertext is not kept anywhere once
    /// decryption has run. A later import stage can call this to decide
    /// whether to warn the caller before writing such a project back out
    /// (finding 7 of the T15 branch review).
    pub fn was_decrypted(&self) -> bool {
        !self.decrypted.is_empty()
    }

    /// The nested payload this container's decrypted entries came from
    /// (e.g. `P-0001.zip`); `None` exactly when
    /// [`was_decrypted`](Container::was_decrypted) is `false`.
    pub fn decrypted_payload(&self) -> Option<&str> {
        self.decrypted_from.as_deref()
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
        let declared = entry.size();
        read_declared(&mut entry, declared, real_path)
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

    /// Every other `P-xxxx` part the archive carries — by signature, nested
    /// payload or top-level folder — besides the one [`Self::project_part`]
    /// imports. Sorted and deduplicated; empty for an ordinary archive (AR18
    /// review M3: such parts were kept as opaque entries but never named).
    pub fn other_project_parts(&self) -> Vec<String> {
        let Some(imported) = self.project_part_opt() else {
            return Vec::new();
        };
        let mut parts: Vec<String> = self
            .entries
            .iter()
            .filter_map(|entry| {
                let path = entry.path.as_str();
                let top = match path.split_once('/') {
                    Some((dir, _)) => dir,
                    None => path
                        .strip_suffix(".signature")
                        .or_else(|| path.strip_suffix(".zip"))?,
                };
                top.starts_with("P-").then(|| top.to_string())
            })
            .filter(|part| !part.eq_ignore_ascii_case(imported))
            .collect();
        parts.sort();
        parts.dedup();
        parts
    }

    fn project_part_opt(&self) -> Option<&str> {
        self.entries.iter().find_map(|e| {
            let name = e.path.rsplit('/').next().unwrap_or(&e.path);
            name.strip_suffix(".signature")
                .filter(|stem| stem.starts_with("P-"))
        })
    }
}

/// Reads one member, never more than its `declared` uncompressed size (AR18
/// review F3). The `zip` reader checks a deflated member's CRC-32 but not
/// its length, so a member whose headers declare 100 bytes could otherwise
/// inflate to gigabytes; one byte past the declaration is read only to
/// tell "exactly as declared" from "more".
fn read_declared(
    entry: &mut impl Read,
    declared: u64,
    path: String,
) -> Result<Vec<u8>, ContainerError> {
    let mut buf = Vec::with_capacity(declared.min(MAX_ENTRY_SIZE) as usize);
    entry
        .take(declared.saturating_add(1))
        .read_to_end(&mut buf)
        .map_err(|e| ContainerError::Read {
            path: path.clone(),
            cause: e.to_string(),
        })?;
    if buf.len() as u64 > declared {
        return Err(ContainerError::Read {
            path,
            cause: format!("inflated to more than the declared {declared} bytes"),
        });
    }
    Ok(buf)
}

/// Refuses an archive whose members together declare more than
/// [`MAX_ARCHIVE_UNCOMPRESSED`] bytes (AR18 review F3).
fn check_total_size(entries: &[EntryInfo]) -> Result<(), ContainerError> {
    let total = entries
        .iter()
        .fold(0u64, |sum, entry| sum.saturating_add(entry.size));
    if total > MAX_ARCHIVE_UNCOMPRESSED {
        return Err(ContainerError::TooLarge {
            total,
            limit: MAX_ARCHIVE_UNCOMPRESSED,
        });
    }
    Ok(())
}

/// Walks the central directory at `start` record by record and refuses a
/// second file member with the same name, byte for byte or ignoring ASCII
/// case (AR18 review F2). Lookups here are case-insensitive (`find`), so
/// either kind of repeat would let one member's bytes stand in for the
/// other's without a word. Directory records carry no data and are skipped.
fn check_member_names(bytes: &[u8], start: u64) -> Result<usize, ContainerError> {
    const RECORD: usize = 46;
    let field = |at: usize| -> Option<usize> {
        bytes
            .get(at..at + 2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]) as usize)
    };
    let mut seen = std::collections::HashSet::new();
    let mut records = 0;
    let mut at = usize::try_from(start).unwrap_or(usize::MAX);
    while bytes.get(at..at + 4) == Some(b"PK\x01\x02".as_slice()) {
        let (Some(name_len), Some(extra_len), Some(comment_len)) =
            (field(at + 28), field(at + 30), field(at + 32))
        else {
            break;
        };
        let Some(name) = bytes.get(at + RECORD..at + RECORD + name_len) else {
            break;
        };
        if !name.ends_with(b"/") && !seen.insert(name.to_ascii_lowercase()) {
            return Err(ContainerError::DuplicateEntry {
                path: String::from_utf8_lossy(name).into_owned(),
            });
        }
        records += 1;
        at += RECORD + name_len + extra_len + comment_len;
    }
    Ok(records)
}

/// The second half of the identity rule (AR18 re-check N1): the names the
/// `zip` reader actually serves. It decodes a name as CP437 or UTF-8 and
/// may replace it with an Info-ZIP Unicode Path field, then keys members by
/// the result — so two records with different raw names can still become
/// one member, or two members with one name. Refused when the reader keeps
/// fewer members than [`check_member_names`] counted `records`, or when two
/// decoded file names are equal ignoring ASCII case (what [`Container::find`]
/// would confuse).
fn check_decoded_names<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
    records: usize,
) -> Result<(), ContainerError> {
    if archive.len() != records {
        return Err(ContainerError::NameCollision {
            records,
            members: archive.len(),
        });
    }
    let mut seen = std::collections::HashSet::new();
    for i in 0..archive.len() {
        let entry = archive
            .by_index_raw(i)
            .map_err(|e| ContainerError::NotAZip(e.to_string()))?;
        if entry.is_dir() {
            continue;
        }
        if !seen.insert(entry.name().to_ascii_lowercase()) {
            return Err(ContainerError::DuplicateEntry {
                path: entry.name().to_string(),
            });
        }
    }
    Ok(())
}

/// Decompresses one nested-payload entry's bytes, called from
/// [`Container::open_with_password`] for both a ZipCrypto-decrypted entry
/// and a plain (never-encrypted) one found inside the same protected
/// payload — nothing here cares which.
///
/// The `zip` crate's own decompression is fused to its own decryption
/// (see the note in [`Container::open_with_password`]), so once
/// `knx_secure::zipcrypto` has stripped the encryption layer, this crate
/// has to finish the job itself. Every entry seen in either reference
/// project and both fixtures is Stored or Deflated; anything else is
/// reported, not guessed at. `declared_size` is the entry's own
/// central-directory uncompressed-size field: inflating is bounded by it
/// (plus one byte, so an entry that inflates to exactly `declared_size`
/// does not itself trip the bound) rather than left to run until memory
/// runs out (finding 5 of the T15 branch review) — the caller still
/// re-checks the *actual* decompressed length and CRC-32 against the
/// entry afterward, this bound only stops the decoder from running away.
fn decompress(
    method: CompressionMethod,
    bytes: &[u8],
    declared_size: u64,
    path: String,
) -> Result<Vec<u8>, ContainerError> {
    match method {
        CompressionMethod::Stored => Ok(bytes.to_vec()),
        CompressionMethod::Deflated => {
            let limit = declared_size.saturating_add(1);
            let mut out = Vec::new();
            DeflateDecoder::new(bytes)
                .take(limit)
                .read_to_end(&mut out)
                .map_err(|e| ContainerError::Read {
                    path: path.clone(),
                    cause: e.to_string(),
                })?;
            if out.len() as u64 > declared_size {
                return Err(ContainerError::Read {
                    path,
                    cause: format!("inflated to more than the declared {declared_size} bytes"),
                });
            }
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
    #[ignore = "requires the gitignored OriginalData/ corpus; run with --ignored"]
    fn the_reference_project_inventory_has_thirty_eight_entries() {
        assert!(
            crate::testutil::corpus_available(),
            "OriginalData/ corpus not present (gitignored, local-only); this test is #[ignore]d and must be run explicitly on a machine that has it"
        );
        let mut c = Container::open(reference_ets4_bytes()).unwrap();
        assert_eq!(c.entries().len(), 38);
        // Measured: unlike the per-installation `0.xml` files, ETS4 writes
        // `knx_master.xml` with no UTF-8 BOM and no XML declaration — it
        // starts directly with the root element.
        assert!(c.read("knx_master.xml").unwrap().starts_with(b"<KNX"));
    }

    #[test]
    #[ignore = "requires the gitignored OriginalData/ corpus; run with --ignored"]
    fn entry_lookup_is_case_insensitive() {
        assert!(
            crate::testutil::corpus_available(),
            "OriginalData/ corpus not present (gitignored, local-only); this test is #[ignore]d and must be run explicitly on a machine that has it"
        );
        let c = Container::open(reference_ets4_bytes()).unwrap();
        assert_eq!(
            c.find("p-0512/project.xml").map(|e| e.path.as_str()),
            Some("P-0512/Project.xml")
        );
    }

    #[test]
    #[ignore = "requires the gitignored OriginalData/ corpus; run with --ignored"]
    fn the_project_part_comes_from_the_signature_filename() {
        assert!(
            crate::testutil::corpus_available(),
            "OriginalData/ corpus not present (gitignored, local-only); this test is #[ignore]d and must be run explicitly on a machine that has it"
        );
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

    /// Builds a minimal one-entry ZIP whose entry is WinZip AES protected:
    /// raw compression method 99 in both headers, general-purpose bit 0
    /// set, and a 0x9901 extra field naming the real underlying method.
    /// Hand-assembled from APPNOTE §4.3.7, §4.3.12 and §4.3.16 rather
    /// than written by a library, for two reasons: nothing in this
    /// workspace's dependency tree can *write* AES, and the whole point
    /// of the fixture is the raw on-disk method field that `zip`
    /// overwrites the instant it reads the extra field. The entry's
    /// "ciphertext" is arbitrary — no code path is ever allowed to reach
    /// it, which is exactly what the test asserts.
    fn zip_with_one_aes_entry(name: &str, data: &[u8]) -> Vec<u8> {
        // APPNOTE §4.5's WinZip AES extra field: header id 0x9901 and 7
        // data bytes — AE version 2, vendor "AE", strength 3 (AES-256),
        // then the compression method the entry would have had without
        // encryption (0, Stored).
        let extra: [u8; 11] = [
            0x01, 0x99, 0x07, 0x00, 0x02, 0x00, b'A', b'E', 0x03, 0x00, 0x00,
        ];
        let name_bytes = name.as_bytes();
        let size = data.len() as u32;

        let mut local = Vec::new();
        local.extend_from_slice(&0x0403_4b50u32.to_le_bytes()); // local file header signature
        local.extend_from_slice(&20u16.to_le_bytes()); // version needed to extract
        local.extend_from_slice(&1u16.to_le_bytes()); // general purpose flags: bit 0, encrypted
        local.extend_from_slice(&AES_RAW_COMPRESSION_METHOD.to_le_bytes());
        local.extend_from_slice(&0u16.to_le_bytes()); // last modified time
        local.extend_from_slice(&0x21u16.to_le_bytes()); // last modified date (1980-01-01)
        local.extend_from_slice(&0u32.to_le_bytes()); // CRC-32 (AE-2 writes zero)
        local.extend_from_slice(&size.to_le_bytes()); // compressed size
        local.extend_from_slice(&size.to_le_bytes()); // uncompressed size
        local.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
        local.extend_from_slice(&(extra.len() as u16).to_le_bytes());
        local.extend_from_slice(name_bytes);
        local.extend_from_slice(&extra);
        local.extend_from_slice(data);
        let central_offset = local.len() as u32;

        let mut central = Vec::new();
        central.extend_from_slice(&0x0201_4b50u32.to_le_bytes()); // central directory signature
        central.extend_from_slice(&20u16.to_le_bytes()); // version made by
        central.extend_from_slice(&20u16.to_le_bytes()); // version needed to extract
        central.extend_from_slice(&1u16.to_le_bytes()); // general purpose flags
        central.extend_from_slice(&AES_RAW_COMPRESSION_METHOD.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes()); // last modified time
        central.extend_from_slice(&0x21u16.to_le_bytes()); // last modified date
        central.extend_from_slice(&0u32.to_le_bytes()); // CRC-32
        central.extend_from_slice(&size.to_le_bytes()); // compressed size
        central.extend_from_slice(&size.to_le_bytes()); // uncompressed size
        central.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
        central.extend_from_slice(&(extra.len() as u16).to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes()); // file comment length
        central.extend_from_slice(&0u16.to_le_bytes()); // disk number start
        central.extend_from_slice(&0u16.to_le_bytes()); // internal file attributes
        central.extend_from_slice(&0u32.to_le_bytes()); // external file attributes
        central.extend_from_slice(&0u32.to_le_bytes()); // local header offset
        central.extend_from_slice(name_bytes);
        central.extend_from_slice(&extra);
        let central_len = central.len() as u32;

        let mut out = local;
        out.extend_from_slice(&central);
        out.extend_from_slice(&0x0605_4b50u32.to_le_bytes()); // end of central directory
        out.extend_from_slice(&0u16.to_le_bytes()); // number of this disk
        out.extend_from_slice(&0u16.to_le_bytes()); // disk with the central directory
        out.extend_from_slice(&1u16.to_le_bytes()); // entries in the central directory, this disk
        out.extend_from_slice(&1u16.to_le_bytes()); // entries in the central directory, total
        out.extend_from_slice(&central_len.to_le_bytes());
        out.extend_from_slice(&central_offset.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // archive comment length
        out
    }

    #[test]
    fn an_aes_payload_is_refused_by_name_and_never_blamed_on_the_password() {
        let nested = zip_with_one_aes_entry("P-0001/Project.xml", b"ciphertext, allegedly");
        let bytes = zip_with_entries(&[("P-0001.signature", b"x"), ("P-0001.zip", &nested)]);
        let Err(err) = Container::open_with_password(bytes, "hunter2knx") else {
            panic!("an AES-protected payload must not open, correct password or not");
        };
        assert!(
            matches!(
                err,
                ContainerError::UnsupportedEncryption {
                    scheme: EncryptionScheme::Aes,
                    ..
                }
            ),
            "an AES payload must be refused by name. `zip` overwrites its own parsed \
             compression method with the entry's *underlying* one as soon as it reads the \
             0x9901 extra field, so a check against the parsed value never fires and the \
             ZipCrypto path ends up telling the user their correct password is wrong. \
             Got: {err:?}"
        );
    }

    // Fixture: `fixtures/zipcrypto-stored.knxproj`, generated with the
    // Info-ZIP `zip` CLI, and the same shape as the fixture above in the
    // respects that matter here (outer archive, a `P-NNNN.signature`, a
    // ZipCrypto-encrypted `P-NNNN.zip`, password `hunter2knx`) — with the
    // one difference it exists for: the nested entry is **Stored**, not
    // Deflated. It is also smaller: part `P-0002` rather than `P-0001`,
    // one nested entry rather than two, no directory entry, and no outer
    // `knx_master.xml`. Against a Deflated entry, a wrong
    // password that slips past the check byte produces bytes that fail to
    // inflate, and the failure is reported long before anyone gets to
    // compare a CRC. Stored bytes always "decompress", so the entry's own
    // CRC-32 is the only thing left standing between a false accept and a
    // caller who thinks they have their project back.
    fn zipcrypto_stored_fixture_bytes() -> Vec<u8> {
        include_bytes!("../fixtures/zipcrypto-stored.knxproj").to_vec()
    }

    /// Pulls a fixture's first nested entry out by hand — raw bytes and
    /// check bytes both — exactly the way [`Container::open_with_password`]
    /// does, so a test can ask `knx_secure` directly whether some password
    /// passes the check byte. The container's own result deliberately no
    /// longer answers that question: it reports every wrong password the
    /// same way, which is the point.
    fn first_nested_entry_of(outer_bytes: Vec<u8>, nested_name: &str) -> (Vec<u8>, CheckBytes) {
        let mut outer = ZipArchive::new(Cursor::new(outer_bytes)).unwrap();
        let mut payload = Vec::new();
        outer
            .by_name(nested_name)
            .unwrap()
            .read_to_end(&mut payload)
            .unwrap();
        let mut inner = ZipArchive::new(Cursor::new(payload)).unwrap();
        let mut entry = inner.by_index_raw(0).unwrap();
        let check = CheckBytes {
            crc32_high_byte: (entry.crc32() >> 24) as u8,
            dos_time_high_byte: entry
                .last_modified()
                .map(|dt| (dt.timepart() >> 8) as u8)
                .unwrap_or(0),
        };
        let mut raw = Vec::new();
        entry.read_to_end(&mut raw).unwrap();
        (raw, check)
    }

    #[test]
    fn the_stored_fixture_decrypts_with_the_right_password() {
        let mut c = Container::open_with_password(zipcrypto_stored_fixture_bytes(), "hunter2knx")
            .expect("known-good password must decrypt the Stored fixture");
        assert_eq!(c.project_part().unwrap(), "P-0002");
        assert!(c.read("P-0002/Project.xml").unwrap().starts_with(b"<?xml"));
        assert!(c.was_decrypted());
    }

    #[test]
    fn a_wrong_password_that_survives_the_check_byte_is_caught_by_the_entrys_crc() {
        let (raw, check) = first_nested_entry_of(zipcrypto_stored_fixture_bytes(), "P-0002.zip");
        // One byte, tried against two conventions: roughly 1 wrong
        // password in 128 walks straight past the check byte (APPNOTE
        // §6.1.6, and `knx_secure::zipcrypto`'s own module docs). Finding
        // one is arithmetic, not luck, and a few hundred candidates are
        // enough — the search is bounded far above that so a fixture
        // change can never turn this test into an infinite loop.
        let lucky = (0..100_000u32)
            .map(|n| format!("wrong{n}"))
            .find(|p| zipcrypto::decrypt(p.as_bytes(), &raw, check).is_ok())
            .expect("a check-byte false accept must exist within 100,000 candidates");
        assert_ne!(lucky, "hunter2knx");

        let result = Container::open_with_password(zipcrypto_stored_fixture_bytes(), &lucky);
        assert!(
            matches!(result, Err(ContainerError::WrongPassword { .. })),
            "a password that passes the check byte but produces the wrong bytes is still a \
             wrong password. Without the entry's own CRC-32 the container hands those bytes \
             on as the project, and the protected path ends up trusted less than the \
             unprotected one, which `zip` CRC-checks for free."
        );
    }

    #[test]
    fn a_nested_entry_colliding_with_an_outer_path_is_refused_not_shadowed() {
        // `knx_master.xml` is the realistic collision: the outer archive
        // carries one and nothing stops a payload from carrying another.
        let nested = zip_with_entries(&[("knx_master.xml", b"<KNX>nested</KNX>")]);
        let bytes = zip_with_entries(&[
            ("P-0001.signature", b"x"),
            ("knx_master.xml", b"<KNX>outer</KNX>"),
            ("P-0001.zip", &nested),
        ]);
        let Err(err) = Container::open_with_password(bytes, "irrelevant") else {
            panic!("a duplicate path must not be accepted: one of the two would vanish");
        };
        assert!(
            matches!(err, ContainerError::DuplicateEntry { ref path } if path == "knx_master.xml"),
            "a colliding path must be named, not silently shadowed by whichever copy the \
             lookup happens to prefer. Got: {err:?}"
        );
    }
}
