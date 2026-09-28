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

use std::collections::{BTreeMap, BTreeSet};
use std::io::Cursor;

use rusqlite::{params, Connection, OptionalExtension};

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

fn inspect_nested(class: MediaClass, bytes: &[u8]) -> NestedArchive {
    if class != MediaClass::Zip {
        return NestedArchive::NotArchive;
    }
    // The package gate first: it bounds the entry count and directory size
    // and refuses ZIP64/multi-disk *before* `ZipArchive` allocates an index
    // sized by the untrusted end-of-directory record.
    if crate::package::preflight_zip(bytes).is_err() {
        return NestedArchive::Unreadable;
    }
    let Ok(mut archive) = zip::ZipArchive::new(Cursor::new(bytes)) else {
        return NestedArchive::Unreadable;
    };
    let (mut expanded, mut encrypted, mut archives) = (0u64, 0u64, 0u64);
    for index in 0..archive.len() {
        // `by_index_raw` exposes directory metadata; the entry is never read.
        let Ok(entry) = archive.by_index_raw(index) else {
            return NestedArchive::Unreadable;
        };
        let Some(sum) = expanded.checked_add(entry.size()) else {
            return NestedArchive::Unreadable;
        };
        expanded = sum;
        encrypted += u64::from(entry.encrypted());
        archives += u64::from(entry.name().to_ascii_lowercase().ends_with(".zip"));
    }
    NestedArchive::Read {
        entries: archive.len() as u64,
        declared_expanded_size: expanded,
        encrypted_entries: encrypted,
        archive_named_entries: archives,
    }
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
        let media_class = sniff_media(bytes);
        Self {
            member_path,
            sha256,
            size: bytes.len() as u64,
            media_class,
            nested: inspect_nested(media_class, bytes),
            declarations: 0,
        }
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

/// Everything one package declares and carries as baggage.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BaggageInventory {
    pub declarations: Vec<ResolvedDeclaration>,
    pub payloads: Vec<BaggagePayload>,
}

impl BaggageInventory {
    /// Joins parsed indexes to measured payloads by exact archive path.
    pub(crate) fn resolve(
        indexes: Vec<(String, Vec<BaggageDeclaration>)>,
        mut payloads: Vec<BaggagePayload>,
    ) -> Self {
        payloads.sort_by(|a, b| a.member_path.cmp(&b.member_path));
        let by_path: BTreeMap<String, usize> = payloads
            .iter()
            .enumerate()
            .map(|(position, payload)| (payload.member_path.clone(), position))
            .collect();
        let mut declarations = Vec::new();
        let mut indexes = indexes;
        indexes.sort_by(|a, b| a.0.cmp(&b.0));
        for (index_path, parsed) in indexes {
            for (ordinal, declaration) in (0u64..).zip(parsed) {
                let (resolution, member_path, detail) =
                    match declared_member_path(&index_path, &declaration) {
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
    let members: BTreeSet<String> = conn
        .prepare("SELECT path FROM package_member WHERE package_sha256 = ?1 AND role = 'Baggage'")?
        .query_map([package], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    let payload_paths: BTreeSet<String> = inventory
        .payloads
        .iter()
        .map(|payload| payload.member_path.clone())
        .collect();
    if members != payload_paths {
        return Err(inventory_error(
            "baggage payload rows do not cover the package's Baggage members",
        ));
    }
    let indexes: BTreeSet<String> = conn
        .prepare("SELECT path FROM package_member WHERE package_sha256 = ?1 AND role = 'Baggages'")?
        .query_map([package], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    let mut grouped: BTreeMap<String, Vec<BaggageDeclaration>> = BTreeMap::new();
    for (position, row) in inventory.declarations.iter().enumerate() {
        if !indexes.contains(&row.index_path) {
            return Err(inventory_error(
                "baggage declaration names a non-index member",
            ));
        }
        let list = grouped.entry(row.index_path.clone()).or_default();
        if row.ordinal != list.len() as u64 {
            return Err(inventory_error(format!(
                "baggage declaration ordinals have a gap at row {position}"
            )));
        }
        list.push(row.declaration.clone());
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
    let expected = BaggageInventory::resolve(grouped.into_iter().collect(), unmeasured);
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
}
