//! Measures changed-caption large snapshots and preserves recovery-envelope ceilings.
use super::*;
use std::time::Instant;

#[tokio::test]
async fn caption_current_project_over_budget_refuses_without_trimming_or_writes() {
    for retained in [false, true] {
        let input = f::inputs(23, retained);
        let bundle = observed(&input.conn, &input.project, &input.profile).await;
        let frozen = serde_json::to_vec(&bundle).unwrap();
        let changed = f::caption(&input.package);
        let input = input.replace(&changed, false);
        assert_eq!(
            parts(&input.conn, &input.project, &bundle),
            Some(ProgrammingParts::ALL)
        );
        let mut large = input.project.clone();
        // Raw snapshot is legal; its canonical base64 expansion exceeds the retained-byte limit.
        large.devices.get_mut(f::DEVICE).unwrap().description = Some("x".repeat(13 * 1024 * 1024));
        let snapshot = knx_store::project_history::NativeSnapshot {
            project: large.clone(),
            opaque: vec![],
            manufacturer_refs: vec![],
        }
        .encode()
        .unwrap();
        assert!(snapshot.len() < 16 * 1024 * 1024);
        assert!(snapshot.len().div_ceil(3) * 4 > 16 * 1024 * 1024);
        let writes = input.conn.total_changes();
        let error =
            current_project_hardware_parts(&input.conn, &large, f::DEVICE, &bundle).unwrap_err();
        assert_eq!(error.to_string(), "retained byte limit");
        assert_eq!(
            large
                .devices
                .get(f::DEVICE)
                .unwrap()
                .description
                .as_ref()
                .unwrap()
                .len(),
            13 * 1024 * 1024
        );
        assert_eq!(input.conn.total_changes(), writes);
        assert_eq!(serde_json::to_vec(&bundle).unwrap(), frozen);
        assert_eq!(
            parts(&input.conn, &input.project, &bundle),
            Some(ProgrammingParts::ALL)
        );
    }
}

/// Parent runs explicitly with --ignored --nocapture; timings have no pass threshold.
#[tokio::test]
#[ignore = "opt-in changed-caption large-project measurement; no hardware or network"]
async fn caption_changed_source_large_project_measurement() {
    const EXTRA: u16 = 5000;
    for retained in [false, true] {
        let mut input = f::inputs(23, retained);
        for n in 0..EXTRA {
            let mut other = input.project.devices.get(f::DEVICE).unwrap().clone();
            other.id = input.project.ids.next_device_id().unwrap();
            other.name = format!("Unrelated synthetic {n}");
            other.source.ets_id = format!("KB-LARGE-{n}");
            other.address = Some(knx_core::IndividualAddress::from_raw(0x2000 + n));
            other.product_ref.clear();
            other.program_ref.clear();
            other.com_objects.clear();
            input.project.installations[0]
                .topology
                .unassigned
                .push(other.id);
            input.project.devices.insert(other);
        }
        let bundle = observed(&input.conn, &input.project, &input.profile).await;
        let source = bundle.project_preparation.as_ref().unwrap();
        let snapshot_bytes = source.project_snapshot.len();
        let bundle_bytes = serde_json::to_vec(&bundle).unwrap().len();
        let frozen = serde_json::to_vec(&bundle).unwrap();
        assert_eq!(input.project.devices.iter().count(), usize::from(EXTRA) + 1);
        assert!(snapshot_bytes > 500_000);
        assert!(bundle_bytes < 16 * 1024 * 1024);
        let changed = f::caption(&input.package);
        let input = input.replace(&changed, false);
        let writes = input.conn.total_changes();
        for sample in 1..=3 {
            let start = Instant::now();
            assert!(admit_current_project_hardware(
                &input.conn,
                &input.project,
                f::DEVICE,
                &bundle
            )
            .is_err());
            let strict_refusal = start.elapsed();
            let start = Instant::now();
            assert_eq!(
                parts(&input.conn, &input.project, &bundle),
                Some(ProgrammingParts::ALL)
            );
            let comparison = start.elapsed();
            eprintln!("E17_LARGE family={} namespace=23 sample={sample} devices={} snapshot_bytes={snapshot_bytes} bundle_bytes={bundle_bytes} strict_refusal_us={} comparison_us={} envelope_ceiling_bytes={}",
                if retained { "retained" } else { "standalone" }, usize::from(EXTRA) + 1,
                strict_refusal.as_micros(), comparison.as_micros(), 16 * 1024 * 1024);
        }
        assert_eq!(input.conn.total_changes(), writes);
        assert_eq!(serde_json::to_vec(&bundle).unwrap(), frozen);
    }
}
