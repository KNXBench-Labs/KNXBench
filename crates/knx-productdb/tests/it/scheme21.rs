//! Scheme-21 acceptance retains exact source bytes and reports observed deltas.
use std::io::{Cursor, Write};

use knx_productdb::report::UnknownKind;
use knx_productdb::{install_package, open_and_migrate, PackageError, ProductDbError};
use rusqlite::Connection;
use zip::write::SimpleFileOptions;

const MASTER: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/21"><MasterData><Manufacturers><Manufacturer Id="M-0021" Name="Example"/></Manufacturers><DatapointTypes><DatapointType Id="D-21" Number="21" VariableLength="true"/></DatapointTypes></MasterData></KNX>"#;
const HARDWARE: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/21"><ManufacturerData><Manufacturer RefId="M-0021"><Hardware><Hardware Id="H-21" Name="Example"><Products><Product Id="P-21" Text="Product"/></Products><Hardware2Programs><Hardware2Program Id="H2P-21" CouplerCapabilities="coupler" RFRxCapabilities="rx" RFTxCapabilities="tx"/></Hardware2Programs></Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;
const PROGRAM: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/21"><ManufacturerData><Manufacturer RefId="M-0021"><ApplicationPrograms><ApplicationProgram Id="A-21" Name="Program" HardwareType="rf"><Static><LoadProcedures><LoadProcedure><LdCtrlDeclarePropDesc ObjIdx="1"/></LoadProcedure></LoadProcedures></Static></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

fn db() -> (tempfile::TempDir, Connection) {
    let directory = tempfile::tempdir().unwrap();
    let connection = open_and_migrate(&directory.path().join("products.sqlite")).unwrap();
    (directory, connection)
}

fn archive(members: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (path, bytes) in members {
        writer
            .start_file(*path, SimpleFileOptions::default())
            .unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

#[test]
fn scheme_21_installs_and_reports_all_three_parser_branches() {
    let (_directory, connection) = db();
    let bytes = archive(&[
        ("knx_master.xml", MASTER),
        ("M-0021/Hardware.xml", HARDWARE),
        ("M-0021/Application.xml", PROGRAM),
    ]);
    let report = install_package(&connection, "scheme-21.knxprod", &bytes).unwrap();
    assert_eq!(report.scheme, 21);
    let facts = report.facts.expect("persisted evidence");
    for (name, suffix, sample) in [
        ("HardwareType", "/ApplicationProgram", "rf"),
        ("CouplerCapabilities", "/Hardware2Program", "coupler"),
        ("RFRxCapabilities", "/Hardware2Program", "rx"),
        ("RFTxCapabilities", "/Hardware2Program", "tx"),
        ("VariableLength", "/DatapointType", "true"),
    ] {
        let row = facts
            .unknown_constructs
            .iter()
            .find(|item| {
                item.kind == UnknownKind::Attribute
                    && item.name == name
                    && item.xpath.ends_with(suffix)
            })
            .unwrap_or_else(|| panic!("missing {name} evidence: {:?}", facts.unknown_constructs));
        assert_eq!(row.sample.as_deref(), Some(sample));
    }
    assert!(facts
        .unknown_constructs
        .iter()
        .any(|item| { item.kind == UnknownKind::Element && item.name == "LdCtrlDeclarePropDesc" }));
    for (path, original) in [
        ("M-0021/Hardware.xml", HARDWARE),
        ("M-0021/Application.xml", PROGRAM),
    ] {
        let stored: Vec<u8> = connection
            .query_row(
                "SELECT bytes FROM source_file WHERE source_path = ?1",
                [path],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(stored, original);
    }
}

#[test]
fn scheme_21_retains_unmodelled_master_and_program_subtree_fields() {
    let (_directory, connection) = db();
    let master = br#"<KNX xmlns="http://knx.org/xml/project/21"><MasterData><Manufacturers><Manufacturer Id="M-0021" Name="Example"/></Manufacturers><Resources><Resource Name="R" Optional="true"/></Resources><InterfaceObjectTypes><InterfaceObjectType><InterfaceObjectProperties><InterfaceObjectProperty AccessPolicy="Read"/></InterfaceObjectProperties></InterfaceObjectType></InterfaceObjectTypes></MasterData></KNX>"#;
    let program = br#"<KNX xmlns="http://knx.org/xml/project/21"><ManufacturerData><Manufacturer RefId="M-0021"><ApplicationPrograms><ApplicationProgram Id="A-21" Name="Program"><Static><ParameterTypes><ParameterType Id="PT-21"><TypeText><Format><String NullTerminated="true" VariableLength="true"/></Format></TypeText></ParameterType></ParameterTypes></Static></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;
    let bytes = archive(&[("knx_master.xml", master), ("M-0021/A.xml", program)]);
    let report = install_package(&connection, "subtree-21.knxprod", &bytes).unwrap();
    let facts = report.facts.unwrap();
    for (name, suffix, sample) in [
        ("Optional", "/Resources/Resource", "true"),
        (
            "AccessPolicy",
            "/InterfaceObjectProperties/InterfaceObjectProperty",
            "Read",
        ),
        ("NullTerminated", "/Format/String", "true"),
        ("VariableLength", "/Format/String", "true"),
    ] {
        let rows: Vec<_> = facts
            .unknown_constructs
            .iter()
            .filter(|row| {
                row.kind == UnknownKind::Attribute
                    && row.name == name
                    && row.xpath.ends_with(suffix)
            })
            .collect();
        assert_eq!(rows.len(), 1, "expected one retained {name} at {suffix}");
        assert_eq!(rows[0].sample.as_deref(), Some(sample));
        assert_eq!(rows[0].occurrences, 1);
    }
    let stored: Vec<u8> = connection
        .query_row(
            "SELECT bytes FROM source_file WHERE source_path = 'knx_master.xml'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(stored, master);
}

#[test]
fn scheme_21_observed_master_paths_report_fields_even_below_opaque_branches() {
    let (_directory, connection) = db();
    let master = br#"<KNX xmlns="http://knx.org/xml/project/21"><MasterData><Manufacturers><Manufacturer Id="M-0021"/></Manufacturers><DatapointTypes><DatapointType Id="D-21" VariableLength="true"><DatapointSubtypes><DatapointSubtype><Format><String NullTerminated="true" VariableLength="true"/></Format></DatapointSubtype></DatapointSubtypes></DatapointType></DatapointTypes><InterfaceObjectProperties><InterfaceObjectProperty AccessPolicy="Read"/></InterfaceObjectProperties><MaskVersions><MaskVersion><HawkConfigurationData><Resources><Resource Optional="true"/></Resources></HawkConfigurationData></MaskVersion></MaskVersions></MasterData></KNX>"#;
    let bytes = archive(&[
        ("knx_master.xml", master),
        ("M-0021/Application.xml", PROGRAM),
    ]);
    let facts = install_package(&connection, "observed-shapes-21.knxprod", &bytes)
        .unwrap()
        .facts
        .unwrap();
    for (name, suffix) in [
        ("VariableLength", "/DatapointTypes/DatapointType"),
        ("NullTerminated", "/Format/String"),
        ("VariableLength", "/Format/String"),
        (
            "AccessPolicy",
            "/InterfaceObjectProperties/InterfaceObjectProperty",
        ),
        ("Optional", "/HawkConfigurationData/Resources/Resource"),
    ] {
        let rows: Vec<_> = facts
            .unknown_constructs
            .iter()
            .filter(|row| {
                row.kind == UnknownKind::Attribute
                    && row.name == name
                    && row.xpath.ends_with(suffix)
            })
            .collect();
        assert_eq!(
            rows.len(),
            1,
            "missing or duplicated observed {name} at {suffix}"
        );
        assert_eq!(rows[0].occurrences, 1);
        assert_eq!(
            rows[0].sample.as_deref(),
            Some(if name == "AccessPolicy" {
                "Read"
            } else {
                "true"
            })
        );
    }
}

#[test]
fn scheme_21_foreign_master_and_hardware_lookalikes_fail_closed() {
    let (_directory, connection) = db();
    let master = br#"<KNX xmlns="http://knx.org/xml/project/21" xmlns:e="urn:extension"><MasterData><Manufacturers><Manufacturer Id="M-0021"/></Manufacturers><Resources><Resource Optional="canonical"/><e:Resource Optional="foreign"/></Resources></MasterData></KNX>"#;
    let hardware = br#"<KNX xmlns="http://knx.org/xml/project/21" xmlns:e="urn:extension"><ManufacturerData><Manufacturer RefId="M-0021"><Hardware><Hardware Id="H-21"><Hardware2Programs><Hardware2Program Id="HP-21" CouplerCapabilities="canonical"/><e:Hardware2Program CouplerCapabilities="foreign"/></Hardware2Programs></Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;
    let bytes = archive(&[
        ("knx_master.xml", master),
        ("M-0021/Hardware.xml", hardware),
    ]);
    let error = install_package(&connection, "foreign-21.knxprod", &bytes).unwrap_err();
    assert!(
        matches!(error, PackageError::Database(_))
            && error
                .to_string()
                .contains("scheme-21 XML contains a non-KNX element namespace"),
        "unexpected foreign namespace error: {error}"
    );
    for table in [
        "package",
        "source_file",
        "hardware2program",
        "package_install_unknown",
    ] {
        let count: i64 = connection
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 0, "foreign lookalike published {table} rows");
    }
}

#[test]
fn scheme_21_every_typed_member_rejects_foreign_xml_atomically() {
    for (path, xml) in [
        ("M-0021/Hardware.xml", br#"<KNX xmlns="http://knx.org/xml/project/21" xmlns:e="urn:extension"><ManufacturerData><Manufacturer RefId="M-0021"><Hardware><Hardware Id="H-21"><e:Hardware2Programs/></Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#.as_slice()),
        ("M-0021/Catalog.xml", br#"<KNX xmlns="http://knx.org/xml/project/21" xmlns:e="urn:extension"><ManufacturerData><Manufacturer RefId="M-0021"><Catalog><e:CatalogItem/></Catalog></Manufacturer></ManufacturerData></KNX>"#),
        ("M-0021/Application.xml", br#"<KNX xmlns="http://knx.org/xml/project/21" xmlns:e="urn:extension"><ManufacturerData><Manufacturer RefId="M-0021"><ApplicationPrograms><ApplicationProgram Id="A-21"><e:ComObjectTable/></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#),
        ("M-0021/Baggages.xml", br#"<KNX xmlns="http://knx.org/xml/project/21" xmlns:e="urn:extension"><ManufacturerData><Manufacturer RefId="M-0021"><Baggages><e:Baggage/></Baggages></Manufacturer></ManufacturerData></KNX>"#),
        ("M-0021/Hardware.xml", br#"<KNX xmlns="http://knx.org/xml/project/21" xmlns:e="urn:extension"><ManufacturerData><Manufacturer RefId="M-0021"><Hardware><Hardware Id="H-21" e:Name="foreign"/></Hardware></Manufacturer></ManufacturerData></KNX>"#),
    ] {
        let (_directory, connection) = db();
        let bytes = archive(&[("knx_master.xml", MASTER), (path, xml)]);
        let error = install_package(&connection, "foreign-typed-21.knxprod", &bytes).unwrap_err();
        let expected = if xml.windows(7).any(|window| window == b"e:Name=") {
            "scheme-21 XML contains a qualified attribute"
        } else {
            "scheme-21 XML contains a non-KNX element namespace"
        };
        assert!(
            matches!(error, PackageError::Database(_)) && error.to_string().contains(expected),
            "unexpected foreign XML error in {path}: {error}"
        );
        for table in ["package", "source_file", "package_install_unknown"] {
            let count: i64 = connection.query_row(&format!("SELECT count(*) FROM {table}"), [], |row| row.get(0)).unwrap();
            assert_eq!(count, 0, "foreign XML in {path} published {table}");
        }
    }
}

#[test]
fn scheme_21_malformed_late_member_rolls_back_everything() {
    let (_directory, connection) = db();
    let before: i64 = connection
        .query_row("SELECT count(*) FROM package", [], |row| row.get(0))
        .unwrap();
    let bytes = archive(&[
        ("knx_master.xml", MASTER),
        ("M-0021/Hardware.xml", HARDWARE),
        (
            "M-0021/broken.xml",
            b"<KNX xmlns=\"http://knx.org/xml/project/21\">",
        ),
    ]);
    let error = install_package(&connection, "broken-21.knxprod", &bytes).unwrap_err();
    assert!(
        matches!(
            error,
            PackageError::Database(ProductDbError::Xml { ref source_path, ref cause })
                if source_path == "M-0021/broken.xml" && !cause.is_empty()
        ),
        "unexpected malformed XML error: {error}"
    );
    assert_eq!(
        connection
            .query_row("SELECT count(*) FROM package", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        before
    );
    assert_eq!(
        connection
            .query_row("SELECT count(*) FROM source_file", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
}

#[test]
fn scheme_21_post_write_evidence_failure_rolls_back_every_table() {
    let (_directory, connection) = db();
    let program = format!(
        "<KNX xmlns=\"http://knx.org/xml/project/21\"><ManufacturerData><Manufacturer RefId=\"M-0021\"><ApplicationPrograms><ApplicationProgram Id=\"A-21\" Name=\"Program\"><Static>{}<Property Occurrence=\"1\"/>{}</Static></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>",
        "<Wrapper>".repeat(1_025),
        "</Wrapper>".repeat(1_025),
    );
    let bytes = archive(&[
        ("knx_master.xml", MASTER),
        ("M-0021/A.xml", program.as_bytes()),
    ]);
    let error = install_package(&connection, "too-deep-21.knxprod", &bytes).unwrap_err();
    assert!(
        matches!(
            error,
            PackageError::Database(ProductDbError::Xml { ref source_path, ref cause })
                if source_path == "M-0021/A.xml" && cause.contains("XML nesting exceeds evidence limit 1024")
        ),
        "unexpected evidence-depth error: {error}"
    );
    for table in [
        "package",
        "source_file",
        "application_program",
        "package_install_unknown",
        "source_parse_evidence",
    ] {
        let count: i64 = connection
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 0, "post-write failure published {table} rows");
    }
}

#[test]
fn older_scheme_master_is_not_subject_to_scheme_21_evidence_limits() {
    for scheme in [12, 14] {
        let (_directory, connection) = db();
        let master = format!(
            "<KNX xmlns=\"http://knx.org/xml/project/{scheme}\"><MasterData><Manufacturers><Manufacturer Id=\"M-0021\"/></Manufacturers><Unmodelled>{}{}</Unmodelled></MasterData></KNX>",
            "<Wrapper>".repeat(1_025),
            "</Wrapper>".repeat(1_025),
        );
        let hardware = String::from_utf8(HARDWARE.to_vec()).unwrap().replace(
            "http://knx.org/xml/project/21",
            &format!("http://knx.org/xml/project/{scheme}"),
        );
        let bytes = archive(&[
            ("knx_master.xml", master.as_bytes()),
            ("M-0021/Hardware.xml", hardware.as_bytes()),
        ]);
        install_package(&connection, "older-scheme.knxprod", &bytes)
            .unwrap_or_else(|error| panic!("scheme {scheme} master rejected: {error}"));
        let count: i64 = connection
            .query_row("SELECT count(*) FROM package", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }
}

#[test]
fn scheme_22_remains_rejected_without_publication() {
    let (_directory, connection) = db();
    let bytes = archive(&[(
        "knx_master.xml",
        br#"<KNX xmlns="http://knx.org/xml/project/22"><MasterData/></KNX>"#,
    )]);
    assert!(
        matches!(install_package(&connection, "unsupported.knxprod", &bytes), Err(PackageError::UnsupportedNamespace { namespace }) if namespace.ends_with("/22"))
    );
    assert_eq!(
        connection
            .query_row("SELECT count(*) FROM package", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
}
