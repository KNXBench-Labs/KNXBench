//! Verifies normalized instruction integrity with synthetic data.
use knx_etsproj::instructions::{
    read_installation_hints as read, InstructionReadError as E, MAX_XML_BYTES, MAX_XML_DEPTH,
    MAX_XML_EVENTS,
};
use knx_etsproj::rtf::TextError;

fn doc(devices: &str) -> String {
    format!(
        r#"<KNX xmlns="http://knx.org/xml/project/23"><Project Id="P-0001"><Installations><Installation InstallationId="0"><Topology><UnassignedDevices>{devices}</UnassignedDevices></Topology></Installation></Installations></Project></KNX>"#
    )
}
fn device(id: &str, attrs: &str) -> String {
    format!(r#"<DeviceInstance Id="{id}" {attrs}/>"#)
}

#[test]
fn selects_the_exact_source_device_not_the_first_retained_attribute() {
    let xml = doc(&(device("first", "InstallationHints=\"not this\"")
        + &device("target", "InstallationHints=\"right &amp; Grüße\"")));
    let view = read(xml.as_bytes(), "P-0001/0.xml", "target")
        .unwrap()
        .unwrap();
    assert_eq!(view.original, "right & Grüße");
    assert_eq!(view.preview.unwrap().plain_text(), "right & Grüße");
}
#[test]
fn absent_and_explicitly_empty_are_different() {
    let xml = doc(&(device("absent", "") + &device("empty", "InstallationHints=\"\"")));
    assert_eq!(read(xml.as_bytes(), "P-0001/0.xml", "absent"), Ok(None));
    assert_eq!(
        read(xml.as_bytes(), "P-0001/0.xml", "empty")
            .unwrap()
            .unwrap()
            .original,
        ""
    );
}
#[test]
fn supported_and_refused_rtf_keep_the_exact_original() {
    for (value, malformed) in [(r"{\rtf1 a\b b}", false), (r"{\rtf1 unfinished", true)] {
        let xml = doc(&device("target", &format!("InstallationHints=\"{value}\"")));
        let original_xml = xml.clone();
        let view = read(xml.as_bytes(), "P-0001/0.xml", "target")
            .unwrap()
            .unwrap();
        assert_eq!(view.original, value);
        if malformed {
            assert_eq!(view.preview, Err(TextError::Malformed));
        } else {
            assert_eq!(view.preview.unwrap().plain_text(), "ab");
        }
        assert_eq!(xml, original_xml);
    }
}
#[test]
fn unknown_rtf_is_reported_without_rewriting_or_fetching_resources() {
    let xml = doc(&device(
        "target",
        r#"InstallationHints="{\rtf1 a{\*\unknown https://invalid.example/image}b}""#,
    ));
    let view = read(xml.as_bytes(), "P-0001/0.xml", "target")
        .unwrap()
        .unwrap();
    let preview = view.preview.unwrap();
    assert_eq!(preview.plain_text(), "ab");
    assert_eq!(preview.unsupported, vec!["unknown"]);
    assert!(view.original.contains("https://invalid.example/image"));
}
#[test]
fn duplicate_source_owners_and_duplicate_fields_refuse() {
    let xml = doc(&(device("target", "InstallationHints=\"first\"")
        + &device("target", "InstallationHints=\"second\"")));
    assert_eq!(
        read(xml.as_bytes(), "P-0001/0.xml", "target"),
        Err(E::AmbiguousDevice)
    );
    let xml = doc(&device(
        "target",
        "InstallationHints=\"a\" InstallationHints=\"b\"",
    ));
    assert!(read(xml.as_bytes(), "P-0001/0.xml", "target").is_err());
}
#[test]
fn missing_owner_is_not_an_absent_instruction() {
    assert_eq!(
        read(
            doc(&device("other", "")).as_bytes(),
            "P-0001/0.xml",
            "target"
        ),
        Err(E::MissingDevice)
    );
}
#[test]
fn foreign_namespace_older_schema_and_child_rebinding_refuse() {
    let xml = doc(&device("target", "InstallationHints=\"hello\""));
    for value in [
        xml.replace("project/23", "project/21"),
        xml.replace(
            "http://knx.org/xml/project/23",
            "https://example.invalid/23",
        ),
        xml.replace(
            "<DeviceInstance ",
            "<DeviceInstance xmlns=\"http://other.invalid\" ",
        ),
    ] {
        assert_eq!(
            read(value.as_bytes(), "P-0001/0.xml", "target"),
            Err(E::UnsupportedNamespace)
        );
    }
}
#[test]
fn malformed_entities_document_type_and_extra_roots_refuse() {
    let xml = doc(&device("target", "InstallationHints=\"hello\""));
    for value in [
        xml.replace("hello", "&unknown;"),
        xml.replace("hello", "&#0;"),
        format!("{xml}{xml}"),
        format!("<!DOCTYPE KNX SYSTEM 'file:///never-read'>{xml}"),
        xml.trim_end_matches("</KNX>").to_string(),
        format!("text{xml}"),
    ] {
        assert!(
            read(value.as_bytes(), "P-0001/0.xml", "target").is_err(),
            "malformed XML must not gain a view"
        );
    }
}
#[test]
fn assigned_and_segment_source_devices_use_the_same_owner_identity() {
    let xml = doc(&device("target", "InstallationHints=\"segment hint\""))
        .replace(
            "<UnassignedDevices>",
            "<Area Id=\"A\" Address=\"1\"><Line Id=\"L\" Address=\"1\"><Segment Id=\"S\">",
        )
        .replace("</UnassignedDevices>", "</Segment></Line></Area>");
    assert_eq!(
        read(xml.as_bytes(), "P-0001/0.xml", "target")
            .unwrap()
            .unwrap()
            .original,
        "segment hint"
    );
}
#[test]
fn xml_byte_event_and_depth_budgets_are_explicit() {
    assert_eq!(
        read(&vec![b' '; MAX_XML_BYTES + 1], "P-0001/0.xml", "target"),
        Err(E::SourceLimit)
    );
    let xml = format!(
        "<KNX xmlns=\"http://knx.org/xml/project/23\">{}</KNX>",
        "<X/>".repeat(MAX_XML_EVENTS)
    );
    assert_eq!(
        read(xml.as_bytes(), "P-0001/0.xml", "target"),
        Err(E::EventLimit)
    );
    let xml = format!(
        "<KNX xmlns=\"http://knx.org/xml/project/23\">{}{}</KNX>",
        "<X>".repeat(MAX_XML_DEPTH),
        "</X>".repeat(MAX_XML_DEPTH)
    );
    assert_eq!(
        read(xml.as_bytes(), "P-0001/0.xml", "target"),
        Err(E::DepthLimit)
    );
}

#[test]
fn invalid_body_references_and_characters_do_not_gain_an_authoritative_view() {
    let xml = doc(&device("target", "InstallationHints=\"ok\""));
    for text in ["&unknown;", "&#0;", "&#xD800;", "\u{1}"] {
        let bad = xml.replace("</Project>", &format!("<Text>{text}</Text></Project>"));
        assert_eq!(
            read(bad.as_bytes(), "P-0001/0.xml", "target"),
            Err(E::Malformed)
        );
    }
}
#[test]
fn misplaced_duplicate_or_non_xml10_declarations_refuse() {
    let xml = doc(&device("target", "InstallationHints=\"ok\""));
    for bad in [
        format!("{xml}<?xml version=\"1.0\"?>"),
        format!("<?xml version=\"1.0\"?><?xml version=\"1.0\"?>{xml}"),
        format!("<?xml version=\"1.1\"?>{xml}"),
        xml.replace("<Project ", "<?xml version=\"1.0\"?><Project "),
    ] {
        assert_eq!(
            read(bad.as_bytes(), "P-0001/0.xml", "target"),
            Err(E::Malformed)
        );
    }
}
#[test]
fn reserved_namespace_bindings_and_invalid_element_names_refuse() {
    let xml = doc(&device("target", "InstallationHints=\"ok\""));
    for bad in [
        xml.replace("<KNX ", "<KNX xmlns:xml=\"http://wrong.invalid\" "),
        xml.replace("<KNX ", "<KNX xmlns:xmlns=\"http://wrong.invalid\" "),
        xml.replace(
            "<Project ",
            "<Project xmlns:p=\"http://www.w3.org/XML/1998/namespace\" ",
        ),
        xml.replace("</Project>", "<1invalid/></Project>"),
    ] {
        assert!(read(bad.as_bytes(), "P-0001/0.xml", "target").is_err());
    }
}
#[test]
fn valid_xml10_utf8_declaration_entities_and_body_unicode_remain_admitted() {
    let xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>{}",
        doc(&device(
            "target",
            "InstallationHints=\"&#x1F600; &amp; &#252;\""
        ))
    )
    .replace("</Project>", "<Text>Grüße &amp; &#x1F600;</Text></Project>");
    assert_eq!(
        read(xml.as_bytes(), "P-0001/0.xml", "target")
            .unwrap()
            .unwrap()
            .original,
        "😀 & ü"
    );
}
