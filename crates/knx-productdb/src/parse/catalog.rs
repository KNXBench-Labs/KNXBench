//! `Catalog.xml`: the recursive `CatalogSection` tree and its `CatalogItem`
//! leaves, which are the product picker's rows (RESEARCH §4).
//!
//! `CatalogItem.product_ref_id` / `hardware2program_ref_id` are the same
//! ids `hardware.rs` writes into `product` / `hardware2program` — the
//! catalog is a navigation view over that data, not a second copy of it.

use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;
use rusqlite::{params, Connection};

use super::report_unknown_attrs;
use crate::report::{UnknownCollector, UnknownConstruct};
use crate::xml::{attrs, local_name, Attrs};
use crate::ProductDbError;

const SECTION_ATTRS: &[&str] = &[
    "Id",
    "Name",
    "Number",
    "VisibleDescription",
    "DefaultLanguage",
    "NonRegRelevantDataVersion",
];

const ITEM_ATTRS: &[&str] = &[
    "Id",
    "Name",
    "Number",
    "VisibleDescription",
    "DefaultLanguage",
    "NonRegRelevantDataVersion",
    "ProductRefId",
    "Hardware2ProgramRefId",
];

pub fn ingest_catalog(
    conn: &Connection,
    source_sha256: &str,
    source_path: &str,
    bytes: &[u8],
) -> Result<Vec<UnknownConstruct>, ProductDbError> {
    let mut reader = Reader::from_reader(bytes);
    let mut buf = Vec::new();
    let mut unknown = UnknownCollector::default();
    let mut manufacturer_id = String::new();
    // The chain of currently-open `CatalogSection` ids, innermost last —
    // its top is the parent of whatever section or item comes next.
    let mut section_stack: Vec<String> = Vec::new();

    loop {
        buf.clear();
        let event = reader
            .read_event_into(&mut buf)
            .map_err(|e| ProductDbError::Xml {
                source_path: source_path.to_string(),
                cause: e.to_string(),
            })?;
        match event {
            Event::Eof => break,
            Event::End(e) => {
                if e.local_name().as_ref() == "CatalogSection" {
                    section_stack.pop();
                }
            }
            Event::Empty(e) => {
                handle_element(
                    conn,
                    &e,
                    source_sha256,
                    source_path,
                    &mut unknown,
                    &mut manufacturer_id,
                    &section_stack,
                )?;
            }
            Event::Start(e) => {
                let name = local_name(&e);
                handle_element(
                    conn,
                    &e,
                    source_sha256,
                    source_path,
                    &mut unknown,
                    &mut manufacturer_id,
                    &section_stack,
                )?;
                if name == "CatalogSection" {
                    let a = attrs(&e, source_path)?;
                    section_stack.push(a.get("Id").unwrap_or_default().to_string());
                }
            }
            _ => {}
        }
    }
    Ok(unknown.into_vec())
}

#[allow(clippy::too_many_arguments)]
fn handle_element(
    conn: &Connection,
    e: &BytesStart,
    source_sha256: &str,
    source_path: &str,
    unknown: &mut UnknownCollector,
    manufacturer_id: &mut String,
    section_stack: &[String],
) -> Result<(), ProductDbError> {
    let name = local_name(e);
    let a: Attrs = attrs(e, source_path)?;
    match name.as_str() {
        "Manufacturer" => {
            *manufacturer_id = a.get("RefId").unwrap_or_default().to_string();
            conn.execute(
                "INSERT OR IGNORE INTO manufacturer (id, name) VALUES (?1, NULL)",
                [manufacturer_id.as_str()],
            )?;
        }
        "CatalogSection" => {
            report_unknown_attrs(
                unknown,
                "/KNX/ManufacturerData/Manufacturer/Catalog/CatalogSection",
                &a,
                SECTION_ATTRS,
            );
            conn.execute(
                "INSERT OR IGNORE INTO catalog_section
                 (id, manufacturer_id, parent_id, name, number, visible_description,
                  default_language, source_sha256)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
                params![
                    a.get("Id"),
                    manufacturer_id.as_str(),
                    section_stack.last(),
                    a.get("Name"),
                    a.get("Number"),
                    a.get("VisibleDescription"),
                    a.get("DefaultLanguage"),
                    source_sha256,
                ],
            )?;
        }
        "CatalogItem" => {
            report_unknown_attrs(
                unknown,
                "/KNX/ManufacturerData/Manufacturer/Catalog/CatalogSection/CatalogItem",
                &a,
                ITEM_ATTRS,
            );
            conn.execute(
                "INSERT OR IGNORE INTO catalog_item
                 (id, manufacturer_id, section_id, name, number, visible_description,
                  product_ref_id, hardware2program_ref_id, default_language, source_sha256)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
                params![
                    a.get("Id"),
                    manufacturer_id.as_str(),
                    section_stack.last(),
                    a.get("Name"),
                    a.get("Number"),
                    a.get("VisibleDescription"),
                    a.get("ProductRefId"),
                    a.get("Hardware2ProgramRefId"),
                    a.get("DefaultLanguage"),
                    source_sha256,
                ],
            )?;
        }
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_and_migrate;

    const CATALOG: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11">
  <ManufacturerData>
    <Manufacturer RefId="M-006A">
      <Catalog>
        <CatalogSection Id="M-006A_CG-1" Name="Sensors" Number="1" DefaultLanguage="de-DE">
          <CatalogSection Id="M-006A_CG-1-1" Name="Presence detectors" Number="1.1"
                          DefaultLanguage="de-DE">
            <CatalogItem Id="M-006A_CI-1" Name="Präsenzmelder" Number="EM12102"
                         DefaultLanguage="de-DE"
                         ProductRefId="M-006A_H-EM12102-6-O0079_P-N000520"
                         Hardware2ProgramRefId="M-006A_H-EM12102-6-O0079_HP-0001-22-26C0-O0079" />
          </CatalogSection>
        </CatalogSection>
      </Catalog>
    </Manufacturer>
  </ManufacturerData>
</KNX>"#;

    fn db() -> (tempfile::TempDir, Connection) {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
        (dir, conn)
    }

    #[test]
    fn a_nested_section_is_stored_with_its_parent_id() {
        let (_dir, conn) = db();
        ingest_catalog(&conn, "sha-1", "M-006A/Catalog.xml", CATALOG.as_bytes()).unwrap();
        let parent: String = conn
            .query_row(
                "SELECT parent_id FROM catalog_section WHERE id = ?1",
                ["M-006A_CG-1-1"],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(parent, "M-006A_CG-1");
        let root_parent: Option<String> = conn
            .query_row(
                "SELECT parent_id FROM catalog_section WHERE id = ?1",
                ["M-006A_CG-1"],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(root_parent, None);
    }

    #[test]
    fn an_item_resolves_to_its_product_and_program_refs() {
        let (_dir, conn) = db();
        ingest_catalog(&conn, "sha-1", "M-006A/Catalog.xml", CATALOG.as_bytes()).unwrap();
        let (section_id, product_ref, program_ref): (String, String, String) = conn
            .query_row(
                "SELECT section_id, product_ref_id, hardware2program_ref_id
                 FROM catalog_item WHERE id = ?1",
                ["M-006A_CI-1"],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(section_id, "M-006A_CG-1-1");
        assert_eq!(product_ref, "M-006A_H-EM12102-6-O0079_P-N000520");
        assert_eq!(
            program_ref,
            "M-006A_H-EM12102-6-O0079_HP-0001-22-26C0-O0079"
        );
    }

    #[test]
    fn ingesting_the_same_catalog_twice_does_not_duplicate_rows() {
        let (_dir, conn) = db();
        ingest_catalog(&conn, "sha-1", "M-006A/Catalog.xml", CATALOG.as_bytes()).unwrap();
        ingest_catalog(&conn, "sha-1", "M-006A/Catalog.xml", CATALOG.as_bytes()).unwrap();
        let sections: i64 = conn
            .query_row("SELECT count(*) FROM catalog_section", [], |r| r.get(0))
            .unwrap();
        let items: i64 = conn
            .query_row("SELECT count(*) FROM catalog_item", [], |r| r.get(0))
            .unwrap();
        assert_eq!(sections, 2);
        assert_eq!(items, 1);
    }

    #[test]
    fn an_unknown_attribute_is_reported_and_the_rest_still_lands() {
        let (_dir, conn) = db();
        let xml = CATALOG.replace("Number=\"1\"", "Number=\"1\" FancyNewFlag=\"7\"");
        let unknown = ingest_catalog(&conn, "sha-2", "M-006A/Catalog.xml", xml.as_bytes()).unwrap();
        assert!(unknown.iter().any(|u| u.name == "FancyNewFlag"));
        let sections: i64 = conn
            .query_row("SELECT count(*) FROM catalog_section", [], |r| r.get(0))
            .unwrap();
        assert_eq!(sections, 2);
    }

    #[test]
    fn truncated_catalog_xml_is_an_error_naming_the_file() {
        let (_dir, conn) = db();
        let truncated = &CATALOG.as_bytes()[..CATALOG.len() / 2];
        let err = ingest_catalog(&conn, "sha-3", "M-006A/Catalog.xml", truncated).unwrap_err();
        assert!(format!("{err}").contains("M-006A/Catalog.xml"));
    }
}
