//! A read-only summary of one legacy file: identity, tables, products and diagnostics.

use std::collections::HashMap;

use super::container::{LegacyMemberKind, LegacyPayload};
use super::error::LegacyError;
use super::exim::{parse_exim, ExImContent, ExImDiagnostic, ExImDocument, ExImTable};
use crate::blob::sha256_hex;

/// The payload charset is not documented anywhere; see `text.rs`.
pub const PAYLOAD_CHARSET_ASSUMPTION: &str = "windows-1252 (assumed)";

/// One table of the payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyTableSummary {
    pub id: u32,
    pub name: String,
    pub columns: usize,
    pub rows: usize,
}

/// One `virtual_device` row with what it refers to. Every field is read by
/// its column name; the names are observed, not documented, so a missing
/// column or reference leaves the field `None` rather than failing.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LegacyProductSummary {
    pub order_number: Option<String>,
    pub name: Option<String>,
    pub manufacturer: Option<String>,
    pub program_name: Option<String>,
    pub program_version: Option<String>,
    /// `MV-xxxx`: the mask's `MASK_VERSION` as four hex digits. `[A]`
    /// backed by one pair: `MASK_VERSION 1793` in the measured `.vd4` and
    /// `MaskVersion="MV-0701"` in ETS's own conversion of that program.
    pub mask_version: Option<String>,
}

/// Everything [`inspect_payload`] learned; nothing was stored anywhere.
#[derive(Debug, Clone)]
pub struct LegacyInspection {
    pub source_sha256: String,
    pub source_len: usize,
    pub member_name: String,
    pub member_kind: LegacyMemberKind,
    pub encrypted: bool,
    pub payload_sha256: String,
    pub payload_len: usize,
    pub content: ExImContent,
    pub format_version: Option<String>,
    pub exported_at: Option<String>,
    pub producer: Option<String>,
    pub tables: Vec<LegacyTableSummary>,
    pub products: Vec<LegacyProductSummary>,
    pub continuation_lines: usize,
    pub diagnostics: Vec<ExImDiagnostic>,
}

impl LegacyInspection {
    pub fn total_rows(&self) -> usize {
        self.tables.iter().map(|t| t.rows).sum()
    }
}

/// Parses and summarises an opened payload. Writes nothing.
pub fn inspect_payload(payload: &LegacyPayload) -> Result<LegacyInspection, LegacyError> {
    let document = parse_exim(payload.bytes())?;
    let container = payload.container();
    Ok(LegacyInspection {
        source_sha256: container.sha256.clone(),
        source_len: container.len,
        member_name: container.member_name.clone(),
        member_kind: container.member_kind,
        encrypted: container.encrypted,
        payload_sha256: sha256_hex(payload.bytes()),
        payload_len: payload.bytes().len(),
        content: document.content(),
        format_version: document.format_version().map(str::to_string),
        exported_at: document.exported_at().map(str::to_string),
        producer: document.producer().map(str::to_string),
        tables: document
            .tables()
            .iter()
            .map(|table| LegacyTableSummary {
                id: table.id(),
                name: table.name().to_string(),
                columns: table.columns().len(),
                rows: table.row_count(),
            })
            .collect(),
        products: products(&document),
        continuation_lines: document.continuation_lines(),
        diagnostics: document.diagnostics().to_vec(),
    })
}

/// Rows of `table` keyed by the text of `key_column` (first row wins).
struct Index<'a> {
    table: Option<&'a ExImTable>,
    rows: HashMap<String, usize>,
}

impl<'a> Index<'a> {
    fn new(document: &'a ExImDocument, table: &str, key_column: &str) -> Self {
        let table = document.table(table);
        let mut rows = HashMap::new();
        if let Some(t) = table {
            if let Some(column) = t.column_index(key_column) {
                for row in 0..t.row_count() {
                    rows.entry(t.text(row, column).into_owned()).or_insert(row);
                }
            }
        }
        Self { table, rows }
    }

    /// The non-empty value of `column` in the row whose key is `key`.
    fn get(&self, key: Option<&str>, column: &str) -> Option<String> {
        let table = self.table?;
        let row = *self.rows.get(key?)?;
        non_empty(table.text_by_name(row, column)?.into_owned())
    }
}

fn non_empty(value: String) -> Option<String> {
    (!value.is_empty()).then_some(value)
}

/// One summary per `virtual_device` row, joined to its catalogue entry,
/// manufacturer, application program and mask by the observed id columns.
fn products(document: &ExImDocument) -> Vec<LegacyProductSummary> {
    let Some(devices) = document.table("virtual_device") else {
        return Vec::new();
    };
    let catalog = Index::new(document, "catalog_entry", "CATALOG_ENTRY_ID");
    let manufacturers = Index::new(document, "manufacturer", "MANUFACTURER_ID");
    let programs = Index::new(document, "application_program", "PROGRAM_ID");
    let masks = Index::new(document, "mask", "MASK_ID");
    (0..devices.row_count())
        .map(|row| {
            let field = |column: &str| {
                devices
                    .text_by_name(row, column)
                    .and_then(|v| non_empty(v.into_owned()))
            };
            let catalog_id = field("CATALOG_ENTRY_ID");
            let program_id = field("PROGRAM_ID");
            let manufacturer_id = catalog.get(catalog_id.as_deref(), "MANUFACTURER_ID");
            let mask_id = programs.get(program_id.as_deref(), "MASK_ID");
            LegacyProductSummary {
                order_number: catalog.get(catalog_id.as_deref(), "ORDER_NUMBER"),
                name: catalog
                    .get(catalog_id.as_deref(), "ENTRY_NAME")
                    .or_else(|| field("VIRTUAL_DEVICE_NAME")),
                manufacturer: manufacturers.get(manufacturer_id.as_deref(), "MANUFACTURER_NAME"),
                program_name: programs.get(program_id.as_deref(), "PROGRAM_NAME"),
                program_version: programs.get(program_id.as_deref(), "PROGRAM_VERSION"),
                mask_version: masks
                    .get(mask_id.as_deref(), "MASK_VERSION")
                    .and_then(|v| v.parse::<u16>().ok())
                    .map(|v| format!("MV-{v:04X}")),
            }
        })
        .collect()
}
