//! System-2 source images use their own descriptor and indexed-association grammars.

use knx_core::{GroupAddress, IndividualAddress};
use knx_productdb::image::{build_system2_image, ImageRequest, Link};

const PROGRAM: &str = r#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0000">
<ApplicationPrograms><ApplicationProgram Id="M-0000_A-0001-01-0000" Name="Synthetic" ApplicationNumber="1"
ApplicationVersion="1" MaskVersion="MV-0025" LoadProcedureStyle="ProductProcedure"><Static>
<Code>
<AbsoluteSegment Id="GA" Address="278" Size="16"><Data>AAAAAAAAAAAAAAAAAAAAAA==</Data></AbsoluteSegment>
<AbsoluteSegment Id="AA" Address="608" Size="16"><Data>AAAAAAAAAAAAAAAAAAAAAA==</Data></AbsoluteSegment>
<AbsoluteSegment Id="OBJ" Address="1024" Size="8"><Data>AqcSJQA0pQg=</Data></AbsoluteSegment>
<AbsoluteSegment Id="PAR" Address="15360" Size="4"><Data>ABIAAA==</Data></AbsoluteSegment>
</Code>
<ParameterTypes><ParameterType Id="PT" Name="Number"><TypeNumber SizeInBit="8" Type="unsignedInt" minInclusive="0" maxInclusive="255" /></ParameterType></ParameterTypes>
<Parameters><Parameter Id="P" Name="Level" ParameterType="PT" Value="18"><Memory CodeSegment="PAR" Offset="1" BitOffset="0" /></Parameter></Parameters>
<ParameterRefs><ParameterRef Id="P_R" RefId="P" /></ParameterRefs>
<ComObjectTable CodeSegment="OBJ" Offset="0"><ComObject Id="O" Name="Switch" Number="1" ObjectSize="1 Byte"
ReadFlag="Enabled" WriteFlag="Disabled" CommunicationFlag="Enabled" TransmitFlag="Enabled" UpdateFlag="Disabled" ReadOnInitFlag="Disabled" /></ComObjectTable>
<ComObjectRefs><ComObjectRef Id="O_R" RefId="O" /></ComObjectRefs>
<AddressTable CodeSegment="GA" Offset="0" MaxEntries="6" />
<AssociationTable CodeSegment="AA" Offset="0" MaxEntries="7" />
<LoadProcedures><LoadProcedure><LdCtrlConnect /><LdCtrlTaskCtrl2 LsmIdx="3" Callback="0" Address="1024" Seg0="0" Seg1="0" />
<LdCtrlTaskSegment LsmIdx="5" Address="1024" /><LdCtrlDisconnect /></LoadProcedure></LoadProcedures>
</Static><Dynamic><Channel Id="CH" Text="Synthetic"><ParameterBlock Id="PB" Text="Block">
<ParameterRefRef RefId="P_R" /><ComObjectRefRef RefId="O_R" />
</ParameterBlock></Channel></Dynamic>
</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

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
fn real_product_evaluator_builds_parameter_and_legacy_table_images_without_admitting_a_download() {
    let (_dir, conn, request) = fixture(PROGRAM);
    let image = build_system2_image(&conn, &request).expect("offline image");
    assert_eq!(
        image.segment("OBJ").unwrap().octets,
        [2, 0xA7, 0x12, 0x21, 0, 0x34, 0x6F, 7]
    );
    assert_eq!(image.segment("PAR").unwrap().octets, [0, 0x56, 0, 0]);
    assert_eq!(
        &image.segment("GA").unwrap().octets[..5],
        &[2, 0x12, 0x34, 0x34, 0x56]
    );
    assert_eq!(
        &image.segment("AA").unwrap().octets[..5],
        &[2, 0xFE, 0, 1, 1]
    );
    assert!(image
        .inferences
        .iter()
        .any(|rule| rule.rule == "system2-source-image"));
    assert!(knx_productdb::download_plan::plan_memory_download(&image).is_err());
}

#[test]
fn sparse_objects_and_sorted_addresses_keep_their_legacy_association_indices() {
    // Four literal source slots, including inactive 0 and 2; pointers and types
    // must survive. The active source order is deliberately 3, then 1.
    let xml = PROGRAM
        .replace(r#"Id="OBJ" Address="1024" Size="8"><Data>AqcSJQA0pQg="#,
                 r#"Id="OBJ" Address="1024" Size="14"><Data>BKcSJQA0pQhW9Ql4IAo="#)
        .replace("</ComObjectTable>", r#"<ComObject Id="O3" Number="3" ObjectSize="2 Bytes" ReadFlag="Disabled" WriteFlag="Enabled" CommunicationFlag="Enabled" TransmitFlag="Disabled" UpdateFlag="Enabled" ReadOnInitFlag="Disabled" /></ComObjectTable>"#)
        .replace("</ComObjectRefs>", r#"<ComObjectRef Id="O3_R" RefId="O3" /></ComObjectRefs>"#)
        .replace(r#"<ComObjectRefRef RefId="O_R" />"#, r#"<ComObjectRefRef RefId="O3_R" /><ComObjectRefRef RefId="O_R" />"#);
    let (_dir, conn, mut request) = fixture(&xml);
    request.links.extend([
        Link {
            object: 3,
            group_address: GroupAddress::from_raw(0x4567),
            sending: true,
        },
        Link {
            object: 3,
            group_address: GroupAddress::from_raw(0x0102),
            sending: false,
        },
    ]);
    let image = build_system2_image(&conn, &request).unwrap();
    assert_eq!(
        image.objects.iter().map(|o| o.number).collect::<Vec<_>>(),
        [1, 3]
    );
    assert_eq!(
        image.segment("OBJ").unwrap().octets,
        [4, 0xA7, 0x12, 0x21, 0, 0x34, 0x6F, 7, 0x56, 0xF1, 9, 0x78, 0xB7, 8]
    );
    assert_eq!(
        &image.segment("GA").unwrap().octets[..9],
        &[4, 0x12, 0x34, 1, 2, 0x34, 0x56, 0x45, 0x67]
    );
    assert_eq!(
        &image.segment("AA").unwrap().octets[..11],
        &[5, 0xFE, 0, 2, 1, 0xFE, 2, 3, 3, 1, 3]
    );
    request.links.reverse();
    assert_eq!(build_system2_image(&conn, &request).unwrap(), image);
    assert!(knx_productdb::download_plan::plan_memory_download(&image).is_err());
}

#[test]
fn two_active_object_references_with_the_same_number_are_a_conflict() {
    let xml = PROGRAM
        .replace(
            "</ComObjectRefs>",
            r#"<ComObjectRef Id="O_ALIAS" RefId="O" /></ComObjectRefs>"#,
        )
        .replace(
            r#"<ComObjectRefRef RefId="O_R" />"#,
            r#"<ComObjectRefRef RefId="O_R" /><ComObjectRefRef RefId="O_ALIAS" />"#,
        );
    let (_dir, conn, request) = fixture(&xml);
    let before = conn.total_changes();
    let error = build_system2_image(&conn, &request).unwrap_err();
    assert_eq!(
        error.to_string(),
        "not supported for a download: group object 1 is listed twice"
    );
    assert_eq!(conn.total_changes(), before);
}

#[test]
fn active_reference_defaults_and_project_values_preserve_unrelated_template_bits() {
    let xml = PROGRAM
        .replace("ABIAAA==", "q6XN7w==")
        .replace(r#"SizeInBit="8""#, r#"SizeInBit="3""#)
        .replace(r#"Value="18""#, r#"Value="2""#)
        .replace(r#"Offset="1" BitOffset="0""#, r#"Offset="1" BitOffset="2""#)
        .replace(
            r#"<ParameterRef Id="P_R" RefId="P" />"#,
            r#"<ParameterRef Id="P_R" RefId="P" Value="5" />"#,
        );
    let (_dir, conn, mut request) = fixture(&xml);
    request.values.clear();
    let defaults = build_system2_image(&conn, &request).unwrap();
    // 0xA5 = 10|100|101; only the middle three bits belong to this field.
    assert_eq!(
        defaults.segment("PAR").unwrap().octets,
        [0xAB, 0xAD, 0xCD, 0xEF]
    );
    assert_eq!(defaults.parameters.get("P_R"), Some(&5));
    request.values.insert("P_R".into(), "3".into());
    let edited = build_system2_image(&conn, &request).unwrap();
    assert_eq!(
        edited.segment("PAR").unwrap().octets,
        [0xAB, 0x9D, 0xCD, 0xEF]
    );
    assert_eq!(edited.parameters.get("P_R"), Some(&3));
    assert_eq!(edited.parameter_fields.len(), 1);
    assert_eq!(edited.parameter_fields[0].reference, "P_R");
    assert_eq!(
        edited.code.segment("PAR").unwrap().data.as_deref(),
        Some(&[0xAB, 0xA5, 0xCD, 0xEF][..])
    );
}

#[test]
fn uninterpreted_fixup_declarations_refuse_system2_images_without_database_changes() {
    let (_dir, baseline, request) = fixture(PROGRAM);
    assert!(build_system2_image(&baseline, &request).is_ok());
    for declaration in [
        "<FixupList />",
        r#"<FixupList><Fixup FunctionRef="MV-0025_ME-Unknown" CodeSegment="PAR"><Offset>1</Offset></Fixup></FixupList>"#,
    ] {
        let xml = PROGRAM.replace("</Static>", &format!("{declaration}</Static>"));
        let (_dir, conn, request) = fixture(&xml);
        assert!(
            knx_productdb::code::load_program_code(&conn, &request.program_id)
                .unwrap()
                .unwrap()
                .has_fixup_list
        );
        let before = conn.total_changes();
        let error = build_system2_image(&conn, &request)
            .expect_err("an uninterpreted transformation must not yield a partial image");
        assert!(
            matches!(error, knx_productdb::image::ImageError::Unsupported { ref what }
            if what == "System2 FixupList requires a separate source interpretation")
        );
        assert_eq!(conn.total_changes(), before);
    }
    let xml = PROGRAM.replace("</Static>", "<Extension><FixupList /></Extension></Static>");
    let (_dir, conn, request) = fixture(&xml);
    assert!(
        !knx_productdb::code::load_program_code(&conn, &request.program_id)
            .unwrap()
            .unwrap()
            .has_fixup_list
    );
    assert!(build_system2_image(&conn, &request).is_ok());
}

#[test]
fn inactive_legacy_patch_declarations_do_not_return_partial_system2_images() {
    let (_dir, baseline, request) = fixture(PROGRAM);
    assert!(build_system2_image(&baseline, &request).is_ok());
    for value in ["", "0", "1", "false", "true"] {
        let inactive = format!("<Parameter Id=\"INACTIVE\" ParameterType=\"PT\" Value=\"2\" LegacyPatchAlways=\"{value}\"><Memory CodeSegment=\"PAR\" Offset=\"2\" BitOffset=\"0\" /></Parameter>");
        let xml = PROGRAM.replace("</Parameters>", &format!("{inactive}</Parameters>"));
        let (_dir, conn, request) = fixture(&xml);
        let before = conn.total_changes();
        let result = build_system2_image(&conn, &request);
        assert!(
            matches!(result, Err(knx_productdb::image::ImageError::Unsupported { ref what })
            if what == "System2 LegacyPatchAlways requires a separate source interpretation"),
            "an inactive uninterpreted patch declaration must not yield an image"
        );
        assert_eq!(conn.total_changes(), before);
    }
}

#[test]
fn explicit_invisible_parameter_policies_never_return_an_uninterpreted_image() {
    // A nonzero inactive field distinguishes background data from an invented zero.
    let xml = PROGRAM
        .replace("ABIAAA==", "ABKlAA==")
        .replace(
            "</Parameters>",
            r#"<Parameter Id="Q" ParameterType="PT" Value="7"><Memory CodeSegment="PAR" Offset="2" BitOffset="0" /></Parameter></Parameters>"#,
        )
        .replace(
            "</ParameterRefs>",
            r#"<ParameterRef Id="Q_R" RefId="Q" Value="9" /></ParameterRefs>"#,
        );
    let (_dir, conn, mut request) = fixture(&xml);
    request.values.insert("Q_R".into(), "33".into());
    assert_eq!(
        build_system2_image(&conn, &request)
            .unwrap()
            .segment("PAR")
            .unwrap()
            .octets,
        [0, 0x56, 0xA5, 0]
    );
    for policy in ["Background", "Default", "None", "", "future-policy"] {
        let source = xml.replace(
            "<Static>",
            &format!(r#"<Static><Options DownloadInvisibleParameters="{policy}" />"#),
        );
        let (_dir, conn, mut request) = fixture(&source);
        request.values.insert("Q_R".into(), "33".into());
        let before = conn.total_changes();
        let error = build_system2_image(&conn, &request).expect_err(
            "an explicit unimplemented policy must not silently use active-only writes",
        );
        assert!(
            matches!(error, knx_productdb::image::ImageError::Unsupported { ref what }
                if what == &format!("System2 DownloadInvisibleParameters={policy:?} requires a separate source interpretation")),
            "unexpected refusal: {error}"
        );
        assert_eq!(conn.total_changes(), before);
    }
}

#[test]
fn extension_policy_lookalikes_do_not_become_download_instructions() {
    for declaration in [
        r#"<Options xmlns:ext="urn:example:extension" ext:DownloadInvisibleParameters="Background" />"#,
        r#"<Extension><Options DownloadInvisibleParameters="Background" /></Extension>"#,
    ] {
        let xml = PROGRAM.replace("</Static>", &format!("{declaration}</Static>"));
        let (_dir, conn, request) = fixture(&xml);
        assert_eq!(
            build_system2_image(&conn, &request)
                .unwrap()
                .segment("PAR")
                .unwrap()
                .octets,
            [0, 0x56, 0, 0]
        );
    }
}

#[test]
fn explicit_suboctet_fields_replace_template_bits_using_source_msb_placement() {
    let xml = PROGRAM
        .replace("<Data>ABIAAA==</Data>", "<Data>AAAAAA==</Data>")
        .replace("SizeInBit=\"8\"", "SizeInBit=\"4\"")
        .replace("Value=\"18\"", "Value=\"2\"");
    let (_dir, conn, mut request) = fixture(&xml);
    request.values.insert("P_R".into(), "6".into());
    let image = build_system2_image(&conn, &request).expect("explicit single-octet field");
    assert_eq!(image.segment("PAR").unwrap().octets, [0, 0x60, 0, 0]);
    assert!(knx_productdb::download_plan::plan_memory_download(&image).is_err());
}

#[test]
fn explicit_octet_fields_replace_template_bytes_instead_of_requiring_default_equality() {
    // Cookbook Load Controls p7 explicitly modifies the template with parameters.
    // A byte-sized value has neither a byte-order nor a bit-position ambiguity.
    let xml = PROGRAM.replace("<Data>ABIAAA==</Data>", "<Data>AAAAAA==</Data>");
    let (_dir, conn, request) = fixture(&xml);
    let image = build_system2_image(&conn, &request).expect("explicit octet placement");
    assert_eq!(image.segment("PAR").unwrap().octets, [0, 0x56, 0, 0]);
}

#[test]
fn union_offsets_are_accepted_only_when_the_default_matches_source_bits() {
    let original = r#"<Parameters><Parameter Id="P" Name="Level" ParameterType="PT" Value="18"><Memory CodeSegment="PAR" Offset="1" BitOffset="0" /></Parameter></Parameters>"#;
    let union = r#"<Parameters><Union SizeInBit="8"><Memory CodeSegment="PAR" Offset="1" BitOffset="0" /><Parameter Id="P" Name="Level" ParameterType="PT" Value="18" Offset="0" BitOffset="0" /></Union></Parameters>"#;
    let xml = PROGRAM.replace(original, union);
    assert_ne!(xml, PROGRAM);
    let (_dir, conn, request) = fixture(&xml);
    let image = build_system2_image(&conn, &request).expect("independently matched union field");
    assert_eq!(image.segment("PAR").unwrap().octets, [0, 0x56, 0, 0]);
    let mismatch = xml.replace("<Data>ABIAAA==</Data>", "<Data>AAAAAA==</Data>");
    let (_dir, conn, request) = fixture(&mismatch);
    let error = build_system2_image(&conn, &request).unwrap_err();
    assert!(error
        .to_string()
        .contains("default disagrees with source bits"));
}

#[test]
fn union_offset_overflow_is_an_error_not_a_panic_or_wrapped_write() {
    let original = r#"<Parameters><Parameter Id="P" Name="Level" ParameterType="PT" Value="18"><Memory CodeSegment="PAR" Offset="1" BitOffset="0" /></Parameter></Parameters>"#;
    let union = r#"<Parameters><Union SizeInBit="8"><Memory CodeSegment="PAR" Offset="4294967295" BitOffset="0" /><Parameter Id="P" Name="Level" ParameterType="PT" Value="18" Offset="2" BitOffset="0" /></Union></Parameters>"#;
    let (_dir, conn, request) = fixture(&PROGRAM.replace(original, union));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        build_system2_image(&conn, &request)
    }));
    assert!(result.is_ok(), "untrusted placement must not panic");
    assert!(result.unwrap().is_err());
}

#[test]
fn two_active_union_members_never_use_070n_last_member_wins() {
    let original = r#"<Parameters><Parameter Id="P" Name="Level" ParameterType="PT" Value="18"><Memory CodeSegment="PAR" Offset="1" BitOffset="0" /></Parameter></Parameters>"#;
    let union = r#"<Parameters><Union SizeInBit="8"><Memory CodeSegment="PAR" Offset="1" BitOffset="0" /><Parameter Id="P" Name="Level" ParameterType="PT" Value="18" Offset="0" BitOffset="0" /><Parameter Id="Q" Name="Other" ParameterType="PT" Value="18" Offset="0" BitOffset="0" /></Union></Parameters>"#;
    let xml = PROGRAM
        .replace(original, union)
        .replace(
            "<ParameterRef Id=\"P_R\" RefId=\"P\" />",
            "<ParameterRef Id=\"P_R\" RefId=\"P\" /><ParameterRef Id=\"Q_R\" RefId=\"Q\" />",
        )
        .replace(
            "<ParameterRefRef RefId=\"P_R\" />",
            "<ParameterRefRef RefId=\"P_R\" /><ParameterRefRef RefId=\"Q_R\" />",
        );
    let (_dir, conn, request) = fixture(&xml);
    assert!(build_system2_image(&conn, &request).is_err());
}

#[test]
fn multi_octet_changes_require_a_discriminating_default_or_explicit_byte_order() {
    let xml = PROGRAM
        .replace("SizeInBit=\"8\"", "SizeInBit=\"16\"")
        .replace("maxInclusive=\"255\"", "maxInclusive=\"65535\"")
        .replace("Value=\"18\"", "Value=\"4660\"")
        .replace("ABIAAA==", "ABI0AA==");
    let (_dir, conn, mut request) = fixture(&xml);
    request.values.insert("P_R".into(), "22136".into());
    assert_eq!(
        build_system2_image(&conn, &request)
            .unwrap()
            .segment("PAR")
            .unwrap()
            .octets,
        [0, 0x56, 0x78, 0]
    );
    let ambiguous = xml
        .replace("Value=\"4660\"", "Value=\"0\"")
        .replace("ABI0AA==", "AAAAAA==");
    let (_dir, conn, mut request) = fixture(&ambiguous);
    request.values.insert("P_R".into(), "22136".into());
    assert!(build_system2_image(&conn, &request)
        .unwrap_err()
        .to_string()
        .contains("ambiguous"));
    let explicit = ambiguous.replace(
        "<Static>",
        "<Static><Options ParameterByteOrder=\"BigEndian\" />",
    );
    let (_dir, conn, mut request) = fixture(&explicit);
    request.values.insert("P_R".into(), "22136".into());
    assert_eq!(
        build_system2_image(&conn, &request)
            .unwrap()
            .segment("PAR")
            .unwrap()
            .octets,
        [0, 0x56, 0x78, 0]
    );
}

#[test]
fn malformed_placements_segments_and_reserved_table_overlaps_fail_closed() {
    let variants = [
        PROGRAM.replace("Address=\"608\"", "Address=\"280\""),
        PROGRAM.replace("Address=\"15360\"", "Address=\"65535\""),
        PROGRAM.replace("MaxEntries=\"6\"", "MaxEntries=\"7\""),
        PROGRAM.replace(
            "CodeSegment=\"OBJ\" Offset=\"0\"",
            "CodeSegment=\"OBJ\" Offset=\"7\"",
        ),
        PROGRAM.replace(
            "CodeSegment=\"PAR\" Offset=\"1\"",
            "CodeSegment=\"GA\" Offset=\"10\"",
        ),
        PROGRAM.replace(
            "CodeSegment=\"AA\" Offset=\"0\"",
            "CodeSegment=\"GA\" Offset=\"0\"",
        ),
        PROGRAM.replace("MaskVersion=\"MV-0025\"", "MaskVersion=\"MV-0701\""),
        PROGRAM.replace("ReadOnInitFlag=\"Disabled\"", "ReadOnInitFlag=\"Enabled\""),
        PROGRAM.replace(
            "ObjectSize=\"1 Byte\"",
            "ObjectSize=\"1 Byte\" Priority=\"Alert\"",
        ),
        PROGRAM.replace(
            "Size=\"4\"><Data>ABIAAA==</Data>",
            "Size=\"4\"><Data>ABIAAA==</Data><Mask>AAAAAA==</Mask>",
        ),
    ];
    for xml in variants {
        assert_ne!(xml, PROGRAM);
        let (_dir, conn, request) = fixture(&xml);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            build_system2_image(&conn, &request)
        }));
        assert!(result.is_ok(), "bounded malformed source must not panic");
        assert!(result.unwrap().is_err());
    }
}
