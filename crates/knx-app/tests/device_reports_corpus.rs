//! Real diagnostic exports from physical devices still pass the strict reader.
//!
//! Corpus: `OriginalData/DeviceReports/` (private); contract:
//! docs/contracts/device-diagnostic-export.md. The simulator tests prove the
//! writer and reader agree with each other. This gate proves that archives
//! collected on real hardware by a released source build still pass today's
//! strict reader, that the current writer reproduces them byte for byte, and
//! that the real replies have the shapes the contract promises. Override the
//! directory with `KNXBENCH_DEVICE_REPORTS`.
//!
//! Only aggregate counts are asserted or printed: no file name, reply byte,
//! project reference or timestamp leaves the corpus. The pins change only with
//! a reviewed explanation (a new report arrived, or the reader changed).

use std::collections::BTreeMap;
use std::path::PathBuf;

use knx_app::device_diagnostics::{bundle, read_bundle, Report};

/// Twelve archives: the issue #22 detector plus eleven more devices received by
/// email (docs/research/2026-10-18-device-diagnostics-eleven-devices.md).
const ARCHIVES: usize = 12;

fn reports_dir() -> PathBuf {
    let dir = std::env::var_os("KNXBENCH_DEVICE_REPORTS")
        .map(PathBuf::from)
        .unwrap_or_else(|| knx_testsupport::workspace_root().join("OriginalData/DeviceReports"));
    assert!(
        dir.is_dir(),
        "SKIP: set KNXBENCH_DEVICE_REPORTS; {} is not a directory",
        dir.display()
    );
    dir
}

/// Every `*.zip` directly in the directory, in a stable order. Directory or
/// entry errors panic: an unreadable corpus must never shrink the sample.
fn archives() -> Vec<Vec<u8>> {
    let dir = reports_dir();
    let mut paths = Vec::new();
    for entry in std::fs::read_dir(&dir).expect("device reports directory unreadable") {
        let entry = entry.expect("device reports directory entry unreadable");
        let kind = entry
            .file_type()
            .expect("device report file type unreadable");
        let path = entry.path();
        if kind.is_file() && path.extension().and_then(|e| e.to_str()) == Some("zip") {
            paths.push(path);
        }
    }
    paths.sort();
    paths
        .iter()
        .map(|p| std::fs::read(p).expect("device report unreadable"))
        .collect()
}

fn item<'a>(report: &'a Report, query: &str) -> &'a knx_app::device_diagnostics::Reading {
    report
        .items
        .iter()
        .find(|i| i.query == query)
        .unwrap_or_else(|| panic!("a real report lacks the {query} item"))
}

fn word(bytes: &[u8]) -> Option<u16> {
    <[u8; 2]>::try_from(bytes).ok().map(u16::from_be_bytes)
}

#[test]
#[ignore = "requires the gitignored OriginalData/DeviceReports corpus; run with --ignored"]
fn real_device_exports_pass_the_strict_reader_and_keep_their_shapes() {
    let archives = archives();
    assert_eq!(archives.len(), ARCHIVES, "device report count drifted");

    let mut statuses: BTreeMap<(String, String), usize> = BTreeMap::new();
    let mut masks: BTreeMap<u16, usize> = BTreeMap::new();
    let mut manufacturers: BTreeMap<u16, usize> = BTreeMap::new();
    let mut asserted = 0;
    let mut byte_identical = 0;

    for (ordinal, bytes) in archives.iter().enumerate() {
        let report = read_bundle(bytes)
            .unwrap_or_else(|code| panic!("archive #{ordinal} refused by the reader: {code}"));

        // The identity-only boundary, as recorded on the device.
        assert_eq!(report.format, 1, "archive #{ordinal}");
        assert_eq!(report.profile, "generic-identity-v1", "archive #{ordinal}");
        assert!(
            !report.simulated,
            "archive #{ordinal} is a simulator capture"
        );
        assert!(!report.written, "archive #{ordinal} claims a write");
        assert!(report.complete, "archive #{ordinal} is partial");
        assert_eq!(report.stop_reason, None, "archive #{ordinal}");
        assert_eq!(report.memory_status, "not-requested", "archive #{ordinal}");
        assert!(report.disconnect_attempt_completed, "archive #{ordinal}");
        assert_eq!(report.items.len(), 3, "archive #{ordinal}");

        // Today's writer reproduces the real archive exactly.
        if bundle(&report).expect("a read report re-bundles") == *bytes {
            byte_identical += 1;
        }

        for reading in &report.items {
            *statuses
                .entry((reading.query.clone(), reading.status.clone()))
                .or_default() += 1;
            match reading.status.as_str() {
                "read" => {
                    assert_eq!(reading.bytes.len(), reading.expected_bytes);
                    assert_eq!(reading.error, None);
                }
                // AL §3.4.4.2: zero elements, no data, the ambiguous token;
                // never zeros standing in for a value.
                "refused-ambiguous" => {
                    assert!(reading.bytes.is_empty());
                    assert_eq!(reading.returned_elements, Some(0));
                    assert_eq!(
                        reading.error.as_deref(),
                        Some("access-denied-or-property-unsupported")
                    );
                }
                other => panic!("archive #{ordinal}: unexpected real status {other}"),
            }
        }

        let mask = word(&item(&report, "descriptor-type-0").bytes).expect("descriptor read");
        let manufacturer = word(&item(&report, "manufacturer-id").bytes).expect("PID 12 read");
        *masks.entry(mask).or_default() += 1;
        *manufacturers.entry(manufacturer).or_default() += 1;
        // A completed run never carries an assertion the device contradicted.
        if let Some(expected) = report.asserted_mask {
            assert_eq!(expected, mask, "archive #{ordinal}: mask assertion");
            asserted += 1;
        }
        if let Some(expected) = report.asserted_manufacturer {
            assert_eq!(
                expected, manufacturer,
                "archive #{ordinal}: manufacturer assertion"
            );
        }
    }

    let count = |query: &str, status: &str| {
        statuses
            .get(&(query.to_string(), status.to_string()))
            .copied()
            .unwrap_or(0)
    };
    eprintln!(
        "device reports: {} archives, {} byte-identical, {} asserted, statuses {:?}",
        archives.len(),
        byte_identical,
        asserted,
        statuses.values().collect::<Vec<_>>()
    );
    assert_eq!(
        byte_identical, ARCHIVES,
        "writer no longer reproduces real archives"
    );
    assert_eq!(asserted, 11, "catalogue-asserted runs");
    assert_eq!(count("descriptor-type-0", "read"), 12);
    assert_eq!(count("manufacturer-id", "read"), 12);
    assert_eq!(count("hardware-type", "read"), 10);
    assert_eq!(count("hardware-type", "refused-ambiguous"), 2);
    assert_eq!(
        statuses.values().sum::<usize>(),
        36,
        "no other status occurred"
    );
    assert_eq!(
        masks,
        BTreeMap::from([(0x0025, 1), (0x0701, 7), (0x0705, 2), (0x07B0, 2)]),
        "mask distribution"
    );
    assert_eq!(manufacturers.len(), 5, "distinct manufacturers");
}
