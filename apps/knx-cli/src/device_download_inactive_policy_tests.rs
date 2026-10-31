//! Pin byte-precise inactive omission, its regular-plan fallback and read-only retention.
use super::*;
use knx_app::device_download::{
    prepare_device_download, prepare_device_download_for_reading, PrepareError,
};
use knx_app::download_support::offline_plan;
use knx_core::commissioning::memory_download::MemoryDownloadStep;
use knx_core::commissioning::partial_memory_download::PartialDownloadParts;
use knx_productdb::download_plan::DownloadPlanError;
use knx_productdb::image::ImageRequest;
use std::collections::BTreeMap;

fn catalogue(xml: &str) -> (tempfile::TempDir, knx_productdb::Connection) {
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    knx_productdb::ingest_file(&conn, "M-FFF6/A.xml", xml.as_bytes()).unwrap();
    knx_productdb::ingest_file(&conn, "M-FFF6/Hardware.xml", HARDWARE_XML.as_bytes()).unwrap();
    (dir, conn)
}

fn writes(plan: &MemoryDownloadPlan) -> Vec<(u16, Vec<u8>)> {
    plan.steps
        .iter()
        .filter_map(|step| match step {
            MemoryDownloadStep::WriteMemory { address, octets } if *address >= 0x4400 => {
                Some((*address, octets.clone()))
            }
            _ => None,
        })
        .collect()
}

fn set_parameter(project: &mut Project, reference: &str, raw: &str) {
    let id = knx_core::ParameterInstanceId(project.installations[0].parameters.len() as u32 + 1);
    project.installations[0]
        .parameters
        .push(knx_core::ParameterInstance {
            id,
            device: DeviceId(1),
            source: SourceRef {
                path: "synthetic".into(),
                ets_id: format!("{PROGRAM_ID}_P-{reference}_R-{reference}"),
            },
            raw: raw.into(),
        });
}

/// Whether some planned write covers `address`.
fn covers(writes: &[(u16, Vec<u8>)], address: u16) -> bool {
    writes
        .iter()
        .any(|(a, d)| *a <= address && usize::from(address - a) < d.len())
}

/// A failed proof omits nothing: write preparation and offline plan stay the regular plan.
fn falls_back(xml: &str) {
    let (_dir, conn) = catalogue(xml);
    let request = ImageRequest {
        program_id: PROGRAM_ID.into(),
        individual_address: TARGET.parse().unwrap(),
        values: Default::default(),
        links: vec![],
        flag_overrides: Default::default(),
    };
    let before = knx_productdb::image::build_download_image(&conn, &request).unwrap();
    let regular = knx_productdb::download_plan::plan_memory_download_with_inferences(&before)
        .unwrap()
        .0;
    let prepared = prepare_device_download(&conn, &project(), TARGET.parse().unwrap())
        .unwrap_or_else(|e| panic!("a failed proof must not refuse the device: {e}"));
    assert_eq!(prepared.plan, regular, "a failed proof omits nothing");
    assert_eq!(
        prepared.write_selection,
        knx_app::device_backup::WriteSelection::SourceMaskOnlyV1,
        "a plan that omits nothing binds the generator that rebuilds it"
    );
    assert!(
        covers(&writes(&prepared.plan), 0x4401),
        "{:?}",
        writes(&prepared.plan)
    );
    assert_eq!(
        writes(&offline_plan(&conn, PROGRAM_ID).unwrap()),
        writes(&regular)
    );
    let reading =
        prepare_device_download_for_reading(&conn, &project(), TARGET.parse().unwrap()).unwrap();
    assert_eq!(
        reading.image, before,
        "new write-only proof never alters the read-only image"
    );
    assert_eq!(reading.plan, prepared.plan);
}

#[test]
fn offline_producer_all_partial_scopes_and_changes_use_selected_plan_counts() {
    let (_dir, conn) = catalogue(&source(1));
    assert_eq!(
        writes(&offline_plan(&conn, PROGRAM_ID).unwrap()),
        vec![
            (0x4400, vec![1]),
            (0x4402, vec![0xC7, 0x53]),
            (0x4405, vec![0x68])
        ]
    );
    for (parameters, ga, application, expected) in [
        (true, true, true, 12),
        (true, false, false, 4),
        (false, true, false, 8),
        (true, true, false, 12),
        (false, false, false, 0),
    ] {
        let mut project = project();
        let state = &mut project.devices.get_mut(DeviceId(1)).unwrap().commissioning;
        state.application_program_loaded = !application;
        state.parameters_loaded = !parameters;
        state.communication_part_loaded = !ga;
        let scoped = prepare_scoped_download(
            &conn,
            &project,
            TARGET.parse().unwrap(),
            ScopeRequest::Changes,
        )
        .unwrap();
        if expected == 0 {
            assert!(scoped.prepared.is_none());
            continue;
        }
        let prepared = scoped.prepared.unwrap();
        assert_eq!(prepared.plan.data_octets(), expected);
        assert_eq!(
            prepared
                .octets_to_write()
                .iter()
                .map(|(_, _, _, n)| n)
                .sum::<usize>(),
            expected
        );
        assert_eq!(writes(&prepared.plan).is_empty(), !parameters);
    }
    for parts in [
        PartialDownloadParts {
            parameters: true,
            group_addresses: false,
        },
        PartialDownloadParts {
            parameters: false,
            group_addresses: true,
        },
        PartialDownloadParts {
            parameters: true,
            group_addresses: true,
        },
    ] {
        let prepared = prepare_scoped_download(
            &conn,
            &project(),
            TARGET.parse().unwrap(),
            ScopeRequest::Partial(parts),
        )
        .unwrap()
        .prepared
        .unwrap();
        assert_eq!(
            prepared.plan.data_octets(),
            if parts.parameters { 4 } else { 0 } + if parts.group_addresses { 8 } else { 0 }
        );
    }
}

#[test]
fn unknown_branch_placement_and_invisible_parameter_options_fall_back_to_the_regular_plan() {
    let xml = source(1);
    falls_back(&xml.replace("<when test=\"0\">", "<when test=\"0\"><UnknownTransform />"));
    falls_back(&xml.replace(
        &format!("<Memory CodeSegment=\"{PROGRAM_ID}_AS-4400\" Offset=\"1\" BitOffset=\"0\" />"),
        "<Property ObjectIndex=\"3\" PropertyId=\"99\" />",
    ));
    falls_back(&xml.replace(
        "<Static>",
        "<Static><Options DownloadInvisibleParameters=\"None\" />",
    ));
    for value in ["", "None", "Background", "DefaultValue", "private-marker"] {
        for later in ["<Options />", "<Options Other=\"opaque\" />"] {
            let declared =
                format!("<Static><Options DownloadInvisibleParameters=\"{value}\" />{later}");
            falls_back(&xml.replace("<Static>", &declared));
        }
    }
    falls_back(&xml.replace(
        "</choose>",
        "<when test=\"unknown-private-token\" /></choose>",
    ));
}

#[test]
fn an_excluded_unmodelled_type_does_not_acquire_inactive_storage_semantics() {
    let xml = source(1)
        .replace("<ParameterTypes>", &format!("<ParameterTypes><ParameterType Id=\"{PROGRAM_ID}_PT-Unknown\"><TypeNumber SizeInBit=\"8\" Type=\"opaqueInt\" /></ParameterType>"))
        .replace(&format!("Name=\"Exclusive mode zero\" ParameterType=\"{PROGRAM_ID}_PT-Octet\""), &format!("Name=\"Exclusive mode zero\" ParameterType=\"{PROGRAM_ID}_PT-Unknown\""));
    falls_back(&xml);
}

#[test]
fn active_alias_keeps_the_inactive_reference_byte_writable_and_literals_stay() {
    let xml = source(1).replace("</ParameterRefs>", &format!("<ParameterRef Id=\"{PROGRAM_ID}_P-I_R-Other\" RefId=\"{PROGRAM_ID}_P-I\" /></ParameterRefs>"))
        .replace("</ParameterBlock>", &format!("<ParameterRefRef RefId=\"{PROGRAM_ID}_P-I_R-Other\" /></ParameterBlock>"));
    let (_dir, conn) = catalogue(&xml);
    assert_eq!(
        writes(&offline_plan(&conn, PROGRAM_ID).unwrap()),
        vec![(0x4400, vec![1, 0xE2, 0xC7, 0x53]), (0x4405, vec![0x68])]
    );
}

#[test]
fn source_protected_non_ff_bytes_are_not_reinterpreted_as_transport_bits() {
    let xml = source(1).replace("/////wD/", "/w///wD/"); // FF 0F FF FF 00 FF
    let (_dir, conn) = catalogue(&xml);
    let prepared = prepare_device_download(&conn, &project(), TARGET.parse().unwrap()).unwrap();
    assert_eq!(
        prepared.image.segments[1].mask.as_ref().unwrap(),
        &[0xFF, 0x0F, 0xFF, 0xFF, 0, 0xFF]
    );
    assert_eq!(
        writes(&prepared.plan),
        vec![
            (0x4400, vec![1]),
            (0x4402, vec![0xC7, 0x53]),
            (0x4405, vec![0x68])
        ]
    );
}

#[test]
fn different_default_topology_omits_nothing_and_writes_the_preserved_value() {
    let (_dir, conn) = catalogue(&source(1));
    let mut project = project();
    set_parameter(&mut project, "Mode", "0");
    set_parameter(&mut project, "I", "226");
    let prepared = prepare_device_download(&conn, &project, TARGET.parse().unwrap()).unwrap();
    let reading =
        prepare_device_download_for_reading(&conn, &project, TARGET.parse().unwrap()).unwrap();
    assert_eq!(
        prepared.plan, reading.plan,
        "restore regenerates from defaults"
    );
    assert_eq!(
        writes(&prepared.plan),
        vec![(0x4400, vec![0, 0xE2, 0xC7, 0xA6]), (0x4405, vec![0x68])]
    );
    let (_dir, sibling) = catalogue(&source(0));
    assert_eq!(
        writes(
            &prepare_device_download(&sibling, &project, TARGET.parse().unwrap())
                .unwrap()
                .plan
        ),
        vec![(0x4400, vec![0, 0xE2, 0xC7, 0xA6]), (0x4405, vec![0x68])]
    );
}

#[test]
fn equal_omission_counts_at_different_addresses_are_not_the_same_topology_and_omit_nothing() {
    let xml=source(1)
        .replace("</Parameters>",&format!("<Parameter Id=\"{PROGRAM_ID}_P-J\" ParameterType=\"{PROGRAM_ID}_PT-Octet\" Value=\"199\"><Memory CodeSegment=\"{PROGRAM_ID}_AS-4400\" Offset=\"2\" BitOffset=\"0\" /></Parameter></Parameters>"))
        .replace("</ParameterRefs>",&format!("<ParameterRef Id=\"{PROGRAM_ID}_P-J_R-J\" RefId=\"{PROGRAM_ID}_P-J\" /></ParameterRefs>"))
        .replace("<when test=\"1\">",&format!("<when test=\"1\"><ParameterRefRef RefId=\"{PROGRAM_ID}_P-J_R-J\" />"));
    let (_dir, conn) = catalogue(&xml);
    let mut project = project();
    set_parameter(&mut project, "Mode", "0");
    let prepared = prepare_device_download(&conn, &project, TARGET.parse().unwrap()).unwrap();
    let reading =
        prepare_device_download_for_reading(&conn, &project, TARGET.parse().unwrap()).unwrap();
    assert_eq!(
        prepared.plan, reading.plan,
        "a moved omission omits nothing"
    );
    assert!(covers(&writes(&prepared.plan), 0x4401));
    assert!(covers(&writes(&prepared.plan), 0x4402));
}

#[test]
fn unreferenced_storage_is_not_inactive_and_an_overlapping_owner_keeps_its_byte_written() {
    let xml = source(1).replace(
        &format!("<ParameterRefRef RefId=\"{PROGRAM_ID}_P-I_R-I\" />"),
        "",
    );
    let (_dir, conn) = catalogue(&xml);
    assert_eq!(
        writes(&offline_plan(&conn, PROGRAM_ID).unwrap()),
        vec![(0x4400, vec![1, 0x9A, 0xC7, 0x53]), (0x4405, vec![0x68])]
    );
    let xml=source(1)
        .replace("</Parameters>",&format!("<Parameter Id=\"{PROGRAM_ID}_P-Other\" ParameterType=\"{PROGRAM_ID}_PT-Octet\" Value=\"154\"><Memory CodeSegment=\"{PROGRAM_ID}_AS-4400\" Offset=\"1\" BitOffset=\"0\" /></Parameter></Parameters>"))
        .replace("</ParameterRefs>",&format!("<ParameterRef Id=\"{PROGRAM_ID}_P-Other_R-Other\" RefId=\"{PROGRAM_ID}_P-Other\" /></ParameterRefs>"));
    falls_back(&xml);
}

#[test]
fn existing_image_and_complete_plan_errors_precede_new_ownership_errors() {
    let xml = source(1).replace("<when test=\"0\">", "<when test=\"0\"><UnknownTransform />");
    let (_dir, conn) = catalogue(&xml.replace("/////wD/", "/////wAA"));
    assert!(matches!(
        prepare_device_download(&conn, &project(), TARGET.parse().unwrap()),
        Err(PrepareError::Image(_))
    ));
    let (_dir, conn) = catalogue(&xml.replace("SegFlags=\"128\"", "SegFlags=\"129\""));
    assert!(matches!(
        prepare_device_download(&conn, &project(), TARGET.parse().unwrap()),
        Err(PrepareError::Plan(DownloadPlanError::SegmentFlags(129)))
    ));
}

#[test]
fn disjoint_active_inactive_bits_keep_their_byte_written_not_a_whole_byte_skip() {
    let xml = source(1)
        .replace("<ParameterTypes>", &format!("<ParameterTypes><ParameterType Id=\"{PROGRAM_ID}_PT-Nibble\"><TypeNumber SizeInBit=\"4\" Type=\"unsignedInt\" minInclusive=\"0\" maxInclusive=\"15\" /></ParameterType>"))
        .replace(&format!("Name=\"Exclusive mode zero\" ParameterType=\"{PROGRAM_ID}_PT-Octet\" Value=\"226\""), &format!("Name=\"Exclusive mode zero\" ParameterType=\"{PROGRAM_ID}_PT-Nibble\" Value=\"14\""))
        .replace("</Parameters>", &format!("<Parameter Id=\"{PROGRAM_ID}_P-N\" ParameterType=\"{PROGRAM_ID}_PT-Nibble\" Value=\"10\"><Memory CodeSegment=\"{PROGRAM_ID}_AS-4400\" Offset=\"1\" BitOffset=\"4\" /></Parameter></Parameters>"))
        .replace("</ParameterRefs>", &format!("<ParameterRef Id=\"{PROGRAM_ID}_P-N_R-N\" RefId=\"{PROGRAM_ID}_P-N\" /></ParameterRefs>"))
        .replace("</ParameterBlock>", &format!("<ParameterRefRef RefId=\"{PROGRAM_ID}_P-N_R-N\" /></ParameterBlock>"));
    falls_back(&xml);
}

#[test]
fn a_shared_byte_stays_written_while_an_independently_proved_byte_is_still_omitted() {
    let inactive = format!("<ParameterRefRef RefId=\"{PROGRAM_ID}_P-I_R-I\" />");
    let xml = source(1)
        .replace("<ParameterTypes>", &format!("<ParameterTypes><ParameterType Id=\"{PROGRAM_ID}_PT-Nibble\"><TypeNumber SizeInBit=\"4\" Type=\"unsignedInt\" minInclusive=\"0\" maxInclusive=\"15\" /></ParameterType>"))
        .replace("</Parameters>", &format!("<Parameter Id=\"{PROGRAM_ID}_P-K\" ParameterType=\"{PROGRAM_ID}_PT-Nibble\" Value=\"5\"><Memory CodeSegment=\"{PROGRAM_ID}_AS-4400\" Offset=\"2\" BitOffset=\"0\" /></Parameter><Parameter Id=\"{PROGRAM_ID}_P-N\" ParameterType=\"{PROGRAM_ID}_PT-Nibble\" Value=\"10\"><Memory CodeSegment=\"{PROGRAM_ID}_AS-4400\" Offset=\"2\" BitOffset=\"4\" /></Parameter></Parameters>"))
        .replace("</ParameterRefs>", &format!("<ParameterRef Id=\"{PROGRAM_ID}_P-K_R-K\" RefId=\"{PROGRAM_ID}_P-K\" /><ParameterRef Id=\"{PROGRAM_ID}_P-N_R-N\" RefId=\"{PROGRAM_ID}_P-N\" /></ParameterRefs>"))
        .replace(&inactive, &format!("{inactive}<ParameterRefRef RefId=\"{PROGRAM_ID}_P-K_R-K\" />"))
        .replace("</ParameterBlock>", &format!("<ParameterRefRef RefId=\"{PROGRAM_ID}_P-N_R-N\" /></ParameterBlock>"));
    assert!(xml.contains(&format!(
        "{inactive}<ParameterRefRef RefId=\"{PROGRAM_ID}_P-K_R-K\" />"
    )));
    assert!(xml.contains(&format!(
        "<ParameterRefRef RefId=\"{PROGRAM_ID}_P-N_R-N\" /></ParameterBlock>"
    )));
    let (_dir, conn) = catalogue(&xml);
    let regular = writes(
        &prepare_device_download_for_reading(&conn, &project(), TARGET.parse().unwrap())
            .unwrap()
            .plan,
    );
    let [(0x4400, ref octets), (0x4405, _)] = regular[..] else {
        panic!("{regular:?}")
    };
    let shared = octets[2];
    let prepared = prepare_device_download(&conn, &project(), TARGET.parse().unwrap())
        .unwrap_or_else(|e| panic!("a shared byte must not refuse the device: {e}"));
    assert_eq!(
        writes(&prepared.plan),
        vec![
            (0x4400, vec![1]),
            (0x4402, vec![shared, 0x53]),
            (0x4405, vec![0x68])
        ]
    );
    assert_eq!(
        writes(&offline_plan(&conn, PROGRAM_ID).unwrap()),
        writes(&prepared.plan)
    );
}

#[test]
fn f2_historical_evidence_requires_plan_equality_in_offline_and_project_consumers() {
    use knx_app::download_support::{support_level, VerifiedEvidence};
    let evidence = BTreeMap::from([(
        PROGRAM_ID.into(),
        VerifiedEvidence {
            program_id: PROGRAM_ID.into(),
            scopes: vec!["complete".into()],
            device: "synthetic historical witness".into(),
            date: "2026-10-01".into(),
            reference: "synthetic source-mask-only run, not new hardware evidence".into(),
        },
    )]);
    for (mode, expected) in [(0, "verified"), (1, "untested")] {
        let (_dir, conn) = catalogue(&source(mode));
        let prepared = prepare_device_download(&conn, &project(), TARGET.parse().unwrap()).unwrap();
        assert!(
            prepared.inferences.is_empty(),
            "selection policy is not a layout inference"
        );
        let historical =
            knx_productdb::download_plan::plan_memory_download_with_inferences(&prepared.image)
                .unwrap()
                .0;
        assert_eq!(prepared.plan == historical, mode == 0);
        let level = support_level(&conn, PROGRAM_ID, &evidence);
        assert_eq!(level.code(), expected, "offline mode={mode}");
        assert_eq!(level.needs_acknowledgement(), mode == 1);
        let rows = knx_app::project_readiness::project_readiness(&conn, &project(), &evidence);
        assert!(
            format!("{:?}", rows[0].readiness).contains(if mode == 0 {
                "Verified"
            } else {
                "Untested"
            }),
            "{:?}",
            rows
        );
    }
}

include!("../tests/fixtures/inactive-selection/namespaces.rs");

#[test]
fn f3_namespace_app_falls_back_on_foreign_expanded_names_after_reopen_without_data_loss() {
    for (case, xml) in namespace_cases(&source(1)) {
        let (dir, conn) = catalogue(&xml);
        drop(conn);
        let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
        let mut project = project();
        set_parameter(&mut project, "I", "231");
        let reading =
            prepare_device_download_for_reading(&conn, &project, TARGET.parse().unwrap()).unwrap();
        let raw: Vec<u8> = conn
            .query_row(
                "SELECT bytes FROM source_file WHERE source_path='M-FFF6/A.xml'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(raw, xml.as_bytes(), "{case}");
        assert_eq!(reading.request.values.values().next().unwrap(), "231");
        let prepared = prepare_device_download(&conn, &project, TARGET.parse().unwrap())
            .unwrap_or_else(|e| panic!("{case}: {e}"));
        assert_eq!(prepared.plan, reading.plan, "{case}: nothing is omitted");
        assert!(covers(&writes(&prepared.plan), 0x4401), "{case}");
        assert_eq!(
            prepare_device_download_for_reading(&conn, &project, TARGET.parse().unwrap())
                .unwrap()
                .image,
            reading.image
        );
    }
}

#[test]
fn f3_namespace_offline_plan_falls_back_on_foreign_context() {
    for (case, xml) in namespace_cases(&source(1)) {
        let (_dir, conn) = catalogue(&xml);
        let plan = offline_plan(&conn, PROGRAM_ID).unwrap_or_else(|e| panic!("{case}: {e:?}"));
        assert_eq!(
            writes(&plan),
            vec![(0x4400, vec![1, 0x9A, 0xC7, 0x53]), (0x4405, vec![0x68])],
            "{case}"
        );
    }
}

#[test]
fn f3_namespace_readiness_grades_the_fallback_plan_untested() {
    for (case, xml) in namespace_cases(&source(1)) {
        let (_dir, conn) = catalogue(&xml);
        let rows =
            knx_app::project_readiness::project_readiness(&conn, &project(), &BTreeMap::new());
        assert_eq!(rows[0].readiness.code(), "untested", "{case}: {rows:?}");
    }
}

#[test]
fn f3_namespace_report_does_not_refuse_foreign_context() {
    for (case, xml) in namespace_cases(&source(1)) {
        let (_dir, conn) = catalogue(&xml);
        for selected in [false, true] {
            let project = project();
            let input = knx_app::support_report::ReportInput {
                program_id: Some(PROGRAM_ID.into()),
                ..Default::default()
            };
            let report = knx_app::support_report::build_report(
                Some(&conn),
                selected.then_some(&project),
                selected.then_some(DeviceId(1)),
                &input,
                "test",
            );
            assert_eq!(report.support, "untested", "{case}: {report:?}");
            assert!(
                !report
                    .facts
                    .iter()
                    .any(|fact| fact.field == "refusalCategory"),
                "{report:?}"
            );
            assert!(!report.written);
            assert!(!report.summary().contains("urn:synthetic"));
        }
    }
}

#[test]
fn f3_namespace_actual_knx_prefixes_keep_exact_positive_writer() {
    for app_scope in [false, true] {
        let xml = namespace_positive(&source(1), app_scope);
        let (_dir, conn) = catalogue(&xml);
        let prepared = prepare_device_download(&conn, &project(), TARGET.parse().unwrap()).unwrap();
        assert_eq!(
            writes(&prepared.plan),
            vec![
                (0x4400, vec![1]),
                (0x4402, vec![0xC7, 0x53]),
                (0x4405, vec![0x68])
            ]
        );
        assert_eq!(
            writes(&offline_plan(&conn, PROGRAM_ID).unwrap()),
            writes(&prepared.plan)
        );
    }
}

#[test]
fn f3_opaque_dynamic_attributes_fall_back_in_app_and_offline_and_preserve_evidence_and_values() {
    let variants: [(&str, &str); 5] = [
        (&format!("<ParameterRefRef RefId=\"{PROGRAM_ID}_P-I_R-I\" />"),
         &format!("<choose Opaque=\"\" ParamRefId=\"{PROGRAM_ID}_P-Mode_R-Mode\"><when test=\"0\"><ParameterRefRef RefId=\"{PROGRAM_ID}_P-I_R-I\" /></when></choose>")),
        ("<choose ParamRefId=", "<choose Opaque=\"\" ParamRefId="),
        (
            "<when test=\"0\">",
            "<when Opaque=\"a=b&#10;c=d\" test=\"0\">",
        ),
        (
            &format!("<ParameterRefRef RefId=\"{PROGRAM_ID}_P-I_R-I\""),
            &format!("<ParameterRefRef Opaque=\"private\" RefId=\"{PROGRAM_ID}_P-I_R-I\""),
        ),
        (
            "<ParameterBlock Id=",
            "<ParameterBlock Name=\"unread-but-recognized\" Id=",
        ),
    ];
    for (needle, replacement) in variants {
        let xml = source(1).replace(needle, replacement);
        let (dir, conn) = catalogue(&xml);
        let mut project = project();
        set_parameter(&mut project, "I", "231");
        assert_eq!(project.ids.next_device_id().unwrap(), DeviceId(1));
        assert_eq!(
            project.ids.next_parameter_instance_id().unwrap(),
            knx_core::ParameterInstanceId(1)
        );
        let reading =
            prepare_device_download_for_reading(&conn, &project, TARGET.parse().unwrap()).unwrap();
        assert_eq!(reading.request.values.values().next().unwrap(), "231");
        let store = knx_store::open_and_migrate(&dir.path().join("project.knxdb")).unwrap();
        knx_store::save_project(&store, &project).unwrap();
        drop(store);
        let store = knx_store::open_and_migrate(&dir.path().join("project.knxdb")).unwrap();
        assert_eq!(knx_store::load_project(&store).unwrap(), project);
        let extras: Vec<String> = conn
            .prepare("SELECT extra FROM dynamic_node WHERE extra IS NOT NULL")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert!(extras
            .iter()
            .any(|s| s.contains("Opaque=") || s.contains("Name=unread-but-recognized")));
        let raw: Vec<u8> = conn
            .query_row(
                "SELECT bytes FROM source_file WHERE source_path='M-FFF6/A.xml'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(raw, xml.as_bytes());
        let again =
            prepare_device_download_for_reading(&conn, &project, TARGET.parse().unwrap()).unwrap();
        assert_eq!(again.image, reading.image);
        falls_back(&xml);
    }
}

#[test]
fn f3_modelled_presentation_and_branch_identity_attributes_keep_positive_selection() {
    let xml = source(1)
        .replace(
            "<when test=\"0\">",
            "<when Id=\"inactive-branch\" test=\"0\">",
        )
        .replace(
            "Text=\"Synthetic\"",
            "Text=\"Synthetic\" Name=\"channel\" Number=\"1\"",
        )
        .replace(
            "Text=\"Selection\"",
            "Text=\"Selection\" Access=\"ReadWrite\"",
        );
    let (_dir, conn) = catalogue(&xml);
    assert_eq!(
        writes(&offline_plan(&conn, PROGRAM_ID).unwrap()),
        vec![
            (0x4400, vec![1]),
            (0x4402, vec![0xC7, 0x53]),
            (0x4405, vec![0x68])
        ]
    );
}
