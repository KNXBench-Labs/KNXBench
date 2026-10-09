//! The legacy ZIP container: content detection, layout checks, bounded inflate and CRC.
//!
//! This crate never decrypts. For an encrypted member it hands out the raw
//! ZipCrypto stream and its two check bytes; the application layer
//! (`knx_app::legacy`) decrypts with the user's password through
//! `knx-secure` and gives the plaintext back to [`LegacyMember::open_decrypted`].
//! That keeps key material out of every binary that links `knx-productdb`
//! without the application layer (the MCP adapter, ADR-0090).

use std::io::Read;

use flate2::read::DeflateDecoder;

use super::error::LegacyError;
use crate::package::{validated_zip_members, RawZipMember};

/// Largest legacy file accepted. The biggest measured one is the Siemens
/// `.vd5` of November 2016, 67,538,254 bytes (64.4 MiB); see
/// `docs/research/legacy-vd-mapping.md` for the memory measured at that size.
pub const MAX_LEGACY_FILE: usize = 128 * 1024 * 1024;
/// Largest declared payload accepted. The biggest measured one is that
/// `.vd5`'s, 173,230,269 bytes (165.2 MiB).
pub const MAX_LEGACY_PAYLOAD: u64 = 256 * 1024 * 1024;

/// What the member's name says the file is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegacyMemberKind {
    /// `ets.vd_` or `ets2.vd_`.
    ProductDatabase,
    /// `ets.pr_`.
    ProjectExport,
}

impl LegacyMemberKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::ProductDatabase => "product database",
            Self::ProjectExport => "project export",
        }
    }
}

/// The two published ZipCrypto check-byte conventions for one member
/// (APPNOTE §6.1.6 and Info-ZIP's streamed variant); a decryptor accepts
/// the password when the decrypted header's last byte equals either.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LegacyCheckBytes {
    pub crc32_high_byte: u8,
    pub dos_time_high_byte: u8,
}

/// Metadata of a detected legacy container; nothing is decrypted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyContainer {
    /// SHA-256 of the whole file.
    pub sha256: String,
    pub len: usize,
    /// The member name for display: UTF-8 when flagged, otherwise ASCII
    /// with every other byte written as `\xNN` (the legacy code page is not
    /// decoded, so nothing is guessed).
    pub member_name: String,
    pub member_kind: LegacyMemberKind,
    pub encrypted: bool,
    pub method: u16,
    pub compressed_size: u64,
    pub uncompressed_size: u64,
    pub crc32: u32,
    /// Every other member of the file, in directory order. The measured
    /// `.vd5` is an installer tree: three mask images next to `ets.vd_`.
    /// They are not decrypted or read; they stay in the stored original
    /// file and are reported, never dropped silently.
    pub other_members: Vec<LegacyOtherMember>,
}

/// A member next to the EX-IM member, described from ZIP metadata only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyOtherMember {
    /// Display name, written like [`LegacyContainer::member_name`].
    pub name: String,
    pub encrypted: bool,
    pub method: u16,
    pub compressed_size: u64,
    pub uncompressed_size: u64,
}

/// A decrypted and inflated payload whose CRC-32 matched.
#[derive(Debug, Clone)]
pub struct LegacyPayload {
    container: LegacyContainer,
    bytes: Vec<u8>,
}

impl LegacyPayload {
    pub fn container(&self) -> &LegacyContainer {
        &self.container
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Recognises a legacy container from its ZIP metadata alone: a ZIP that
/// passes the package validator and has exactly one member whose base name
/// is `ets.vd_`, `ets2.vd_` or `ets.pr_` (ASCII case-insensitive), whatever
/// other members it has. Never decrypts. `None` for everything else,
/// including oversized input.
pub fn detect_legacy_container(bytes: &[u8]) -> Option<LegacyContainer> {
    locate(bytes).ok().map(|located| located.container)
}

/// A legacy member that passed every container check and is ready to be
/// opened. Borrowing the file keeps the encrypted stream a slice of it.
#[derive(Debug)]
pub struct LegacyMember<'a> {
    container: LegacyContainer,
    stream: &'a [u8],
    check: LegacyCheckBytes,
}

/// Reads the container around a legacy member: the detector's checks, the
/// observed layout (the member records tile the file from offset 0 up to
/// the central directory, with nothing between them), encryption and
/// compression method, and the declared payload size. Decrypts and inflates
/// nothing.
pub fn read_legacy_member(bytes: &[u8]) -> Result<LegacyMember<'_>, LegacyError> {
    let located = locate(bytes)?;
    let member = &located.member;
    if !records_tile(&located.records, located.directory_start) {
        return Err(LegacyError::InvalidContainer {
            reason: "the member records do not start the file and run without a gap to the central directory"
                .into(),
        });
    }
    if member.flags & STRONG_ENCRYPTION_FLAG != 0 {
        return Err(LegacyError::UnsupportedEncryption {
            reason: "strong encryption (general-purpose bit 6)",
        });
    }
    if member.method == AES_METHOD {
        return Err(LegacyError::UnsupportedEncryption {
            reason: "AES (compression method 99)",
        });
    }
    if member.method != STORED && member.method != DEFLATED {
        return Err(LegacyError::UnsupportedCompression {
            method: member.method,
        });
    }
    if member.uncompressed_size > MAX_LEGACY_PAYLOAD {
        return Err(LegacyError::SizeLimit {
            what: "declared payload size in bytes",
            limit: MAX_LEGACY_PAYLOAD,
        });
    }
    let stream = usize::try_from(member.compressed_size)
        .ok()
        .and_then(|len| bytes.get(member.data_start..member.data_start.checked_add(len)?))
        .ok_or_else(|| LegacyError::InvalidContainer {
            reason: "the member's data range lies outside the file".into(),
        })?;
    let check = LegacyCheckBytes {
        crc32_high_byte: (member.crc32 >> 24) as u8,
        dos_time_high_byte: (member.dos_time >> 8) as u8,
    };
    Ok(LegacyMember {
        container: located.container,
        stream,
        check,
    })
}

impl<'a> LegacyMember<'a> {
    pub fn container(&self) -> &LegacyContainer {
        &self.container
    }

    /// The raw ZipCrypto stream (12-byte encryption header plus compressed
    /// data) when the member is encrypted; `None` otherwise.
    pub fn encrypted_stream(&self) -> Option<&'a [u8]> {
        self.container.encrypted.then_some(self.stream)
    }

    pub fn check_bytes(&self) -> LegacyCheckBytes {
        self.check
    }

    /// Opens an unencrypted member; an encrypted one needs a password.
    pub fn open_unencrypted(self) -> Result<LegacyPayload, LegacyError> {
        if self.container.encrypted {
            return Err(LegacyError::PasswordRequired);
        }
        let stream = self.stream;
        self.finish(stream)
    }

    /// Finishes an encrypted member from its decrypted, still compressed
    /// bytes. A failure to inflate or a CRC mismatch is reported as
    /// [`LegacyError::WrongPasswordOrCorrupt`]: after a passed check byte the
    /// two cannot be told apart. On an unencrypted member the argument is
    /// ignored and the member opens as with [`Self::open_unencrypted`].
    pub fn open_decrypted(self, compressed: &[u8]) -> Result<LegacyPayload, LegacyError> {
        if !self.container.encrypted {
            return self.open_unencrypted();
        }
        self.finish(compressed)
    }

    fn finish(self, compressed: &[u8]) -> Result<LegacyPayload, LegacyError> {
        let encrypted = self.container.encrypted;
        let damaged = |reason: &str| {
            if encrypted {
                LegacyError::WrongPasswordOrCorrupt
            } else {
                LegacyError::Corrupt {
                    reason: reason.to_string(),
                }
            }
        };
        let declared = self.container.uncompressed_size;
        // Read at most one byte more than declared, so a member that lies
        // about its size is caught without inflating it completely. The
        // reservation is also capped by what deflate can produce from these
        // bytes (at most 1,032 to 1), so a small file that declares a payload
        // near the bound does not reserve the bound.
        let possible = (compressed.len() as u64).saturating_mul(1032);
        let mut payload = Vec::with_capacity(declared.min(possible) as usize);
        if self.container.method == DEFLATED {
            DeflateDecoder::new(compressed)
                .take(declared + 1)
                .read_to_end(&mut payload)
                .map_err(|_| damaged("the member does not inflate"))?;
        } else {
            let keep = compressed.len().min(declared as usize + 1);
            payload.extend_from_slice(&compressed[..keep]);
        }
        if payload.len() as u64 != declared {
            return Err(damaged("the member does not have its declared size"));
        }
        if crc32fast::hash(&payload) != self.container.crc32 {
            return Err(damaged("CRC-32 mismatch"));
        }
        Ok(LegacyPayload {
            container: self.container,
            bytes: payload,
        })
    }
}

const ENCRYPTED_FLAG: u16 = 0x0001;
const STRONG_ENCRYPTION_FLAG: u16 = 0x0040;
const UTF8_NAME_FLAG: u16 = 0x0800;
const STORED: u16 = 0;
const DEFLATED: u16 = 8;
const AES_METHOD: u16 = 99;
/// Member base names of the observed family (`ets2.vd_` is named by the
/// design study but has no sample).
const PRODUCT_DATABASE_MEMBERS: [&str; 2] = ["ets.vd_", "ets2.vd_"];
const PROJECT_EXPORT_MEMBERS: [&str; 1] = ["ets.pr_"];

struct Located {
    container: LegacyContainer,
    member: RawZipMember,
    /// `(local_offset, record_end)` of every member.
    records: Vec<(usize, usize)>,
    directory_start: usize,
}

/// The observed layout, generalised from one member to several: sorted by
/// offset, the first record starts at 0, each one ends where the next one
/// starts, and the last one ends at the central directory. Nothing can hide
/// between or before them.
fn records_tile(records: &[(usize, usize)], directory_start: usize) -> bool {
    let mut sorted = records.to_vec();
    sorted.sort_unstable();
    let mut at = 0;
    for (start, end) in sorted {
        if start != at {
            return false;
        }
        at = end;
    }
    at == directory_start
}

fn locate(bytes: &[u8]) -> Result<Located, LegacyError> {
    if bytes.len() > MAX_LEGACY_FILE {
        return Err(LegacyError::SizeLimit {
            what: "file size in bytes",
            limit: MAX_LEGACY_FILE as u64,
        });
    }
    let (members, directory_start) =
        validated_zip_members(bytes).map_err(|e| LegacyError::NotLegacyContainer {
            reason: e.to_string(),
        })?;
    let records = members
        .iter()
        .map(|m| (m.local_offset, m.record_end))
        .collect();
    let (mut exim, others): (Vec<_>, Vec<_>) = members
        .into_iter()
        .partition(|m| member_kind(&m.name).is_some());
    if exim.len() != 1 {
        return Err(LegacyError::NotLegacyContainer {
            reason: if exim.is_empty() {
                "no member is named ets.vd_, ets2.vd_ or ets.pr_".into()
            } else {
                format!(
                    "{} members are named ets.vd_, ets2.vd_ or ets.pr_; a legacy file has exactly one",
                    exim.len()
                )
            },
        });
    }
    let member = exim.remove(0);
    let member_kind = member_kind(&member.name).expect("partitioned on it");
    let other_members = others
        .iter()
        .map(|m| LegacyOtherMember {
            name: display_name(&m.name, m.flags & UTF8_NAME_FLAG != 0),
            encrypted: m.flags & ENCRYPTED_FLAG != 0,
            method: m.method,
            compressed_size: m.compressed_size,
            uncompressed_size: m.uncompressed_size,
        })
        .collect();
    let container = LegacyContainer {
        sha256: crate::blob::sha256_hex(bytes),
        len: bytes.len(),
        member_name: display_name(&member.name, member.flags & UTF8_NAME_FLAG != 0),
        member_kind,
        encrypted: member.flags & ENCRYPTED_FLAG != 0,
        method: member.method,
        compressed_size: member.compressed_size,
        uncompressed_size: member.uncompressed_size,
        crc32: member.crc32,
        other_members,
    };
    Ok(Located {
        container,
        member,
        records,
        directory_start,
    })
}

fn member_kind(name: &[u8]) -> Option<LegacyMemberKind> {
    let base = name
        .rsplit(|&b| b == b'/' || b == b'\\')
        .next()
        .unwrap_or_default();
    let is = |candidates: &[&str]| {
        candidates
            .iter()
            .any(|candidate| base.eq_ignore_ascii_case(candidate.as_bytes()))
    };
    if is(&PRODUCT_DATABASE_MEMBERS) {
        Some(LegacyMemberKind::ProductDatabase)
    } else if is(&PROJECT_EXPORT_MEMBERS) {
        Some(LegacyMemberKind::ProjectExport)
    } else {
        None
    }
}

/// UTF-8 when flagged and valid; otherwise printable ASCII as is and every
/// other byte as `\xNN`. The legacy code page is not decoded: the name is
/// for display only and is never used as a path.
fn display_name(name: &[u8], utf8: bool) -> String {
    if utf8 {
        if let Ok(text) = std::str::from_utf8(name) {
            return text.to_string();
        }
    }
    name.iter()
        .map(|&b| {
            if b.is_ascii_graphic() || b == b' ' {
                char::from(b).to_string()
            } else {
                format!("\\x{b:02X}")
            }
        })
        .collect()
}
