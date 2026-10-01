//! The golden regression test for the whole import pipeline.
//!
//! Every count here comes from ROADMAP Session 3 and RESEARCH §3, and each
//! was re-measured against the reference project directly (see the Session
//! 3 SDD ledger's per-task notes) rather than assumed from the plan alone.
//! If any stage starts dropping or miscounting entities, one of these
//! numbers moves — that is what this test exists to catch.

use std::path::PathBuf;

use knx_core::{BuildingPartType, Direction, Override};
use knx_etsproj::import_knxproj;

#[test]
fn space_type_schema_23_preserves_documented_tokens_and_reports_unknown() {
    use knx_etsproj::known_schema;
    use knx_etsproj::map::{map, MapProblemDetail};
    use knx_etsproj::parse::parse_installation_v21;
    use knx_etsproj::values::ValueError;
    use std::io::{Cursor, Write};

    for token in [
        "Stairway",
        "RoomPart",
        "Area",
        "Ground",
        "Segment",
        "FutureSpace",
    ] {
        let xml = format!(
            r#"<KNX xmlns="http://knx.org/xml/project/23"><Project Id="P-0001"><Installations><Installation InstallationId="0" Name="Test"><Locations><Space Id="P-0001-0_BP-1" Name="Test space" Type="{token}" Puid="1" /></Locations></Installation></Installations></Project></KNX>"#
        );
        let parsed =
            parse_installation_v21(xml.as_bytes(), "P-0001/0.xml", known_schema(23).unwrap())
                .unwrap();
        let mapped = map(&parsed.document, "P-0001/0.xml").unwrap();
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for (path, bytes) in [
            ("P-0001.signature", b"x".as_slice()),
            ("P-0001/0.xml", xml.as_bytes()),
            ("P-0001/Project.xml", br#"<KNX xmlns="http://knx.org/xml/project/23"><Project Id="P-0001"><ProjectInformation Name="Test" GroupAddressStyle="ThreeLevel" /></Project></KNX>"#.as_slice()),
        ] {
            zip.start_file(path, zip::write::SimpleFileOptions::default()).unwrap();
            zip.write_all(bytes).unwrap();
        }
        let imported =
            knx_etsproj::import_knxproj_bytes(zip.finish().unwrap().into_inner(), "spaces.knxproj")
                .unwrap();
        let reported_type_errors: Vec<_> = imported
            .report
            .errors
            .iter()
            .filter(|error| error.stage == "map" && error.detail.contains("BuildingPart/@Type"))
            .collect();
        assert_eq!(
            reported_type_errors.len(),
            usize::from(token == "FutureSpace")
        );
        if token == "FutureSpace" {
            assert!(reported_type_errors[0].detail.contains(token));
        }
        let type_problems: Vec<_> = mapped
            .problems
            .iter()
            .filter(|problem| {
                matches!(
                    &problem.detail,
                    MapProblemDetail::Value(ValueError::UnknownEnumValue {
                        kind: "BuildingPart/@Type",
                        ..
                    })
                )
            })
            .collect();
        if token == "FutureSpace" {
            assert_eq!(type_problems.len(), 1);
            assert!(
                matches!(&type_problems[0].detail, MapProblemDetail::Value(ValueError::UnknownEnumValue { value, .. }) if value == token)
            );
            assert_eq!(
                mapped.project.installations[0].buildings[0].kind,
                BuildingPartType::BuildingPart
            );
        } else {
            assert!(type_problems.is_empty(), "{token}: {type_problems:?}");
            assert_eq!(
                format!("{:?}", mapped.project.installations[0].buildings[0].kind),
                token
            );
        }
    }
}

fn reference_ets4_path() -> PathBuf {
    knx_testsupport::reference_ets4_path()
}

#[test]
#[ignore = "requires the gitignored OriginalData/ corpus; run with --ignored"]
fn the_reference_project_imports_with_the_measured_counts() {
    assert!(
        reference_ets4_path().exists(),
        "OriginalData/ corpus not present (gitignored, local-only); this test is #[ignore]d and must be run explicitly on a machine that has it"
    );
    let out = import_knxproj(&reference_ets4_path()).unwrap();
    let p = &out.project;
    let inst = &p.installations[0];

    assert_eq!(p.installations.len(), 1);
    assert_eq!(inst.topology.areas.len(), 1);
    assert_eq!(inst.topology.lines.len(), 1);

    // 35 devices on the line plus the one unassigned device xknxproject loses.
    assert_eq!(p.devices.iter().count(), 36);
    assert_eq!(inst.topology.unassigned.len(), 1);

    assert_eq!(inst.group_ranges.len(), 35);
    assert_eq!(
        inst.group_ranges
            .iter()
            .filter(|r| r.parent.is_none())
            .count(),
        7
    );
    assert_eq!(inst.group_addresses.len(), 514);
    assert_eq!(inst.group_addresses.iter().filter(|g| g.central).count(), 2);
    assert_eq!(
        inst.group_addresses.iter().filter(|g| g.unfiltered).count(),
        2
    );

    assert_eq!(inst.buildings.len(), 22);
    assert_eq!(
        inst.buildings
            .iter()
            .filter(|b| b.kind == BuildingPartType::Room)
            .count(),
        14
    );
    assert_eq!(
        inst.buildings
            .iter()
            .map(|b| b.devices.len())
            .sum::<usize>(),
        29
    );

    assert_eq!(inst.parameters.len(), 1390);
    assert_eq!(
        inst.parameters
            .iter()
            .filter(|p| p.source.ets_id.contains("_UP-"))
            .count(),
        216
    );

    let coms: Vec<_> = p
        .devices
        .iter()
        .flat_map(|d| d.com_objects.clone())
        .filter_map(|id| p.devices.com_object(id))
        .collect();
    assert_eq!(coms.len(), 907);

    let (send, receive) =
        coms.iter()
            .flat_map(|c| c.links.iter())
            .fold((0, 0), |(s, r), l| match l.direction {
                Direction::Send => (s + 1, r),
                Direction::Receive => (s, r + 1),
            });
    assert_eq!((send, receive), (569, 27));

    assert_eq!(
        coms.iter()
            .filter(|c| matches!(c.dpt, Override::Value(_)))
            .count(),
        261
    );
    assert_eq!(
        coms.iter()
            .filter(|c| matches!(c.dpt, Override::Empty))
            .count(),
        497
    );
    assert_eq!(
        coms.iter().filter(|c| c.description.is_present()).count(),
        691
    );
    assert_eq!(coms.iter().filter(|c| c.text.is_present()).count(), 121);
    assert_eq!(
        coms.iter().filter(|c| c.flags.read.is_present()).count(),
        39
    );
    assert_eq!(
        coms.iter().filter(|c| c.flags.update.is_present()).count(),
        30
    );
    assert_eq!(
        coms.iter()
            .filter(|c| c.flags.transmit.is_present())
            .count(),
        27
    );
    assert_eq!(
        coms.iter().filter(|c| c.flags.write.is_present()).count(),
        18
    );
    assert_eq!(
        coms.iter()
            .filter(|c| c.flags.communication.is_present())
            .count(),
        8
    );

    assert_eq!(
        p.devices.iter().flat_map(|d| d.binary_data.iter()).count(),
        3
    );
}

#[test]
#[ignore = "requires the gitignored OriginalData/ corpus; run with --ignored"]
fn nothing_in_the_reference_project_is_unknown_or_lost() {
    assert!(
        reference_ets4_path().exists(),
        "OriginalData/ corpus not present (gitignored, local-only); this test is #[ignore]d and must be run explicitly on a machine that has it"
    );
    let out = import_knxproj(&reference_ets4_path()).unwrap();
    assert_eq!(out.report.unknown, vec![]);
    assert_eq!(out.report.errors, vec![]);
    assert!(!out.report.has_losses());
    for row in &out.report.counts.rows {
        assert_eq!(
            row.read, row.mapped,
            "{} lost entities in mapping",
            row.entity
        );
    }
}
