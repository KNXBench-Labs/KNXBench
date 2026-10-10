//! KL-106 audit: MAC redaction and retained ETS source evidence in the debug bundle.
//!
//! Every planted value is synthetic: documentation address ranges
//! (RFC 5737), locally administered MAC addresses and `.invalid` host
//! names (`knx_testsupport::retained_privacy_knxproj_bytes`).

use super::*;

fn plain() -> Redactor {
    Redactor::new(None, None)
}

#[test]
fn colon_and_hyphen_mac_addresses_are_redacted() {
    let r = plain();
    for (input, expected) in [
        (
            "iface 02:00:5e:10:00:01 up",
            format!("iface {MAC_PLACEHOLDER} up"),
        ),
        ("02-00-5E-10-00-02", MAC_PLACEHOLDER.to_string()),
        ("mac=0A:1B:2C:3D:4E:5F.", format!("mac={MAC_PLACEHOLDER}.")),
        // More than six pairs is still a hardware address (EUI-64), and
        // keeping the tail would leak most of it.
        ("02-00-5E-FF-FE-10-00-01", MAC_PLACEHOLDER.to_string()),
        ("02:00:5e:ff:fe:10:00", MAC_PLACEHOLDER.to_string()),
        // Eight colon pairs are also a valid IPv6 literal, and the IPv6
        // pass (which runs first) takes them: redacted either way.
        ("02:00:5e:ff:fe:10:00:01", IPV6_PLACEHOLDER.to_string()),
    ] {
        assert_eq!(r.apply(input), expected, "input {input:?}");
    }
}

#[test]
fn shapes_that_are_not_mac_addresses_survive() {
    let r = plain();
    for keep in [
        // Five pairs, a time of day, a date, a UUID, a KNX group address,
        // a serial-number shape, a SHA-256 prefix and mixed separators.
        "02:00:5e:10:00",
        "12:30:45",
        "2026-10-10",
        "123e4567-e89b-12d3-a456-426614174000",
        "1/2/3",
        "00FA:10203040",
        "9f86d081884c7d659a2feaa0c55ad015",
        "02:00-5e:10-00:01",
        // Glued to a word on either side: an identifier, not an address.
        "x02:00:5e:10:00:01",
        "02:00:5e:10:00:01g",
    ] {
        assert_eq!(r.apply(keep), keep, "must survive: {keep:?}");
    }
}

#[test]
fn a_mac_inside_an_ipv6_literal_is_left_to_the_ipv6_pass() {
    // The IPv6 pass runs first, so the whole literal goes, not just a tail.
    assert_eq!(
        plain().apply("peer fe80::1a:2b:3c:4d:5e:6f"),
        format!("peer {IPV6_PLACEHOLDER}")
    );
}

#[test]
fn the_report_names_mac_as_redacted_and_user_names_as_kept() {
    let md = report_markdown(
        &BundleInput {
            description: String::new(),
            app_version: None,
            server_version: "knx-server 0.0.0".into(),
            shell: None,
            ui_language: None,
            theme: None,
            project_open: false,
            log: None,
            project_summary: None,
            bus_telegrams: None,
        },
        &[REPORT_MD, ENVIRONMENT_JSON],
    );
    let redacted = md
        .split("Nothing else is replaced")
        .next()
        .expect("the privacy paragraph");
    assert!(redacted.contains("MAC addresses"), "{md}");
    let kept = md
        .split("Nothing else is replaced")
        .nth(1)
        .expect("the kept list");
    assert!(kept.contains("user names"), "{md}");
    assert!(!kept.contains("MAC addresses"), "{md}");
    assert!(md.contains("retained ETS source"), "{md}");
}

use knx_testsupport::{
    retained_privacy_knxproj_bytes, RETAINED_PRIVACY_PLANTED, RETAINED_PRIVACY_UNKNOWN_MAC,
    RETAINED_PRIVACY_UNKNOWN_OWNER,
};

/// The retained subtrees are kept (data integrity) but never reach the
/// bundle; the one unknown-attribute MAC that does reach `log.json` is
/// redacted, and the unknown-attribute user name stays as the report says.
#[test]
fn retained_endpoints_macs_and_trace_user_names_never_reach_the_bundle() {
    let outcome =
        knx_etsproj::import_knxproj_bytes(retained_privacy_knxproj_bytes(), "synthetic.knxproj")
            .expect("the synthetic witness imports");

    // Data integrity first: every planted subtree is still retained.
    let retained: Vec<u8> = outcome
        .opaque
        .iter()
        .flat_map(|e| e.bytes.iter().copied())
        .collect();
    let retained = String::from_utf8_lossy(&retained);
    for planted in RETAINED_PRIVACY_PLANTED {
        assert!(retained.contains(planted), "{planted} must stay retained");
    }

    let log = serde_json::to_value(crate::session_log::from_import_report(&outcome.report))
        .expect("log entries serialize");
    let bundle = build_bundle(
        &BundleInput {
            description: "import looked odd".into(),
            app_version: None,
            server_version: "knx-server 0.0.0".into(),
            shell: None,
            ui_language: None,
            theme: None,
            project_open: true,
            log: Some(log),
            project_summary: None,
            bus_telegrams: None,
        },
        &plain(),
    );
    let text = |name: &str| {
        let file = bundle.files.iter().find(|f| f.name == name).expect(name);
        String::from_utf8(file.bytes.clone()).expect("utf-8")
    };
    for name in [REPORT_MD, ENVIRONMENT_JSON, LOG_JSON] {
        let body = text(name);
        for planted in RETAINED_PRIVACY_PLANTED
            .iter()
            .chain([&RETAINED_PRIVACY_UNKNOWN_MAC])
        {
            assert!(!body.contains(planted), "{planted} leaked into {name}");
        }
    }
    let log = text(LOG_JSON);
    assert!(
        log.contains(MAC_PLACEHOLDER),
        "the unknown attribute's sample reaches the log redacted, not dropped: {log}"
    );
    assert!(log.contains(RETAINED_PRIVACY_UNKNOWN_OWNER), "{log}");
    assert!(text(REPORT_MD).contains("user names"));
}
