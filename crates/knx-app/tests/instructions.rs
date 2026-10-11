//! Verifies normalized instruction integrity with synthetic data.
use knx_app::instructions::{
    original_device_instructions as read, InstructionSourceError as E, OriginalInstructions,
};
use knx_core::{DeviceId, Project};
use knx_store::StoredOpaqueEntry;
use knx_testsupport::zip_with_entries;

fn imported(hint: &str) -> (Project, Vec<StoredOpaqueEntry>) {
    let topology = format!(
        r#"<KNX xmlns="http://knx.org/xml/project/23"><Project Id="P-0001"><Installations><Installation InstallationId="0"><Topology><UnassignedDevices><DeviceInstance Id="P-0001-0_DI-1" Name="First" InstallationHints="not the target"/><DeviceInstance Id="P-0001-0_DI-2" Name="Target" {hint}/></UnassignedDevices></Topology></Installation></Installations></Project></KNX>"#
    );
    let info = r#"<KNX xmlns="http://knx.org/xml/project/23"><Project Id="P-0001"><ProjectInformation Name="Synthetic instructions"/></Project></KNX>"#;
    let container = zip_with_entries(&[
        ("P-0001.signature", b""),
        ("P-0001/0.xml", topology.as_bytes()),
        ("P-0001/Project.xml", info.as_bytes()),
    ]);
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_store::open_and_migrate(&dir.path().join("read.knxdb")).unwrap();
    let result = knx_app::import_ets_project_bytes(
        container,
        "synthetic.knxproj",
        &conn,
        Default::default(),
    )
    .unwrap();
    assert_eq!(result.project.devices.iter().count(), 2);
    (result.project, knx_store::load_opaque(&conn).unwrap())
}
fn target(project: &Project) -> DeviceId {
    project
        .devices
        .iter()
        .find(|device| device.source.ets_id == "P-0001-0_DI-2")
        .unwrap()
        .id
}
fn xml(entries: &mut [StoredOpaqueEntry]) -> &mut StoredOpaqueEntry {
    entries
        .iter_mut()
        .find(|entry| entry.source_path == "P-0001/0.xml" && entry.xpath.is_empty())
        .unwrap()
}

#[test]
fn correct_device_original_and_digest_are_independent_of_fragment_order() {
    let (project, mut opaque) = imported("InstallationHints=\"right &amp; Grüße\"");
    let before = project.clone();
    opaque.reverse();
    let original = opaque.clone();
    let view = read(&project, &opaque, target(&project)).unwrap();
    let OriginalInstructions {
        xml_sha256,
        hints: Some(hints),
    } = view
    else {
        panic!("expected source-backed instructions");
    };
    assert_eq!(hints.original, "right & Grüße");
    assert_eq!(hints.preview.unwrap().plain_text(), "right & Grüße");
    assert_eq!(xml_sha256, xml(&mut opaque).sha256);
    assert_eq!(opaque, original);
    assert_eq!(project, before);
}
#[test]
fn field_absent_empty_and_missing_source_are_not_conflated() {
    for attr in ["", "InstallationHints=\"\""] {
        let (mut project, opaque) = imported(attr);
        let id = target(&project);
        let OriginalInstructions { hints, .. } = read(&project, &opaque, id).unwrap();
        assert_eq!(hints.is_some(), !attr.is_empty());
        if let Some(value) = hints {
            assert_eq!(value.original, "");
        }
        project.devices.get_mut(id).unwrap().source.path.clear();
        assert_eq!(read(&project, &opaque, id), Err(E::MissingSource));
    }
}
#[test]
fn fragments_and_other_files_cannot_replace_a_missing_original() {
    let (project, mut opaque) = imported("InstallationHints=\"correct\"");
    opaque.retain(|entry| !(entry.source_path == "P-0001/0.xml" && entry.xpath.is_empty()));
    assert!(opaque.iter().any(|entry| entry.name == "InstallationHints"));
    assert_eq!(
        read(&project, &opaque, target(&project)),
        Err(E::MissingSource)
    );
}
#[test]
fn ambiguous_and_tampered_originals_fail_closed() {
    let (project, mut opaque) = imported("InstallationHints=\"correct\"");
    let mut duplicate = xml(&mut opaque).clone();
    opaque.push(duplicate.clone());
    assert_eq!(
        read(&project, &opaque, target(&project)),
        Err(E::AmbiguousSource)
    );
    opaque.pop();
    xml(&mut opaque).bytes.push(b' ');
    assert_eq!(read(&project, &opaque, target(&project)), Err(E::Integrity));
    xml(&mut opaque).bytes = duplicate.bytes.clone();
    duplicate.kind = "RetainedElement".into();
    let entry = xml(&mut opaque);
    entry.kind = duplicate.kind;
    assert_eq!(
        read(&project, &opaque, target(&project)),
        Err(E::MissingSource)
    );
}
#[test]
fn unknown_domain_target_is_not_native_or_empty() {
    let (project, opaque) = imported("");
    assert_eq!(
        read(&project, &opaque, DeviceId(u32::MAX)),
        Err(E::DeviceNotFound)
    );
}
#[test]
fn preview_refusal_still_exposes_exact_logical_original() {
    let value = r"{\rtf1 incomplete";
    let (project, opaque) = imported(&format!("InstallationHints=\"{value}\""));
    let OriginalInstructions {
        hints: Some(hints), ..
    } = read(&project, &opaque, target(&project)).unwrap()
    else {
        panic!("source must remain accessible");
    };
    assert_eq!(hints.original, value);
    assert_eq!(hints.preview, Err(knx_etsproj::rtf::TextError::Malformed));
}
#[test]
fn source_bound_read_is_identical_after_native_resave_and_reopen() {
    let (project, opaque) = imported(r#"InstallationHints="{\rtf1 a\b b}""#);
    let id = target(&project);
    let expected = read(&project, &opaque, id).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("roundtrip.knxdb");
    let conn = knx_store::open_and_migrate(&path).unwrap();
    knx_store::save_project(&conn, &project).unwrap();
    knx_store::insert_opaque(&conn, &opaque).unwrap();
    drop(conn);
    let conn = knx_store::open_existing_and_migrate(&path).unwrap();
    let reopened = knx_store::load_project(&conn).unwrap();
    let original = knx_store::load_opaque(&conn).unwrap();
    assert_eq!(reopened, project);
    assert_eq!(original, opaque);
    assert_eq!(read(&reopened, &original, id).unwrap(), expected);
    knx_store::save_project(&conn, &reopened).unwrap();
    assert_eq!(knx_store::load_opaque(&conn).unwrap(), opaque);
}
