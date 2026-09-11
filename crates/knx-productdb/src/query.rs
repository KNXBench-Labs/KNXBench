//! The read side: resolving a device's program reference, and the merged
//! `ComObject` + `ComObjectRef` view the enrichment consumes.
//!
//! The merge keeps the layer that supplied each value. That is the whole
//! point of the override chain (DATA_MODEL §3): a value without its layer
//! cannot be written back correctly, so this view never returns one.

use rusqlite::{Connection, OptionalExtension};

use crate::ProductDbError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueLayer {
    Program,
    ProgramRef,
}

/// Picks the `ComObjectRef` value when it has one, otherwise the
/// `ComObject`'s, reporting which layer won.
fn pick(program: Option<String>, program_ref: Option<String>) -> (Option<String>, ValueLayer) {
    match program_ref {
        Some(v) => (Some(v), ValueLayer::ProgramRef),
        None => (program, ValueLayer::Program),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComObjectView {
    pub number: Option<i64>,
    pub text: Option<String>,
    pub text_layer: ValueLayer,
    pub function_text: Option<String>,
    pub function_text_layer: ValueLayer,
    pub visible_description: Option<String>,
    pub description_layer: ValueLayer,
    pub object_size: Option<String>,
    pub object_size_layer: ValueLayer,
    pub priority: Option<String>,
    pub dpt_list: Option<String>,
    pub dpt_layer: ValueLayer,
    pub read: Option<String>,
    pub read_layer: ValueLayer,
    pub write: Option<String>,
    pub write_layer: ValueLayer,
    pub transmit: Option<String>,
    pub transmit_layer: ValueLayer,
    pub update: Option<String>,
    pub update_layer: ValueLayer,
    pub communication: Option<String>,
    pub communication_layer: ValueLayer,
}

pub fn resolve_program(
    conn: &Connection,
    hardware2program_id: &str,
) -> Result<Option<String>, ProductDbError> {
    let id: Option<String> = conn
        .query_row(
            "SELECT ap.id
             FROM hardware2program h2p
             JOIN application_program ap ON ap.id = h2p.application_program_ref
             WHERE h2p.id = ?1",
            [hardware2program_id],
            |r| r.get(0),
        )
        .optional()?;
    Ok(id)
}

struct RawRow {
    number: Option<i64>,
    co_text: Option<String>,
    co_function_text: Option<String>,
    co_visible_description: Option<String>,
    co_object_size: Option<String>,
    co_priority: Option<String>,
    co_dpt_list: Option<String>,
    co_read: Option<String>,
    co_write: Option<String>,
    co_transmit: Option<String>,
    co_update: Option<String>,
    co_communication: Option<String>,
    cor_text: Option<String>,
    cor_function_text: Option<String>,
    cor_visible_description: Option<String>,
    cor_object_size: Option<String>,
    cor_priority: Option<String>,
    cor_dpt_list: Option<String>,
    cor_read: Option<String>,
    cor_write: Option<String>,
    cor_transmit: Option<String>,
    cor_update: Option<String>,
    cor_communication: Option<String>,
}

/// Joins one `ComObjectRef` row to the `ComObject` it refers to, within the
/// same program, and folds every attribute through `pick`.
pub fn com_object_view(
    conn: &Connection,
    program_id: &str,
    com_object_ref_id: &str,
) -> Result<Option<ComObjectView>, ProductDbError> {
    let raw: Option<RawRow> = conn
        .query_row(
            "SELECT co.number,
                    co.text, co.function_text, co.visible_description, co.object_size,
                    co.priority, co.dpt_list, co.read_flag, co.write_flag,
                    co.transmit_flag, co.update_flag, co.communication_flag,
                    cor.text, cor.function_text, cor.visible_description, cor.object_size,
                    cor.priority, cor.dpt_list, cor.read_flag, cor.write_flag,
                    cor.transmit_flag, cor.update_flag, cor.communication_flag
             FROM com_object_ref cor
             JOIN com_object co
               ON co.program_id = cor.program_id AND co.id = cor.com_object_id
             WHERE cor.program_id = ?1 AND cor.id = ?2",
            [program_id, com_object_ref_id],
            |r| {
                Ok(RawRow {
                    number: r.get(0)?,
                    co_text: r.get(1)?,
                    co_function_text: r.get(2)?,
                    co_visible_description: r.get(3)?,
                    co_object_size: r.get(4)?,
                    co_priority: r.get(5)?,
                    co_dpt_list: r.get(6)?,
                    co_read: r.get(7)?,
                    co_write: r.get(8)?,
                    co_transmit: r.get(9)?,
                    co_update: r.get(10)?,
                    co_communication: r.get(11)?,
                    cor_text: r.get(12)?,
                    cor_function_text: r.get(13)?,
                    cor_visible_description: r.get(14)?,
                    cor_object_size: r.get(15)?,
                    cor_priority: r.get(16)?,
                    cor_dpt_list: r.get(17)?,
                    cor_read: r.get(18)?,
                    cor_write: r.get(19)?,
                    cor_transmit: r.get(20)?,
                    cor_update: r.get(21)?,
                    cor_communication: r.get(22)?,
                })
            },
        )
        .optional()?;

    let Some(raw) = raw else {
        return Ok(None);
    };

    let (text, text_layer) = pick(raw.co_text, raw.cor_text);
    let (function_text, function_text_layer) = pick(raw.co_function_text, raw.cor_function_text);
    let (visible_description, description_layer) =
        pick(raw.co_visible_description, raw.cor_visible_description);
    let (object_size, object_size_layer) = pick(raw.co_object_size, raw.cor_object_size);
    let (dpt_list, dpt_layer) = pick(raw.co_dpt_list, raw.cor_dpt_list);
    let (read, read_layer) = pick(raw.co_read, raw.cor_read);
    let (write, write_layer) = pick(raw.co_write, raw.cor_write);
    let (transmit, transmit_layer) = pick(raw.co_transmit, raw.cor_transmit);
    let (update, update_layer) = pick(raw.co_update, raw.cor_update);
    let (communication, communication_layer) = pick(raw.co_communication, raw.cor_communication);
    let priority = raw.cor_priority.or(raw.co_priority);

    Ok(Some(ComObjectView {
        number: raw.number,
        text,
        text_layer,
        function_text,
        function_text_layer,
        visible_description,
        description_layer,
        object_size,
        object_size_layer,
        priority,
        dpt_list,
        dpt_layer,
        read,
        read_layer,
        write,
        write_layer,
        transmit,
        transmit_layer,
        update,
        update_layer,
        communication,
        communication_layer,
    }))
}

/// A single `Parameter`, resolved through its own program's `ParameterRef`
/// and `ParameterType` (design D22). Follows `ComObjectView`'s `pick()`/
/// `ValueLayer` idiom for the one field a `ParameterRef` can override —
/// there is no three-layer override chain here, `ComObjectInstanceRef`'s
/// wider one does not apply to parameters (ARCHITECTURE.md §5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParameterView {
    /// `parameter_ref.id`, the `ValueMap`/`ets_id` key (D21).
    pub id: String,
    pub display_order: i64,
    pub tag: Option<String>,
    /// `parameter.name`.
    pub name: Option<String>,
    /// `pick(parameter.text, parameter_ref.text)`.
    pub text: Option<String>,
    pub text_layer: ValueLayer,
    /// `parameter_type.kind`, verbatim: one of `Restriction`, `Number`,
    /// `Text`, `None`, `Float`, `IPAddress`, `Picture`, `Raw`, `Other`.
    pub kind: String,
    /// `parameter.access`, verbatim — display only, D24 does not gate on it.
    pub access: Option<String>,
    pub min_inclusive: Option<String>,
    pub max_inclusive: Option<String>,
    /// `(value, text)`, only non-empty when `kind == "Restriction"` — the
    /// other seven kinds never have rows in `parameter_type_enum`.
    pub enum_options: Vec<(String, Option<String>)>,
}

struct ParameterRawRow {
    id: String,
    display_order: i64,
    tag: Option<String>,
    name: Option<String>,
    p_text: Option<String>,
    pr_text: Option<String>,
    kind: String,
    access: Option<String>,
    min_inclusive: Option<String>,
    max_inclusive: Option<String>,
    parameter_type_id: String,
}

/// Every `ParameterView` a program declares, in `parameter_ref.
/// display_order`. One query, not one per field — a single `ModuleDef` can
/// own on the order of hundreds of these (RESEARCH.md §4.4 Q3), so N calls
/// is the wrong shape, exactly as `com_object_view`'s own doc comment
/// already reasons for communication objects.
pub fn parameter_views(
    conn: &Connection,
    program_id: &str,
) -> Result<Vec<ParameterView>, ProductDbError> {
    // `pr.display_order` is `COALESCE`d to 0: real-world packages exist
    // where `ParameterRef/@DisplayOrder` is simply absent (observed on the
    // full corpus, not a hypothetical — every one of one MDT program's 543
    // `ParameterRef`s omits it), and `ParameterView.display_order` is `i64`
    // per D22, not `Option<i64>`. `ORDER BY` still sorts on the raw
    // (possibly-NULL) column so ties among absent values do not get a
    // fabricated secondary order on top of what the plan asks for.
    let mut stmt = conn.prepare(
        "SELECT pr.id, COALESCE(pr.display_order, 0), pr.tag,
                p.name, p.text, pr.text,
                pt.kind, p.access, pt.min_inclusive, pt.max_inclusive, pt.id
         FROM parameter_ref pr
         JOIN parameter p ON p.program_id = pr.program_id AND p.id = pr.parameter_id
         JOIN parameter_type pt ON pt.program_id = p.program_id AND pt.id = p.parameter_type_id
         WHERE pr.program_id = ?1
         ORDER BY pr.display_order",
    )?;
    let raw_rows: Vec<ParameterRawRow> = stmt
        .query_map([program_id], |r| {
            Ok(ParameterRawRow {
                id: r.get(0)?,
                display_order: r.get(1)?,
                tag: r.get(2)?,
                name: r.get(3)?,
                p_text: r.get(4)?,
                pr_text: r.get(5)?,
                kind: r.get(6)?,
                access: r.get(7)?,
                min_inclusive: r.get(8)?,
                max_inclusive: r.get(9)?,
                parameter_type_id: r.get(10)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut views = Vec::with_capacity(raw_rows.len());
    for raw in raw_rows {
        let (text, text_layer) = pick(raw.p_text, raw.pr_text);
        // Only `Restriction` kinds ever have rows in `parameter_type_enum`
        // (the other seven kinds have no enumeration concept at all) — the
        // kind check keeps this a second query for the fraction of rows
        // that need it, not a blind per-row lookup.
        let enum_options = if raw.kind == "Restriction" {
            parameter_type_enum_options(conn, program_id, &raw.parameter_type_id)?
        } else {
            Vec::new()
        };
        views.push(ParameterView {
            id: raw.id,
            display_order: raw.display_order,
            tag: raw.tag,
            name: raw.name,
            text,
            text_layer,
            kind: raw.kind,
            access: raw.access,
            min_inclusive: raw.min_inclusive,
            max_inclusive: raw.max_inclusive,
            enum_options,
        });
    }
    Ok(views)
}

fn parameter_type_enum_options(
    conn: &Connection,
    program_id: &str,
    parameter_type_id: &str,
) -> Result<Vec<(String, Option<String>)>, ProductDbError> {
    let mut stmt = conn.prepare(
        "SELECT value, text FROM parameter_type_enum
         WHERE program_id = ?1 AND parameter_type_id = ?2
         ORDER BY display_order",
    )?;
    let rows = stmt
        .query_map([program_id, parameter_type_id], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// The bare `parameter_ref.id` set for a program — D21's stale-value diff
/// (a stored value whose id is no longer a declared `ParameterRef`) needs
/// only this, not the full `ParameterView`, so it is kept as its own thin
/// query rather than mapped off `parameter_views`'s output.
pub fn parameter_ref_ids(
    conn: &Connection,
    program_id: &str,
) -> Result<std::collections::HashSet<String>, ProductDbError> {
    let mut stmt = conn.prepare("SELECT id FROM parameter_ref WHERE program_id = ?1")?;
    let rows = stmt
        .query_map([program_id], |r| r.get(0))?
        .collect::<Result<std::collections::HashSet<_>, _>>()?;
    Ok(rows)
}

/// `(id, name)` for every manufacturer, ordered by id — the listing
/// `knx products list` prints.
pub fn manufacturers(conn: &Connection) -> Result<Vec<(String, Option<String>)>, ProductDbError> {
    let mut stmt = conn.prepare("SELECT id, name FROM manufacturer ORDER BY id")?;
    let rows = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramRow {
    pub id: String,
    pub manufacturer_id: String,
    pub name: Option<String>,
    pub application_number: Option<String>,
    pub application_version: Option<String>,
    pub mask_version: Option<String>,
}

/// Every application program, optionally narrowed to one manufacturer,
/// ordered by id — the listing `knx products list` prints.
pub fn programs(
    conn: &Connection,
    manufacturer: Option<&str>,
) -> Result<Vec<ProgramRow>, ProductDbError> {
    let sql =
        "SELECT id, manufacturer_id, name, application_number, application_version, mask_version
               FROM application_program
               WHERE ?1 IS NULL OR manufacturer_id = ?1
               ORDER BY id";
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt
        .query_map([manufacturer], |r| {
            Ok(ProgramRow {
                id: r.get(0)?,
                manufacturer_id: r.get(1)?,
                name: r.get(2)?,
                application_number: r.get(3)?,
                application_version: r.get(4)?,
                mask_version: r.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogItemRow {
    pub id: String,
    pub manufacturer_id: String,
    pub name: Option<String>,
    pub number: Option<String>,
    pub visible_description: Option<String>,
    pub product_ref_id: Option<String>,
    pub hardware2program_ref_id: Option<String>,
}

fn row_to_catalog_item(r: &rusqlite::Row) -> rusqlite::Result<CatalogItemRow> {
    Ok(CatalogItemRow {
        id: r.get(0)?,
        manufacturer_id: r.get(1)?,
        name: r.get(2)?,
        number: r.get(3)?,
        visible_description: r.get(4)?,
        product_ref_id: r.get(5)?,
        hardware2program_ref_id: r.get(6)?,
    })
}

const CATALOG_ITEM_COLUMNS: &str = "id, manufacturer_id, name, number, visible_description, product_ref_id, hardware2program_ref_id";

/// Every `catalog_item` row, optionally narrowed to one manufacturer and/or
/// a case-insensitive substring match on `name`/`number` — backs the future
/// catalog browser (T2). Device creation (`apps/knx-server`) goes straight
/// to `catalog_item` by id instead: nothing yet picks an id through this
/// listing.
pub fn catalog_items(
    conn: &Connection,
    manufacturer: Option<&str>,
    search: Option<&str>,
) -> Result<Vec<CatalogItemRow>, ProductDbError> {
    let sql = format!(
        "SELECT {CATALOG_ITEM_COLUMNS}
         FROM catalog_item
         WHERE (?1 IS NULL OR manufacturer_id = ?1)
           AND (?2 IS NULL
                OR LOWER(name) LIKE '%' || LOWER(?2) || '%'
                OR LOWER(number) LIKE '%' || LOWER(?2) || '%')
         ORDER BY manufacturer_id, name"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt
        .query_map([manufacturer, search], row_to_catalog_item)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// The single-row lookup `apps/knx-server`'s device creation uses.
pub fn catalog_item(conn: &Connection, id: &str) -> Result<Option<CatalogItemRow>, ProductDbError> {
    let sql = format!("SELECT {CATALOG_ITEM_COLUMNS} FROM catalog_item WHERE id = ?1");
    conn.query_row(&sql, [id], row_to_catalog_item)
        .optional()
        .map_err(Into::into)
}

/// The evidence a catalog item provides for device creation.  A catalog row
/// alone is insufficient: its product, hardware, hardware-to-program and
/// application-program references must form one consistent chain before the
/// application is allowed to create a configured device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogItemProgram {
    Program {
        product_ref_id: String,
        hardware2program_ref_id: String,
        program_id: String,
    },
    /// The product's hardware explicitly says it has no application program.
    /// This is the only database-evidenced empty-program case that remains
    /// creatable; an absent or broken relation is not silently treated alike.
    Programless { product_ref_id: String },
}

#[derive(Debug)]
pub enum CatalogItemRelationError {
    ProductMissing {
        product_ref_id: String,
    },
    HardwareMissing {
        product_ref_id: String,
        hardware_id: String,
    },
    Hardware2ProgramMissing {
        hardware2program_ref_id: String,
    },
    Hardware2ProgramMismatch {
        product_ref_id: String,
        hardware2program_ref_id: String,
    },
    ProgramMissing {
        hardware2program_ref_id: String,
        program_ref_id: Option<String>,
    },
    Database(ProductDbError),
}

impl From<rusqlite::Error> for CatalogItemRelationError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Database(ProductDbError::from(error))
    }
}

impl std::fmt::Display for CatalogItemRelationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProductMissing { product_ref_id } => {
                write!(f, "catalog product relation is missing: {product_ref_id}")
            }
            Self::HardwareMissing { hardware_id, .. } => {
                write!(
                    f,
                    "catalog product hardware relation is missing: {hardware_id}"
                )
            }
            Self::Hardware2ProgramMissing {
                hardware2program_ref_id,
            } => write!(
                f,
                "catalog hardware-to-program relation is missing: {hardware2program_ref_id}"
            ),
            Self::Hardware2ProgramMismatch {
                product_ref_id,
                hardware2program_ref_id,
            } => write!(
                f,
                "catalog product {product_ref_id} and hardware-to-program {hardware2program_ref_id} do not share hardware"
            ),
            Self::ProgramMissing {
                hardware2program_ref_id,
                program_ref_id,
            } => write!(
                f,
                "catalog hardware-to-program {hardware2program_ref_id} has no installed application program{}",
                program_ref_id
                    .as_deref()
                    .map(|id| format!(": {id}"))
                    .unwrap_or_default()
            ),
            Self::Database(error) => write!(f, "product database query failed: {error}"),
        }
    }
}

/// Resolves a catalog item only when all persisted relations support creating
/// a device.  Kept in the product database because these are normalized
/// manufacturer-data facts, not server/UI policy.
pub fn resolve_catalog_item_program(
    conn: &Connection,
    item: &CatalogItemRow,
) -> Result<CatalogItemProgram, CatalogItemRelationError> {
    let product_ref_id = item
        .product_ref_id
        .as_deref()
        .filter(|id| !id.is_empty())
        .ok_or_else(|| CatalogItemRelationError::ProductMissing {
            product_ref_id: "(absent)".into(),
        })?;
    let product: Option<(String, Option<i64>)> = conn
        .query_row(
            "SELECT p.hardware_id, h.has_application_program
             FROM product p LEFT JOIN hardware h ON h.id = p.hardware_id
             WHERE p.id = ?1",
            [product_ref_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let Some((hardware_id, has_application_program)) = product else {
        return Err(CatalogItemRelationError::ProductMissing {
            product_ref_id: product_ref_id.into(),
        });
    };

    let hardware_exists: Option<i64> = conn
        .query_row(
            "SELECT 1 FROM hardware WHERE id = ?1",
            [&hardware_id],
            |row| row.get(0),
        )
        .optional()?;
    if hardware_exists.is_none() {
        return Err(CatalogItemRelationError::HardwareMissing {
            product_ref_id: product_ref_id.into(),
            hardware_id,
        });
    }

    let Some(hardware2program_ref_id) = item
        .hardware2program_ref_id
        .as_deref()
        .filter(|id| !id.is_empty())
    else {
        return if has_application_program == Some(0) {
            Ok(CatalogItemProgram::Programless {
                product_ref_id: product_ref_id.into(),
            })
        } else {
            Err(CatalogItemRelationError::Hardware2ProgramMissing {
                hardware2program_ref_id: "(absent)".into(),
            })
        };
    };
    let h2p: Option<(String, Option<String>)> = conn
        .query_row(
            "SELECT hardware_id, application_program_ref FROM hardware2program WHERE id = ?1",
            [hardware2program_ref_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let Some((h2p_hardware_id, program_ref_id)) = h2p else {
        return Err(CatalogItemRelationError::Hardware2ProgramMissing {
            hardware2program_ref_id: hardware2program_ref_id.into(),
        });
    };
    if h2p_hardware_id != hardware_id {
        return Err(CatalogItemRelationError::Hardware2ProgramMismatch {
            product_ref_id: product_ref_id.into(),
            hardware2program_ref_id: hardware2program_ref_id.into(),
        });
    }
    let Some(program_id) = program_ref_id.as_deref() else {
        return Err(CatalogItemRelationError::ProgramMissing {
            hardware2program_ref_id: hardware2program_ref_id.into(),
            program_ref_id,
        });
    };
    let exists: Option<i64> = conn
        .query_row(
            "SELECT 1 FROM application_program WHERE id = ?1",
            [program_id],
            |row| row.get(0),
        )
        .optional()?;
    if exists.is_none() {
        return Err(CatalogItemRelationError::ProgramMissing {
            hardware2program_ref_id: hardware2program_ref_id.into(),
            program_ref_id: Some(program_id.into()),
        });
    }
    Ok(CatalogItemProgram::Program {
        product_ref_id: product_ref_id.into(),
        hardware2program_ref_id: hardware2program_ref_id.into(),
        program_id: program_id.into(),
    })
}

/// Every `com_object_ref.id` for `program_id`, in document/ingest order.
/// `ORDER BY rowid` rather than `ORDER BY id`: `com_object_ref` is not
/// declared `WITHOUT ROWID`, so `rowid` preserves insertion order, and the
/// ids themselves (`A-1_O-1_R-1`, `A-1_O-1_R-10`, `A-1_O-1_R-2`, …) do not
/// sort into that order lexically.
pub fn com_object_ref_ids(
    conn: &Connection,
    program_id: &str,
) -> Result<Vec<String>, ProductDbError> {
    let mut stmt =
        conn.prepare("SELECT id FROM com_object_ref WHERE program_id = ?1 ORDER BY rowid")?;
    let rows = stmt
        .query_map([program_id], |r| r.get(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_and_migrate;
    use crate::parse::{
        catalog::ingest_catalog, hardware::ingest_hardware, program::ingest_program,
    };

    const HARDWARE: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-006A">
<Hardware><Hardware Id="H-1" Name="X" SerialNumber="S" VersionNumber="1">
<Products><Product Id="M-006A_H-1_P-1" /></Products>
<Hardware2Programs><Hardware2Program Id="H-1_HP-1" MediumTypes="MT-0">
<ApplicationProgramRef RefId="A-1" /></Hardware2Program></Hardware2Programs>
</Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;

    const PROGRAM: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-006A">
<ApplicationPrograms><ApplicationProgram Id="A-1" Name="P" ApplicationNumber="1"
  ApplicationVersion="22" MaskVersion="MV-0701"><Static>
<ComObjectTable>
  <ComObject Id="A-1_O-1" Number="1" Text="Schalten" ObjectSize="1 Bit"
             DatapointType="DPST-1-1" WriteFlag="Enabled" ReadFlag="Disabled" />
</ComObjectTable>
<ComObjectRefs>
  <ComObjectRef Id="A-1_O-1_R-1" RefId="A-1_O-1" />
  <ComObjectRef Id="A-1_O-1_R-2" RefId="A-1_O-1" Text="Dimmen" DatapointType="DPST-3-7" />
</ComObjectRefs>
</Static></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

    fn db() -> (tempfile::TempDir, Connection) {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
        ingest_hardware(&conn, "sha-h", "M-006A/Hardware.xml", HARDWARE.as_bytes()).unwrap();
        ingest_program(&conn, "sha-p", "M-006A/A.xml", PROGRAM.as_bytes()).unwrap();
        (dir, conn)
    }

    const CATALOG: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11">
  <ManufacturerData>
    <Manufacturer RefId="M-006A">
      <Catalog>
        <CatalogSection Id="M-006A_CG-1" Name="Actuators" Number="1" DefaultLanguage="de-DE">
          <CatalogItem Id="M-006A_CI-1" Name="Schaltaktor" Number="EM12102"
                       DefaultLanguage="de-DE"
                       ProductRefId="M-006A_H-1_P-1"
                       Hardware2ProgramRefId="H-1_HP-1" />
        </CatalogSection>
      </Catalog>
    </Manufacturer>
  </ManufacturerData>
</KNX>"#;

    #[test]
    fn catalog_items_lists_and_filters_by_manufacturer_and_search() {
        let (_dir, conn) = db();
        ingest_catalog(&conn, "sha-c", "M-006A/Catalog.xml", CATALOG.as_bytes()).unwrap();

        let all = catalog_items(&conn, None, None).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, "M-006A_CI-1");

        assert_eq!(catalog_items(&conn, Some("M-006A"), None).unwrap().len(), 1);
        assert_eq!(catalog_items(&conn, Some("M-999X"), None).unwrap().len(), 0);
        assert_eq!(
            catalog_items(&conn, None, Some("schalt")).unwrap().len(),
            1,
            "search is case-insensitive"
        );
        assert_eq!(
            catalog_items(&conn, None, Some("EM12102")).unwrap().len(),
            1,
            "search also matches on number"
        );
        assert_eq!(catalog_items(&conn, None, Some("nope")).unwrap().len(), 0);
    }

    #[test]
    fn catalog_item_looks_up_a_single_row_by_id() {
        let (_dir, conn) = db();
        ingest_catalog(&conn, "sha-c", "M-006A/Catalog.xml", CATALOG.as_bytes()).unwrap();

        let item = catalog_item(&conn, "M-006A_CI-1").unwrap().unwrap();
        assert_eq!(item.manufacturer_id, "M-006A");
        assert_eq!(item.hardware2program_ref_id.as_deref(), Some("H-1_HP-1"));
        assert!(catalog_item(&conn, "nope").unwrap().is_none());
    }

    #[test]
    fn catalog_item_program_resolves_a_complete_relation_chain() {
        let (_dir, conn) = db();
        ingest_catalog(&conn, "sha-c", "M-006A/Catalog.xml", CATALOG.as_bytes()).unwrap();
        let item = catalog_item(&conn, "M-006A_CI-1").unwrap().unwrap();

        assert!(matches!(
            resolve_catalog_item_program(&conn, &item).unwrap(),
            CatalogItemProgram::Program {
                product_ref_id,
                hardware2program_ref_id,
                program_id,
            } if product_ref_id == "M-006A_H-1_P-1"
                && hardware2program_ref_id == "H-1_HP-1"
                && program_id == "A-1"
        ));
    }

    #[test]
    fn catalog_item_program_reports_a_missing_relation() {
        let (_dir, conn) = db();
        ingest_catalog(&conn, "sha-c", "M-006A/Catalog.xml", CATALOG.as_bytes()).unwrap();
        conn.execute("DELETE FROM hardware2program WHERE id = 'H-1_HP-1'", [])
            .unwrap();
        let item = catalog_item(&conn, "M-006A_CI-1").unwrap().unwrap();

        assert!(matches!(
            resolve_catalog_item_program(&conn, &item),
            Err(CatalogItemRelationError::Hardware2ProgramMissing { .. })
        ));
    }

    #[test]
    fn catalog_item_program_preserves_database_errors() {
        let (_dir, conn) = db();
        ingest_catalog(&conn, "sha-c", "M-006A/Catalog.xml", CATALOG.as_bytes()).unwrap();
        let item = catalog_item(&conn, "M-006A_CI-1").unwrap().unwrap();
        conn.execute_batch("DROP TABLE product").unwrap();

        assert!(matches!(
            resolve_catalog_item_program(&conn, &item),
            Err(CatalogItemRelationError::Database(ProductDbError::Sqlite(
                _
            )))
        ));
    }

    #[test]
    fn com_object_ref_ids_returns_every_ref_in_document_order() {
        let (_dir, conn) = db();
        let ids = com_object_ref_ids(&conn, "A-1").unwrap();
        assert_eq!(
            ids,
            vec!["A-1_O-1_R-1".to_string(), "A-1_O-1_R-2".to_string()]
        );
    }

    #[test]
    fn a_hardware2program_id_resolves_to_its_application_program() {
        let (_dir, conn) = db();
        assert_eq!(
            resolve_program(&conn, "H-1_HP-1").unwrap(),
            Some("A-1".into())
        );
        assert_eq!(resolve_program(&conn, "H-9_HP-9").unwrap(), None);
    }

    #[test]
    fn a_ref_without_overrides_shows_the_program_layer_values() {
        let (_dir, conn) = db();
        let v = com_object_view(&conn, "A-1", "A-1_O-1_R-1")
            .unwrap()
            .unwrap();
        assert_eq!(v.text.as_deref(), Some("Schalten"));
        assert_eq!(v.text_layer, ValueLayer::Program);
        assert_eq!(v.dpt_list.as_deref(), Some("DPST-1-1"));
        assert_eq!(v.dpt_layer, ValueLayer::Program);
        assert_eq!(v.number, Some(1));
        assert_eq!(v.write.as_deref(), Some("Enabled"));
    }

    #[test]
    fn a_ref_with_overrides_shows_the_program_ref_layer_for_those_values_only() {
        let (_dir, conn) = db();
        let v = com_object_view(&conn, "A-1", "A-1_O-1_R-2")
            .unwrap()
            .unwrap();
        assert_eq!(v.text.as_deref(), Some("Dimmen"));
        assert_eq!(v.text_layer, ValueLayer::ProgramRef);
        assert_eq!(v.dpt_list.as_deref(), Some("DPST-3-7"));
        assert_eq!(v.dpt_layer, ValueLayer::ProgramRef);
        // Not overridden: still the program's own value and layer.
        assert_eq!(v.object_size.as_deref(), Some("1 Bit"));
        assert_eq!(v.write.as_deref(), Some("Enabled"));
    }

    #[test]
    fn an_unknown_ref_id_is_none_not_an_error() {
        let (_dir, conn) = db();
        assert!(com_object_view(&conn, "A-1", "A-1_O-9_R-9")
            .unwrap()
            .is_none());
    }

    #[test]
    fn listings_return_what_the_cli_prints() {
        let (_dir, conn) = db();
        assert_eq!(
            manufacturers(&conn).unwrap(),
            vec![("M-006A".to_string(), None)]
        );
        let programs = programs(&conn, Some("M-006A")).unwrap();
        assert_eq!(programs.len(), 1);
        assert_eq!(programs[0].id, "A-1");
        assert_eq!(programs[0].application_version.as_deref(), Some("22"));
    }

    // -----------------------------------------------------------------
    // parameter_views / parameter_ref_ids (T18, design D22).
    // -----------------------------------------------------------------

    /// Three `ParameterRef`s of kind `Number`, `Restriction` and `Text`
    /// respectively. `DisplayOrder` deliberately does not match id order
    /// (`PR-2` first, then `PR-3`, then `PR-1`), so a test asserting
    /// `parameter_views`'s output order actually proves it sorts by
    /// `display_order` and not by id or insertion order.
    const PARAMETER_PROGRAM: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-006A">
<ApplicationPrograms><ApplicationProgram Id="A-2" Name="P" ApplicationNumber="2"
  ApplicationVersion="22" MaskVersion="MV-0701"><Static>
<ParameterTypes>
  <ParameterType Id="PT-Num" Name="num"><TypeNumber maxInclusive="255" minInclusive="0" SizeInBit="8" Type="unsignedInt" /></ParameterType>
  <ParameterType Id="PT-Enum" Name="enum"><TypeRestriction Base="Value" SizeInBit="8">
    <Enumeration Id="PT-Enum_EN-0" Text="Off" Value="0" DisplayOrder="0" />
    <Enumeration Id="PT-Enum_EN-1" Text="On" Value="1" DisplayOrder="1" />
  </TypeRestriction></ParameterType>
  <ParameterType Id="PT-Text" Name="text"><TypeText /></ParameterType>
</ParameterTypes>
<Parameters>
  <Parameter Id="P-1" Name="Delay" Text="Delay" ParameterType="PT-Num" Access="ReadWrite" Value="5" />
  <Parameter Id="P-2" Name="Mode" Text="Mode" ParameterType="PT-Enum" Access="ReadWrite" Value="0" />
  <Parameter Id="P-3" Name="Label" Text="Label" ParameterType="PT-Text" Access="ReadWrite" Value="hi" />
</Parameters>
<ParameterRefs>
  <ParameterRef Id="PR-1" RefId="P-1" DisplayOrder="30" Tag="1" />
  <ParameterRef Id="PR-2" RefId="P-2" DisplayOrder="10" Tag="2" Text="On (override)" />
  <ParameterRef Id="PR-3" RefId="P-3" DisplayOrder="20" Tag="3" />
</ParameterRefs>
</Static></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

    fn parameter_db() -> (tempfile::TempDir, Connection) {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
        ingest_program(
            &conn,
            "sha-p2",
            "M-006A/A2.xml",
            PARAMETER_PROGRAM.as_bytes(),
        )
        .unwrap();
        (dir, conn)
    }

    #[test]
    fn parameter_views_returns_three_views_in_display_order_with_enum_options_only_on_the_restriction(
    ) {
        let (_dir, conn) = parameter_db();
        let views = parameter_views(&conn, "A-2").unwrap();
        assert_eq!(
            views.iter().map(|v| v.id.as_str()).collect::<Vec<_>>(),
            vec!["PR-2", "PR-3", "PR-1"],
            "sorted by display_order (10, 20, 30), not by id"
        );

        let pr2 = &views[0];
        assert_eq!(pr2.kind, "Restriction");
        assert_eq!(
            pr2.enum_options,
            vec![
                ("0".to_string(), Some("Off".to_string())),
                ("1".to_string(), Some("On".to_string())),
            ]
        );

        let pr3 = &views[1];
        assert_eq!(pr3.kind, "Text");
        assert!(pr3.enum_options.is_empty());

        let pr1 = &views[2];
        assert_eq!(pr1.kind, "Number");
        assert!(pr1.enum_options.is_empty());
        assert_eq!(pr1.min_inclusive.as_deref(), Some("0"));
        assert_eq!(pr1.max_inclusive.as_deref(), Some("255"));
    }

    #[test]
    fn parameter_ref_ids_returns_the_declared_id_set() {
        let (_dir, conn) = parameter_db();
        let ids = parameter_ref_ids(&conn, "A-2").unwrap();
        assert_eq!(
            ids,
            std::collections::HashSet::from([
                "PR-1".to_string(),
                "PR-2".to_string(),
                "PR-3".to_string(),
            ])
        );
    }

    /// Mirrors `com_object_view`'s own `pick()` tests above: a `ParameterRef`
    /// with no `Text` of its own falls back to its `Parameter`'s, reporting
    /// `ValueLayer::Program`; one with an override reports `ProgramRef`.
    #[test]
    fn parameter_views_reports_pick_layer_the_same_way_com_object_view_does() {
        let (_dir, conn) = parameter_db();
        let views = parameter_views(&conn, "A-2").unwrap();
        let pr1 = views.iter().find(|v| v.id == "PR-1").unwrap();
        assert_eq!(pr1.text.as_deref(), Some("Delay"));
        assert_eq!(pr1.text_layer, ValueLayer::Program);

        let pr2 = views.iter().find(|v| v.id == "PR-2").unwrap();
        assert_eq!(pr2.text.as_deref(), Some("On (override)"));
        assert_eq!(pr2.text_layer, ValueLayer::ProgramRef);
    }
}
