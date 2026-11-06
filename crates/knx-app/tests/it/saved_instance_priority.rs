//! Exercises schema-23 captured declarations through import, native history and projection.

use knx_core::{
    Command, CommandStack, SavedInstancePriority as Fact, SavedPriorityOmission as Omission,
    SavedPriorityUnavailable as Reason,
};
use knx_projection::{SavedInstancePriorityView as View, SavedPriorityTokenStatusView as Token};

const MEMBER: &str = "P-TEST/0.xml";
const INFO: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/23"><Project Id="P-TEST"><ProjectInformation Name="Synthetic saved priority"/></Project></KNX>"#;

fn device(id: &str, refs: &str, tree: &str) -> String {
    format!(
        r#"<DeviceInstance Id="{id}" Address="1" ProductRefId="M-TEST_P-1" Hardware2ProgramRefId="M-TEST_H-1_HP-1"><ComObjectInstanceRefs>{refs}</ComObjectInstanceRefs><GroupObjectTree GroupObjectInstances="{tree}"/></DeviceInstance>"#
    )
}
fn topology(line: &str, unassigned: &str) -> String {
    format!(
        r#"<KNX xmlns="http://knx.org/xml/project/23"><Project Id="P-TEST"><Installations><Installation InstallationId="0"><Topology><Area Id="P-TEST-0_A-1" Address="1"><Line Id="P-TEST-0_L-1" Address="1"><Segment Id="P-TEST-0_S-1" Number="1">{line}</Segment></Line></Area><UnassignedDevices>{unassigned}</UnassignedDevices></Topology></Installation></Installations></Project></KNX>"#
    )
}
fn archive(xml: &str) -> Vec<u8> {
    knx_testsupport::zip_with_entries(&[
        ("P-TEST.signature", b""),
        (MEMBER, xml.as_bytes()),
        ("P-TEST/project.xml", INFO),
    ])
}
fn import(xml: &str, conn: &knx_store::Connection) -> knx_app::ImportedProject {
    knx_app::import_ets_project_bytes(
        archive(xml),
        "synthetic.knxproj",
        conn,
        knx_app::ImportOptions::default(),
    )
    .unwrap()
}
fn fact(project: &knx_core::Project) -> &Fact {
    &project
        .devices
        .com_objects()
        .next()
        .unwrap()
        .saved_instance_priority
}
fn assert_declared(project: &knx_core::Project, xml: &str, raw: &str, unassigned: bool) {
    let Fact::Declared(value) = fact(project) else {
        panic!("expected captured declaration: {:?}", fact(project));
    };
    assert_eq!(value.raw, raw);
    assert_eq!(value.source.source_schema, 23);
    assert_eq!(value.source.device_source.path, MEMBER);
    assert_eq!(value.source.device_source.ets_id, "P-TEST-0_DI-1");
    assert_eq!(value.source.object_source.path, MEMBER);
    assert_eq!(value.source.object_source.ets_id, "O-7_R-1");
    assert_eq!(
        value.source.topology_sha256,
        knx_etsproj::opaque::sha256_hex(xml.as_bytes())
    );
    let owner = if unassigned {
        "/Topology/UnassignedDevices/DeviceInstance"
    } else {
        "/Topology/Area/Line/Segment/DeviceInstance"
    };
    assert_eq!(value.source.owner_xpath,format!("/KNX/Project/Installations/Installation{owner}[@Id='P-TEST-0_DI-1']/ComObjectInstanceRefs/ComObjectInstanceRef[@RefId='O-7_R-1']"));
}

#[test]
fn saved_instance_priority_fresh_literals_raw_presence_and_native_reopen() {
    for unassigned in [false, true] {
        for (attribute, raw, token) in [
            ("Low", "Low", Token::Known),
            ("High", "High", Token::Known),
            ("Alert", "Alert", Token::Known),
            ("", "", Token::Empty),
            (" ", " ", Token::Unknown),
            ("low", "low", Token::Unknown),
            ("Future", "Future", Token::Unknown),
            (" L&#x6f;w ", " Low ", Token::Unknown),
            (
                "Gr&#xFC;&#xDF;e &amp; &#x03A9;",
                "Grüße & Ω",
                Token::Unknown,
            ),
        ] {
            let d = device(
                "P-TEST-0_DI-1",
                &format!(r#"<ComObjectInstanceRef RefId="O-7_R-1" Priority="{attribute}"/>"#),
                "O-7_R-1",
            );
            let xml = if unassigned {
                topology("", &d)
            } else {
                topology(&d, "")
            };
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("native.knxdb");
            let conn = knx_store::open_and_migrate(&path).unwrap();
            let imported = import(&xml, &conn);
            assert_declared(&imported.project, &xml, raw, unassigned);
            let opaque = knx_store::load_opaque(&conn).unwrap();
            assert_eq!(opaque.iter().filter(|e| e.name == "Priority").count(), 1);
            knx_store::save_project(&conn, &imported.project).unwrap();
            drop(conn);
            for _ in 0..2 {
                let conn = knx_store::open_existing_and_migrate(&path).unwrap();
                let project = knx_store::load_project(&conn).unwrap();
                assert_eq!(project, imported.project);
                assert_eq!(knx_store::load_opaque(&conn).unwrap(), opaque);
                let com = project.devices.com_objects().next().unwrap();
                assert_eq!(
                    knx_store::devices::load_com_object_instance(&conn, com.id)
                        .unwrap()
                        .saved_instance_priority,
                    com.saved_instance_priority
                );
                let view = knx_projection::build_device_detail(&project, com.device).unwrap();
                let View::Declared {
                    raw: view_raw,
                    token_status,
                    source,
                } = &view.com_objects[0].saved_instance_priority
                else {
                    panic!("declaration not projected");
                };
                assert_eq!(view_raw, raw);
                assert_eq!(*token_status, token);
                assert_eq!(
                    source.topology_sha256,
                    knx_etsproj::opaque::sha256_hex(xml.as_bytes())
                );
                knx_store::save_project_with_passthrough(&conn, &project, &opaque, &[]).unwrap();
            }
        }
    }
}

#[test]
fn saved_instance_priority_absence_causes_scope_and_out_of_tree() {
    let refs = r#"<ComObjectInstanceRef RefId="O-7_R-1"/><ComObjectInstanceRef RefId="O-9_R-1" Priority="High"/>"#;
    let xml = topology(&device("P-TEST-0_DI-1", refs, "O-7_R-1 O-8_R-1"), "");
    let conn = knx_store::open_and_migrate_in_memory().unwrap();
    let imported = import(&xml, &conn);
    let objects: Vec<_> = imported.project.devices.com_objects().collect();
    assert_eq!(objects.len(), 2);
    assert!(
        matches!(&objects[0].saved_instance_priority,Fact::Absent(v) if v.omission == Omission::AttributeOmitted)
    );
    assert!(
        matches!(&objects[1].saved_instance_priority,Fact::Absent(v) if v.omission == Omission::InstanceElementOmitted)
    );
    assert!(imported
        .report
        .errors
        .iter()
        .any(|e| e.detail.contains("UnmappedObjectOverride")));
    let original = knx_store::load_opaque(&conn).unwrap();
    assert!(original
        .iter()
        .any(|e| e.source_path == MEMBER && e.xpath.is_empty() && e.bytes == xml.as_bytes()));
    for attrs in [
        r#"priority="High""#,
        r#"xmlns:x="urn:foreign" x:Priority="High""#,
    ] {
        let xml = topology(
            &device(
                "P-TEST-0_DI-1",
                &format!(
                    r#"<ComObjectInstanceRef RefId="O-7_R-1" {attrs}/><x:ComObjectInstanceRef xmlns:x="urn:foreign" RefId="O-7_R-1" Priority="Alert"/>"#
                ),
                "O-7_R-1",
            ),
            "",
        );
        let c = knx_store::open_and_migrate_in_memory().unwrap();
        assert!(
            matches!(fact(&import(&xml,&c).project),Fact::Absent(v) if v.omission==Omission::AttributeOmitted)
        );
    }
    let xml = topology(
        &device(
            "P-TEST-0_DI-1",
            r#"<ComObjectInstanceRef RefId="O-7_R-1" Priority="Low"/>"#,
            "O-7_R-1",
        ),
        "",
    )
    .replace("</GroupObjectTree>", "</GroupObjectTree>");
    let xml = xml.replace("</DeviceInstance>",r#"<Foreign><ComObjectInstanceRefs><ComObjectInstanceRef RefId="O-7_R-1" Priority="Alert"/></ComObjectInstanceRefs></Foreign></DeviceInstance>"#);
    let c = knx_store::open_and_migrate_in_memory().unwrap();
    assert_declared(&import(&xml, &c).project, &xml, "Low", false);
}

#[test]
fn saved_instance_priority_owner_binding_ambiguity_and_parser_precedence() {
    let a = device(
        "P-TEST-0_DI-1",
        r#"<ComObjectInstanceRef RefId="O-7_R-1" Priority="Low"/>"#,
        "O-7_R-1",
    );
    let b = device(
        "P-TEST-0_DI-2",
        r#"<ComObjectInstanceRef RefId="O-7_R-1" Priority="Alert"/><ComObjectInstanceRef RefId="O-7_R-2" Priority="High"/>"#,
        "O-7_R-1 O-7_R-2",
    );
    let conn = knx_store::open_and_migrate_in_memory().unwrap();
    let imported = import(&topology(&a, &b), &conn);
    let raws: Vec<_> = imported
        .project
        .devices
        .com_objects()
        .map(|c| match &c.saved_instance_priority {
            Fact::Declared(v) => v.raw.as_str(),
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(raws, ["Low", "Alert", "High"]);
    for duplicate in [topology(&a,&a),topology(&(a.clone()+&a),""),topology(&a,"").replace("</Installations>",&format!("<Installation InstallationId=\"1\"><Topology><UnassignedDevices>{a}</UnassignedDevices></Topology></Installation></Installations>"))] {
        let c=knx_store::open_and_migrate_in_memory().unwrap(); let imported=import(&duplicate,&c);
        for object in imported.project.devices.com_objects() { assert_eq!(object.saved_instance_priority,Fact::Unavailable {reason:Reason::AmbiguousOwner}); }
    }
    for pair in [
        ("Low", "High"),
        ("Low", "Low"),
        ("", ""),
        ("omitted", "omitted"),
    ] {
        let refs = [pair.0, pair.1]
            .iter()
            .map(|raw| {
                if *raw == "omitted" {
                    r#"<ComObjectInstanceRef RefId="O-7_R-1"/>"#.into()
                } else {
                    format!(r#"<ComObjectInstanceRef RefId="O-7_R-1" Priority="{raw}"/>"#)
                }
            })
            .collect::<String>();
        let c = knx_store::open_and_migrate_in_memory().unwrap();
        let imported = import(
            &topology(&device("P-TEST-0_DI-1", &refs, "O-7_R-1"), ""),
            &c,
        );
        assert_eq!(
            *fact(&imported.project),
            Fact::Unavailable {
                reason: Reason::AmbiguousDeclaration
            }
        );
        assert!(imported
            .report
            .errors
            .iter()
            .any(|e| e.detail.contains("AmbiguousObjectOverride")));
    }
    for refs in [
        r#"<ComObjectInstanceRef RefId="O-7_R-1" Priority="Low" Priority="High"/>"#,
        r#"<ComObjectInstanceRef Priority="Low"/>"#,
        r#"<ComObjectInstanceRef RefId="O-7_R-1" Priority="Low">"#,
    ] {
        let c = knx_store::open_and_migrate_in_memory().unwrap();
        let result = knx_app::import_ets_project_bytes(
            archive(&topology(&device("P-TEST-0_DI-1", refs, "O-7_R-1"), "")),
            "synthetic.knxproj",
            &c,
            Default::default(),
        );
        assert!(matches!(
            result,
            Err(knx_app::AppError::Import(
                knx_etsproj::ImportFailure::Parse(_)
            ))
        ));
        assert!(knx_store::load_opaque(&c).unwrap().is_empty());
    }
}

#[test]
fn saved_instance_priority_commands_working_history_and_source_rebound() {
    let conn = knx_store::open_and_migrate_in_memory().unwrap();
    let xml = topology(
        &device(
            "P-TEST-0_DI-1",
            r#"<ComObjectInstanceRef RefId="O-7_R-1" Priority="High"/>"#,
            "O-7_R-1",
        ),
        "",
    );
    let imported = import(&xml, &conn);
    let mut snapshot = knx_store::project_history::NativeSnapshot {
        project: imported.project,
        opaque: knx_store::load_opaque(&conn).unwrap(),
        manufacturer_refs: vec![],
    };
    let root = snapshot.clone();
    let id = snapshot.project.devices.iter().next().unwrap().id;
    let mut stack = CommandStack::new();
    let g = knx_store::project_history::save_editor(&conn, &snapshot, &stack, None, true).unwrap();
    stack
        .do_command(
            &mut snapshot.project,
            Command::RenameDevice {
                device: id,
                name: "Renamed".into(),
            },
        )
        .unwrap();
    stack
        .do_command(
            &mut snapshot.project,
            Command::SetGroupAddressStyle {
                style: knx_core::GroupAddressStyle::Free,
            },
        )
        .unwrap();
    stack.undo(&mut snapshot.project).unwrap();
    let g =
        knx_store::project_history::save_editor(&conn, &snapshot, &stack, Some(g), false).unwrap();
    assert_eq!(knx_store::load_project(&conn).unwrap(), root.project);
    let recovered = knx_store::project_history::load_editor(&conn)
        .unwrap()
        .unwrap();
    assert_eq!(recovered.working, snapshot);
    assert_eq!(*fact(&recovered.working.project), *fact(&root.project));
    let view = knx_projection::build_device_detail(&recovered.working.project, id).unwrap();
    assert_eq!(view.name, "Renamed");
    assert!(
        matches!(&view.com_objects[0].saved_instance_priority,View::Declared{raw,..} if raw=="High")
    );
    assert!(g > 0);
    stack
        .do_command(&mut snapshot.project, Command::DeleteDevice { id })
        .unwrap();
    assert_eq!(snapshot.project.devices.com_objects().count(), 0);
    stack.undo(&mut snapshot.project).unwrap();
    assert_eq!(*fact(&snapshot.project), *fact(&root.project));
    stack.redo(&mut snapshot.project).unwrap();
    stack.undo(&mut snapshot.project).unwrap();
    assert_eq!(*fact(&snapshot.project), *fact(&root.project));
    let object = snapshot.project.devices.com_objects().next().unwrap().id;
    snapshot
        .project
        .devices
        .com_object_mut(object)
        .unwrap()
        .source
        .path = "scoped-copy/0.xml".into();
    knx_store::save_project_with_passthrough(&conn, &snapshot.project, &snapshot.opaque, &[])
        .unwrap();
    let rebound = knx_store::load_project(&conn).unwrap();
    assert_eq!(*fact(&rebound), *fact(&root.project));
    let view = knx_projection::build_device_detail(&rebound, id).unwrap();
    assert_eq!(
        view.com_objects[0].saved_instance_priority,
        View::Unavailable {
            reason: knx_projection::SavedPriorityUnavailableView::SourceBindingMismatch
        }
    );
}

#[test]
fn saved_instance_priority_actual_product_enrichment_remains_independent() {
    let xml = topology(
        &device(
            "P-TEST-0_DI-1",
            r#"<ComObjectInstanceRef RefId="O-7_R-1" Priority="High"/>"#,
            "O-7_R-1",
        ),
        "",
    );
    let plain = knx_store::open_and_migrate_in_memory().unwrap();
    let expected = import(&xml, &plain).project;
    let dir = tempfile::tempdir().unwrap();
    let products = knx_productdb::open_and_migrate(&dir.path().join("catalogue.sqlite")).unwrap();
    let hardware=br#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-TEST"><Hardware><Hardware Id="M-TEST_H-1" Name="Synthetic" SerialNumber="S" VersionNumber="1"><Products><Product Id="M-TEST_P-1"/></Products><Hardware2Programs><Hardware2Program Id="M-TEST_H-1_HP-1" MediumTypes="MT-0"><ApplicationProgramRef RefId="M-TEST_A-1"/></Hardware2Program></Hardware2Programs></Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;
    let program=br#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-TEST"><ApplicationPrograms><ApplicationProgram Id="M-TEST_A-1" Name="Synthetic" ApplicationVersion="1" MaskVersion="MV-0701"><Static><ComObjectTable><ComObject Id="M-TEST_A-1_O-7" Number="7" Text="Product text" ObjectSize="1 Bit" Priority="Alert"/></ComObjectTable><ComObjectRefs><ComObjectRef Id="M-TEST_A-1_O-7_R-1" RefId="M-TEST_A-1_O-7" Priority="Low"/></ComObjectRefs></Static></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;
    knx_productdb::parse::hardware::ingest_hardware(
        &products,
        "synthetic-h",
        "M-TEST/Hardware.xml",
        hardware,
    )
    .unwrap();
    knx_productdb::parse::program::ingest_program(
        &products,
        "synthetic-p",
        "M-TEST/A.xml",
        program,
    )
    .unwrap();
    let conn = knx_store::open_and_migrate_in_memory().unwrap();
    let enriched = knx_app::import_ets_project_bytes(
        archive(&xml),
        "synthetic.knxproj",
        &conn,
        knx_app::ImportOptions {
            product_db: Some(&products),
        },
    )
    .unwrap();
    assert_eq!(enriched.enrichment.unwrap().com_objects_enriched, 1);
    assert_eq!(fact(&enriched.project), fact(&expected));
    let com = enriched.project.devices.com_objects().next().unwrap();
    assert!(com.text.value().is_some());
    let detail = knx_projection::build_device_detail(&enriched.project, com.device).unwrap();
    assert!(
        matches!(&detail.com_objects[0].saved_instance_priority,View::Declared{raw,..} if raw=="High")
    );
}

#[test]
fn saved_instance_priority_resource_limit_withholds_whole_import_population_without_opaque_loss() {
    let oversized = "ü".repeat(2049);
    let refs = format!(
        r#"<ComObjectInstanceRef RefId="O-7_R-1" Priority="Low"/><ComObjectInstanceRef RefId="O-8_R-1" Priority="{oversized}"/>"#
    );
    let xml = topology(&device("P-TEST-0_DI-1", &refs, "O-7_R-1 O-8_R-1"), "");
    let conn = knx_store::open_and_migrate_in_memory().unwrap();
    let imported = import(&xml, &conn);
    assert_eq!(imported.project.devices.com_objects().count(), 2);
    for object in imported.project.devices.com_objects() {
        assert_eq!(
            object.saved_instance_priority,
            Fact::Unavailable {
                reason: Reason::ResourceLimit
            }
        );
    }
    let opaque = knx_store::load_opaque(&conn).unwrap();
    assert!(opaque.iter().any(|e| e.name == "Priority"
        && e.bytes == oversized.as_bytes()
        && e.sha256 == knx_etsproj::opaque::sha256_hex(oversized.as_bytes())));
    assert!(opaque
        .iter()
        .any(|e| e.xpath.is_empty() && e.source_path == MEMBER && e.bytes == xml.as_bytes()));
    knx_store::save_project(&conn, &imported.project).unwrap();
    assert_eq!(knx_store::load_project(&conn).unwrap(), imported.project);
    assert_eq!(knx_store::load_opaque(&conn).unwrap(), opaque);
}
