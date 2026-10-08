//! Publishes a mapped legacy product database into the product database, atomically.
//!
//! One transaction writes the original file, the decrypted payload, the
//! provenance rows and every mapped row, or nothing (ADR-0094). The same
//! payload published again (renamed, or encrypted afresh) adds only its
//! name and original file; it never rewrites rows. The password is not an
//! input here and is stored nowhere.

use rusqlite::{params, Connection, OptionalExtension};

use super::container::LegacyPayload;
use super::exim::parse_exim;
use super::inspect::PAYLOAD_CHARSET_ASSUMPTION;
use super::mapping::{map_legacy_database, LegacyMapping, MappedProgram, MappingDiagnostic};
use super::secrets::withhold_secret_values;
use super::LegacyError;
use crate::{sha256_hex, store_source_file, ProductDbError, SourceFile};

/// What a publication wrote, or found already written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyPublishReport {
    pub payload_sha256: String,
    pub original_sha256: String,
    /// `LX` plus eight hex digits: the id namespace of this payload.
    pub namespace: String,
    /// The payload was already published; only the name was recorded.
    pub skipped: bool,
    pub programs: Vec<String>,
    pub catalog_items: usize,
    pub parameters: usize,
    pub parameter_refs: usize,
    pub com_object_refs: usize,
    pub translations: usize,
    /// `(kind, detail)` of every mapping diagnostic, in order.
    pub diagnostics: Vec<(String, String)>,
}

#[derive(Debug)]
pub enum LegacyPublishError {
    Legacy(LegacyError),
    Store(ProductDbError),
}

impl std::fmt::Display for LegacyPublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Legacy(e) => e.fmt(f),
            Self::Store(e) => write!(f, "product database: {e}"),
        }
    }
}

impl std::error::Error for LegacyPublishError {}

impl From<LegacyError> for LegacyPublishError {
    fn from(e: LegacyError) -> Self {
        Self::Legacy(e)
    }
}

impl From<ProductDbError> for LegacyPublishError {
    fn from(e: ProductDbError) -> Self {
        Self::Store(e)
    }
}

impl From<rusqlite::Error> for LegacyPublishError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Store(e.into())
    }
}

/// Publishes `payload`, decrypted from `original`, under `source_name`.
pub fn publish_legacy(
    conn: &Connection,
    source_name: &str,
    original: &[u8],
    payload: &LegacyPayload,
) -> Result<LegacyPublishReport, LegacyPublishError> {
    let container = payload.container();
    let original_sha256 = sha256_hex(original);
    if original_sha256 != container.sha256 {
        return Err(LegacyError::Mapping {
            reason: "the payload was not read from these file bytes".into(),
        }
        .into());
    }
    // Only the copy without secret-class values is stored, parsed and keyed
    // (design decision B-3): its digest is the payload's identity.
    let withheld = withhold_secret_values(payload.bytes())?;
    let stored = withheld.bytes;
    let document = parse_exim(&stored)?;
    let payload_sha256 = sha256_hex(&stored);
    let mut mapping = map_legacy_database(&document, &payload_sha256)?;
    mapping
        .diagnostics
        .extend(
            withheld
                .columns
                .into_iter()
                .map(|c| MappingDiagnostic::SecretWithheld {
                    table: c.table,
                    column: c.column,
                    rows: c.rows,
                }),
        );

    let tx = conn.unchecked_transaction()?;
    let known: Option<String> = tx
        .query_row(
            "SELECT namespace FROM legacy_source WHERE payload_sha256 = ?1",
            [&payload_sha256],
            |r| r.get(0),
        )
        .optional()?;
    if known.is_none() {
        // Ids carry only eight hex digits of the digest; another payload
        // owning the same namespace would share them. Refused, never merged.
        let owner: Option<String> = tx
            .query_row(
                "SELECT payload_sha256 FROM legacy_source WHERE namespace = ?1",
                [&mapping.namespace],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(owner) = owner {
            return Err(LegacyError::Mapping {
                reason: format!(
                    "id namespace {} already belongs to another legacy database (payload {owner})",
                    mapping.namespace
                ),
            }
            .into());
        }
    }
    let manufacturer = mapping.manufacturers.first().map(|m| m.id.clone());
    store_source_file(
        &tx,
        &SourceFile {
            source_path: source_name.to_string(),
            manufacturer_id: manufacturer.clone(),
            bytes: original.to_vec(),
        },
    )?;
    if known.is_none() {
        tx.execute(
            "INSERT INTO legacy_source
             (payload_sha256, namespace, member_name, member_kind, format_version,
              exported_at, producer, charset)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![
                payload_sha256,
                mapping.namespace,
                container.member_name,
                format!("{:?}", container.member_kind),
                document.format_version(),
                document.exported_at(),
                document.producer(),
                PAYLOAD_CHARSET_ASSUMPTION,
            ],
        )?;
    }
    tx.execute(
        "INSERT OR IGNORE INTO legacy_source_file
         (payload_sha256, original_sha256, source_name, encrypted) VALUES (?1,?2,?3,?4)",
        params![
            payload_sha256,
            original_sha256,
            source_name,
            container.encrypted
        ],
    )?;
    if known.is_some() {
        let report = stored_report(&tx, &payload_sha256, &original_sha256, &mapping, true)?;
        tx.commit()?;
        return Ok(report);
    }

    store_source_file(
        &tx,
        &SourceFile {
            source_path: format!("{source_name}!{}", container.member_name),
            manufacturer_id: manufacturer,
            bytes: stored,
        },
    )?;
    write_rows(&tx, &payload_sha256, &mapping)?;
    for (ordinal, d) in mapping.diagnostics.iter().enumerate() {
        tx.execute(
            "INSERT INTO legacy_diagnostic (payload_sha256, ordinal, kind, detail)
             VALUES (?1,?2,?3,?4)",
            params![payload_sha256, ordinal as i64, d.kind(), d.to_string()],
        )?;
    }
    let report = stored_report(&tx, &payload_sha256, &original_sha256, &mapping, false)?;
    tx.commit()?;
    Ok(report)
}

fn stored_report(
    conn: &Connection,
    payload_sha256: &str,
    original_sha256: &str,
    mapping: &LegacyMapping,
    skipped: bool,
) -> Result<LegacyPublishReport, LegacyPublishError> {
    let programs = conn
        .prepare(
            "SELECT program_id FROM legacy_program WHERE payload_sha256 = ?1 ORDER BY program_id",
        )?
        .query_map([payload_sha256], |r| r.get(0))?
        .collect::<Result<Vec<String>, _>>()?;
    let diagnostics = conn
        .prepare(
            "SELECT kind, detail FROM legacy_diagnostic WHERE payload_sha256 = ?1 ORDER BY ordinal",
        )?
        .query_map([payload_sha256], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<Vec<(String, String)>, _>>()?;
    let count = |rows: fn(&MappedProgram) -> usize| mapping.programs.iter().map(rows).sum();
    Ok(LegacyPublishReport {
        payload_sha256: payload_sha256.to_string(),
        original_sha256: original_sha256.to_string(),
        namespace: mapping.namespace.clone(),
        skipped,
        programs,
        catalog_items: mapping.catalog_items.len(),
        parameters: count(|p| p.parameters.len()),
        parameter_refs: count(|p| p.parameter_refs.len()),
        com_object_refs: count(|p| p.com_object_refs.len()),
        translations: mapping.translations.len(),
        diagnostics,
    })
}

fn write_rows(
    conn: &Connection,
    sha: &str,
    mapping: &LegacyMapping,
) -> Result<(), LegacyPublishError> {
    for m in &mapping.manufacturers {
        conn.execute(
            "INSERT OR IGNORE INTO manufacturer (id, name) VALUES (?1, ?2)",
            params![m.id, m.name],
        )?;
        // Fill a missing name; never overwrite one another source gave.
        conn.execute(
            "UPDATE manufacturer SET name = ?2 WHERE id = ?1 AND name IS NULL",
            params![m.id, m.name],
        )?;
    }
    for s in &mapping.catalog_sections {
        conn.execute(
            "INSERT INTO catalog_section (id, manufacturer_id, parent_id, name, number, source_sha256)
             VALUES (?1,?2,?3,?4,?5,?6)",
            params![s.id, s.manufacturer_id, s.parent_id, s.name, s.number, sha],
        )?;
    }
    for h in &mapping.hardware {
        conn.execute(
            "INSERT INTO hardware (id, manufacturer_id, name, serial_number, version_number,
              bus_current, has_application_program, original_manufacturer, source_sha256)
             VALUES (?1,?2,?3,?4,?5,?6,1,?7,?8)",
            params![
                h.id,
                h.manufacturer_id,
                h.name,
                h.serial_number,
                h.version_number,
                h.bus_current,
                h.original_manufacturer,
                sha
            ],
        )?;
    }
    for p in &mapping.products {
        conn.execute(
            "INSERT INTO product (id, manufacturer_id, hardware_id, text, order_number,
              is_rail_mounted, width_in_millimeter, source_sha256)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![
                p.id,
                p.manufacturer_id,
                p.hardware_id,
                p.text,
                p.order_number,
                p.is_rail_mounted,
                p.width_in_millimeter,
                sha
            ],
        )?;
    }
    for h in &mapping.hardware2programs {
        conn.execute(
            "INSERT INTO hardware2program (id, manufacturer_id, hardware_id,
              application_program_ref, registration_number, source_sha256)
             VALUES (?1,?2,?3,?4,?5,?6)",
            params![
                h.id,
                h.manufacturer_id,
                h.hardware_id,
                h.application_program_ref,
                h.registration_number,
                sha
            ],
        )?;
    }
    for i in &mapping.catalog_items {
        conn.execute(
            "INSERT INTO catalog_item (id, manufacturer_id, section_id, name, number,
              product_ref_id, hardware2program_ref_id, source_sha256)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![
                i.id,
                i.manufacturer_id,
                i.section_id,
                i.name,
                i.number,
                i.product_ref_id,
                i.hardware2program_ref_id,
                sha
            ],
        )?;
    }
    for p in &mapping.programs {
        write_program(conn, sha, p)?;
    }
    for t in &mapping.translations {
        conn.execute(
            "INSERT INTO translation (scope, scope_id, language, ref_id, attribute_name, text)
             VALUES (?1,?2,?3,?4,?5,?6)",
            params![
                t.scope,
                t.scope_id,
                t.language,
                t.ref_id,
                t.attribute_name,
                t.text
            ],
        )?;
    }
    Ok(())
}

/// Writes one program. Its write authority (ADR-0080) is complete when the
/// rows are: `parameter_ref.access` carries every member's own access level,
/// and the EX-IM format has no `ParameterCalculation` that could make a
/// parameter computed, so `write_authority_recorded` is set here.
fn write_program(
    conn: &Connection,
    sha: &str,
    p: &MappedProgram,
) -> Result<(), LegacyPublishError> {
    conn.execute(
        "INSERT INTO application_program (id, manufacturer_id, name, application_number,
          application_version, program_type, mask_version, pei_type, default_language,
          linkable, original_manufacturer, source_sha256, write_authority_recorded)
         VALUES (?1,?2,?3,?4,?5,'ApplicationProgram',?6,?7,?8,?9,?10,?11,1)",
        params![
            p.id,
            p.manufacturer_id,
            p.name,
            p.application_number,
            p.application_version,
            p.mask_version,
            p.pei_type,
            p.default_language,
            p.linkable,
            p.original_manufacturer,
            sha
        ],
    )?;
    conn.execute(
        "INSERT INTO legacy_program (program_id, payload_sha256, exim_program_id) VALUES (?1,?2,?3)",
        params![p.id, sha, p.exim_program_id],
    )?;
    for t in &p.parameter_types {
        conn.execute(
            "INSERT INTO parameter_type (program_id, id, name, kind, size_in_bit, base,
              min_inclusive, max_inclusive, number_type)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![
                p.id,
                t.id,
                t.name,
                t.kind,
                t.size_in_bit,
                t.base,
                t.min_inclusive,
                t.max_inclusive,
                t.number_type
            ],
        )?;
    }
    for e in &p.enumerations {
        conn.execute(
            "INSERT INTO parameter_type_enum (program_id, parameter_type_id, id, value, text,
              display_order) VALUES (?1,?2,?3,?4,?5,?6)",
            params![
                p.id,
                e.parameter_type_id,
                e.id,
                e.value,
                e.text,
                e.display_order
            ],
        )?;
    }
    for q in &p.parameters {
        conn.execute(
            "INSERT INTO parameter (program_id, id, name, text, parameter_type_id, access, value,
              code_segment, offset, bit_offset, union_id, union_size_in_bit)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
            params![
                p.id,
                q.id,
                q.name,
                q.text,
                q.parameter_type_id,
                q.access,
                q.value,
                q.code_segment,
                q.offset,
                q.bit_offset,
                q.union_id,
                q.union_size_in_bit
            ],
        )?;
    }
    for r in &p.parameter_refs {
        conn.execute(
            "INSERT INTO parameter_ref (program_id, id, parameter_id, display_order, tag, text,
              value, access) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![
                p.id,
                r.id,
                r.parameter_id,
                r.display_order,
                r.tag,
                r.text,
                r.value,
                r.access
            ],
        )?;
    }
    for o in &p.com_objects {
        conn.execute(
            "INSERT INTO com_object (program_id, id, number, name, text, function_text,
              object_size, priority, read_flag, write_flag, transmit_flag, update_flag,
              communication_flag, read_on_init_flag)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
            params![
                p.id,
                o.id,
                o.number,
                o.name,
                o.text,
                o.function_text,
                o.object_size,
                o.priority,
                o.read_flag,
                o.write_flag,
                o.transmit_flag,
                o.update_flag,
                o.communication_flag,
                o.read_on_init_flag
            ],
        )?;
    }
    for r in &p.com_object_refs {
        conn.execute(
            "INSERT INTO com_object_ref (program_id, id, com_object_id, tag, text, function_text,
              object_size, priority, read_flag, write_flag, transmit_flag, update_flag,
              communication_flag, read_on_init_flag)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
            params![
                p.id,
                r.id,
                r.com_object_id,
                r.tag,
                r.text,
                r.function_text,
                r.object_size,
                r.priority,
                r.read_flag,
                r.write_flag,
                r.transmit_flag,
                r.update_flag,
                r.communication_flag,
                r.read_on_init_flag
            ],
        )?;
    }
    for n in &p.dynamic {
        conn.execute(
            "INSERT INTO dynamic_node (program_id, module_def_id, node_id, parent_id, position,
              kind, element_id, ref_id, test, is_default, text)
             VALUES (?1,'',?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![
                p.id,
                n.node_id,
                n.parent_id,
                n.position,
                n.kind,
                n.element_id,
                n.ref_id,
                n.test,
                n.is_default,
                n.text
            ],
        )?;
    }
    Ok(())
}
