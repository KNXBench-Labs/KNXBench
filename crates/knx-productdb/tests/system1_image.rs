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
fn explicit_single_octet_bitfields_replace_template_bits_without_default_equality() {
    // Schema23 BitOffset_t fixes the MSB position; no byte order or Union
    // arithmetic is involved. Expected octets are independently specified.
    for (width, bit_offset, default, requested, base, expected_default, expected) in [
        (1, 0, 1, 0, 0x25, 0xA5, 0x25),
        (1, 7, 0, 1, 0xA5, 0xA4, 0xA5),
        (3, 2, 5, 3, 0x12, 0x2A, 0x1A),
        (4, 4, 2, 9, 0xB7, 0xB2, 0xB9),
        (7, 1, 3, 85, 0x80, 0x83, 0xD5),
    ] {
        let xml = program();
        let begin = xml.find("<Data>").unwrap() + 6;
        let end = xml.find("</Data>").unwrap();
        let mut source = STANDARD.decode(&xml[begin..end]).unwrap();
        source[223..226].copy_from_slice(&[0xD6, base, 0x6B]);
        let xml = format!(
            "{}{}{}",
            &xml[..begin],
            STANDARD.encode(&source),
            &xml[end..]
        )
        .replace("SizeInBit=\"8\"", &format!("SizeInBit=\"{width}\""))
        .replace("Value=\"18\"", &format!("Value=\"{default}\""))
        .replace(
            "maxInclusive=\"255\"",
            &format!("maxInclusive=\"{}\"", (1 << width) - 1),
        )
        .replace(
            "Offset=\"224\" BitOffset=\"0\"",
            &format!("Offset=\"224\" BitOffset=\"{bit_offset}\""),
        )
        .replace("ParameterByteOrder=\"BigEndian\"", "");
        let (_dir, conn, mut request) = fixture(&xml);
        let before = conn.total_changes();
        request.values.clear();
        let image =
            build_system1_image(&conn, &request).expect("explicit default replaces template bits");
        assert_eq!(
            &image.segment("S").unwrap().octets[223..226],
            &[0xD6, expected_default, 0x6B]
        );
        request.values.insert("P_R".into(), requested.to_string());
        let image =
            build_system1_image(&conn, &request).expect("explicit edited bits replace template");
        assert_eq!(
            &image.segment("S").unwrap().octets[223..226],
            &[0xD6, expected, 0x6B]
        );
        assert_eq!(image.parameter_fields.len(), 1);
        assert_eq!(image.parameter_fields[0].field.bit_offset, bit_offset);
        let retained = knx_productdb::code::load_program_code(&conn, &request.program_id)
            .unwrap()
            .unwrap();
        assert_eq!(
            retained.segment("S").unwrap().data.as_ref().unwrap(),
            &source
        );
        assert_eq!(conn.total_changes(), before);
        assert!(knx_productdb::download_plan::plan_memory_download(&image).is_err());
    }
}

#[test]
fn explicit_bits_keep_masks_bounds_references_and_overlap_checks() {
    let base = program()
        .replace("SizeInBit=\"8\"", "SizeInBit=\"3\"")
        .replace("Value=\"18\"", "Value=\"5\"")
        .replace("maxInclusive=\"255\"", "maxInclusive=\"7\"")
        .replace(
            "Offset=\"224\" BitOffset=\"0\"",
            "Offset=\"224\" BitOffset=\"2\"",
        );
    let mut bad_inputs = vec![
        base.replace("CodeSegment=\"S\" Offset=\"224\"", "CodeSegment=\"MISSING\" Offset=\"224\""),
        base.replace("Offset=\"224\"", "Offset=\"256\""),
        base.replace("Offset=\"224\"", "Offset=\"17\""),
        base.replace("Offset=\"224\"", "Offset=\"128\""),
        base.replace("BitOffset=\"2\"", "BitOffset=\"8\""),
        base.replace("BitOffset=\"2\"", "BitOffset=\"6\""), // Crosses an octet, not admitted here.
        base.replace(
            "</Parameters>",
            "<Parameter Id=\"Q\" ParameterType=\"PT\" Value=\"5\"><Memory CodeSegment=\"S\" Offset=\"224\" BitOffset=\"3\" /></Parameter></Parameters>",
        ).replace("</ParameterRefs>", "<ParameterRef Id=\"Q_R\" RefId=\"Q\" /></ParameterRefs>")
         .replace("<ParameterRefRef RefId=\"P_R\" />", "<ParameterRefRef RefId=\"P_R\" /><ParameterRefRef RefId=\"Q_R\" />"),
    ];
    // Protect just one changed field bit; the surrounding bits remain writable.
    let begin = base.find("<Mask>").unwrap() + 6;
    let end = base.find("</Mask>").unwrap();
    let mut mask = STANDARD.decode(&base[begin..end]).unwrap();
    mask[224] = 0xF7;
    bad_inputs.push(format!(
        "{}{}{}",
        &base[..begin],
        STANDARD.encode(mask),
        &base[end..]
    ));
    for xml in bad_inputs {
        let (_dir, conn, mut request) = fixture(&xml);
        request.values.insert("P_R".into(), "3".into());
        let before = conn.total_changes();
        assert!(build_system1_image(&conn, &request).is_err());
        assert_eq!(conn.total_changes(), before);
    }
    let (_dir, conn, mut request) = fixture(&base);
    request.values.insert("P_R".into(), "8".into());
    assert!(build_system1_image(&conn, &request).is_err());
}

#[test]
fn explicit_bitfield_substitution_still_validates_the_default_width() {
    // Declared min/max alone do not establish that a default fits its field.
    let xml = program()
        .replace("SizeInBit=\"8\"", "SizeInBit=\"3\"")
        .replace("Value=\"18\"", "Value=\"8\"")
        .replace(
            "Offset=\"224\" BitOffset=\"0\"",
            "Offset=\"224\" BitOffset=\"2\"",
        );
    let (_dir, conn, mut request) = fixture(&xml);
    request.values.insert("P_R".into(), "3".into());
    let before = conn.total_changes();
    let result = build_system1_image(&conn, &request);
    assert!(
        matches!(
            result,
            Err(knx_productdb::image::ImageError::Parameter {
                error:
                    knx_core::commissioning::parameter_image::ParameterImageError::ValueTooWide {
                        value: 8,
                        ..
                    },
                ..
            })
        ),
        "a valid request cannot conceal a malformed source default"
    );
    assert_eq!(conn.total_changes(), before);
}

#[test]
fn inactive_legacy_patch_declarations_do_not_return_partial_system1_images() {
    let (_dir, baseline, request) = fixture(&program());
    assert!(build_system1_image(&baseline, &request).is_ok());
    for value in ["", "0", "1", "false", "true"] {
        let inactive = format!("<Parameter Id=\"INACTIVE\" ParameterType=\"PT\" Value=\"2\" LegacyPatchAlways=\"{value}\"><Memory CodeSegment=\"S\" Offset=\"225\" BitOffset=\"0\" /></Parameter>");
        let xml = program().replace("</Parameters>", &format!("{inactive}</Parameters>"));
        let (_dir, conn, request) = fixture(&xml);
        let before = conn.total_changes();
        let result = build_system1_image(&conn, &request);
        assert!(
            matches!(result, Err(knx_productdb::image::ImageError::Unsupported { ref what })
            if what == "System1 LegacyPatchAlways requires a separate source interpretation"),
            "an inactive uninterpreted patch declaration must not yield an image"
        );
        assert_eq!(conn.total_changes(), before);
    }
}

#[test]
fn legacy_patch_presence_tracks_direct_parameters_and_union_members_not_extensions() {
    for (declaration, expected) in [
        ("<Parameter Id=\"INACTIVE\" LegacyPatchAlways=\"1\" />", true),
        ("<Union SizeInBit=\"8\"><Memory CodeSegment=\"S\" Offset=\"225\" BitOffset=\"0\" /><Parameter Id=\"INACTIVE\" LegacyPatchAlways=\"0\" Offset=\"0\" BitOffset=\"0\" /></Union>", true),
        ("<Extension><Parameter Id=\"INACTIVE\" LegacyPatchAlways=\"1\" /></Extension>", false),
        ("<Parameter xmlns:x=\"urn:extension\" Id=\"INACTIVE\" x:LegacyPatchAlways=\"1\" />", false),
    ] {
        let xml = program().replace("</Parameters>", &format!("{declaration}</Parameters>"));
        let (_dir, conn, request) = fixture(&xml);
        let before = conn.total_changes();
        let code = knx_productdb::code::load_program_code(&conn, &request.program_id).unwrap().unwrap();
        assert_eq!(code.has_legacy_patch_always, expected);
        let result = build_system1_image(&conn, &request);
        if expected {
            assert!(matches!(result, Err(knx_productdb::image::ImageError::Unsupported { ref what })
                if what == "System1 LegacyPatchAlways requires a separate source interpretation"));
        } else {
            assert!(result.is_ok());
        }
        assert_eq!(conn.total_changes(), before);
    }
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
