//! Maps a parsed legacy EX-IM database onto product-database rows by the measured ETS rules.
//!
//! Every rule here was measured against ETS's own conversion of a real
//! `.vd4` program (docs/research/legacy-vd-mapping.md, ADR-0094). The result
//! is plain data: nothing is written here, so the rules are testable without
//! a database and the publication step stays one transaction.
//!
//! Identifiers live in a namespace of their own, `LX<sha8>` (the first eight
//! hex digits of the decrypted payload's SHA-256), placed inside the
//! segment ETS would use so every id keeps the segment count of a real ETS
//! id: `M-1092_A-LX1A2B3C4D-300`. They cannot collide with ETS ids, and two
//! different files never share one.

use std::collections::{BTreeMap, BTreeSet};

use super::exim::{ExImContent, ExImDocument, ExImTable};
use super::mapping_program::{map_programs, TranslationTarget, TranslationTargets};
use super::LegacyError;

pub(super) const NO_MANUFACTURER: &str = "MANUFACTURER_ID is empty or not a manufacturer number";

/// Tables the mapping reads. Every other table with rows is reported as
/// [`MappingDiagnostic::UnmappedTable`] and stays in the stored payload.
pub(super) const MAPPED_TABLES: &[&str] = &[
    "manufacturer",
    "ete_language",
    "mask",
    "hw_product",
    "catalog_entry",
    "application_program",
    "product_to_program",
    "virtual_device",
    "functional_entity",
    "parameter_atomic_type",
    "parameter_type",
    "parameter_list_of_values",
    "parameter",
    "object_type",
    "object_priority",
    "communication_object",
    "text_attribute",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyMapping {
    /// `LX` plus the payload digest's first eight hex digits, upper case.
    pub namespace: String,
    pub manufacturers: Vec<MappedManufacturer>,
    pub catalog_sections: Vec<MappedCatalogSection>,
    pub catalog_items: Vec<MappedCatalogItem>,
    pub hardware: Vec<MappedHardware>,
    pub products: Vec<MappedProduct>,
    pub hardware2programs: Vec<MappedHardware2Program>,
    pub programs: Vec<MappedProgram>,
    pub translations: Vec<MappedTranslation>,
    pub diagnostics: Vec<MappingDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappedManufacturer {
    pub id: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappedCatalogSection {
    pub id: String,
    pub manufacturer_id: String,
    pub parent_id: Option<String>,
    pub name: Option<String>,
    pub number: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappedCatalogItem {
    pub id: String,
    pub manufacturer_id: String,
    pub section_id: String,
    pub name: Option<String>,
    /// The order number, as ETS shows it in the catalog.
    pub number: Option<String>,
    pub product_ref_id: Option<String>,
    pub hardware2program_ref_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappedHardware {
    pub id: String,
    pub manufacturer_id: String,
    pub name: Option<String>,
    pub serial_number: Option<String>,
    pub version_number: Option<String>,
    pub bus_current: Option<String>,
    pub original_manufacturer: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappedProduct {
    pub id: String,
    pub manufacturer_id: String,
    pub hardware_id: String,
    pub text: Option<String>,
    pub order_number: Option<String>,
    pub is_rail_mounted: Option<bool>,
    pub width_in_millimeter: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappedHardware2Program {
    pub id: String,
    pub manufacturer_id: String,
    pub hardware_id: String,
    pub application_program_ref: Option<String>,
    pub registration_number: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappedProgram {
    pub id: String,
    /// `application_program.PROGRAM_ID` in the file.
    pub exim_program_id: String,
    pub manufacturer_id: String,
    pub name: Option<String>,
    pub application_number: Option<String>,
    pub application_version: Option<String>,
    pub mask_version: Option<String>,
    pub pei_type: Option<String>,
    pub linkable: Option<bool>,
    pub original_manufacturer: Option<String>,
    pub default_language: Option<String>,
    pub parameter_types: Vec<MappedParameterType>,
    pub enumerations: Vec<MappedEnumeration>,
    pub parameters: Vec<MappedParameter>,
    pub parameter_refs: Vec<MappedParameterRef>,
    pub com_objects: Vec<MappedComObject>,
    pub com_object_refs: Vec<MappedComObjectRef>,
    pub dynamic: Vec<MappedDynamicNode>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappedParameterType {
    pub id: String,
    pub name: Option<String>,
    /// `Number`, `Restriction` or `None`, as the XML path stores them.
    pub kind: String,
    pub size_in_bit: Option<i64>,
    pub base: Option<String>,
    pub min_inclusive: Option<String>,
    pub max_inclusive: Option<String>,
    pub number_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappedEnumeration {
    pub parameter_type_id: String,
    pub id: String,
    pub value: String,
    pub text: Option<String>,
    pub display_order: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappedParameter {
    pub id: String,
    pub name: Option<String>,
    pub text: Option<String>,
    pub parameter_type_id: String,
    pub access: Option<String>,
    pub value: Option<String>,
    pub code_segment: Option<String>,
    pub offset: Option<i64>,
    pub bit_offset: Option<i64>,
    pub union_id: Option<i64>,
    pub union_size_in_bit: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappedParameterRef {
    pub id: String,
    pub parameter_id: String,
    pub display_order: Option<i64>,
    /// `PARAMETER_NUMBER`, as ETS writes it.
    pub tag: Option<String>,
    /// Set only where this member's text differs from its parameter's.
    pub text: Option<String>,
    /// Set only where this member's default differs from its parameter's.
    pub value: Option<String>,
    /// Set only where this member's access differs from its parameter's.
    pub access: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappedComObject {
    pub id: String,
    pub number: Option<i64>,
    pub name: Option<String>,
    pub text: Option<String>,
    pub function_text: Option<String>,
    pub object_size: Option<String>,
    pub priority: Option<String>,
    pub read_flag: Option<String>,
    pub write_flag: Option<String>,
    pub transmit_flag: Option<String>,
    pub update_flag: Option<String>,
    pub communication_flag: Option<String>,
    pub read_on_init_flag: Option<String>,
}

/// Fields other than `id`/`com_object_id`/`tag` are set only where the
/// member differs from its object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappedComObjectRef {
    pub id: String,
    pub com_object_id: String,
    /// `OBJECT_UNIQUE_NUMBER`, as ETS writes it.
    pub tag: Option<String>,
    pub text: Option<String>,
    pub function_text: Option<String>,
    pub object_size: Option<String>,
    pub priority: Option<String>,
    pub read_flag: Option<String>,
    pub write_flag: Option<String>,
    pub transmit_flag: Option<String>,
    pub update_flag: Option<String>,
    pub communication_flag: Option<String>,
    pub read_on_init_flag: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappedDynamicNode {
    pub node_id: i64,
    pub parent_id: Option<i64>,
    pub position: i64,
    pub kind: String,
    pub element_id: Option<String>,
    pub ref_id: Option<String>,
    pub test: Option<String>,
    pub is_default: bool,
    pub text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct MappedTranslation {
    /// `Program` or `Catalog`, as the XML path stores them.
    pub scope: String,
    pub scope_id: String,
    pub language: String,
    pub ref_id: String,
    pub attribute_name: String,
    pub text: String,
}

/// What the mapping could not carry over, or carried over by a rule worth
/// naming. Reported, never silently applied (ADR-0094).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MappingDiagnostic {
    /// A table with rows that no mapping reads. Its rows stay in the stored
    /// payload.
    UnmappedTable { name: String, rows: usize },
    /// `text_attribute` rows whose `COLUMN_ID` has no measured meaning.
    UnknownTextColumn { column_id: String, rows: usize },
    /// `text_attribute` rows in a language id without a known BCP 47 tag.
    UnknownLanguage { language_id: String, rows: usize },
    /// A translation whose entity no mapped row has.
    OrphanTranslation { column_id: String, rows: usize },
    /// An access level other than 0/1/2; the parameter is mapped `None`.
    UnknownAccessLevel { parameter: String, level: String },
    /// An atomic type other than 0/1/2/4; its parameters are not mapped.
    UnknownAtomicType {
        parameter_type: String,
        atomic_type: String,
    },
    /// A value that should name a row of another table and names none.
    DanglingReference {
        table: String,
        column: String,
        value: String,
    },
    /// Parameters whose memory overlaps without starting at the same bit,
    /// which a union cannot express here. Mapped as separate parameters.
    OverlappingMemory { parameters: Vec<String> },
    /// Child rows of a page parameter that carry a parent value; a page has
    /// no value, so they are placed as always visible.
    ValueOnPageParent { parameter: String },
    /// A program without any page; its parameters sit directly in the
    /// channel.
    NoPage { program: String },
    /// A column with values that no mapping reads (for example
    /// `EIB_DATA_TYPE_CODE`, whose encoding is unmeasured).
    UnmappedColumn {
        table: String,
        column: String,
        rows: usize,
    },
    /// A parameter the visibility tree could not reach (a cycle in its
    /// parent chain). It exists, but no page shows it.
    UnplacedParameter { parameter: String },
    /// Two translations for one element, attribute and language with
    /// different texts; the first is kept.
    ConflictingTranslation { ref_id: String, language: String },
    /// Non-empty values of a secret-class column (`*PASSWORD*`) were blanked
    /// in the stored payload. Only the count is kept, never a value.
    SecretWithheld {
        table: String,
        column: String,
        rows: usize,
    },
    /// Rows that cannot be mapped (a key is empty, or they belong to
    /// nothing that was mapped). They stay in the stored payload.
    SkippedRows {
        table: String,
        reason: String,
        rows: usize,
    },
}

impl MappingDiagnostic {
    /// A stable kind name for reports and storage.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::UnmappedTable { .. } => "unmapped-table",
            Self::UnknownTextColumn { .. } => "unknown-text-column",
            Self::UnknownLanguage { .. } => "unknown-language",
            Self::OrphanTranslation { .. } => "orphan-translation",
            Self::UnknownAccessLevel { .. } => "unknown-access-level",
            Self::UnknownAtomicType { .. } => "unknown-atomic-type",
            Self::DanglingReference { .. } => "dangling-reference",
            Self::OverlappingMemory { .. } => "overlapping-memory",
            Self::ValueOnPageParent { .. } => "value-on-page-parent",
            Self::NoPage { .. } => "no-page",
            Self::UnmappedColumn { .. } => "unmapped-column",
            Self::UnplacedParameter { .. } => "unplaced-parameter",
            Self::ConflictingTranslation { .. } => "conflicting-translation",
            Self::SecretWithheld { .. } => "secret-withheld",
            Self::SkippedRows { .. } => "skipped-rows",
        }
    }
}

impl std::fmt::Display for MappingDiagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnmappedTable { name, rows } => {
                write!(f, "table {name} ({rows} rows) is kept in the payload but not mapped")
            }
            Self::UnknownTextColumn { column_id, rows } => {
                write!(f, "{rows} text_attribute rows with unknown COLUMN_ID {column_id} were not mapped")
            }
            Self::UnknownLanguage { language_id, rows } => {
                write!(f, "{rows} text_attribute rows in unknown language {language_id} were not mapped")
            }
            Self::OrphanTranslation { column_id, rows } => {
                write!(f, "{rows} text_attribute rows (COLUMN_ID {column_id}) translate nothing that was mapped")
            }
            Self::UnknownAccessLevel { parameter, level } => {
                write!(f, "parameter {parameter} has access level {level:?}; mapped as None")
            }
            Self::UnknownAtomicType { parameter_type, atomic_type } => write!(
                f,
                "parameter type {parameter_type} has unknown atomic type {atomic_type}; its parameters were not mapped"
            ),
            Self::DanglingReference { table, column, value } => {
                write!(f, "{table}.{column} = {value} names no row")
            }
            Self::OverlappingMemory { parameters } => write!(
                f,
                "parameters {} overlap in memory from different starts; mapped separately",
                parameters.join(", ")
            ),
            Self::ValueOnPageParent { parameter } => write!(
                f,
                "children of page {parameter} carry a parent value; placed as always visible"
            ),
            Self::NoPage { program } => {
                write!(f, "program {program} has no page; its parameters sit in the channel")
            }
            Self::UnmappedColumn { table, column, rows } => {
                write!(f, "{table}.{column} has values in {rows} rows that were not mapped")
            }
            Self::UnplacedParameter { parameter } => write!(
                f,
                "parameter {parameter} is in no page (cycle in its parent chain)"
            ),
            Self::ConflictingTranslation { ref_id, language } => write!(
                f,
                "conflicting {language} translations for {ref_id}; the first was kept"
            ),
            Self::SecretWithheld {
                table,
                column,
                rows,
            } => write!(
                f,
                "{rows} value{} of {table}.{column} {} withheld from the stored payload (secret-class column)",
                if *rows == 1 { "" } else { "s" },
                if *rows == 1 { "was" } else { "were" }
            ),
            Self::SkippedRows { table, reason, rows } => write!(
                f,
                "{rows} {table} rows were not mapped ({reason}); they stay in the payload"
            ),
        }
    }
}

/// One row of an EX-IM table, read by column name. Empty values read as
/// `None`, the way the files write NULL.
#[derive(Clone, Copy)]
pub(super) struct Row<'a> {
    table: &'a ExImTable,
    index: usize,
}

impl<'a> Row<'a> {
    pub(super) fn get(&self, column: &str) -> Option<String> {
        self.table
            .text_by_name(self.index, column)
            .map(|v| v.into_owned())
            .filter(|v| !v.is_empty())
    }

    /// The value as written, empty included; `None` only when the table
    /// has no such column. ETS's conversion keeps empty texts and defaults
    /// as `""` (measured), which the evaluator treats differently from an
    /// absent value.
    pub(super) fn text(&self, column: &str) -> Option<String> {
        self.table
            .text_by_name(self.index, column)
            .map(|v| v.into_owned())
    }

    pub(super) fn int(&self, column: &str) -> Option<i64> {
        self.get(column).and_then(|v| v.trim().parse().ok())
    }
}

/// Notes one row that cannot be mapped; [`aggregate_skips`] sums them.
pub(super) fn skip(diagnostics: &mut Vec<MappingDiagnostic>, table: &str, reason: &str) {
    diagnostics.push(MappingDiagnostic::SkippedRows {
        table: table.into(),
        reason: reason.into(),
        rows: 1,
    });
}

/// Sums [`MappingDiagnostic::SkippedRows`] per (table, reason), keeping the
/// first occurrence's position.
fn aggregate_skips(diagnostics: Vec<MappingDiagnostic>) -> Vec<MappingDiagnostic> {
    let mut out: Vec<MappingDiagnostic> = Vec::new();
    for d in diagnostics {
        if let MappingDiagnostic::SkippedRows {
            table,
            reason,
            rows,
        } = &d
        {
            if let Some(MappingDiagnostic::SkippedRows { rows: total, .. }) =
                out.iter_mut().find(|o| {
                    matches!(o, MappingDiagnostic::SkippedRows { table: t, reason: r, .. }
                        if t == table && r == reason)
                })
            {
                *total += rows;
                continue;
            }
        }
        out.push(d);
    }
    out
}

/// Counts rows of program-scoped tables whose `PROGRAM_ID` names no
/// mapped program, and enumeration rows of no mapped enumeration type.
fn skip_orphans(
    document: &ExImDocument,
    programs: &[MappedProgram],
    diagnostics: &mut Vec<MappingDiagnostic>,
) {
    let ids: BTreeSet<&str> = programs
        .iter()
        .map(|p| p.exim_program_id.as_str())
        .collect();
    for table in ["parameter_type", "parameter", "communication_object"] {
        for r in rows(document, table) {
            if !r
                .get("PROGRAM_ID")
                .is_some_and(|p| ids.contains(p.as_str()))
            {
                skip(diagnostics, table, "PROGRAM_ID names no mapped program");
            }
        }
    }
    let enum_types: BTreeSet<String> = programs
        .iter()
        .flat_map(|p| {
            p.parameter_types
                .iter()
                .filter(|t| t.kind == "Restriction")
                .filter_map(|t| t.id.rsplit("_PT-").next().map(str::to_string))
        })
        .collect();
    for r in rows(document, "parameter_list_of_values") {
        if !r
            .get("PARAMETER_TYPE_ID")
            .is_some_and(|t| enum_types.contains(&t))
        {
            skip(
                diagnostics,
                "parameter_list_of_values",
                "PARAMETER_TYPE_ID names no mapped enumeration type",
            );
        }
    }
}

pub(super) fn rows<'a>(document: &'a ExImDocument, table: &str) -> Vec<Row<'a>> {
    document
        .table(table)
        .map(|t| {
            (0..t.row_count())
                .map(|index| Row { table: t, index })
                .collect()
        })
        .unwrap_or_default()
}

/// `M-` and the KNX manufacturer number as four upper-case hex digits
/// (measured: 106 → `M-006A`, 121 → `M-0079`, 131 → `M-0083`).
pub(super) fn manufacturer_id(number: &str) -> Option<String> {
    let n: u16 = number.trim().parse().ok()?;
    Some(format!("M-{n:04X}"))
}

/// Windows LCIDs as the files write them, to BCP 47 (Microsoft's published
/// LCID table). An LCID outside this list is reported, not guessed.
pub(super) fn bcp47(lcid: &str) -> Option<&'static str> {
    Some(match lcid.trim() {
        "1026" => "bg-BG",
        "1029" => "cs-CZ",
        "1030" => "da-DK",
        "1031" => "de-DE",
        "1032" => "el-GR",
        "1033" => "en-US",
        "1034" | "3082" => "es-ES",
        "1035" => "fi-FI",
        "1036" => "fr-FR",
        "1038" => "hu-HU",
        "1040" => "it-IT",
        "1041" => "ja-JP",
        "1042" => "ko-KR",
        "1043" => "nl-NL",
        "1044" => "nb-NO",
        "1045" => "pl-PL",
        "1046" => "pt-BR",
        "1048" => "ro-RO",
        "1049" => "ru-RU",
        "1050" => "hr-HR",
        "1051" => "sk-SK",
        "1053" => "sv-SE",
        "1055" => "tr-TR",
        "1058" => "uk-UA",
        "1060" => "sl-SI",
        "1061" => "et-EE",
        "1062" => "lv-LV",
        "1063" => "lt-LT",
        "2052" => "zh-CN",
        "2055" => "de-CH",
        "2057" => "en-GB",
        "2067" => "nl-BE",
        "2070" => "pt-PT",
        "3079" => "de-AT",
        _ => return None,
    })
}

/// Maps a decrypted, parsed product database. `payload_sha256` is the hex
/// digest of the decrypted payload; it names the id namespace.
pub fn map_legacy_database(
    document: &ExImDocument,
    payload_sha256: &str,
) -> Result<LegacyMapping, LegacyError> {
    if document.content() != ExImContent::ProductDatabase {
        return Err(LegacyError::Mapping {
            reason: "only a product database (H virtual_device) can be published".into(),
        });
    }
    let digest = payload_sha256
        .get(..8)
        .filter(|d| d.bytes().all(|b| b.is_ascii_hexdigit()));
    let Some(digest) = digest else {
        return Err(LegacyError::Mapping {
            reason: "the payload digest is not hexadecimal".into(),
        });
    };
    let ns = format!("LX{}", digest.to_ascii_uppercase());
    let mut diagnostics = Vec::new();

    for table in document.tables() {
        if table.row_count() > 0 && !MAPPED_TABLES.contains(&table.name()) {
            diagnostics.push(MappingDiagnostic::UnmappedTable {
                name: table.name().to_string(),
                rows: table.row_count(),
            });
        }
    }

    let mut manufacturers = Vec::new();
    for r in rows(document, "manufacturer") {
        match r.get("MANUFACTURER_ID").and_then(|m| manufacturer_id(&m)) {
            Some(id) => manufacturers.push(MappedManufacturer {
                id,
                name: r.get("MANUFACTURER_NAME"),
            }),
            None => skip(&mut diagnostics, "manufacturer", NO_MANUFACTURER),
        }
    }

    let database_language = rows(document, "ete_language")
        .iter()
        .find(|r| r.get("DATABASE_LANGUAGE").as_deref() == Some("1"))
        .and_then(|r| r.get("LANGUAGE_ID"));
    let default_language = database_language.as_deref().and_then(bcp47);

    let mut programs = map_programs(document, &ns, default_language, &mut diagnostics);
    skip_orphans(document, &programs, &mut diagnostics);
    programs.sort_by(|a, b| a.id.cmp(&b.id));
    let program_ids: BTreeMap<String, String> = programs
        .iter()
        .map(|p| (p.exim_program_id.clone(), p.id.clone()))
        .collect();

    if programs.is_empty() {
        return Err(LegacyError::Mapping {
            reason: "the file holds no application program".into(),
        });
    }
    let catalog = map_catalog(document, &ns, &program_ids, &mut diagnostics);
    let translations = map_translations(
        document,
        &ns,
        &programs,
        &catalog.items_by_device,
        &mut diagnostics,
    );

    let diagnostics = aggregate_skips(diagnostics);
    Ok(LegacyMapping {
        namespace: ns,
        manufacturers,
        catalog_sections: catalog.sections,
        catalog_items: catalog.items,
        hardware: catalog.hardware,
        products: catalog.products,
        hardware2programs: catalog.hardware2programs,
        programs,
        translations,
        diagnostics,
    })
}

struct Catalog {
    sections: Vec<MappedCatalogSection>,
    items: Vec<MappedCatalogItem>,
    hardware: Vec<MappedHardware>,
    products: Vec<MappedProduct>,
    hardware2programs: Vec<MappedHardware2Program>,
    /// `VIRTUAL_DEVICE_ID` → (manufacturer id, catalog item id).
    items_by_device: BTreeMap<String, (String, String)>,
}

fn dangling(diagnostics: &mut Vec<MappingDiagnostic>, table: &str, column: &str, value: &str) {
    diagnostics.push(MappingDiagnostic::DanglingReference {
        table: table.into(),
        column: column.into(),
        value: value.into(),
    });
}

fn map_catalog(
    document: &ExImDocument,
    ns: &str,
    program_ids: &BTreeMap<String, String>,
    diagnostics: &mut Vec<MappingDiagnostic>,
) -> Catalog {
    let mut hardware = Vec::new();
    let mut hardware_by_product = BTreeMap::new();
    for r in rows(document, "hw_product") {
        let (Some(pid), Some(mid)) = (
            r.get("PRODUCT_ID"),
            r.get("MANUFACTURER_ID").and_then(|m| manufacturer_id(&m)),
        ) else {
            skip(
                diagnostics,
                "hw_product",
                "PRODUCT_ID or MANUFACTURER_ID is empty or invalid",
            );
            continue;
        };
        let id = format!("{mid}_H-{ns}-{pid}");
        hardware_by_product.insert(pid, (mid.clone(), id.clone()));
        hardware.push(MappedHardware {
            id,
            manufacturer_id: mid,
            name: r.get("PRODUCT_NAME"),
            serial_number: r.get("PRODUCT_SERIAL_NUMBER"),
            version_number: r.get("PRODUCT_VERSION_NUMBER"),
            bus_current: r.get("BUS_CURRENT"),
            original_manufacturer: r
                .get("ORIGINAL_MANUFACTURER_ID")
                .and_then(|m| manufacturer_id(&m)),
        });
    }

    let mut hardware2programs = Vec::new();
    let mut h2p_by_pair = BTreeMap::new();
    for r in rows(document, "product_to_program") {
        let (Some(id), Some(product), Some(program)) = (
            r.get("PROD2PROG_ID"),
            r.get("PRODUCT_ID"),
            r.get("PROGRAM_ID"),
        ) else {
            skip(
                diagnostics,
                "product_to_program",
                "PROD2PROG_ID, PRODUCT_ID or PROGRAM_ID is empty",
            );
            continue;
        };
        let Some((mid, hw)) = hardware_by_product.get(&product) else {
            dangling(diagnostics, "product_to_program", "PRODUCT_ID", &product);
            continue;
        };
        let Some(program_id) = program_ids.get(&program) else {
            dangling(diagnostics, "product_to_program", "PROGRAM_ID", &program);
            continue;
        };
        let h2p = format!("{hw}_HP-{id}");
        h2p_by_pair.insert((product, program), h2p.clone());
        hardware2programs.push(MappedHardware2Program {
            id: h2p,
            manufacturer_id: mid.clone(),
            hardware_id: hw.clone(),
            application_program_ref: Some(program_id.clone()),
            registration_number: r.get("REGISTRATION_NUMBER"),
        });
    }

    let mut products = Vec::new();
    let mut product_by_entry = BTreeMap::new();
    for r in rows(document, "catalog_entry") {
        let (Some(entry), Some(product)) = (r.get("CATALOG_ENTRY_ID"), r.get("PRODUCT_ID")) else {
            skip(
                diagnostics,
                "catalog_entry",
                "CATALOG_ENTRY_ID or PRODUCT_ID is empty",
            );
            continue;
        };
        let Some((mid, hw)) = hardware_by_product.get(&product) else {
            dangling(diagnostics, "catalog_entry", "PRODUCT_ID", &product);
            continue;
        };
        let id = format!("{hw}_P-{entry}");
        product_by_entry.insert(entry, (id.clone(), product, r));
        products.push(MappedProduct {
            id,
            manufacturer_id: mid.clone(),
            hardware_id: hw.clone(),
            text: r.get("ENTRY_NAME"),
            order_number: r.get("ORDER_NUMBER"),
            is_rail_mounted: r.get("DIN_FLAG").map(|v| v == "1"),
            width_in_millimeter: r.get("ENTRY_WIDTH_IN_MILLIMETERS"),
        });
    }

    let mut sections = Vec::new();
    let mut section_by_entity = BTreeMap::new();
    let entities = rows(document, "functional_entity");
    for r in &entities {
        let (Some(fe), Some(mid)) = (
            r.get("FUNCTIONAL_ENTITY_ID"),
            r.get("MANUFACTURER_ID").and_then(|m| manufacturer_id(&m)),
        ) else {
            skip(
                diagnostics,
                "functional_entity",
                "FUNCTIONAL_ENTITY_ID or MANUFACTURER_ID is empty or invalid",
            );
            continue;
        };
        section_by_entity.insert(fe.clone(), format!("{mid}_CS-{ns}-{fe}"));
    }
    for r in &entities {
        let (Some(fe), Some(mid)) = (
            r.get("FUNCTIONAL_ENTITY_ID"),
            r.get("MANUFACTURER_ID").and_then(|m| manufacturer_id(&m)),
        ) else {
            continue;
        };
        let parent_id = match r.get("FUN_FUNCTIONAL_ENTITY_ID") {
            None => None,
            Some(parent) => match section_by_entity.get(&parent) {
                Some(id) => Some(id.clone()),
                None => {
                    dangling(
                        diagnostics,
                        "functional_entity",
                        "FUN_FUNCTIONAL_ENTITY_ID",
                        &parent,
                    );
                    None
                }
            },
        };
        sections.push(MappedCatalogSection {
            id: section_by_entity[&fe].clone(),
            manufacturer_id: mid,
            parent_id,
            name: r.get("FUNCTIONAL_ENTITY_NAME"),
            number: r.get("FUNCTIONAL_ENTITY_NUMB"),
        });
    }

    let mut items = Vec::new();
    let mut items_by_device = BTreeMap::new();
    let mut fallback_sections = BTreeSet::new();
    for r in rows(document, "virtual_device") {
        let (Some(device), Some(entry)) = (r.get("VIRTUAL_DEVICE_ID"), r.get("CATALOG_ENTRY_ID"))
        else {
            skip(
                diagnostics,
                "virtual_device",
                "VIRTUAL_DEVICE_ID or CATALOG_ENTRY_ID is empty",
            );
            continue;
        };
        let Some((product_id, product, entry_row)) = product_by_entry.get(&entry) else {
            dangling(diagnostics, "virtual_device", "CATALOG_ENTRY_ID", &entry);
            continue;
        };
        let (mid, _) = &hardware_by_product[product];
        let h2p = r
            .get("PROGRAM_ID")
            .and_then(|program| h2p_by_pair.get(&(product.clone(), program)).cloned());
        let section_id = match r
            .get("FUNCTIONAL_ENTITY_ID")
            .and_then(|fe| section_by_entity.get(&fe).cloned())
        {
            Some(id) => id,
            None => {
                // A catalog item needs a section. A device the file files
                // under no functional entity goes to one per manufacturer,
                // named so the catalog shows where it came from.
                let id = format!("{mid}_CS-{ns}");
                if fallback_sections.insert(id.clone()) {
                    sections.push(MappedCatalogSection {
                        id: id.clone(),
                        manufacturer_id: mid.clone(),
                        parent_id: None,
                        name: None,
                        number: None,
                    });
                }
                id
            }
        };
        let id = format!("{mid}_CI-{ns}-{device}");
        items_by_device.insert(device, (mid.clone(), id.clone()));
        items.push(MappedCatalogItem {
            id,
            manufacturer_id: mid.clone(),
            section_id,
            name: entry_row.get("ENTRY_NAME"),
            number: entry_row.get("ORDER_NUMBER"),
            product_ref_id: Some(product_id.clone()),
            hardware2program_ref_id: h2p,
        });
    }

    Catalog {
        sections,
        items,
        hardware,
        products,
        hardware2programs,
        items_by_device,
    }
}

/// Measured `COLUMN_ID` meanings (docs/research/legacy-vd-mapping.md):
/// 1 catalog entry name, 10 parameter text, 11 enumeration text, 20 object
/// name, 22 object function. The others are known but have no mapped
/// target; they are counted as orphans, not as unknown.
const KNOWN_TEXT_COLUMNS: &[&str] = &[
    "1", "10", "11", "20", "22", "30", "31", "40", "41", "80", "90",
];

fn map_translations(
    document: &ExImDocument,
    ns: &str,
    programs: &[MappedProgram],
    items_by_device: &BTreeMap<String, (String, String)>,
    diagnostics: &mut Vec<MappingDiagnostic>,
) -> Vec<MappedTranslation> {
    let mut targets = TranslationTargets::new();
    for program in programs {
        super::mapping_program::translation_targets(document, ns, program, &mut targets);
    }
    // Catalog entry names translate the catalog items that show them.
    let devices = rows(document, "virtual_device");
    for (device, (mid, item)) in items_by_device {
        if let Some(entry) = devices
            .iter()
            .find(|r| r.get("VIRTUAL_DEVICE_ID").as_deref() == Some(device.as_str()))
            .and_then(|r| r.get("CATALOG_ENTRY_ID"))
        {
            targets
                .entry(("1".into(), entry))
                .or_default()
                .push(TranslationTarget {
                    scope: "Catalog".into(),
                    scope_id: mid.clone(),
                    element: item.clone(),
                    attribute: "Name".into(),
                    unless_same_as: None,
                });
        }
    }

    // The text of every (column, entity, language), first row wins, for the
    // "differs from the representative" rule.
    let mut index: BTreeMap<(String, String, String), String> = BTreeMap::new();
    for r in rows(document, "text_attribute") {
        if let (Some(language), Some(column), Some(entity), Some(text)) = (
            r.get("LANGUAGE_ID"),
            r.get("COLUMN_ID"),
            r.get("ENTITY_ID"),
            r.text("TEXT_ATTRIBUTE_TEXT"),
        ) {
            index.entry((column, entity, language)).or_insert(text);
        }
    }
    let mut out: BTreeMap<(String, String, String, String, String), String> = BTreeMap::new();
    let mut conflicts = BTreeSet::new();
    let mut unknown_columns: BTreeMap<String, usize> = BTreeMap::new();
    let mut unknown_languages: BTreeMap<String, usize> = BTreeMap::new();
    let mut orphans: BTreeMap<String, usize> = BTreeMap::new();
    for r in rows(document, "text_attribute") {
        let (Some(language), Some(column), Some(entity), Some(text)) = (
            r.get("LANGUAGE_ID"),
            r.get("COLUMN_ID"),
            r.get("ENTITY_ID"),
            r.get("TEXT_ATTRIBUTE_TEXT"),
        ) else {
            skip(
                diagnostics,
                "text_attribute",
                "LANGUAGE_ID, COLUMN_ID, ENTITY_ID or the text is empty",
            );
            continue;
        };
        if !KNOWN_TEXT_COLUMNS.contains(&column.as_str()) {
            *unknown_columns.entry(column).or_default() += 1;
            continue;
        }
        let Some(tag) = bcp47(&language) else {
            *unknown_languages.entry(language).or_default() += 1;
            continue;
        };
        match targets.get(&(column.clone(), entity)) {
            Some(list) => {
                for target in list {
                    if let Some(rep) = &target.unless_same_as {
                        let rep_text = index.get(&(column.clone(), rep.clone(), language.clone()));
                        if rep_text == Some(&text) {
                            continue;
                        }
                    }
                    let ref_id = &target.element;
                    let key = (
                        target.scope.clone(),
                        target.scope_id.clone(),
                        tag.to_string(),
                        ref_id.clone(),
                        target.attribute.clone(),
                    );
                    match out.get(&key) {
                        Some(kept) if kept != &text => {
                            conflicts.insert((ref_id.clone(), tag.to_string()));
                        }
                        Some(_) => {}
                        None => {
                            out.insert(key, text.clone());
                        }
                    }
                }
            }
            None => *orphans.entry(column).or_default() += 1,
        }
    }
    for (column_id, rows) in unknown_columns {
        diagnostics.push(MappingDiagnostic::UnknownTextColumn { column_id, rows });
    }
    for (language_id, rows) in unknown_languages {
        diagnostics.push(MappingDiagnostic::UnknownLanguage { language_id, rows });
    }
    for (column_id, rows) in orphans {
        diagnostics.push(MappingDiagnostic::OrphanTranslation { column_id, rows });
    }
    for (ref_id, language) in conflicts {
        diagnostics.push(MappingDiagnostic::ConflictingTranslation { ref_id, language });
    }
    out.into_iter()
        .map(
            |((scope, scope_id, language, ref_id, attribute_name), text)| MappedTranslation {
                scope,
                scope_id,
                language,
                ref_id,
                attribute_name,
                text,
            },
        )
        .collect()
}
