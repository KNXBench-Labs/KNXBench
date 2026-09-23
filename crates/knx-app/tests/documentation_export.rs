//! A corpus-gated proof that `knx-report::render_html` survives contact
//! with a real project: import the reference `.knxproj`, render its
//! documentation, and check the things a hand-built fixture cannot
//! exercise realistically — every real group address and device name,
//! thousands of table rows that must balance, and determinism against a
//! project big enough that a stray `HashMap` iteration would actually show
//! up as flaky output.
//!
//! Lives here, not in `knx-report/tests/`, for the same reason
//! `csv_roundtrip.rs` does: `check-layering` walks dev-dependency edges too
//! (`xtask/src/layering.rs:94-98`), so a `knx-etsproj` dev-dependency on
//! `knx-report` would trip `knx-report`'s own layering rule (it may depend
//! on `knx-core` and `knx-projection` only). `knx-app` is deliberately the
//! one crate that sees both sides, so its `tests/` is where a test that
//! needs both `knx-etsproj` and `knx-report` belongs.
//!
//! `knx-app`'s tests cannot reach `knx-etsproj`'s `tests/support` module —
//! a crate's integration tests are private to it — so this uses
//! `knx-testsupport`, the shared dev-dependency every crate's tests can
//! reach, instead.
//!
//! Gated by the standard `corpus_available()` pattern: `OriginalData/` is
//! the maintainer's own real KNX installation, gitignored and local-only,
//! so CI (and any contributor without a copy) skips this test rather than
//! failing it.

use std::path::PathBuf;

use chrono::{TimeZone, Utc};
use knx_report::{html::escape_text, render_html, ReportOptions};

fn reference_ets4_path() -> PathBuf {
    knx_testsupport::reference_ets4_path()
}

#[test]
fn rendering_the_reference_project_produces_a_complete_self_contained_document() {
    if !reference_ets4_path().exists() {
        eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
        return;
    }

    let outcome = knx_etsproj::import_knxproj(&reference_ets4_path())
        .expect("the reference project is a known-good fixture");
    let project = outcome.project;

    // Proof the corpus path actually ran, not the skip path: a project
    // with real substance to render.
    let device_count = project.devices.iter().count();
    let com_object_count = project.devices.com_objects().count();
    let group_address_count: usize = project
        .installations
        .iter()
        .map(|i| i.group_addresses.len())
        .sum();
    assert!(
        device_count > 0 && group_address_count > 0,
        "expected the reference project to contain real devices and group addresses; \
         got {device_count} devices and {group_address_count} group addresses — \
         did the skip guard actually trigger?"
    );
    eprintln!(
        "documentation_export corpus test: {device_count} devices, \
         {com_object_count} communication objects, {group_address_count} group addresses"
    );

    let options = ReportOptions::new(Utc.with_ymd_and_hms(2026, 9, 10, 12, 0, 0).unwrap());
    let report = render_html(&project, &options);
    eprintln!(
        "documentation_export corpus test: rendered {} bytes of HTML, {} structural warnings",
        report.html.len(),
        report.warnings.len()
    );

    // Every group address's formatted string appears in the output.
    let style = project.info.group_address_style;
    for installation in &project.installations {
        for entry in &installation.group_addresses {
            let formatted = entry.address.format(style);
            assert!(
                report.html.contains(&formatted),
                "missing formatted group address {formatted} (entry {})",
                entry.id
            );
        }
    }

    // Every device name appears in the output (escaped, the same way the
    // renderer writes it).
    for device in project.devices.iter() {
        let escaped = escape_text(&device.name);
        assert!(
            report.html.contains(&escaped),
            "missing device name {:?} (device {})",
            device.name,
            device.id
        );
    }

    // <table>/</table> and <tr ...>/</tr> counts balance. Rows may carry
    // classes, so counting only the literal `<tr>` spelling would mistake
    // valid attributed rows for missing opening tags.
    let table_open = report.html.matches("<table").count();
    let table_close = report.html.matches("</table>").count();
    assert_eq!(
        table_open, table_close,
        "expected <table> and </table> counts to match"
    );
    let tr_open = report.html.matches("<tr").count();
    let tr_close = report.html.matches("</tr>").count();
    assert_eq!(tr_open, tr_close, "expected <tr> and </tr> counts to match");
    assert!(table_open > 0 && tr_open > 0, "expected at least one table");

    // The Summary section's stated counts equal counts computed straight
    // from the Project — the same computation `knx-report`'s own
    // `compute_counts` does, re-derived independently here so the test
    // does not simply assert the renderer agrees with itself.
    let installations = project.installations.len();
    let areas: usize = project
        .installations
        .iter()
        .map(|i| i.topology.areas.len())
        .sum();
    let lines: usize = project
        .installations
        .iter()
        .map(|i| i.topology.lines.len())
        .sum();
    let building_parts: usize = project
        .installations
        .iter()
        .map(|i| i.buildings.len())
        .sum();
    let group_ranges: usize = project
        .installations
        .iter()
        .map(|i| i.group_ranges.len())
        .sum();
    let parameter_values: usize = project
        .installations
        .iter()
        .map(|i| i.parameters.len())
        .sum();

    let summary_rows: [(&str, usize); 9] = [
        ("Installations", installations),
        ("Areas", areas),
        ("Lines", lines),
        ("Devices", device_count),
        ("Communication objects", com_object_count),
        ("Group ranges", group_ranges),
        ("Group addresses", group_address_count),
        ("Building parts", building_parts),
        ("Parameter values", parameter_values),
    ];
    for (label, value) in summary_rows {
        let expected = format!("<tr><th>{label}</th><td>{value}</td></tr>");
        assert!(
            report.html.contains(&expected),
            "summary row for {label:?} not found with expected value {value}; expected fragment {expected:?}"
        );
    }

    // The document is self-contained: no script, no network references.
    assert!(!report.html.contains("<script"), "expected no <script tags");
    assert!(!report.html.contains("http://"), "expected no http:// URLs");
    assert!(
        !report.html.contains("https://"),
        "expected no https:// URLs"
    );

    // A second render with the same timestamp is byte-identical.
    let report_again = render_html(&project, &options);
    assert_eq!(
        report.html, report_again.html,
        "expected byte-identical output from a second render with the same timestamp"
    );
    assert_eq!(report.warnings.len(), report_again.warnings.len());
}
