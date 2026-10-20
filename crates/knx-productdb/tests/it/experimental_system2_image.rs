//! Explicit experimental image rules are isolated from ordinary admission.

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
