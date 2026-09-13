//! `Catalog.xml`: the recursive `CatalogSection` tree and its `CatalogItem`
//! leaves, which are the product picker's rows (RESEARCH §4).
//!
//! `CatalogItem.product_ref_id` / `hardware2program_ref_id` are the same
//! ids `hardware.rs` writes into `product` / `hardware2program` — the
//! catalog is a navigation view over that data, not a second copy of it.

use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;
use rusqlite::{params, Connection, OptionalExtension};

use super::report_unknown_attrs;
use crate::report::{IdConflict, UnknownCollector, UnknownConstruct};
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

#[derive(Debug)]
pub struct CatalogIngest {
    pub unknown: Vec<UnknownConstruct>,
    pub conflicts: Vec<IdConflict>,
}

pub fn ingest_catalog(
    conn: &Connection,
    source_sha256: &str,
    source_path: &str,
    bytes: &[u8],
) -> Result<CatalogIngest, ProductDbError> {
    let mut reader = Reader::from_reader(bytes);
    let mut buf = Vec::new();
    let mut unknown = UnknownCollector::default();
    let mut conflicts = Vec::new();
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
                    &mut conflicts,
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
                    &mut conflicts,
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
    Ok(CatalogIngest {
        unknown: unknown.into_vec(),
        conflicts,
    })
}

#[allow(clippy::too_many_arguments)]
fn handle_element(
    conn: &Connection,
    e: &BytesStart,
    source_sha256: &str,
    source_path: &str,
    unknown: &mut UnknownCollector,
    conflicts: &mut Vec<IdConflict>,
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
            if first_winner(
                conn,
                "catalog_section",
                a.get("Id"),
                source_sha256,
                conflicts,
            )? {
                conn.execute(
                    "INSERT INTO catalog_section
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
        }
        "CatalogItem" => {
            report_unknown_attrs(
                unknown,
                "/KNX/ManufacturerData/Manufacturer/Catalog/CatalogSection/CatalogItem",
                &a,
                ITEM_ATTRS,
            );
            if first_winner(conn, "catalog_item", a.get("Id"), source_sha256, conflicts)? {
                conn.execute(
                    "INSERT INTO catalog_item
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
        }
        _ => {}
    }
    Ok(())
}

fn first_winner(
    conn: &Connection,
    table: &str,
    id: Option<&str>,
    source_sha256: &str,
    conflicts: &mut Vec<IdConflict>,
) -> Result<bool, ProductDbError> {
    let id = id.unwrap_or_default();
    let existing: Option<String> = conn
        .query_row(
            &format!("SELECT source_sha256 FROM {table} WHERE id = ?1"),
            [id],
            |row| row.get(0),
        )
        .optional()?;
    match existing {
        None => Ok(true),
        Some(kept) => {
            if kept != source_sha256 {
                conflicts.push(IdConflict {
                    table: table.to_string(),
                    id: id.to_string(),
                    kept_sha256: kept,
                    other_sha256: source_sha256.to_string(),
                });
            }
            Ok(false)
        }
    }
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
        assert!(unknown.unknown.iter().any(|u| u.name == "FancyNewFlag"));
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

    /// KNOWN_LIMITATIONS.md §86, `catalog.rs`'s half of the same gap
    /// `hardware.rs::two_hardware_elements_sharing_an_id_in_one_file_
    /// conflict_silently` pins: `first_winner` compares the existing row's
    /// `source_sha256` to *this call's* `source_sha256`, one hash per
    /// whole file, so two `CatalogItem` elements sharing an `@Id` inside
    /// one `Catalog.xml` always compare equal and never reach the
    /// `IdConflict` branch. Pinned, not fixed — see the sibling test's
    /// doc comment for why a real fix is a schema change out of scope.
    #[test]
    fn two_catalog_items_sharing_an_id_in_one_file_conflict_silently() {
        let (_dir, conn) = db();
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11">
  <ManufacturerData>
    <Manufacturer RefId="M-006A">
      <Catalog>
        <CatalogSection Id="M-006A_CG-1" Name="Sensors" Number="1" DefaultLanguage="de-DE">
          <CatalogItem Id="CI-DUP" Name="First" Number="1"
                       DefaultLanguage="de-DE" ProductRefId="P-1"
                       Hardware2ProgramRefId="HP-1" />
          <CatalogItem Id="CI-DUP" Name="Second" Number="2"
                       DefaultLanguage="de-DE" ProductRefId="P-2"
                       Hardware2ProgramRefId="HP-2" />
        </CatalogSection>
      </Catalog>
    </Manufacturer>
  </ManufacturerData>
</KNX>"#;
        let ingest =
            ingest_catalog(&conn, "one-file-sha", "M-006A/Catalog.xml", xml.as_bytes()).unwrap();

        assert!(
            ingest.conflicts.is_empty(),
            "first_winner cannot see a same-file collision — this is the documented gap"
        );

        let rows: i64 = conn
            .query_row("SELECT count(*) FROM catalog_item", [], |r| r.get(0))
            .unwrap();
        assert_eq!(rows, 1, "the second CatalogItem row never lands at all");

        let (name, product_ref): (String, String) = conn
            .query_row(
                "SELECT name, product_ref_id FROM catalog_item WHERE id = 'CI-DUP'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(name, "First");
        assert_eq!(product_ref, "P-1");
    }
}
