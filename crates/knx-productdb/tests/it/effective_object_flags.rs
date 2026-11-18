//! Checks shared flag resolution against activation and literal Easy3 image bytes.

use std::collections::BTreeMap;

use knx_core::commissioning::group_object_table::GroupObjectTableError;
use knx_core::IndividualAddress;
use knx_productdb::image::{
    build_download_image, resolve_effective_object_flags, EffectiveObjectFlags, FlagOverrides,
    ImageError, ImageRequest, READ_ON_INIT_NOT_ON_070N,
};

const ID: &str = "M-0001_A-0001-01-0000";

type Fixture = fn(&str) -> (tempfile::TempDir, knx_productdb::Connection, ImageRequest);
type Builder = fn(
    &knx_productdb::Connection,
    &ImageRequest,
) -> Result<knx_productdb::image::DownloadImage, ImageError>;

// Run the same public contract on each family's existing admitted fixture.
pub(super) fn check_legacy_flags(
    xml: &str,
    fixture: Fixture,
    build: Builder,
    segment: &str,
    config_at: usize,
    (baseline, changed, has_update): (u8, u8, bool),
) {
    let (_dir, conn, mut request) = fixture(xml);
    let original = build(&conn, &request).unwrap();
    assert_eq!(
        original.segment(segment).unwrap().octets[config_at],
        baseline
    );
    assert_eq!(resolve(&conn, &request).unwrap(), [(1, DEFAULT0)].into());
    request.flag_overrides.insert(
        "O_R".into(),
        FlagOverrides {
            read: Some(true),
            write: Some(false),
            transmit: Some(true),
            update: Some(false),
            communication: Some(true),
            read_on_init: None,
        },
    );
    // A statement equal to the product and an inactive override change no byte.
    request.flag_overrides.insert(
        "INACTIVE".into(),
        FlagOverrides {
            read_on_init: Some(true),
            ..Default::default()
        },
    );
    assert_eq!(resolve(&conn, &request).unwrap(), [(1, DEFAULT0)].into());
    assert_eq!(build(&conn, &request).unwrap(), original);
    request.flag_overrides.insert(
        "O_R".into(),
        FlagOverrides {
            read: Some(false),
            write: Some(true),
            transmit: Some(false),
            update: Some(false),
            communication: Some(false),
            read_on_init: None,
        },
    );
    assert_eq!(
        resolve(&conn, &request).unwrap(),
        [(
            1,
            EffectiveObjectFlags {
                read: false,
                write: true,
                transmit: false,
                update: false,
                communication: false,
            }
        )]
        .into()
    );
    assert_eq!(
        build(&conn, &request)
            .unwrap()
            .segment(segment)
            .unwrap()
            .octets[config_at],
        changed
    );
    request.flag_overrides.get_mut("O_R").unwrap().update = Some(true);
    assert!(resolve(&conn, &request).unwrap()[&1].update);
    if has_update {
        assert_eq!(
            build(&conn, &request)
                .unwrap()
                .segment(segment)
                .unwrap()
                .octets[config_at],
            0xB3
        );
    } else {
        assert!(
            matches!(build(&conn, &request), Err(ImageError::Unsupported { ref what }) if what == "legacy Type-1 object 1 cannot enable Update"),
            "Type1 still refuses update"
        );
    }
    let xml = xml.replace(
        "<ComObjectRef Id=\"O_R\" RefId=\"O\" />",
        "<ComObjectRef Id=\"O_R\" RefId=\"O\" WriteFlag=\"Enabled\" />",
    );
    assert!(xml.contains("RefId=\"O\" WriteFlag=\"Enabled\""));
    let (_dir, conn, mut request) = fixture(&xml);
    assert_eq!(
        resolve(&conn, &request).unwrap(),
        [(
            1,
            EffectiveObjectFlags {
                write: true,
                ..DEFAULT0
            }
        )]
        .into()
    );
    let expected = if has_update { 0x7F } else { 0xFF };
    assert_eq!(
        build(&conn, &request)
            .unwrap()
            .segment(segment)
            .unwrap()
            .octets[config_at],
        expected
    );
    request.flag_overrides.insert(
        "O_R".into(),
        FlagOverrides {
            write: Some(false),
            ..Default::default()
        },
    );
    assert_eq!(resolve(&conn, &request).unwrap(), [(1, DEFAULT0)].into());
    assert_eq!(
        build(&conn, &request)
            .unwrap()
            .segment(segment)
            .unwrap()
            .octets[config_at],
        baseline
    );
}

pub(super) fn check_legacy_activation_and_read_on_init(
    xml: &str,
    fixture: Fixture,
    build: Builder,
    segment: &str,
    config_at: usize,
    baseline: u8,
    family: &str,
) {
    for product_enabled in [false, true] {
        let xml = if product_enabled {
            xml.replace("ReadOnInitFlag=\"Disabled\"", "ReadOnInitFlag=\"Enabled\"")
        } else {
            xml.to_owned()
        };
        assert!(xml.contains(if product_enabled {
            "ReadOnInitFlag=\"Enabled\""
        } else {
            "ReadOnInitFlag=\"Disabled\""
        }));
        let (_dir, conn, mut request) = fixture(&xml);
        for requested in [None, Some(false), Some(true)] {
            request.flag_overrides.insert(
                "O_R".into(),
                FlagOverrides {
                    read_on_init: requested,
                    ..Default::default()
                },
            );
            assert_eq!(resolve(&conn, &request).unwrap(), [(1, DEFAULT0)].into());
            if requested.unwrap_or(product_enabled) {
                assert!(
                    matches!(build(&conn, &request), Err(ImageError::Unsupported { ref what }) if what == &format!("O_R: {family} ReadOnInit encoding is not established"))
                );
            } else {
                let image = build(&conn, &request).unwrap();
                assert_eq!(image.segment(segment).unwrap().octets[config_at], baseline);
                assert!(!image
                    .inferences
                    .iter()
                    .any(|i| i.rule == READ_ON_INIT_NOT_ON_070N));
            }
        }
    }
    let empty = xml.replace("<ComObjectRefRef RefId=\"O_R\" />", "");
    assert!(!empty.contains("<ComObjectRefRef RefId=\"O_R\" />"));
    let (_dir, conn, mut request) = fixture(&empty);
    request.links.clear();
    request.flag_overrides.insert(
        "O_R".into(),
        FlagOverrides {
            read_on_init: Some(true),
            ..Default::default()
        },
    );
    assert!(resolve(&conn, &request).unwrap().is_empty());
    let image = build(&conn, &request).unwrap();
    assert!(image.objects.is_empty());
    let expected = if family == "System1" { 0xE1 } else { 0xA1 };
    assert_eq!(image.segment(segment).unwrap().octets[config_at], expected);
    let uncertain = xml.replace("<ComObjectRefRef RefId=\"O_R\" />", "<choose ParamRefId=\"MISSING\"><when test=\"1\"><ComObjectRefRef RefId=\"O_R\" /></when></choose>");
    assert!(uncertain.contains("<choose ParamRefId=\"MISSING\">"));
    let (_dir, conn, request) = fixture(&uncertain);
    assert!(matches!(
        resolve(&conn, &request),
        Err(ImageError::Evaluation(_))
    ));
    assert!(matches!(
        build(&conn, &request),
        Err(ImageError::Evaluation(_))
    ));
    let duplicate = xml
        .replace(
            "</ComObjectRefs>",
            "<ComObjectRef Id=\"ALIAS\" RefId=\"O\" /></ComObjectRefs>",
        )
        .replace(
            "<ComObjectRefRef RefId=\"O_R\" />",
            "<ComObjectRefRef RefId=\"O_R\" /><ComObjectRefRef RefId=\"ALIAS\" />",
        );
    assert!(duplicate.contains("<ComObjectRefRef RefId=\"ALIAS\" />"));
    let (_dir, conn, request) = fixture(&duplicate);
    assert!(matches!(
        resolve(&conn, &request),
        Err(ImageError::GroupObjects(
            GroupObjectTableError::DuplicateObject { number: 1 }
        ))
    ));
    assert!(
        matches!(build(&conn, &request), Err(ImageError::Unsupported { ref what }) if what == "group object 1 is listed twice")
    );
    let scoped = xml.replace("</Static><Dynamic>", "<ModuleDefs><ModuleDef Id=\"MD\"><Static /><Dynamic><ComObjectRefRef RefId=\"O_R\" /></Dynamic></ModuleDef></ModuleDefs></Static><Dynamic><Module Id=\"MI\" RefId=\"MD\" />");
    assert!(scoped.contains("<Module Id=\"MI\" RefId=\"MD\" />"));
    let (_dir, conn, request) = fixture(&scoped);
    for result in [
        resolve(&conn, &request).map(|_| ()),
        build(&conn, &request).map(|_| ()),
    ] {
        assert!(
            matches!(result, Err(ImageError::Unsupported { ref what }) if what == "O_R: a module instance"),
            "{result:?}"
        );
    }
}
// The object segment and descriptor layout match image.rs's synthetic Easy3
// fixture. Mode 3 is legal but uncovered; that is known-empty, not uncertainty.
const PROGRAM: &str = r#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0001">
<ApplicationPrograms><ApplicationProgram Id="M-0001_A-0001-01-0000" Name="Synthetic" ApplicationNumber="1"
ApplicationVersion="1" MaskVersion="MV-0701" LoadProcedureStyle="ProductProcedure"><Static>
<Code><AbsoluteSegment Id="S" Address="17408" Size="16"><Data>AgAAB0BMAAdBbAeqAAu4AA==</Data></AbsoluteSegment></Code>
<ParameterTypes><ParameterType Id="PT" Name="Mode"><TypeNumber SizeInBit="8" Type="unsignedInt" minInclusive="0" maxInclusive="3" /></ParameterType></ParameterTypes>
<Parameters><Parameter Id="P" Name="Mode" ParameterType="PT" Value="1"><Memory CodeSegment="S" Offset="11" BitOffset="0" /></Parameter></Parameters>
<ParameterRefs><ParameterRef Id="P_R" RefId="P" /></ParameterRefs>
<ComObjectTable CodeSegment="S" Offset="0">
<ComObject Id="O0" Number="0" ObjectSize="1 Bit" ReadFlag="Enabled" WriteFlag="Disabled" CommunicationFlag="Enabled" TransmitFlag="Enabled" UpdateFlag="Disabled" ReadOnInitFlag="Disabled" />
<ComObject Id="O1" Number="1" ObjectSize="1 Byte" ReadFlag="Disabled" WriteFlag="Disabled" CommunicationFlag="Enabled" TransmitFlag="Disabled" UpdateFlag="Enabled" ReadOnInitFlag="Disabled" />
</ComObjectTable><ComObjectRefs>
<ComObjectRef Id="O0_R" RefId="O0" />
<ComObjectRef Id="O1_R" RefId="O1" WriteFlag="Enabled" />
</ComObjectRefs></Static><Dynamic><Channel Id="CH"><ParameterBlock Id="PB">
<ParameterRefRef RefId="P_R" /><choose ParamRefId="P_R">
<when test="1"><ComObjectRefRef RefId="O0_R" /></when>
<when test="2"><ComObjectRefRef RefId="O1_R" /></when>
</choose></ParameterBlock></Channel></Dynamic>
</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

fn fixture(xml: &str) -> (tempfile::TempDir, knx_productdb::Connection, ImageRequest) {
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    knx_productdb::ingest_file(&conn, "synthetic.xml", xml.as_bytes()).unwrap();
    let request = ImageRequest {
        program_id: ID.into(),
        individual_address: IndividualAddress::from_raw(0x1234),
        values: BTreeMap::new(),
        links: Vec::new(),
        flag_overrides: BTreeMap::new(),
        priority_statements: BTreeMap::new(),
    };
    (dir, conn, request)
}

fn resolve(
    conn: &knx_productdb::Connection,
    request: &ImageRequest,
) -> Result<BTreeMap<u8, EffectiveObjectFlags>, ImageError> {
    resolve_effective_object_flags(
        conn,
        &request.program_id,
        &request.values,
        &request.flag_overrides,
    )
}

const DEFAULT0: EffectiveObjectFlags = EffectiveObjectFlags {
    read: true,
    write: false,
    transmit: true,
    update: false,
    communication: true,
};
const DEFAULT1: EffectiveObjectFlags = EffectiveObjectFlags {
    read: false,
    write: true, // ComObjectRef wins over the object's Disabled.
    transmit: false,
    update: true,
    communication: true,
};

#[test]
fn chosen_and_default_activation_use_product_then_ref_flags_and_literal_image_octets() {
    let (_dir, conn, mut request) = fixture(PROGRAM);
    let before = conn.total_changes();
    assert_eq!(resolve(&conn, &request).unwrap(), [(0, DEFAULT0)].into());
    let image = build_download_image(&conn, &request).unwrap();
    assert_eq!(
        &image.segment("S").unwrap().octets[..11],
        &[2, 0, 0, 7, 0x40, 0x4F, 0, 7, 0x41, 0x68, 7]
    );
    request.values.insert("P_R".into(), "2".into());
    assert_eq!(resolve(&conn, &request).unwrap(), [(1, DEFAULT1)].into());
    let image = build_download_image(&conn, &request).unwrap();
    assert_eq!(
        &image.segment("S").unwrap().octets[..11],
        &[2, 0, 0, 7, 0x40, 0x48, 0, 7, 0x41, 0xB7, 7]
    );
    assert_eq!(conn.total_changes(), before);
}

#[test]
fn all_five_explicit_overrides_reach_the_image_and_same_as_default_is_not_a_difference() {
    let (_dir, conn, mut request) = fixture(PROGRAM);
    let original = build_download_image(&conn, &request).unwrap();
    request.flag_overrides.insert(
        "O0_R".into(),
        FlagOverrides {
            read: Some(true),
            write: Some(false),
            transmit: Some(true),
            update: Some(false),
            communication: Some(true),
            read_on_init: None,
        },
    );
    assert_eq!(resolve(&conn, &request).unwrap(), [(0, DEFAULT0)].into());
    assert_eq!(
        build_download_image(&conn, &request).unwrap().segments,
        original.segments
    );
    request.flag_overrides.insert(
        "O0_R".into(),
        FlagOverrides {
            read: Some(false),
            write: Some(true),
            transmit: Some(false),
            update: Some(true),
            communication: Some(false),
            read_on_init: None,
        },
    );
    assert_eq!(
        resolve(&conn, &request).unwrap(),
        [(
            0,
            EffectiveObjectFlags {
                read: false,
                write: true,
                transmit: false,
                update: true,
                communication: false,
            }
        )]
        .into()
    );
    assert_eq!(
        build_download_image(&conn, &request)
            .unwrap()
            .segment("S")
            .unwrap()
            .octets[5],
        0x93
    );
}

#[test]
fn inactive_overrides_do_not_create_written_objects_and_legal_uncovered_mode_is_known_empty() {
    let (_dir, conn, mut request) = fixture(PROGRAM);
    let original = build_download_image(&conn, &request).unwrap();
    request.flag_overrides.insert(
        "O1_R".into(),
        FlagOverrides {
            read: Some(true),
            communication: Some(false),
            read_on_init: Some(true),
            ..Default::default()
        },
    );
    assert_eq!(resolve(&conn, &request).unwrap(), [(0, DEFAULT0)].into());
    assert_eq!(build_download_image(&conn, &request).unwrap(), original);
    request.values.insert("P_R".into(), "3".into());
    assert!(resolve(&conn, &request).unwrap().is_empty());
    let image = build_download_image(&conn, &request).unwrap();
    assert!(image.objects.is_empty());
    assert_eq!(image.segment("S").unwrap().octets[5], 0x48);
    assert_eq!(image.segment("S").unwrap().octets[9], 0x68);
}

/// An unplaced controller still steers the object set. A background Assign
/// cannot write another controller value after activation and call it empty.
#[test]
fn background_assignments_cannot_turn_uncertain_object_flags_into_known_empty() {
    let xml = PROGRAM
        .replace(
            "<Static>",
            "<Static><Options DownloadInvisibleParameters=\"Background\" />",
        )
        .replace("<ParameterRefRef RefId=\"P_R\" />", "")
        .replace(
            "<choose ParamRefId=\"P_R\">",
            "<Assign TargetParamRefRef=\"P_R\" Value=\"2\" /><choose ParamRefId=\"P_R\">",
        );
    assert!(xml.contains("<Options DownloadInvisibleParameters=\"Background\" />"));
    assert!(!xml.contains("<ParameterRefRef RefId=\"P_R\" />"));
    assert!(xml.contains("<Assign TargetParamRefRef=\"P_R\" Value=\"2\" />"));
    let (_dir, conn, mut request) = fixture(&xml);
    request.values.insert("P_R".into(), "3".into());
    // The evaluator sees a legal uncovered value, but the image would write
    // 2, selecting object 1. Neither seam may claim a known empty object set.
    let expected = "P_R: an active Assign sets \"2\", the evaluation read Some(\"3\")";
    for result in [
        resolve(&conn, &request).map(|_| ()),
        build_download_image(&conn, &request).map(|_| ()),
    ] {
        assert!(
            matches!(result, Err(ImageError::Unsupported { ref what }) if what == expected),
            "{result:?}"
        );
    }
    // An agreeing assignment is admitted, not rejected by a blanket guard.
    request.values.insert("P_R".into(), "2".into());
    assert_eq!(resolve(&conn, &request).unwrap(), [(1, DEFAULT1)].into());
    let image = build_download_image(&conn, &request).unwrap();
    assert_eq!(image.objects.len(), 1);
    assert_eq!(image.objects[0].number, 1);
    assert_eq!(image.segment("S").unwrap().octets[9], 0xB7);
    assert_eq!(image.segment("S").unwrap().octets[11], 2);
}

#[test]
fn uncertain_activation_is_an_explicit_evaluation_error_at_both_seams() {
    let xml = PROGRAM.replace(
        "<choose ParamRefId=\"P_R\">",
        "<choose ParamRefId=\"MISSING\">",
    );
    assert!(xml.contains("<choose ParamRefId=\"MISSING\">"));
    let (_dir, conn, request) = fixture(&xml);
    for result in [
        resolve(&conn, &request).map(|_| ()),
        build_download_image(&conn, &request).map(|_| ()),
    ] {
        assert!(
            matches!(result, Err(ImageError::Evaluation(ref diagnostics)) if !diagnostics.is_empty()),
            "{result:?}"
        );
    }
}

#[test]
fn duplicate_active_numbers_refuse_equal_and_different_flags_before_map_collection() {
    let xml = PROGRAM
        .replace(
            "</ComObjectRefs>",
            "<ComObjectRef Id=\"ALIAS\" RefId=\"O0\" /></ComObjectRefs>",
        )
        .replace(
            "<when test=\"1\">",
            "<when test=\"1\"><ComObjectRefRef RefId=\"ALIAS\" />",
        );
    assert!(xml.contains("<when test=\"1\"><ComObjectRefRef RefId=\"ALIAS\" />"));
    let (_dir, conn, mut request) = fixture(&xml);
    for different in [false, true] {
        request.flag_overrides.insert(
            "ALIAS".into(),
            FlagOverrides {
                read: Some(!different),
                ..Default::default()
            },
        );
        for result in [
            resolve(&conn, &request).map(|_| ()),
            build_download_image(&conn, &request).map(|_| ()),
        ] {
            assert!(
                matches!(
                    result,
                    Err(ImageError::GroupObjects(
                        GroupObjectTableError::DuplicateObject { number: 0 }
                    ))
                ),
                "{result:?}"
            );
        }
    }
}

#[test]
fn module_objects_are_refused_without_flattening_their_scope() {
    let xml = PROGRAM.replace("</Static><Dynamic>", "<ModuleDefs><ModuleDef Id=\"MD\"><Static /><Dynamic><ComObjectRefRef RefId=\"O0_R\" /></Dynamic></ModuleDef></ModuleDefs></Static><Dynamic><Module Id=\"M1\" RefId=\"MD\" /><Module Id=\"M2\" RefId=\"MD\" />");
    assert!(xml.contains("<Module Id=\"M1\" RefId=\"MD\" /><Module Id=\"M2\" RefId=\"MD\" />"));
    let (_dir, conn, request) = fixture(&xml);
    for result in [
        resolve(&conn, &request).map(|_| ()),
        build_download_image(&conn, &request).map(|_| ()),
    ] {
        assert!(
            matches!(result, Err(ImageError::Unsupported { ref what }) if what == "O0_R: a module instance"),
            "{result:?}"
        );
    }
}

#[test]
fn comparison_does_not_require_object_size_or_priority_image_admission() {
    for (attribute, expected_image_error) in [
        ("ObjectSize=\"Unknown\"", "ObjectSize"),
        ("Priority=\"High\"", "Priority"),
    ] {
        let xml = if attribute.starts_with("ObjectSize") {
            PROGRAM.replace("ObjectSize=\"1 Bit\"", attribute)
        } else {
            PROGRAM.replace("Id=\"O0\" Number", &format!("Id=\"O0\" {attribute} Number"))
        };
        assert!(xml.contains(attribute));
        let (_dir, conn, request) = fixture(&xml);
        assert_eq!(resolve(&conn, &request).unwrap(), [(0, DEFAULT0)].into());
        assert!(
            matches!(build_download_image(&conn, &request), Err(ImageError::Unsupported { ref what }) if what.contains(expected_image_error))
        );
    }
    let (_dir, conn, mut request) = fixture(PROGRAM);
    request
        .priority_statements
        .insert("O0_R".into(), "High".into());
    assert_eq!(resolve(&conn, &request).unwrap(), [(0, DEFAULT0)].into());
    assert!(matches!(
        build_download_image(&conn, &request),
        Err(ImageError::PriorityDiffers { .. })
    ));
}

#[test]
fn read_on_init_is_not_a_written_flag_and_retains_easy3_inference_precedence() {
    let xml = PROGRAM.replace("ReadOnInitFlag=\"Disabled\"", "ReadOnInitFlag=\"Enabled\"");
    assert!(xml.contains("ReadOnInitFlag=\"Enabled\""));
    let (_dir, conn, mut request) = fixture(&xml);
    for requested in [None, Some(false), Some(true)] {
        request.flag_overrides.insert(
            "O0_R".into(),
            FlagOverrides {
                read_on_init: requested,
                ..Default::default()
            },
        );
        assert_eq!(resolve(&conn, &request).unwrap(), [(0, DEFAULT0)].into());
        let image = build_download_image(&conn, &request).unwrap();
        assert_eq!(image.segment("S").unwrap().octets[5], 0x4F);
        assert_eq!(
            image
                .inferences
                .iter()
                .filter(|i| i.rule == READ_ON_INIT_NOT_ON_070N)
                .count(),
            usize::from(requested != Some(false))
        );
    }
    let (_dir, conn, mut request) = fixture(PROGRAM);
    request.flag_overrides.insert(
        "O0_R".into(),
        FlagOverrides {
            read_on_init: Some(true),
            ..Default::default()
        },
    );
    assert_eq!(resolve(&conn, &request).unwrap(), [(0, DEFAULT0)].into());
    let image = build_download_image(&conn, &request).unwrap();
    assert_eq!(image.segment("S").unwrap().octets[5], 0x4F);
    assert_eq!(
        image
            .inferences
            .iter()
            .filter(|i| i.rule == READ_ON_INIT_NOT_ON_070N)
            .count(),
        1
    );
}

#[test]
fn unreadable_product_flags_still_refuse_even_when_explicitly_overridden() {
    for name in [
        "ReadFlag",
        "WriteFlag",
        "TransmitFlag",
        "UpdateFlag",
        "CommunicationFlag",
        "ReadOnInitFlag",
    ] {
        let enabled = format!("{name}=\"Enabled\"");
        let disabled = format!("{name}=\"Disabled\"");
        let malformed = format!("{name}=\"Unknown\"");
        let xml = PROGRAM
            .replace(&enabled, &malformed)
            .replace(&disabled, &malformed);
        assert!(xml.contains(&malformed));
        let (_dir, conn, mut request) = fixture(&xml);
        request.flag_overrides.insert(
            "O0_R".into(),
            FlagOverrides {
                read: Some(true),
                write: Some(true),
                transmit: Some(true),
                update: Some(true),
                communication: Some(true),
                read_on_init: Some(false),
            },
        );
        for result in [
            resolve(&conn, &request).map(|_| ()),
            build_download_image(&conn, &request).map(|_| ()),
        ] {
            assert!(
                matches!(result, Err(ImageError::Unsupported { ref what }) if what == &format!("O0_R: {name} Some(\"Unknown\")")),
                "{result:?}"
            );
        }
    }
}

#[test]
fn unknown_program_missing_active_ref_and_invalid_numbers_never_become_known_empty() {
    let (_dir, conn, mut request) = fixture(PROGRAM);
    request.program_id = "missing".into();
    assert!(matches!(
        resolve(&conn, &request),
        Err(ImageError::UnknownProgram(_))
    ));
    for (old, new, expected) in [
        (
            "RefId=\"O0_R\"",
            "RefId=\"MISSING\"",
            "MISSING: no such ComObjectRef",
        ),
        (
            "Number=\"0\"",
            "Number=\"256\"",
            "O0_R: object number Some(256)",
        ),
        (
            "Number=\"0\"",
            "Number=\"-1\"",
            "O0_R: object number Some(-1)",
        ),
    ] {
        let xml = PROGRAM.replace(old, new);
        assert!(xml.contains(new));
        let (_dir, conn, request) = fixture(&xml);
        for result in [
            resolve(&conn, &request).map(|_| ()),
            build_download_image(&conn, &request).map(|_| ()),
        ] {
            assert!(
                matches!(result, Err(ImageError::Unsupported { ref what }) if what == expected),
                "{result:?}"
            );
        }
    }
}

#[test]
fn image_keeps_size_and_priority_refusals_before_unreadable_flags() {
    for (attribute, expected) in [
        ("ObjectSize=\"Unknown\"", "ObjectSize"),
        ("Priority=\"High\"", "Priority"),
    ] {
        let xml = PROGRAM.replace("ReadFlag=\"Enabled\"", "ReadFlag=\"Unknown\"");
        let xml = if attribute.starts_with("ObjectSize") {
            xml.replace("ObjectSize=\"1 Bit\"", attribute)
        } else {
            xml.replace("Id=\"O0\" Number", &format!("Id=\"O0\" {attribute} Number"))
        };
        assert!(xml.contains(attribute));
        let (_dir, conn, request) = fixture(&xml);
        assert!(
            matches!(resolve(&conn, &request), Err(ImageError::Unsupported { ref what }) if what.contains("ReadFlag"))
        );
        assert!(
            matches!(build_download_image(&conn, &request), Err(ImageError::Unsupported { ref what }) if what.contains(expected))
        );
    }
}
