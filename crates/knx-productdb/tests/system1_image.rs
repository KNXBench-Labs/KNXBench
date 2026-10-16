//! System-1 source images keep Type1 tables separate from executable download admission.

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use knx_core::{GroupAddress, IndividualAddress};
use knx_productdb::image::{build_system1_image, ImageRequest, Link};

fn program() -> String {
    let mut data = vec![0; 256];
    data[17] = 0x60; // Association pointer: 0100h + 60h.
    data[18] = 0x80; // Object pointer: 0100h + 80h.
    data[22..25].copy_from_slice(&[1, 255, 255]);
    data[128..136].copy_from_slice(&[2, 0xA7, 0x12, 0xA5, 0, 0x34, 0xE5, 8]);
    data[224] = 18;
    let mut mask = vec![255; 256];
    mask[23..25].fill(0); // Only the individual-address slot is exempt.
    format!(
        r#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0000"><ApplicationPrograms>
<ApplicationProgram Id="M-0000_A-0001-01-0000" Name="Synthetic" ApplicationNumber="1" ApplicationVersion="1" MaskVersion="MV-0012" LoadProcedureStyle="DefaultProcedure"><Static>
<Code><AbsoluteSegment Id="S" Address="256" Size="256"><Data>{}</Data><Mask>{}</Mask></AbsoluteSegment></Code>
<ParameterTypes><ParameterType Id="PT" Name="Number"><TypeNumber SizeInBit="8" Type="unsignedInt" minInclusive="0" maxInclusive="255" /></ParameterType></ParameterTypes>
<Parameters><Parameter Id="P" Name="Level" ParameterType="PT" Value="18"><Memory CodeSegment="S" Offset="224" BitOffset="0" /></Parameter></Parameters>
<ParameterRefs><ParameterRef Id="P_R" RefId="P" /></ParameterRefs>
<ComObjectTable CodeSegment="S" Offset="128"><ComObject Id="O" Name="Switch" Number="1" ObjectSize="1 Byte" ReadFlag="Enabled" WriteFlag="Disabled" CommunicationFlag="Enabled" TransmitFlag="Enabled" UpdateFlag="Disabled" ReadOnInitFlag="Disabled" /></ComObjectTable>
<ComObjectRefs><ComObjectRef Id="O_R" RefId="O" /></ComObjectRefs>
<AddressTable CodeSegment="S" Offset="22" MaxEntries="6" /><AssociationTable CodeSegment="S" Offset="96" MaxEntries="7" />
<Options ParameterByteOrder="BigEndian" /></Static><Dynamic><Channel Id="CH" Text="Synthetic"><ParameterBlock Id="PB" Text="Block"><ParameterRefRef RefId="P_R" /><ComObjectRefRef RefId="O_R" /></ParameterBlock></Channel></Dynamic>
</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#,
        STANDARD.encode(data),
        STANDARD.encode(mask)
    )
}

fn fixture(xml: &str) -> (tempfile::TempDir, knx_productdb::Connection, ImageRequest) {
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    knx_productdb::ingest_file(&conn, "synthetic.xml", xml.as_bytes()).unwrap();
    let request = ImageRequest {
        program_id: "M-0000_A-0001-01-0000".into(),
        individual_address: IndividualAddress::from_raw(0x1234),
        values: [("P_R".into(), "86".into())].into(),
        links: vec![Link {
            object: 1,
            group_address: GroupAddress::from_raw(0x3456),
            sending: true,
        }],
        flag_overrides: Default::default(),
    };
    (dir, conn, request)
}

#[test]
fn actual_evaluator_builds_literal_type1_bytes_and_preserves_pointers_without_download_admission() {
    let (_dir, conn, request) = fixture(&program());
    let before = conn.total_changes();
    let image = build_system1_image(&conn, &request).expect("bounded offline System1 image");
    let data = &image.segment("S").unwrap().octets;
    assert_eq!(&data[17..19], &[0x60, 0x80]);
    assert_eq!(&data[22..27], &[2, 0x12, 0x34, 0x34, 0x56]);
    assert_eq!(&data[96..101], &[2, 0xFE, 0, 1, 1]);
    // Inactive0 retains pointer/selector/type, C cleared. Active1 has bit7=1,
    // source selector20h, transmit/read/communication/low priority = EFh.
    assert_eq!(&data[128..136], &[2, 0xA7, 0x12, 0xA1, 0, 0x34, 0xEF, 7]);
    assert_eq!(data[224], 0x56);
    assert_eq!(image.parameter_fields.len(), 1);
    assert!(image
        .inferences
        .iter()
        .any(|r| r.rule == "system1-source-image"));
    assert!(!image
        .inferences
        .iter()
        .any(|r| r.rule == "system2-source-image"));
    assert!(knx_productdb::download_plan::plan_memory_download(&image).is_err());
    assert_eq!(conn.total_changes(), before);
}

#[test]
fn every_other_family_and_nondefault_style_is_refused_by_this_explicit_entry() {
    for xml in [
        program().replace("MV-0012", "MV-0701"),
        program().replace("MV-0012", "MV-0025"),
        program().replace("DefaultProcedure", "MergedProcedure"),
        program().replace("DefaultProcedure", "ProductProcedure"),
    ] {
        let (_dir, conn, request) = fixture(&xml);
        assert!(build_system1_image(&conn, &request).is_err());
    }
}

#[test]
fn type1_update_initial_read_and_cross_family_priority_inferences_are_refused() {
    for xml in [
        program().replace("UpdateFlag=\"Disabled\"", "UpdateFlag=\"Enabled\""),
        program().replace("ReadOnInitFlag=\"Disabled\"", "ReadOnInitFlag=\"Enabled\""),
        program().replace(
            "ObjectSize=\"1 Byte\"",
            "ObjectSize=\"1 Byte\" Priority=\"Alert\"",
        ),
    ] {
        let (_dir, conn, request) = fixture(&xml);
        assert!(build_system1_image(&conn, &request).is_err());
    }
}

#[test]
fn table_pointers_reserved_regions_and_capacity_are_not_guessed_or_overwritten() {
    for xml in [
        program().replace(
            "Offset=\"96\" MaxEntries=\"7\"",
            "Offset=\"97\" MaxEntries=\"7\"",
        ),
        program().replace("Offset=\"128\"", "Offset=\"129\""),
        program().replace("Offset=\"22\"", "Offset=\"23\""),
        program().replace("Offset=\"224\"", "Offset=\"17\""),
        program().replace("Offset=\"224\"", "Offset=\"18\""),
        program().replace("Offset=\"224\"", "Offset=\"100\""),
        program().replace("MaxEntries=\"6\"", "MaxEntries=\"50\""),
        program().replace("MaxEntries=\"7\"", "MaxEntries=\"1\""),
        program().replace("Address=\"256\"", "Address=\"65535\""),
    ] {
        let (_dir, conn, request) = fixture(&xml);
        assert!(build_system1_image(&conn, &request).is_err(), "{xml}");
    }
}

#[test]
fn relative_and_dataless_sources_never_return_a_partial_absolute_image() {
    let relative = r#"<RelativeSegment Id="R" Offset="0" Size="1" LoadStateMachine="4"><Data>AQ==</Data></RelativeSegment>"#;
    for xml in [
        program().replace("</Code>", &format!("{relative}</Code>")),
        program().replace(
            "</Code>",
            "<AbsoluteSegment Id=\"EMPTY\" Address=\"4096\" Size=\"1\" /></Code>",
        ),
    ] {
        let (_dir, conn, request) = fixture(&xml);
        assert!(build_system1_image(&conn, &request).is_err());
    }
}

#[test]
fn masks_and_uncorroborated_multioctet_encoding_remain_fail_closed() {
    let xml = program()
        .replace("SizeInBit=\"8\"", "SizeInBit=\"16\"")
        .replace("maxInclusive=\"255\"", "maxInclusive=\"65535\"")
        .replace("Value=\"18\"", "Value=\"4660\"")
        .replace("ParameterByteOrder=\"BigEndian\"", "");
    let (_dir, conn, request) = fixture(&xml);
    assert!(build_system1_image(&conn, &request).is_err());
    // Distinct from the exempt individual-address slot: a changed parameter
    // at a masked octet must fail, not disappear from the image.
    let doc = program();
    let begin = doc.find("<Mask>").unwrap() + 6;
    let end = doc.find("</Mask>").unwrap();
    let mut mask = STANDARD.decode(&doc[begin..end]).unwrap();
    mask[224] = 0;
    let xml = format!("{}{}{}", &doc[..begin], STANDARD.encode(mask), &doc[end..]);
    let (_dir, conn, request) = fixture(&xml);
    assert!(build_system1_image(&conn, &request).is_err());
}

#[test]
fn retained_fixup_lists_are_not_silently_ignored_by_system1_image_assembly() {
    for fixups in [
        "<FixupList />",
        "<FixupList><Fixup Offset=\"200\" /></FixupList>",
    ] {
        let xml = program().replace("</Static>", &format!("{fixups}</Static>"));
        let (_dir, conn, request) = fixture(&xml);
        let error = build_system1_image(&conn, &request).unwrap_err();
        assert!(error.to_string().contains("FixupList"), "{error}");
    }
}

#[test]
fn invalid_source_type1_reserved_fields_are_not_hidden_by_descriptor_rewriting() {
    for (offset, value) in [(131, 0x25), (132, 0x80)] {
        let xml = program();
        let begin = xml.find("<Data>").unwrap() + 6;
        let end = xml.find("</Data>").unwrap();
        let mut data = STANDARD.decode(&xml[begin..end]).unwrap();
        data[offset] = value;
        let xml = format!("{}{}{}", &xml[..begin], STANDARD.encode(data), &xml[end..]);
        let (_dir, conn, request) = fixture(&xml);
        assert!(build_system1_image(&conn, &request).is_err());
    }
}

#[test]
fn fixup_presence_is_a_source_view_with_exact_static_ancestry() {
    for (extra, expected) in [
        ("", false),
        ("<FixupList />", true),
        ("<Extension><FixupList /></Extension>", false),
    ] {
        let xml = program().replace("</Static>", &format!("{extra}</Static>"));
        let code = knx_productdb::code::parse_program_code(
            "synthetic.xml",
            xml.as_bytes(),
            "M-0000_A-0001-01-0000",
        )
        .unwrap()
        .unwrap();
        assert_eq!(code.has_fixup_list, expected);
    }
}
