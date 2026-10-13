//! Content evidence for retained payloads, separate from unverified path roles.
//!
//! Reuses the product adapter's byte sniffing. No nested archive reader,
//! extraction, execution, schema change or authenticated ETS-App claim.

use knx_etsproj::opaque::{ManufacturerFile, OpaqueEntry, OpaqueKind};
use knx_etsproj::report::UnsupportedFeature;
use knx_etsproj::ImportReport;

/// Ends every per-file content-evidence notice [`annotate_member`] writes.
const RETAINED_SUFFIX: &str =
    "retained byte-exact, uninterpreted; not rendered or executed; contents not unpacked";
/// The parser's generic per-file notice, after its `"{path}: "` prefix.
const GENERIC_NOTICE: &str = "opaque manufacturer payload; preserved, not interpreted or executed";

/// The shared, path-free wording of a per-file "retained payload" notice
/// (an unsupported-capability row's `what`/`consequence`), or `None` for
/// every other unsupported-capability row.
///
/// Presentation layers (the session log) group these notices by this
/// label so one project with hundreds of retained manufacturer files does
/// not turn into hundreds of identical lines; the report itself keeps one
/// row per file. Recognition lives next to the writer so the two cannot
/// drift apart silently (a unit test pins both).
pub fn retained_payload_label<'a>(what: &str, consequence: &'a str) -> Option<&'a str> {
    if consequence.ends_with(RETAINED_SUFFIX) {
        return Some(consequence);
    }
    consequence
        .strip_prefix(what)
        .and_then(|rest| rest.strip_prefix(": "))
        .filter(|rest| *rest == GENERIC_NOTICE)
}

pub(crate) fn annotate(
    report: &mut ImportReport,
    opaque: &[OpaqueEntry],
    manufacturer: &[ManufacturerFile],
) {
    for entry in opaque.iter().filter(|entry| entry.xpath.is_empty()) {
        annotate_member(report, &entry.source_path, &entry.bytes, entry.kind);
    }
    for file in manufacturer {
        annotate_member(report, &file.source_path, &file.bytes, file.kind);
    }
}

fn annotate_member(report: &mut ImportReport, path: &str, bytes: &[u8], kind: OpaqueKind) {
    let lower = path.to_ascii_lowercase();
    let app_named = lower.ends_with(".etsapp");
    let addin_located = lower.contains("/addindata/");
    let user_file = lower.contains("/userfiles/");
    if kind != OpaqueKind::Baggage && !app_named && !addin_located && !user_file {
        return;
    }
    let mut reason = String::new();
    if app_named {
        reason.push_str("ETS-app-named candidate (identity unverified); ");
    }
    if addin_located {
        reason.push_str("add-in-state candidate (semantics unverified); ");
    }
    let owner = if matches!(kind, OpaqueKind::Baggage | OpaqueKind::ManufacturerData) {
        "manufacturer"
    } else {
        "project"
    };
    let class = knx_productdb::sniff_media(bytes).as_str();
    reason.push_str(&format!("{owner} payload ({class}); {RETAINED_SUFFIX}"));
    for summary in &mut report.opaque {
        if summary.source_path == path && summary.xpath.is_empty() {
            summary.reason.clone_from(&reason);
        }
    }
    // Replace only the parser's generic opaque-payload notice. Other
    // same-member capability/validation diagnostics remain independent.
    let generic = format!("{path}: {GENERIC_NOTICE}");
    let mut matched = false;
    for feature in &mut report.unsupported {
        if feature.what == path {
            if feature.consequence == generic {
                feature.consequence.clone_from(&reason);
                matched = true;
            } else if feature.consequence == reason {
                matched = true;
            }
        }
    }
    if !matched {
        report.unsupported.push(UnsupportedFeature {
            what: path.to_owned(),
            consequence: reason,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROJECT_XML: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/23"><Project Id="P-TEST"><Installations><Installation InstallationId="0"><Topology/><GroupAddresses/></Installation></Installations></Project></KNX>"#;

    #[test]
    fn both_writers_of_per_file_payload_notices_are_recognised_and_nothing_else() {
        let baggage = "M-0008/Baggages/help.zip";
        let outcome = knx_etsproj::import_knxproj_bytes(
            knx_testsupport::zip_with_entries(&[
                ("P-TEST.signature", b""),
                ("P-TEST/0.xml", PROJECT_XML),
                ("P-TEST/project.xml", PROJECT_XML),
                (baggage, b"PK\x05\x06synthetic"),
            ]),
            "synthetic.knxproj",
        )
        .unwrap();
        let mut report = outcome.report;
        // The parser's own wording, before content evidence replaces it.
        let generic = report
            .unsupported
            .iter()
            .find(|row| row.what == baggage)
            .expect("the parser names every baggage file");
        assert_eq!(
            retained_payload_label(&generic.what, &generic.consequence),
            Some(GENERIC_NOTICE)
        );

        annotate(&mut report, &outcome.opaque, &outcome.manufacturer);
        let annotated = report
            .unsupported
            .iter()
            .find(|row| row.what == baggage)
            .unwrap();
        assert_eq!(
            retained_payload_label(&annotated.what, &annotated.consequence),
            Some("manufacturer payload (zip); retained byte-exact, uninterpreted; not rendered or executed; contents not unpacked")
        );
        for row in report.unsupported.iter().filter(|row| row.what != baggage) {
            assert_eq!(
                retained_payload_label(&row.what, &row.consequence),
                None,
                "{row:?}"
            );
        }
        let foreign = UnsupportedFeature {
            what: baggage.into(),
            consequence: "synthetic independent capability refusal".into(),
        };
        assert_eq!(
            retained_payload_label(&foreign.what, &foreign.consequence),
            None
        );
        // The generic wording only counts for its own path.
        let misattributed = UnsupportedFeature {
            what: "other".into(),
            consequence: format!("{baggage}: {GENERIC_NOTICE}"),
        };
        assert_eq!(
            retained_payload_label(&misattributed.what, &misattributed.consequence),
            None
        );
    }

    #[test]
    fn independent_capability_diagnostics_are_not_replaced_by_content_evidence() {
        let xml = br#"<KNX xmlns="http://knx.org/xml/project/23"><Project Id="P-TEST"><Installations><Installation InstallationId="0"><Topology/><GroupAddresses/></Installation></Installations></Project></KNX>"#;
        let mut report = knx_etsproj::import_knxproj_bytes(
            knx_testsupport::zip_with_entries(&[
                ("P-TEST.signature", b""),
                ("P-TEST/0.xml", xml),
                ("P-TEST/project.xml", xml),
            ]),
            "synthetic.knxproj",
        )
        .unwrap()
        .report;
        let path = "P-TEST/UserFiles/unknown.dat";
        report.unsupported.push(UnsupportedFeature {
            what: path.into(),
            consequence: "synthetic independent capability refusal".into(),
        });
        annotate_member(&mut report, path, b"synthetic", OpaqueKind::ContainerEntry);
        assert!(report
            .unsupported
            .iter()
            .any(|row| row.what == path
                && row.consequence == "synthetic independent capability refusal"));
        annotate_member(&mut report, path, b"synthetic", OpaqueKind::ContainerEntry);
        assert_eq!(
            report
                .unsupported
                .iter()
                .filter(|row| row.what == path && row.consequence.contains("payload (unknown)"))
                .count(),
            1
        );
    }
}
