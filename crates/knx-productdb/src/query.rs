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

const CATALOG_ITEM_COLUMNS: &str =
    "id, manufacturer_id, name, number, visible_description, product_ref_id, hardware2program_ref_id";

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

/// Every `com_object_ref.id` for `program_id`, in document/ingest order.
/// `ORDER BY rowid` rather than `ORDER BY id`: `com_object_ref` is not
/// declared `WITHOUT ROWID`, so `rowid` preserves insertion order, and the
/// ids themselves (`A-1_O-1_R-1`, `A-1_O-1_R-10`, `A-1_O-1_R-2`, …) do not
/// sort into that order lexically.
pub fn com_object_ref_ids(conn: &Connection, program_id: &str) -> Result<Vec<String>, ProductDbError> {
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
    use crate::parse::{catalog::ingest_catalog, hardware::ingest_hardware, program::ingest_program};

    const HARDWARE: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-006A">
<Hardware><Hardware Id="H-1" Name="X" SerialNumber="S" VersionNumber="1">
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
    fn com_object_ref_ids_returns_every_ref_in_document_order() {
        let (_dir, conn) = db();
        let ids = com_object_ref_ids(&conn, "A-1").unwrap();
        assert_eq!(ids, vec!["A-1_O-1_R-1".to_string(), "A-1_O-1_R-2".to_string()]);
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
}
