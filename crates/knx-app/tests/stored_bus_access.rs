//! Stored bus access and IP configuration, read-only and hidden by default (Goal B3).
//!
//! Synthetic records only (`knx-testsupport`'s RFC 5737 privacy witness and
//! hand-built rows); no private project, no network.

use knx_app::stored_bus_access::{
    stored_bus_access, FieldPart, OwnerKind, RecordStatus, StoredRecord, UnattributedReason,
    ValueClass, MAX_RECORD_BYTES,
};
use knx_store::StoredOpaqueEntry;
use knx_testsupport::{retained_privacy_knxproj_bytes, RETAINED_PRIVACY_PLANTED};

fn imported() -> (knx_core::Project, Vec<StoredOpaqueEntry>) {
    let outcome =
        knx_etsproj::import_knxproj_bytes(retained_privacy_knxproj_bytes(), "synthetic.knxproj")
            .expect("the synthetic witness imports");
    let opaque = outcome
        .opaque
        .iter()
        .map(|e| StoredOpaqueEntry {
            source_path: e.source_path.clone(),
            xpath: e.xpath.clone(),
            kind: format!("{:?}", e.kind),
            name: e.name.clone(),
            bytes: e.bytes.clone(),
            sha256: e.sha256.clone(),
        })
        .collect();
    (outcome.project, opaque)
}

fn row(xpath: &str, name: &str, bytes: &str) -> StoredOpaqueEntry {
    StoredOpaqueEntry {
        source_path: "P-0001/0.xml".into(),
        xpath: xpath.into(),
        kind: "RetainedElement".into(),
        name: name.into(),
        bytes: bytes.as_bytes().to_vec(),
        sha256: format!("{:064x}", bytes.len()),
    }
}

fn field<'a>(record: &'a StoredRecord, name: &str) -> &'a knx_app::stored_bus_access::RecordField {
    record
        .fields
        .iter()
        .find(|f| f.name == name)
        .unwrap_or_else(|| panic!("no field {name}"))
}

fn json(view: &impl serde::Serialize) -> String {
    serde_json::to_string(view).unwrap()
}

#[test]
fn a_line_access_is_listed_on_its_line_with_every_endpoint_hidden() {
    let (project, opaque) = imported();
    let view = stored_bus_access(&project, &opaque, None);
    let access = view
        .records
        .iter()
        .find(|r| r.element == "BusAccess")
        .expect("the line's BusAccess is listed");
    assert_eq!(access.status, RecordStatus::Read);
    assert_eq!(access.owner.kind, OwnerKind::Line);
    assert_eq!(access.owner.address.as_deref(), Some("1.1"));
    let line = project.installations[0].topology.lines[0].id;
    assert_eq!(access.owner.id, Some(line.0));
    // Provenance is exact.
    assert!(access.source.xpath.contains("Line[@Id='P-0001-0_L-2']"));
    assert_eq!(
        access.source.size,
        opaque
            .iter()
            .find(|o| o.name == "BusAccess")
            .unwrap()
            .bytes
            .len()
    );
    // Names and keys are shown; every value that could name an endpoint is not.
    let target = field(access, "Target");
    assert_eq!(target.part, FieldPart::Parameter);
    assert_eq!(target.value, None);
    assert_eq!(target.hidden.as_ref().unwrap().class, ValueClass::Text);
    assert_eq!(
        field(access, "Peer").hidden.as_ref().unwrap().class,
        ValueClass::Ipv4Address
    );
    assert_eq!(
        field(access, "Port").hidden.as_ref().unwrap().class,
        ValueClass::Number
    );
    assert_eq!(field(access, "Name").part, FieldPart::Attribute);
    assert_eq!(field(access, "Name").value, None);
    assert!(!access.revealed);
    // Not one planted value is in the serialized answer.
    let text = json(&view);
    for planted in RETAINED_PRIVACY_PLANTED {
        assert!(
            !text.contains(planted),
            "{planted} leaked into the hidden view"
        );
    }
}

#[test]
fn an_unkeyed_ip_configuration_is_listed_but_never_given_a_device() {
    let (project, opaque) = imported();
    let view = stored_bus_access(&project, &opaque, None);
    let ip = view
        .records
        .iter()
        .find(|r| r.element == "IPConfig")
        .expect("the device's IPConfig is listed");
    // The importer keys an unknown element by element names only: which
    // device it belongs to cannot be read from the stored row.
    assert_eq!(ip.owner.kind, OwnerKind::Unattributed);
    assert_eq!(ip.owner.reason, Some(UnattributedReason::NotKeyed));
    assert_eq!(ip.owner.id, None);
    assert_eq!(
        field(ip, "IPAddress").hidden.as_ref().unwrap().class,
        ValueClass::Ipv4Address
    );
    assert_eq!(
        field(ip, "MACAddress").hidden.as_ref().unwrap().class,
        ValueClass::MacAddress
    );
}

#[test]
fn revealing_one_record_shows_its_values_and_no_other_record_s() {
    let (project, opaque) = imported();
    let hidden = stored_bus_access(&project, &opaque, None);
    let access_id = hidden
        .records
        .iter()
        .find(|r| r.element == "BusAccess")
        .unwrap()
        .record_id
        .clone();
    let view = stored_bus_access(&project, &opaque, Some(&access_id));
    let access = view
        .records
        .iter()
        .find(|r| r.record_id == access_id)
        .unwrap();
    assert!(access.revealed);
    assert_eq!(
        field(access, "Target").value.as_deref(),
        Some(RETAINED_PRIVACY_PLANTED[0])
    );
    assert_eq!(
        field(access, "Peer").value.as_deref(),
        Some(RETAINED_PRIVACY_PLANTED[1])
    );
    assert_eq!(field(access, "Port").value.as_deref(), Some("3671"));
    assert!(field(access, "Peer").hidden.is_none());
    let ip = view
        .records
        .iter()
        .find(|r| r.element == "IPConfig")
        .unwrap();
    assert!(!ip.revealed);
    assert_eq!(field(ip, "IPAddress").value, None);
    // An unknown id reveals nothing at all.
    let none = stored_bus_access(&project, &opaque, Some("99:0000000000000000"));
    assert!(none.records.iter().all(|r| !r.revealed));
}

#[test]
fn harmless_enumerations_are_shown_and_anything_else_is_hidden() {
    let project = knx_core::Project::new(knx_core::Language("en".into()));
    let opaque = vec![
        row(
            "/KNX/Project/Installations/Installation/Topology/Area/Line[@Id='L-9']/Segment/BusAccess",
            "BusAccess",
            r#"<BusAccess Name="Office" Parameter="Type=KnxIpTunneling;HostAddress=192.0.2.5;IpPort=3671;UseNat=False;Name=Office gateway;" Owner="x"/>"#,
        ),
        row(
            "/KNX/Project/Installations/Installation/Topology/Area/Line/Segment/DeviceInstance/IPConfig",
            "IPConfig",
            r#"<IPConfig Assign="Fixed" IPAddress="192.0.2.9" SubnetMask="255.255.255.0"/>"#,
        ),
    ];
    let view = stored_bus_access(&project, &opaque, None);
    let access = &view.records[0];
    assert_eq!(
        field(access, "Type").value.as_deref(),
        Some("KnxIpTunneling")
    );
    assert_eq!(field(access, "UseNat").value.as_deref(), Some("False"));
    assert_eq!(field(access, "HostAddress").value, None);
    assert_eq!(field(access, "Name").value, None);
    // An attribute the schema does not name is listed, hidden and unknown.
    let owner = field(access, "Owner");
    assert!(!owner.known);
    assert_eq!(owner.value, None);
    // No project line has this source id: listed, but on no line.
    assert_eq!(access.owner.kind, OwnerKind::Unattributed);
    assert_eq!(access.owner.reason, Some(UnattributedReason::NoMatch));
    let ip = &view.records[1];
    assert_eq!(field(ip, "Assign").value.as_deref(), Some("Fixed"));
    assert_eq!(field(ip, "SubnetMask").value, None);
}

#[test]
fn a_parameter_that_does_not_split_cleanly_stays_one_hidden_value() {
    let project = knx_core::Project::new(knx_core::Language("en".into()));
    for parameter in ["Type=A;just text;Port=1", "a=b=c", "1x=2"] {
        let bytes = format!(r#"<BusAccess Name="n" Parameter="{parameter}"/>"#);
        let view = stored_bus_access(&project, &[row("/x/BusAccess", "BusAccess", &bytes)], None);
        let record = &view.records[0];
        let whole = field(record, "Parameter");
        assert_eq!(whole.part, FieldPart::Attribute, "{parameter}");
        assert_eq!(whole.value, None, "{parameter}");
        assert!(
            record.unread.iter().any(|u| u == "parameterNotSplit"),
            "{parameter}"
        );
        assert!(
            !record.fields.iter().any(|f| f.part == FieldPart::Parameter),
            "{parameter}"
        );
    }
}

#[test]
fn malformed_oversized_and_unrelated_rows_are_said_not_guessed() {
    let project = knx_core::Project::new(knx_core::Language("en".into()));
    let huge = format!(
        r#"<BusAccess Name="n" Parameter="Type={}"/>"#,
        "A".repeat(MAX_RECORD_BYTES)
    );
    let opaque = vec![
        row("/x/BusAccess", "BusAccess", "<BusAccess Name=\"n"),
        row("/x/BusAccess", "BusAccess", &huge),
        row(
            "/x/BusAccess",
            "BusAccess",
            r#"<BusAccess Name="n" Parameter="Type=A"><Extra/><Extra/></BusAccess>"#,
        ),
        row("/x/Security", "Security", "<Security/>"),
        StoredOpaqueEntry {
            kind: "RetainedAttribute".into(),
            ..row("/x/BusAccess", "BusAccess", "x")
        },
    ];
    let view = stored_bus_access(&project, &opaque, None);
    assert_eq!(
        view.records.len(),
        3,
        "only retained BusAccess/IPConfig elements count"
    );
    assert_eq!(view.records[0].status, RecordStatus::NotWellFormed);
    assert!(view.records[0].fields.is_empty());
    assert_eq!(view.records[1].status, RecordStatus::TooLarge);
    assert!(view.records[1].fields.is_empty());
    assert_eq!(view.records[2].status, RecordStatus::Read);
    assert_eq!(
        view.records[2].unread,
        vec!["Extra".to_string(), "Extra".to_string()]
    );
    // Ids are distinct and stable.
    let ids: Vec<_> = view.records.iter().map(|r| r.record_id.clone()).collect();
    assert_eq!(
        ids.len(),
        ids.iter().collect::<std::collections::HashSet<_>>().len()
    );
    assert_eq!(
        ids,
        stored_bus_access(&project, &opaque, None)
            .records
            .iter()
            .map(|r| r.record_id.clone())
            .collect::<Vec<_>>()
    );
}

#[test]
fn two_lines_with_one_source_id_decide_nothing() {
    let (mut project, opaque) = imported();
    let mut twin = project.installations[0].topology.lines[0].clone();
    twin.id = knx_core::LineId(twin.id.0 + 100);
    project.installations[0].topology.lines.push(twin);
    let view = stored_bus_access(&project, &opaque, None);
    let access = view
        .records
        .iter()
        .find(|r| r.element == "BusAccess")
        .unwrap();
    assert_eq!(access.owner.kind, OwnerKind::Unattributed);
    assert_eq!(
        access.owner.reason,
        Some(UnattributedReason::SeveralMatches)
    );
}

#[test]
fn reading_changes_nothing() {
    let (project, opaque) = imported();
    let (before_project, before_opaque) = (project.clone(), opaque.clone());
    let hidden = stored_bus_access(&project, &opaque, None);
    let _ = stored_bus_access(&project, &opaque, Some(&hidden.records[0].record_id));
    assert_eq!(project, before_project);
    assert_eq!(opaque, before_opaque);
}
