//! Exercises customer-adjustable refusal through current App writers and read-only consumers.

use super::*;
use knx_app::device_download::{source_still_plans, PrepareError};
use knx_app::download_scope::{prepare_scoped_download, ScopeRequest, ScopedPrepareError};
use knx_app::download_support::{support_level, VerifiedEvidence};
use knx_app::support_report::{build_report, ReportInput};
use knx_core::commissioning::partial_memory_download::derive_partial_plan;

const CUSTOMER: &str = "customer-adjustable-parameters";
const PRIVATE: &str = "privateSentinel";
const PARAMETER: &str = r#"<Parameters><Parameter Id="M-FFF6_A-0001-01-0000_P-991" CustomerAdjustable="privateSentinel" /></Parameters>"#;

#[test]
fn admitted_union_and_module_scopes_reach_the_public_writer_without_module_translation() {
    let dir = tempfile::tempdir().unwrap();
    let (products, project) = installed(dir.path(), "");
    let plain = prepare_device_download(&products, &project, target()).unwrap();
    let member = r#"<Parameter Id="M-FFF6_A-0001-01-0000_P-991" CustomerAdjustable="false" />"#;
    let module = format!("<ModuleDefs><ModuleDef><Static><Parameters>{member}</Parameters></Static></ModuleDef></ModuleDefs>");
    let module_union = format!("<ModuleDefs><ModuleDef><Static><Parameters><Union>{member}</Union></Parameters></Static></ModuleDef></ModuleDefs>");
    for xml in [
        program_xml(&format!("<Parameters><Union>{member}</Union></Parameters>")),
        program_xml(&module),
        program_xml(&module_union),
        program_xml("").replace("<Dynamic />", &format!("{module}<Dynamic />")),
        program_xml(&format!(
            "<ModuleDefs><ModuleDef>{module}</ModuleDef></ModuleDefs>"
        )),
    ] {
        replace_source(&products, xml);
        assert_refusal(prepare_device_download(&products, &project, target()).unwrap_err());
        assert_eq!(
            offline_plan(&products, PROGRAM_ID).unwrap_err().0.code(),
            CUSTOMER
        );
        let read = prepare_device_download_for_reading(&products, &project, target()).unwrap();
        assert_eq!(read.image.segments, plain.image.segments);
    }
}

#[test]
fn excluded_module_options_and_foreign_prefixes_do_not_invent_the_new_refusal() {
    let dir = tempfile::tempdir().unwrap();
    let (products, project) = installed(dir.path(), "");
    let plain = prepare_device_download(&products, &project, target()).unwrap();
    for contents in [
        r#"<ModuleDefs><ModuleDef><Static><Options CustomerAdjustableParameters="Sync" /></Static></ModuleDef></ModuleDefs>"#,
        r#"<Options xmlns:x="urn:foreign" x:CustomerAdjustableParameters="Sync" />"#,
        r#"<Parameters><Parameter Id="M-FFF6_A-0001-01-0000_P-991" xmlns:x="urn:foreign" x:CustomerAdjustable="true" /></Parameters>"#,
    ] {
        replace_source(&products, program_xml(contents));
        let prepared = prepare_device_download(&products, &project, target()).unwrap();
        assert_eq!(prepared.plan, plain.plan);
        assert_eq!(prepared.image.segments, plain.image.segments);
    }
}

#[test]
fn a_fully_typed_active_placed_parameter_refuses_but_its_read_image_is_unchanged() {
    let mut plain = None;
    for marker in ["", r#"CustomerAdjustable="false""#] {
        let dir = tempfile::tempdir().unwrap();
        let products =
            knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
        let contents = format!(
            r#"<ParameterTypes><ParameterType Id="M-FFF6_A-0001-01-0000_PT-1" Name="number"><TypeNumber SizeInBit="8" Type="unsignedInt" minInclusive="0" maxInclusive="100" /></ParameterType></ParameterTypes>
<Parameters><Parameter Id="M-FFF6_A-0001-01-0000_P-991" Name="number" ParameterType="M-FFF6_A-0001-01-0000_PT-1" Value="37" {marker}><Memory CodeSegment="M-FFF6_A-0001-01-0000_AS-4400" Offset="2" BitOffset="0" /></Parameter></Parameters>
<ParameterRefs><ParameterRef Id="M-FFF6_A-0001-01-0000_P-991_R-1" RefId="M-FFF6_A-0001-01-0000_P-991" /></ParameterRefs>"#
        );
        let xml = program_xml(&contents).replace("<Dynamic />", r#"<Dynamic><Channel Id="C" Text="C"><ParameterBlock Id="PB" Text="B"><ParameterRefRef RefId="M-FFF6_A-0001-01-0000_P-991_R-1" /></ParameterBlock></Channel></Dynamic>"#);
        knx_productdb::ingest_file(&products, "M-FFF6/A.xml", xml.as_bytes()).unwrap();
        knx_productdb::ingest_file(&products, "M-FFF6/Hardware.xml", HARDWARE_XML.as_bytes())
            .unwrap();
        let project = project();
        if marker.is_empty() {
            let prepared = prepare_device_download(&products, &project, target()).unwrap();
            assert_eq!(prepared.plan.data_octets(), 12);
            assert_eq!(prepared.image.segments[1].octets, [1, 2, 37, 4]);
            plain = Some((prepared.plan, prepared.image.segments));
        } else {
            assert_refusal(prepare_device_download(&products, &project, target()).unwrap_err());
            let read = prepare_device_download_for_reading(&products, &project, target()).unwrap();
            let (plan, segments) = plain.as_ref().unwrap();
            assert_eq!(&read.plan, plan);
            assert_eq!(&read.image.segments, segments);
        }
    }
}

fn assert_private_free(text: &str) {
    for private in [PRIVATE, "P-991", "Static(1)", "ModuleDef("] {
        assert!(
            !text.contains(private),
            "new refusal/report leaked source payload"
        );
    }
}

fn assert_refusal(error: PrepareError) {
    assert_eq!(prepare_category(&error).code(), CUSTOMER, "{error}");
    assert!(error.to_string().contains("no regular download scope"));
    assert_private_free(&error.to_string());
    assert_private_free(&format!("{error:?}"));
}

fn replace_source(products: &knx_productdb::Connection, xml: String) {
    let bytes = xml.into_bytes();
    let sha = knx_productdb::blob::sha256_hex(&bytes);
    knx_productdb::store_source_file(
        products,
        &knx_productdb::SourceFile {
            source_path: "M-FFF6/current.xml".into(),
            manufacturer_id: Some("M-FFF6".into()),
            bytes,
        },
    )
    .unwrap();
    assert_eq!(
        products
            .execute(
                "UPDATE application_program SET source_sha256=?1 WHERE id=?2",
                [&sha, PROGRAM_ID]
            )
            .unwrap(),
        1
    );
}

#[test]
fn every_token_refuses_complete_all_partials_and_concrete_changes() {
    for token in [
        "true", "Sync", "false", "0", "1", "TRUE", " \t\n ", "", PRIVATE,
    ] {
        for marker in [
            format!(
                r#"<Parameters><Parameter Id="M-FFF6_A-0001-01-0000_P-991" CustomerAdjustable="{token}" /></Parameters>"#
            ),
            format!(r#"<Options CustomerAdjustableParameters="{token}" />"#),
        ] {
            let dir = tempfile::tempdir().unwrap();
            let (products, mut project) = installed(dir.path(), &marker);
            let scopes = [
                ScopeRequest::Complete,
                ScopeRequest::Partial(SCOPES[0]),
                ScopeRequest::Partial(SCOPES[1]),
                ScopeRequest::Partial(SCOPES[2]),
            ];
            for scope in scopes {
                match prepare_scoped_download(&products, &project, target(), scope) {
                    Err(ScopedPrepareError::Prepare(error)) => assert_refusal(error),
                    other => panic!("marker must refuse {scope:?}, not {other:?}"),
                }
            }
            // A genuinely concrete Changes request for each supported pending scope.
            for (app_loaded, params_loaded, ga_loaded, expected) in [
                (false, true, true, None),
                (true, false, true, Some(SCOPES[0])),
                (true, true, false, Some(SCOPES[1])),
                (true, false, false, Some(SCOPES[2])),
            ] {
                let state = &mut project.devices.get_mut(DeviceId(1)).unwrap().commissioning;
                state.application_program_loaded = app_loaded;
                state.parameters_loaded = params_loaded;
                state.communication_part_loaded = ga_loaded;
                assert_eq!(
                    knx_app::download_scope::resolve_scope(
                        &project,
                        target(),
                        ScopeRequest::Changes
                    )
                    .unwrap()
                    .resolved,
                    Some(expected)
                );
                match prepare_scoped_download(&products, &project, target(), ScopeRequest::Changes)
                {
                    Err(ScopedPrepareError::Prepare(error)) => assert_refusal(error),
                    other => panic!("concrete Changes must refuse: {other:?}"),
                }
            }
            let state = &mut project.devices.get_mut(DeviceId(1)).unwrap().commissioning;
            state.application_program_loaded = true;
            state.parameters_loaded = true;
            state.communication_part_loaded = true;
            let nothing =
                prepare_scoped_download(&products, &project, target(), ScopeRequest::Changes)
                    .unwrap();
            assert_eq!(nothing.resolved, None);
            assert!(nothing.prepared.is_none());
        }
    }
}

#[test]
fn all_project_device_statuses_and_download_flags_leave_the_refusal_in_force() {
    let dir = tempfile::tempdir().unwrap();
    let (products, mut project) = installed(dir.path(), PARAMETER);
    let statuses = [
        CompletionStatus::Undefined,
        CompletionStatus::Editing,
        CompletionStatus::FinishedDesign,
        CompletionStatus::FinishedCommissioning,
        CompletionStatus::Tested,
        CompletionStatus::Locked,
        CompletionStatus::Accepted,
    ];
    for project_status in statuses {
        project.installations[0].completion = project_status;
        for device_status in statuses {
            for bits in 0..32 {
                let state = &mut project.devices.get_mut(DeviceId(1)).unwrap().commissioning;
                state.completion = device_status;
                state.individual_address_loaded = bits & 1 != 0;
                state.application_program_loaded = bits & 2 != 0;
                state.parameters_loaded = bits & 4 != 0;
                state.communication_part_loaded = bits & 8 != 0;
                state.medium_config_loaded = bits & 16 != 0;
                assert_refusal(prepare_device_download(&products, &project, target()).unwrap_err());
            }
        }
    }
}

#[test]
fn kind_then_plugin_then_customer_guard_precede_image_failures() {
    let dir = tempfile::tempdir().unwrap();
    let (products, project) = installed(dir.path(), PARAMETER);
    for (xml, category) in [
        (
            program_xml(PARAMETER).replace("MV-0701", "MV-0012"),
            "not-memory-mapped",
        ),
        (
            program_xml(PARAMETER).replace("ProductProcedure", "DefaultProcedure"),
            "procedure-style",
        ),
        (
            program_xml(&format!("{}{PARAMETER}", declaration())),
            "vendor-download-plugin",
        ),
        (
            program_xml(&format!(
                r#"<Options ParameterByteOrder="LittleEndian" />{PARAMETER}"#
            )),
            CUSTOMER,
        ),
    ] {
        replace_source(&products, xml);
        let error = prepare_device_download(&products, &project, target()).unwrap_err();
        assert_eq!(prepare_category(&error).code(), category, "{error}");
        let (offline_category, _) = offline_plan(&products, PROGRAM_ID).unwrap_err();
        assert_eq!(offline_category.code(), category);
    }
    replace_source(
        &products,
        program_xml(r#"<Options ParameterByteOrder="LittleEndian" />"#),
    );
    assert!(matches!(
        prepare_device_download(&products, &project, target()),
        Err(PrepareError::Image(
            knx_productdb::image::ImageError::UnsupportedByteOrder { .. }
        ))
    ));
}

#[test]
fn offline_coverage_readiness_and_safe_support_report_keep_the_named_category() {
    let dir = tempfile::tempdir().unwrap();
    let (products, project) = installed(dir.path(), PARAMETER);
    let evidence = BTreeMap::from([(
        PROGRAM_ID.into(),
        VerifiedEvidence {
            program_id: PROGRAM_ID.into(),
            scopes: vec!["complete".into()],
            device: "synthetic".into(),
            date: "2026-10-30".into(),
            reference: "synthetic-test-not-hardware".into(),
        },
    )]);
    let levels = [
        support_level(&products, PROGRAM_ID, &evidence),
        coverage(&products, None, &evidence)
            .unwrap()
            .into_iter()
            .find(|r| r.program_id == PROGRAM_ID)
            .unwrap()
            .level,
    ];
    for level in levels {
        match level {
            SupportLevel::Unsupported { category, detail } => {
                assert_eq!(category.code(), CUSTOMER);
                assert_private_free(&detail);
            }
            other => panic!("{other:?}"),
        }
    }
    let rows = project_readiness(&products, &project, &evidence);
    assert!(
        matches!(&rows[0].readiness, DeviceReadiness::Graded(SupportLevel::Unsupported { category, .. }) if category.code() == CUSTOMER)
    );
    let (category, detail) = offline_plan(&products, PROGRAM_ID).unwrap_err();
    assert_eq!(category.code(), CUSTOMER);
    assert_private_free(&detail);
    for selected in [false, true] {
        let report = build_report(
            Some(&products),
            selected.then_some(&project),
            selected.then_some(DeviceId(1)),
            &ReportInput {
                program_id: Some(PROGRAM_ID.into()),
                include_context: true,
                context_rights: true,
                ..Default::default()
            },
            "synthetic",
        );
        assert_eq!(report.support, "unsupported");
        assert!(!report.written);
        assert_eq!(
            report
                .facts
                .iter()
                .find(|f| f.field == "refusalCategory")
                .unwrap()
                .value
                .as_deref(),
            Some(CUSTOMER)
        );
        assert_private_free(&report.summary());
        assert_private_free(&serde_json::to_string(&report).unwrap());
        assert!(report.prior_run.is_none());
    }
}

#[test]
fn genuine_current_source_bound_loaded_backups_refuse_for_complete_and_all_partial_scopes() {
    let plain_dir = tempfile::tempdir().unwrap();
    let (plain_products, _) = installed(plain_dir.path(), "");
    let complete = offline_plan(&plain_products, PROGRAM_ID).unwrap();
    let declaring_dir = tempfile::tempdir().unwrap();
    let (declaring_products, _) = installed(declaring_dir.path(), PARAMETER);
    for partial in [None, Some(SCOPES[0]), Some(SCOPES[1]), Some(SCOPES[2])] {
        let plan = partial.map_or_else(
            || complete.clone(),
            |parts| derive_partial_plan(&complete, parts).unwrap().plan,
        );
        let mut stored = stored_backup(&plan);
        stored.partial = partial;
        // Current serializer/parser roundtrip, not an old-format or malformed fixture.
        let stored =
            knx_app::device_backup::from_json(&knx_app::device_backup::to_json(&stored)).unwrap();
        assert!(stored.backup.was_loaded());
        assert!(matches!(
            stored.backup.prior_application,
            knx_core::commissioning::device_backup::PriorApplication::Recorded(_)
        ));
        assert!(matches!(
            stored.backup.exemplar_serial,
            knx_core::commissioning::device_backup::ExemplarEvidence::Recorded(_)
        ));
        prepare_restore(&plain_products, &stored)
            .expect("markerless current backup must actually qualify");
        match prepare_restore(&declaring_products, &stored) {
            Err(RestorePrepareError::Plan(detail)) => {
                assert!(detail.contains(CUSTOMER));
                assert_private_free(&detail);
            }
            other => panic!("{partial:?} must fail only new offline admission: {other:?}"),
        }
    }
}

#[test]
fn source_freshness_rejects_old_complete_and_all_partial_plans_after_marker_appears() {
    let dir = tempfile::tempdir().unwrap();
    let (products, project) = installed(dir.path(), "");
    let prepared = prepare_device_download(&products, &project, target()).unwrap();
    let plans: Vec<_> = [None, Some(SCOPES[0]), Some(SCOPES[1]), Some(SCOPES[2])]
        .into_iter()
        .map(|partial| {
            let plan = partial.map_or_else(
                || prepared.plan.clone(),
                |p| derive_partial_plan(&prepared.plan, p).unwrap().plan,
            );
            assert!(source_still_plans(
                &products,
                &prepared.request,
                partial,
                &plan
            ));
            (partial, plan)
        })
        .collect();
    replace_source(&products, program_xml(PARAMETER));
    for (partial, plan) in plans {
        assert!(!source_still_plans(
            &products,
            &prepared.request,
            partial,
            &plan
        ));
    }
}

#[test]
fn reading_and_real_comparison_adoption_keep_the_markerless_image_and_plan() {
    let dir = tempfile::tempdir().unwrap();
    let (products, project) = installed(dir.path(), "");
    let plain = prepare_device_download(&products, &project, target()).unwrap();
    for marker in [
        PARAMETER,
        r#"<Options CustomerAdjustableParameters="privateSentinel" />"#,
    ] {
        replace_source(&products, program_xml(marker));
        let read = prepare_device_download_for_reading(&products, &project, target()).unwrap();
        assert_eq!(read.plan, plain.plan);
        assert_eq!(read.image.segments, plain.image.segments);
        let scope = ComparisonScope {
            server_incarnation: "synthetic-customer".into(),
            project_incarnation: 1,
        };
        let comparison = prepare_comparison(
            &products,
            &project,
            &CommandStack::new(),
            &scope,
            DeviceId(1),
        )
        .unwrap();
        assert_eq!(comparison.plan(), &plain.plan);
        assert_eq!(comparison.image().segments, plain.image.segments);
    }
}

#[test]
fn retained_unknown_rows_and_source_are_unchanged_by_new_on_demand_consumers() {
    let dir = tempfile::tempdir().unwrap();
    let (products, project) = installed(dir.path(), PARAMETER);
    fn rows(conn: &knx_productdb::Connection) -> Vec<(String, String, i64, Option<String>)> {
        conn.prepare("SELECT xpath,name,occurrences,sample FROM ingest_unknown ORDER BY xpath,name")
            .unwrap()
            .query_map([], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    }
    let before = rows(&products);
    assert!(before
        .iter()
        .any(|(_, name, count, sample)| name == "CustomerAdjustable"
            && *count == 1
            && sample.as_deref().is_some_and(|s| s.contains(PRIVATE))));
    let identity = knx_productdb::query::program_identity(&products, PROGRAM_ID)
        .unwrap()
        .unwrap();
    let raw = knx_productdb::load_source_file(&products, &identity.source_sha256)
        .unwrap()
        .unwrap();
    assert_eq!(raw, program_xml(PARAMETER).as_bytes());
    let changes = products.total_changes();
    assert_refusal(prepare_device_download(&products, &project, target()).unwrap_err());
    assert_eq!(
        offline_plan(&products, PROGRAM_ID).unwrap_err().0.code(),
        CUSTOMER
    );
    let _ = project_readiness(&products, &project, &BTreeMap::new());
    assert_eq!(rows(&products), before);
    assert_eq!(products.total_changes(), changes);
    assert_eq!(
        knx_productdb::load_source_file(&products, &identity.source_sha256)
            .unwrap()
            .unwrap(),
        raw
    );
}
