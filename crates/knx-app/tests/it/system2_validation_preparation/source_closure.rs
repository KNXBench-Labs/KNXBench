//! Synthetic source-closure preparation, replay and refusal coverage.
use super::*;
mod admission;
mod bounds;
mod execution;
mod xml;
use std::io::{Cursor, Read, Write};

fn imported_sources(
    namespace: u8,
) -> (
    tempfile::TempDir,
    knx_productdb::Connection,
    knx_core::Project,
    Vec<u8>,
) {
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    let store = knx_store::open_and_migrate_in_memory().unwrap();
    let (package, source) = synthetic_package(namespace);
    let mut zip = zip::ZipArchive::new(Cursor::new(&package)).unwrap();
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for index in 0..zip.len() {
        let mut member = zip.by_index(index).unwrap();
        let mut bytes = Vec::new();
        member.read_to_end(&mut bytes).unwrap();
        writer
            .start_file(member.name(), zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(&bytes).unwrap();
    }
    let project_xml = format!(
        r#"<KNX xmlns="http://knx.org/xml/project/{namespace}"><Project Id="P-0001"><Installations><Installation InstallationId="0"><Topology><Area Id="A" Address="1"><Line Id="L" Address="2"><DeviceInstance Id="D" Address="52" ProductRefId="M-0000_H-1_P-1" Hardware2ProgramRefId="M-0000_H-1_HP-1"><ParameterInstanceRefs><ParameterInstanceRef RefId="{}" Value="258"/><ParameterInstanceRef RefId="{}" Value="1"/></ParameterInstanceRefs></DeviceInstance></Line></Area></Topology></Installation></Installations></Project></KNX>"#,
        syn_ref("R"),
        syn_ref("RS")
    );
    let project_xml = if namespace == 23 {
        project_xml
            .replace(
                "<DeviceInstance",
                "<Segment Id=\"SEG\" Number=\"1\"><DeviceInstance",
            )
            .replace("</DeviceInstance>", "</DeviceInstance></Segment>")
    } else {
        project_xml
    };
    writer
        .start_file("P-0001.signature", zip::write::SimpleFileOptions::default())
        .unwrap();
    writer
        .start_file(
            "P-0001/project.xml",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
    writer.write_all(format!(r#"<KNX xmlns="http://knx.org/xml/project/{namespace}"><Project Id="P-0001"><ProjectInformation Name="Synthetic"/></Project></KNX>"#).as_bytes()).unwrap();
    writer
        .start_file("P-0001/0.xml", zip::write::SimpleFileOptions::default())
        .unwrap();
    writer.write_all(project_xml.as_bytes()).unwrap();
    let archive = writer.finish().unwrap().into_inner();
    let imported = knx_app::import::import_ets_project_bytes(
        archive.clone(),
        "synthetic.knxproj",
        &store,
        knx_app::import::ImportOptions {
            product_db: Some(&conn),
        },
    )
    .unwrap();
    assert_eq!(
        imported
            .project
            .devices
            .get(knx_core::DeviceId(1))
            .unwrap()
            .program_ref,
        "M-0000_H-1_HP-1"
    );
    assert_eq!(
        conn.query_row("SELECT count(*) FROM package", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
    let changes = conn.total_changes();
    assert!(matches!(
        knx_productdb::install_package(&conn, "synthetic.knxproj", &archive),
        Err(knx_productdb::PackageError::ProjectArchive)
    ));
    assert_eq!(conn.total_changes(), changes);
    (
        dir,
        conn,
        imported.project,
        synthetic_profile(namespace, &package, &source),
    )
}

fn source_profile(
    conn: &knx_productdb::Connection,
    project: &knx_core::Project,
    namespace: u8,
    profile: &[u8],
) -> Vec<u8> {
    let descriptor = knx_app::system2_validation_preparation::describe_project_source_closure(
        conn,
        project,
        knx_core::DeviceId(1),
        &format!("http://knx.org/xml/project/{namespace}"),
    )
    .unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(profile).unwrap();
    value.as_object_mut().unwrap().remove("package_sha256");
    value["product_input"] = serde_json::json!({"kind":"retained-source-members-v1", "closure_sha256":descriptor.sha256().unwrap()});
    serde_json::to_vec(&value).unwrap()
}

#[test]
fn actual_imported_sources_prepare_without_package_ownership() {
    for namespace in [20, 23] {
        let (_dir, conn, project, profile) = imported_sources(namespace);
        let profile = source_profile(&conn, &project, namespace, &profile);
        let changes = conn.total_changes();
        let before = project.clone();
        let prepared = knx_app::system2_validation_preparation::prepare_project_simulated_candidate(
            &conn,
            &project,
            knx_core::DeviceId(1),
            &profile,
        );
        match prepared {
            Ok(_) => {}
            Err(error) => panic!(
                "actual imported winning sources must prepare without package ownership: {error}"
            ),
        }
        assert_eq!(conn.total_changes(), changes);
        assert_eq!(project, before);
    }
}
