//! Schema-23 `Function` import (MODEL-07, ADR-0124) on a synthetic witness.
//!
//! No supported-schema corpus project contains a Function; the only real
//! ETS sample (xknxproject `testprojekt-ets6-functions`, ETS 6.1, schema 22)
//! is a schema KNXBench refuses. So the mapping rests on *Project Schema23*
//! §1.2.6.4/.9/.10 and is checked here on synthetic files whose attribute
//! set copies that sample's (`Id`, `Name`, `Type="FT-1"`, `Puid`;
//! `GroupAddressRef` with `Name=""`, `Role`, `RefId`, `Puid`).

use knx_core::{CompletionStatus, GroupAddressId};
use knx_etsproj::opaque::OpaqueKind;
use knx_etsproj::parse::UnknownKind;

const LIGHT: &str = r#"<Function Id="P-0001-0_F-1" Name="LivingroomLight" Type="FT-1" Number="7" Comment="c&#10;2" Description="d" CompletionStatus="Editing" Implements="FT-1" DefaulGroupRange="P-0001-0_GR-1" Puid="11">
  <GroupAddressRef Id="P-0001-0_GF-1" Name="" Role="SwitchOnOff" RefId="P-0001-0_GA-1" Puid="15" />
  <GroupAddressRef Id="P-0001-0_GF-2" Name="Feedback" Role="InfoOnOff" RefId="P-0001-0_GA-2" Puid="17" />
</Function>"#;

fn import(version: u32, building: &str, room: &str) -> knx_etsproj::ImportOutcome {
    knx_etsproj::import_knxproj_bytes(
        knx_testsupport::functions_knxproj_bytes(version, building, room),
        "functions.knxproj",
    )
    .unwrap()
}

fn row(outcome: &knx_etsproj::ImportOutcome, entity: &str) -> (u32, u32) {
    let row = outcome
        .report
        .counts
        .rows
        .iter()
        .find(|r| r.entity == entity)
        .unwrap();
    (row.read, row.mapped)
}

fn function_raw(outcome: &knx_etsproj::ImportOutcome, ets_id: &str) -> String {
    let key =
        format!("/KNX/Project/Installations/Installation/Locations/Space/Function[@Id='{ets_id}']");
    let entries: Vec<_> = outcome
        .opaque
        .iter()
        .filter(|e| e.kind == OpaqueKind::RetainedElement && e.xpath == key)
        .collect();
    assert_eq!(entries.len(), 1, "exactly one raw copy keyed {key}");
    String::from_utf8(entries[0].bytes.clone()).unwrap()
}

#[test]
fn a_schema_23_function_maps_into_its_room_with_ordered_roles_and_verbatim_text() {
    let outcome = import(23, "", LIGHT);
    assert_eq!(outcome.report.errors, vec![]);
    let in_functions: Vec<_> = outcome
        .report
        .unknown
        .iter()
        .filter(|u| u.xpath.contains("/Function"))
        .collect();
    assert!(
        in_functions.is_empty(),
        "Puid, Implements and DefaulGroupRange are known: {in_functions:?}"
    );
    let project = &outcome.project;
    assert_eq!(project.functions.len(), 1);
    let function = &project.functions[0];
    let room = project.installations[0]
        .buildings
        .iter()
        .find(|b| b.source.ets_id == "P-0001-0_BP-2")
        .unwrap();
    assert_eq!(
        function.location, room.id,
        "the nested room, not the building"
    );
    assert_eq!(function.source.ets_id, "P-0001-0_F-1");
    let d = &function.details;
    assert_eq!(d.name, "LivingroomLight");
    assert_eq!(d.function_type.as_deref(), Some("FT-1"));
    assert_eq!(d.number.as_deref(), Some("7"));
    assert_eq!(d.comment.as_deref(), Some("c\n2"));
    assert_eq!(d.description.as_deref(), Some("d"));
    assert_eq!(d.completion, CompletionStatus::Editing);
    let address = |ets: &str| {
        project.installations[0]
            .group_addresses
            .iter()
            .find(|g| g.source.ets_id == ets)
            .unwrap()
            .id
    };
    let members: Vec<_> = function
        .members
        .iter()
        .map(|m| {
            (
                m.source.ets_id.as_str(),
                m.group_address,
                m.name.as_str(),
                m.role.as_deref(),
            )
        })
        .collect();
    assert_eq!(
        members,
        [
            (
                "P-0001-0_GF-1",
                address("P-0001-0_GA-1"),
                "",
                Some("SwitchOnOff")
            ),
            (
                "P-0001-0_GF-2",
                address("P-0001-0_GA-2"),
                "Feedback",
                Some("InfoOnOff")
            ),
        ]
    );
    assert_eq!(row(&outcome, "Function"), (1, 1));
    assert_eq!(row(&outcome, "GroupAddressRef"), (2, 2));
    // The unmodeled attributes survive verbatim in the retained element.
    let raw = function_raw(&outcome, "P-0001-0_F-1");
    for kept in [
        r#"Implements="FT-1""#,
        r#"DefaulGroupRange="P-0001-0_GR-1""#,
        r#"Puid="11""#,
        r#"Puid="17""#,
    ] {
        assert!(raw.contains(kept), "{kept} retained");
    }
    assert!(project.ids.peek_function() >= function.id.0);
    assert_eq!(knx_core::function::validate_functions(project), Ok(()));
}

#[test]
fn functions_keep_their_room_and_document_order_across_nesting() {
    let outcome = import(
        23,
        r#"<Function Id="P-0001-0_F-3" Name="House scene" Puid="30"/>"#,
        r#"<Function Id="P-0001-0_F-1" Name="First" Puid="10"/><Function Id="P-0001-0_F-2" Name="Second" Puid="20"><GroupAddressRef Id="P-0001-0_GF-9" Name="" RefId="P-0001-0_GA-3" Puid="21"/></Function>"#,
    );
    assert_eq!(outcome.report.errors, vec![]);
    let names: Vec<_> = outcome
        .project
        .functions
        .iter()
        .map(|f| f.details.name.as_str())
        .collect();
    assert_eq!(names, ["House scene", "First", "Second"]);
    let building = outcome.project.installations[0]
        .buildings
        .iter()
        .find(|b| b.source.ets_id == "P-0001-0_BP-1")
        .unwrap()
        .id;
    assert_eq!(outcome.project.functions[0].location, building);
    assert!(
        outcome.project.functions[1].members.is_empty(),
        "an empty Function is kept"
    );
    assert_eq!(row(&outcome, "Function"), (3, 3));
}

#[test]
fn unknown_attributes_and_children_are_reported_and_kept_while_the_function_still_maps() {
    let outcome = import(
        23,
        "",
        r#"<Function Id="P-0001-0_F-1" Name="Light" Colour="red" Puid="1"><Extra Note="x"/><GroupAddressRef Id="P-0001-0_GF-1" Name="" RefId="P-0001-0_GA-1" Weight="2" Puid="2"/></Function>"#,
    );
    let unknown: Vec<_> = outcome
        .report
        .unknown
        .iter()
        .map(|u| (u.kind, u.name.as_str()))
        .collect();
    assert!(
        unknown.contains(&(UnknownKind::Attribute, "Colour")),
        "{unknown:?}"
    );
    assert!(
        unknown.contains(&(UnknownKind::Attribute, "Weight")),
        "{unknown:?}"
    );
    assert!(
        unknown.contains(&(UnknownKind::Element, "Extra")),
        "{unknown:?}"
    );
    assert_eq!(outcome.project.functions[0].members.len(), 1);
    let raw = function_raw(&outcome, "P-0001-0_F-1");
    for kept in [r#"Colour="red""#, r#"<Extra Note="x"/>"#, r#"Weight="2""#] {
        assert!(raw.contains(kept), "{kept} retained");
    }
}

#[test]
fn an_unresolved_member_is_reported_and_left_out_but_kept_in_the_raw_copy() {
    let outcome = import(
        23,
        "",
        r#"<Function Id="P-0001-0_F-1" Name="Light" Puid="1"><GroupAddressRef Id="P-0001-0_GF-1" Name="" RefId="P-0001-0_GA-404" Puid="2"/><GroupAddressRef Id="P-0001-0_GF-2" Name="" RefId="P-0001-0_GA-2" Puid="3"/></Function>"#,
    );
    let errors: Vec<_> = outcome
        .report
        .errors
        .iter()
        .map(|e| (e.stage, e.detail.as_str()))
        .collect();
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(errors[0].0, "map");
    assert!(errors[0].1.contains("UnresolvedReference"), "{errors:?}");
    assert!(errors[0].1.contains("P-0001-0_GA-404"), "{errors:?}");
    let members: Vec<GroupAddressId> = outcome.project.functions[0]
        .members
        .iter()
        .map(|m| m.group_address)
        .collect();
    assert_eq!(members.len(), 1);
    assert_eq!(row(&outcome, "GroupAddressRef"), (2, 1));
    assert!(function_raw(&outcome, "P-0001-0_F-1").contains("P-0001-0_GA-404"));
}

#[test]
fn schema_21_functions_stay_unknown_and_retained_because_only_schema_23_is_documented() {
    let outcome = import(21, "", LIGHT);
    assert!(outcome.project.functions.is_empty());
    assert!(
        outcome
            .report
            .unknown
            .iter()
            .any(|u| u.kind == UnknownKind::Element && u.name == "Function"),
        "{:?}",
        outcome.report.unknown
    );
    assert!(outcome
        .opaque
        .iter()
        .any(|e| e.kind == OpaqueKind::RetainedElement
            && e.name == "Function"
            && String::from_utf8_lossy(&e.bytes).contains("LivingroomLight")));
    assert_eq!(row(&outcome, "Function"), (0, 0));
}

#[test]
fn a_function_without_an_id_fails_the_parse_like_any_modeled_element() {
    let result = knx_etsproj::import_knxproj_bytes(
        knx_testsupport::functions_knxproj_bytes(23, "", r#"<Function Name="No id"/>"#),
        "functions.knxproj",
    );
    let Err(error) = result else {
        panic!("missing Id is fatal")
    };
    assert!(
        format!("{error:?}").contains("MissingRequiredAttribute"),
        "{error:?}"
    );
}
