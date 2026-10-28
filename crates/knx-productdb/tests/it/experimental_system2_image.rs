//! Explicit experimental image rules are isolated from ordinary admission.

mod background_admission;

use std::collections::BTreeMap;

use knx_core::IndividualAddress;
use knx_productdb::image::{
    build_experimental_system2_image, build_system2_image, ExperimentalSystem2ImagePolicy,
    ExperimentalSystem2SourceBinding, ImageRequest,
};

const ID: &str = "M-0000_A-0001-01-0000";
const XML: &str = r#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0000"><ApplicationPrograms>
<ApplicationProgram Id="M-0000_A-0001-01-0000" Name="Synthetic" ApplicationNumber="1" ApplicationVersion="1" MaskVersion="MV-0025" LoadProcedureStyle="ProductProcedure"><Static>
<Code>
<AbsoluteSegment Id="GA" Address="278" Size="16"><Data>AAAAAAAAAAAAAAAAAAAAAA==</Data></AbsoluteSegment>
<AbsoluteSegment Id="AA" Address="608" Size="16"><Data>AAAAAAAAAAAAAAAAAAAAAA==</Data></AbsoluteSegment>
<AbsoluteSegment Id="OBJ" Address="1024" Size="8"><Data>AqcSJQA0pQg=</Data></AbsoluteSegment>
<AbsoluteSegment Id="PAR" Address="15360" Size="8"><Data>AAAAAAAAAAA=</Data><Mask>//////////8=</Mask></AbsoluteSegment>
</Code>
<ParameterTypes><ParameterType Id="PT" Name="Number"><TypeNumber SizeInBit="16" Type="unsignedInt" minInclusive="0" maxInclusive="65535" /></ParameterType></ParameterTypes>
<Parameters><Parameter Id="P" Name="Level" ParameterType="PT" Value="4660"><Memory CodeSegment="PAR" Offset="0" BitOffset="0" /></Parameter></Parameters>
<ParameterRefs><ParameterRef Id="R" RefId="P" /></ParameterRefs>
<ComObjectTable CodeSegment="OBJ" Offset="0"><ComObject Id="O" Number="1" ObjectSize="1 Byte" ReadFlag="Enabled" WriteFlag="Disabled" CommunicationFlag="Enabled" TransmitFlag="Enabled" UpdateFlag="Disabled" ReadOnInitFlag="Disabled" /></ComObjectTable>
<ComObjectRefs><ComObjectRef Id="OR" RefId="O" /></ComObjectRefs>
<AddressTable CodeSegment="GA" Offset="0" MaxEntries="6" /><AssociationTable CodeSegment="AA" Offset="0" MaxEntries="7" />
<LoadProcedures><LoadProcedure><LdCtrlConnect /><LdCtrlTaskCtrl2 LsmIdx="3" Callback="0" Address="1024" Seg0="0" Seg1="0" /><LdCtrlTaskSegment LsmIdx="5" Address="1024" /><LdCtrlDisconnect /></LoadProcedure></LoadProcedures>
</Static><Dynamic><Channel Id="CH"><ParameterBlock Id="PB"><ParameterRefRef RefId="R" /><ComObjectRefRef RefId="OR" /></ParameterBlock></Channel></Dynamic>
</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

fn policy(xml: &str) -> ExperimentalSystem2ImagePolicy {
    ExperimentalSystem2ImagePolicy {
        revision: 1,
        binding: ExperimentalSystem2SourceBinding {
            program_id: ID.into(),
            schema_namespace: "http://knx.org/xml/project/20".into(),
            source_sha256: knx_productdb::sha256_hex(xml.as_bytes()),
            package_sha256: None,
        },
        numeric_order: "high-octet-first-explicit-or-schema-default-v1".into(),
        bit_union_offsets: "msb0-byte-aligned-union-relative-v1".into(),
        legacy_patch_always: "inactive-default-ref-consensus-active-wins-v1".into(),
        invisible_parameters: "background-parameter-default-union-base-active-overlay-v1".into(),
        binary_restrictions: "single-octet-enumeration-binary-payload-v1".into(),
        reference_activation: "schema10-no-dynamic-reference-is-active-v1".into(),
    }
}

/// Revision 2 retains the template only for an explicit `Background` declaration.
fn explicit_background(xml: &str) -> String {
    assert_eq!(xml.matches("<Static>").count(), 1);
    xml.replace(
        "<Static>",
        r#"<Static><Options DownloadInvisibleParameters="Background" />"#,
    )
}

fn project_union_xml() -> String {
    explicit_background(XML)
        .replace("<Data>AAAAAAAAAAA=</Data>", "<Data>qrvM3e7/ABI=</Data>")
        .replace("</ParameterTypes>", r#"<ParameterType Id="PT3"><TypeNumber SizeInBit="3" Type="unsignedInt" minInclusive="0" maxInclusive="7" /></ParameterType></ParameterTypes>"#)
        .replace("</Parameters>", r#"<Union SizeInBit="16"><Memory CodeSegment="PAR" Offset="4" BitOffset="0" /><Parameter Id="UD" ParameterType="PT" Value="4660" Offset="0" BitOffset="0" DefaultUnionParameter="true" /><Parameter Id="UA" ParameterType="PT3" Value="5" Offset="1" BitOffset="2" /></Union></Parameters>"#)
        .replace("</ParameterRefs>", r#"<ParameterRef Id="UDR" RefId="UD" /><ParameterRef Id="UAR" RefId="UA" /></ParameterRefs>"#)
        .replace("</ParameterBlock>", r#"<ParameterRefRef RefId="UAR" /><choose ParamRefId="R"><when test="0"><ParameterRefRef RefId="UDR" /></when></choose></ParameterBlock>"#)
}

#[test]
fn project_experimental_union_legacy_patch_precedes_active_subfield_without_writing_default_base() {
    let xml = project_union_xml()
        .replace(
            r#"Id="UD" ParameterType="PT""#,
            r#"Id="UD" LegacyPatchAlways="true" ParameterType="PT""#,
        )
        .replace(
            r#"Id="UDR" RefId="UD""#,
            r#"Id="UDR" RefId="UD" Value="772""#,
        );
    let (_dir, conn, request) = fixture(&xml);
    let result = knx_productdb::image::build_project_experimental_system2_image(
        &conn,
        &request,
        &project_policy(&xml),
    )
    .unwrap();
    assert_eq!(
        &result.image.segment("PAR").unwrap().octets[4..6],
        &[3, 0x2C]
    );
    assert_eq!(
        format!(
            "{:?}",
            result
                .coverage
                .iter()
                .find(|c| c.parameter_id == "UD")
                .unwrap()
                .disposition
        ),
        "LegacyPatchDefaultReference"
    );
}

#[test]
fn project_experimental_active_legacy_and_implicit_activation_keep_production_ref_rules() {
    let xml=inactive_xml().replace(r#"Id="P" Name="Level""#,r#"Id="P" Name="Level" LegacyPatchAlways="true""#)
        .replace("</ParameterRefs>",r#"<ParameterRef Id="R2" RefId="P" Value="772" /></ParameterRefs>"#)
        .replace("</ParameterBlock>",r#"<choose ParamRefId="R"><when test="0"><ParameterRefRef RefId="R2" /></when></choose></ParameterBlock>"#);
    let (_dir, conn, mut request) = fixture(&xml);
    request.values.insert("R".into(), "22136".into());
    let result = knx_productdb::image::build_project_experimental_system2_image(
        &conn,
        &request,
        &project_policy(&xml),
    )
    .unwrap();
    assert_eq!(
        &result.image.segment("PAR").unwrap().octets[..2],
        &[0x56, 0x78]
    );
    let implicit = inactive_xml().replace(
        r#"<choose ParamRefId="R"><when test="0"><ParameterRefRef RefId="QR" /></when></choose>"#,
        "",
    );
    let (_dir, conn, request) = fixture(&implicit);
    let result = knx_productdb::image::build_project_experimental_system2_image(
        &conn,
        &request,
        &project_policy(&implicit),
    )
    .unwrap();
    assert_eq!(&result.image.segment("PAR").unwrap().octets[2..4], &[3, 4]);
    assert_eq!(result.image.parameter_fields.len(), 2);
    let duplicate = implicit.replace(
        "</ParameterRefs>",
        r#"<ParameterRef Id="R2" RefId="P" /></ParameterRefs>"#,
    );
    let (_dir, conn, request) = fixture(&duplicate);
    assert!(
        knx_productdb::image::build_project_experimental_system2_image(
            &conn,
            &request,
            &project_policy(&duplicate)
        )
        .unwrap_err()
        .to_string()
        .contains("multiple active references")
    );
}

#[test]
fn project_experimental_binding_and_policy_mismatches_refuse_without_db_mutation() {
    use knx_productdb::image::build_project_experimental_system2_image as build;
    let (_dir, conn, request) = fixture(XML);
    let before = conn.total_changes();
    for case in 0..12 {
        let mut policy = project_policy(XML);
        match case {
            0 => policy.revision = 1,
            1 => {
                policy.invisible_parameters =
                    "background-parameter-default-union-base-active-overlay-v1".into()
            }
            2 => policy.binding.schema_namespace = "http://knx.org/xml/project/23".into(),
            3 => policy.binding.source_sha256 = "A".repeat(64),
            4 => policy.binding.source_sha256 = "0".repeat(64),
            5 => policy.binding.package_sha256 = Some("0".repeat(64)),
            6 => policy.numeric_order.clear(),
            7 => policy.bit_union_offsets.clear(),
            8 => policy.legacy_patch_always.clear(),
            9 => policy.binary_restrictions.clear(),
            10 => policy.reference_activation.clear(),
            11 => policy.binding.program_id.push('X'),
            _ => unreachable!(),
        }
        assert!(
            build(&conn, &request, &policy).is_err(),
            "policy case {case}"
        );
        assert_eq!(conn.total_changes(), before);
    }
    let unsupported = XML.replace("project/20", "project/21");
    let (_dir, conn, request) = fixture(&unsupported);
    let mut policy = project_policy(&unsupported);
    policy.binding.schema_namespace = "http://knx.org/xml/project/21".into();
    assert!(build(&conn, &request, &policy)
        .unwrap_err()
        .to_string()
        .contains("schema binding"));
}

#[test]
fn project_experimental_union_overlay_retains_template_not_default_union_base() {
    let xml = project_union_xml();
    let (_dir, conn, request) = fixture(&xml);
    let result = knx_productdb::image::build_project_experimental_system2_image(
        &conn,
        &request,
        &project_policy(&xml),
    )
    .unwrap();
    assert_eq!(
        &result.image.segment("PAR").unwrap().octets[4..6],
        &[0xEE, 0xEF]
    );
    assert_eq!(
        result
            .image
            .parameter_fields
            .iter()
            .map(|f| f.reference.as_str())
            .collect::<Vec<_>>(),
        ["R", "UAR"]
    );
    assert_eq!(
        format!(
            "{:?}",
            result
                .coverage
                .iter()
                .find(|c| c.parameter_id == "UD")
                .unwrap()
                .disposition
        ),
        "RetainedSourceTemplate"
    );
}

#[test]
fn project_experimental_inactive_legacy_consensus_writes_and_reports_actual_patch() {
    let xml = inactive_xml().replace(
        r#"Id="Q" Name="Hidden""#,
        r#"Id="Q" Name="Hidden" LegacyPatchAlways="true""#,
    );
    let (_dir, conn, request) = fixture(&xml);
    let result = knx_productdb::image::build_project_experimental_system2_image(
        &conn,
        &request,
        &project_policy(&xml),
    )
    .unwrap();
    assert_eq!(&result.image.segment("PAR").unwrap().octets[2..4], &[3, 4]);
    assert_eq!(result.image.parameter_fields.len(), 1);
    assert_eq!(
        format!(
            "{:?}",
            result
                .coverage
                .iter()
                .find(|c| c.parameter_id == "Q")
                .unwrap()
                .disposition
        ),
        "LegacyPatchDefaultReference"
    );
    let xml = xml
        .replace(
            "</ParameterRefs>",
            r#"<ParameterRef Id="QR2" RefId="Q" Value="773" /></ParameterRefs>"#,
        )
        .replace(
            r#"<ParameterRefRef RefId="QR" />"#,
            r#"<ParameterRefRef RefId="QR" /><ParameterRefRef RefId="QR2" />"#,
        );
    let (_dir, conn, request) = fixture(&xml);
    let error = knx_productdb::image::build_project_experimental_system2_image(
        &conn,
        &request,
        &project_policy(&xml),
    )
    .unwrap_err();
    assert!(error.to_string().contains("references disagree"), "{error}");
}

#[test]
fn project_experimental_rejects_inactive_invalid_union_overlap_type_and_table_placements() {
    let sources = [
        project_union_xml().replace(
            r#"Id="UD" ParameterType="PT" Value="4660" Offset="0""#,
            r#"Id="UD" ParameterType="PT" Value="4660" Offset="1""#,
        ),
        project_union_xml().replace(
            "DefaultUnionParameter=\"true\"",
            "DefaultUnionParameter=\"false\"",
        ),
        project_union_xml().replace("SizeInBit=\"16\"><Memory", "SizeInBit=\"15\"><Memory"),
        inactive_xml().replace(
            r#"CodeSegment="PAR" Offset="2""#,
            r#"CodeSegment="PAR" Offset="0""#,
        ),
        inactive_xml().replace(
            r#"CodeSegment="PAR" Offset="2""#,
            r#"CodeSegment="GA" Offset="5""#,
        ),
        inactive_xml().replace(
            r#"CodeSegment="PAR" Offset="2""#,
            r#"CodeSegment="PAR" Offset="8""#,
        ),
        inactive_xml().replace(
            r#"Id="Q" Name="Hidden" ParameterType="PT" Value="258""#,
            r#"Id="Q" Name="Hidden" ParameterType="PT" Value="70000""#,
        ),
        inactive_xml().replace("<TypeNumber ", "<TypeFloat "),
        inactive_xml().replace("</Static>", "<UnknownMandatory /></Static>"),
    ];
    for (i, xml) in sources.iter().enumerate() {
        let (_dir, conn, request) = fixture(xml);
        let before = conn.total_changes();
        assert!(
            knx_productdb::image::build_project_experimental_system2_image(
                &conn,
                &request,
                &project_policy(xml)
            )
            .is_err(),
            "invalid alternative {i} accepted"
        );
        assert_eq!(conn.total_changes(), before);
    }
}

#[test]
fn project_experimental_exact_active_flags_no_alias_reassignment_and_standard_refusals() {
    use knx_productdb::image::{build_project_experimental_system2_image as build, FlagOverrides};
    let xml = explicit_background(XML).replace("</ComObjectRefs>", r#"<ComObjectRef Id="ALIAS" RefId="O" /></ComObjectRefs>"#)
        .replace("</ParameterBlock>", r#"<choose ParamRefId="R"><when test="0"><ComObjectRefRef RefId="ALIAS" /></when></choose></ParameterBlock>"#);
    let (_dir, conn, mut request) = fixture(&xml);
    request.flag_overrides.insert(
        "ALIAS".into(),
        FlagOverrides {
            write: Some(true),
            ..Default::default()
        },
    );
    assert!(build(&conn, &request, &project_policy(&xml))
        .unwrap_err()
        .to_string()
        .contains("exact active unscoped"));
    request.flag_overrides.clear();
    request
        .flag_overrides
        .insert("unknown".into(), FlagOverrides::default());
    assert!(
        build(&conn, &request, &project_policy(&xml)).is_ok(),
        "empty override is not a requested flag change"
    );
    request.flag_overrides.insert(
        "OR".into(),
        FlagOverrides {
            read_on_init: Some(true),
            ..Default::default()
        },
    );
    assert!(build(&conn, &request, &project_policy(&xml))
        .unwrap_err()
        .to_string()
        .contains("ReadOnInit"));
    let high = explicit_background(XML).replace(
        "ObjectSize=\"1 Byte\"",
        "ObjectSize=\"1 Byte\" Priority=\"High\"",
    );
    let (_dir, conn, request) = fixture(&high);
    assert!(build(&conn, &request, &project_policy(&high))
        .unwrap_err()
        .to_string()
        .contains("priority"));
}

fn complete_project_xml(namespace: u8, table_offset: usize, suffix: u8) -> String {
    use base64::Engine as _;
    let mut ga = vec![0x91; 20];
    ga[0] = 0xFE;
    ga[1] = 0;
    ga[19] = suffix;
    let aa = ga.clone();
    let b64 = |v: &[u8]| base64::engine::general_purpose::STANDARD.encode(v);
    let mut xml = inactive_xml()
        .replace("project/20", &format!("project/{namespace}"))
        .replace("Name=\"Synthetic\"", "Name=\"Synthetic\" PeiType=\"7\"")
        .replace(
            r#"Id="GA" Address="278" Size="16"><Data>AAAAAAAAAAAAAAAAAAAAAA==</Data>"#,
            &format!(
                r#"Id="GA" Address="278" Size="20"><Data>{}</Data>"#,
                b64(&ga)
            ),
        )
        .replace(
            r#"Id="AA" Address="608" Size="16"><Data>AAAAAAAAAAAAAAAAAAAAAA==</Data>"#,
            &format!(
                r#"Id="AA" Address="608" Size="20"><Data>{}</Data>"#,
                b64(&aa)
            ),
        )
        .replace(
            r#"<AddressTable CodeSegment="GA" Offset="0""#,
            &format!(r#"<AddressTable CodeSegment="GA" Offset="{table_offset}""#),
        )
        .replace(
            r#"<AssociationTable CodeSegment="AA" Offset="0""#,
            &format!(r#"<AssociationTable CodeSegment="AA" Offset="{table_offset}""#),
        );
    let a = xml.find("<LoadProcedures>").unwrap();
    let b = xml.find("</LoadProcedures>").unwrap() + "</LoadProcedures>".len();
    let allocate = |lsm, address, size| {
        format!(
            r#"<LdCtrlAbsSegment LsmIdx="{lsm}" SegType="0" Address="{address}" Size="{size}" Access="51" MemType="3" SegFlags="0" />"#
        )
    };
    let procedure = format!(
        r#"<LoadProcedures><LoadProcedure><LdCtrlConnect /><LdCtrlCompareProp ObjIdx="0" PropId="78" InlineData="000000000000" /><LdCtrlUnload LsmIdx="1" /><LdCtrlUnload LsmIdx="2" /><LdCtrlUnload LsmIdx="3" /><LdCtrlLoad LsmIdx="1" />{}<LdCtrlTaskSegment LsmIdx="1" Address="{}" /><LdCtrlLoad LsmIdx="2" />{}<LdCtrlTaskSegment LsmIdx="2" Address="{}" /><LdCtrlLoad LsmIdx="3" />{}{}<LdCtrlTaskSegment LsmIdx="3" Address="0" /><LdCtrlTaskCtrl2 LsmIdx="3" Callback="0" Address="1024" Seg0="0" Seg1="0" /><LdCtrlTaskSegment LsmIdx="5" Address="1024" /><LdCtrlLoadCompleted LsmIdx="1" /><LdCtrlLoadCompleted LsmIdx="2" /><LdCtrlLoadCompleted LsmIdx="3" /><LdCtrlDisconnect /></LoadProcedure></LoadProcedures>"#,
        allocate(1, 278, 20),
        278 + table_offset,
        allocate(2, 608, 20),
        608 + table_offset,
        allocate(3, 1024, 8),
        allocate(3, 15360, 8)
    );
    xml.replace_range(a..b, &procedure);
    xml
}

#[test]
fn project_experimental_generated_tables_feed_capacity_planner_ns20_and_ns23() {
    use knx_core::commissioning::property_download::PropertyOperation as Op;
    use knx_productdb::image::{
        build_project_experimental_system2_image as build, FlagOverrides, Link,
    };
    use knx_productdb::system2_procedure::{plan_system2_candidate, System2ProcedurePolicy};
    for namespace in [20, 23] {
        for offset in [0, 2] {
            for suffix in [0, 0xA5] {
                let xml = complete_project_xml(namespace, offset, suffix);
                let (_dir, conn, mut request) = fixture(&xml);
                let before = conn.total_changes();
                request.values.insert("R".into(), "22136".into());
                request.links.push(Link {
                    object: 1,
                    group_address: knx_core::GroupAddress::from_raw(0x3456),
                    sending: true,
                });
                request.flag_overrides.insert(
                    "OR".into(),
                    FlagOverrides {
                        write: Some(true),
                        ..Default::default()
                    },
                );
                let mut policy = project_policy(&xml);
                policy.binding.schema_namespace = format!("http://knx.org/xml/project/{namespace}");
                let result = build(&conn, &request, &policy).unwrap();
                assert_eq!(result.image.objects.len(), 1);
                assert!(result.image.objects[0].flags.write);
                assert_eq!(
                    &result.image.segment("PAR").unwrap().octets[..4],
                    &[0x56, 0x78, 0, 0]
                );
                assert_eq!(
                    result
                        .image
                        .table_writes
                        .iter()
                        .map(|s| (s.segment_id.as_str(), s.start, s.end))
                        .collect::<Vec<_>>(),
                    [("GA", offset, offset + 5), ("AA", offset, offset + 5)]
                );
                assert_eq!(
                    &result.image.segment("GA").unwrap().octets[offset..offset + 5],
                    &[2, 0x12, 0x34, 0x34, 0x56]
                );
                assert_eq!(
                    &result.image.segment("AA").unwrap().octets[offset..offset + 5],
                    &[2, 0xFE, 0, 1, 1]
                );
                assert!(knx_productdb::download_plan::plan_memory_download(&result.image).is_err());
                assert!(build_experimental_system2_image(&conn, &request, &policy).is_err());
                let plan = plan_system2_candidate(
                    &result.image,
                    request.individual_address,
                    &System2ProcedurePolicy::source_capacity_preserving_independent_data(),
                )
                .unwrap();
                assert_eq!(plan.source_positions(), 21);
                for (source_index, id, base, table) in [
                    (6, "GA", 278u16, vec![2, 0x12, 0x34, 0x34, 0x56]),
                    (9, "AA", 608, vec![2, 0xFE, 0, 1, 1]),
                ] {
                    let actual: Vec<_> = plan
                        .steps()
                        .iter()
                        .filter_map(|s| match &s.operation {
                            Op::WriteMemory { address, data, .. }
                                if s.source_index == source_index =>
                            {
                                Some((*address, data.clone()))
                            }
                            _ => None,
                        })
                        .collect();
                    let mut expected = vec![];
                    if offset > 0 {
                        let mut prefix = vec![0xFE, 0];
                        prefix.extend(table);
                        expected.push((base, prefix));
                    } else {
                        expected.push((base, table));
                    }
                    expected.push((
                        base + (offset + 15) as u16,
                        result.image.segment(id).unwrap().octets[offset + 15..].to_vec(),
                    ));
                    assert_eq!(
                        actual, expected,
                        "ns{namespace} offset{offset} suffix{suffix} {id}"
                    );
                }
                assert_eq!(conn.total_changes(), before);
            }
        }
    }
}

#[test]
fn project_experimental_real_generated_tables_preserve_active_parameter_after_reservation() {
    use knx_core::commissioning::property_download::PropertyOperation as Op;
    use knx_productdb::image::build_project_experimental_system2_image as build;
    use knx_productdb::system2_procedure::{plan_system2_candidate, System2ProcedurePolicy};
    let xml = complete_project_xml(20, 2, 0xA5).replace(
        r#"CodeSegment="PAR" Offset="0""#,
        r#"CodeSegment="GA" Offset="17""#,
    );
    let (_dir, conn, mut request) = fixture(&xml);
    request.values.insert("R".into(), "22136".into());
    request.links.push(knx_productdb::image::Link {
        object: 1,
        group_address: knx_core::GroupAddress::from_raw(0x3456),
        sending: true,
    });
    let result = build(&conn, &request, &project_policy(&xml)).unwrap();
    assert_eq!(
        &result.image.segment("GA").unwrap().octets[17..],
        &[0x56, 0x78, 0xA5]
    );
    let plan = plan_system2_candidate(
        &result.image,
        request.individual_address,
        &System2ProcedurePolicy::source_capacity_preserving_independent_data(),
    )
    .unwrap();
    let writes: Vec<_> = plan
        .steps()
        .iter()
        .filter_map(|s| match &s.operation {
            Op::WriteMemory { address, data, .. } if s.source_index == 6 => {
                Some((*address, data.clone()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        writes,
        [
            (278, vec![0xFE, 0, 2, 0x12, 0x34, 0x34, 0x56]),
            (295, vec![0x56, 0x78, 0xA5])
        ]
    );
    for overlap in [7, 16] {
        let bad = xml.replace(
            r#"CodeSegment="GA" Offset="17""#,
            &format!(r#"CodeSegment="GA" Offset="{overlap}""#),
        );
        let (_dir, conn, request) = fixture(&bad);
        assert!(build(&conn, &request, &project_policy(&bad))
            .unwrap_err()
            .to_string()
            .contains("reserved table"));
    }
}

#[test]
fn project_experimental_valid_synthetic_package_ownership_is_pinned() {
    use std::io::{Cursor, Write};
    let xml = complete_project_xml(23, 0, 0);
    let mut archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
    archive
        .start_file("knx_master.xml", zip::write::SimpleFileOptions::default())
        .unwrap();
    archive.write_all(br#"<KNX xmlns="http://knx.org/xml/project/23"><MasterData><Manufacturers><Manufacturer Id="M-0000" Name="Synthetic" /></Manufacturers></MasterData></KNX>"#).unwrap();
    archive
        .start_file(
            "M-0000/M-0000_A-0001-01-0000.xml",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
    archive.write_all(xml.as_bytes()).unwrap();
    let bytes = archive.finish().unwrap().into_inner();
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    knx_productdb::install_package(&conn, "synthetic.knxprod", &bytes).unwrap();
    let mut p = project_policy(&xml);
    p.binding.schema_namespace = "http://knx.org/xml/project/23".into();
    p.binding.package_sha256 = Some(knx_productdb::sha256_hex(&bytes));
    let request = ImageRequest {
        program_id: ID.into(),
        individual_address: IndividualAddress::from_raw(0x1234),
        values: BTreeMap::new(),
        links: vec![],
        flag_overrides: BTreeMap::new(),
    };
    let before = conn.total_changes();
    let result =
        knx_productdb::image::build_project_experimental_system2_image(&conn, &request, &p)
            .unwrap();
    assert_eq!(result.image.segments.len(), 4);
    assert_eq!(conn.total_changes(), before);
    // The revision1 entry must still reject namespace23 even without overrides.
    let mut old = p.clone();
    old.revision = 1;
    old.invisible_parameters = policy(XML).invisible_parameters;
    assert!(build_experimental_system2_image(&conn, &request, &old)
        .unwrap_err()
        .to_string()
        .contains("schema binding"));
}

#[test]
fn explicit_binary_restriction_policy_uses_payload_not_enumeration_index() {
    let xml = XML.replace("<TypeNumber SizeInBit=\"16\" Type=\"unsignedInt\" minInclusive=\"0\" maxInclusive=\"65535\" />", "<TypeRestriction Base=\"BinaryValue\" SizeInBit=\"8\"><Enumeration Id=\"E\" Value=\"0\" Text=\"six\" BinaryValue=\"Bg==\" /></TypeRestriction>")
        .replace("Value=\"4660\"", "Value=\"0\"");
    let (_dir, conn, request) = fixture(&xml);
    let result = build_experimental_system2_image(&conn, &request, &policy(&xml)).unwrap();
    assert_eq!(result.image.segment("PAR").unwrap().octets[0], 6);
}

fn fixture(xml: &str) -> (tempfile::TempDir, knx_productdb::Connection, ImageRequest) {
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    knx_productdb::ingest_file(&conn, "synthetic.xml", xml.as_bytes()).unwrap();
    let request = ImageRequest {
        program_id: ID.into(),
        individual_address: IndividualAddress::from_raw(0x1234),
        values: BTreeMap::new(),
        links: vec![],
        flag_overrides: BTreeMap::new(),
    };
    (dir, conn, request)
}

#[test]
fn explicit_experimental_numeric_rule_writes_nonmatching_multioctet_default() {
    let (_dir, conn, request) = fixture(XML);
    let before = conn.total_changes();
    assert!(build_system2_image(&conn, &request).is_err());
    let result = build_experimental_system2_image(&conn, &request, &policy(XML)).unwrap();
    assert_eq!(
        result.image.segment("PAR").unwrap().octets,
        [0x12, 0x34, 0, 0, 0, 0, 0, 0]
    );
    assert_eq!(result.image.segments.len(), 4);
    assert_eq!(result.coverage.len(), 1);
    assert_eq!(result.coverage[0].parameter_id, "P");
    assert_eq!(result.coverage[0].active_reference.as_deref(), Some("R"));
    assert_eq!(result.coverage[0].field.unwrap().size_in_bit, 16);
    assert!(result
        .image
        .inferences
        .iter()
        .any(|i| i.rule == "experimental-system2-numeric-order"));
    assert!(knx_productdb::download_plan::plan_memory_download(&result.image).is_err());
    assert_eq!(conn.total_changes(), before);
}

fn project_policy(xml: &str) -> ExperimentalSystem2ImagePolicy {
    let mut p = policy(xml);
    p.revision = 2;
    p.invisible_parameters = "source-template-inactive-legacy-patch-active-overlay-v1".into();
    p
}

#[test]
fn project_experimental_retains_inactive_source_template() {
    let xml = inactive_xml().replace("<Data>AAAAAAAAAAA=</Data>", "<Data>qrvM3e7/ABI=</Data>");
    let (_dir, conn, mut request) = fixture(&xml);
    request.values.insert("R".into(), "22136".into());
    let result = knx_productdb::image::build_project_experimental_system2_image(
        &conn,
        &request,
        &project_policy(&xml),
    )
    .expect("project experimental image must retain source background");
    assert_eq!(
        result.image.segment("PAR").unwrap().octets,
        [0x56, 0x78, 0xCC, 0xDD, 0xEE, 0xFF, 0, 0x12]
    );
    assert_eq!(result.image.parameter_fields.len(), 1);
    assert_eq!(
        format!(
            "{:?}",
            result
                .coverage
                .iter()
                .find(|c| c.parameter_id == "Q")
                .unwrap()
                .disposition
        ),
        "RetainedSourceTemplate"
    );
}

#[test]
fn project_experimental_refuses_unconsumed_nonempty_flag_override() {
    use knx_productdb::image::{build_project_experimental_system2_image as build, FlagOverrides};
    let (_dir, conn, mut request) = fixture(XML);
    request.flag_overrides.insert(
        "unknown".into(),
        FlagOverrides {
            write: Some(true),
            ..Default::default()
        },
    );
    let error = build(&conn, &request, &project_policy(XML))
        .expect_err("unknown override must not disappear");
    assert!(
        error.to_string().contains("exact active unscoped"),
        "{error}"
    );
}

#[test]
fn project_experimental_validates_inactive_alternative_masks() {
    // Q is never selected, but its default cannot alter protected source bits.
    let xml = inactive_xml().replace("//////////8=", "//8A//////8=");
    let (_dir, conn, request) = fixture(&xml);
    let error = knx_productdb::image::build_project_experimental_system2_image(
        &conn,
        &request,
        &project_policy(&xml),
    )
    .expect_err("inactive alternatives still require mask validation");
    assert!(error.to_string().contains("octet 2"), "{error}");
}

#[test]
fn project_experimental_validates_requested_binary_payload_using_policy() {
    let xml = explicit_background(XML).replace("<TypeNumber SizeInBit=\"16\" Type=\"unsignedInt\" minInclusive=\"0\" maxInclusive=\"65535\" />", "<TypeRestriction Base=\"BinaryValue\" SizeInBit=\"8\"><Enumeration Id=\"E\" Value=\"0\" Text=\"six\" BinaryValue=\"Bg==\" /><Enumeration Id=\"E2\" Value=\"1\" Text=\"nine\" BinaryValue=\"CQ==\" /></TypeRestriction>")
        .replace("Value=\"4660\"", "Value=\"0\"");
    let (_dir, conn, mut request) = fixture(&xml);
    request.values.insert("R".into(), "1".into());
    let result = knx_productdb::image::build_project_experimental_system2_image(
        &conn,
        &request,
        &project_policy(&xml),
    )
    .expect("requested binary enumeration must use selected payload");
    assert_eq!(result.image.segment("PAR").unwrap().octets[0], 9);
}

#[test]
fn project_experimental_validates_requested_inactive_field_width() {
    let xml = inactive_xml()
        .replace("SizeInBit=\"16\"", "SizeInBit=\"8\"")
        .replace("Value=\"4660\"", "Value=\"1\"")
        .replace("Value=\"258\"", "Value=\"2\"")
        .replace("Value=\"772\"", "Value=\"3\"");
    let (_dir, conn, mut request) = fixture(&xml);
    request.values.insert("QR".into(), "256".into());
    let error = knx_productdb::image::build_project_experimental_system2_image(
        &conn,
        &request,
        &project_policy(&xml),
    )
    .expect_err("requested inactive values must fit actual field width");
    assert!(error.to_string().contains("QR"), "{error}");
}

fn inactive_xml() -> String {
    XML.replace("<Static>", "<Static><Options DownloadInvisibleParameters=\"Background\" />")
        .replace("</Parameters>", "<Parameter Id=\"Q\" Name=\"Hidden\" ParameterType=\"PT\" Value=\"258\"><Memory CodeSegment=\"PAR\" Offset=\"2\" BitOffset=\"0\" /></Parameter></Parameters>")
        .replace("</ParameterRefs>", "<ParameterRef Id=\"QR\" RefId=\"Q\" Value=\"772\" /></ParameterRefs>")
        .replace("</ParameterBlock>", "<choose ParamRefId=\"R\"><when test=\"0\"><ParameterRefRef RefId=\"QR\" /></when></choose></ParameterBlock>")
}

#[test]
fn background_writes_inactive_parameter_default_not_reference_override() {
    let xml = inactive_xml();
    let (_dir, conn, request) = fixture(&xml);
    let result = build_experimental_system2_image(&conn, &request, &policy(&xml)).unwrap();
    assert_eq!(
        result.image.segment("PAR").unwrap().octets,
        [0x12, 0x34, 1, 2, 0, 0, 0, 0]
    );
}

#[test]
fn legacy_patch_always_uses_inactive_reference_default_and_discloses_hypothesis() {
    let xml = inactive_xml().replace(
        "Id=\"Q\" Name=\"Hidden\"",
        "Id=\"Q\" Name=\"Hidden\" LegacyPatchAlways=\"true\"",
    );
    let (_dir, conn, request) = fixture(&xml);
    let result = build_experimental_system2_image(&conn, &request, &policy(&xml)).unwrap();
    assert_eq!(&result.image.segment("PAR").unwrap().octets[2..4], &[3, 4]);
    assert!(result
        .image
        .inferences
        .iter()
        .any(|i| i.rule == "experimental-system2-legacy-patch-always"));
    assert!(build_system2_image(&conn, &request).is_err());
}

#[test]
fn default_union_base_then_active_subfield_overlay_preserves_other_union_bits() {
    let xml = XML.replace("</ParameterTypes>", "<ParameterType Id=\"PT3\"><TypeNumber SizeInBit=\"3\" Type=\"unsignedInt\" minInclusive=\"0\" maxInclusive=\"7\" /></ParameterType></ParameterTypes>")
        .replace("</Parameters>", "<Union SizeInBit=\"16\"><Memory CodeSegment=\"PAR\" Offset=\"4\" BitOffset=\"0\" /><Parameter Id=\"UD\" ParameterType=\"PT\" Value=\"4660\" Offset=\"0\" BitOffset=\"0\" DefaultUnionParameter=\"true\" /><Parameter Id=\"UA\" ParameterType=\"PT3\" Value=\"5\" Offset=\"1\" BitOffset=\"2\" /></Union></Parameters>")
        .replace("</ParameterRefs>", "<ParameterRef Id=\"UDR\" RefId=\"UD\" /><ParameterRef Id=\"UAR\" RefId=\"UA\" /></ParameterRefs>")
        .replace("</ParameterBlock>", "<ParameterRefRef RefId=\"UAR\" /><choose ParamRefId=\"R\"><when test=\"0\"><ParameterRefRef RefId=\"UDR\" /></when></choose></ParameterBlock>");
    let (_dir, conn, request) = fixture(&xml);
    let result = build_experimental_system2_image(&conn, &request, &policy(&xml)).unwrap();
    assert_eq!(
        &result.image.segment("PAR").unwrap().octets[4..6],
        &[0x12, 0x2C]
    );
    assert!(result
        .image
        .inferences
        .iter()
        .any(|i| i.rule == "experimental-system2-background-union-overlay"));
}

#[test]
fn non_byte_aligned_union_is_outside_the_explicit_offset_policy() {
    let xml = XML.replace("<Parameters>", "<Parameters><Union SizeInBit=\"16\"><Memory CodeSegment=\"PAR\" Offset=\"0\" BitOffset=\"1\" />")
        .replace("<Memory CodeSegment=\"PAR\" Offset=\"0\" BitOffset=\"0\" />", "")
        .replace("Value=\"4660\"", "Value=\"4660\" Offset=\"0\" BitOffset=\"0\" DefaultUnionParameter=\"true\"")
        .replace("</Parameter></Parameters>", "</Parameter></Union></Parameters>");
    let (_dir, conn, request) = fixture(&xml);
    let error = match build_experimental_system2_image(&conn, &request, &policy(&xml)) {
        Err(error) => error,
        Ok(_) => panic!("non-byte-aligned union was accepted outside policy"),
    };
    assert!(error.to_string().contains("byte-aligned union"));
}

#[test]
fn active_legacy_reference_wins_over_disagreeing_inactive_alias_defaults() {
    let xml = XML.replace("Id=\"P\" Name=\"Level\"", "Id=\"P\" Name=\"Level\" LegacyPatchAlways=\"true\"")
        .replace("</ParameterRefs>", "<ParameterRef Id=\"R2\" RefId=\"P\" Value=\"772\" /></ParameterRefs>")
        .replace("</ParameterBlock>", "<choose ParamRefId=\"R\"><when test=\"0\"><ParameterRefRef RefId=\"R2\" /></when></choose></ParameterBlock>");
    let (_dir, conn, request) = fixture(&xml);
    let result = build_experimental_system2_image(&conn, &request, &policy(&xml)).unwrap();
    assert_eq!(
        &result.image.segment("PAR").unwrap().octets[..2],
        &[0x12, 0x34]
    );
}

#[test]
fn two_active_references_to_one_parameter_refuse_even_when_values_agree() {
    let xml = XML
        .replace(
            "</ParameterRefs>",
            "<ParameterRef Id=\"R2\" RefId=\"P\" /></ParameterRefs>",
        )
        .replace(
            "</ParameterBlock>",
            "<ParameterRefRef RefId=\"R2\" /></ParameterBlock>",
        );
    let (_dir, conn, request) = fixture(&xml);
    let error = match build_experimental_system2_image(&conn, &request, &policy(&xml)) {
        Err(error) => error,
        Ok(_) => panic!("two active references were ambiguously accepted"),
    };
    assert!(error.to_string().contains("multiple active references"));
}

#[test]
fn reference_with_no_dynamic_occurrence_is_implicitly_active() {
    let xml = inactive_xml().replace("<choose ParamRefId=\"R\"><when test=\"0\"><ParameterRefRef RefId=\"QR\" /></when></choose>", "");
    let (_dir, conn, request) = fixture(&xml);
    let result = build_experimental_system2_image(&conn, &request, &policy(&xml)).unwrap();
    assert_eq!(&result.image.segment("PAR").unwrap().octets[2..4], &[3, 4]);
    assert_eq!(result.image.parameters.get("QR"), Some(&772));
}

#[test]
fn unknown_policy_source_bindings_and_external_configuration_refuse() {
    let (_dir, conn, request) = fixture(XML);
    let before = conn.total_changes();
    for mutate in 0..12 {
        let mut candidate = policy(XML);
        match mutate {
            0 => candidate.revision = 2,
            1 => candidate.numeric_order.clear(),
            2 => candidate.bit_union_offsets.clear(),
            3 => candidate.legacy_patch_always.clear(),
            4 => candidate.invisible_parameters.clear(),
            5 => candidate.binary_restrictions.clear(),
            6 => candidate.binding.program_id.push('X'),
            7 => candidate.binding.source_sha256 = "0".repeat(64),
            8 => candidate.binding.schema_namespace.push('X'),
            9 => candidate.binding.package_sha256 = Some("0".repeat(64)),
            10 => candidate.reference_activation.clear(),
            11 => candidate.reference_activation = "unknown-activation".into(),
            _ => unreachable!(),
        }
        assert!(
            build_experimental_system2_image(&conn, &request, &candidate).is_err(),
            "policy mutation {mutate}"
        );
        assert_eq!(
            conn.total_changes(),
            before,
            "refused policy must not mutate the product database"
        );
    }
    let mut external = request.clone();
    external.values.insert("R".into(), "4660".into());
    assert!(build_experimental_system2_image(&conn, &external, &policy(XML)).is_err());
}

#[test]
fn unsupported_source_controls_types_overlap_and_masks_never_return_partial_images() {
    let sources = [
        XML.replace(
            "<Static>",
            "<Static><Options DownloadInvisibleParameters=\"Never\" />",
        ),
        XML.replace(
            "<Static>",
            "<Static><Options ParameterByteOrder=\"LittleEndian\" />",
        ),
        XML.replace(
            "CodeSegment=\"PAR\" Offset=\"0\"",
            "Future=\"1\" CodeSegment=\"PAR\" Offset=\"0\"",
        ),
        XML.replace(
            "<Parameter Id=\"P\"",
            "<Parameter xmlns=\"urn:unknown\" Id=\"P\"",
        ),
        XML.replace("<TypeNumber ", "<TypeFloat "),
        XML.replace(
            "<Memory CodeSegment=\"PAR\"",
            "<Property CodeSegment=\"PAR\"",
        ),
        XML.replace("</Static>", "<UnknownMandatory /><!----></Static>"),
        XML.replace("//////////8=", "AAAAAAAAAAA="),
        inactive_xml().replace("Offset=\"2\"", "Offset=\"0\""),
        inactive_xml().replace(
            "Id=\"Q\" Name=\"Hidden\"",
            "Id=\"Q\" Name=\"Hidden\" LegacyPatchAlways=\"invalid\"",
        ),
    ];
    for (index, xml) in sources.iter().enumerate() {
        let (_dir, conn, request) = fixture(xml);
        let before = conn.total_changes();
        assert!(
            build_experimental_system2_image(&conn, &request, &policy(xml)).is_err(),
            "unsupported source {index}"
        );
        assert_eq!(conn.total_changes(), before);
    }
}

#[test]
fn explicit_big_endian_and_default_value_policy_use_the_same_bounded_codec() {
    let xml = XML.replace("<Static>", "<Static><Options ParameterByteOrder=\"BigEndian\" DownloadInvisibleParameters=\"DefaultValue\" />");
    let (_dir, conn, request) = fixture(&xml);
    let result = build_experimental_system2_image(&conn, &request, &policy(&xml)).unwrap();
    assert_eq!(
        &result.image.segment("PAR").unwrap().octets[..2],
        &[0x12, 0x34]
    );
}

#[test]
fn nested_application_declarations_are_not_an_ignored_extension() {
    let xml = XML.replace("</Static>", "<ApplicationProgram Id=\"OTHER\" /></Static>");
    let (_dir, conn, request) = fixture(&xml);
    assert!(build_experimental_system2_image(&conn, &request, &policy(&xml)).is_err());
}

#[test]
fn unexpected_source_text_is_not_discarded() {
    let xml = XML.replace(
        "<Static>",
        "<Static><Options>uninterpreted image data</Options>",
    );
    let (_dir, conn, request) = fixture(&xml);
    let error = match build_experimental_system2_image(&conn, &request, &policy(&xml)) {
        Err(error) => error,
        Ok(_) => panic!("unexpected source text was discarded"),
    };
    assert!(error.to_string().contains("source text"));
}

#[test]
fn unknown_image_option_is_not_silently_discarded() {
    let xml = XML.replace("<Static>", "<Static><Options FutureImageRule=\"true\" />");
    let (_dir, conn, request) = fixture(&xml);
    let error = match build_experimental_system2_image(&conn, &request, &policy(&xml)) {
        Err(error) => error,
        Ok(_) => panic!("unknown mandatory option was silently ignored"),
    };
    assert!(error.to_string().contains("FutureImageRule"));
}

#[test]
#[ignore = "requires digest-pinned inert public package; KNXBENCH_PUBLIC_UP258"]
fn public_default_complete_candidate_exercises_real_installer_builder_and_resolver() {
    let bytes = std::fs::read(std::env::var_os("KNXBENCH_PUBLIC_UP258").unwrap()).unwrap();
    assert_eq!(
        knx_productdb::sha256_hex(&bytes),
        "520d3fcee5b63d079ac7d0bd5e7ce9467ff84cb5cde06ac341b77aba47e3ee0f"
    );
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    knx_productdb::install_package_with_limits(
        &conn,
        "public.knxprod",
        &bytes,
        knx_productdb::PackageLimits::LARGE,
    )
    .unwrap();
    let mut policy = policy(XML);
    policy.binding.program_id = "M-0001_A-9205-04-9270".into();
    policy.binding.source_sha256 =
        "d06a04eb4434684762f276e6fce61fbae6056f94f9076f5fcf5b7a314bb0a746".into();
    policy.binding.package_sha256 =
        Some("520d3fcee5b63d079ac7d0bd5e7ce9467ff84cb5cde06ac341b77aba47e3ee0f".into());
    let request = ImageRequest {
        program_id: policy.binding.program_id.clone(),
        individual_address: IndividualAddress::from_raw(0x1234),
        values: BTreeMap::new(),
        links: vec![],
        flag_overrides: BTreeMap::new(),
    };
    let before = conn.total_changes();
    let result = build_experimental_system2_image(&conn, &request, &policy).unwrap();
    assert_eq!(
        result
            .image
            .segments
            .iter()
            .map(|s| (s.address, s.octets.len()))
            .collect::<Vec<_>>(),
        [(0x0116, 330), (0x0260, 416), (0x0400, 512), (0x3C00, 256)]
    );
    assert_eq!(result.coverage.len(), 550);
    assert_eq!(
        result.coverage.iter().filter(|c| c.field.is_some()).count(),
        339
    );
    assert_eq!(
        result
            .coverage
            .iter()
            .filter(|c| c.default_union_parameter)
            .count(),
        33
    );
    let mut statement = conn
        .prepare("SELECT id FROM parameter WHERE program_id = ?1 ORDER BY id")
        .unwrap();
    let source_ids = statement
        .query_map([&request.program_id], |r| r.get::<_, String>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(
        result
            .coverage
            .iter()
            .map(|c| c.parameter_id.clone())
            .collect::<Vec<_>>(),
        source_ids
    );
    for segment in &result.image.segments {
        assert_eq!(
            segment.mask,
            result.image.code.segment(&segment.id).unwrap().mask
        );
    }
    let parameters = result
        .image
        .segment("M-0001_A-9205-04-9270_AS-3C00")
        .unwrap();
    assert_eq!(&parameters.octets[240..242], &[0, 120]);
    assert_eq!(&parameters.octets[248..250], &[7, 8]);
    assert_eq!(parameters.octets[250], 3);
    if let Some(path) = std::env::var_os("KNXBENCH_EXPERIMENTAL_IMAGE_OUTPUT") {
        let segments = result.image.segments.iter().map(|s| serde_json::json!({"id":s.id,"address":s.address,"octets":s.octets,"mask":s.mask,"sha256":knx_productdb::sha256_hex(&s.octets)})).collect::<Vec<_>>();
        let coverage = result.coverage.iter().map(|c| serde_json::json!({"parameter_id":c.parameter_id,"references":c.references,"source_default":c.source_default,"segment":c.segment,"field":c.field.map(|f|serde_json::json!({"offset":f.offset,"bit_offset":f.bit_offset,"size_in_bit":f.size_in_bit})),"legacy_patch_always":c.legacy_patch_always,"default_union_parameter":c.default_union_parameter,"active_reference":c.active_reference,"disposition":format!("{:?}",c.disposition)})).collect::<Vec<_>>();
        let inferences = result
            .image
            .inferences
            .iter()
            .map(|i| serde_json::json!({"rule":i.rule,"detail":i.detail,"reference":i.reference}))
            .collect::<Vec<_>>();
        std::fs::write(path, serde_json::to_vec_pretty(&serde_json::json!({"classification":"STRONG HYPOTHESIS; offline image only; no hardware validation", "segments":segments,"coverage":coverage,"inferences":inferences})).unwrap()).unwrap();
    }
    assert_eq!(conn.total_changes(), before);
    assert!(build_system2_image(&conn, &request).is_err());
    assert!(knx_productdb::download_plan::plan_memory_download(&result.image).is_err());
}
