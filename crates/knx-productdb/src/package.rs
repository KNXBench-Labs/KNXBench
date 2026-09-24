//! Evidence-backed, atomic installation of readable scheme 11/20 product ZIPs.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::io::{Cursor, Read, Seek, SeekFrom};

use quick_xml::events::Event;
use quick_xml::name::ResolveResult;
use rusqlite::{params, Connection, OptionalExtension};

use crate::ingest::{classify, ingest_file_in_transaction};
use crate::report::{insert_unknown, IdConflict, TranslationCounts, UnknownCollector};
use crate::{sha256_hex, FileKind, IngestOutcome, ProductDbError};

const MAX_MEMBER_SIZE: u64 = 64 * 1024 * 1024;
const MAX_PACKAGE_SIZE: usize = 256 * 1024 * 1024;
const MAX_EXPANDED_SIZE: u64 = 256 * 1024 * 1024;
const MAX_MEMBERS: usize = 4096;
const MAX_PATH_NODES: usize = 65_536;
const MAX_CENTRAL_DIRECTORY_SIZE: usize = 24 * 1024 * 1024;
const MAX_LOCAL_METADATA_SIZE: usize = 24 * 1024 * 1024;
const MAX_DECODED_PATH_SIZE: usize = 72 * 1024 * 1024;

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
struct ValidatedMember {
    archive_index: usize,
    member: PackageMember,
}

#[derive(Debug)]
pub struct InstallReport {
    pub sha256: String,
    pub scheme: u32,
    pub skipped: bool,
    pub members: Vec<PackageMember>,
    pub unknown: usize,
    pub conflicts: Vec<IdConflict>,
    /// Translation rows this package actually contributed, by scope (R3).
    /// Measured from what every member's ingest pass wrote, never predicted
    /// from the XML. A package retried from the `skipped` branch reports the
    /// counts recorded at its original install — `0` in every field for a
    /// package installed before the columns existed (see
    /// `migrate_v4_to_v5`'s doc comment).
    pub translations: TranslationCounts,
    /// `datapoint_type` rows this package's `knx_master.xml` declared but
    /// that an id already in the table (installed by an earlier package —
    /// see `MasterIngest::dropped_datapoint_types`) caused `INSERT OR
    /// IGNORE` to drop. `0` for a package installed before this column
    /// existed, same convention as `translations` above
    /// (KNOWN_LIMITATIONS.md §86).
    pub dropped_datapoint_types: usize,
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
    Ok(conn.prepare("SELECT table_name, logical_id, kept_sha256, other_sha256, occurrence FROM package_conflict WHERE package_sha256 = ?1 ORDER BY ordinal")?.query_map([sha256], |r| Ok(IdConflict { table: r.get(0)?, id: r.get(1)?, kept_sha256: r.get(2)?, other_sha256: r.get(3)?, occurrence: r.get(4)? }))?.collect::<Result<Vec<_>, _>>()?)
}

#[derive(Debug, Clone, Copy)]
struct ZipDirectory {
    start: usize,
    end: usize,
    count: usize,
}

#[derive(Debug)]
struct CheckedZipEntry {
    central_offset: u64,
    local_offset: u64,
    crc32: u32,
    compressed_size: u64,
    uncompressed_size: u64,
}

#[derive(Debug)]
struct ValidatedZipDirectory {
    entries: Vec<CheckedZipEntry>,
    neutralized_fields: Vec<usize>,
    parser_comment_ranges: Vec<std::ops::Range<usize>>,
}

/// Read-only overlay used only by `zip`: stale Unicode-path field IDs are
/// replaced without cloning or altering the archive that is persisted.
struct ZipParserReader<'a> {
    inner: Cursor<&'a [u8]>,
    neutralized_fields: Vec<usize>,
}

impl<'a> ZipParserReader<'a> {
    fn new(bytes: &'a [u8], neutralized_fields: Vec<usize>) -> Self {
        Self {
            inner: Cursor::new(bytes),
            neutralized_fields,
        }
    }
}

impl Read for ZipParserReader<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let start = self.inner.position() as usize;
        let read = self.inner.read(buffer)?;
        let end = start + read;
        let first = self
            .neutralized_fields
            .partition_point(|offset| offset.saturating_add(2) <= start);
        for &offset in &self.neutralized_fields[first..] {
            if offset >= end {
                break;
            }
            for (index, byte) in 0xA11E_u16.to_le_bytes().into_iter().enumerate() {
                let absolute = offset + index;
                if (start..end).contains(&absolute) {
                    buffer[absolute - start] = byte;
                }
            }
        }
        Ok(read)
    }
}

impl Seek for ZipParserReader<'_> {
    fn seek(&mut self, position: SeekFrom) -> std::io::Result<u64> {
        self.inner.seek(position)
    }
}

#[derive(Debug, Default)]
struct MemberPathNode {
    children: HashMap<String, usize>,
    is_directory: Option<bool>,
}

#[derive(Debug)]
struct MemberPathTree {
    nodes: Vec<MemberPathNode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PathInsertError {
    Collision,
    NodeLimit,
}

impl Default for MemberPathTree {
    fn default() -> Self {
        Self {
            nodes: vec![MemberPathNode::default()],
        }
    }
}

impl MemberPathTree {
    fn insert(&mut self, path: &str, is_directory: bool) -> Result<(), PathInsertError> {
        let mut node = 0;
        for component in path.split('/') {
            if self.nodes[node].is_directory == Some(false) {
                return Err(PathInsertError::Collision);
            }
            node = match self.nodes[node].children.get(component) {
                Some(next) => *next,
                None => {
                    if self.nodes.len() >= MAX_PATH_NODES {
                        return Err(PathInsertError::NodeLimit);
                    }
                    let next = self.nodes.len();
                    self.nodes.push(MemberPathNode::default());
                    self.nodes[node]
                        .children
                        .insert(component.to_string(), next);
                    next
                }
            };
        }
        if self.nodes[node].is_directory.is_some()
            || (!is_directory && !self.nodes[node].children.is_empty())
        {
            return Err(PathInsertError::Collision);
        }
        self.nodes[node].is_directory = Some(is_directory);
        Ok(())
    }
}

// Bound metadata allocation before ZipArchive constructs its entry index.
// ZIP64 is outside this small, corpus-proven package slice.
fn preflight_zip(bytes: &[u8]) -> Result<ZipDirectory, PackageError> {
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
        let disk = u16::from_le_bytes([record[4], record[5]]);
        let directory_disk = u16::from_le_bytes([record[6], record[7]]);
        let disk_count = u16::from_le_bytes([record[8], record[9]]) as usize;
        let count = u16::from_le_bytes([record[10], record[11]]) as usize;
        if disk != 0 || directory_disk != 0 || disk_count != count {
            return Err(zip_error("multi-disk product packages are unsupported"));
        }
        if count > MAX_MEMBERS {
            return Err(PackageError::SizeLimit {
                path: "member count (ZIP64 unsupported)".into(),
            });
        }
        let directory_size = u32::from_le_bytes(record[12..16].try_into().unwrap()) as usize;
        if directory_size > MAX_CENTRAL_DIRECTORY_SIZE {
            return Err(PackageError::SizeLimit {
                path: "central directory metadata".into(),
            });
        }
        let directory_start = u32::from_le_bytes(record[16..20].try_into().unwrap()) as usize;
        let directory_end = directory_start
            .checked_add(directory_size)
            .ok_or_else(|| zip_error("central directory size or extent mismatch"))?;
        if directory_start >= offset || directory_end != offset {
            return Err(zip_error("central directory size or extent mismatch"));
        }
        return Ok(ZipDirectory {
            start: directory_start,
            end: directory_end,
            count,
        });
    }
    Err(zip_error("missing complete end-of-directory record"))
}

fn unicode_path(
    archive: &[u8],
    range: std::ops::Range<usize>,
    raw_name: &[u8],
    neutralized_fields: &mut Vec<usize>,
) -> Result<Option<String>, PackageError> {
    let mut offset = range.start;
    let mut seen = false;
    let mut decoded = None;
    while offset < range.end {
        let header = archive
            .get(offset..offset + 4)
            .ok_or_else(|| zip_error("truncated ZIP extra field"))?;
        let id = u16::from_le_bytes([header[0], header[1]]);
        let len = u16::from_le_bytes([header[2], header[3]]) as usize;
        let payload_start = offset + 4;
        let payload_end = payload_start
            .checked_add(len)
            .filter(|end| *end <= range.end)
            .ok_or_else(|| zip_error("truncated ZIP extra field payload"))?;
        if id == 0x0001 {
            return Err(zip_error("ZIP64 member metadata is unsupported"));
        }
        if id == 0x7075 {
            if seen {
                return Err(zip_error("duplicate Unicode path field"));
            }
            seen = true;
            let payload = archive
                .get(payload_start..payload_end)
                .ok_or_else(|| zip_error("truncated Unicode path field"))?;
            if payload.len() < 5 {
                return Err(zip_error("truncated Unicode path field"));
            }
            let version = payload[0];
            let crc = u32::from_le_bytes(payload[1..5].try_into().unwrap());
            if version != 1 || crc != crc32fast::hash(raw_name) {
                // APPNOTE 4.6.9 says an unknown version or stale CRC does not
                // override the header name. Hide only this ignored field from
                // `zip`, whose v8.6 parser otherwise rejects stale CRCs.
                neutralized_fields.push(offset);
            } else {
                let name = std::str::from_utf8(&payload[5..])
                    .map_err(|_| zip_error("invalid Unicode path UTF-8"))?;
                decoded = Some(name.to_string());
            }
        }
        offset = payload_end;
    }
    Ok(decoded)
}

fn data_descriptor_end(
    archive: &[u8],
    start: usize,
    crc32: u32,
    compressed_size: u32,
    uncompressed_size: u32,
) -> Result<usize, PackageError> {
    let expected = [crc32, compressed_size, uncompressed_size];
    for signature_len in [4_usize, 0] {
        if signature_len == 4 && archive.get(start..start + 4) != Some(b"PK\x07\x08") {
            continue;
        }
        let payload_start = start
            .checked_add(signature_len)
            .ok_or_else(|| zip_error("data descriptor length overflow"))?;
        let payload = archive
            .get(payload_start..payload_start + 12)
            .ok_or_else(|| zip_error("truncated data descriptor"))?;
        let actual = [
            u32::from_le_bytes(payload[0..4].try_into().unwrap()),
            u32::from_le_bytes(payload[4..8].try_into().unwrap()),
            u32::from_le_bytes(payload[8..12].try_into().unwrap()),
        ];
        if actual == expected {
            return payload_start
                .checked_add(12)
                .ok_or_else(|| zip_error("data descriptor length overflow"));
        }
    }
    Err(zip_error("data descriptor differs from central directory"))
}

// ZipArchive indexes by name and collapses duplicate entries. Inspect the
// physical central and local headers first so no input member can disappear or
// acquire two identities. Return the offsets of ignored Unicode-path fields so
// the parser view can neutralize them without copying the whole archive.
fn validate_central_directory(
    archive: &[u8],
    directory: ZipDirectory,
) -> Result<ValidatedZipDirectory, PackageError> {
    let mut offset = directory.start;
    let mut paths = HashSet::new();
    let mut local_offsets = HashSet::new();
    let mut local_ranges = Vec::with_capacity(directory.count);
    let mut checked_entries = Vec::with_capacity(directory.count);
    let mut neutralized_fields = Vec::new();
    let mut central_comment_ranges = Vec::new();
    let mut local_metadata_size = 0_usize;
    for _ in 0..directory.count {
        if archive.get(offset..offset + 4) != Some(b"PK\x01\x02") {
            return Err(zip_error("truncated central directory"));
        }
        let header = archive
            .get(offset..offset + 46)
            .ok_or_else(|| zip_error("truncated central directory"))?
            .to_vec();
        let flags = u16::from_le_bytes([header[8], header[9]]);
        let method = u16::from_le_bytes([header[10], header[11]]);
        let crc32 = u32::from_le_bytes(header[16..20].try_into().unwrap());
        let compressed_size = u32::from_le_bytes(header[20..24].try_into().unwrap());
        let uncompressed_size = u32::from_le_bytes(header[24..28].try_into().unwrap());
        let name_len = u16::from_le_bytes([header[28], header[29]]) as usize;
        let extra_len = u16::from_le_bytes([header[30], header[31]]) as usize;
        let comment_len = u16::from_le_bytes([header[32], header[33]]) as usize;
        let disk_start = u16::from_le_bytes([header[34], header[35]]);
        if disk_start != 0 {
            return Err(zip_error("central member starts on another disk"));
        }
        let local_offset = u32::from_le_bytes(header[42..46].try_into().unwrap()) as usize;
        if !local_offsets.insert(local_offset) {
            return Err(zip_error("multiple members reference one local header"));
        }
        let name_start = offset
            .checked_add(46)
            .ok_or_else(|| zip_error("central directory length overflow"))?;
        let name_end = name_start
            .checked_add(name_len)
            .ok_or_else(|| zip_error("central directory length overflow"))?;
        let extra_end = name_end
            .checked_add(extra_len)
            .ok_or_else(|| zip_error("central directory length overflow"))?;
        let record_end = extra_end
            .checked_add(comment_len)
            .filter(|end| *end <= directory.end)
            .ok_or_else(|| zip_error("central directory size or extent mismatch"))?;
        if extra_end != record_end {
            central_comment_ranges.push(extra_end..record_end);
        }
        let name = archive
            .get(name_start..name_end)
            .ok_or_else(|| zip_error("truncated central directory name"))?
            .to_vec();
        if !paths.insert(name.clone()) {
            return Err(PackageError::DuplicateMember {
                path: String::from_utf8_lossy(&name).into_owned(),
            });
        }
        if flags & (1 << 11) != 0 && std::str::from_utf8(&name).is_err() {
            return Err(zip_error("member name has invalid flagged UTF-8"));
        }

        let local = archive
            .get(local_offset..local_offset + 30)
            .ok_or_else(|| zip_error("truncated local file header"))?
            .to_vec();
        if local.get(..4) != Some(b"PK\x03\x04") {
            return Err(zip_error("invalid local file header"));
        }
        let local_flags = u16::from_le_bytes([local[6], local[7]]);
        let local_method = u16::from_le_bytes([local[8], local[9]]);
        let local_crc32 = u32::from_le_bytes(local[14..18].try_into().unwrap());
        let local_compressed_size = u32::from_le_bytes(local[18..22].try_into().unwrap());
        let local_uncompressed_size = u32::from_le_bytes(local[22..26].try_into().unwrap());
        let local_name_len = u16::from_le_bytes([local[26], local[27]]) as usize;
        let local_extra_len = u16::from_le_bytes([local[28], local[29]]) as usize;
        local_metadata_size = local_metadata_size
            .checked_add(30 + local_name_len + local_extra_len)
            .filter(|size| *size <= MAX_LOCAL_METADATA_SIZE)
            .ok_or_else(|| PackageError::SizeLimit {
                path: "local header metadata".into(),
            })?;
        let local_name_start = local_offset
            .checked_add(30)
            .ok_or_else(|| zip_error("local header length overflow"))?;
        let local_name_end = local_name_start
            .checked_add(local_name_len)
            .ok_or_else(|| zip_error("local header length overflow"))?;
        let local_extra_end = local_name_end
            .checked_add(local_extra_len)
            .filter(|end| *end <= directory.start)
            .ok_or_else(|| zip_error("local header length overflow"))?;
        let local_name = archive
            .get(local_name_start..local_name_end)
            .ok_or_else(|| zip_error("truncated local member name"))?
            .to_vec();
        if local_name != name {
            return Err(zip_error("local and central member names differ"));
        }
        if local_flags != flags || local_method != method {
            return Err(zip_error("local and central member metadata differ"));
        }
        let uses_descriptor = flags & (1 << 3) != 0;
        if uses_descriptor {
            if local_crc32 != 0 || local_compressed_size != 0 || local_uncompressed_size != 0 {
                return Err(zip_error("invalid local data descriptor placeholders"));
            }
        } else if (local_crc32, local_compressed_size, local_uncompressed_size)
            != (crc32, compressed_size, uncompressed_size)
        {
            return Err(zip_error("local and central CRC or sizes differ"));
        }
        let data_end = local_extra_end
            .checked_add(compressed_size as usize)
            .ok_or_else(|| zip_error("local file record length overflow"))?;
        let local_end = if uses_descriptor {
            data_descriptor_end(archive, data_end, crc32, compressed_size, uncompressed_size)?
        } else {
            data_end
        };
        if local_end > directory.start {
            return Err(zip_error("local file record overlaps central directory"));
        }
        local_ranges.push(local_offset..local_end);

        let local_unicode = unicode_path(
            archive,
            local_name_end..local_extra_end,
            &name,
            &mut neutralized_fields,
        )?;
        let central_unicode =
            unicode_path(archive, name_end..extra_end, &name, &mut neutralized_fields)?;
        if local_unicode != central_unicode {
            return Err(zip_error("local and central Unicode path fields differ"));
        }
        if flags & (1 << 11) != 0
            && central_unicode
                .as_deref()
                .is_some_and(|unicode| unicode.as_bytes() != name)
        {
            return Err(zip_error("flagged UTF-8 and Unicode path field differ"));
        }
        checked_entries.push(CheckedZipEntry {
            central_offset: offset as u64,
            local_offset: local_offset as u64,
            crc32,
            compressed_size: u64::from(compressed_size),
            uncompressed_size: u64::from(uncompressed_size),
        });
        offset = record_end;
    }
    if offset != directory.end {
        return Err(zip_error("central directory size or extent mismatch"));
    }
    local_ranges.sort_unstable_by_key(|range| range.start);
    if local_ranges
        .windows(2)
        .any(|pair| pair[0].end > pair[1].start)
    {
        return Err(zip_error("overlapping local file records"));
    }
    neutralized_fields.sort_unstable();
    Ok(ValidatedZipDirectory {
        entries: checked_entries,
        neutralized_fields,
        parser_comment_ranges: central_comment_ranges,
    })
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

fn package_member_role(path: &str, kind: FileKind) -> String {
    if path == "knx_master.xml" {
        "Master".into()
    } else if path.ends_with(".signature") {
        // This classifies retained bytes; it is not a cryptographic verdict.
        "Signature".into()
    } else if path.contains("/Baggages/") {
        "Baggage".into()
    } else if manufacturer_partition(path).is_none() || matches!(kind, FileKind::Baggage) {
        "Unrecognized".into()
    } else {
        format!("{kind:?}")
    }
}

/// Install a standalone product package, preserving the archive and every file.
/// All storage and parsing share one transaction. Identical archive bytes are
/// revalidated and re-extracted, but skip domain ingestion and database
/// mutation, including when the caller supplies a different source name.
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
    let prior: Option<(u32, usize, TranslationCounts, usize)> = tx
        .query_row(
            "SELECT scheme, unknown_count, translation_program_count, translation_catalog_count,
                    translation_hardware_count, translation_master_count,
                    dropped_datapoint_type_count
             FROM package WHERE sha256 = ?1",
            [&sha256],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get::<_, i64>(1)? as usize,
                    TranslationCounts {
                        program: r.get::<_, i64>(2)? as usize,
                        catalog: r.get::<_, i64>(3)? as usize,
                        hardware: r.get::<_, i64>(4)? as usize,
                        master: r.get::<_, i64>(5)? as usize,
                    },
                    r.get::<_, i64>(6)? as usize,
                ))
            },
        )
        .optional()?;
    let directory = preflight_zip(bytes)?;
    let ValidatedZipDirectory {
        entries: checked_entries,
        neutralized_fields,
        mut parser_comment_ranges,
    } = validate_central_directory(bytes, directory)?;
    let eocd_comment_start = directory
        .end
        .checked_add(22)
        .ok_or_else(|| zip_error("end-of-directory length overflow"))?;
    if eocd_comment_start != bytes.len() {
        parser_comment_ranges.push(eocd_comment_start..bytes.len());
    }
    let mut central_view = bytes[directory.start..].to_vec();
    for &offset in &neutralized_fields {
        if (directory.start..directory.end).contains(&offset) {
            let relative = offset - directory.start;
            central_view[relative..relative + 2].copy_from_slice(&0xA11E_u16.to_le_bytes());
        }
    }
    let relative_eocd = directory.end - directory.start;
    central_view[relative_eocd + 16..relative_eocd + 20].copy_from_slice(&0_u32.to_le_bytes());
    let mut comment_index = 0;
    for relative in 0..central_view.len().saturating_sub(3) {
        if central_view[relative..relative + 4] != *b"PK\x05\x06" {
            continue;
        }
        let absolute = directory.start + relative;
        if absolute == directory.end {
            continue;
        }
        while comment_index < parser_comment_ranges.len()
            && parser_comment_ranges[comment_index].end <= absolute
        {
            comment_index += 1;
        }
        let safely_ignorable = parser_comment_ranges
            .get(comment_index)
            .is_some_and(|range| range.start <= absolute && absolute + 4 <= range.end);
        if !safely_ignorable {
            return Err(zip_error(
                "end-of-directory signature inside member metadata",
            ));
        }
        // Comments are retained in the stored original, but have no effect on
        // member identity. Hiding their signatures binds `zip` to the one EOCD
        // selected and range-checked above.
        central_view[relative..relative + 4].fill(0);
    }
    let metadata_archive = zip::ZipArchive::with_config(
        zip::read::Config {
            archive_offset: zip::read::ArchiveOffset::Known(0),
        },
        Cursor::new(central_view),
    )
    .map_err(zip_error)?;
    let metadata = metadata_archive.metadata();
    // SAFETY: `metadata` was parsed from this archive's already-validated,
    // selected central directory. Its local offsets remain absolute and are
    // cross-checked below before any member is read.
    let mut archive = unsafe {
        zip::ZipArchive::unsafe_new_with_metadata(
            ZipParserReader::new(bytes, neutralized_fields),
            metadata,
        )
    };
    if archive.len() != directory.count {
        return Err(PackageError::DuplicateMember {
            path: "aliased ZIP member names".into(),
        });
    }
    for (index, checked) in checked_entries.iter().enumerate() {
        let parsed = archive.by_index_raw(index).map_err(zip_error)?;
        if parsed.central_header_start() + directory.start as u64 != checked.central_offset
            || parsed.header_start() != checked.local_offset
            || parsed.crc32() != checked.crc32
            || parsed.compressed_size() != checked.compressed_size
            || parsed.size() != checked.uncompressed_size
        {
            return Err(zip_error("ZIP parser selected unchecked member metadata"));
        }
    }
    let mut paths = MemberPathTree::default();
    let mut has_master = false;
    let mut decoded_path_size = 0_usize;
    let mut total = 0_u64;
    for i in 0..archive.len() {
        let file = archive.by_index_raw(i).map_err(zip_error)?;
        let path = file.name().to_string();
        let path = path.as_str();
        decoded_path_size = decoded_path_size
            .checked_add(path.len())
            .filter(|size| *size <= MAX_DECODED_PATH_SIZE)
            .ok_or_else(|| PackageError::SizeLimit {
                path: "decoded member paths".into(),
            })?;
        let normalized = path.strip_suffix('/').unwrap_or(path);
        let is_directory = file.is_dir();
        if normalized.is_empty()
            || path.contains(['\\', ':', '\0'])
            || normalized
                .split('/')
                .any(|part| matches!(part, "" | "." | ".."))
            || file.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000)
        {
            return Err(PackageError::UnsafeMember { path: path.into() });
        }
        match paths.insert(normalized, is_directory) {
            Ok(()) => {}
            Err(PathInsertError::Collision) => {
                return Err(PackageError::DuplicateMember { path: path.into() });
            }
            Err(PathInsertError::NodeLimit) => {
                return Err(PackageError::SizeLimit {
                    path: "decoded path component budget".into(),
                });
            }
        }
        has_master |= normalized == "knx_master.xml";
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
    if !has_master {
        return Err(PackageError::MissingMaster);
    }
    let mut validated_members = Vec::new();
    let mut scheme = None;
    let mut has_manufacturer_data = false;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(zip_error)?;
        let path = file.name().to_string();
        let is_directory = file.is_dir();
        let mut data = Vec::new();
        (&mut file)
            .take(MAX_MEMBER_SIZE + 1)
            .read_to_end(&mut data)
            .map_err(zip_error)?;
        if data.len() as u64 > MAX_MEMBER_SIZE {
            return Err(PackageError::SizeLimit { path });
        }
        if data.len() as u64 != file.size() {
            return Err(zip_error(format!("size mismatch for {path}")));
        }
        if is_directory {
            if !data.is_empty() {
                return Err(PackageError::UnsafeMember { path });
            }
            continue;
        }
        if path.ends_with(".xml") {
            validate_xml(&path, &data)?;
        }
        let kind = classify(&data);
        let role = package_member_role(&path, kind);
        if matches!(role.as_str(), "Catalog" | "Hardware" | "ApplicationProgram") {
            validate_manufacturer(&path, &data)?;
        }
        if path == "knx_master.xml" {
            scheme = Some(master_scheme(&data)?);
        }
        has_manufacturer_data |= manufacturer_partition(&path).is_some()
            && matches!(
                kind,
                FileKind::Catalog | FileKind::Hardware | FileKind::ApplicationProgram
            );
        validated_members.push(ValidatedMember {
            archive_index: i,
            member: PackageMember {
                path,
                role,
                sha256: sha256_hex(&data),
                size: data.len() as u64,
            },
        });
    }
    let scheme = scheme.ok_or(PackageError::MissingMaster)?;
    if !has_manufacturer_data {
        return Err(PackageError::MissingManufacturerData);
    }
    if let Some((stored_scheme, unknown, translations, dropped_datapoint_types)) = prior {
        let members = tx.prepare("SELECT path, role, source_sha256, size FROM package_member WHERE package_sha256 = ?1 ORDER BY ordinal")?.query_map([&sha256], |r| Ok(PackageMember { path: r.get(0)?, role: r.get(1)?, sha256: r.get(2)?, size: r.get::<_, i64>(3)? as u64 }))?.collect::<Result<Vec<_>, _>>()?;
        let member_index_matches = members.len() == validated_members.len()
            && members
                .iter()
                .eq(validated_members.iter().map(|validated| &validated.member));
        if stored_scheme != scheme || !member_index_matches {
            return Err(zip_error(
                "stored package member index differs from validated archive",
            ));
        }
        let conflicts = package_conflicts(&tx, &sha256)?;
        tx.commit().map_err(ProductDbError::from)?;
        return Ok(InstallReport {
            sha256,
            scheme,
            skipped: true,
            members,
            unknown,
            conflicts,
            translations,
            dropped_datapoint_types,
        });
    }
    tx.execute("INSERT INTO package (sha256, source_name, scheme, size, bytes, unknown_count) VALUES (?1, ?2, ?3, ?4, ?5, 0)", params![sha256, source_name, scheme, bytes.len() as i64, bytes])?;
    let mut report = InstallReport {
        sha256,
        scheme,
        skipped: false,
        dropped_datapoint_types: 0,
        members: Vec::new(),
        unknown: 0,
        conflicts: Vec::new(),
        translations: TranslationCounts::default(),
    };
    for (ordinal, validated) in validated_members.into_iter().enumerate() {
        let PackageMember {
            path,
            role,
            sha256: member_sha,
            size,
        } = validated.member;
        let mut file = archive
            .by_index(validated.archive_index)
            .map_err(zip_error)?;
        let mut data = Vec::new();
        (&mut file)
            .take(MAX_MEMBER_SIZE + 1)
            .read_to_end(&mut data)
            .map_err(zip_error)?;
        if data.len() as u64 != size || sha256_hex(&data) != member_sha {
            return Err(zip_error("member changed between validation passes"));
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
            unknown,
            conflicts,
            translations,
            ..
        } = outcome
        {
            report.unknown += unknown;
            report.conflicts.extend(conflicts);
            report.translations.add(translations);
        }
        if role == "Master" {
            let master = crate::ingest_master_data(&tx, &data)?;
            insert_unknown(&tx, &member_sha, &master.unknown)?;
            report.unknown += master.unknown.len();
            report.translations.master += master.translations;
            report.dropped_datapoint_types += master.dropped_datapoint_types;
        } else if role == "Unrecognized" || role == "Baggages" {
            let mut unknown = UnknownCollector::default();
            unknown.element("/Package", &path);
            let unknown = unknown.into_vec();
            insert_unknown(&tx, &member_sha, &unknown)?;
            report.unknown += unknown.len();
        }
        tx.execute("INSERT INTO package_member (package_sha256, ordinal, path, role, source_sha256, size) VALUES (?1, ?2, ?3, ?4, ?5, ?6)", params![report.sha256, ordinal as i64, path, role, member_sha, size as i64])?;
        report.members.push(PackageMember {
            path,
            role,
            sha256: member_sha,
            size,
        });
    }
    tx.execute(
        "UPDATE package SET unknown_count = ?2, translation_program_count = ?3,
                translation_catalog_count = ?4, translation_hardware_count = ?5,
                translation_master_count = ?6, dropped_datapoint_type_count = ?7
         WHERE sha256 = ?1",
        params![
            report.sha256,
            report.unknown as i64,
            report.translations.program as i64,
            report.translations.catalog as i64,
            report.translations.hardware as i64,
            report.translations.master as i64,
            report.dropped_datapoint_types as i64,
        ],
    )?;
    for (ordinal, conflict) in report.conflicts.iter().enumerate() {
        tx.execute(
            "INSERT INTO package_conflict (package_sha256, ordinal, table_name, logical_id, kept_sha256, other_sha256, occurrence) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                report.sha256,
                ordinal as i64,
                conflict.table,
                conflict.id,
                conflict.kept_sha256,
                conflict.other_sha256,
                conflict.occurrence,
            ],
        )?;
    }
    report.conflicts = package_conflicts(&tx, &report.sha256)?;
    tx.commit().map_err(ProductDbError::from)?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_overlay_survives_split_reads_and_seeks() {
        let input = b"0123456789abcdef";
        let mut expected = input.to_vec();
        expected[2..4].copy_from_slice(&0xA11E_u16.to_le_bytes());
        expected[11..13].copy_from_slice(&0xA11E_u16.to_le_bytes());

        for chunk_size in 1..=5 {
            let mut reader = ZipParserReader::new(input, vec![2, 11]);
            let mut actual = Vec::new();
            loop {
                let mut chunk = vec![0; chunk_size];
                let read = reader.read(&mut chunk).unwrap();
                if read == 0 {
                    break;
                }
                actual.extend_from_slice(&chunk[..read]);
            }
            assert_eq!(actual, expected, "chunk size {chunk_size}");
        }

        for start in 0..=input.len() {
            let mut reader = ZipParserReader::new(input, vec![2, 11]);
            reader.seek(SeekFrom::Start(start as u64)).unwrap();
            let mut actual = Vec::new();
            reader.read_to_end(&mut actual).unwrap();
            assert_eq!(actual, expected[start..], "seek offset {start}");

            reader.seek(SeekFrom::Start(start as u64)).unwrap();
            let mut one_byte_at_a_time = Vec::new();
            let mut byte = [0];
            while reader.read(&mut byte).unwrap() != 0 {
                one_byte_at_a_time.push(byte[0]);
            }
            assert_eq!(
                one_byte_at_a_time,
                expected[start..],
                "one-byte reads after seek {start}"
            );
        }
    }
}
