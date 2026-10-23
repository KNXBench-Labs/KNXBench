//! Synthetic XML admission and replay tests for project-bound source closures.
use super::admission::*;
use super::*;
use knx_app::system2_validation_preparation::ProjectProductInput;

// Keep the original ingest and its actual owner witnesses: a descriptor/hash
// mismatch must not masquerade as refusal of a qualified canonical alias.
fn changed_hardware(
    namespace: u8,
    change: impl FnOnce(&str) -> String,
) -> (
    tempfile::TempDir,
    knx_productdb::Connection,
    knx_core::Project,
    PreparedBundle,
) {
    changed_source(namespace, 0, change)
}

fn changed_source(
    namespace: u8,
    index: usize,
    change: impl FnOnce(&str) -> String,
) -> (
    tempfile::TempDir,
    knx_productdb::Connection,
    knx_core::Project,
    PreparedBundle,
) {
    let original = source_bundle(namespace);
    let mut members = [source_member(&original, 0), source_member(&original, 1)];
    members[index] = change(&members[index]);
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    for (path, bytes) in [
        ("M-0000/Hardware.xml", members[0].as_bytes()),
        ("M-0000/Application.xml", members[1].as_bytes()),
    ] {
        assert!(matches!(
            knx_productdb::ingest_file(&conn, path, bytes).unwrap(),
            knx_productdb::IngestOutcome::Ingested { .. }
        ));
    }
    let mut bundle = with_member(original, index, members[index].as_bytes().to_vec());
    let record = bundle.project_preparation.as_mut().unwrap();
    let ProjectProductInput::RetainedSourceMembers { descriptor, .. } = &mut record.product_input
    else {
        panic!("source fixture variant")
    };
    let member = &mut descriptor.members[index];
    let kind = if index == 0 {
        knx_productdb::identity::IdentityKind::Hardware2Program
    } else {
        knx_productdb::identity::IdentityKind::ApplicationProgram
    };
    let owner = knx_productdb::identity::owned_source_identity(
        &conn,
        kind,
        &member.owner.logical_id,
        &member.source_sha256,
    )
    .unwrap();
    member.owner.element_digest = owner.digest;
    member.owner.occurrence = owner.occurrence;
    repin_closure(&mut bundle);
    (dir, conn, synthetic_project(), bundle)
}

fn source_boundary_outcomes(
    dir: &tempfile::TempDir,
    conn: &knx_productdb::Connection,
    project: &knx_core::Project,
    bundle: &PreparedBundle,
    namespace: u8,
) -> Vec<(&'static str, Result<(), String>)> {
    let record = bundle.project_preparation.as_ref().unwrap();
    let before = conn.total_changes();
    let project_before = project.clone();
    let discovery = knx_app::system2_validation_preparation::describe_project_source_closure(
        conn,
        project,
        knx_core::DeviceId(1),
        &format!("http://knx.org/xml/project/{namespace}"),
    )
    .map(|_| ())
    .map_err(|e| e.to_string());
    let preparation = knx_app::system2_validation_preparation::prepare_project_simulated_candidate(
        conn,
        project,
        knx_core::DeviceId(1),
        &record.profile_bytes,
    )
    .map(|_| ())
    .map_err(|e| e.to_string());
    let file = dir.path().join("candidate.json");
    std::fs::write(&file, serde_json::to_vec(bundle).unwrap()).unwrap();
    let outcomes = vec![
        ("discovery", discovery),
        ("preparation", preparation),
        ("plan", bundle.plan().map(|_| ()).map_err(|e| e.to_string())),
        (
            "read",
            app::read_bundle(&file)
                .map(|_| ())
                .map_err(|e| e.to_string()),
        ),
        (
            "replay",
            knx_app::system2_validation_preparation::replay_project_simulated_candidate(bundle)
                .map(|_| ())
                .map_err(|e| e.to_string()),
        ),
    ];
    assert_eq!(conn.total_changes(), before);
    assert_eq!(project, &project_before);
    outcomes
}

fn qualified_alias_refuses(namespace: u8, element: &str, attribute: &str) {
    let (dir, conn, project, bundle) = changed_hardware(namespace, |source| {
        let canonical = format!("<{element} {attribute}=");
        assert_eq!(source.matches(&canonical).count(), 1);
        source.replacen(
            &canonical,
            &format!("<{element} xmlns:p=\"urn:synthetic-foreign\" p:{attribute}="),
            1,
        )
    });
    let outcomes = source_boundary_outcomes(&dir, &conn, &project, &bundle, namespace);
    assert!(
        outcomes.iter().all(|(_, result)| result
            .as_ref()
            .is_err_and(|e| e.contains("qualified attribute"))),
        "foreign qualified {attribute} must not supply the canonical {element} attribute in ns{namespace}: {outcomes:?}"
    );
}

macro_rules! qualified_alias_test {
    ($name:ident, $namespace:literal, $element:literal, $attribute:literal) => {
        #[test]
        fn $name() {
            qualified_alias_refuses($namespace, $element, $attribute);
        }
    };
}
qualified_alias_test!(
    foreign_qualified_refid_refuses_ns20,
    20,
    "ApplicationProgramRef",
    "RefId"
);
qualified_alias_test!(
    foreign_qualified_refid_refuses_ns23,
    23,
    "ApplicationProgramRef",
    "RefId"
);
qualified_alias_test!(
    foreign_qualified_id_refuses_ns20,
    20,
    "Hardware2Program",
    "Id"
);
qualified_alias_test!(
    foreign_qualified_id_refuses_ns23,
    23,
    "Hardware2Program",
    "Id"
);

#[test]
fn namespace_declarations_cannot_supply_missing_canonical_bridge_or_owner() {
    let mut admitted = Vec::new();
    for namespace in [20, 23] {
        for (element, attribute) in [
            ("ApplicationProgramRef", "RefId"),
            ("Hardware2Program", "Id"),
        ] {
            let (dir, conn, project, bundle) = changed_hardware(namespace, |source| {
                let canonical = format!("<{element} {attribute}=");
                assert_eq!(source.matches(&canonical).count(), 1);
                source.replacen(&canonical, &format!("<{element} xmlns:{attribute}="), 1)
            });
            let outcomes = source_boundary_outcomes(&dir, &conn, &project, &bundle, namespace);
            if !outcomes.iter().all(|(_, result)| {
                result
                    .as_ref()
                    .is_err_and(|e| e.contains("namespace declaration alias"))
            }) {
                admitted.push((namespace, element, attribute, outcomes));
            }
        }
    }
    assert!(
        admitted.is_empty(),
        "namespace declarations must not supply missing canonical attributes: {admitted:?}"
    );
}

#[test]
fn namespace_declaration_aliases_cannot_supply_other_consumed_semantic_keys() {
    let mut admitted = Vec::new();
    for namespace in [20, 23] {
        for (index, canonical, alias) in [
            (0, "MediumTypes=\"MT-0\"", "xmlns:MediumTypes=\"MT-0\""),
            (0, "Name=\"Synthetic\"", "xmlns:Name=\"Synthetic\""),
            (0, "Text=\"Synthetic\"", "xmlns:Text=\"Synthetic\""),
            (0, "<Hardware Id=", "<Hardware xmlns:IsAccessory=\"1\" Id="),
            (
                0,
                "<Hardware2Program Id=",
                "<Hardware2Program xmlns:Hash=\"synthetic\" Id=",
            ),
            (1, "Value=\"4660\"", "xmlns:Value=\"4660\""),
            (1, "SizeInBit=\"16\"", "xmlns:SizeInBit=\"16\""),
            (1, "minInclusive=\"0\"", "xmlns:minInclusive=\"0\""),
            (1, "Offset=\"0\"", "xmlns:Offset=\"0\""),
            (1, "MaxEntries=\"6\"", "xmlns:MaxEntries=\"6\""),
            (1, "<when test=\"1\"", "<when xmlns:test=\"1\""),
            (1, "<Static>", "<Static xmlns:Id=\"synthetic-control\">"),
            (
                1,
                "<when test=\"1\"",
                "<when xmlns:ParamRefId=\"synthetic-control\" test=\"1\"",
            ),
        ] {
            let (dir, conn, project, bundle) = changed_source(namespace, index, |source| {
                assert!(source.contains(canonical), "semantic alias fixture token");
                source.replacen(canonical, alias, 1)
            });
            let outcomes = source_boundary_outcomes(&dir, &conn, &project, &bundle, namespace);
            if !outcomes.iter().all(|(_, result)| {
                result
                    .as_ref()
                    .is_err_and(|e| e.contains("namespace declaration alias"))
            }) {
                admitted.push((namespace, canonical, outcomes));
            }
        }
    }
    assert!(
        admitted.is_empty(),
        "semantic namespace aliases must refuse: {admitted:?}"
    );
}

#[test]
fn differing_namespace_aliases_cannot_override_present_canonical_hardware_attributes() {
    for namespace in [20, 23] {
        for (element, attribute, value) in [
            ("ApplicationProgramRef", "RefId", SYN_ID),
            ("Hardware2Program", "Id", "M-0000_H-1_HP-1"),
            ("Hardware2Program", "MediumTypes", "MT-0"),
            ("Hardware", "Name", "Synthetic"),
            ("Product", "Text", "Synthetic"),
        ] {
            for declaration_first in [true, false] {
                let (dir, conn, project, bundle) = changed_hardware(namespace, |source| {
                    let start = source.find(&format!("<{element} ")).unwrap();
                    let end = start + source[start..].find('>').unwrap();
                    let tag = &source[start..end];
                    let canonical = format!("{attribute}=\"{value}\"");
                    assert_eq!(tag.matches(&canonical).count(), 1);
                    let declaration = format!("xmlns:{attribute}=\"urn:synthetic:different\"");
                    let replacement = if declaration_first {
                        format!("{declaration} {canonical}")
                    } else {
                        format!("{canonical} {declaration}")
                    };
                    let mut changed = source.to_owned();
                    changed.replace_range(start..end, &tag.replace(&canonical, &replacement));
                    changed
                });
                let outcomes = source_boundary_outcomes(&dir, &conn, &project, &bundle, namespace);
                assert!(
                    outcomes.iter().all(|(_, result)| result.is_ok()),
                    "ordinary {element}/@{attribute} must win regardless of alias value/order: {outcomes:?}"
                );
            }
        }
    }
}

#[test]
fn ordinary_namespace_prefixes_on_semantic_elements_and_unused_key_prefixes_remain_valid() {
    for namespace in [20, 23] {
        let (dir, conn, project, bundle) = changed_hardware(namespace, |source| {
            source
                .replace(
                    "<ManufacturerData>",
                    "<ManufacturerData xmlns:RefId=\"urn:synthetic:unused\" xmlns:Id=\"urn:synthetic:unused-id\" xmlns:é=\"urn:synthetic:unicode\" étiquette=\"retained\">",
                )
                .replace(
                    "<Hardware2Program Id=",
                    "<Hardware2Program xmlns:p=\"urn:synthetic:ordinary\" xmlns:é=\"urn:synthetic:unicode\" xmlns:Value=\"urn:synthetic:unused-value\" Id=",
                )
                .replace("</KNX>", "<étiquette état=\"normal\"></étiquette></KNX>")
        });
        let outcomes = source_boundary_outcomes(&dir, &conn, &project, &bundle, namespace);
        assert!(
            outcomes.iter().all(|(_, result)| result.is_ok()),
            "{outcomes:?}"
        );
    }
}

#[test]
fn missing_unqualified_bridge_and_owner_attributes_do_not_discover_or_prepare() {
    for namespace in [20, 23] {
        for (element, attribute, value) in [
            ("ApplicationProgramRef", "RefId", SYN_ID),
            ("Hardware2Program", "Id", "M-0000_H-1_HP-1"),
        ] {
            let original = source_bundle(namespace);
            let token = format!("<{element} {attribute}=\"{value}\"");
            let source = source_member(&original, 0);
            assert_eq!(source.matches(&token).count(), 1);
            let hardware = source.replacen(&token, &format!("<{element}"), 1);
            let application = source_member(&original, 1);
            let dir = tempfile::tempdir().unwrap();
            let conn =
                knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
            for (path, bytes) in [
                ("M-0000/Hardware.xml", hardware.as_bytes()),
                ("M-0000/Application.xml", application.as_bytes()),
            ] {
                assert!(matches!(
                    knx_productdb::ingest_file(&conn, path, bytes).unwrap(),
                    knx_productdb::IngestOutcome::Ingested { .. }
                ));
            }
            let project = synthetic_project();
            let before = conn.total_changes();
            assert!(
                knx_app::system2_validation_preparation::describe_project_source_closure(
                    &conn,
                    &project,
                    knx_core::DeviceId(1),
                    &format!("http://knx.org/xml/project/{namespace}"),
                )
                .is_err(),
                "missing canonical {attribute} on {element} must not discover in ns{namespace}"
            );
            assert!(
                knx_app::system2_validation_preparation::prepare_project_simulated_candidate(
                    &conn,
                    &project,
                    knx_core::DeviceId(1),
                    &original.project_preparation.as_ref().unwrap().profile_bytes,
                )
                .is_err(),
                "missing canonical {attribute} on {element} must not prepare in ns{namespace}"
            );
            assert_eq!(conn.total_changes(), before);
        }
    }
}

#[test]
fn qualified_extension_attributes_are_explicitly_unsupported_even_with_canonical_fields() {
    for namespace in [20, 23] {
        for metadata in [
            "xmlns:p=\"urn:synthetic-foreign\" p:Note=\"metadata\"".to_string(),
            format!("xmlns:p=\"http://knx.org/xml/project/{namespace}\" p:RefId=\"ignored\""),
            "xml:lang=\"en\"".to_string(),
        ] {
            let (dir, conn, project, bundle) = changed_hardware(namespace, |source| {
                source.replace(
                    "<ManufacturerData>",
                    &format!("<ManufacturerData {metadata}>"),
                )
            });
            let outcomes = source_boundary_outcomes(&dir, &conn, &project, &bundle, namespace);
            assert!(
                outcomes.iter().all(|(_, result)| result
                    .as_ref()
                    .is_err_and(|e| e.contains("qualified attribute"))),
                "qualified metadata requires an explicit verified exception: {outcomes:?}"
            );
        }
    }
}

#[test]
fn namespace_declarations_require_xml10_ncname_prefixes() {
    for namespace in [20, 23] {
        for prefix in ["1illegal", "bad:prefix", "\u{300}illegal"] {
            let (dir, conn, project, bundle) = changed_hardware(namespace, |source| {
                source.replace(
                    "<ManufacturerData>",
                    &format!("<ManufacturerData xmlns:{prefix}=\"urn:synthetic-foreign\">"),
                )
            });
            let outcomes = source_boundary_outcomes(&dir, &conn, &project, &bundle, namespace);
            assert!(
                outcomes
                    .iter()
                    .all(|(_, result)| result.as_ref().is_err_and(|e| e.contains("XML name"))),
                "malformed namespace prefix must refuse: {outcomes:?}"
            );
        }
    }
}

#[test]
fn ordinary_namespace_declarations_and_unqualified_metadata_remain_valid() {
    for namespace in [20, 23] {
        let (dir, conn, project, bundle) = changed_hardware(namespace, |source| {
            source
                .replace(
                    "<ManufacturerData>",
                    "<ManufacturerData xmlns:p=\"urn:synthetic-foreign\" Metadata=\"retained\">",
                )
                .replace("</KNX>", "<Extra Note=\"retained\"/></KNX>")
        });
        let outcomes = source_boundary_outcomes(&dir, &conn, &project, &bundle, namespace);
        assert!(outcomes.iter().all(|(_, r)| r.is_ok()), "{outcomes:?}");
    }
}

fn malformed_names_refuse(namespace: u8, attribute: bool) {
    let mut accepted = Vec::new();
    for name in [
        "1illegal",
        "\u{b7}illegal",
        "\u{300}illegal",
        "\u{37e}illegal",
        "bad\u{d7}name",
        "bad\u{f0000}name",
    ] {
        let (dir, conn, project, bundle) = changed_hardware(namespace, |source| {
            if attribute {
                source.replace(
                    "<ManufacturerData>",
                    &format!("<ManufacturerData {name}=\"x\">"),
                )
            } else {
                source.replace("</KNX>", &format!("<{name}/></KNX>"))
            }
        });
        let outcomes = source_boundary_outcomes(&dir, &conn, &project, &bundle, namespace);
        if !outcomes
            .iter()
            .all(|(_, result)| result.as_ref().is_err_and(|e| e.contains("XML name")))
        {
            accepted.push((name, outcomes));
        }
    }
    assert!(
        accepted.is_empty(),
        "malformed XML {} names must refuse in ns{namespace}: {accepted:?}",
        if attribute { "attribute" } else { "element" }
    );
}

#[test]
fn malformed_element_names_refuse_ns20() {
    malformed_names_refuse(20, false);
}
#[test]
fn malformed_element_names_refuse_ns23() {
    malformed_names_refuse(23, false);
}
#[test]
fn malformed_attribute_names_refuse_ns20() {
    malformed_names_refuse(20, true);
}
#[test]
fn malformed_attribute_names_refuse_ns23() {
    malformed_names_refuse(23, true);
}

#[test]
fn xml10_unicode_names_and_namespace_prefixes_remain_valid() {
    // XML1.0 Fifth Edition NameStartChar range endpoints, not an ASCII or
    // Unicode-alphabetic approximation. NameChar-only characters follow '_'.
    let names = [
        "A",
        "Z",
        "_",
        "a",
        "z",
        "\u{c0}",
        "\u{d6}",
        "\u{d8}",
        "\u{f6}",
        "\u{f8}",
        "\u{2ff}",
        "\u{370}",
        "\u{37d}",
        "\u{37f}",
        "\u{1fff}",
        "\u{200c}",
        "\u{200d}",
        "\u{2070}",
        "\u{218f}",
        "\u{2c00}",
        "\u{2fef}",
        "\u{3001}",
        "\u{d7ff}",
        "\u{f900}",
        "\u{fdcf}",
        "\u{fdf0}",
        "\u{fffd}",
        "\u{10000}",
        "\u{effff}",
        "_-",
        "_.",
        "_0",
        "_9",
        "_\u{b7}",
        "_\u{300}",
        "_\u{36f}",
        "_\u{203f}",
        "_\u{2040}",
    ];
    let metadata: String = names
        .iter()
        .map(|name| format!("<{name} {name}=\"retained\"></{name}>"))
        .collect();
    for namespace in [20, 23] {
        let (dir, conn, project, bundle) = changed_hardware(namespace, |source| {
            source
                .replace(
                    "<ManufacturerData>",
                    "<ManufacturerData xmlns:é=\"urn:synthetic-foreign\">",
                )
                .replace("</KNX>", &format!("{metadata}</KNX>"))
        });
        let outcomes = source_boundary_outcomes(&dir, &conn, &project, &bundle, namespace);
        assert!(outcomes.iter().all(|(_, r)| r.is_ok()), "{outcomes:?}");
    }
}

#[test]
fn strict_xml_boundary_rejects_malformed_comments_declarations_and_text_delimiters() {
    let original = source_bundle(20);
    let source = source_member(&original, 0);
    assert!(with_member(
        original.clone(),
        0,
        format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>{source}").into_bytes()
    )
    .plan()
    .is_ok());
    for (case, bad) in [
        format!("<!-- bad -- comment -->{source}"),
        format!("<?xml encoding=\"UTF-8\"?>{source}"),
        format!("<?xml version=\"1.0\" extra=\"unrecognized\"?>{source}"),
        source.replace("<ManufacturerData>", "<ManufacturerData>]]>"),
    ]
    .into_iter()
    .enumerate()
    {
        assert!(
            with_member(original.clone(), 0, bad.into_bytes())
                .plan()
                .is_err(),
            "malformed XML case {case} must refuse"
        );
    }
}

pub(super) fn with_member(
    mut bundle: PreparedBundle,
    index: usize,
    bytes: Vec<u8>,
) -> PreparedBundle {
    let record = bundle.project_preparation.as_mut().unwrap();
    if let ProjectProductInput::RetainedSourceMembers {
        descriptor,
        member_bytes,
        ..
    } = &mut record.product_input
    {
        descriptor.members[index].byte_len = bytes.len() as u64;
        descriptor.members[index].source_sha256 = knx_productdb::sha256_hex(&bytes);
        member_bytes[index] = bytes;
        if index == 1 {
            record.source_sha256 = descriptor.members[1].source_sha256.clone();
        }
    }
    repin_closure(&mut bundle);
    bundle
}
pub(super) fn source_member(bundle: &PreparedBundle, index: usize) -> String {
    if let ProjectProductInput::RetainedSourceMembers { member_bytes, .. } =
        &bundle.project_preparation.as_ref().unwrap().product_input
    {
        String::from_utf8(member_bytes[index].clone()).unwrap()
    } else {
        panic!("source fixture variant")
    }
}
#[test]
fn rehashed_source_members_refuse_foreign_rebound_prefixed_and_malformed_documents() {
    for namespace in [20, 23] {
        let original = source_bundle(namespace);
        for index in [0, 1] {
            let source = source_member(&original, index);
            let ns = format!("http://knx.org/xml/project/{namespace}");
            for bad in [
                source.replacen(&ns, "urn:foreign", 1),
                source.replacen(
                    "<ManufacturerData>",
                    "<ManufacturerData xmlns=\"urn:foreign\">",
                    1,
                ),
                source.replacen("<ManufacturerData>", "<ManufacturerData xmlns=\"\">", 1),
                source
                    .replacen("<ManufacturerData>", "<p:ManufacturerData>", 1)
                    .replacen("</ManufacturerData>", "</p:ManufacturerData>", 1),
                source
                    .replacen(
                        "<ManufacturerData>",
                        &format!("<p:ManufacturerData xmlns:p=\"{ns}\">"),
                        1,
                    )
                    .replacen("</ManufacturerData>", "</p:ManufacturerData>", 1),
                source.replacen("<KNX ", &format!("<KNX xmlns=\"{ns}\" "), 1),
                source.replacen(
                    "<ManufacturerData>",
                    "<ManufacturerData bad=\"1\" bad=\"2\">",
                    1,
                ),
                source.replacen("<ManufacturerData>", "<ManufacturerData p:bad=\"1\">", 1),
                format!("{source}{source}"),
                format!("{source}trailing"),
                source[..source.len() - 6].into(),
                format!("<!DOCTYPE KNX [<!ENTITY bad SYSTEM 'file:///unavailable'>]>{source}"),
            ] {
                assert!(with_member(original.clone(), index, bad.into_bytes())
                    .plan()
                    .is_err());
            }
        }
    }
}
#[test]
fn full_xml_boundary_refuses_repeated_declarations_and_illegal_literal_characters() {
    let original = source_bundle(20);
    let source = source_member(&original, 0);
    for bad in [
        format!("<?xml version=\"1.0\"?><?xml version=\"1.0\"?>{source}"),
        source.replace("<ManufacturerData>", "<ManufacturerData>\0"),
        source.replace("Name=\"Synthetic\"", "Name=\"Synthetic\0\""),
    ] {
        let bundle = with_member(original.clone(), 0, bad.into_bytes());
        let result = bundle.plan();
        assert!(
            result.is_err(),
            "strict full XML boundary must reject repeated declarations and illegal characters"
        );
    }
}
