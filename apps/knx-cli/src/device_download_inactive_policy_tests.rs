//! Discriminate write-only inactive ownership, source-default topology and read-only retention.
use super::*;
use knx_app::device_download::{
    prepare_device_download, prepare_device_download_for_reading, PrepareError,
};
use knx_app::download_support::{offline_plan, UnsupportedCategory};
use knx_core::commissioning::memory_download::MemoryDownloadStep;
use knx_core::commissioning::partial_memory_download::PartialDownloadParts;
use knx_productdb::download_plan::{DownloadPlanError, InactiveWriteSelectionError as Reason};
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

fn refusal(xml: &str, expected: Reason) {
    let (_dir, conn) = catalogue(xml);
    let before = knx_productdb::image::build_download_image(
        &conn,
        &ImageRequest {
            program_id: PROGRAM_ID.into(),
            individual_address: TARGET.parse().unwrap(),
            values: Default::default(),
            links: vec![],
            flag_overrides: Default::default(),
        },
    )
    .unwrap();
    let error = prepare_device_download(&conn, &project(), TARGET.parse().unwrap()).unwrap_err();
    assert!(
        matches!(error, PrepareError::Plan(DownloadPlanError::InactiveWriteSelection(reason)) if reason == expected),
        "{error}"
    );
    assert_eq!(
        format!("{error}"),
        format!("load procedure: inactive-write-selection: {expected}")
    );
    assert_eq!(
        offline_plan(&conn, PROGRAM_ID).unwrap_err().0,
        UnsupportedCategory::InactiveWriteSelection
    );
    let reading =
        prepare_device_download_for_reading(&conn, &project(), TARGET.parse().unwrap()).unwrap();
    assert_eq!(
        reading.image, before,
        "new write-only proof never alters the read-only image"
    );
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
fn inactive_unknown_branch_and_placement_fail_only_on_write_preparation() {
    let xml = source(1);
    refusal(
        &xml.replace("<when test=\"0\">", "<when test=\"0\"><UnknownTransform />"),
        Reason::UnresolvedOwnership,
    );
    refusal(
        &xml.replace(
            &format!(
                "<Memory CodeSegment=\"{PROGRAM_ID}_AS-4400\" Offset=\"1\" BitOffset=\"0\" />"
            ),
            "<Property ObjectIndex=\"3\" PropertyId=\"99\" />",
        ),
        Reason::UnresolvedOwnership,
    );
    refusal(
        &xml.replace(
            "<Static>",
            "<Static><Options DownloadInvisibleParameters=\"None\" />",
        ),
        Reason::UnresolvedOwnership,
    );
    for value in ["", "None", "Background", "DefaultValue", "private-marker"] {
        for later in ["<Options />", "<Options Other=\"opaque\" />"] {
            let declared =
                format!("<Static><Options DownloadInvisibleParameters=\"{value}\" />{later}");
            refusal(
                &xml.replace("<Static>", &declared),
                Reason::UnresolvedOwnership,
            );
        }
    }
    refusal(
        &xml.replace(
            "</choose>",
            "<when test=\"unknown-private-token\" /></choose>",
        ),
        Reason::UnresolvedOwnership,
    );
}

#[test]
fn an_excluded_unmodelled_type_does_not_acquire_inactive_storage_semantics() {
    let xml = source(1)
        .replace("<ParameterTypes>", &format!("<ParameterTypes><ParameterType Id=\"{PROGRAM_ID}_PT-Unknown\"><TypeNumber SizeInBit=\"8\" Type=\"opaqueInt\" /></ParameterType>"))
        .replace(&format!("Name=\"Exclusive mode zero\" ParameterType=\"{PROGRAM_ID}_PT-Octet\""), &format!("Name=\"Exclusive mode zero\" ParameterType=\"{PROGRAM_ID}_PT-Unknown\""));
    refusal(&xml, Reason::UnresolvedOwnership);
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
fn different_default_topology_is_refused_and_the_preserved_value_can_be_written_by_its_source() {
    let (_dir, conn) = catalogue(&source(1));
    let mut project = project();
    set_parameter(&mut project, "Mode", "0");
    set_parameter(&mut project, "I", "226");
    let error = prepare_device_download(&conn, &project, TARGET.parse().unwrap()).unwrap_err();
    assert!(matches!(
        error,
        PrepareError::Plan(DownloadPlanError::InactiveWriteSelection(
            Reason::DifferentSourceDefaultTopology
        ))
    ));
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
fn equal_omission_counts_at_different_addresses_are_not_the_same_topology() {
    let xml=source(1)
        .replace("</Parameters>",&format!("<Parameter Id=\"{PROGRAM_ID}_P-J\" ParameterType=\"{PROGRAM_ID}_PT-Octet\" Value=\"199\"><Memory CodeSegment=\"{PROGRAM_ID}_AS-4400\" Offset=\"2\" BitOffset=\"0\" /></Parameter></Parameters>"))
        .replace("</ParameterRefs>",&format!("<ParameterRef Id=\"{PROGRAM_ID}_P-J_R-J\" RefId=\"{PROGRAM_ID}_P-J\" /></ParameterRefs>"))
        .replace("<when test=\"1\">",&format!("<when test=\"1\"><ParameterRefRef RefId=\"{PROGRAM_ID}_P-J_R-J\" />"));
    let (_dir, conn) = catalogue(&xml);
    let mut project = project();
    set_parameter(&mut project, "Mode", "0");
    let error = prepare_device_download(&conn, &project, TARGET.parse().unwrap()).unwrap_err();
    assert!(matches!(
        error,
        PrepareError::Plan(DownloadPlanError::InactiveWriteSelection(
            Reason::DifferentSourceDefaultTopology
        ))
    ));
}

#[test]
fn unreferenced_storage_is_not_inactive_and_unknown_overlapping_owners_prevent_omission() {
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
    refusal(&xml, Reason::UnresolvedOwnership);
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
fn disjoint_active_inactive_bits_are_unsupported_not_a_whole_byte_skip() {
    let xml = source(1)
        .replace("<ParameterTypes>", &format!("<ParameterTypes><ParameterType Id=\"{PROGRAM_ID}_PT-Nibble\"><TypeNumber SizeInBit=\"4\" Type=\"unsignedInt\" minInclusive=\"0\" maxInclusive=\"15\" /></ParameterType>"))
        .replace(&format!("Name=\"Exclusive mode zero\" ParameterType=\"{PROGRAM_ID}_PT-Octet\" Value=\"226\""), &format!("Name=\"Exclusive mode zero\" ParameterType=\"{PROGRAM_ID}_PT-Nibble\" Value=\"14\""))
        .replace("</Parameters>", &format!("<Parameter Id=\"{PROGRAM_ID}_P-N\" ParameterType=\"{PROGRAM_ID}_PT-Nibble\" Value=\"10\"><Memory CodeSegment=\"{PROGRAM_ID}_AS-4400\" Offset=\"1\" BitOffset=\"4\" /></Parameter></Parameters>"))
        .replace("</ParameterRefs>", &format!("<ParameterRef Id=\"{PROGRAM_ID}_P-N_R-N\" RefId=\"{PROGRAM_ID}_P-N\" /></ParameterRefs>"))
        .replace("</ParameterBlock>", &format!("<ParameterRefRef RefId=\"{PROGRAM_ID}_P-N_R-N\" /></ParameterBlock>"));
    refusal(&xml, Reason::SharedActiveInactiveByte);
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
fn f3_namespace_app_refuses_foreign_expanded_names_after_reopen_without_data_loss() {
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
        let result = prepare_device_download(&conn, &project, TARGET.parse().unwrap());
        assert!(
            matches!(
                result,
                Err(PrepareError::Plan(
                    DownloadPlanError::InactiveWriteSelection(Reason::UnresolvedOwnership)
                ))
            ),
            "{case}: {result:?}"
        );
        assert_eq!(
            prepare_device_download_for_reading(&conn, &project, TARGET.parse().unwrap())
                .unwrap()
                .image,
            reading.image
        );
    }
}

#[test]
fn f3_namespace_offline_refuses_foreign_context_in_readiness_and_report() {
    for (case, xml) in namespace_cases(&source(1)) {
        let (_dir, conn) = catalogue(&xml);
        let result = offline_plan(&conn, PROGRAM_ID);
        assert!(
            matches!(
                result,
                Err((UnsupportedCategory::InactiveWriteSelection, _))
            ),
            "{case}: {result:?}"
        );
    }
}

#[test]
fn f3_namespace_readiness_refuses_foreign_context() {
    for (case, xml) in namespace_cases(&source(1)) {
        let (_dir, conn) = catalogue(&xml);
        let rows =
            knx_app::project_readiness::project_readiness(&conn, &project(), &BTreeMap::new());
        assert!(
            matches!(
                rows[0].readiness,
                knx_app::project_readiness::DeviceReadiness::Graded(
                    knx_app::download_support::SupportLevel::Unsupported {
                        category: UnsupportedCategory::InactiveWriteSelection,
                        ..
                    }
                )
            ),
            "{case}: {rows:?}"
        );
    }
}

#[test]
fn f3_namespace_report_refuses_foreign_context() {
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
            assert_eq!(report.support, "unsupported", "{case}: {report:?}");
            assert!(
                report
                    .facts
                    .iter()
                    .any(|fact| fact.field == "refusalCategory"
                        && fact.value.as_deref() == Some("inactive-write-selection")),
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
fn f3_opaque_dynamic_attributes_refuse_app_and_offline_but_preserve_evidence_and_values() {
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
        refusal(&xml, Reason::UnresolvedOwnership);
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
