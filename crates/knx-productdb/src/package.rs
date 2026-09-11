//! Evidence-backed, atomic installation of readable scheme 11/20 product ZIPs.

use std::collections::HashSet;
use std::fmt;
use std::io::{Cursor, Read};

use quick_xml::events::Event;
use quick_xml::name::ResolveResult;
use rusqlite::{params, Connection, OptionalExtension};

use crate::ingest::{classify, ingest_file_in_transaction};
use crate::report::{insert_unknown, IdConflict, UnknownCollector};
use crate::{sha256_hex, FileKind, IngestOutcome, ProductDbError};

const MAX_MEMBER_SIZE: u64 = 64 * 1024 * 1024;
const MAX_PACKAGE_SIZE: usize = 256 * 1024 * 1024;
const MAX_EXPANDED_SIZE: u64 = 256 * 1024 * 1024;
const MAX_MEMBERS: usize = 4096;

#[derive(Debug)]
pub enum PackageError {
    LegacyVd2 { sha256: String, len: usize },
    InvalidZip { cause: String },
    Encrypted { path: String },
    UnsafeMember { path: String },
    DuplicateMember { path: String },
    SizeLimit { path: String },
    MissingMaster,
    UnsupportedNamespace { namespace: String },
    ProjectArchive,
    MissingManufacturerData,
    Database(ProductDbError),
}

impl fmt::Display for PackageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LegacyVd2 { sha256, len } => write!(
                f,
                "legacy .vd2 product data is unsupported (sha256 {sha256}, {len} bytes)"
            ),
            Self::InvalidZip { cause } => write!(f, "invalid product ZIP: {cause}"),
            Self::Encrypted { path } => write!(f, "encrypted product ZIP member: {path}"),
            Self::UnsafeMember { path } => write!(f, "unsafe product ZIP member: {path}"),
            Self::DuplicateMember { path } => write!(f, "duplicate product ZIP member: {path}"),
            Self::SizeLimit { path } => write!(f, "product ZIP size limit exceeded: {path}"),
            Self::MissingMaster => write!(f, "product ZIP is missing knx_master.xml"),
            Self::UnsupportedNamespace { namespace } => {
                write!(f, "unsupported product master namespace: {namespace}")
            }
            Self::ProjectArchive => {
                write!(f, "project archive is not a standalone product package")
            }
            Self::MissingManufacturerData => {
                write!(f, "product ZIP contains no recognized manufacturer data")
            }
            Self::Database(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for PackageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Database(error) => Some(error),
            _ => None,
        }
    }
}

impl From<ProductDbError> for PackageError {
    fn from(error: ProductDbError) -> Self {
        Self::Database(error)
    }
}

impl From<rusqlite::Error> for PackageError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Database(error.into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageMember {
    pub path: String,
    pub role: String,
    pub sha256: String,
    pub size: u64,
}

#[derive(Debug)]
pub struct InstallReport {
    pub sha256: String,
    pub scheme: u32,
    pub skipped: bool,
    pub members: Vec<PackageMember>,
    pub unknown: usize,
    pub conflicts: Vec<IdConflict>,
}

fn zip_error(error: impl fmt::Display) -> PackageError {
    PackageError::InvalidZip {
        cause: error.to_string(),
    }
}

fn xml_error(path: &str, error: impl fmt::Display) -> PackageError {
    ProductDbError::Xml {
        source_path: path.to_string(),
        cause: error.to_string(),
    }
    .into()
}

fn manufacturer_partition(path: &str) -> Option<&str> {
    let (manufacturer, name) = path.split_once('/')?;
    (manufacturer.len() == 6
        && manufacturer.starts_with("M-")
        && manufacturer[2..].bytes().all(|b| b.is_ascii_hexdigit())
        && !name.contains('/'))
    .then_some(manufacturer)
}

fn validate_manufacturer(path: &str, bytes: &[u8]) -> Result<(), PackageError> {
    let expected = manufacturer_partition(path)
        .ok_or_else(|| xml_error(path, "invalid manufacturer partition"))?;
    let mut reader = quick_xml::Reader::from_reader(bytes);
    let mut matched = false;
    let mut parents = Vec::new();
    loop {
        let event = reader.read_event().map_err(|e| xml_error(path, e))?;
        match &event {
            Event::Start(e) | Event::Empty(e) => {
                let name = e.local_name().as_ref().to_string();
                if name == "Manufacturer" {
                    if parents != ["KNX", "ManufacturerData"]
                        || crate::xml::attrs(e, path)?.get("RefId") != Some(expected)
                    {
                        return Err(xml_error(
                            path,
                            "Manufacturer RefId or XML root does not match its partition",
                        ));
                    }
                    matched = true;
                }
                if matches!(event, Event::Start(_)) {
                    parents.push(name);
                }
            }
            Event::End(_) => {
                parents.pop();
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if !matched {
        return Err(xml_error(path, "missing Manufacturer RefId"));
    }
    Ok(())
}

fn package_conflicts(conn: &Connection, sha256: &str) -> Result<Vec<IdConflict>, ProductDbError> {
    Ok(conn.prepare("SELECT table_name, logical_id, kept_sha256, other_sha256 FROM package_conflict WHERE package_sha256 = ?1 ORDER BY ordinal")?.query_map([sha256], |r| Ok(IdConflict { table: r.get(0)?, id: r.get(1)?, kept_sha256: r.get(2)?, other_sha256: r.get(3)? }))?.collect::<Result<Vec<_>, _>>()?)
}

// Bound metadata allocation before ZipArchive constructs its entry index.
// ZIP64 is outside this small, corpus-proven package slice.
fn preflight_zip(bytes: &[u8]) -> Result<(), PackageError> {
    let start = bytes.len().saturating_sub(22 + u16::MAX as usize);
    for offset in (start..bytes.len().saturating_sub(21)).rev() {
        if bytes.get(offset..offset + 4) != Some(b"PK\x05\x06") {
            continue;
        }
        let record = &bytes[offset..offset + 22];
        let comment_len = u16::from_le_bytes([record[20], record[21]]) as usize;
        if offset + 22 + comment_len != bytes.len() {
            continue;
        }
        if offset >= 20 && bytes.get(offset - 20..offset - 16) == Some(b"PK\x06\x07") {
            return Err(zip_error("ZIP64 product packages are unsupported"));
        }
        let count = u16::from_le_bytes([record[10], record[11]]) as usize;
        if count > MAX_MEMBERS {
            return Err(PackageError::SizeLimit {
                path: "member count (ZIP64 unsupported)".into(),
            });
        }
        return Ok(());
    }
    Err(zip_error("missing complete end-of-directory record"))
}

// ZipArchive indexes by name and collapses duplicate entries. Inspect the
// physical central directory first so no input member can disappear that way.
fn validate_central_directory(
    bytes: &[u8],
    start: u64,
    visible_count: usize,
) -> Result<(), PackageError> {
    let mut offset = usize::try_from(start).map_err(zip_error)?;
    let mut paths = HashSet::new();
    let mut count = 0;
    while bytes.get(offset..offset + 4) == Some(b"PK\x01\x02") {
        let header = bytes
            .get(offset..offset + 46)
            .ok_or_else(|| zip_error("truncated central directory"))?;
        let name_len = u16::from_le_bytes([header[28], header[29]]) as usize;
        let extra_len = u16::from_le_bytes([header[30], header[31]]) as usize;
        let comment_len = u16::from_le_bytes([header[32], header[33]]) as usize;
        let name = bytes
            .get(offset + 46..offset + 46 + name_len)
            .ok_or_else(|| zip_error("truncated central directory name"))?;
        if !paths.insert(name) {
            return Err(PackageError::DuplicateMember {
                path: String::from_utf8_lossy(name).into_owned(),
            });
        }
        count += 1;
        if count > MAX_MEMBERS {
            return Err(PackageError::SizeLimit {
                path: "member count".into(),
            });
        }
        offset += 46 + name_len + extra_len + comment_len;
    }
    if count != visible_count {
        return Err(PackageError::DuplicateMember {
            path: "aliased ZIP member names".into(),
        });
    }
    Ok(())
}

fn master_scheme(bytes: &[u8]) -> Result<u32, PackageError> {
    let mut reader = quick_xml::NsReader::from_reader(bytes);
    loop {
        match reader
            .read_resolved_event()
            .map_err(|e| xml_error("knx_master.xml", e))?
        {
            (namespace, Event::Start(root) | Event::Empty(root)) => {
                let namespace = match namespace {
                    ResolveResult::Bound(ns) => ns.as_ref().to_string(),
                    _ => String::new(),
                };
                if root.local_name().as_ref() == "KNX" {
                    match namespace.as_str() {
                        "http://knx.org/xml/project/11" => return Ok(11),
                        "http://knx.org/xml/project/20" => return Ok(20),
                        _ => {}
                    }
                }
                return Err(PackageError::UnsupportedNamespace { namespace });
            }
            (_, Event::Eof) => return Err(xml_error("knx_master.xml", "missing XML root")),
            _ => {}
        }
    }
}

// quick-xml is a streaming tokenizer; enforce a complete document even for
// unrecognized XML, which otherwise has no domain parser to detect truncation.
fn validate_xml(path: &str, bytes: &[u8]) -> Result<(), PackageError> {
    let mut reader = quick_xml::Reader::from_reader(bytes);
    let mut depth = 0;
    let mut roots = 0;
    loop {
        match reader.read_event().map_err(|e| xml_error(path, e))? {
            Event::Start(element) => {
                for attribute in element.attributes().with_checks(true) {
                    let attribute = attribute.map_err(|e| xml_error(path, e))?;
                    attribute
                        .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                        .map_err(|e| xml_error(path, e))?;
                }
                if depth == 0 {
                    roots += 1;
                }
                depth += 1;
            }
            Event::Empty(element) => {
                for attribute in element.attributes().with_checks(true) {
                    let attribute = attribute.map_err(|e| xml_error(path, e))?;
                    attribute
                        .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                        .map_err(|e| xml_error(path, e))?;
                }
                if depth == 0 {
                    roots += 1;
                }
            }
            Event::End(_) => {
                depth -= 1;
            }
            Event::Text(text)
                if depth == 0 && !text.as_ref().bytes().all(|b| b.is_ascii_whitespace()) =>
            {
                return Err(xml_error(path, "text outside XML root"))
            }
            Event::CData(_) if depth == 0 => return Err(xml_error(path, "CDATA outside XML root")),
            Event::GeneralRef(reference) => {
                let name = reference.as_ref();
                let predefined = matches!(name, "lt" | "gt" | "amp" | "apos" | "quot");
                let numeric = name
                    .strip_prefix("#x")
                    .filter(|digits| {
                        !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_hexdigit())
                    })
                    .map(|digits| u32::from_str_radix(digits, 16).ok())
                    .or_else(|| {
                        name.strip_prefix('#')
                            .filter(|digits| {
                                !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
                            })
                            .map(|digits| digits.parse().ok())
                    })
                    .flatten()
                    .filter(|code| {
                        matches!(code, 0x9 | 0xA | 0xD)
                            || (0x20..=0xD7FF).contains(code)
                            || (0xE000..=0xFFFD).contains(code)
                            || (0x10000..=0x10FFFF).contains(code)
                    })
                    .is_some();
                if depth == 0 || (!predefined && !numeric) {
                    return Err(xml_error(path, format!("undeclared XML entity {name:?}")));
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if depth != 0 || roots != 1 {
        return Err(xml_error(path, "incomplete XML document"));
    }
    Ok(())
}

/// Install a standalone product package, preserving the archive and every file.
/// All storage and parsing share one transaction. Identical archive bytes skip
/// parsing, including when the caller supplies a different source name.
pub fn install_package(
    conn: &Connection,
    source_name: &str,
    bytes: &[u8],
) -> Result<InstallReport, PackageError> {
    if bytes.len() > MAX_PACKAGE_SIZE {
        return Err(PackageError::SizeLimit {
            path: source_name.into(),
        });
    }
    if source_name.to_ascii_lowercase().ends_with(".vd2") {
        return Err(PackageError::LegacyVd2 {
            sha256: sha256_hex(bytes),
            len: bytes.len(),
        });
    }
    let sha256 = sha256_hex(bytes);
    let tx = conn.unchecked_transaction().map_err(ProductDbError::from)?;
    let prior: Option<(u32, usize)> = tx
        .query_row(
            "SELECT scheme, unknown_count FROM package WHERE sha256 = ?1",
            [&sha256],
            |r| Ok((r.get(0)?, r.get::<_, i64>(1)? as usize)),
        )
        .optional()?;
    if let Some((scheme, unknown)) = prior {
        let members = tx.prepare("SELECT path, role, source_sha256, size FROM package_member WHERE package_sha256 = ?1 ORDER BY ordinal")?.query_map([&sha256], |r| Ok(PackageMember { path: r.get(0)?, role: r.get(1)?, sha256: r.get(2)?, size: r.get::<_, i64>(3)? as u64 }))?.collect::<Result<Vec<_>, _>>()?;
        let conflicts = package_conflicts(&tx, &sha256)?;
        tx.commit().map_err(ProductDbError::from)?;
        return Ok(InstallReport {
            sha256,
            scheme,
            skipped: true,
            members,
            unknown,
            conflicts,
        });
    }
    preflight_zip(bytes)?;
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(zip_error)?;
    validate_central_directory(bytes, archive.central_directory_start(), archive.len())?;
    if archive.len() > MAX_MEMBERS {
        return Err(PackageError::SizeLimit {
            path: "member count".into(),
        });
    }
    let mut paths = HashSet::new();
    let mut total = 0_u64;
    for i in 0..archive.len() {
        let file = archive.by_index_raw(i).map_err(zip_error)?;
        let path = file.name();
        let normalized = path.strip_suffix('/').unwrap_or(path);
        if std::str::from_utf8(file.name_raw()).is_err()
            || normalized.is_empty()
            || path.contains(['\\', ':', '\0'])
            || normalized
                .split('/')
                .any(|part| matches!(part, "" | "." | ".."))
            || file.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000)
        {
            return Err(PackageError::UnsafeMember { path: path.into() });
        }
        if !paths.insert(normalized.to_string()) {
            return Err(PackageError::DuplicateMember { path: path.into() });
        }
        if file.encrypted() {
            return Err(PackageError::Encrypted { path: path.into() });
        }
        total = total
            .checked_add(file.size())
            .ok_or_else(|| PackageError::SizeLimit { path: path.into() })?;
        if file.size() > MAX_MEMBER_SIZE || total > MAX_EXPANDED_SIZE {
            return Err(PackageError::SizeLimit { path: path.into() });
        }
        if normalized
            .split('/')
            .next()
            .is_some_and(|top| top.starts_with("P-"))
        {
            return Err(PackageError::ProjectArchive);
        }
    }
    if !paths.contains("knx_master.xml") {
        return Err(PackageError::MissingMaster);
    }
    let mut extracted = Vec::new();
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(zip_error)?;
        if file.is_dir() {
            continue;
        }
        let mut data = Vec::new();
        (&mut file)
            .take(MAX_MEMBER_SIZE + 1)
            .read_to_end(&mut data)
            .map_err(zip_error)?;
        if data.len() as u64 > MAX_MEMBER_SIZE {
            return Err(PackageError::SizeLimit {
                path: file.name().into(),
            });
        }
        if data.len() as u64 != file.size() {
            return Err(zip_error(format!("size mismatch for {}", file.name())));
        }
        extracted.push((file.name().to_string(), data));
    }
    let master = extracted
        .iter()
        .find(|(path, _)| path == "knx_master.xml")
        .ok_or(PackageError::MissingMaster)?;
    let scheme = master_scheme(&master.1)?;
    if !extracted.iter().any(|(path, data)| {
        manufacturer_partition(path).is_some()
            && matches!(
                classify(data),
                FileKind::Catalog | FileKind::Hardware | FileKind::ApplicationProgram
            )
    }) {
        return Err(PackageError::MissingManufacturerData);
    }
    tx.execute("INSERT INTO package (sha256, source_name, scheme, size, bytes, unknown_count) VALUES (?1, ?2, ?3, ?4, ?5, 0)", params![sha256, source_name, scheme, bytes.len() as i64, bytes])?;
    let mut report = InstallReport {
        sha256,
        scheme,
        skipped: false,
        members: Vec::new(),
        unknown: 0,
        conflicts: Vec::new(),
    };
    for (ordinal, (path, data)) in extracted.into_iter().enumerate() {
        if path.ends_with(".xml") {
            validate_xml(&path, &data)?;
        }
        let kind = classify(&data);
        let role = if path == "knx_master.xml" {
            "Master".into()
        } else if path.ends_with(".signature") {
            "Signature".into()
        } else if path.contains("/Baggages/") {
            "Baggage".into()
        } else if manufacturer_partition(&path).is_none() || matches!(kind, FileKind::Baggage) {
            "Unrecognized".into()
        } else {
            format!("{kind:?}")
        };
        let member_sha = sha256_hex(&data);
        if matches!(role.as_str(), "Catalog" | "Hardware" | "ApplicationProgram") {
            validate_manufacturer(&path, &data)?;
        }
        let outcome = if matches!(
            role.as_str(),
            "Catalog" | "Hardware" | "ApplicationProgram" | "Baggages"
        ) {
            // A retained raw member is not proof its domain rows were parsed.
            // The package hash, not the blob hash, controls package retries.
            ingest_file_in_transaction(&tx, &path, &data, true)?
        } else {
            crate::store_source_file(
                &tx,
                &crate::SourceFile {
                    source_path: path.clone(),
                    manufacturer_id: path
                        .split('/')
                        .next()
                        .filter(|p| p.starts_with("M-"))
                        .map(str::to_string),
                    bytes: data.clone(),
                },
            )?;
            IngestOutcome::Skipped {
                sha256: member_sha.clone(),
            }
        };
        if let IngestOutcome::Ingested {
            unknown, conflicts, ..
        } = outcome
        {
            report.unknown += unknown;
            report.conflicts.extend(conflicts);
        }
        if role == "Master" {
            let unknown = crate::ingest_master_data(&tx, &data)?;
            insert_unknown(&tx, &member_sha, &unknown)?;
            report.unknown += unknown.len();
        } else if role == "Unrecognized" || role == "Baggages" {
            let mut unknown = UnknownCollector::default();
            unknown.element("/Package", &path);
            let unknown = unknown.into_vec();
            insert_unknown(&tx, &member_sha, &unknown)?;
            report.unknown += unknown.len();
        }
        tx.execute("INSERT INTO package_member (package_sha256, ordinal, path, role, source_sha256, size) VALUES (?1, ?2, ?3, ?4, ?5, ?6)", params![report.sha256, ordinal as i64, path, role, member_sha, data.len() as i64])?;
        report.members.push(PackageMember {
            path,
            role,
            sha256: member_sha,
            size: data.len() as u64,
        });
    }
    tx.execute(
        "UPDATE package SET unknown_count = ?2 WHERE sha256 = ?1",
        params![report.sha256, report.unknown as i64],
    )?;
    for (ordinal, conflict) in report.conflicts.iter().enumerate() {
        tx.execute(
            "INSERT INTO package_conflict (package_sha256, ordinal, table_name, logical_id, kept_sha256, other_sha256) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                report.sha256,
                ordinal as i64,
                conflict.table,
                conflict.id,
                conflict.kept_sha256,
                conflict.other_sha256,
            ],
        )?;
    }
    report.conflicts = package_conflicts(&tx, &report.sha256)?;
    tx.commit().map_err(ProductDbError::from)?;
    Ok(report)
}
