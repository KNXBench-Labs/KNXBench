//! Bounded support diagnostics must never send an application write or unlock.
use knx_net::commissioning::simulator::{Seen, SimulatedDevice, SimulatorConfig};
use knx_net::commissioning::support_diagnostics::{collect, profile};
use knx_net::SessionTiming;

#[test]
fn similar_masks_do_not_inherit_unreviewed_resource_mappings() {
    for mask in [0x0020, 0x0021, 0x0701] {
        assert!(profile(mask).is_empty(), "{mask:04X}");
    }
}

#[tokio::test]
async fn unknown_family_reads_descriptor_only_and_never_writes() {
    let d = SimulatedDevice::with_config_at(
        "1.1.67".parse().unwrap(),
        SimulatorConfig {
            mask_version: 0xFAFE,
            ..Default::default()
        },
    );
    let r = collect(
        &d,
        "1.1.67".parse().unwrap(),
        SessionTiming::default(),
        Some(0xFAFE),
    )
    .await
    .unwrap();
    assert_eq!(r.mask, 0xFAFE);
    assert_eq!(r.profile, "descriptor-only-v1");
    assert!(r.facts.is_empty());
    assert!(!d.memory_was_written());
    assert_eq!(d.authorize_requests(), 0);
    assert!(!d.seen().iter().any(|x| matches!(
        x,
        Seen::PropertyWrite { .. } | Seen::Restart { .. } | Seen::Authorize(_)
    )));
    assert_eq!(profile(0xFAFE).len(), 0);
}

#[tokio::test]
async fn a_system2_profile_exports_only_fixed_fields_without_unlock_or_writes() {
    let d = SimulatedDevice::with_config_at(
        "1.1.67".parse().unwrap(),
        SimulatorConfig {
            mask_version: 0x0025,
            ..Default::default()
        },
    );
    d.preset_property(0, 12, &[0, 1]);
    d.preset_property(0, 78, &[0, 1, 0, 0, 0, 0x5B]);
    let r = collect(
        &d,
        "1.1.67".parse().unwrap(),
        SessionTiming::default(),
        Some(0x0025),
    )
    .await
    .unwrap();
    assert_eq!(r.profile, "system2-status-v1");
    assert_eq!(r.facts.len(), 5);
    assert_eq!(r.facts[1].value.as_deref(), Some("00010000005B"));
    assert!(!d.seen().iter().any(|x| matches!(
        x,
        Seen::MemoryWrite { .. }
            | Seen::PropertyWrite { .. }
            | Seen::Restart { .. }
            | Seen::Authorize(_)
    )));
    assert_eq!(d.serial_number_reads(), 0);
}

#[tokio::test]
async fn mismatching_identity_never_starts_family_reads() {
    let d = SimulatedDevice::with_config_at(
        "1.1.67".parse().unwrap(),
        SimulatorConfig {
            mask_version: 0x0701,
            ..Default::default()
        },
    );
    assert_eq!(
        collect(
            &d,
            "1.1.67".parse().unwrap(),
            SessionTiming::default(),
            Some(0x0025)
        )
        .await
        .unwrap_err(),
        "identity-mismatch"
    );
    assert!(!d
        .seen()
        .iter()
        .any(|x| matches!(x, Seen::PropertyRead { .. } | Seen::MemoryRead { .. })));
}

#[tokio::test]
async fn unknown_expected_mask_does_not_silently_expand_the_displayed_scope() {
    let d = SimulatedDevice::with_config_at(
        "1.1.67".parse().unwrap(),
        SimulatorConfig {
            mask_version: 0x0025,
            ..Default::default()
        },
    );
    let r = collect(
        &d,
        "1.1.67".parse().unwrap(),
        SessionTiming::default(),
        None,
    )
    .await
    .unwrap();
    assert_eq!(r.profile, "descriptor-only-v1");
    assert!(r.facts.is_empty());
}

#[tokio::test]
async fn bounded_memory_profile_exports_bit_zero_not_raw_memory() {
    let d = SimulatedDevice::with_config_at(
        "1.1.67".parse().unwrap(),
        SimulatorConfig {
            mask_version: 0x0012,
            ..Default::default()
        },
    );
    d.preset_memory(0x60, &[0xFF]);
    let r = collect(
        &d,
        "1.1.67".parse().unwrap(),
        SessionTiming::default(),
        Some(0x0012),
    )
    .await
    .unwrap();
    assert_eq!(r.facts.len(), 1);
    assert!(matches!(
        r.facts[0].value.as_deref(),
        Some("true" | "false")
    ));
    let reads: Vec<_> = d
        .seen()
        .into_iter()
        .filter_map(|x| match x {
            Seen::MemoryRead {
                address, number, ..
            } => Some((address, number)),
            _ => None,
        })
        .collect();
    assert_eq!(reads, [(0x60, 1)]);
    assert!(!d.memory_was_written());
}
