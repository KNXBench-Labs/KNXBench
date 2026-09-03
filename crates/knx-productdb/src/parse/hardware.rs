//! `Hardware.xml`: `Hardware` → `Products`/`Product` and
//! `Hardware2Programs`/`Hardware2Program` (RESEARCH §4).
//!
//! `DeviceInstance.product_ref` resolves into `product`, and
//! `DeviceInstance.program_ref` into `hardware2program`, whose
//! `application_program_ref` is the bridge to the application program.

use quick_xml::events::Event;
use quick_xml::Reader;
use rusqlite::{params, Connection};

use super::{bool_flag, report_unknown_attrs};
use crate::report::{UnknownCollector, UnknownConstruct};
use crate::xml::{attrs, local_name};
use crate::ProductDbError;

const HARDWARE_ATTRS: &[&str] = &[
    "Id",
    "Name",
    "SerialNumber",
    "VersionNumber",
    "BusCurrent",
    "HasIndividualAddress",
    "HasApplicationProgram",
    "HasApplicationProgram2",
    "IsAccessory",
    "IsCoupler",
    "IsPowerSupply",
    "IsIPEnabled",
    "IsPowerLineRepeater",
    "IsPowerLineSignalFilter",
    "IsChoke",
    "IsCable",
    "OriginalManufacturer",
    "NonRegRelevantDataVersion",
];

const PRODUCT_ATTRS: &[&str] = &[
    "Id",
    "Text",
    "OrderNumber",
    "IsRailMounted",
    "WidthInMillimeter",
    "DefaultLanguage",
    "Hash",
    "NonRegRelevantDataVersion",
    "VisibleDescription",
];

const H2P_ATTRS: &[&str] = &["Id", "MediumTypes", "Hash", "NonRegRelevantDataVersion"];

pub fn ingest_hardware(
    conn: &Connection,
    source_sha256: &str,
    source_path: &str,
    bytes: &[u8],
) -> Result<Vec<UnknownConstruct>, ProductDbError> {
    let mut reader = Reader::from_reader(bytes);
    let mut buf = Vec::new();
    let mut unknown = UnknownCollector::default();
    let mut manufacturer_id = String::new();
    let mut hardware_id = String::new();
    let mut h2p_id = String::new();

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
            Event::Start(e) | Event::Empty(e) => {
                let name = local_name(&e);
                let a = attrs(&e, source_path)?;
                match name.as_str() {
                    "Manufacturer" => {
                        manufacturer_id = a.get("RefId").unwrap_or_default().to_string();
                        conn.execute(
                            "INSERT OR IGNORE INTO manufacturer (id, name) VALUES (?1, NULL)",
                            [&manufacturer_id],
                        )?;
                    }
                    // The outer <Hardware> is the collection, the inner one
                    // the entity: only the one carrying an @Id is a device.
                    "Hardware" if a.get("Id").is_some() => {
                        hardware_id = a.get("Id").unwrap_or_default().to_string();
                        report_unknown_attrs(
                            &mut unknown,
                            "/KNX/ManufacturerData/Manufacturer/Hardware/Hardware",
                            &a,
                            HARDWARE_ATTRS,
                        );
                        conn.execute(
                            "INSERT OR IGNORE INTO hardware
                             (id, manufacturer_id, name, serial_number, version_number,
                              bus_current, has_individual_address, has_application_program,
                              is_accessory, is_coupler, is_power_supply, is_ip_enabled,
                              is_power_line_repeater, original_manufacturer, source_sha256)
                             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
                            params![
                                hardware_id,
                                manufacturer_id,
                                a.get("Name"),
                                a.get("SerialNumber"),
                                a.get("VersionNumber"),
                                a.get("BusCurrent"),
                                bool_flag(&a, "HasIndividualAddress"),
                                bool_flag(&a, "HasApplicationProgram"),
                                bool_flag(&a, "IsAccessory"),
                                bool_flag(&a, "IsCoupler"),
                                bool_flag(&a, "IsPowerSupply"),
                                bool_flag(&a, "IsIPEnabled"),
                                bool_flag(&a, "IsPowerLineRepeater"),
                                a.get("OriginalManufacturer"),
                                source_sha256,
                            ],
                        )?;
                    }
                    "Product" => {
                        report_unknown_attrs(
                            &mut unknown,
                            "/KNX/ManufacturerData/Manufacturer/Hardware/Hardware/Products/Product",
                            &a,
                            PRODUCT_ATTRS,
                        );
                        conn.execute(
                            "INSERT OR IGNORE INTO product
                             (id, manufacturer_id, hardware_id, text, order_number,
                              is_rail_mounted, width_in_millimeter, default_language, hash,
                              registration_status, source_sha256)
                             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,NULL,?10)",
                            params![
                                a.get("Id"),
                                manufacturer_id,
                                hardware_id,
                                a.get("Text"),
                                a.get("OrderNumber"),
                                bool_flag(&a, "IsRailMounted"),
                                a.get("WidthInMillimeter"),
                                a.get("DefaultLanguage"),
                                a.get("Hash"),
                                source_sha256,
                            ],
                        )?;
                    }
                    "Hardware2Program" => {
                        h2p_id = a.get("Id").unwrap_or_default().to_string();
                        report_unknown_attrs(
                            &mut unknown,
                            "/KNX/ManufacturerData/Manufacturer/Hardware/Hardware/Hardware2Programs/Hardware2Program",
                            &a,
                            H2P_ATTRS,
                        );
                        conn.execute(
                            "INSERT OR IGNORE INTO hardware2program
                             (id, manufacturer_id, hardware_id, application_program_ref,
                              medium_types, hash, registration_number, registration_status,
                              registration_signature, source_sha256)
                             VALUES (?1,?2,?3,NULL,?4,?5,NULL,NULL,NULL,?6)",
                            params![
                                h2p_id,
                                manufacturer_id,
                                hardware_id,
                                a.get("MediumTypes"),
                                a.get("Hash"),
                                source_sha256,
                            ],
                        )?;
                    }
                    "ApplicationProgramRef" => {
                        conn.execute(
                            "UPDATE hardware2program SET application_program_ref = ?1 WHERE id = ?2",
                            params![a.get("RefId"), h2p_id],
                        )?;
                    }
                    // RegistrationInfo appears under both Product and
                    // Hardware2Program; the last id seen decides which.
                    "RegistrationInfo" => {
                        conn.execute(
                            "UPDATE hardware2program
                             SET registration_number = ?1, registration_status = ?2,
                                 registration_signature = ?3
                             WHERE id = ?4",
                            params![
                                a.get("RegistrationNumber"),
                                a.get("RegistrationStatus"),
                                a.get("RegistrationSignature"),
                                h2p_id,
                            ],
                        )?;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    Ok(unknown.into_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_and_migrate;

    const HARDWARE: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11">
  <ManufacturerData>
    <Manufacturer RefId="M-006A">
      <Hardware>
        <Hardware Id="M-006A_H-EM12102-6-O0079" Name="Präsenzmelder" SerialNumber="EM12102"
                  VersionNumber="6" BusCurrent="1.5000000e+001" HasIndividualAddress="1"
                  HasApplicationProgram="1" IsPowerSupply="0" IsCoupler="0" IsIPEnabled="0"
                  OriginalManufacturer="M-0079">
          <Products>
            <Product Id="M-006A_H-EM12102-6-O0079_P-N000520" Text="Präsenzmelder"
                     OrderNumber="N000520" IsRailMounted="0" DefaultLanguage="de-DE"
                     Hash="x2MHcIV+s36yBfIVtjnD38UVuOo=">
              <RegistrationInfo RegistrationStatus="Unregistered" />
            </Product>
          </Products>
          <Hardware2Programs>
            <Hardware2Program Id="M-006A_H-EM12102-6-O0079_HP-0001-22-26C0-O0079"
                              MediumTypes="MT-0" Hash="VD7KKaEG0iE5BAyQVg4gHPpNxaQ=">
              <ApplicationProgramRef RefId="M-006A_A-0001-22-26C0-O0079" />
              <RegistrationInfo RegistrationNumber="0001/22" RegistrationStatus="Registered" />
            </Hardware2Program>
          </Hardware2Programs>
        </Hardware>
      </Hardware>
    </Manufacturer>
  </ManufacturerData>
</KNX>"#;

    fn db() -> (tempfile::TempDir, Connection) {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
        (dir, conn)
    }

    #[test]
    fn hardware_product_and_program_link_are_stored() {
        let (_dir, conn) = db();
        ingest_hardware(&conn, "sha-1", "M-006A/Hardware.xml", HARDWARE.as_bytes()).unwrap();

        let (name, serial, bus_current, coupler): (String, String, String, i64) = conn
            .query_row(
                "SELECT name, serial_number, bus_current, is_coupler FROM hardware WHERE id = ?1",
                ["M-006A_H-EM12102-6-O0079"],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .unwrap();
        assert_eq!(name, "Präsenzmelder");
        assert_eq!(serial, "EM12102");
        // Stored verbatim: the source writes it in scientific notation and
        // we do not reinterpret manufacturer values (CLAUDE.md).
        assert_eq!(bus_current, "1.5000000e+001");
        assert_eq!(coupler, 0);

        let order: String = conn
            .query_row(
                "SELECT order_number FROM product WHERE hardware_id = ?1",
                ["M-006A_H-EM12102-6-O0079"],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(order, "N000520");

        let (program_ref, media, status): (String, String, String) = conn
            .query_row(
                "SELECT application_program_ref, medium_types, registration_status
                 FROM hardware2program WHERE id = ?1",
                ["M-006A_H-EM12102-6-O0079_HP-0001-22-26C0-O0079"],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(program_ref, "M-006A_A-0001-22-26C0-O0079");
        assert_eq!(media, "MT-0");
        assert_eq!(status, "Registered");
    }

    #[test]
    fn the_manufacturer_row_is_created_from_the_ref_id() {
        let (_dir, conn) = db();
        ingest_hardware(&conn, "sha-1", "M-006A/Hardware.xml", HARDWARE.as_bytes()).unwrap();
        let id: String = conn
            .query_row("SELECT id FROM manufacturer", [], |r| r.get(0))
            .unwrap();
        assert_eq!(id, "M-006A");
    }

    #[test]
    fn ingesting_the_same_hardware_twice_does_not_duplicate_rows() {
        let (_dir, conn) = db();
        ingest_hardware(&conn, "sha-1", "M-006A/Hardware.xml", HARDWARE.as_bytes()).unwrap();
        ingest_hardware(&conn, "sha-1", "M-006A/Hardware.xml", HARDWARE.as_bytes()).unwrap();
        let rows: i64 = conn
            .query_row("SELECT count(*) FROM hardware", [], |r| r.get(0))
            .unwrap();
        assert_eq!(rows, 1);
    }

    #[test]
    fn an_unknown_attribute_is_reported_and_the_rest_still_lands() {
        let (_dir, conn) = db();
        let xml = HARDWARE.replace("IsCoupler=\"0\"", "IsCoupler=\"0\" FancyNewFlag=\"7\"");
        let unknown =
            ingest_hardware(&conn, "sha-2", "M-006A/Hardware.xml", xml.as_bytes()).unwrap();
        assert!(unknown.iter().any(|u| u.name == "FancyNewFlag"));
        let rows: i64 = conn
            .query_row("SELECT count(*) FROM hardware", [], |r| r.get(0))
            .unwrap();
        assert_eq!(rows, 1);
    }

    #[test]
    fn truncated_hardware_xml_is_an_error_naming_the_file() {
        let (_dir, conn) = db();
        let truncated = &HARDWARE.as_bytes()[..HARDWARE.len() / 2];
        let err = ingest_hardware(&conn, "sha-3", "M-006A/Hardware.xml", truncated).unwrap_err();
        assert!(format!("{err}").contains("M-006A/Hardware.xml"));
    }
}
