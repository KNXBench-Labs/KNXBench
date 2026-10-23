//! Synthetic retained-source cardinality, namespace and size refusal controls.
use super::admission::*;
use super::xml::{source_member, with_member};
use super::*;
use knx_app::system2_validation_preparation::describe_project_source_closure;

fn refused<T>(result: std::io::Result<T>) -> std::io::Error {
    match result {
        Err(error) => error,
        Ok(_) => panic!("expected bounded source refusal"),
    }
}
fn describe_hardware(
    source: &[u8],
) -> std::io::Result<knx_app::system2_validation_preparation::SourceClosureDescriptor> {
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    let (_, application) = synthetic_package(20);
    knx_productdb::ingest_file(&conn, "M-0000/Hardware.xml", source).unwrap();
    knx_productdb::ingest_file(&conn, "M-0000/Application.xml", &application).unwrap();
    describe_project_source_closure(
        &conn,
        &synthetic_project(),
        knx_core::DeviceId(1),
        "http://knx.org/xml/project/20",
    )
}
#[test]
fn source_xml_depth64_and_element_event250000_boundaries_are_exact() {
    let original = source_bundle(20);
    let hardware = source_member(&original, 0);
    let nested = |depth: usize| {
        hardware.replace(
            "</KNX>",
            &format!(
                "{}{}{}",
                "<X>".repeat(depth),
                "</X>".repeat(depth),
                "</KNX>"
            ),
        )
    };
    assert!(
        describe_hardware(nested(63).as_bytes()).is_ok(),
        "root plus 63 nested elements is depth64"
    );
    let error = refused(describe_hardware(nested(64).as_bytes()));
    assert!(error.to_string().contains("event/depth limit"));
    let base_events = hardware.matches('<').count();
    let repeated =
        |extra: usize| hardware.replace("</KNX>", &format!("{}</KNX>", "<X/>".repeat(extra)));
    assert!(
        describe_hardware(repeated(250_000 - base_events).as_bytes()).is_ok(),
        "exact250000 element events"
    );
    let error = refused(describe_hardware(
        repeated(250_001 - base_events).as_bytes(),
    ));
    assert!(error.to_string().contains("event/depth limit"));
}
#[test]
fn actual_blob_length_and_nested_recovery_budget_cannot_use_stale_len_or_compact_size() {
    let (_dir, conn, project, profile) = source_inputs(20);
    let descriptor = describe_project_source_closure(
        &conn,
        &project,
        knx_core::DeviceId(1),
        "http://knx.org/xml/project/20",
    )
    .unwrap();
    conn.execute(
        "UPDATE source_file SET bytes=?1,len=1 WHERE sha256=?2",
        (
            &vec![0u8; 16 * 1024 * 1024 + 1],
            &descriptor.members[0].source_sha256,
        ),
    )
    .unwrap();
    let changes = conn.total_changes();
    let error = refused(
        knx_app::system2_validation_preparation::prepare_project_simulated_candidate(
            &conn,
            &project,
            knx_core::DeviceId(1),
            &profile,
        ),
    );
    assert!(error
        .to_string()
        .contains("aggregate exceeds bundle budget"));
    assert_eq!(conn.total_changes(), changes);
    let original = source_bundle(20);
    let over = with_member(original, 0, vec![255; 23 * 512 * 1024]);
    assert!(serde_json::to_vec(&over).unwrap().len() < 16 * 1024 * 1024);
    let error = refused(over.plan());
    assert_eq!(
        error.to_string(),
        "project bundle exceeds recovery envelope budget"
    );
}
#[test]
fn compact_retention_admits_genuine_large_synthetic_source_and_replays_original_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    let (package, application) = synthetic_package(20);
    let original = source_bundle(20);
    let hardware = source_member(&original, 0).replace(
        "</KNX>",
        &format!("<!--{}--></KNX>", "x".repeat(1024 * 1024)),
    );
    knx_productdb::ingest_file(&conn, "M-0000/Hardware.xml", hardware.as_bytes()).unwrap();
    knx_productdb::ingest_file(&conn, "M-0000/Application.xml", &application).unwrap();
    let project = synthetic_project();
    let before = project.clone();
    let profile = source_profile(
        &conn,
        &project,
        20,
        &synthetic_profile(20, &package, &application),
    );
    let changes = conn.total_changes();
    let candidate = knx_app::system2_validation_preparation::prepare_project_simulated_candidate(
        &conn,
        &project,
        knx_core::DeviceId(1),
        &profile,
    );
    let bundle = match candidate {
        Ok(candidate) => candidate.bundle,
        Err(error) => {
            panic!("valid large synthetic source must fit unchanged recovery budget: {error}")
        }
    };
    assert_eq!(conn.total_changes(), changes);
    assert_eq!(project, before);
    assert_eq!(source_member(&bundle, 0).as_bytes(), hardware.as_bytes());
    let file = dir.path().join("bundle.json");
    std::fs::write(&file, serde_json::to_vec_pretty(&bundle).unwrap()).unwrap();
    drop(conn);
    drop(project);
    drop(profile);
    let reopened = app::read_bundle(&file).unwrap();
    assert_eq!(reopened, bundle);
    let replay =
        knx_app::system2_validation_preparation::replay_project_simulated_candidate(&reopened)
            .unwrap();
    assert_eq!(replay.bundle, bundle);
}

#[test]
fn source_selection_uses_actual_winner_not_loser_or_last_ingested_path() {
    let (_dir, conn, project, profile) = source_inputs(20);
    let before = describe_project_source_closure(
        &conn,
        &project,
        knx_core::DeviceId(1),
        "http://knx.org/xml/project/20",
    )
    .unwrap();
    let winner = source_member(&project_candidate(&conn, &project, &profile).bundle, 0);
    // Exact bytes under a later name retain the first path, not archive provenance.
    knx_productdb::ingest_file(&conn, "M-0000/Later.xml", winner.as_bytes()).unwrap();
    let loser = winner.replace("MediumTypes=\"MT-0\"", "MediumTypes=\"MT-1\"");
    knx_productdb::ingest_file(&conn, "M-0000/Loser.xml", loser.as_bytes()).unwrap();
    let after = describe_project_source_closure(
        &conn,
        &project,
        knx_core::DeviceId(1),
        "http://knx.org/xml/project/20",
    )
    .unwrap();
    assert_eq!(after, before);
    assert_eq!(after.members[0].source_path, "M-0000/Hardware.xml");
    let bundle = project_candidate(&conn, &project, &profile).bundle;
    let swapped = with_member(bundle, 0, loser.into_bytes());
    assert!(
        swapped.plan().is_err(),
        "a rehashed losing declaration cannot carry the winning owner witness"
    );
}
#[test]
fn repeated_real_hardware_declarations_and_mixed_namespaces_are_not_a_source_closure() {
    let original = source_bundle(20);
    let hardware = source_member(&original, 0);
    let hp_start = hardware.find("<Hardware2Program Id=").unwrap();
    let hp_end = hardware.find("</Hardware2Program>").unwrap() + "</Hardware2Program>".len();
    let hp = &hardware[hp_start..hp_end];
    let duplicate = hardware.replace("</Hardware2Programs>", &format!("{hp}</Hardware2Programs>"));
    assert!(describe_hardware(duplicate.as_bytes()).is_err());
    assert!(describe_hardware(hardware.replace("project/20", "project/23").as_bytes()).is_err());
}
