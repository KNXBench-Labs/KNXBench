//! The fictional demo module: golden export bytes, structural findings, rules and refusals.

use crate::{demo, demo_doc, fixture};
use knx_devconf::{crc16_ccitt_false, ExportError, FindingKind};
use serde_json::{json, Value};

fn findings(doc: &[u8]) -> Vec<(FindingKind, String, Option<String>)> {
    demo()
        .validate(doc)
        .expect("evaluates")
        .findings
        .into_iter()
        .map(|f| (f.kind, f.path, f.rule))
        .collect()
}

#[test]
fn valid_document_exports_the_hand_computed_bytes() {
    let module = demo();
    assert_eq!(
        (module.id(), module.version()),
        ("example.demo-panel", "1.0.0")
    );
    assert_eq!(module.sha256().len(), 64);
    let doc = fixture("demo-panel.valid.json");
    assert!(module.validate(&doc).unwrap().is_valid());
    let bytes = module.export(&doc, "image").expect("exports");
    #[rustfmt::skip]
    let mut expected = vec![
        b'D', b'E', b'M', b'O', 1, 2, 3, 0, 65, 0, 0, 0, // header, length 65, CRC later
        0xfe, 0xff,                                     // offset -2
        b'H', b'a', b'l', b'l', 0, 0, 0, 0,             // name, zero padded
        1, 0, b'H', b'o', b'm', b'e', 0, 0, 0, 0,       // page 1
        2, 1, b'L', b'i', b'g', b'h', b't', b's', 0, 0, // page 2
        10, 2, 60, 0, 0,                                // dim_up = ordinal 1 + 1
        11, 3, 61, 0, 0,                                // dim_down
        12, 5, 62, 1, 255,                              // literal with payload
        10, 1, 11, 0, 12, 255, 0, 255,                  // buttons
    ];
    assert_eq!(expected.len(), 65);
    let crc = crc16_ccitt_false(&expected).to_le_bytes();
    expected[10..12].copy_from_slice(&crc);
    assert_eq!(bytes, expected);
}

#[test]
fn member_order_does_not_change_the_bytes_but_row_order_does() {
    let module = demo();
    let reference = module
        .export(&fixture("demo-panel.valid.json"), "image")
        .unwrap();
    let text = String::from_utf8(fixture("demo-panel.valid.json")).unwrap();
    let reordered = text.replace(
        r#"{ "id": 10, "kind": "dim_up", "object": 60, "payload": [0, 0] }"#,
        r#"{ "payload": [0, 0], "object": 60, "kind": "dim_up", "id": 10 }"#,
    );
    assert_ne!(reordered, text, "fixture edit applied");
    assert_eq!(
        module.export(reordered.as_bytes(), "image").unwrap(),
        reference
    );
    let swapped = demo_doc(|d| d["pages"].as_array_mut().unwrap().swap(0, 1));
    let bytes = module.export(&swapped, "image").unwrap();
    assert_ne!(bytes, reference);
    assert_eq!(&bytes[22..24], &[2, 1], "page 2 now comes first");
}

#[test]
fn structural_problems_are_all_reported_with_paths() {
    let doc = demo_doc(|d| {
        d["extra"] = json!(true);
        d["features"] = json!({ "night_mode": "supported" });
        d["offset"] = json!(101);
        d["name"] = json!("");
        d["pages"][1]["label"] = json!("Lights!!");
        d["pages"][1]["parent"] = json!(7);
        d["actions"][0]["kind"] = json!("blink");
        d["actions"][1]["payload"] = json!([0]);
        d["actions"][2]["id"] = json!(10);
        d["buttons"].as_array_mut().unwrap().pop();
        d["buttons"][0]["partner"] = json!("1");
    });
    let paths: Vec<String> = findings(&doc)
        .into_iter()
        .inspect(|(kind, _, rule)| {
            assert_eq!((*kind, rule.as_deref()), (FindingKind::Structure, None))
        })
        .map(|(_, path, _)| path)
        .collect();
    for expected in [
        "",
        "/features",
        "/offset",
        "/name",
        "/pages/1/label",
        "/pages/1/parent",
        "/actions/0/kind",
        "/actions/1/payload",
        "/actions/2/id",
        "/buttons",
        "/buttons/0/partner",
    ] {
        assert!(
            paths.iter().any(|p| p == expected),
            "no finding at {expected:?} in {paths:?}"
        );
    }
}

#[test]
fn strict_json_problems_are_document_findings() {
    for text in [
        &br#"{"version":1,"version":1}"#[..],
        b"{\"offset\": 1.0}",
        b"not json",
        b"",
    ] {
        let found = findings(text);
        assert_eq!(found.len(), 1, "{:?}", String::from_utf8_lossy(text));
        assert_eq!(found[0].1, "");
    }
    let oversized = vec![b' '; 65537];
    assert!(demo().validate(&oversized).unwrap().findings[0]
        .message
        .contains("limit is 65536"));
}

#[test]
fn rules_report_at_the_declared_field() {
    type Edit = Box<dyn Fn(&mut Value)>;
    let cases: Vec<(Edit, &str, &str)> = vec![
        (
            Box::new(|d| d["pages"][1]["parent"] = json!(0)),
            "one-root",
            "",
        ),
        (
            Box::new(|d| d["pages"][0]["parent"] = json!(2)),
            "page-depth",
            "/pages/0/parent",
        ),
        (
            Box::new(|d| d["actions"][0]["payload"] = json!([0, 1])),
            "payload-only-literal",
            "/actions/0/payload",
        ),
        (
            Box::new(|d| d["actions"][1]["object"] = json!(60)),
            "unique-object",
            "/actions/0/object",
        ),
        (
            Box::new(|d| d["buttons"][1]["partner"] = json!(255)),
            "partner-pair",
            "/buttons/0/partner",
        ),
        (
            Box::new(|d| d["buttons"][2]["partner"] = json!(9)),
            "partner-pair",
            "/buttons/2/partner",
        ),
        (
            Box::new(|d| d["actions"][1]["kind"] = json!("stop")),
            "dim-pair",
            "/buttons/0/action",
        ),
    ];
    for (edit, rule, path) in cases {
        let found = findings(&demo_doc(edit));
        assert!(
            found.iter().any(|(k, p, r)| *k == FindingKind::Rule
                && r.as_deref() == Some(rule)
                && p == path),
            "{rule} at {path:?} missing in {found:?}"
        );
    }
}

#[test]
fn page_cycles_and_deep_nesting_are_violations_not_hangs() {
    let cycle = demo_doc(|d| {
        d["pages"] = json!([
            { "id": 1, "parent": 0, "label": "Home" },
            { "id": 2, "parent": 3, "label": "A" },
            { "id": 3, "parent": 2, "label": "B" }]);
    });
    let found = findings(&cycle);
    assert_eq!(
        found
            .iter()
            .filter(|(_, _, r)| r.as_deref() == Some("page-depth"))
            .count(),
        2,
        "{found:?}"
    );
    let deep = demo_doc(|d| {
        d["pages"]
            .as_array_mut()
            .unwrap()
            .push(json!({ "id": 3, "parent": 2, "label": "Deep" }))
    });
    let found = findings(&deep);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].1, "/pages/2/parent");
}

#[test]
fn exports_refuse_values_that_do_not_fit_and_invalid_documents() {
    let module = demo();
    let long_name = demo_doc(|d| d["name"] = json!("Entrance hall"));
    assert!(
        module.validate(&long_name).unwrap().is_valid(),
        "valid for the module"
    );
    match module.export(&long_name, "image") {
        Err(ExportError::Refused { path, message }) => {
            assert_eq!(path, "");
            assert!(message.contains("13 bytes, 8 fit"), "{message}");
        }
        other => panic!("expected a refusal, got {other:?}"),
    }
    let invalid = demo_doc(|d| d["offset"] = json!(500));
    assert!(
        matches!(module.export(&invalid, "image"), Err(ExportError::Invalid(v)) if !v.is_valid())
    );
    let doc = fixture("demo-panel.valid.json");
    assert_eq!(
        module.export(&doc, "other"),
        Err(ExportError::UnknownExport("other".into()))
    );
}

#[test]
fn outline_lists_members_tables_and_rules() {
    let outline = demo().outline();
    let names: Vec<(&str, &str)> = outline
        .members
        .iter()
        .map(|m| (m.name.as_str(), m.kind))
        .collect();
    assert_eq!(
        names,
        [
            ("version", "const"),
            ("features", "const"),
            ("name", "text"),
            ("offset", "int"),
            ("pages", "table"),
            ("actions", "table"),
            ("buttons", "table")
        ]
    );
    assert_eq!(outline.members[2].title.as_deref(), Some("Panel name"));
    assert_eq!(outline.members[4].title.as_deref(), Some("Pages"));
    let buttons = &outline.tables[2];
    assert_eq!(
        (buttons.min_rows, buttons.max_rows, buttons.key.as_deref()),
        (4, 4, None)
    );
    assert_eq!(outline.tables[0].key.as_deref(), Some("id"));
    assert_eq!(outline.rules, 6);
}

#[test]
fn nul_terminated_text_needs_one_spare_byte_and_zero_padding_does_not() {
    let members = json!({ "label": { "type": "text", "charset": "printable-ascii", "min_bytes": 0, "max_bytes": 8 } });
    let layout = |pad: &str| {
        json!([{ "id": "e", "title": "E", "extension": "bin", "max_bytes": 8,
        "layout": [{ "text": { "field": "label" }, "size": 4, "pad": pad }] }])
    };
    let nul = crate::module(members.clone(), json!([]), layout("nul-terminated")).unwrap();
    assert_eq!(nul.export(br#"{"label":"abc"}"#, "e").unwrap(), b"abc\0");
    assert!(matches!(
        nul.export(br#"{"label":"abcd"}"#, "e"),
        Err(ExportError::Refused { .. })
    ));
    let zero = crate::module(members, json!([]), layout("zero")).unwrap();
    assert_eq!(zero.export(br#"{"label":"abcd"}"#, "e").unwrap(), b"abcd");
    assert_eq!(zero.export(br#"{"label":""}"#, "e").unwrap(), b"\0\0\0\0");
    assert!(matches!(
        zero.export(br#"{"label":"abcde"}"#, "e"),
        Err(ExportError::Refused { .. })
    ));
}
