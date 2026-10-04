//! Evidence-backed, atomic installation of readable scheme 11/12/13/14/20/21 product ZIPs.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;
use std::io::{Cursor, Read, Seek, SeekFrom};

use quick_xml::events::Event;
use quick_xml::name::ResolveResult;
use rusqlite::{params, Connection, OptionalExtension};

use crate::ingest::{classify, ingest_file_in_transaction, DetailedIngestOutcome};
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
const MASTER_DIAGNOSTIC_PREFIX: &str = "/KNX/MasterData/";
const BAGGAGE_DIAGNOSTIC_XML_PATH: &str = "/KNX/ManufacturerData/Manufacturer/Baggages/Baggage";
/// A payload is a whole archive member, not an XML node: its diagnostic
/// path is the document root.
const PAYLOAD_DIAGNOSTIC_XML_PATH: &str = "/";
const UNDECLARED_PAYLOAD_DETAIL: &str =
    "baggage payload is retained but no Baggages.xml declaration names it";

#[derive(Debug)]
pub enum PackageError {
    LegacyVd2 { sha256: String, len: usize },
    UnsupportedLegacyFormat { extension: String },
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
            Self::UnsupportedLegacyFormat { extension } => write!(
                f,
                "legacy ETS filename extension .{extension} is unsupported; legacy import is not implemented"
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

/// Closed vocabulary for persisted install evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InstallCategory {
    ArchiveMember,
    Product,
    ApplicationProgram,
    Parameter,
    CommunicationObject,
    DynamicNode,
    Module,
    BaggageIndex,
    Baggage,
    UnknownConstruct,
    MasterSection,
    /// PDB-8: an element subtree inside a *supported* master section that
    /// the parser does not interpret (see `parse::master`).
    MasterSubtree,
    DatapointType,
}

impl InstallCategory {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ArchiveMember => "archive_member",
            Self::Product => "product",
            Self::ApplicationProgram => "application_program",
            Self::Parameter => "parameter",
            Self::CommunicationObject => "communication_object",
            Self::DynamicNode => "dynamic_node",
            Self::Module => "module",
            Self::BaggageIndex => "baggage_index",
            Self::Baggage => "baggage",
            Self::UnknownConstruct => "unknown_construct",
            Self::MasterSection => "master_section",
            Self::MasterSubtree => "master_subtree",
            Self::DatapointType => "datapoint_type",
        }
    }
    fn from_db(value: String) -> Result<Self, ProductDbError> {
        match value.as_str() {
            "archive_member" => Ok(Self::ArchiveMember),
            "product" => Ok(Self::Product),
            "application_program" => Ok(Self::ApplicationProgram),
            "parameter" => Ok(Self::Parameter),
            "communication_object" => Ok(Self::CommunicationObject),
            "dynamic_node" => Ok(Self::DynamicNode),
            "module" => Ok(Self::Module),
            "baggage_index" => Ok(Self::BaggageIndex),
            "baggage" => Ok(Self::Baggage),
            "unknown_construct" => Ok(Self::UnknownConstruct),
            "master_section" => Ok(Self::MasterSection),
            "master_subtree" => Ok(Self::MasterSubtree),
            "datapoint_type" => Ok(Self::DatapointType),
            _ => Err(ProductDbError::Xml {
                source_path: "package_install_count".into(),
                cause: format!("invalid category {value:?}"),
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InstallDisposition {
    Read,
    Stored,
    Deduplicated,
    RetainedButUninterpreted,
    Unsupported,
    Dropped,
}

impl InstallDisposition {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Stored => "stored",
            Self::Deduplicated => "deduplicated",
            Self::RetainedButUninterpreted => "retained-but-uninterpreted",
            Self::Unsupported => "unsupported",
            Self::Dropped => "dropped",
        }
    }
    fn from_db(value: String) -> Result<Self, ProductDbError> {
        match value.as_str() {
            "read" => Ok(Self::Read),
            "stored" => Ok(Self::Stored),
            "deduplicated" => Ok(Self::Deduplicated),
            "retained-but-uninterpreted" => Ok(Self::RetainedButUninterpreted),
            "unsupported" => Ok(Self::Unsupported),
            "dropped" => Ok(Self::Dropped),
            _ => Err(ProductDbError::Xml {
                source_path: "package_install_count".into(),
                cause: format!("invalid disposition {value:?}"),
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallCount {
    pub category: InstallCategory,
    pub disposition: InstallDisposition,
    pub count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InstallFacts {
    pub counts: Vec<InstallCount>,
    pub unknown_constructs: Vec<crate::report::UnknownConstruct>,
    pub unknown_occurrences: u64,
    pub diagnostics: Vec<InstallDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallDiagnostic {
    kind: InstallDiagnosticKind,
    archive_path: String,
    xml_path: String,
    detail: String,
    occurrences: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum InstallDiagnosticKind {
    UnsupportedMasterSection,
    UnsupportedMasterSubtree,
    UnresolvedBaggageDeclaration,
    UndeclaredBaggagePayload,
}

impl InstallDiagnosticKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UnsupportedMasterSection => "unsupported-master-section",
            Self::UnsupportedMasterSubtree => "unsupported-master-subtree",
            Self::UnresolvedBaggageDeclaration => "unresolved-baggage-declaration",
            Self::UndeclaredBaggagePayload => "undeclared-baggage-payload",
        }
    }

    fn from_db(value: &str) -> Result<Self, ProductDbError> {
        match value {
            "unsupported-master-section" => Ok(Self::UnsupportedMasterSection),
            "unsupported-master-subtree" => Ok(Self::UnsupportedMasterSubtree),
            "unresolved-baggage-declaration" => Ok(Self::UnresolvedBaggageDeclaration),
            "undeclared-baggage-payload" => Ok(Self::UndeclaredBaggagePayload),
            _ => Err(report_error(format!("invalid diagnostic kind {value:?}"))),
        }
    }
}

impl InstallDiagnostic {
    fn new(
        kind: InstallDiagnosticKind,
        archive_path: String,
        xml_path: String,
        detail: String,
        occurrences: u64,
    ) -> Result<Self, ProductDbError> {
        validate_archive_path(&archive_path)?;
        validate_xml_path(&xml_path)?;
        if occurrences == 0 {
            return Err(report_error("diagnostic occurrences must be positive"));
        }
        Ok(Self {
            kind,
            archive_path,
            xml_path,
            detail,
            occurrences,
        })
    }

    pub const fn kind(&self) -> InstallDiagnosticKind {
        self.kind
    }

    pub fn archive_path(&self) -> &str {
        &self.archive_path
    }

    pub fn xml_path(&self) -> &str {
        &self.xml_path
    }

    pub fn detail(&self) -> &str {
        &self.detail
    }

    pub const fn occurrences(&self) -> u64 {
        self.occurrences
    }
}

impl InstallFacts {
    fn sort(&mut self) -> Result<(), ProductDbError> {
        self.counts
            .sort_by(|a, b| (&a.category, &a.disposition).cmp(&(&b.category, &b.disposition)));
        self.unknown_constructs.sort_by(|a, b| {
            (&a.xpath, a.kind.as_str(), &a.name).cmp(&(&b.xpath, b.kind.as_str(), &b.name))
        });
        self.diagnostics.sort_by(|a, b| {
            (a.kind, &a.archive_path, &a.xml_path, &a.detail).cmp(&(
                b.kind,
                &b.archive_path,
                &b.xml_path,
                &b.detail,
            ))
        });
        self.unknown_occurrences =
            self.unknown_constructs
                .iter()
                .try_fold(0u64, |total, unknown| {
                    total
                        .checked_add(u64::from(unknown.occurrences))
                        .ok_or_else(|| report_error("unknown occurrence counter overflow"))
                })?;
        Ok(())
    }
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
    /// None means the package predates the v12 encounter/write ledger.
    pub facts: Option<InstallFacts>,
    /// PDB-10 baggage inventory. `None` only when the v16 upgrade could not
    /// measure it from retained bytes; a fresh install always measures.
    pub baggage: Option<crate::baggage::BaggageInventory>,
    /// PDB-11: every source name these package bytes arrived under, sorted,
    /// including this call's. The name never decides identity (ADR-0043).
    pub source_names: Vec<String>,
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

fn report_error(cause: impl Into<String>) -> ProductDbError {
    ProductDbError::Xml {
        source_path: "package install report".into(),
        cause: cause.into(),
    }
}

fn validate_archive_path(path: &str) -> Result<(), ProductDbError> {
    if path.is_empty()
        || path.starts_with('/')
        || path.contains(['\\', ':', '\0'])
        || path
            .split('/')
            .any(|component| matches!(component, "" | "." | ".."))
    {
        return Err(report_error(format!(
            "diagnostic archive path is not normalized: {path:?}"
        )));
    }
    Ok(())
}

fn validate_xml_path(path: &str) -> Result<(), ProductDbError> {
    if !path.starts_with('/')
        || path.contains(['\\', '\0'])
        || (path != "/"
            && path[1..]
                .split('/')
                .any(|component| matches!(component, "" | "." | "..")))
    {
        return Err(report_error(format!(
            "diagnostic XML path is not normalized and absolute: {path:?}"
        )));
    }
    Ok(())
}

const UNRESOLVED_DECLARATION_PREFIX: &str = "baggage declaration does not resolve: ";

/// Diagnostics for a measured inventory: one per (index, reason) group of
/// unresolved declarations and one per undeclared payload.
pub(crate) fn baggage_diagnostics(
    inventory: &crate::baggage::BaggageInventory,
) -> Result<Vec<InstallDiagnostic>, ProductDbError> {
    let mut out = Vec::new();
    for ((index_path, reason), count) in inventory.unresolved_groups() {
        out.push(InstallDiagnostic::new(
            InstallDiagnosticKind::UnresolvedBaggageDeclaration,
            index_path,
            BAGGAGE_DIAGNOSTIC_XML_PATH.into(),
            format!("{UNRESOLVED_DECLARATION_PREFIX}{reason}"),
            count,
        )?);
    }
    for payload in inventory.undeclared() {
        out.push(InstallDiagnostic::new(
            InstallDiagnosticKind::UndeclaredBaggagePayload,
            payload.member_path.clone(),
            PAYLOAD_DIAGNOSTIC_XML_PATH.into(),
            UNDECLARED_PAYLOAD_DETAIL.into(),
            1,
        )?);
    }
    Ok(out)
}

fn master_diagnostic_detail(section: &str) -> String {
    format!("master section {section} is retained but not interpreted")
}

/// `relative` is slash-joined below `/KNX/MasterData`; the detail names
/// the subtree's root element, the path carries where it sits.
pub(crate) fn master_subtree_diagnostic_detail(relative: &str) -> String {
    let leaf = relative.rsplit('/').next().unwrap_or(relative);
    format!("master subtree {leaf} is retained but not interpreted")
}

/// The one constructor for a subtree diagnostic, shared by install and the
/// v13 -> v14 backfill so both write byte-identical rows.
pub(crate) fn master_subtree_diagnostic(
    archive_path: String,
    subtree: &crate::parse::master::UninterpretedMasterSubtree,
) -> Result<InstallDiagnostic, ProductDbError> {
    InstallDiagnostic::new(
        InstallDiagnosticKind::UnsupportedMasterSubtree,
        archive_path,
        format!("{MASTER_DIAGNOSTIC_PREFIX}{}", subtree.path),
        master_subtree_diagnostic_detail(&subtree.path),
        subtree.occurrences,
    )
}

fn is_xml_local_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|first| first == '_' || first.is_alphabetic())
        && chars.all(|ch| ch == '_' || ch == '-' || ch == '.' || ch.is_alphanumeric())
}

fn validate_diagnostic(
    conn: &Connection,
    sha256: &str,
    diagnostic: &InstallDiagnostic,
) -> Result<(), ProductDbError> {
    validate_archive_path(&diagnostic.archive_path)?;
    validate_xml_path(&diagnostic.xml_path)?;
    if diagnostic.occurrences == 0 {
        return Err(report_error("diagnostic occurrences must be positive"));
    }

    let role: Option<String> = conn
        .query_row(
            "SELECT role FROM package_member WHERE package_sha256 = ?1 AND path = ?2",
            params![sha256, diagnostic.archive_path],
            |row| row.get(0),
        )
        .optional()?;
    let Some(role) = role else {
        return Err(report_error(
            "diagnostic archive path is not a package member",
        ));
    };

    match diagnostic.kind {
        InstallDiagnosticKind::UnsupportedMasterSection => {
            if role != "Master" {
                return Err(report_error(
                    "unsupported-master-section diagnostic does not identify the Master member",
                ));
            }
            let section = diagnostic
                .xml_path
                .strip_prefix(MASTER_DIAGNOSTIC_PREFIX)
                .filter(|section| is_xml_local_name(section))
                .ok_or_else(|| {
                    report_error("unsupported-master-section diagnostic has a noncanonical path")
                })?;
            if crate::parse::master::is_supported_master_section(section) {
                return Err(report_error(
                    "unsupported-master-section diagnostic identifies a supported section",
                ));
            }
            if diagnostic.detail != master_diagnostic_detail(section) {
                return Err(report_error(
                    "unsupported-master-section diagnostic has noncanonical detail",
                ));
            }
        }
        InstallDiagnosticKind::UnsupportedMasterSubtree => {
            if role != "Master" {
                return Err(report_error(
                    "unsupported-master-subtree diagnostic does not identify the Master member",
                ));
            }
            let relative = diagnostic
                .xml_path
                .strip_prefix(MASTER_DIAGNOSTIC_PREFIX)
                .filter(|relative| relative.split('/').all(is_xml_local_name))
                .ok_or_else(|| {
                    report_error("unsupported-master-subtree diagnostic has a noncanonical path")
                })?;
            // Parent interpreted, itself not: excludes interpreted structure,
            // descendants of an uninterpreted root, and whole unsupported
            // sections (whose parent `MasterData` is not on the list).
            if !crate::parse::master::is_uninterpreted_master_subtree_root(relative) {
                return Err(report_error(
                    "unsupported-master-subtree diagnostic does not identify an uninterpreted subtree root",
                ));
            }
            if diagnostic.detail != master_subtree_diagnostic_detail(relative) {
                return Err(report_error(
                    "unsupported-master-subtree diagnostic has noncanonical detail",
                ));
            }
        }
        InstallDiagnosticKind::UnresolvedBaggageDeclaration => {
            if role != "Baggages" || diagnostic.xml_path != BAGGAGE_DIAGNOSTIC_XML_PATH {
                return Err(report_error(
                    "unresolved-baggage-declaration diagnostic does not identify a Baggages declaration",
                ));
            }
            // The detail is the stored reason; the count must be exactly the
            // unresolved declarations of this index that carry it.
            let reason = diagnostic
                .detail
                .strip_prefix(UNRESOLVED_DECLARATION_PREFIX)
                .ok_or_else(|| {
                    report_error(
                        "unresolved-baggage-declaration diagnostic has noncanonical detail",
                    )
                })?;
            let matching: i64 = conn.query_row(
                "SELECT count(*) FROM package_baggage_declaration
                 WHERE package_sha256 = ?1 AND index_path = ?2 AND detail = ?3
                   AND resolution <> 'resolved'",
                params![sha256, diagnostic.archive_path, reason],
                |row| row.get(0),
            )?;
            if i64_to_u64(matching, "unresolved declarations")? != diagnostic.occurrences {
                return Err(report_error(
                    "unresolved-baggage-declaration diagnostic disagrees with the inventory",
                ));
            }
        }
        InstallDiagnosticKind::UndeclaredBaggagePayload => {
            if role != "Baggage"
                || diagnostic.xml_path != PAYLOAD_DIAGNOSTIC_XML_PATH
                || diagnostic.detail != UNDECLARED_PAYLOAD_DETAIL
                || diagnostic.occurrences != 1
            {
                return Err(report_error(
                    "undeclared-baggage-payload diagnostic has a noncanonical shape",
                ));
            }
            let declarations: Option<i64> = conn
                .query_row(
                    "SELECT declarations FROM package_baggage_payload
                     WHERE package_sha256 = ?1 AND member_path = ?2",
                    params![sha256, diagnostic.archive_path],
                    |row| row.get(0),
                )
                .optional()?;
            if declarations != Some(0) {
                return Err(report_error(
                    "undeclared-baggage-payload diagnostic names a declared payload",
                ));
            }
        }
    }
    Ok(())
}

fn u64_to_i64(value: u64, field: &str) -> Result<i64, ProductDbError> {
    i64::try_from(value).map_err(|_| report_error(format!("{field} exceeds SQLite INTEGER")))
}

fn usize_to_i64(value: usize, field: &str) -> Result<i64, ProductDbError> {
    i64::try_from(value).map_err(|_| report_error(format!("{field} exceeds SQLite INTEGER")))
}

fn usize_to_u64(value: usize, field: &str) -> Result<u64, ProductDbError> {
    u64::try_from(value).map_err(|_| report_error(format!("{field} exceeds u64")))
}

fn i64_to_u64(value: i64, field: &str) -> Result<u64, ProductDbError> {
    u64::try_from(value).map_err(|_| report_error(format!("negative {field}")))
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
pub(crate) struct ZipDirectory {
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
        let start = usize::try_from(self.inner.position()).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "ZIP parser cursor position exceeds usize",
            )
        })?;
        let read = self.inner.read(buffer)?;
        let end = start + read;
        let first = self
            .neutralized_fields
            .partition_point(|offset| offset.checked_add(2).is_some_and(|end| end <= start));
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
// ZIP64 is outside this small, corpus-proven package slice. PDB-10 applies
// the same gate to ZIP payloads nested in baggage (`crate::baggage`).
pub(crate) fn preflight_zip(bytes: &[u8]) -> Result<ZipDirectory, PackageError> {
    let search_window = 22 + usize::from(u16::MAX);
    let start = bytes.len() - bytes.len().min(search_window);
    let end = bytes
        .len()
        .checked_sub(21)
        .ok_or_else(|| zip_error("end-of-directory record is truncated"))?;
    for offset in (start..end).rev() {
        if bytes.get(offset..offset + 4) != Some(b"PK\x05\x06") {
            continue;
        }
        let record = &bytes[offset..offset + 22];
        let comment_len = usize::from(u16::from_le_bytes([record[20], record[21]]));
        if offset + 22 + comment_len != bytes.len() {
            continue;
        }
        if offset >= 20 && bytes.get(offset - 20..offset - 16) == Some(b"PK\x06\x07") {
            return Err(zip_error("ZIP64 product packages are unsupported"));
        }
        let disk = u16::from_le_bytes([record[4], record[5]]);
        let directory_disk = u16::from_le_bytes([record[6], record[7]]);
        let disk_count = usize::from(u16::from_le_bytes([record[8], record[9]]));
        let count = usize::from(u16::from_le_bytes([record[10], record[11]]));
        if disk != 0 || directory_disk != 0 || disk_count != count {
            return Err(zip_error("multi-disk product packages are unsupported"));
        }
        if count > MAX_MEMBERS {
            return Err(PackageError::SizeLimit {
                path: "member count (ZIP64 unsupported)".into(),
            });
        }
        let directory_size =
            usize::try_from(u32::from_le_bytes(record[12..16].try_into().unwrap()))
                .map_err(zip_error)?;
        if directory_size > MAX_CENTRAL_DIRECTORY_SIZE {
            return Err(PackageError::SizeLimit {
                path: "central directory metadata".into(),
            });
        }
        let directory_start =
            usize::try_from(u32::from_le_bytes(record[16..20].try_into().unwrap()))
                .map_err(zip_error)?;
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
        let len = usize::from(u16::from_le_bytes([header[2], header[3]]));
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
        let name_len = usize::from(u16::from_le_bytes([header[28], header[29]]));
        let extra_len = usize::from(u16::from_le_bytes([header[30], header[31]]));
        let comment_len = usize::from(u16::from_le_bytes([header[32], header[33]]));
        let disk_start = u16::from_le_bytes([header[34], header[35]]);
        if disk_start != 0 {
            return Err(zip_error("central member starts on another disk"));
        }
        let local_offset = usize::try_from(u32::from_le_bytes(header[42..46].try_into().unwrap()))
            .map_err(zip_error)?;
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
        let local_name_len = usize::from(u16::from_le_bytes([local[26], local[27]]));
        let local_extra_len = usize::from(u16::from_le_bytes([local[28], local[29]]));
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
            .checked_add(usize::try_from(compressed_size).map_err(zip_error)?)
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
            central_offset: usize_to_u64(offset, "central-directory offset")?,
            local_offset: usize_to_u64(local_offset, "local-header offset")?,
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

/// Directory metadata of one member of a ZIP that passed
/// [`validate_central_directory`]. Nothing is decompressed.
pub(crate) struct ZipEntryMeta {
    pub name: Vec<u8>,
    pub flags: u16,
    pub uncompressed_size: u64,
}

/// The package validator applied to a nested archive, reduced to the
/// checked central-directory metadata.
pub(crate) fn validated_zip_metadata(bytes: &[u8]) -> Result<Vec<ZipEntryMeta>, PackageError> {
    // An empty archive is only its end-of-directory record (no entries, no
    // directory, optional comment). A package cannot be empty; a nested
    // archive can, and is then read as zero entries rather than unreadable.
    if bytes.len() >= 22
        && bytes[..4] == *b"PK\x05\x06"
        && bytes[4..20].iter().all(|&b| b == 0)
        && 22 + usize::from(u16::from_le_bytes([bytes[20], bytes[21]])) == bytes.len()
    {
        return Ok(Vec::new());
    }
    let directory = preflight_zip(bytes)?;
    let validated = validate_central_directory(bytes, directory)?;
    validated
        .entries
        .iter()
        .map(|entry| {
            let at = usize::try_from(entry.central_offset).map_err(zip_error)?;
            let header = bytes
                .get(at..at + 46)
                .ok_or_else(|| zip_error("truncated central directory"))?;
            let flags = u16::from_le_bytes([header[8], header[9]]);
            let name_len = usize::from(u16::from_le_bytes([header[28], header[29]]));
            let name = bytes
                .get(at + 46..at + 46 + name_len)
                .ok_or_else(|| zip_error("truncated central directory name"))?
                .to_vec();
            Ok(ZipEntryMeta {
                name,
                flags,
                uncompressed_size: entry.uncompressed_size,
            })
        })
        .collect()
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
                        "http://knx.org/xml/project/12" => return Ok(12),
                        "http://knx.org/xml/project/13" => return Ok(13),
                        "http://knx.org/xml/project/14" => return Ok(14),
                        "http://knx.org/xml/project/20" => return Ok(20),
                        "http://knx.org/xml/project/21" => return Ok(21),
                        "http://knx.org/xml/project/23" => return Ok(23),
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

// Domain readers currently dispatch by local name. Reject foreign elements and
// qualified attributes in schemes 21/23 rather than publishing extension data as
// typed KNX rows. This is a deliberately narrow, corpus-evidenced boundary.
fn validate_extended_member_namespace(
    path: &str,
    bytes: &[u8],
    scheme: u32,
) -> Result<(), PackageError> {
    let expected_namespace = format!("http://knx.org/xml/project/{scheme}");
    let mut reader = quick_xml::NsReader::from_reader(bytes);
    loop {
        let (namespace, event) = reader
            .read_resolved_event()
            .map_err(|error| xml_error(path, error))?;
        match event {
            Event::Start(element) | Event::Empty(element) => {
                if !matches!(namespace, ResolveResult::Bound(uri) if uri.as_ref() == expected_namespace.as_str())
                {
                    return Err(xml_error(
                        path,
                        format!("scheme-{scheme} XML contains a non-KNX element namespace"),
                    ));
                }
                for attribute in element.attributes().with_checks(true) {
                    let attribute = attribute.map_err(|error| xml_error(path, error))?;
                    let name = attribute.key.as_ref();
                    if name.contains(':') && !name.starts_with("xmlns:") {
                        return Err(xml_error(
                            path,
                            format!("scheme-{scheme} XML contains a qualified attribute"),
                        ));
                    }
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(())
}

// quick-xml is a streaming tokenizer; enforce a complete document even for
// unrecognized XML, which otherwise has no domain parser to detect truncation.
fn validate_xml(path: &str, bytes: &[u8]) -> Result<(), PackageError> {
    crate::xml::validate_complete_document(path, bytes).map_err(PackageError::Database)
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

fn add_count(
    facts: &mut InstallFacts,
    category: InstallCategory,
    disposition: InstallDisposition,
    count: u64,
) -> Result<(), ProductDbError> {
    if let Some(row) = facts
        .counts
        .iter_mut()
        .find(|row| row.category == category && row.disposition == disposition)
    {
        row.count = row
            .count
            .checked_add(count)
            .ok_or_else(|| report_error("install count overflow"))?;
    } else {
        facts.counts.push(InstallCount {
            category,
            disposition,
            count,
        });
    }
    Ok(())
}

const REQUIRED_COUNTS: &[(InstallCategory, InstallDisposition)] = &[
    (InstallCategory::ArchiveMember, InstallDisposition::Read),
    (InstallCategory::ArchiveMember, InstallDisposition::Stored),
    (
        InstallCategory::ArchiveMember,
        InstallDisposition::Deduplicated,
    ),
    (InstallCategory::Product, InstallDisposition::Read),
    (InstallCategory::Product, InstallDisposition::Stored),
    (InstallCategory::Product, InstallDisposition::Deduplicated),
    (
        InstallCategory::ApplicationProgram,
        InstallDisposition::Read,
    ),
    (
        InstallCategory::ApplicationProgram,
        InstallDisposition::Stored,
    ),
    (
        InstallCategory::ApplicationProgram,
        InstallDisposition::Deduplicated,
    ),
    (InstallCategory::Parameter, InstallDisposition::Read),
    (InstallCategory::Parameter, InstallDisposition::Stored),
    (
        InstallCategory::CommunicationObject,
        InstallDisposition::Read,
    ),
    (
        InstallCategory::CommunicationObject,
        InstallDisposition::Stored,
    ),
    (InstallCategory::DynamicNode, InstallDisposition::Read),
    (InstallCategory::DynamicNode, InstallDisposition::Stored),
    (InstallCategory::Module, InstallDisposition::Read),
    (InstallCategory::DatapointType, InstallDisposition::Read),
    (InstallCategory::DatapointType, InstallDisposition::Stored),
    (InstallCategory::DatapointType, InstallDisposition::Dropped),
    (InstallCategory::BaggageIndex, InstallDisposition::Read),
    (InstallCategory::BaggageIndex, InstallDisposition::Stored),
    (InstallCategory::Baggage, InstallDisposition::Read),
    (InstallCategory::Baggage, InstallDisposition::Stored),
    (InstallCategory::Baggage, InstallDisposition::Deduplicated),
    (
        InstallCategory::Baggage,
        InstallDisposition::RetainedButUninterpreted,
    ),
    (InstallCategory::MasterSection, InstallDisposition::Read),
    (
        InstallCategory::MasterSection,
        InstallDisposition::Unsupported,
    ),
    (
        InstallCategory::MasterSubtree,
        InstallDisposition::Unsupported,
    ),
    (InstallCategory::UnknownConstruct, InstallDisposition::Read),
    (
        InstallCategory::UnknownConstruct,
        InstallDisposition::Stored,
    ),
];

fn add_entities(
    facts: &mut InstallFacts,
    entities: &crate::report::EntityCounts,
) -> Result<(), ProductDbError> {
    use crate::report::EntityKind;
    for (kind, category) in [
        (EntityKind::Product, InstallCategory::Product),
        (
            EntityKind::ApplicationProgram,
            InstallCategory::ApplicationProgram,
        ),
        (EntityKind::Parameter, InstallCategory::Parameter),
        (
            EntityKind::CommunicationObject,
            InstallCategory::CommunicationObject,
        ),
        (EntityKind::DynamicNode, InstallCategory::DynamicNode),
        (EntityKind::ModuleDef, InstallCategory::Module),
        (EntityKind::DatapointType, InstallCategory::DatapointType),
    ] {
        let count = entities.get(kind);
        add_count(facts, category, InstallDisposition::Read, count.read)?;
        if count.stored != 0 {
            add_count(facts, category, InstallDisposition::Stored, count.stored)?;
        }
        if count.deduplicated != 0 {
            add_count(
                facts,
                category,
                InstallDisposition::Deduplicated,
                count.deduplicated,
            )?;
        }
        if count.dropped != 0 {
            add_count(facts, category, InstallDisposition::Dropped, count.dropped)?;
        }
    }
    Ok(())
}

fn count_of(
    facts: &InstallFacts,
    category: InstallCategory,
    disposition: InstallDisposition,
) -> Result<u64, ProductDbError> {
    facts
        .counts
        .iter()
        .find(|row| row.category == category && row.disposition == disposition)
        .map(|row| row.count)
        .ok_or_else(|| {
            report_error(format!(
                "missing required count {}/{}",
                category.as_str(),
                disposition.as_str()
            ))
        })
}

fn finalize_facts(facts: &mut InstallFacts) -> Result<(), ProductDbError> {
    for &(category, disposition) in REQUIRED_COUNTS {
        if !facts
            .counts
            .iter()
            .any(|row| row.category == category && row.disposition == disposition)
        {
            facts.counts.push(InstallCount {
                category,
                disposition,
                count: 0,
            });
        }
    }
    facts.sort()
}

fn checked_sum(left: u64, right: u64, label: &str) -> Result<u64, ProductDbError> {
    left.checked_add(right)
        .ok_or_else(|| report_error(format!("{label} count overflow")))
}

fn validate_facts(
    conn: &Connection,
    sha256: &str,
    facts: &InstallFacts,
) -> Result<(), ProductDbError> {
    if facts.counts.len() != REQUIRED_COUNTS.len() {
        return Err(report_error(
            "install report has missing or extra count rows",
        ));
    }
    let mut seen = HashSet::new();
    for row in &facts.counts {
        if !REQUIRED_COUNTS.contains(&(row.category, row.disposition))
            || !seen.insert((row.category, row.disposition))
        {
            return Err(report_error(
                "illegal or duplicate category/disposition row",
            ));
        }
    }
    for unknown in &facts.unknown_constructs {
        validate_xml_path(&unknown.xpath)?;
    }
    let mut diagnostic_identities = HashSet::new();
    for diagnostic in &facts.diagnostics {
        validate_diagnostic(conn, sha256, diagnostic)?;
        if !diagnostic_identities.insert((
            diagnostic.kind.as_str(),
            diagnostic.archive_path.as_str(),
            diagnostic.xml_path.as_str(),
            diagnostic.detail.as_str(),
        )) {
            return Err(report_error("duplicate diagnostic identity"));
        }
    }
    for category in [
        InstallCategory::ArchiveMember,
        InstallCategory::Product,
        InstallCategory::ApplicationProgram,
    ] {
        let read = count_of(facts, category, InstallDisposition::Read)?;
        let stored = count_of(facts, category, InstallDisposition::Stored)?;
        let deduplicated = count_of(facts, category, InstallDisposition::Deduplicated)?;
        if read != checked_sum(stored, deduplicated, category.as_str())? {
            return Err(report_error(format!(
                "{} outcome mismatch",
                category.as_str()
            )));
        }
    }
    for category in [
        InstallCategory::Parameter,
        InstallCategory::CommunicationObject,
        InstallCategory::DynamicNode,
    ] {
        if count_of(facts, category, InstallDisposition::Stored)?
            > count_of(facts, category, InstallDisposition::Read)?
        {
            return Err(report_error(format!(
                "{} stored exceeds read",
                category.as_str()
            )));
        }
    }
    let dpt_read = count_of(
        facts,
        InstallCategory::DatapointType,
        InstallDisposition::Read,
    )?;
    let dpt_stored = count_of(
        facts,
        InstallCategory::DatapointType,
        InstallDisposition::Stored,
    )?;
    let dpt_dropped = count_of(
        facts,
        InstallCategory::DatapointType,
        InstallDisposition::Dropped,
    )?;
    if dpt_read != checked_sum(dpt_stored, dpt_dropped, "datapoint type")? {
        return Err(report_error("datapoint type outcome mismatch"));
    }
    let baggage_index_read = count_of(
        facts,
        InstallCategory::BaggageIndex,
        InstallDisposition::Read,
    )?;
    let baggage_index_stored = count_of(
        facts,
        InstallCategory::BaggageIndex,
        InstallDisposition::Stored,
    )?;
    let stored_declarations = i64_to_u64(
        conn.query_row(
            "SELECT count(*) FROM package_baggage_declaration WHERE package_sha256 = ?1",
            [sha256],
            |row| row.get::<_, i64>(0),
        )?,
        "stored baggage declarations",
    )?;
    if baggage_index_read != baggage_index_stored || baggage_index_stored != stored_declarations {
        return Err(report_error("baggage-index outcome mismatch"));
    }
    let baggage_read = count_of(facts, InstallCategory::Baggage, InstallDisposition::Read)?;
    let baggage_stored = count_of(facts, InstallCategory::Baggage, InstallDisposition::Stored)?;
    let baggage_deduplicated = count_of(
        facts,
        InstallCategory::Baggage,
        InstallDisposition::Deduplicated,
    )?;
    let baggage_retained = count_of(
        facts,
        InstallCategory::Baggage,
        InstallDisposition::RetainedButUninterpreted,
    )?;
    if baggage_read != checked_sum(baggage_stored, baggage_deduplicated, "baggage")?
        || baggage_read != baggage_retained
    {
        return Err(report_error("opaque baggage outcome mismatch"));
    }
    let master_read = count_of(
        facts,
        InstallCategory::MasterSection,
        InstallDisposition::Read,
    )?;
    let master_unsupported = count_of(
        facts,
        InstallCategory::MasterSection,
        InstallDisposition::Unsupported,
    )?;
    if master_unsupported > master_read {
        return Err(report_error(
            "unsupported master sections exceed sections read",
        ));
    }
    let unknown_distinct = u64::try_from(facts.unknown_constructs.len())
        .map_err(|_| report_error("unknown distinct count exceeds u64"))?;
    if count_of(
        facts,
        InstallCategory::UnknownConstruct,
        InstallDisposition::Read,
    )? != facts.unknown_occurrences
        || count_of(
            facts,
            InstallCategory::UnknownConstruct,
            InstallDisposition::Stored,
        )? != unknown_distinct
    {
        return Err(report_error("unknown construct header/count mismatch"));
    }
    let diagnostic_master = facts.diagnostics.iter().try_fold(0u64, |total, row| {
        if row.kind == InstallDiagnosticKind::UnsupportedMasterSection {
            total
                .checked_add(row.occurrences)
                .ok_or_else(|| report_error("master diagnostic counter overflow"))
        } else {
            Ok(total)
        }
    })?;
    let diagnostic_subtree = facts.diagnostics.iter().try_fold(0u64, |total, row| {
        if row.kind == InstallDiagnosticKind::UnsupportedMasterSubtree {
            total
                .checked_add(row.occurrences)
                .ok_or_else(|| report_error("master subtree diagnostic counter overflow"))
        } else {
            Ok(total)
        }
    })?;
    if diagnostic_subtree
        != count_of(
            facts,
            InstallCategory::MasterSubtree,
            InstallDisposition::Unsupported,
        )?
    {
        return Err(report_error("master subtree diagnostic/count mismatch"));
    }
    if diagnostic_master != master_unsupported {
        return Err(report_error("diagnostic/unsupported count mismatch"));
    }
    // Every unresolved declaration and undeclared payload of the stored
    // inventory has a diagnostic; `validate_diagnostic` already proved each
    // one matches, so equal totals mean none is missing.
    let sum_kind = |kind: InstallDiagnosticKind| {
        facts.diagnostics.iter().try_fold(0u64, |total, row| {
            if row.kind == kind {
                total
                    .checked_add(row.occurrences)
                    .ok_or_else(|| report_error("baggage diagnostic counter overflow"))
            } else {
                Ok(total)
            }
        })
    };
    let (unresolved, undeclared): (i64, i64) = conn.query_row(
        "SELECT (SELECT count(*) FROM package_baggage_declaration
                  WHERE package_sha256 = ?1 AND resolution <> 'resolved'),
                (SELECT count(*) FROM package_baggage_payload
                  WHERE package_sha256 = ?1 AND declarations = 0)",
        [sha256],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    if sum_kind(InstallDiagnosticKind::UnresolvedBaggageDeclaration)?
        != i64_to_u64(unresolved, "unresolved declarations")?
        || sum_kind(InstallDiagnosticKind::UndeclaredBaggagePayload)?
            != i64_to_u64(undeclared, "undeclared payloads")?
    {
        return Err(report_error("baggage diagnostic/inventory mismatch"));
    }
    let member_total = i64_to_u64(
        conn.query_row(
            "SELECT count(*) FROM package_member WHERE package_sha256 = ?1",
            [sha256],
            |row| row.get::<_, i64>(0),
        )?,
        "archive member total",
    )?;
    if count_of(
        facts,
        InstallCategory::ArchiveMember,
        InstallDisposition::Read,
    )? != member_total
    {
        return Err(report_error("archive member total mismatch"));
    }
    Ok(())
}

fn load_facts(conn: &Connection, sha256: &str) -> Result<Option<InstallFacts>, ProductDbError> {
    let facts = load_facts_unvalidated(conn, sha256)?;
    if let Some(facts) = &facts {
        validate_facts(conn, sha256, facts)?;
    }
    Ok(facts)
}

/// `load_facts` without the cross-row validation, for the v15 -> v16
/// upgrade, which reads a report in its old shape and re-persists it in
/// the new one. Header/detail agreement is still checked.
fn load_facts_unvalidated(
    conn: &Connection,
    sha256: &str,
) -> Result<Option<InstallFacts>, ProductDbError> {
    let header: Option<(i64, String, i64, i64)> = conn.query_row(
        "SELECT report_version, status, unknown_distinct, unknown_occurrences FROM package_install_report WHERE package_sha256 = ?1",
        [sha256],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
    ).optional()?;
    let Some(header) = header else {
        return Err(report_error(
            "installed package is missing its report marker",
        ));
    };
    if header.0 != 1 {
        return Err(report_error("invalid report version"));
    }
    let header_distinct = i64_to_u64(header.2, "unknown distinct header")?;
    let header_occurrences = i64_to_u64(header.3, "unknown occurrences header")?;
    if header.1 == "unavailable" {
        let detail_rows: i64 = conn.query_row(
            "SELECT (SELECT count(*) FROM package_install_count WHERE package_sha256 = ?1)
                  + (SELECT count(*) FROM package_install_unknown WHERE package_sha256 = ?1)
                  + (SELECT count(*) FROM package_install_diagnostic WHERE package_sha256 = ?1)",
            [sha256],
            |row| row.get(0),
        )?;
        if header_distinct != 0 || header_occurrences != 0 || detail_rows != 0 {
            return Err(report_error(
                "unavailable report marker has measured details",
            ));
        }
        return Ok(None);
    }
    if header.1 != "measured" {
        return Err(report_error("invalid report status"));
    }
    let counts = {
        let rows = conn.prepare("SELECT category, disposition, count FROM package_install_count WHERE package_sha256 = ?1 ORDER BY ordinal")?.query_map([sha256], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, i64>(2)?)))?.collect::<Result<Vec<_>, _>>()?;
        rows.into_iter()
            .map(|(category, disposition, count)| {
                Ok(InstallCount {
                    category: InstallCategory::from_db(category)?,
                    disposition: InstallDisposition::from_db(disposition)?,
                    count: i64_to_u64(count, "install count")?,
                })
            })
            .collect::<Result<Vec<_>, ProductDbError>>()?
    };
    let unknown_constructs = {
        let mut stmt = conn.prepare("SELECT xpath, kind, name, occurrences, sample FROM package_install_unknown WHERE package_sha256 = ?1 ORDER BY xpath, kind, name")?;
        let mut out = Vec::new();
        for row in stmt.query_map([sha256], |r| {
            let kind: String = r.get(1)?;
            let occurrences: i64 = r.get(3)?;
            Ok((
                r.get::<_, String>(0)?,
                kind,
                r.get::<_, String>(2)?,
                occurrences,
                r.get(4)?,
            ))
        })? {
            let (xpath, kind, name, occurrences, sample) = row?;
            validate_xml_path(&xpath)?;
            let kind = match kind.as_str() {
                "Attribute" => crate::report::UnknownKind::Attribute,
                "Element" => crate::report::UnknownKind::Element,
                _ => {
                    return Err(ProductDbError::Xml {
                        source_path: "package_install_unknown".into(),
                        cause: "invalid unknown kind".into(),
                    })
                }
            };
            if occurrences <= 0 {
                return Err(ProductDbError::Xml {
                    source_path: "package_install_unknown".into(),
                    cause: "invalid unknown occurrences".into(),
                });
            }
            out.push(crate::report::UnknownConstruct {
                xpath,
                kind,
                name,
                occurrences: u32::try_from(occurrences)
                    .map_err(|_| report_error("invalid unknown occurrences"))?,
                sample,
            });
        }
        out
    };
    let diagnostics = {
        let rows = conn.prepare("SELECT kind, archive_path, xml_path, detail, occurrences FROM package_install_diagnostic WHERE package_sha256 = ?1 ORDER BY kind, archive_path, xml_path, detail")?.query_map([sha256], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, String>(3)?, r.get::<_, i64>(4)?)))?.collect::<Result<Vec<_>, _>>()?;
        rows.into_iter()
            .map(|(kind, archive_path, xml_path, detail, occurrences)| {
                InstallDiagnostic::new(
                    InstallDiagnosticKind::from_db(&kind)?,
                    archive_path,
                    xml_path,
                    detail,
                    i64_to_u64(occurrences, "diagnostic occurrences")?,
                )
            })
            .collect::<Result<Vec<_>, ProductDbError>>()?
    };
    let mut facts = InstallFacts {
        counts,
        unknown_constructs,
        unknown_occurrences: 0,
        diagnostics,
    };
    facts.sort()?;
    let actual_distinct = u64::try_from(facts.unknown_constructs.len())
        .map_err(|_| report_error("unknown distinct count exceeds u64"))?;
    if actual_distinct != header_distinct || facts.unknown_occurrences != header_occurrences {
        return Err(report_error("report header/detail mismatch"));
    }
    Ok(Some(facts))
}

/// Why a measured v15 report cannot be carried into v16. The caller
/// downgrades just that report to `unavailable`.
#[derive(Debug)]
pub(crate) struct ReportNotUpgradable(pub String);

/// v15 -> v16: rewrites one package's measured report in the shape a fresh
/// v16 install writes. `baggage_index` becomes `stored` (the declarations
/// now are), the old `unsupported-baggage-index` diagnostic is gone (the
/// migration dropped it), the inventory's own diagnostics are added and the
/// index parser's unknowns are merged in exactly as install merges member
/// unknowns. Rows are then re-persisted through `persist_facts`, so
/// ordinals, sorting and validation are install's own. `inventory` must
/// already be persisted.
pub(crate) fn upgrade_report_for_baggage(
    conn: &Connection,
    sha256: &str,
    inventory: &crate::baggage::BaggageInventory,
    index_unknowns: Vec<crate::report::UnknownConstruct>,
) -> Result<Result<(), ReportNotUpgradable>, ProductDbError> {
    let Some(mut facts) = load_facts_unvalidated(conn, sha256)? else {
        return Ok(Ok(()));
    };
    let declarations = usize_to_u64(inventory.declarations.len(), "baggage declarations")?;
    let read = count_of(
        &facts,
        InstallCategory::BaggageIndex,
        InstallDisposition::Read,
    )?;
    if read != declarations {
        return Ok(Err(ReportNotUpgradable(format!(
            "report counted {read} baggage declarations, retained indexes hold {declarations}"
        ))));
    }
    facts.counts.retain(|row| {
        !(row.category == InstallCategory::BaggageIndex
            && row.disposition == InstallDisposition::Unsupported)
            && row.category != InstallCategory::UnknownConstruct
    });
    add_count(
        &mut facts,
        InstallCategory::BaggageIndex,
        InstallDisposition::Stored,
        read,
    )?;
    facts.diagnostics.extend(baggage_diagnostics(inventory)?);
    let mut merged: BTreeMap<(String, String, String), crate::report::UnknownConstruct> =
        BTreeMap::new();
    for unknown in facts.unknown_constructs.drain(..).chain(index_unknowns) {
        let key = (
            unknown.xpath.clone(),
            unknown.kind.as_str().to_string(),
            unknown.name.clone(),
        );
        if let Some(current) = merged.get_mut(&key) {
            current.occurrences = current
                .occurrences
                .checked_add(unknown.occurrences)
                .ok_or_else(|| report_error("unknown occurrence counter overflow"))?;
        } else {
            merged.insert(key, unknown);
        }
    }
    facts.unknown_constructs = merged.into_values().collect();
    facts.sort()?;
    let distinct = usize_to_u64(facts.unknown_constructs.len(), "unknown distinct")?;
    let occurrences = facts.unknown_occurrences;
    add_count(
        &mut facts,
        InstallCategory::UnknownConstruct,
        InstallDisposition::Read,
        occurrences,
    )?;
    add_count(
        &mut facts,
        InstallCategory::UnknownConstruct,
        InstallDisposition::Stored,
        distinct,
    )?;
    for table in [
        "package_install_count",
        "package_install_unknown",
        "package_install_diagnostic",
        "package_install_report",
    ] {
        conn.execute(
            &format!("DELETE FROM {table} WHERE package_sha256 = ?1"),
            [sha256],
        )?;
    }
    persist_facts(conn, sha256, facts)?;
    Ok(Ok(()))
}

/// v17 -> v18 (ADR-0052): rewrites one package's measured report without
/// the unknown attribute `name` at `xpaths`, which the parser now models.
/// The rows are removed and the header and `unknown_construct` counts are
/// recomputed from what remains, exactly as install computes them; the
/// result then goes through `persist_facts`' own validation. An
/// `unavailable` report has no rows to change and stays as it is.
pub(crate) fn retire_report_unknowns(
    conn: &Connection,
    sha256: &str,
    xpaths: &[&str],
    name: &str,
) -> Result<Result<(), ReportNotUpgradable>, ProductDbError> {
    let Some(mut facts) = load_facts_unvalidated(conn, sha256)? else {
        return Ok(Ok(()));
    };
    facts.unknown_constructs.retain(|unknown| {
        !(unknown.kind == crate::report::UnknownKind::Attribute
            && unknown.name == name
            && xpaths.contains(&unknown.xpath.as_str()))
    });
    facts
        .counts
        .retain(|row| row.category != InstallCategory::UnknownConstruct);
    // `sort` re-derives `unknown_occurrences` from the remaining rows.
    facts.sort()?;
    let occurrences = facts.unknown_occurrences;
    let distinct = usize_to_u64(facts.unknown_constructs.len(), "unknown distinct")?;
    add_count(
        &mut facts,
        InstallCategory::UnknownConstruct,
        InstallDisposition::Read,
        occurrences,
    )?;
    add_count(
        &mut facts,
        InstallCategory::UnknownConstruct,
        InstallDisposition::Stored,
        distinct,
    )?;
    for table in [
        "package_install_count",
        "package_install_unknown",
        "package_install_diagnostic",
        "package_install_report",
    ] {
        conn.execute(
            &format!("DELETE FROM {table} WHERE package_sha256 = ?1"),
            [sha256],
        )?;
    }
    match persist_facts(conn, sha256, facts) {
        Ok(()) => Ok(Ok(())),
        Err(ProductDbError::Sqlite(error)) => Err(ProductDbError::Sqlite(error)),
        Err(error) => Ok(Err(ReportNotUpgradable(error.to_string()))),
    }
}

/// Downgrades one package's measured report to `unavailable`, with no
/// detail rows: what a migration does when it cannot carry a report
/// forward truthfully. A package with no report row, or an already
/// `unavailable` one, is left as it is.
pub(crate) fn mark_report_unavailable(
    conn: &Connection,
    sha256: &str,
) -> Result<(), ProductDbError> {
    let measured: Option<bool> = conn
        .query_row(
            "SELECT status = 'measured' FROM package_install_report WHERE package_sha256 = ?1",
            [sha256],
            |r| r.get(0),
        )
        .optional()?;
    if measured != Some(true) {
        return Ok(());
    }
    for table in [
        "package_install_count",
        "package_install_unknown",
        "package_install_diagnostic",
    ] {
        conn.execute(
            &format!("DELETE FROM {table} WHERE package_sha256 = ?1"),
            [sha256],
        )?;
    }
    conn.execute(
        "UPDATE package_install_report
         SET status = 'unavailable', unknown_distinct = 0, unknown_occurrences = 0
         WHERE package_sha256 = ?1",
        [sha256],
    )?;
    Ok(())
}

fn persist_facts(
    conn: &Connection,
    sha256: &str,
    mut facts: InstallFacts,
) -> Result<(), ProductDbError> {
    finalize_facts(&mut facts)?;
    validate_facts(conn, sha256, &facts)?;
    conn.execute("INSERT INTO package_install_report (package_sha256, report_version, status, unknown_distinct, unknown_occurrences) VALUES (?1, 1, 'measured', ?2, ?3)", params![sha256, usize_to_i64(facts.unknown_constructs.len(), "unknown distinct")?, u64_to_i64(facts.unknown_occurrences, "unknown occurrences")?])?;
    for (ordinal, row) in facts.counts.iter().enumerate() {
        conn.execute(
            "INSERT INTO package_install_count VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                sha256,
                usize_to_i64(ordinal, "count ordinal")?,
                row.category.as_str(),
                row.disposition.as_str(),
                u64_to_i64(row.count, "install count")?
            ],
        )?;
    }
    for (ordinal, row) in facts.unknown_constructs.iter().enumerate() {
        conn.execute(
            "INSERT INTO package_install_unknown VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                sha256,
                usize_to_i64(ordinal, "unknown ordinal")?,
                row.xpath,
                row.kind.as_str(),
                row.name,
                i64::from(row.occurrences),
                row.sample
            ],
        )?;
    }
    for (ordinal, row) in facts.diagnostics.iter().enumerate() {
        conn.execute(
            "INSERT INTO package_install_diagnostic VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                sha256,
                usize_to_i64(ordinal, "diagnostic ordinal")?,
                row.kind.as_str(),
                row.archive_path,
                row.xml_path,
                row.detail,
                u64_to_i64(row.occurrences, "diagnostic occurrences")?
            ],
        )?;
    }
    Ok(())
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
    // Filename admission only: do not decrypt or guess the legacy container.
    // Keep this before transactions and hash retries, including known bytes.
    if let Some(extension) = std::path::Path::new(source_name)
        .extension()
        .and_then(|extension| extension.to_str())
    {
        if ["vd3", "vd4", "vd5", "pr3", "pr4", "pr5"]
            .iter()
            .any(|legacy| extension.eq_ignore_ascii_case(legacy))
        {
            return Err(PackageError::UnsupportedLegacyFormat {
                extension: extension.to_owned(),
            });
        }
    }
    let sha256 = sha256_hex(bytes);
    let tx = conn.unchecked_transaction().map_err(ProductDbError::from)?;
    let prior_raw: Option<(i64, i64, i64, i64, i64, i64, i64)> = tx
        .query_row(
            "SELECT scheme, unknown_count, translation_program_count, translation_catalog_count,
                    translation_hardware_count, translation_master_count,
                    dropped_datapoint_type_count
             FROM package WHERE sha256 = ?1",
            [&sha256],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                ))
            },
        )
        .optional()?;
    let prior = prior_raw
        .map(|row| -> Result<_, ProductDbError> {
            Ok((
                u32::try_from(row.0).map_err(|_| report_error("invalid stored package scheme"))?,
                usize::try_from(row.1).map_err(|_| report_error("invalid stored unknown count"))?,
                TranslationCounts {
                    program: usize::try_from(row.2)
                        .map_err(|_| report_error("invalid program translation count"))?,
                    catalog: usize::try_from(row.3)
                        .map_err(|_| report_error("invalid catalog translation count"))?,
                    hardware: usize::try_from(row.4)
                        .map_err(|_| report_error("invalid hardware translation count"))?,
                    master: usize::try_from(row.5)
                        .map_err(|_| report_error("invalid master translation count"))?,
                },
                usize::try_from(row.6)
                    .map_err(|_| report_error("invalid dropped datapoint count"))?,
            ))
        })
        .transpose()?;
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
    let signature_scan_end = central_view
        .len()
        .checked_sub(3)
        .ok_or_else(|| zip_error("central directory is truncated"))?;
    for relative in 0..signature_scan_end {
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
        let central_header_start = parsed
            .central_header_start()
            .checked_add(usize_to_u64(directory.start, "central-directory start")?)
            .ok_or_else(|| zip_error("central-header offset overflow"))?;
        if central_header_start != checked.central_offset
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
        let data_len = usize_to_u64(data.len(), "decoded member size")?;
        if data_len > MAX_MEMBER_SIZE {
            return Err(PackageError::SizeLimit { path });
        }
        if data_len != file.size() {
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
                size: data_len,
            },
        });
    }
    let scheme = scheme.ok_or(PackageError::MissingMaster)?;
    if matches!(scheme, 21 | 23) {
        for validated in &validated_members {
            if !matches!(
                validated.member.role.as_str(),
                "Master" | "Catalog" | "Hardware" | "ApplicationProgram" | "Baggages"
            ) {
                continue;
            }
            let mut file = archive
                .by_index(validated.archive_index)
                .map_err(zip_error)?;
            let mut data = Vec::new();
            (&mut file)
                .take(MAX_MEMBER_SIZE + 1)
                .read_to_end(&mut data)
                .map_err(zip_error)?;
            if usize_to_u64(data.len(), "scheme-21 member size")? != validated.member.size
                || sha256_hex(&data) != validated.member.sha256
            {
                return Err(zip_error("member changed between validation passes"));
            }
            validate_extended_member_namespace(&validated.member.path, &data, scheme)?;
        }
    }
    if !has_manufacturer_data {
        return Err(PackageError::MissingManufacturerData);
    }
    if let Some((stored_scheme, unknown, translations, dropped_datapoint_types)) = prior {
        let member_rows = tx.prepare("SELECT path, role, source_sha256, size FROM package_member WHERE package_sha256 = ?1 ORDER BY ordinal")?.query_map([&sha256], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, i64>(3)?)))?.collect::<Result<Vec<_>, _>>()?;
        let members = member_rows
            .into_iter()
            .map(|(path, role, member_sha, size)| {
                Ok(PackageMember {
                    path,
                    role,
                    sha256: member_sha,
                    size: i64_to_u64(size, "stored package member size")?,
                })
            })
            .collect::<Result<Vec<_>, ProductDbError>>()?;
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
        let facts = load_facts(&tx, &sha256)?;
        let baggage = crate::baggage::load_baggage_inventory(&tx, &sha256)?;
        // The only write of a retry: the name these bytes arrived under.
        tx.execute(
            "INSERT OR IGNORE INTO package_source_name (package_sha256, source_name) VALUES (?1, ?2)",
            params![sha256, source_name],
        )?;
        let source_names = crate::identity::package_source_names(&tx, &sha256)?;
        tx.commit().map_err(ProductDbError::from)?;
        return Ok(InstallReport {
            sha256: sha256.clone(),
            scheme,
            skipped: true,
            members,
            unknown,
            conflicts,
            translations,
            dropped_datapoint_types,
            facts,
            baggage,
            source_names,
        });
    }
    tx.execute("INSERT INTO package (sha256, source_name, scheme, size, bytes, unknown_count) VALUES (?1, ?2, ?3, ?4, ?5, 0)", params![sha256, source_name, scheme, usize_to_i64(bytes.len(), "package size")?, bytes])?;
    tx.execute(
        "INSERT INTO package_source_name (package_sha256, source_name) VALUES (?1, ?2)",
        params![sha256, source_name],
    )?;
    let mut report = InstallReport {
        source_names: vec![source_name.to_string()],
        sha256,
        scheme,
        skipped: false,
        dropped_datapoint_types: 0,
        members: Vec::new(),
        unknown: 0,
        conflicts: Vec::new(),
        translations: TranslationCounts::default(),
        facts: None,
        baggage: None,
    };
    let mut facts = InstallFacts::default();
    let mut package_unknowns = Vec::new();
    let mut baggage_indexes = Vec::new();
    let mut baggage_payloads = Vec::new();
    for (ordinal, validated) in validated_members.into_iter().enumerate() {
        let PackageMember {
            path,
            role,
            sha256: member_sha,
            size,
        } = validated.member;
        add_count(
            &mut facts,
            InstallCategory::ArchiveMember,
            InstallDisposition::Read,
            1,
        )?;
        let already_stored: Option<i64> = tx
            .query_row(
                "SELECT 1 FROM source_file WHERE sha256 = ?1",
                [&member_sha],
                |r| r.get(0),
            )
            .optional()?;
        add_count(
            &mut facts,
            InstallCategory::ArchiveMember,
            if already_stored.is_some() {
                InstallDisposition::Deduplicated
            } else {
                InstallDisposition::Stored
            },
            1,
        )?;
        if role == "Baggage" {
            add_count(
                &mut facts,
                InstallCategory::Baggage,
                InstallDisposition::Read,
                1,
            )?;
            add_count(
                &mut facts,
                InstallCategory::Baggage,
                if already_stored.is_some() {
                    InstallDisposition::Deduplicated
                } else {
                    InstallDisposition::Stored
                },
                1,
            )?;
            add_count(
                &mut facts,
                InstallCategory::Baggage,
                InstallDisposition::RetainedButUninterpreted,
                1,
            )?;
        }
        let mut file = archive
            .by_index(validated.archive_index)
            .map_err(zip_error)?;
        let mut data = Vec::new();
        (&mut file)
            .take(MAX_MEMBER_SIZE + 1)
            .read_to_end(&mut data)
            .map_err(zip_error)?;
        if usize_to_u64(data.len(), "decoded member size")? != size
            || sha256_hex(&data) != member_sha
        {
            return Err(zip_error("member changed between validation passes"));
        }
        let outcome = if matches!(
            role.as_str(),
            "Catalog" | "Hardware" | "ApplicationProgram" | "Baggages"
        ) {
            // A retained raw member is not proof its domain rows were parsed.
            // The package hash, not the blob hash, controls package retries.
            ingest_file_in_transaction(&tx, &path, &data, true, matches!(scheme, 21 | 23))?
        } else {
            let stored = crate::store_source_file(
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
            if stored {
                crate::identity::record_producer(&tx, &member_sha, &data)?;
            }
            DetailedIngestOutcome {
                outcome: IngestOutcome::Skipped {
                    sha256: member_sha.clone(),
                },
                unknown_constructs: Vec::new(),
                entities: crate::report::EntityCounts::default(),
            }
        };
        let DetailedIngestOutcome {
            outcome,
            unknown_constructs,
            entities,
        } = outcome;
        if let IngestOutcome::Ingested {
            unknown,
            conflicts,
            translations,
            ..
        } = outcome
        {
            report.unknown = report
                .unknown
                .checked_add(unknown)
                .ok_or_else(|| report_error("package unknown counter overflow"))?;
            package_unknowns.extend(unknown_constructs);
            report.conflicts.extend(conflicts);
            report.translations.checked_add(translations)?;
            add_entities(&mut facts, &entities)?;
        }
        if role == "Master" {
            let master = crate::parse::master::ingest_master_data_detailed(&tx, &data)?;
            let crate::parse::master::DetailedMasterIngest {
                outcome: mut master_outcome,
                entities,
                master_sections_read,
                unsupported_sections,
                uninterpreted_subtrees,
            } = master;
            if matches!(scheme, 21 | 23) {
                crate::parse::scheme_evidence::reconcile_package_unknowns(
                    &data,
                    &path,
                    &mut master_outcome.unknown,
                )?;
            }
            insert_unknown(&tx, &member_sha, &master_outcome.unknown)?;
            report.unknown = report
                .unknown
                .checked_add(master_outcome.unknown.len())
                .ok_or_else(|| report_error("package unknown counter overflow"))?;
            package_unknowns.extend(master_outcome.unknown.clone());
            report.translations.master = report
                .translations
                .master
                .checked_add(master_outcome.translations)
                .ok_or_else(|| report_error("master translation counter overflow"))?;
            report.dropped_datapoint_types = report
                .dropped_datapoint_types
                .checked_add(master_outcome.dropped_datapoint_types)
                .ok_or_else(|| report_error("dropped datapoint counter overflow"))?;
            add_entities(&mut facts, &entities)?;
            add_count(
                &mut facts,
                InstallCategory::MasterSection,
                InstallDisposition::Read,
                master_sections_read,
            )?;
            for unsupported in unsupported_sections {
                add_count(
                    &mut facts,
                    InstallCategory::MasterSection,
                    InstallDisposition::Unsupported,
                    unsupported.occurrences,
                )?;
                facts.diagnostics.push(InstallDiagnostic::new(
                    InstallDiagnosticKind::UnsupportedMasterSection,
                    path.clone(),
                    format!("{MASTER_DIAGNOSTIC_PREFIX}{}", unsupported.name),
                    master_diagnostic_detail(&unsupported.name),
                    unsupported.occurrences,
                )?);
            }
            for subtree in &uninterpreted_subtrees {
                add_count(
                    &mut facts,
                    InstallCategory::MasterSubtree,
                    InstallDisposition::Unsupported,
                    subtree.occurrences,
                )?;
                facts
                    .diagnostics
                    .push(master_subtree_diagnostic(path.clone(), subtree)?);
            }
        } else if role == "Baggages" {
            // Its unknowns already arrived through `ingest_file_in_transaction`.
            let (index, _) = crate::baggage::parse_index(&path, &data)?;
            let count = usize_to_u64(index.1.len(), "baggage declarations")?;
            add_count(
                &mut facts,
                InstallCategory::BaggageIndex,
                InstallDisposition::Read,
                count,
            )?;
            add_count(
                &mut facts,
                InstallCategory::BaggageIndex,
                InstallDisposition::Stored,
                count,
            )?;
            baggage_indexes.push(index);
        } else if role == "Baggage" {
            baggage_payloads.push(crate::baggage::BaggagePayload::measure(
                path.clone(),
                member_sha.clone(),
                &data,
            ));
        } else if role == "Unrecognized" {
            let mut unknown = UnknownCollector::default();
            unknown.element("/Package", &path);
            let unknown = unknown.into_vec();
            insert_unknown(&tx, &member_sha, &unknown)?;
            report.unknown = report
                .unknown
                .checked_add(unknown.len())
                .ok_or_else(|| report_error("package unknown counter overflow"))?;
            package_unknowns.extend(unknown);
        }
        tx.execute("INSERT INTO package_member (package_sha256, ordinal, path, role, source_sha256, size) VALUES (?1, ?2, ?3, ?4, ?5, ?6)", params![report.sha256, usize_to_i64(ordinal, "package member ordinal")?, path, role, member_sha, u64_to_i64(size, "package member size")?])?;
        report.members.push(PackageMember {
            path,
            role,
            sha256: member_sha,
            size,
        });
    }
    let mut unknowns: BTreeMap<(String, String, String), crate::report::UnknownConstruct> =
        BTreeMap::new();
    for unknown in package_unknowns {
        let key = (
            unknown.xpath.clone(),
            unknown.kind.as_str().to_string(),
            unknown.name.clone(),
        );
        if let Some(current) = unknowns.get_mut(&key) {
            current.occurrences = current
                .occurrences
                .checked_add(unknown.occurrences)
                .ok_or_else(|| report_error("unknown occurrence counter overflow"))?;
        } else {
            unknowns.insert(key, unknown);
        }
    }
    facts.unknown_constructs = unknowns.into_values().collect();
    let inventory = crate::baggage::BaggageInventory::resolve(baggage_indexes, baggage_payloads);
    crate::baggage::persist(&tx, &report.sha256, &inventory)?;
    facts.diagnostics.extend(baggage_diagnostics(&inventory)?);
    report.baggage = Some(inventory);
    facts.sort()?;
    let unknown_distinct = u64::try_from(facts.unknown_constructs.len())
        .map_err(|_| report_error("unknown distinct count exceeds u64"))?;
    let unknown_occurrences = facts.unknown_occurrences;
    add_count(
        &mut facts,
        InstallCategory::UnknownConstruct,
        InstallDisposition::Read,
        unknown_occurrences,
    )?;
    add_count(
        &mut facts,
        InstallCategory::UnknownConstruct,
        InstallDisposition::Stored,
        unknown_distinct,
    )?;
    finalize_facts(&mut facts)?;
    persist_facts(&tx, &report.sha256, facts.clone())?;
    report.facts = Some(facts);
    tx.execute(
        "UPDATE package SET unknown_count = ?2, translation_program_count = ?3,
                translation_catalog_count = ?4, translation_hardware_count = ?5,
                translation_master_count = ?6, dropped_datapoint_type_count = ?7
         WHERE sha256 = ?1",
        params![
            report.sha256,
            usize_to_i64(report.unknown, "package unknown count")?,
            usize_to_i64(report.translations.program, "program translation count")?,
            usize_to_i64(report.translations.catalog, "catalog translation count")?,
            usize_to_i64(report.translations.hardware, "hardware translation count")?,
            usize_to_i64(report.translations.master, "master translation count")?,
            usize_to_i64(
                report.dropped_datapoint_types,
                "dropped datapoint type count"
            )?,
        ],
    )?;
    for (ordinal, conflict) in report.conflicts.iter().enumerate() {
        tx.execute(
            "INSERT INTO package_conflict (package_sha256, ordinal, table_name, logical_id, kept_sha256, other_sha256, occurrence) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                report.sha256,
                usize_to_i64(ordinal, "package conflict ordinal")?,
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
