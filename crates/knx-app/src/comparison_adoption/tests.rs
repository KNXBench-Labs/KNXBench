//! Public offline comparison → preview → adoption → simulator re-comparison.
use super::*;
use knx_core::{
    Command, CompletionStatus, DeviceInstance, Installation, InstallationId, Language, SourceRef,
    Topology,
};
use knx_net::commissioning::simulator::{SimulatedDevice, SimulatorConfig};

const PROGRAM: &str = r#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0001"><ApplicationPrograms>
<ApplicationProgram Id="M-0001_A-0001-01-0000" Name="Synthetic" ApplicationNumber="1" ApplicationVersion="1" PeiType="1" MaskVersion="MV-0701" LoadProcedureStyle="ProductProcedure"><Static>
<Code><AbsoluteSegment Id="S" Address="17408" Size="4"><Data>AQIDBA==</Data></AbsoluteSegment></Code>
<ParameterTypes><ParameterType Id="T"><TypeNumber SizeInBit="8" Type="unsignedInt" minInclusive="0" maxInclusive="255" /></ParameterType>
<ParameterType Id="Signed"><TypeNumber SizeInBit="8" Type="signedInt" minInclusive="0" maxInclusive="127" /></ParameterType></ParameterTypes>
<Parameters><Parameter Id="P1" Name="First" ParameterType="T" Value="1"><Memory CodeSegment="S" Offset="0" BitOffset="0" /></Parameter>
<Parameter Id="P2" Name="Second" ParameterType="T" Value="2"><Memory CodeSegment="S" Offset="1" BitOffset="0" /></Parameter>
<Parameter Id="P3" Name="Unknown device pattern" ParameterType="Signed" Value="3"><Memory CodeSegment="S" Offset="2" BitOffset="0" /></Parameter></Parameters>
<ParameterRefs><ParameterRef Id="M-0001_A-0001-01-0000_P-1_R-1" RefId="P1" /><ParameterRef Id="M-0001_A-0001-01-0000_P-2_R-2" RefId="P2" /><ParameterRef Id="M-0001_A-0001-01-0000_P-3_R-3" RefId="P3" /></ParameterRefs>
<LoadProcedures><LoadProcedure><LdCtrlConnect /><LdCtrlUnload LsmIdx="3" /><LdCtrlLoad LsmIdx="3" />
<LdCtrlAbsSegment LsmIdx="3" SegType="0" Address="17408" Size="4" Access="255" MemType="3" SegFlags="128" />
<LdCtrlTaskSegment LsmIdx="3" Address="17408" /><LdCtrlLoadCompleted LsmIdx="3" /><LdCtrlDisconnect /></LoadProcedure></LoadProcedures>
</Static><Dynamic><ChannelIndependentBlock><ParameterRefRef RefId="M-0001_A-0001-01-0000_P-1_R-1" /><ParameterRefRef RefId="M-0001_A-0001-01-0000_P-2_R-2" /><ParameterRefRef RefId="M-0001_A-0001-01-0000_P-3_R-3" /></ChannelIndependentBlock></Dynamic>
</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;
const HARDWARE: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0001"><Hardware><Hardware Id="H" Name="Synthetic" SerialNumber="synthetic" VersionNumber="1"><Products><Product Id="P" /></Products><Hardware2Programs><Hardware2Program Id="HP" MediumTypes="MT-0"><ApplicationProgramRef RefId="M-0001_A-0001-01-0000" /></Hardware2Program></Hardware2Programs></Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;
fn fixture() -> (
    tempfile::TempDir,
    Connection,
    NativeSnapshot,
    CommandStack,
    ComparisonScope,
    SimulatedDevice,
) {
    fixture_with(PROGRAM)
}
fn fixture_with(
    program: &str,
) -> (
    tempfile::TempDir,
    Connection,
    NativeSnapshot,
    CommandStack,
    ComparisonScope,
    SimulatedDevice,
) {
    let dir = tempfile::tempdir().unwrap();
    let db = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    knx_productdb::ingest_file(&db, "program.xml", program.as_bytes()).unwrap();
    knx_productdb::ingest_file(&db, "Hardware.xml", HARDWARE).unwrap();
    let mut project = Project::new(Language("en".into()));
    let id = project.ids.next_device_id().unwrap();
    project.installations.push(Installation {
        id: InstallationId(0),
        name: "I".into(),
        default_line: None,
        multicast_address: None,
        completion: CompletionStatus::FinishedDesign,
        topology: Topology {
            areas: vec![],
            lines: vec![],
            unassigned: vec![id],
        },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses: vec![],
        parameters: vec![],
    });
    project.devices.insert(DeviceInstance {
        id,
        source: SourceRef {
            path: "synthetic".into(),
            ets_id: "D".into(),
        },
        name: "D".into(),
        description: None,
        address: Some("1.1.7".parse().unwrap()),
        product_ref: "P".into(),
        program_ref: "HP".into(),
        commissioning: Default::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
    let parameter_id = project.ids.next_parameter_instance_id().unwrap();
    project.installations[0]
        .parameters
        .push(knx_core::ParameterInstance {
            id: parameter_id,
            device: id,
            source: SourceRef {
                path: "retained".into(),
                ets_id: "M-0001_A-0001-01-0000_P-3_R-3".into(),
            },
            raw: "3".into(),
        });
    let snapshot = NativeSnapshot {
        project,
        opaque: vec![],
        manufacturer_refs: vec![],
    };
    let sim = SimulatedDevice::with_config_at(
        "1.1.7".parse().unwrap(),
        SimulatorConfig {
            mask_version: 0x0701,
            ..Default::default()
        },
    );
    sim.preset_property(0, 12, &[0, 1]);
    sim.preset_property(3, 13, &[0, 1, 0, 1, 1]);
    sim.preset_memory(0x4400, &[17, 29, 0xFF, 0xAB]);
    (
        dir,
        db,
        snapshot,
        CommandStack::new(),
        ComparisonScope {
            server_incarnation: "server-test".into(),
            project_incarnation: 1,
        },
        sim,
    )
}
async fn observe(
    db: &Connection,
    snapshot: &NativeSnapshot,
    stack: &CommandStack,
    scope: &ComparisonScope,
    sim: &SimulatedDevice,
) -> ObservedComparison {
    prepare_comparison(db, &snapshot.project, stack, scope, DeviceId(1))
        .unwrap()
        .observe(sim, Default::default())
        .await
        .unwrap()
}
fn selection(observed: &ObservedComparison) -> Vec<usize> {
    observed
        .rows()
        .iter()
        .enumerate()
        .filter(|(_, r)| r.kind == "parameter" && r.device.is_some())
        .map(|(i, _)| i)
        .collect()
}
#[tokio::test]
async fn adoption_preserves_unknown_rows_and_one_undo_then_recompares_selected_fields() {
    let (_dir, db, mut snapshot, mut stack, scope, sim) = fixture();
    let observed = observe(&db, &snapshot, &stack, &scope, &sim).await;
    let selected = selection(&observed);
    assert_eq!(selected.len(), 2);
    assert!(observed
        .rows()
        .iter()
        .any(|r| r.kind == "parameter" && r.device.is_none()));
    assert!(observed.rows().iter().any(|r| r.kind == "raw"));
    let before = snapshot.clone();
    let preview = preview_adoption(&db, &snapshot, &stack, &scope, &observed, &selected).unwrap();
    assert_eq!(
        preview.rows(),
        observed.rows(),
        "all unsupported evidence retained"
    );
    assert_eq!(snapshot, before);
    let outcome = apply_adoption(&db, &mut snapshot, &mut stack, &scope, &preview, None).unwrap();
    assert!(outcome.changed);
    assert_eq!(stack.history_lengths(), (1, 0));
    assert_eq!(snapshot.project.installations[0].parameters.len(), 3);
    assert_eq!(
        snapshot.project.installations[0].parameters[0],
        before.project.installations[0].parameters[0],
        "unselected unknown device field leaves the exact retained row untouched"
    );
    let after = snapshot.clone();
    let compared = observe(&db, &snapshot, &stack, &scope, &sim).await;
    for i in selection(&compared) {
        assert_eq!(compared.rows()[i].project, compared.rows()[i].device);
    }
    let no_op = preview_adoption(
        &db,
        &snapshot,
        &stack,
        &scope,
        &compared,
        &selection(&compared),
    )
    .unwrap();
    assert!(
        !apply_adoption(&db, &mut snapshot, &mut stack, &scope, &no_op, None)
            .unwrap()
            .changed
    );
    assert_eq!(stack.history_lengths(), (1, 0));
    assert!(!sim.memory_was_written());
    assert_eq!(sim.authorize_requests(), 0);
    stack.undo(&mut snapshot.project).unwrap();
    assert!(snapshot.project.same_user_content_as(&before.project));
    assert_eq!(snapshot.opaque, before.opaque);
    stack.redo(&mut snapshot.project).unwrap();
    assert_eq!(snapshot, after);
    assert!(matches!(
        apply_adoption(&db, &mut snapshot, &mut stack, &scope, &preview, None),
        Err(AdoptionError::Stale)
    ));
}
#[tokio::test]
async fn unsupported_requested_rows_refuse_without_silently_dropping_selection() {
    let (_dir, db, snapshot, stack, scope, sim) = fixture();
    let observed = observe(&db, &snapshot, &stack, &scope, &sim).await;
    let mut selected = selection(&observed);
    let raw = observed
        .rows()
        .iter()
        .position(|r| r.kind == "raw")
        .unwrap();
    selected.push(raw);
    assert!(
        matches!(preview_adoption(&db, &snapshot, &stack, &scope, &observed, &selected), Err(AdoptionError::UnsupportedRow { index, .. }) if index == raw)
    );
    let unknown = observed
        .rows()
        .iter()
        .position(|r| r.kind == "parameter" && r.device.is_none())
        .unwrap();
    assert!(
        matches!(preview_adoption(&db, &snapshot, &stack, &scope, &observed, &[unknown]), Err(AdoptionError::UnknownRow { index }) if index == unknown)
    );
    assert!(matches!(
        preview_adoption(&db, &snapshot, &stack, &scope, &observed, &[0, 0]),
        Err(AdoptionError::DuplicateRow(0))
    ));
    assert!(preview_adoption(&db, &snapshot, &stack, &scope, &observed, &[usize::MAX]).is_err());
    assert!(preview_adoption(&db, &snapshot, &stack, &scope, &observed, &[]).is_err());
    assert_eq!(stack.history_lengths(), (0, 0));
}
#[tokio::test]
async fn exact_source_scope_revision_and_history_are_rechecked_before_publication() {
    let (_dir, db, mut snapshot, mut stack, scope, sim) = fixture();
    let observed = observe(&db, &snapshot, &stack, &scope, &sim).await;
    let preview = preview_adoption(
        &db,
        &snapshot,
        &stack,
        &scope,
        &observed,
        &selection(&observed),
    )
    .unwrap();
    let before = snapshot.clone();
    let other = ComparisonScope {
        project_incarnation: 2,
        ..scope.clone()
    };
    assert!(matches!(
        apply_adoption(&db, &mut snapshot, &mut stack, &other, &preview, None),
        Err(AdoptionError::Stale)
    ));
    stack
        .do_command(
            &mut snapshot.project,
            Command::RenameDevice {
                device: DeviceId(1),
                name: "renamed".into(),
            },
        )
        .unwrap();
    stack.undo(&mut snapshot.project).unwrap();
    assert_eq!(snapshot, before);
    assert!(matches!(
        apply_adoption(&db, &mut snapshot, &mut stack, &scope, &preview, None),
        Err(AdoptionError::Stale)
    ));
    let mut clean_stack = CommandStack::new();
    let changed_db =
        knx_productdb::open_and_migrate(&_dir.path().join("changed-products.sqlite")).unwrap();
    knx_productdb::ingest_file(
        &changed_db,
        "program.xml",
        PROGRAM
            .replace("Name=\"First\"", "Name=\"Changed source\"")
            .as_bytes(),
    )
    .unwrap();
    knx_productdb::ingest_file(&changed_db, "Hardware.xml", HARDWARE).unwrap();
    assert!(matches!(
        apply_adoption(
            &changed_db,
            &mut snapshot,
            &mut clean_stack,
            &scope,
            &preview,
            None
        ),
        Err(AdoptionError::SourceChanged)
    ));
    assert_eq!(snapshot, before);
    assert_eq!(clean_stack.history_lengths(), (0, 0));
}
#[tokio::test]
async fn wrong_application_and_persistence_failure_never_publish_an_edit() {
    let (dir, db, mut snapshot, mut stack, scope, sim) = fixture();
    sim.preset_property(3, 13, &[0, 1, 0, 2, 1]);
    let result = prepare_comparison(&db, &snapshot.project, &stack, &scope, DeviceId(1))
        .unwrap()
        .observe(&sim, Default::default())
        .await;
    assert!(matches!(
        result,
        Err(AdoptionError::Compare(
            CompareError::ApplicationMismatch { .. }
        ))
    ));
    assert!(!sim.memory_was_written());
    sim.preset_property(3, 13, &[0, 1, 0, 1, 1]);
    let observed = observe(&db, &snapshot, &stack, &scope, &sim).await;
    let preview = preview_adoption(
        &db,
        &snapshot,
        &stack,
        &scope,
        &observed,
        &selection(&observed),
    )
    .unwrap();
    let before = snapshot.clone();
    let path = dir.path().join("missing.knxdb");
    assert!(matches!(
        apply_adoption(
            &db,
            &mut snapshot,
            &mut stack,
            &scope,
            &preview,
            Some(AdoptionNativeTarget {
                path: &path,
                history_generation: 0
            })
        ),
        Err(AdoptionError::Native(_))
    ));
    assert_eq!(snapshot, before);
    assert_eq!(stack.history_lengths(), (0, 0));
    let mut malformed_history = before.project.clone();
    malformed_history.installations[0]
        .topology
        .unassigned
        .push(DeviceId(1));
    let malformed_stack = CommandStack::from_snapshot_states(vec![malformed_history], vec![]);
    assert!(
        matches!(
            preview_adoption(
                &db,
                &snapshot,
                &malformed_stack,
                &scope,
                &observed,
                &selection(&observed)
            ),
            Err(AdoptionError::Native(_))
        ),
        "the entire prior history must be natively representable"
    );
    assert_eq!(snapshot, before);
    let generation = crate::project_history::save_as(&path, &snapshot, &stack).unwrap();
    let outcome = apply_adoption(
        &db,
        &mut snapshot,
        &mut stack,
        &scope,
        &preview,
        Some(AdoptionNativeTarget {
            path: &path,
            history_generation: generation,
        }),
    )
    .unwrap();
    let persisted = knx_store::project_history::open_editor_store(&path, false).unwrap();
    let reopened = knx_store::project_history::load_editor(&persisted)
        .unwrap()
        .unwrap();
    assert_eq!(Some(reopened.generation), outcome.native_generation);
    assert_eq!(reopened.working, snapshot);
    assert_eq!(reopened.undo.len(), 1);
}

#[tokio::test]
async fn observation_uses_p3_content_not_labels_but_rejects_edit_undo_aba() {
    let (_dir, db, mut snapshot, mut stack, scope, sim) = fixture();
    let observed = observe(&db, &snapshot, &stack, &scope, &sim).await;
    stack
        .do_command(
            &mut snapshot.project,
            Command::RenameDevice {
                device: DeviceId(1),
                name: "Other label".into(),
            },
        )
        .unwrap();
    assert!(
        observed.is_current(&snapshot.project, &stack, &scope),
        "labels do not invalidate P3 evidence"
    );
    let preview = preview_adoption(
        &db,
        &snapshot,
        &stack,
        &scope,
        &observed,
        &selection(&observed),
    )
    .unwrap();
    apply_adoption(&db, &mut snapshot, &mut stack, &scope, &preview, None).unwrap();
    stack.undo(&mut snapshot.project).unwrap();
    assert!(
        !observed.is_current(&snapshot.project, &stack, &scope),
        "edit/undo never revives an old read"
    );
    assert!(matches!(
        preview_adoption(
            &db,
            &snapshot,
            &stack,
            &scope,
            &observed,
            &selection(&observed)
        ),
        Err(AdoptionError::Stale)
    ));
}

#[tokio::test]
async fn ambiguous_source_or_parameter_placement_refuses_the_entire_adoption() {
    let program = PROGRAM
        .replace("Offset=\"1\"", "Offset=\"0\"")
        .replace("Value=\"2\"", "Value=\"1\"");
    let (_dir, db, snapshot, stack, scope, sim) = fixture_with(&program);
    assert!(
        matches!(
            prepare_comparison(&db, &snapshot.project, &stack, &scope, DeviceId(1)),
            Err(AdoptionError::Preparation(_))
        ),
        "existing image rules refuse overlapping active fields before a read"
    );
    assert!(sim.seen().is_empty());
    let (_dir, db, mut snapshot, stack, scope, sim) = fixture();
    for _ in 0..2 {
        let id = snapshot.project.ids.next_parameter_instance_id().unwrap();
        snapshot.project.installations[0]
            .parameters
            .push(knx_core::ParameterInstance {
                id,
                device: DeviceId(1),
                source: SourceRef {
                    path: "retained".into(),
                    ets_id: "M-0001_A-0001-01-0000_P-1_R-1".into(),
                },
                raw: "1".into(),
            });
    }
    let observed = observe(&db, &snapshot, &stack, &scope, &sim).await;
    assert!(matches!(
        preview_adoption(
            &db,
            &snapshot,
            &stack,
            &scope,
            &observed,
            &selection(&observed)
        ),
        Err(AdoptionError::Command(_))
    ));
    assert_eq!(stack.history_lengths(), (0, 0));
}
