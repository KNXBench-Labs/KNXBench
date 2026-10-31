//! Regresses decoded secret learning and bounded XML inspection.

use super::*;

#[test]
fn secret_free_xml_attribute_count_is_bounded_before_duplicate_materialization() {
    let base = xml::Base {
        text: "a:",
        root_in_base: false,
    };
    let attrs = (0..4096)
        .map(|i| format!(" a{i}=\"v\""))
        .collect::<String>();
    let at = format!("<a{attrs}/>");
    assert!(xml::redact(at.as_bytes(), &base, &mut Findings::default())
        .unwrap()
        .is_none());
    let over = format!("<a{attrs} extra=\"v\"/>");
    assert_eq!(
        xml::redact(over.as_bytes(), &base, &mut Findings::default()).unwrap_err(),
        "XML attribute count budget exceeded"
    );
}

fn decoded_sibling(xml: &str, secret: &str) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("input.knxproj");
    std::fs::write(
        &path,
        knx_testsupport::zip_with_entries(&[
            ("secrets.xml", xml.as_bytes()),
            ("notes.txt", secret.as_bytes()),
            ("safe.bin", b"unrelated bytes"),
        ]),
    )
    .unwrap();
    let copy = import_source_without_secrets(&path).unwrap();
    let out = members(&copy.bytes);
    assert_eq!(out["safe.bin"], b"unrelated bytes");
    assert!(
        !out.contains_key("notes.txt"),
        "decoded removed secret survived in sibling"
    );
    assert!(copy.withheld.iter().any(|w| w.starts_with("notes.txt:")));
}

#[test]
fn secret_free_review_r4_cdata_runs_learn_complete_value() {
    decoded_sibling(
        "<a><Password>abc<![CDATA[def]]>ghi</Password></a>",
        "abcdefghi",
    );
}

#[test]
fn secret_free_review_r4_references_learn_complete_value() {
    decoded_sibling(
        "<a><Password>ab&amp;cd&#x65;&#102;</Password></a>",
        "ab&cdef",
    );
}

#[test]
fn secret_free_review_r4_cdata_only_learns_decoded_value() {
    decoded_sibling(
        "<a><Password><![CDATA[abcdefghi]]></Password></a>",
        "abcdefghi",
    );
}

#[test]
fn secret_free_review_r7_deep_xml_refuses_before_path_materialization() {
    let base = xml::Base {
        text: "deep.xml:",
        root_in_base: false,
    };
    let at = format!("{}{}", "<a>".repeat(256), "</a>".repeat(256));
    assert!(xml::redact(at.as_bytes(), &base, &mut Findings::default())
        .unwrap()
        .is_none());
    let over = format!("{}{}", "<a>".repeat(257), "</a>".repeat(257));
    let result = xml::redact(over.as_bytes(), &base, &mut Findings::default());
    assert_eq!(result.unwrap_err(), "XML depth budget exceeded");
}

#[test]
fn secret_free_xml_exact_path_boundary_is_checked_before_copy() {
    let base = xml::Base {
        text: "a:",
        root_in_base: false,
    };
    let at = format!("<{} />", "a".repeat(8189));
    assert!(xml::redact(at.as_bytes(), &base, &mut Findings::default())
        .unwrap()
        .is_none());
    let over = format!("<{} />", "a".repeat(8190));
    assert_eq!(
        xml::redact(over.as_bytes(), &base, &mut Findings::default()).unwrap_err(),
        "XML path budget exceeded"
    );
}

#[test]
fn secret_free_xml_undecodable_secret_forms_are_withheld() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("input.knxproj");
    std::fs::write(
        &path,
        knx_testsupport::zip_with_entries(&[
            ("attribute.xml", br#"<a Password="&unknown;"/>"#),
            ("element.xml", b"<Password>ab&unknown;cd</Password>"),
            ("safe.bin", b"preserve"),
        ]),
    )
    .unwrap();
    let copy = import_source_without_secrets(&path).unwrap();
    let out = members(&copy.bytes);
    assert_eq!(out.len(), 1);
    assert_eq!(out["safe.bin"], b"preserve");
    assert_eq!(copy.withheld.len(), 2);
}
