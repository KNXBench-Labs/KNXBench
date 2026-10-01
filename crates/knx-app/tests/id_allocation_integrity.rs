//! ADR-0039 appendix regression: a stale CSV plan is refused instead of losing data on save.
//!
//! A CSV import plan built against one project state, applied after another
//! edit consumed ids in between. Before phase 1 the plan's
//! `SetIdAllocators` could lower the counter, a later
//! create reused a live id, and `save_project` silently dropped an entity.
//! This test drives the real `knx-csv` planner, `knx-core` commands and the
//! `knx-store` save/load path, so it lives in `knx-app`, the one crate that
//! sees all three.

use knx_core::installation::Installation;
use knx_core::project::Project;
use knx_core::string_table::Language;
use knx_core::topology::Topology;
use knx_core::{
    Command, CommandError, CompletionStatus, GroupAddress, GroupAddressEntry, GroupAddressId,
    GroupAddressStyle, IdKind, InstallationId, SourceRef,
};
use knx_csv::{parse_group_addresses, plan_import};
use knx_store::{load_project, open_and_migrate_in_memory, save_project};

fn empty_project() -> Project {
    let mut project = Project::new(Language("en".into()));
    project.info.group_address_style = GroupAddressStyle::Free;
    project.installations.push(Installation {
        id: InstallationId(0),
        name: "I".into(),
        default_line: None,
        multicast_address: None,
        completion: CompletionStatus::FinishedDesign,
        topology: Topology {
            areas: vec![],
            lines: vec![],
            unassigned: vec![],
        },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses: vec![],
        parameters: vec![],
    });
    project
}

fn group_address(id: GroupAddressId, raw: u16, name: &str) -> GroupAddressEntry {
    let tag = format!("KB-GA-{}", id.0);
    GroupAddressEntry {
        id,
        source: SourceRef {
            path: tag.clone(),
            ets_id: tag,
        },
        name: name.into(),
        address: GroupAddress::from_raw(raw),
        central: false,
        unfiltered: false,
        range: None,
    }
}

#[test]
fn a_stale_csv_plan_can_no_longer_make_save_drop_a_group_address() {
    let mut project = empty_project();

    // 1. The CSV plan is built now, against counters at 0.
    let csv = parse_group_addresses("Address,Name\n100,From CSV\n", GroupAddressStyle::Free);
    let stale_plan = plan_import(&project, &csv)
        .command
        .expect("an unseen address plans a create");

    // 2. Meanwhile, the user creates a group address; it takes id 1.
    let user_id = project.ids.next_group_address_id().unwrap();
    Command::CreateGroupAddress {
        entry: group_address(user_id, 200, "User"),
    }
    .apply(&mut project)
    .unwrap();

    // 3. The stale plan is applied. It wants id 1 as well: refused whole,
    //    nothing half-applied, the user's address untouched.
    let before = project.clone();
    let refused = stale_plan.apply(&mut project);
    assert_eq!(
        refused.unwrap_err(),
        CommandError::BatchItem {
            index: 0,
            source: Box::new(CommandError::IdInUse {
                kind: IdKind::GroupAddress,
                id: user_id.0,
            }),
        }
    );
    assert_eq!(project, before, "a refused batch must leave no trace");

    // 4. Re-planning against the current state succeeds, and both addresses
    //    survive a save/load round trip — the entity the old defect lost.
    let fresh_plan = plan_import(&project, &csv).command.unwrap();
    fresh_plan.apply(&mut project).unwrap();
    let names: Vec<&str> = project.installations[0]
        .group_addresses
        .iter()
        .map(|g| g.name.as_str())
        .collect();
    assert_eq!(names, ["User", "From CSV"]);

    let conn = open_and_migrate_in_memory().unwrap();
    save_project(&conn, &project).unwrap();
    let loaded = load_project(&conn).unwrap();
    assert_eq!(loaded, project);
    assert_eq!(loaded.installations[0].group_addresses.len(), 2);
}

#[test]
fn reserve_ids_from_a_stale_snapshot_never_lowers_what_an_earlier_edit_consumed() {
    let mut project = empty_project();
    let stale_snapshot = project.ids.clone();
    let taken = project.ids.next_group_address_id().unwrap();
    Command::CreateGroupAddress {
        entry: group_address(taken, 1, "Taken"),
    }
    .apply(&mut project)
    .unwrap();

    Command::ReserveIds {
        through: stale_snapshot,
    }
    .apply(&mut project)
    .unwrap();

    assert!(
        project.ids.next_group_address_id().unwrap().0 > taken.0,
        "the next id must be above the one already in use"
    );
}
