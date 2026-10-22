//! PDB-10 baggage inventory: declarations resolved exactly, payloads classified by content.
//!
//! ADR-0042: what each package's `Baggages.xml` declares, which retained
//! `Baggages/` member each declaration names, and what every payload *is*
//! by content, never by extension.
//!
//! Payloads stay opaque. Nothing here executes, renders or extracts one: a
//! payload is sniffed by its leading magic bytes, and a ZIP payload's own
//! central directory is read for entry count, declared expanded size and
//! encryption flags without decompressing a single entry.

use std::collections::BTreeMap;

use rusqlite::{params, Connection, OptionalExtension};

use crate::operation::Cancellation;
use crate::parse::baggage::{declared_member_path, BaggageDeclaration};
use crate::ProductDbError;

/// Leading-bytes classification. `Unknown` is an honest answer, not a
/// fallback to the file extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MediaClass {
    Empty,
    Png,
    Jpeg,
    Gif,
    Bmp,
    Pdf,
    Zip,
    PeExecutable,
    Ole2Compound,
    Xml,
    Unknown,
}

impl MediaClass {
    pub const ALL: [Self; 11] = [
        Self::Empty,
        Self::Png,
        Self::Jpeg,
        Self::Gif,
        Self::Bmp,
        Self::Pdf,
        Self::Zip,
        Self::PeExecutable,
        Self::Ole2Compound,
        Self::Xml,
        Self::Unknown,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Png => "png",
            Self::Jpeg => "jpeg",
            Self::Gif => "gif",
            Self::Bmp => "bmp",
            Self::Pdf => "pdf",
            Self::Zip => "zip",
            Self::PeExecutable => "pe-executable",
            Self::Ole2Compound => "ole2-compound",
            Self::Xml => "xml",
            Self::Unknown => "unknown",
        }
    }

    fn from_db(value: &str) -> Result<Self, ProductDbError> {
        Self::ALL
            .into_iter()
            .find(|class| class.as_str() == value)
            .ok_or_else(|| inventory_error(format!("invalid media class {value:?}")))
    }

    /// Lower-case file extensions this class is conventionally named with,
    /// used only to *report* disagreement, never to classify.
    pub const fn usual_extensions(self) -> &'static [&'static str] {
        match self {
            Self::Png => &["png"],
            Self::Jpeg => &["jpg", "jpeg"],
            Self::Gif => &["gif"],
            Self::Bmp => &["bmp"],
            Self::Pdf => &["pdf"],
            Self::Zip => &["zip"],
            Self::PeExecutable => &["exe", "dll"],
            Self::Ole2Compound => &["msi", "doc", "xls"],
            Self::Xml => &["xml"],
            Self::Empty | Self::Unknown => &[],
        }
    }
}

/// Classifies by magic bytes only. BMP needs its two reserved header words
/// to be zero and PE its `PE\0\0` signature at `e_lfanew`, so a text file
/// starting with `BM` or `MZ` stays `Unknown`.
pub fn sniff_media(bytes: &[u8]) -> MediaClass {
    let starts = |magic: &[u8]| bytes.starts_with(magic);
    if bytes.is_empty() {
        MediaClass::Empty
    } else if starts(b"\x89PNG\r\n\x1a\n") {
        MediaClass::Png
    } else if starts(b"\xFF\xD8\xFF") {
        MediaClass::Jpeg
    } else if starts(b"GIF87a") || starts(b"GIF89a") {
        MediaClass::Gif
    } else if starts(b"BM") && bytes.len() >= 14 && bytes[6..10] == [0, 0, 0, 0] {
        MediaClass::Bmp
    } else if starts(b"%PDF-") {
        MediaClass::Pdf
    } else if starts(b"PK\x03\x04") || starts(b"PK\x05\x06") {
        MediaClass::Zip
    } else if starts(b"MZ") && is_pe(bytes) {
        MediaClass::PeExecutable
    } else if starts(b"\xD0\xCF\x11\xE0\xA1\xB1\x1A\xE1") {
        MediaClass::Ole2Compound
    } else if starts(b"<?xml") || starts(b"\xEF\xBB\xBF<?xml") {
        MediaClass::Xml
    } else {
        MediaClass::Unknown
    }
}

fn is_pe(bytes: &[u8]) -> bool {
    let Some(offset) = bytes.get(0x3C..0x40) else {
        return false;
    };
    let offset = u32::from_le_bytes([offset[0], offset[1], offset[2], offset[3]]) as usize;
    offset
        .checked_add(4)
        .and_then(|end| bytes.get(offset..end))
        .is_some_and(|signature| signature == b"PE\0\0")
}

/// A nested ZIP's central directory, read without extracting anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NestedArchive {
    /// The payload is not a ZIP.
    NotArchive,
    /// The payload is a ZIP whose directory could not be read.
    Unreadable,
    /// Counts from the directory. Sizes are what the directory *declares*.
    Read {
        entries: u64,
        declared_expanded_size: u64,
        encrypted_entries: u64,
        /// Entries whose *name* ends in `.zip`. Their content is not opened.
        archive_named_entries: u64,
    },
}

/// [`inspect_nested_controlled`] with a quiet cancellation control.
///
/// The quiet control never reports [`crate::operation::Stop`], so this
/// collapses any refusal into `Unreadable`, exactly as the pre-cancellation
/// inspect did.
fn inspect_nested_controlled(
    class: MediaClass,
    bytes: &[u8],
    cancellation: Cancellation<'_>,
) -> Result<NestedArchive, crate::package::PackageError> {
    if class != MediaClass::Zip {
        return Ok(NestedArchive::NotArchive);
    }
    // The package validator in full (entry/directory budgets, no ZIP64 or
    // multi-disk, local/central agreement, no overlaps), then only its
    // checked directory metadata: no `ZipArchive`, whose end-of-directory
    // search is superlinear on hostile input, and nothing is decompressed.
    let entries = crate::package::validated_zip_metadata(bytes, cancellation)?;
    let (mut expanded, mut encrypted, mut archives) = (0u64, 0u64, 0u64);
    for entry in &entries {
        cancellation.checkpoint()?;
        let Some(sum) = expanded.checked_add(entry.uncompressed_size) else {
            return Ok(NestedArchive::Unreadable);
        };
        expanded = sum;
        // General-purpose flag bit 0: the entry is encrypted.
        encrypted += u64::from(entry.flags & 1);
        archives += u64::from(entry.name.to_ascii_lowercase().ends_with(b".zip"));
    }
    Ok(NestedArchive::Read {
        entries: entries.len() as u64,
        declared_expanded_size: expanded,
        encrypted_entries: encrypted,
        archive_named_entries: archives,
    })
}

/// One retained `Baggages/` member.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaggagePayload {
    pub member_path: String,
    pub sha256: String,
    pub size: u64,
    pub media_class: MediaClass,
    pub nested: NestedArchive,
    /// Declarations in this package that resolve to this member.
    pub declarations: u64,
}

impl BaggagePayload {
    pub fn measure(member_path: String, sha256: String, bytes: &[u8]) -> Self {
        match BaggagePayload::measure_controlled(
            member_path.clone(),
            sha256.clone(),
            bytes,
            Cancellation::Quiet,
        ) {
            Ok(payload) => payload,
            Err(_) => {
                // The quiet path collapses any refusal into `Unreadable`,
                // exactly as the pre-cancellation measure did, keeping the
                // member identity and its content class.
                BaggagePayload {
                    member_path,
                    sha256,
                    size: bytes.len() as u64,
                    media_class: sniff_media(bytes),
                    nested: NestedArchive::Unreadable,
                    declarations: 0,
                }
            }
        }
    }

    /// [`measure`] on the installer path: a controlled measure propagates a
    /// typed [`crate::operation::Stop`] rather than collapsing it into
    /// `Unreadable`, so a cancelled install never silently succeeds.
    pub(crate) fn measure_controlled(
        member_path: String,
        sha256: String,
        bytes: &[u8],
        cancellation: Cancellation<'_>,
    ) -> Result<Self, crate::package::PackageError> {
        let media_class = sniff_media(bytes);
        let nested = inspect_nested_controlled(media_class, bytes, cancellation)?;
        Ok(Self {
            member_path,
            sha256,
            size: bytes.len() as u64,
            media_class,
            nested,
            declarations: 0,
        })
    }

    /// The member's extension, lower-cased, if its last component has one.
    pub fn extension(&self) -> Option<String> {
        let name = self.member_path.rsplit('/').next()?;
        let (stem, extension) = name.rsplit_once('.')?;
        (!stem.is_empty() && !extension.is_empty()).then(|| extension.to_ascii_lowercase())
    }

    /// `Some(false)` when the extension names a different known format than
    /// the content; `None` when either side says nothing.
    pub fn extension_agrees(&self) -> Option<bool> {
        let extension = self.extension()?;
        let owner = MediaClass::ALL
            .into_iter()
            .find(|class| class.usual_extensions().contains(&extension.as_str()))?;
        if matches!(self.media_class, MediaClass::Unknown | MediaClass::Empty) {
            return None;
        }
        Some(owner == self.media_class)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolution {
    Resolved,
    /// The path is well formed but no member carries it.
    Missing,
    /// The declaration cannot name a path at all.
    Invalid,
}

impl Resolution {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Resolved => "resolved",
            Self::Missing => "missing",
            Self::Invalid => "invalid",
        }
    }

    fn from_db(value: &str) -> Result<Self, ProductDbError> {
        match value {
            "resolved" => Ok(Self::Resolved),
            "missing" => Ok(Self::Missing),
            "invalid" => Ok(Self::Invalid),
            _ => Err(inventory_error(format!("invalid resolution {value:?}"))),
        }
    }
}

/// A declaration as stored: its index, document order, raw lexemes and
/// what it resolved to. `detail` says why an unresolved one did not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedDeclaration {
    pub index_path: String,
    pub ordinal: u64,
    pub declaration: BaggageDeclaration,
    pub resolution: Resolution,
    pub member_path: Option<String>,
    pub detail: Option<String>,
}

/// One parsed index: its archive path, its declarations in document order,
/// and why none of them may be bound to its directory, if so.
pub(crate) type ParsedIndex = (String, Vec<BaggageDeclaration>, Option<&'static str>);

/// Parses a retained index into the shape [`BaggageInventory::resolve`]
/// takes, with the unknown constructs the parser reported.
pub(crate) fn parse_index(
    path: &str,
    bytes: &[u8],
) -> Result<(ParsedIndex, Vec<crate::report::UnknownConstruct>), ProductDbError> {
    let index = crate::parse::baggage::parse_baggage_index(path, bytes, Cancellation::Quiet)?;
    let refused = crate::parse::baggage::manufacturer_mismatch(path, &index);
    Ok((
        (path.to_string(), index.declarations, refused),
        index.unknown,
    ))
}

/// Everything one package declares and carries as baggage.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BaggageInventory {
    pub declarations: Vec<ResolvedDeclaration>,
    pub payloads: Vec<BaggagePayload>,
}

impl BaggageInventory {
    /// Joins parsed indexes to measured payloads by exact archive path.
    pub(crate) fn resolve(indexes: Vec<ParsedIndex>, mut payloads: Vec<BaggagePayload>) -> Self {
        payloads.sort_by(|a, b| a.member_path.cmp(&b.member_path));
        let by_path: BTreeMap<String, usize> = payloads
            .iter()
            .enumerate()
            .map(|(position, payload)| (payload.member_path.clone(), position))
            .collect();
        let mut declarations = Vec::new();
        let mut indexes = indexes;
        indexes.sort_by(|a, b| a.0.cmp(&b.0));
        for (index_path, parsed, refused) in indexes {
            for (ordinal, declaration) in (0u64..).zip(parsed) {
                let bound = match refused {
                    Some(why) => Err(why),
                    None => declared_member_path(&index_path, &declaration),
                };
                let (resolution, member_path, detail) = match bound {
                    Err(why) => (Resolution::Invalid, None, Some(why.to_string())),
                    Ok(path) => match by_path.get(&path) {
                        Some(&position) => {
                            payloads[position].declarations += 1;
                            (Resolution::Resolved, Some(path), None)
                        }
                        None => (Resolution::Missing, None, Some(MISSING_DETAIL.to_string())),
                    },
                };
                declarations.push(ResolvedDeclaration {
                    index_path: index_path.clone(),
                    ordinal,
                    declaration,
                    resolution,
                    member_path,
                    detail,
                });
            }
        }
        Self {
            declarations,
            payloads,
        }
    }

    pub fn unresolved(&self) -> impl Iterator<Item = &ResolvedDeclaration> {
        self.declarations
            .iter()
            .filter(|row| row.resolution != Resolution::Resolved)
    }

    pub fn undeclared(&self) -> impl Iterator<Item = &BaggagePayload> {
        self.payloads
            .iter()
            .filter(|payload| payload.declarations == 0)
    }

    /// Unresolved declarations grouped by (index, reason), for diagnostics.
    pub(crate) fn unresolved_groups(&self) -> BTreeMap<(String, String), u64> {
        let mut groups = BTreeMap::new();
        for row in self.unresolved() {
            let reason = row.detail.clone().unwrap_or_default();
            *groups.entry((row.index_path.clone(), reason)).or_insert(0) += 1;
        }
        groups
    }
}

pub(crate) const MISSING_DETAIL: &str = "no package member at the declared path";

fn inventory_error(cause: impl Into<String>) -> ProductDbError {
    ProductDbError::Xml {
        source_path: "package_baggage".into(),
        cause: cause.into(),
    }
}

fn to_i64(value: u64, what: &str) -> Result<i64, ProductDbError> {
    i64::try_from(value).map_err(|_| inventory_error(format!("{what} exceeds SQLite INTEGER")))
}

fn to_u64(value: i64, what: &str) -> Result<u64, ProductDbError> {
    u64::try_from(value).map_err(|_| inventory_error(format!("negative {what}")))
}

pub(crate) fn persist(
    conn: &Connection,
    package: &str,
    inventory: &BaggageInventory,
) -> Result<(), ProductDbError> {
    conn.execute(
        "INSERT INTO package_baggage_inventory (package_sha256, status) VALUES (?1, 'measured')",
        [package],
    )?;
    for payload in &inventory.payloads {
        let (status, entries, expanded, encrypted, archives) = match payload.nested {
            NestedArchive::NotArchive => ("not-archive", None, None, None, None),
            NestedArchive::Unreadable => ("unreadable", None, None, None, None),
            NestedArchive::Read {
                entries,
                declared_expanded_size,
                encrypted_entries,
                archive_named_entries,
            } => (
                "read",
                Some(to_i64(entries, "entries")?),
                Some(to_i64(declared_expanded_size, "expanded size")?),
                Some(to_i64(encrypted_entries, "encrypted entries")?),
                Some(to_i64(archive_named_entries, "archive entries")?),
            ),
        };
        conn.execute(
            "INSERT INTO package_baggage_payload VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                package,
                payload.member_path,
                payload.sha256,
                to_i64(payload.size, "size")?,
                payload.media_class.as_str(),
                to_i64(payload.declarations, "declarations")?,
                status,
                entries,
                expanded,
                encrypted,
                archives
            ],
        )?;
    }
    for row in &inventory.declarations {
        let d = &row.declaration;
        conn.execute(
            "INSERT INTO package_baggage_declaration VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                package,
                row.index_path,
                to_i64(row.ordinal, "ordinal")?,
                d.id,
                d.name,
                d.target_path,
                d.install_on_import,
                d.time_info,
                d.file_version,
                row.resolution.as_str(),
                row.member_path,
                row.detail
            ],
        )?;
    }
    Ok(())
}

pub(crate) fn persist_unavailable(conn: &Connection, package: &str) -> Result<(), ProductDbError> {
    conn.execute(
        "INSERT INTO package_baggage_inventory (package_sha256, status) VALUES (?1, 'unavailable')",
        [package],
    )?;
    Ok(())
}

/// The stored inventory of `package`: `Ok(None)` when it could not be
/// measured (see the migration), never an invented empty inventory.
/// Re-validates every cross-row invariant, so a tampered table is an error.
pub fn load_baggage_inventory(
    conn: &Connection,
    package: &str,
) -> Result<Option<BaggageInventory>, ProductDbError> {
    let status: Option<String> = conn
        .query_row(
            "SELECT status FROM package_baggage_inventory WHERE package_sha256 = ?1",
            [package],
            |r| r.get(0),
        )
        .optional()?;
    match status.as_deref() {
        None => {
            return Err(inventory_error(
                "installed package has no baggage inventory marker",
            ))
        }
        Some("unavailable") => {
            let rows: i64 = conn.query_row(
                "SELECT (SELECT count(*) FROM package_baggage_payload WHERE package_sha256 = ?1)
                      + (SELECT count(*) FROM package_baggage_declaration WHERE package_sha256 = ?1)",
                [package],
                |r| r.get(0),
            )?;
            if rows != 0 {
                return Err(inventory_error("unavailable baggage inventory has rows"));
            }
            return Ok(None);
        }
        Some("measured") => {}
        Some(other) => {
            return Err(inventory_error(format!(
                "invalid inventory status {other:?}"
            )))
        }
    }
    let mut payloads = Vec::new();
    let mut stmt = conn.prepare(
        "SELECT p.member_path, p.sha256, p.size, p.media_class, p.declarations,
                p.nested_status, p.nested_entries, p.nested_expanded_size,
                p.nested_encrypted_entries, p.nested_archive_names, m.role, m.source_sha256, m.size
         FROM package_baggage_payload p
         LEFT JOIN package_member m ON m.package_sha256 = p.package_sha256 AND m.path = p.member_path
         WHERE p.package_sha256 = ?1 ORDER BY p.member_path",
    )?;
    let mut rows = stmt.query([package])?;
    while let Some(r) = rows.next()? {
        let member_path: String = r.get(0)?;
        let sha256: String = r.get(1)?;
        let size = to_u64(r.get(2)?, "size")?;
        let role: Option<String> = r.get(10)?;
        if role.as_deref() != Some("Baggage")
            || r.get::<_, Option<String>>(11)?.as_deref() != Some(sha256.as_str())
            || r.get::<_, Option<i64>>(12)? != Some(r.get(2)?)
        {
            return Err(inventory_error(
                "baggage payload does not match its package member",
            ));
        }
        let nested = match (r.get::<_, String>(5)?.as_str(), r.get::<_, Option<i64>>(6)?) {
            ("not-archive", None) => NestedArchive::NotArchive,
            ("unreadable", None) => NestedArchive::Unreadable,
            ("read", Some(entries)) => NestedArchive::Read {
                entries: to_u64(entries, "entries")?,
                declared_expanded_size: to_u64(r.get(7)?, "expanded size")?,
                encrypted_entries: to_u64(r.get(8)?, "encrypted entries")?,
                archive_named_entries: to_u64(r.get(9)?, "archive entries")?,
            },
            _ => return Err(inventory_error("inconsistent nested-archive columns")),
        };
        let media_class = MediaClass::from_db(&r.get::<_, String>(3)?)?;
        if (media_class == MediaClass::Zip) == (nested == NestedArchive::NotArchive) {
            return Err(inventory_error(
                "nested-archive status contradicts media class",
            ));
        }
        payloads.push(BaggagePayload {
            member_path,
            sha256,
            size,
            media_class,
            nested,
            declarations: to_u64(r.get(4)?, "declarations")?,
        });
    }
    let mut declarations = Vec::new();
    let mut stmt = conn.prepare(
        "SELECT index_path, ordinal, baggage_id, name, target_path, install_on_import, time_info,
                file_version, resolution, member_path, detail
         FROM package_baggage_declaration WHERE package_sha256 = ?1 ORDER BY index_path, ordinal",
    )?;
    let mut rows = stmt.query([package])?;
    while let Some(r) = rows.next()? {
        declarations.push(ResolvedDeclaration {
            index_path: r.get(0)?,
            ordinal: to_u64(r.get(1)?, "ordinal")?,
            declaration: BaggageDeclaration {
                id: r.get(2)?,
                name: r.get(3)?,
                target_path: r.get(4)?,
                install_on_import: r.get(5)?,
                time_info: r.get(6)?,
                file_version: r.get(7)?,
            },
            resolution: Resolution::from_db(&r.get::<_, String>(8)?)?,
            member_path: r.get(9)?,
            detail: r.get(10)?,
        });
    }
    let loaded = BaggageInventory {
        declarations,
        payloads,
    };
    validate(conn, package, &loaded)?;
    Ok(Some(loaded))
}

/// Re-derives resolution and per-payload declaration counts from the stored
/// lexemes, re-measures every payload from its retained blob (the join in
/// `load_baggage_inventory` already tied the row's hash to the member), and
/// requires every `Baggage` member of the package to have a payload row: a
/// stored inventory that disagrees with itself or its bytes is corrupt.
fn validate(
    conn: &Connection,
    package: &str,
    inventory: &BaggageInventory,
) -> Result<(), ProductDbError> {
    let members_of = |role: &str| -> Result<BTreeMap<String, String>, ProductDbError> {
        Ok(conn
            .prepare(
                "SELECT path, source_sha256 FROM package_member
                 WHERE package_sha256 = ?1 AND role = ?2",
            )?
            .query_map(params![package, role], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<Result<_, _>>()?)
    };
    let members = members_of("Baggage")?;
    let payload_members: BTreeMap<String, String> = inventory
        .payloads
        .iter()
        .map(|payload| (payload.member_path.clone(), payload.sha256.clone()))
        .collect();
    // Paths only: the loader already binds each payload row to its member's
    // blob hash. A repeated path collapses in the map and fails the length.
    if members.keys().ne(payload_members.keys())
        || payload_members.len() != inventory.payloads.len()
    {
        return Err(inventory_error(
            "baggage payload rows do not match the package's Baggage members",
        ));
    }
    // The declarations are re-derived from the retained index blobs, never
    // taken from the stored rows: a forged lexeme must not survive reload.
    let mut expected_indexes = Vec::new();
    for (path, sha) in members_of("Baggages")? {
        let bytes = crate::load_source_file(conn, &sha)?
            .ok_or_else(|| inventory_error("baggage index blob is missing"))?;
        let (index, _) = parse_index(&path, &bytes)
            .map_err(|_| inventory_error("retained baggage index no longer parses"))?;
        expected_indexes.push(index);
    }
    let mut unmeasured = Vec::with_capacity(inventory.payloads.len());
    for payload in &inventory.payloads {
        let bytes = crate::load_source_file(conn, &payload.sha256)?
            .ok_or_else(|| inventory_error("baggage payload blob is missing"))?;
        unmeasured.push(BaggagePayload::measure(
            payload.member_path.clone(),
            payload.sha256.clone(),
            &bytes,
        ));
    }
    let expected = BaggageInventory::resolve(expected_indexes, unmeasured);
    if &expected != inventory {
        return Err(inventory_error(
            "stored baggage resolution disagrees with its declarations",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn sniffing_uses_content_and_refuses_weak_two_byte_matches() {
        let mut pe = b"MZ".to_vec();
        pe.resize(0x40, 0);
        pe[0x3C] = 0x40;
        pe.extend_from_slice(b"PE\0\0");
        let cases: [(&[u8], MediaClass); 12] = [
            (b"", MediaClass::Empty),
            (b"\x89PNG\r\n\x1a\nrest", MediaClass::Png),
            (b"\xFF\xD8\xFF\xE0", MediaClass::Jpeg),
            (b"GIF89a..", MediaClass::Gif),
            (b"BM\x10\0\0\0\0\0\0\0\x36\0\0\0", MediaClass::Bmp),
            (b"BMW is not a bitmap header", MediaClass::Unknown),
            (b"%PDF-1.7", MediaClass::Pdf),
            (b"PK\x05\x06", MediaClass::Zip),
            (&pe, MediaClass::PeExecutable),
            (b"MZ but no PE header", MediaClass::Unknown),
            (
                b"\xD0\xCF\x11\xE0\xA1\xB1\x1A\xE1",
                MediaClass::Ole2Compound,
            ),
            (b"<?xml version=\"1.0\"?>", MediaClass::Xml),
        ];
        for (bytes, class) in cases {
            assert_eq!(sniff_media(bytes), class, "{bytes:?}");
        }
    }

    #[test]
    fn extension_disagreement_is_reported_not_used() {
        let bmp_named_png = BaggagePayload::measure(
            "M-0001/Baggages/logo.png".into(),
            "00".into(),
            b"BM\x10\0\0\0\0\0\0\0\x36\0\0\0",
        );
        assert_eq!(bmp_named_png.media_class, MediaClass::Bmp);
        assert_eq!(bmp_named_png.extension_agrees(), Some(false));
        let extensionless =
            BaggagePayload::measure("M-0001/Baggages/README".into(), "00".into(), b"x");
        assert_eq!(extensionless.extension(), None);
        assert_eq!(extensionless.extension_agrees(), None);
    }

    #[test]
    fn nested_zip_directory_is_read_without_extraction() {
        use std::io::Write;
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default();
        writer.start_file("inner.zip", options).unwrap();
        writer.write_all(&[0u8; 100]).unwrap();
        writer.start_file("a.txt", options).unwrap();
        writer.write_all(b"hello").unwrap();
        let bytes = writer.finish().unwrap().into_inner();
        let payload = BaggagePayload::measure("M-0001/Baggages/x.zip".into(), "00".into(), &bytes);
        assert_eq!(
            payload.nested,
            NestedArchive::Read {
                entries: 2,
                declared_expanded_size: 105,
                encrypted_entries: 0,
                archive_named_entries: 1,
            }
        );
        let truncated =
            BaggagePayload::measure("M-0001/Baggages/y.zip".into(), "00".into(), &bytes[..20]);
        assert_eq!(truncated.nested, NestedArchive::Unreadable);
    }

    #[test]
    fn nested_zip_claiming_a_huge_directory_is_refused_before_indexing() {
        // An end-of-directory record claiming 65,535 entries in 22 bytes:
        // over the package member budget, so never handed to `ZipArchive`.
        let mut eocd = b"PK\x05\x06".to_vec();
        eocd.extend_from_slice(&[0, 0, 0, 0, 0xFF, 0xFF, 0xFF, 0xFF]);
        eocd.extend_from_slice(&[0; 10]);
        let payload =
            BaggagePayload::measure("M-0001/Baggages/bomb.zip".into(), "00".into(), &eocd);
        assert_eq!(payload.media_class, MediaClass::Zip);
        assert_eq!(payload.nested, NestedArchive::Unreadable);
        // A ZIP64 locator in front of the record is refused as well.
        let mut zip64 = b"PK\x06\x07".to_vec();
        zip64.extend_from_slice(&[0; 16]);
        zip64.extend_from_slice(b"PK\x05\x06");
        zip64.extend_from_slice(&[0; 18]);
        let mut named = b"PK\x03\x04".to_vec();
        named.extend_from_slice(&zip64);
        let payload =
            BaggagePayload::measure("M-0001/Baggages/z64.zip".into(), "00".into(), &named);
        assert_eq!(payload.nested, NestedArchive::Unreadable);
    }

    /// The two hostile shapes at a given size. The first is a run of
    /// complete end-of-directory records, each claiming a one-entry directory
    /// spanning everything before it: `zip` 8.6 tries every candidate and
    /// re-parses that span each time (measured 24 ms at 16 KiB, 343 ms at
    /// 64 KiB, 8.3 s at 256 KiB). The bounded validator accepts only the
    /// record that ends the buffer. The second points one valid-looking
    /// record at a garbage run.
    fn hostile_zip_shapes(size: usize) -> [Vec<u8>; 2] {
        fn eocd(size: u32, start: u32) -> Vec<u8> {
            let mut record = b"PK\x05\x06".to_vec();
            record.extend_from_slice(&[0, 0, 0, 0, 1, 0, 1, 0]);
            record.extend_from_slice(&size.to_le_bytes());
            record.extend_from_slice(&start.to_le_bytes());
            record.extend_from_slice(&[0, 0]);
            record
        }
        let mut candidates = b"PK\x03\x04".to_vec();
        while candidates.len() < size {
            let at = u32::try_from(candidates.len()).unwrap();
            candidates.extend_from_slice(&eocd(at, 0));
        }
        let mut garbage = b"PK\x03\x04".to_vec();
        garbage.resize(size, 0xA5);
        let start = u32::try_from(garbage.len() - 46).unwrap();
        garbage.extend_from_slice(&eocd(46, start));
        [candidates, garbage]
    }

    #[test]
    fn hostile_nested_zips_are_refused_in_linear_time() {
        // Linearity is proven by counting, not timing: the validator's meter
        // (`package::zip_work`) records every byte it inspects, and that count
        // must stay within twice the input at every size. A host under load
        // can stall a thread for seconds inside a microsecond window, so the
        // clock is only a backstop below.
        use crate::package::zip_work;
        for size in [16 * 1024, 64 * 1024, 256 * 1024, 2048 * 1024] {
            for bytes in hostile_zip_shapes(size) {
                zip_work::take();
                let started = std::time::Instant::now();
                let payload =
                    BaggagePayload::measure("M-0001/Baggages/h.zip".into(), "00".into(), &bytes);
                let elapsed = started.elapsed();
                let inspected = zip_work::take();
                assert_eq!(payload.media_class, MediaClass::Zip);
                assert_eq!(payload.nested, NestedArchive::Unreadable);
                // Nonzero: the refusal came from the metered validator, not
                // from a parser the meter cannot see.
                assert!(inspected > 0, "{} bytes: validator not used", bytes.len());
                assert!(
                    inspected <= 2 * bytes.len() as u64,
                    "{} bytes: {inspected} bytes inspected",
                    bytes.len()
                );
                // Backstop for work outside the meter: a quadratic parser
                // needs minutes at 2 MiB (`zip` 8.6 extrapolates to ~9 min);
                // the metered validator needs microseconds plus any stall.
                assert!(
                    elapsed < std::time::Duration::from_secs(30),
                    "{} bytes took {elapsed:?}",
                    bytes.len()
                );
            }
        }
    }

    #[test]
    fn the_zip_work_meter_counts_a_valid_nested_archive() {
        // Control for the linear-time test: a well-formed archive with many
        // entries is read, metered, and still inside the same bound.
        use crate::package::zip_work;
        use std::io::Write;
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default();
        for index in 0..200 {
            writer
                .start_file(format!("member-{index}.txt"), options)
                .unwrap();
            writer.write_all(b"payload").unwrap();
        }
        let bytes = writer.finish().unwrap().into_inner();
        zip_work::take();
        let payload = BaggagePayload::measure("M-0001/Baggages/v.zip".into(), "00".into(), &bytes);
        let inspected = zip_work::take();
        assert!(matches!(
            payload.nested,
            NestedArchive::Read { entries: 200, .. }
        ));
        // Every central and local header was read at least once.
        assert!(inspected >= 200 * (46 + 30), "{inspected} bytes inspected");
        assert!(
            inspected <= 2 * bytes.len() as u64,
            "{inspected} of {}",
            bytes.len()
        );
    }

    #[test]
    fn an_empty_nested_zip_is_read_as_zero_entries() {
        let mut bytes = b"PK\x05\x06".to_vec();
        bytes.extend_from_slice(&[0; 18]);
        let payload = BaggagePayload::measure("M-0001/Baggages/e.zip".into(), "00".into(), &bytes);
        assert_eq!(payload.media_class, MediaClass::Zip);
        assert_eq!(
            payload.nested,
            NestedArchive::Read {
                entries: 0,
                declared_expanded_size: 0,
                encrypted_entries: 0,
                archive_named_entries: 0,
            }
        );
        // A record claiming entries it does not have is still unreadable.
        bytes[8] = 1;
        bytes[10] = 1;
        let claimed = BaggagePayload::measure("M-0001/Baggages/f.zip".into(), "00".into(), &bytes);
        assert_eq!(claimed.nested, NestedArchive::Unreadable);
    }

    #[test]
    fn a_nested_zip_naming_one_entry_twice_is_unreadable_not_collapsed() {
        use std::io::Write;
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        writer.start_file("aa", options).unwrap();
        writer.write_all(b"12").unwrap();
        writer.start_file("bb", options).unwrap();
        writer.write_all(b"345").unwrap();
        let mut bytes = writer.finish().unwrap().into_inner();
        // Rename the second entry to the first's name in both headers: a
        // name-keyed reader would count one entry of 3 bytes.
        let mut renamed = 0;
        for at in 0..bytes.len() - 4 {
            let header = &bytes[at..at + 4];
            let name_at = if header == b"PK\x03\x04" {
                at + 30
            } else if header == b"PK\x01\x02" {
                at + 46
            } else {
                continue;
            };
            if &bytes[name_at..name_at + 2] == b"bb" {
                bytes[name_at..name_at + 2].copy_from_slice(b"aa");
                renamed += 1;
            }
        }
        assert_eq!(renamed, 2);
        let payload = BaggagePayload::measure("M-0001/Baggages/d.zip".into(), "00".into(), &bytes);
        assert_eq!(payload.nested, NestedArchive::Unreadable);
    }

    #[test]
    fn encrypted_and_archive_named_entries_come_from_validated_metadata() {
        use std::io::Write;
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        writer.start_file("DEEP.ZIP", options).unwrap();
        writer.write_all(b"x").unwrap();
        writer.start_file("plain.txt", options).unwrap();
        writer.write_all(b"yy").unwrap();
        let mut bytes = writer.finish().unwrap().into_inner();
        // Mark the first entry encrypted (flag bit 0) in both headers, as a
        // traditionally encrypted member would be; nothing is decrypted.
        let local = 0;
        let central = bytes.windows(4).position(|w| w == b"PK\x01\x02").unwrap();
        bytes[local + 6] |= 1;
        bytes[central + 8] |= 1;
        let payload = BaggagePayload::measure("M-0001/Baggages/e.zip".into(), "00".into(), &bytes);
        assert_eq!(
            payload.nested,
            NestedArchive::Read {
                entries: 2,
                declared_expanded_size: 3,
                encrypted_entries: 1,
                archive_named_entries: 1,
            }
        );
    }
}
