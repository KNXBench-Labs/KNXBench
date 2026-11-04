//! Both selector families settle one caption change without lending start or recovery authority.
mod adversaries;
mod limits;
mod review;
use super::super::caption_fixture as f;
use super::per_part::{observed, parameter, parts, ADR_APP, ADR_APP_PAR};
use super::*;
use knx_app::system2_validation_preparation::current_project_hardware_parts;
use knx_core::commissioning::pending::ProgrammingParts;
use knx_core::{flags::Direction, ComObjectInstanceId};

async fn positive(namespace: u8, retained: bool) {
    let input = f::inputs(namespace, retained);
    let bundle = observed(&input.conn, &input.project, &input.profile).await;
    let frozen = serde_json::to_vec(&bundle).unwrap();
    assert_eq!(
        bundle
            .project_preparation
            .as_ref()
            .unwrap()
            .request
            .program_id,
        f::PROGRAM
    );
    let changed = f::caption(&input.package);
    let input = input.replace(&changed, false);
    assert!(
        admit_current_project_hardware(&input.conn, &input.project, f::DEVICE, &bundle).is_err()
    );
    assert_eq!(
        parts(&input.conn, &input.project, &bundle),
        Some(ProgrammingParts::ALL)
    );
    assert_eq!(serde_json::to_vec(&bundle).unwrap(), frozen);
    bundle.plan().unwrap();
    bundle.preconditions().unwrap();
}
#[tokio::test]
async fn caption_standalone_namespace20() {
    positive(20, false).await;
}
#[tokio::test]
async fn caption_standalone_namespace23() {
    positive(23, false).await;
}
#[tokio::test]
async fn caption_retained_namespace20() {
    positive(20, true).await;
}
#[tokio::test]
async fn caption_retained_namespace23() {
    positive(23, true).await;
}

#[tokio::test]
async fn caption_strict_success_ignores_other_containers_and_preserves_current_masks() {
    for namespace in [20, 23] {
        for retained in [false, true] {
            let input = f::inputs(namespace, retained);
            // Original noncanonical JSON formatting is retained byte-exact.
            let profile = serde_json::to_vec_pretty(
                &serde_json::from_slice::<serde_json::Value>(&input.profile).unwrap(),
            )
            .unwrap();
            let bundle = observed(&input.conn, &input.project, &profile).await;
            let other = f::package(
                &f::members(&input.package),
                "P1 same members other container",
            );
            assert_ne!(
                knx_productdb::sha256_hex(&input.package),
                knx_productdb::sha256_hex(&other)
            );
            knx_productdb::install_package(&input.conn, "P1.knxprod", &other).unwrap();
            assert_eq!(
                parts(&input.conn, &input.project, &bundle),
                Some(ProgrammingParts::ALL)
            );
            let mut edited = input.project.clone();
            parameter(&mut edited, "772");
            assert_eq!(parts(&input.conn, &edited, &bundle), Some(ADR_APP));
            assert_eq!(
                bundle.project_preparation.as_ref().unwrap().profile_bytes,
                profile
            );
            // A current-request refusal after strict success is terminal, never fallback.
            parameter(&mut edited, "not-a-number");
            assert!(
                current_project_hardware_parts(&input.conn, &edited, f::DEVICE, &bundle).is_err()
            );
        }
    }
}

#[tokio::test]
async fn caption_changed_standalone_ambiguous_containers_refuse_but_retained_does_not_discover_packages(
) {
    for retained in [false, true] {
        let input = f::inputs(20, retained);
        let bundle = observed(&input.conn, &input.project, &input.profile).await;
        let changed = f::caption(&input.package);
        let input = input.replace(&changed, true);
        assert_eq!(
            parts(&input.conn, &input.project, &bundle),
            retained.then_some(ProgrammingParts::ALL)
        );
    }
}

#[tokio::test]
async fn caption_both_families_preserve_current_request_dependency_masks_and_refs() {
    for retained in [false, true] {
        let input = f::inputs(20, retained);
        let bundle = observed(&input.conn, &input.project, &input.profile).await;
        let changed = f::caption(&input.package);
        let input = input.replace(&changed, false);
        let mut renamed = input.project.clone();
        renamed.devices.get_mut(f::DEVICE).unwrap().name.push('!');
        renamed.installations[0].group_addresses[0].name.push('!');
        assert_eq!(
            parts(&input.conn, &renamed, &bundle),
            Some(ProgrammingParts::ALL)
        );
        let mut links = input.project.clone();
        links
            .devices
            .com_object_mut(ComObjectInstanceId(1))
            .unwrap()
            .links[0]
            .direction = Direction::Receive;
        assert_eq!(parts(&input.conn, &links, &bundle), Some(ADR_APP_PAR));
        let mut flags = input.project.clone();
        flags
            .devices
            .com_object_mut(ComObjectInstanceId(1))
            .unwrap()
            .flags
            .write = exported(false);
        assert_eq!(parts(&input.conn, &flags, &bundle), Some(ADR_APP_PAR));
        let mut parameters = input.project.clone();
        parameter(&mut parameters, "772");
        assert_eq!(parts(&input.conn, &parameters, &bundle), Some(ADR_APP));
        parameter(&mut parameters, "not-a-number");
        assert_eq!(parts(&input.conn, &parameters, &bundle), None);
        for field in 0..4 {
            let mut switched = input.project.clone();
            let d = switched.devices.get_mut(f::DEVICE).unwrap();
            match field {
                0 => d.product_ref.push('!'),
                1 => d.program_ref.push('!'),
                2 => d.source.path.push('!'),
                _ => d.source.ets_id.push('!'),
            }
            assert_eq!(parts(&input.conn, &switched, &bundle), None);
        }
    }
}

#[tokio::test]
async fn caption_nonclass_equal_plan_and_unrelated_package_bytes_refuse() {
    for retained in [false, true] {
        for attack in 0..3 {
            let input = f::inputs(20, retained);
            let bundle = observed(&input.conn, &input.project, &input.profile).await;
            let current = f::changed(&input.package, |s| match attack {
                0 => s.replace("Name=\"Synthetic\"", "Name='Revised caption'"),
                1 => s.replace("Name=\"Synthetic\"", "Name=\"Revised caption\" "),
                _ => s
                    .replace("Name=\"Synthetic\"", "Name=\"Revised caption\"")
                    .replace("Value=\"4660\"", "Value=\"4661\""),
            });
            let input = input.replace(&current, false);
            assert_eq!(
                parts(&input.conn, &input.project, &bundle),
                None,
                "family={retained} attack={attack}"
            );
        }
    }
    let input = f::inputs(20, false);
    let bundle = observed(&input.conn, &input.project, &input.profile).await;
    let mut entries = f::members(&f::caption(&input.package));
    entries.push(("unrelated.bin".into(), vec![1]));
    let input = input.replace(&f::package(&entries, ""), false);
    assert_eq!(parts(&input.conn, &input.project, &bundle), None);
}

#[tokio::test]
async fn caption_standalone_inventory_keeps_empty_directories_order_and_hardware_bytes() {
    for attack in 0..3 {
        let input = f::inputs(20, false);
        let bundle = observed(&input.conn, &input.project, &input.profile).await;
        let mut entries = f::members(&f::caption(&input.package));
        assert!(entries
            .iter()
            .any(|(name, bytes)| name == "empty/" && bytes.is_empty()));
        match attack {
            0 => entries.retain(|(name, _)| name != "empty/"),
            1 => entries.swap(0, 1),
            _ => {
                let (_, hardware) = entries
                    .iter_mut()
                    .find(|(name, _)| name.ends_with("Hardware.xml"))
                    .unwrap();
                *hardware = String::from_utf8(hardware.clone())
                    .unwrap()
                    .replace("Name=\"Synthetic\"", "Name=\"Other hardware caption\"")
                    .into_bytes();
            }
        }
        let input = input.replace(&f::package(&entries, ""), false);
        assert_eq!(
            parts(&input.conn, &input.project, &bundle),
            None,
            "attack={attack}"
        );
    }
}

#[tokio::test]
async fn caption_original_partial_corruption_cannot_be_healed() {
    let input = f::inputs(20, true);
    let bundle = observed(&input.conn, &input.project, &input.profile).await;
    let changed = f::caption(&input.package);
    let input = input.replace(&changed, false);
    for attack in 0..5 {
        let mut bad = bundle.clone();
        match attack {
            0 => {
                bad.hardware_preparation
                    .as_mut()
                    .unwrap()
                    .source_preparation_sha256 = "0".repeat(64)
            }
            1 => bad
                .project_preparation
                .as_mut()
                .unwrap()
                .request
                .program_id
                .push('!'),
            2 => bad.project_preparation.as_mut().unwrap().coverage.reverse(),
            3 => bad.binding.serial = [9; 6],
            _ => {
                bad.hardware_preparation
                    .as_mut()
                    .unwrap()
                    .pointer_width_policy = Some("unknown".into())
            }
        }
        assert_eq!(
            parts(&input.conn, &input.project, &bad),
            None,
            "attack={attack}"
        );
    }
}

#[tokio::test]
async fn caption_unchanged_source_failed_exact_pins_and_operational_failure_are_terminal() {
    for retained in [false, true] {
        let input = f::inputs(20, retained);
        let bundle = observed(&input.conn, &input.project, &input.profile).await;
        input.conn.execute_batch("BEGIN").unwrap();
        assert_eq!(parts(&input.conn, &input.project, &bundle), None);
        input.conn.execute_batch("ROLLBACK").unwrap();
        assert_eq!(
            parts(&input.conn, &input.project, &bundle),
            Some(ProgrammingParts::ALL)
        );
        input
            .conn
            .execute_batch("PRAGMA foreign_keys=OFF; DELETE FROM package_member")
            .unwrap();
        if !retained {
            assert_eq!(parts(&input.conn, &input.project, &bundle), None);
        }
    }
}

#[tokio::test]
async fn caption_unchanged_source_with_valid_other_container_cannot_repair_failed_original_pins() {
    let input = f::inputs(20, false);
    let bundle = observed(&input.conn, &input.project, &input.profile).await;
    let other = f::package(
        &f::members(&input.package),
        "P1 available while P0 binding refuses",
    );
    knx_productdb::install_package(&input.conn, "P1.knxprod", &other).unwrap();
    let old = knx_productdb::sha256_hex(&input.package);
    input
        .conn
        .execute("DELETE FROM package_member WHERE package_sha256=?1", [&old])
        .unwrap();
    let other_pin = knx_productdb::sha256_hex(&other);
    let count: i64 = input
        .conn
        .query_row(
            "SELECT COUNT(*) FROM package_member WHERE package_sha256=?1",
            [&other_pin],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 4);
    assert_eq!(parts(&input.conn, &input.project, &bundle), None);
}

#[tokio::test]
async fn caption_changed_source_missing_stale_duplicate_or_repathed_owners_are_not_repaired() {
    for retained in [false, true] {
        for attack in [
            "DELETE FROM source_parse_evidence",
            "DELETE FROM source_identity_scan",
            "UPDATE source_identity_scan SET scanner=scanner+1",
            "UPDATE source_identity_scan SET status='unavailable',reason='synthetic unavailable'",
            "INSERT INTO source_identity SELECT source_sha256,table_name,logical_id,2,digest FROM source_identity WHERE table_name='application_program'",
            "UPDATE source_file SET source_path='different/Application.xml' WHERE sha256=(SELECT source_sha256 FROM application_program LIMIT 1)",
        ] {
            let input = f::inputs(20, retained);
            let bundle = observed(&input.conn, &input.project, &input.profile).await;
            let changed = f::caption(&input.package);
            let input = input.replace(&changed, false);
            assert_eq!(parts(&input.conn, &input.project, &bundle), Some(ProgrammingParts::ALL));
            // Deliberately corrupt measured evidence, not a failed SQL setup operation.
            input.conn.execute_batch("PRAGMA foreign_keys=OFF").unwrap();
            input.conn.execute_batch(attack).unwrap();
            let changes = input.conn.total_changes();
            assert_eq!(parts(&input.conn, &input.project, &bundle), None, "{retained}: {attack}");
            assert_eq!(input.conn.total_changes(), changes, "read proof never repairs evidence");
        }
    }
}
