//! Functions appear escaped in the report's Buildings section.
//!
//! ADR-0124: Functions appear in the Buildings section, escaped, with
//! their ordered group addresses and roles.
use knx_core::function::{Function, FunctionDetails, FunctionMember};
use knx_core::{
    BuildingPart, BuildingPartId, BuildingPartType, CompletionStatus, FunctionId, FunctionMemberId,
    GroupAddress, GroupAddressEntry, GroupAddressId, Installation, InstallationId, Language,
    Project, SourceRef, Topology,
};
use knx_report::{render_html, ReportLanguage, ReportOptions, ReportSection};

fn source(id: &str) -> SourceRef {
    SourceRef {
        path: "P-0001/0.xml".into(),
        ets_id: id.into(),
    }
}

/// One installation with a room, two group addresses and one Function.
fn project(function_ids: (u32, u32)) -> Project {
    let mut project = Project::new(Language("en".into()));
    project.installations.push(Installation {
        id: InstallationId(0),
        name: "Home".into(),
        default_line: None,
        multicast_address: None,
        completion: CompletionStatus::Undefined,
        topology: Topology {
            areas: vec![],
            lines: vec![],
            unassigned: vec![],
        },
        buildings: vec![BuildingPart {
            id: BuildingPartId(1),
            source: source("P-0001-0_BP-1"),
            name: "Living <room>".into(),
            number: None,
            kind: BuildingPartType::Room,
            default_line: None,
            completion: CompletionStatus::Undefined,
            children: vec![],
            devices: vec![],
            parent: None,
        }],
        group_ranges: vec![],
        group_addresses: [(1, 0x0801, "Switch"), (2, 0x0802, "Status")]
            .into_iter()
            .map(|(id, raw, name)| GroupAddressEntry {
                id: GroupAddressId(id),
                source: source(&format!("P-0001-0_GA-{id}")),
                name: name.into(),
                address: GroupAddress::from_raw(raw),
                central: false,
                unfiltered: false,
                range: None,
                declared_dpt: Default::default(),
            })
            .collect(),
        parameters: vec![],
    });
    let (function, first_member) = function_ids;
    project.functions.push(Function {
        id: FunctionId(function),
        source: source("P-0001-0_F-1"),
        location: BuildingPartId(1),
        details: FunctionDetails {
            name: "Light <script>".into(),
            function_type: Some("FT-1".into()),
            number: None,
            comment: Some("first\nsecond".into()),
            description: None,
            completion: CompletionStatus::Editing,
        },
        members: vec![
            FunctionMember {
                id: FunctionMemberId(first_member),
                source: source("P-0001-0_GF-1"),
                group_address: GroupAddressId(1),
                name: String::new(),
                role: Some("SwitchOnOff".into()),
            },
            FunctionMember {
                id: FunctionMemberId(first_member + 1),
                source: source("P-0001-0_GF-2"),
                group_address: GroupAddressId(2),
                name: String::new(),
                role: Some("InfoOnOff".into()),
            },
        ],
    });
    project
}

fn options() -> ReportOptions {
    ReportOptions::new(
        chrono::DateTime::parse_from_rfc3339("2026-10-13T00:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc),
    )
}

#[test]
fn functions_are_documented_in_the_buildings_section_with_escaped_text() {
    let html = render_html(&project((1, 1)), &options()).html;
    let section = &html[html.find("id=\"buildings\"").unwrap()..];
    assert!(section.contains("id=\"functions-0\""));
    assert!(section.contains("Light &lt;script&gt;"));
    assert!(!html.contains("Light <script>"));
    assert!(section.contains("Living &lt;room&gt;"));
    let switch = section.find("Switch [SwitchOnOff]").expect("first member");
    let status = section.find("Status [InfoOnOff]").expect("second member");
    assert!(switch < status, "members keep their order");
    assert!(section.contains("FT-1"));
}

#[test]
fn no_functions_no_table_and_the_section_switch_still_applies() {
    let mut empty = project((1, 1));
    empty.functions.clear();
    assert!(!render_html(&empty, &options()).html.contains("functions-0"));
    let mut without_buildings = options();
    without_buildings.sections.remove(&ReportSection::Buildings);
    assert!(!render_html(&project((1, 1)), &without_buildings)
        .html
        .contains("functions-0"));
    let mut german = options();
    german.language = ReportLanguage::German;
    assert!(render_html(&project((1, 1)), &german)
        .html
        .contains("Funktionen"));
}
