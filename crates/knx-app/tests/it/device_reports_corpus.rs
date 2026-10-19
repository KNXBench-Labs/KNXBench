//! Real diagnostic exports from physical devices still pass the strict reader.
//!
//! Corpus: `OriginalData/DeviceReports/` (private); contract:
//! docs/contracts/device-diagnostic-export.md. The simulator tests prove the
//! writer and reader agree with each other. This gate proves that archives
//! collected on real hardware still pass today's strict reader, that the
//! current writer reproduces them byte for byte, and that the real replies have
//! the shapes the contract promises. Two sets, each pinned separately: the
//! emailed archives (`KNXBENCH_DEVICE_REPORTS`, the `*.zip` directly in the
//! directory) and the maintainer's own house run
//! (`KNXBENCH_HOUSE_DEVICE_REPORTS`, default `Unser Zuhause 2026-10-18/` below
//! it).
//!
//! Only aggregate counts are asserted or printed: no file name, reply byte,
//! project reference or timestamp leaves the corpus. The pins change only with
//! a reviewed explanation (a new report arrived, or the reader changed).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use knx_app::device_diagnostics::{bundle, read_bundle, Reading, Report};

/// Error tokens a real run produced. Anything else is new evidence to review.
const REAL_ERRORS: &[&str] = &["access-denied-or-property-unsupported", "connection-lost"];

fn dir_from(var: &str, default: &Path) -> PathBuf {
    let dir = std::env::var_os(var)
        .map(PathBuf::from)
        .unwrap_or_else(|| default.to_path_buf());
    assert!(
        dir.is_dir(),
        "SKIP: set {var}; {} is not a directory",
        dir.display()
    );
    dir
}

fn reports_root() -> PathBuf {
    knx_testsupport::workspace_root().join("OriginalData/DeviceReports")
}

/// Every `*.zip` directly in the directory, in a stable order. Directory or
/// entry errors panic: an unreadable corpus must never shrink the sample.
fn archives(dir: &Path) -> Vec<Vec<u8>> {
    let mut paths = Vec::new();
    for entry in std::fs::read_dir(dir).expect("device reports directory unreadable") {
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

fn item<'a>(report: &'a Report, query: &str) -> &'a Reading {
    report
        .items
        .iter()
        .find(|i| i.query == query)
        .unwrap_or_else(|| panic!("a real report lacks the {query} item"))
}

fn word(bytes: &[u8]) -> Option<u16> {
    <[u8; 2]>::try_from(bytes).ok().map(u16::from_be_bytes)
}

/// Aggregates of one set; every field is pinned by the caller.
#[derive(Debug, Default, PartialEq, Eq)]
struct Summary {
    archives: usize,
    byte_identical: usize,
    asserted: usize,
    statuses: BTreeMap<(String, String), usize>,
    stop_reasons: BTreeMap<String, usize>,
    masks: BTreeMap<u16, usize>,
    manufacturers: usize,
}

/// Checks one reading against the contract's per-item shapes. Returns whether
/// the run has stopped at or before this item.
fn check_reading(ordinal: usize, report: &Report, reading: &Reading, stopped: bool) -> bool {
    match reading.status.as_str() {
        "read" => {
            assert!(!stopped, "archive #{ordinal}: a read after the stop");
            assert_eq!(reading.bytes.len(), reading.expected_bytes);
            assert_eq!(reading.error, None);
            false
        }
        // AL §3.4.4.2: zero elements, no data, the ambiguous token; never
        // zeros standing in for a value.
        "refused-ambiguous" => {
            assert!(reading.bytes.is_empty());
            assert_eq!(reading.returned_elements, Some(0));
            assert_eq!(
                reading.error.as_deref(),
                Some("access-denied-or-property-unsupported")
            );
            stopped
        }
        // An attempted read the device ended: no bytes invented, and nothing
        // is requested after it.
        "error" => {
            assert!(reading.bytes.is_empty(), "archive #{ordinal}");
            assert!(reading.attempted_at.is_some(), "archive #{ordinal}");
            let token = reading.error.as_deref().unwrap_or_default();
            assert!(REAL_ERRORS.contains(&token), "archive #{ordinal}: {token}");
            assert_eq!(report.stop_reason.as_deref(), Some(token));
            true
        }
        "not-read" => {
            assert!(reading.bytes.is_empty() && reading.attempted_at.is_none());
            assert!(report.stop_reason.is_some(), "archive #{ordinal}");
            true
        }
        other => panic!("archive #{ordinal}: unexpected real status {other}"),
    }
}

fn summarise(archives: &[Vec<u8>]) -> Summary {
    let mut summary = Summary {
        archives: archives.len(),
        ..Summary::default()
    };
    let mut manufacturers = BTreeMap::<u16, usize>::new();
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
        assert!(report.complete, "archive #{ordinal} is unfinished");
        assert_eq!(report.memory_status, "not-requested", "archive #{ordinal}");
        assert!(report.disconnect_attempt_completed, "archive #{ordinal}");
        assert_eq!(report.items.len(), 3, "archive #{ordinal}");
        if let Some(reason) = &report.stop_reason {
            *summary.stop_reasons.entry(reason.clone()).or_default() += 1;
        }

        // Today's writer reproduces the real archive exactly.
        if bundle(&report).expect("a read report re-bundles") == *bytes {
            summary.byte_identical += 1;
        }

        let mut stopped = false;
        for reading in &report.items {
            *summary
                .statuses
                .entry((reading.query.clone(), reading.status.clone()))
                .or_default() += 1;
            stopped = check_reading(ordinal, &report, reading, stopped);
        }

        let mask = word(&item(&report, "descriptor-type-0").bytes);
        if let Some(mask) = mask {
            *summary.masks.entry(mask).or_default() += 1;
        }
        if let Some(id) = word(&item(&report, "manufacturer-id").bytes) {
            *manufacturers.entry(id).or_default() += 1;
            if let Some(expected) = report.asserted_manufacturer {
                assert_eq!(expected, id, "archive #{ordinal}: manufacturer assertion");
            }
        }
        // A finished run never carries an assertion the device contradicted.
        if let Some(expected) = report.asserted_mask {
            assert_eq!(Some(expected), mask, "archive #{ordinal}: mask assertion");
            summary.asserted += 1;
        }
    }
    summary.manufacturers = manufacturers.len();
    eprintln!(
        "device reports: {} archives, {} byte-identical, {} asserted, statuses {:?}, stops {:?}",
        summary.archives,
        summary.byte_identical,
        summary.asserted,
        summary.statuses.values().collect::<Vec<_>>(),
        summary.stop_reasons.values().collect::<Vec<_>>()
    );
    summary
}

fn statuses(pins: &[(&str, &str, usize)]) -> BTreeMap<(String, String), usize> {
    pins.iter()
        .map(|(q, s, n)| ((q.to_string(), s.to_string()), *n))
        .collect()
}

/// The issue #22 detector plus eleven more devices received by email
/// (docs/research/2026-10-18-device-diagnostics-eleven-devices.md).
#[test]
#[ignore = "requires the gitignored OriginalData/DeviceReports corpus; run with --ignored"]
fn emailed_device_exports_pass_the_strict_reader_and_keep_their_shapes() {
    let dir = dir_from("KNXBENCH_DEVICE_REPORTS", &reports_root());
    let summary = summarise(&archives(&dir));
    assert_eq!(
        summary,
        Summary {
            archives: 12,
            byte_identical: 12,
            asserted: 11,
            statuses: statuses(&[
                ("descriptor-type-0", "read", 12),
                ("hardware-type", "read", 10),
                ("hardware-type", "refused-ambiguous", 2),
                ("manufacturer-id", "read", 12),
            ]),
            stop_reasons: BTreeMap::new(),
            masks: BTreeMap::from([(0x0025, 1), (0x0701, 7), (0x0705, 2), (0x07B0, 2)]),
            manufacturers: 5,
        }
    );
}

/// The maintainer's own 34 contactable house devices, read once on 18 October
/// 2026 (docs/research/2026-10-18-house-device-diagnostics.md).
#[test]
#[ignore = "requires the gitignored OriginalData/DeviceReports corpus; run with --ignored"]
fn house_device_exports_pass_the_strict_reader_and_keep_their_shapes() {
    let dir = dir_from(
        "KNXBENCH_HOUSE_DEVICE_REPORTS",
        &reports_root().join("Unser Zuhause 2026-10-18"),
    );
    let summary = summarise(&archives(&dir));
    assert_eq!(
        summary,
        Summary {
            archives: 34,
            byte_identical: 34,
            asserted: 0,
            statuses: statuses(&[
                ("descriptor-type-0", "not-read", 1),
                ("descriptor-type-0", "read", 33),
                ("hardware-type", "not-read", 3),
                ("hardware-type", "read", 30),
                ("hardware-type", "refused-ambiguous", 1),
                ("manufacturer-id", "error", 2),
                ("manufacturer-id", "not-read", 1),
                ("manufacturer-id", "read", 31),
            ]),
            stop_reasons: BTreeMap::from([
                ("connect-rejected".to_string(), 1),
                ("connection-lost".to_string(), 2),
            ]),
            masks: BTreeMap::from([(0x0012, 2), (0x0701, 31)]),
            manufacturers: 3,
        }
    );
}
