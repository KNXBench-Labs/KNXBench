//! Association rows → object ↔ group-address links, on a simulated `0701h` device.
//!
//! The synthetic program is `knx_testsupport::LINK_ADOPTION_PROGRAM_XML`.
use super::*;
use knx_core::{
    ComObjectInstance, Command, CompletionStatus, DeviceInstance, GroupAddressEntry, GroupLink,
    Installation, InstallationId, Language, Override, SourceRef, Topology,
};
use knx_net::commissioning::simulator::{SimulatedDevice, SimulatorConfig};

const ID: &str = "M-0001_A-0002-01-0000";
const GA_1: u16 = 0x0801; // 1/0/1
const GA_2: u16 = 0x0802; // 1/0/2
const GA_3: u16 = 0x0803; // 1/0/3
/// Device address table: 1.1.7, 1/0/1, 1/0/2, 1/0/3.
const ADDRESSES: [u8; 11] = [4, 0x11, 0x07, 0x08, 0x01, 0x08, 0x02, 0x08, 0x03, 0, 0];
/// Object 0 sends on 1/0/2; object 1 sends on 1/0/1 and receives 1/0/3.
const ASSOCIATIONS: [u8; 9] = [3, 2, 0, 1, 1, 3, 1, 0, 0];

struct Fixture {
    _dir: tempfile::TempDir,
    db: Connection,
    snapshot: NativeSnapshot,
    stack: CommandStack,
    scope: ComparisonScope,
    sim: SimulatedDevice,
}

fn object(id: u32, number: u16, links: Vec<GroupLink>) -> ComObjectInstance {
    ComObjectInstance {
        id: ComObjectInstanceId(id),
        source: SourceRef {
            path: "synthetic".into(),
            ets_id: format!("{ID}_O-{number}_R-{}", number + 1),
        },
        device: DeviceId(1),
        number,
        text: Override::Absent,
        description: Override::Absent,
        dpt: Override::Absent,
        flags: knx_core::ResolvedFlags::none(),
        size: None,
        is_active: true,
        links,
        module_instance: None,
    }
}

fn add_group_address(project: &mut Project, raw: u16) -> GroupAddressId {
    let id = project.ids.next_group_address_id().unwrap();
    project.installations[0]
        .group_addresses
        .push(GroupAddressEntry {
            id,
            source: SourceRef {
                path: "synthetic".into(),
                ets_id: format!("GA-{}", id.0),
            },
            name: format!("GA {raw:04X}"),
            address: GroupAddress::from_raw(raw),
            central: false,
            unfiltered: false,
            range: None,
            declared_dpt: Default::default(),
        });
    id
}

fn add_object(project: &mut Project, number: u16, links: Vec<GroupLink>) -> ComObjectInstanceId {
    let id = project.ids.next_com_object_instance_id().unwrap();
    project
        .devices
        .insert_com_object(object(id.0, number, links));
    project
        .devices
        .get_mut(DeviceId(1))
        .unwrap()
        .com_objects
        .push(id);
    id
}

/// Project: object 0 sends on 1/0/1; objects 1 and 2 have no links.
fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let db = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    knx_productdb::ingest_file(
        &db,
        "program.xml",
        knx_testsupport::LINK_ADOPTION_PROGRAM_XML.as_bytes(),
    )
    .unwrap();
    knx_productdb::ingest_file(
        &db,
        "Hardware.xml",
        knx_testsupport::LINK_ADOPTION_HARDWARE_XML.as_bytes(),
    )
    .unwrap();
    let mut project = Project::new(Language("en".into()));
    let device = project.ids.next_device_id().unwrap();
    project.installations.push(Installation {
        id: InstallationId(0),
        name: "I".into(),
        default_line: None,
        multicast_address: None,
        completion: CompletionStatus::FinishedDesign,
        topology: Topology {
            areas: vec![],
            lines: vec![],
            unassigned: vec![device],
        },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses: vec![],
        parameters: vec![],
    });
    project.devices.insert(DeviceInstance {
        id: device,
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
    let first = add_group_address(&mut project, GA_1);
    add_group_address(&mut project, GA_2);
    add_group_address(&mut project, GA_3);
    add_object(
        &mut project,
        0,
        vec![GroupLink {
            ga: first,
            direction: Direction::Send,
        }],
    );
    add_object(&mut project, 1, vec![]);
    add_object(&mut project, 2, vec![]);
    let sim = SimulatedDevice::with_config_at(
        "1.1.7".parse().unwrap(),
        SimulatorConfig {
            mask_version: 0x0701,
            ..Default::default()
        },
    );
    sim.preset_property(0, 12, &[0, 1]);
    sim.preset_property(3, 13, &[0, 1, 0, 2, 1]);
    let fixture = Fixture {
        _dir: dir,
        db,
        snapshot: NativeSnapshot {
            project,
            opaque: vec![],
            manufacturer_refs: vec![],
        },
        stack: CommandStack::new(),
        scope: ComparisonScope {
            server_incarnation: "server-test".into(),
            project_incarnation: 1,
        },
        sim,
    };
    // The object table and parameter as the project writes them; tables differ.
    let prepared = prepare_comparison(
        &fixture.db,
        &fixture.snapshot.project,
        &fixture.stack,
        &fixture.scope,
        DeviceId(1),
    )
    .unwrap();
    for step in &prepared.plan().steps {
        if let knx_core::commissioning::memory_download::MemoryDownloadStep::WriteMemory {
            address,
            octets,
        } = step
        {
            fixture.sim.preset_memory(u32::from(*address), octets);
        }
    }
    fixture.sim.preset_memory(0x4000, &ADDRESSES);
    fixture.sim.preset_memory(0x4100, &ASSOCIATIONS);
    fixture
}

impl Fixture {
    async fn observe(&self) -> ObservedComparison {
        prepare_comparison(
            &self.db,
            &self.snapshot.project,
            &self.stack,
            &self.scope,
            DeviceId(1),
        )
        .unwrap()
        .observe(&self.sim, Default::default())
        .await
        .unwrap()
    }
    fn preview(
        &self,
        observed: &ObservedComparison,
        selected: &[usize],
    ) -> Result<PreparedAdoption, AdoptionError> {
        preview_adoption(
            &self.db,
            &self.snapshot,
            &self.stack,
            &self.scope,
            observed,
            selected,
        )
    }
}

fn rows_of(observed: &ObservedComparison, kind: &str) -> Vec<usize> {
    observed
        .rows()
        .iter()
        .enumerate()
        .filter(|(_, r)| r.kind == kind)
        .map(|(i, _)| i)
        .collect()
}

fn links_of(project: &Project, number: u16) -> Vec<(GroupAddress, Direction)> {
    let addresses: BTreeMap<GroupAddressId, GroupAddress> = project.installations[0]
        .group_addresses
        .iter()
        .map(|g| (g.id, g.address))
        .collect();
    project
        .devices
        .com_objects()
        .find(|o| o.device == DeviceId(1) && o.number == number)
        .unwrap()
        .links
        .iter()
        .map(|l| (addresses[&l.ga], l.direction))
        .collect()
}

fn ga(raw: u16) -> GroupAddress {
    GroupAddress::from_raw(raw)
}

#[tokio::test]
async fn association_rows_adopt_links_in_one_undo_step_and_recompare_equal() {
    let mut f = fixture();
    let observed = f.observe().await;
    let associations = rows_of(&observed, "association");
    assert_eq!(associations.len(), 3);
    assert!(associations
        .iter()
        .all(|&i| adoption_hint(&observed.rows()[i])));
    for kind in ["addressCount", "groupAddress", "associationCount", "raw"] {
        for i in rows_of(&observed, kind) {
            assert!(
                !adoption_hint(&observed.rows()[i]),
                "{kind} is never adoptable"
            );
        }
    }
    let before = f.snapshot.clone();
    let preview = f.preview(&observed, &associations).unwrap();
    assert!(preview.changed());
    assert_eq!(f.snapshot, before, "preview is read-only");
    let changes: Vec<_> = preview
        .link_changes()
        .iter()
        .map(|c| (c.add, c.object, c.group_address, c.sending))
        .collect();
    assert_eq!(
        changes,
        vec![
            (false, 0, ga(GA_1), true),
            (true, 0, ga(GA_2), true),
            (true, 1, ga(GA_1), true),
            (true, 1, ga(GA_3), false),
        ]
    );
    let outcome = apply_adoption(
        &f.db,
        &mut f.snapshot,
        &mut f.stack,
        &f.scope,
        &preview,
        None,
    )
    .unwrap();
    assert!(outcome.changed);
    assert_eq!(f.stack.history_lengths(), (1, 0));
    assert_eq!(
        links_of(&f.snapshot.project, 0),
        vec![(ga(GA_2), Direction::Send)]
    );
    assert_eq!(
        links_of(&f.snapshot.project, 1),
        vec![(ga(GA_1), Direction::Send), (ga(GA_3), Direction::Receive)]
    );
    assert_eq!(
        f.snapshot.project.installations[0].group_addresses.len(),
        3,
        "no group address created"
    );
    let after = f.snapshot.clone();
    let compared = f.observe().await;
    for kind in [
        "addressCount",
        "groupAddress",
        "individualAddress",
        "associationCount",
        "association",
    ] {
        for i in rows_of(&compared, kind) {
            let row = &compared.rows()[i];
            assert_eq!(row.project, row.device, "{kind} row {i} re-compares equal");
            assert!(row.project.is_some());
        }
    }
    let no_op = f
        .preview(&compared, &rows_of(&compared, "association"))
        .unwrap();
    assert!(!no_op.changed());
    assert!(no_op.link_changes().is_empty());
    assert!(!f.sim.memory_was_written());
    assert_eq!(f.sim.authorize_requests(), 0);
    f.stack.undo(&mut f.snapshot.project).unwrap();
    assert!(f.snapshot.project.same_user_content_as(&before.project));
    assert_eq!(
        links_of(&f.snapshot.project, 0),
        links_of(&before.project, 0)
    );
    assert!(links_of(&f.snapshot.project, 1).is_empty());
    f.stack.redo(&mut f.snapshot.project).unwrap();
    assert_eq!(f.snapshot, after);
}

#[tokio::test]
async fn object_granularity_mixed_unknown_and_unsupported_rows_refuse_the_whole_selection() {
    let f = fixture();
    let observed = f.observe().await;
    let a = rows_of(&observed, "association");
    // Row 1 names object 1; row 2 differs for object 1 too.
    assert!(matches!(
        f.preview(&observed, &[a[1]]),
        Err(AdoptionError::IncompleteObjectSelection { object: 1, index }) if index == a[2]
    ));
    // Object 1 alone is a complete object: object 0 stays as it is.
    let object_one = f.preview(&observed, &[a[1], a[2]]).unwrap();
    assert!(object_one
        .link_changes()
        .iter()
        .all(|c| c.add && c.object == 1));
    for kind in ["groupAddress", "associationCount", "raw"] {
        let other = rows_of(&observed, kind)[0];
        assert!(
            matches!(f.preview(&observed, &[a[0], a[1], a[2], other]), Err(AdoptionError::UnsupportedRow { index, .. }) if index == other),
            "{kind} cannot ride along"
        );
    }
    // An unknown device entry (TSAP 0) leaves the device link set unknown.
    f.sim.preset_memory(0x4100 + 5, &[0]);
    let unknown = f.observe().await;
    let a = rows_of(&unknown, "association");
    assert_eq!(unknown.rows()[a[2]].device, None);
    assert!(!adoption_hint(&unknown.rows()[a[2]]));
    assert!(matches!(
        f.preview(&unknown, &[a[0]]),
        Err(AdoptionError::IncompleteAssociationTable { index }) if index == a[2]
    ));
    assert!(matches!(
        f.preview(&unknown, &[a[0], a[1], a[2]]),
        Err(AdoptionError::UnknownRow { index }) if index == a[2]
    ));
    // The unselected unknown association row does not block a parameter.
    let parameter = rows_of(&unknown, "parameter")[0];
    assert!(f.preview(&unknown, &[parameter]).is_ok());
    assert!(matches!(
        f.preview(&unknown, &[parameter, a[0]]),
        Err(AdoptionError::IncompleteAssociationTable { .. })
    ));
    assert_eq!(f.stack.history_lengths(), (0, 0));
}

#[tokio::test]
async fn missing_or_ambiguous_group_addresses_and_objects_refuse() {
    // Missing: 1/0/3 is not in the project; adoption never creates it.
    let mut f = fixture();
    f.snapshot.project.installations[0]
        .group_addresses
        .retain(|g| g.address != ga(GA_3));
    let observed = f.observe().await;
    let a = rows_of(&observed, "association");
    assert!(matches!(
        f.preview(&observed, &a),
        Err(AdoptionError::GroupAddressNotInProject { index, group_address })
            if index == a[2] && group_address == ga(GA_3)
    ));
    // Ambiguous: two project group addresses are 1/0/3.
    let mut f = fixture();
    add_group_address(&mut f.snapshot.project, GA_3);
    let observed = f.observe().await;
    assert!(matches!(
        f.preview(&observed, &rows_of(&observed, "association")),
        Err(AdoptionError::AmbiguousGroupAddress { group_address, .. }) if group_address == ga(GA_3)
    ));
    // Ambiguous: two active instances with number 1.
    let mut f = fixture();
    add_object(&mut f.snapshot.project, 1, vec![]);
    let observed = f.observe().await;
    assert!(matches!(
        f.preview(&observed, &rows_of(&observed, "association")),
        Err(AdoptionError::AmbiguousObject { object: 1, .. })
    ));
    // Module-based instance with the number: refused, not resolved.
    let mut f = fixture();
    let module = add_object(&mut f.snapshot.project, 1, vec![]);
    f.snapshot
        .project
        .devices
        .com_object_mut(module)
        .unwrap()
        .module_instance = Some(knx_core::ModuleInstanceId(1));
    let observed = f.observe().await;
    assert!(matches!(
        f.preview(&observed, &rows_of(&observed, "association")),
        Err(AdoptionError::AmbiguousObject { object: 1, .. })
    ));
}

#[tokio::test]
async fn inactive_objects_need_the_activating_parameter_in_the_same_adoption() {
    let mut f = fixture();
    // Object 2 instead of 1 for 1/0/3, and the device's parameter activates it.
    f.sim.preset_memory(0x4100, &[3, 2, 0, 1, 1, 3, 2, 0, 0]);
    f.sim.preset_memory(0x4400 + 15, &[1]);
    let observed = f.observe().await;
    let a = rows_of(&observed, "association");
    assert!(matches!(
        f.preview(&observed, &a),
        Err(AdoptionError::ObjectNotActive { object: 2, .. })
    ));
    let parameter = rows_of(&observed, "parameter")[0];
    assert_eq!(observed.rows()[parameter].device.as_deref(), Some("1"));
    let mut selected = a.clone();
    selected.push(parameter);
    let preview = f.preview(&observed, &selected).unwrap();
    apply_adoption(
        &f.db,
        &mut f.snapshot,
        &mut f.stack,
        &f.scope,
        &preview,
        None,
    )
    .unwrap();
    assert_eq!(
        f.stack.history_lengths(),
        (1, 0),
        "parameter and links: one step"
    );
    assert_eq!(
        links_of(&f.snapshot.project, 2),
        vec![(ga(GA_3), Direction::Send)]
    );
    let compared = f.observe().await;
    for i in rows_of(&compared, "association") {
        assert_eq!(compared.rows()[i].project, compared.rows()[i].device);
    }
}

#[tokio::test]
async fn a_retained_receive_flag_is_not_flipped_to_match_the_device_order() {
    let mut f = fixture();
    let first = f.snapshot.project.installations[0].group_addresses[0].id;
    let one = f
        .snapshot
        .project
        .devices
        .com_objects()
        .find(|o| o.number == 1)
        .unwrap()
        .id;
    f.snapshot
        .project
        .devices
        .com_object_mut(one)
        .unwrap()
        .links
        .push(GroupLink {
            ga: first,
            direction: Direction::Receive,
        });
    // Object 1 lists 1/0/3 first: on this device it sends on 1/0/3.
    f.sim.preset_memory(0x4100, &[3, 2, 0, 3, 1, 1, 1, 0, 0]);
    let observed = f.observe().await;
    let a = rows_of(&observed, "association");
    assert!(matches!(
        f.preview(&observed, &a),
        Err(AdoptionError::SendingLinkUndetermined { object: 1 })
    ));
    assert_eq!(f.stack.history_lengths(), (0, 0));
}

#[tokio::test]
async fn stale_revision_and_history_refuse_link_adoption() {
    let mut f = fixture();
    let observed = f.observe().await;
    let a = rows_of(&observed, "association");
    let preview = f.preview(&observed, &a).unwrap();
    // History-only drift (edit + undo) refuses the retained preview.
    f.stack
        .do_command(
            &mut f.snapshot.project,
            Command::RenameDevice {
                device: DeviceId(1),
                name: "renamed".into(),
            },
        )
        .unwrap();
    f.stack.undo(&mut f.snapshot.project).unwrap();
    let before = f.snapshot.clone();
    assert!(matches!(
        apply_adoption(
            &f.db,
            &mut f.snapshot,
            &mut f.stack,
            &f.scope,
            &preview,
            None
        ),
        Err(AdoptionError::Stale)
    ));
    assert_eq!(f.snapshot, before);
    // A programming-content edit after the read refuses a new preview.
    let one = f
        .snapshot
        .project
        .devices
        .com_objects()
        .find(|o| o.number == 1)
        .unwrap()
        .id;
    let second = f.snapshot.project.installations[0].group_addresses[1].id;
    f.stack
        .do_command(
            &mut f.snapshot.project,
            Command::LinkComObject {
                com_object: one,
                ga: second,
                direction: Direction::Receive,
            },
        )
        .unwrap();
    assert!(matches!(
        f.preview(&observed, &a),
        Err(AdoptionError::Stale)
    ));
}
