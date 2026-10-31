//! Exercises distribution scope, namespace, identity and refusal contracts with authored sources.

use crate::distribution_inspection::*;
use knx_productdb::distribution::{
    inspect_distribution_with_limits, DistributionInspection, DistributionInspectionError as E,
    DistributionLimits as L, DistributionReferenceState as State,
};

fn xml(body: &str) -> Vec<u8> {
    format!(r#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData>{body}</ManufacturerData></KNX>"#).into_bytes()
}
fn manufacturer(body: &str) -> String {
    format!(r#"<Manufacturer RefId="M-0001">{body}</Manufacturer>"#)
}
fn inspect(body: &str) -> DistributionInspection {
    inspect_distribution("authored", &xml(&manufacturer(body))).unwrap()
}

#[test]
fn split_oracle_order_and_repeat_keep_typed_facts() {
    let combined = inspect_distribution("source", COMBINED).unwrap();
    let text = std::str::from_utf8(COMBINED).unwrap();
    let mut bodies = Vec::new();
    for name in ["Catalog", "ApplicationPrograms", "Baggages", "Hardware"] {
        let start = text.find(&format!("<{name}>")).unwrap();
        let close = format!("</{name}>");
        let end = if name == "Hardware" {
            text.rfind(&close).unwrap()
        } else {
            text.find(&close).unwrap()
        } + close.len();
        bodies.push(text[start..end].to_string());
    }
    let split: Vec<_> = bodies.iter().map(|b| inspect(b)).collect();
    assert_eq!(split.iter().map(|r| r.sections.len()).sum::<usize>(), 4);
    assert_eq!(split[0].catalog_items[0].id, combined.catalog_items[0].id);
    assert_eq!(
        split[1].applications[0].parameter_fallbacks[0].value,
        Some("1".into())
    );
    assert_eq!(
        split[1].applications[0].active_parameter_ref_ids,
        ["R-Control"]
    );
    assert_eq!(
        split[3].hardware[0].program_links[0].application_program_ref_ids,
        ["A-1"]
    );
    bodies.reverse();
    let reordered = inspect(&bodies.concat());
    assert_eq!(reordered.catalog_items[0].id, combined.catalog_items[0].id);
    assert_eq!(
        reordered.sections[0].kind,
        DistributionSectionKind::Hardware
    );
    assert_eq!(
        reordered.applications[0].active_parameter_ref_ids,
        ["R-Control"]
    );
    assert_eq!(combined, inspect_distribution("source", COMBINED).unwrap());
    assert!(!combined.complete);
}

#[test]
fn multi_manufacturer_application_duplicate_and_dangling_scopes_are_explicit() {
    let apps = r#"<ApplicationPrograms><ApplicationProgram Id="A" Name="first"><Static><Parameters><Parameter Id="P" Value="one"/></Parameters><ParameterRefs><ParameterRef Id="R" RefId="P"/></ParameterRefs></Static><Dynamic/></ApplicationProgram><ApplicationProgram Id="A" Name="second"><Static><Parameters><Parameter Id="P" Value="two"/></Parameters><ParameterRefs><ParameterRef Id="R" RefId="P"/><ParameterRef Id="missing" RefId="nope"/></ParameterRefs></Static><Dynamic/></ApplicationProgram></ApplicationPrograms>"#;
    let hardware = r#"<Hardware><Hardware Id="H"><Hardware2Programs><Hardware2Program Id="H2P"><ApplicationProgramRef RefId="A"/></Hardware2Program></Hardware2Programs></Hardware></Hardware>"#;
    let first = manufacturer(&format!("{apps}{hardware}"));
    let other = r#"<Manufacturer RefId="M-0002"><ApplicationPrograms><ApplicationProgram Id="A" Name="third"><Static/><Dynamic/></ApplicationProgram></ApplicationPrograms><Hardware><Hardware Id="H"><Hardware2Programs><Hardware2Program Id="H2P"><ApplicationProgramRef RefId="A"/></Hardware2Program></Hardware2Programs></Hardware></Hardware></Manufacturer>"#;
    let r = inspect_distribution("multi", &xml(&format!("{first}{other}"))).unwrap();
    assert_eq!(r.applications.len(), 3);
    assert_eq!(r.hardware.len(), 2);
    assert_eq!(
        r.applications[0].parameter_fallbacks[0].value,
        Some("one".into())
    );
    assert_eq!(
        r.applications[1].parameter_fallbacks[0].value,
        Some("two".into())
    );
    let links: Vec<_> = r
        .references
        .iter()
        .filter(|r| r.attribute == "RefId" && r.declared_target == "A")
        .collect();
    assert_eq!(links.len(), 2);
    assert_eq!(links[0].state, State::Ambiguous);
    assert_eq!(links[0].candidates.len(), 2);
    assert_eq!(links[1].state, State::Resolved);
    assert_eq!(links[1].candidates.len(), 1);
    assert!(r
        .references
        .iter()
        .any(|r| r.declared_target == "nope" && r.state == State::Dangling));
    assert!(
        r.diagnostics.iter().any(|d| d.code == "duplicateIdentity"),
        "duplicates must be explicit even without incoming refs"
    );
}

#[test]
fn absent_empty_override_and_ambiguous_defaults_stay_distinct() {
    let r = inspect(
        r#"<ApplicationPrograms><ApplicationProgram Id="A"><Static><Parameters><Parameter Id="P" Value="base"/><Parameter Id="empty" Value=""/><Parameter Id="absent"/><Parameter Id="dup" Value="first"/><Parameter Id="dup" Value="second"/></Parameters><ParameterRefs><ParameterRef Id="r1" RefId="P" Value=""/><ParameterRef Id="r2" RefId="empty"/><ParameterRef Id="r3" RefId="absent"/><ParameterRef Id="r4" RefId="dup"/><ParameterRef Id="r5" RefId="missing"/></ParameterRefs></Static><Dynamic><ParameterRefRef RefId="r1"/></Dynamic></ApplicationProgram></ApplicationPrograms>"#,
    );
    let a = &r.applications[0];
    let f = &a.parameter_fallbacks;
    assert_eq!(f.len(), 5);
    assert_eq!(f[0].value, Some("".into()));
    assert_eq!(f[0].origin, DistributionFallbackOrigin::ParameterRef);
    assert_eq!(f[1].value, Some("".into()));
    assert_eq!(f[1].origin, DistributionFallbackOrigin::Parameter);
    let origins = [
        DistributionFallbackOrigin::Absent,
        DistributionFallbackOrigin::AmbiguousParameter,
        DistributionFallbackOrigin::DanglingParameter,
    ];
    for (row, origin) in f[2..].iter().zip(origins) {
        assert_eq!(row.value, None, "{}", row.parameter_ref_id);
        assert_eq!(row.origin, origin, "{}", row.parameter_ref_id);
    }
    assert_eq!(a.dynamic_state, DistributionPassState::NotAttempted);
}

#[test]
fn unknown_occurrences_are_not_multiplied_by_sibling_readers() {
    let source = String::from_utf8(COMBINED.to_vec())
        .unwrap()
        .replace(
            "<Catalog>",
            "<Catalog><Mystery Odd=\"1\"/><Mystery Odd=\"2\"/>",
        )
        .replace("<Static>", "<Static Extra=\"yes\"><Mystery/>");
    let r = inspect_distribution("unknown", source.as_bytes()).unwrap();
    assert_eq!(
        r.unknowns
            .iter()
            .filter(|u| u.kind == "element" && u.raw_name == "Mystery")
            .count(),
        3
    );
    assert_eq!(
        r.unknowns
            .iter()
            .filter(|u| u.kind == "attribute" && u.raw_name == "Odd")
            .count(),
        2
    );
    assert!(!r.unknowns.iter().any(|u| u.kind == "element"
        && ["Catalog", "Hardware", "ApplicationPrograms", "Baggages"]
            .contains(&u.raw_name.as_str())));
    let mut keys = std::collections::HashSet::new();
    for u in &r.unknowns {
        assert!(keys.insert((
            u.source.member_ordinal,
            r.resolve_coordinate(u.source).unwrap().element_ordinal,
            u.kind.clone(),
            u.raw_name.clone()
        )));
    }
}

#[test]
fn foreign_names_and_qualified_attributes_cannot_escape_scope() {
    let body = r#"<x:ApplicationPrograms xmlns:x="urn:foreign"><ApplicationProgram Id="wrong"/></x:ApplicationPrograms><ApplicationPrograms xmlns:x="urn:foreign"><ApplicationProgram x:Id="foreign-only"/><ApplicationProgram Id="real" x:Id="shadow"><Static><Parameters><x:Parameter Id="wrong" Value="42"/><Parameter Id="P" Value="original" x:Value="shadow"/></Parameters><ParameterRefs><ParameterRef Id="R" RefId="P"/></ParameterRefs><x:Wrapper><Parameter Id="injected"/></x:Wrapper></Static><Dynamic><ParameterRefRef RefId="R"/></Dynamic></ApplicationProgram></ApplicationPrograms><Catalog><x:CatalogItem xmlns:x="urn:foreign" Id="wrong"/><CatalogItem Id="real-item"/></Catalog>"#;
    let r = inspect(body);
    assert_eq!(r.sections.len(), 2);
    assert_eq!(r.applications.len(), 2);
    assert_eq!(r.applications[0].id, None);
    assert_eq!(r.applications[1].id, Some("real".into()));
    assert_eq!(
        r.applications[1].parameter_fallbacks[0].value,
        Some("original".into())
    );
    assert_eq!(r.catalog_items.len(), 1);
    assert_eq!(r.catalog_items[0].id, Some("real-item".into()));
    assert!(r
        .unknowns
        .iter()
        .any(|u| u.expanded_name == "{urn:foreign}Id"));
    assert!(r.unknowns.iter().any(|u| u.raw_name == "x:Wrapper"));
    let prefixed=br#"<k:KNX xmlns:k="http://knx.org/xml/project/20"><k:ManufacturerData><k:Manufacturer RefId="M"><k:Catalog><k:CatalogItem Id="C"/></k:Catalog></k:Manufacturer></k:ManufacturerData></k:KNX>"#;
    assert_eq!(
        inspect_distribution("prefix", prefixed)
            .unwrap()
            .catalog_items[0]
            .id,
        Some("C".into())
    );
    let reset = inspect(
        r#"<Catalog><CatalogItem xmlns="" Id="not-knx"/><CatalogItem Id="knx"/></Catalog>"#,
    );
    assert_eq!(reset.catalog_items.len(), 1);
    assert_eq!(reset.catalog_items[0].id, Some("knx".into()));
}

#[test]
fn invalid_bindings_and_duplicate_expanded_attributes_are_typed_refusals() {
    for body in [
        r#"<Catalog x:Id="unbound"/>"#,
        r#"<Catalog xmlns:a="urn:x" xmlns:b="urn:x" a:Id="one" b:Id="two"/>"#,
        r#"<Catalog xmlns:xml="urn:wrong"/>"#,
        r#"<Catalog xmlns:x="http://www.w3.org/XML/1998/namespace"/>"#,
        r#"<Catalog xmlns:x=""/>"#,
        r#"<x:Catalog/>"#,
    ] {
        assert!(
            matches!(
                inspect_distribution("bad-ns", &xml(&manufacturer(body))),
                Err(E::Namespace(_))
            ),
            "{body}"
        );
    }
    assert_eq!(inspect(r#"<Catalog xmlns:xml="http://www.w3.org/XML/1998/namespace" xmlns:a="urn:a" xmlns:b="urn:b" a:Id="one" b:Id="two" xml:lang="en"/>"#).sections.len(),1);
}

#[test]
fn full_document_dtd_encoding_and_schema_refusals_are_distinct() {
    for bytes in [
        b"<KNX>".as_slice(),
        b"<a/><b/>",
        b"<a/>junk",
        b"<a>&unknown;</a>",
        b"<a><b></a>",
        b"<a x='&#0;'/>",
    ] {
        assert!(
            matches!(
                inspect_distribution("malformed", bytes),
                Err(E::MalformedXml(_))
            ),
            "{bytes:?}"
        );
    }
    assert!(matches!(
        inspect_distribution("dtd", b"<!DOCTYPE KNX [<!ENTITY x 'y'>]><KNX/>"),
        Err(E::DtdForbidden)
    ));
    assert!(matches!(
        inspect_distribution("schema", b"<KNX xmlns='http://knx.org/xml/project/99'/>"),
        Err(E::UnsupportedSchema(_))
    ));
    let utf16 = |declaration: &str| {
        let mut b = vec![0xff, 0xfe];
        b.extend(declaration.encode_utf16().flat_map(u16::to_le_bytes));
        b
    };
    assert!(matches!(
        inspect_distribution(
            "encoding",
            &utf16("<?xml version=\"1.0\" encoding=\"UTF-8\"?><KNX/>")
        ),
        Err(E::EncodingConflict)
    ));
    assert!(matches!(
        inspect_distribution(
            "encoding",
            &utf16("<?xml version=\"1.0\" encoding=\"UTF-16\"?><KNX/>")
        ),
        Err(E::UnsupportedEncoding(_))
    ));
    assert!(matches!(
        inspect_distribution(
            "legal-utf16-with-utf8-text",
            &utf16("<?xml version=\"1.0\" encoding=\"UTF-16\"?><KNX CreatedBy=\"UTF-8\"/>")
        ),
        Err(E::UnsupportedEncoding(_))
    ));
    assert!(matches!(
        inspect_distribution(
            "encoding",
            b"<?xml version='1.0' encoding='ISO-8859-1'?><KNX/>"
        ),
        Err(E::UnsupportedEncoding(_))
    ));
    assert!(matches!(
        inspect_distribution("encoding", b"<KNX>\xff</KNX>"),
        Err(E::EncodingConflict)
    ));
    let mut trailing = COMBINED.to_vec();
    trailing.extend(b"<extra/>");
    assert!(matches!(
        inspect_distribution("trailing", &trailing),
        Err(E::MalformedXml(_))
    ));
    assert!(inspect_distribution("utf8", COMBINED).is_ok());
}

#[test]
fn reference_work_boundaries_are_inclusive_without_a_masking_parameter_fallback() {
    let bytes = xml(&manufacturer(
        r#"<Catalog><CatalogItem Id="C" ProductRefId="P"/></Catalog>"#,
    ));
    let l = L {
        max_references: 0,
        ..L::default()
    };
    assert_eq!(
        inspect_distribution_with_limits("refs", &bytes, l),
        Err(E::ResourceLimit("references".into()))
    );
    let l = L {
        max_references: 1,
        ..L::default()
    };
    assert_eq!(
        inspect_distribution_with_limits("refs", &bytes, l)
            .unwrap()
            .references[0]
            .state,
        State::Dangling
    );
    let bytes = xml(&manufacturer(
        r#"<Catalog><CatalogItem Id="C" ProductRefId="P"/></Catalog><Hardware><Hardware Id="H"><Products><Product Id="P"/><Product Id="P"/></Products></Hardware></Hardware>"#,
    ));
    let l = L {
        max_references: 2,
        ..L::default()
    };
    assert_eq!(
        inspect_distribution_with_limits("candidates", &bytes, l),
        Err(E::ResourceLimit("reference candidates".into()))
    );
    let l = L {
        max_references: 3,
        ..L::default()
    };
    let r = inspect_distribution_with_limits("candidates", &bytes, l).unwrap();
    assert_eq!(r.references[0].state, State::Ambiguous);
    assert_eq!(r.references[0].candidates.len(), 2);
}

#[test]
fn exposed_resource_boundaries_refuse_without_partial_success() {
    let limits = [
        (
            L {
                max_input_bytes: COMBINED.len() - 1,
                ..L::default()
            },
            "input bytes/name",
        ),
        (
            L {
                max_member_bytes: COMBINED.len() as u64 - 1,
                ..L::default()
            },
            "XML member bytes",
        ),
        (
            L {
                max_elements: 3,
                ..L::default()
            },
            "elements",
        ),
        (
            L {
                max_depth: 3,
                ..L::default()
            },
            "depth",
        ),
        (
            L {
                max_attributes_per_element: 1,
                ..L::default()
            },
            "attributes per element",
        ),
        (
            L {
                max_text_bytes: 1,
                ..L::default()
            },
            "text bytes",
        ),
        (
            L {
                max_references: 0,
                ..L::default()
            },
            "parameter fallbacks",
        ),
        (
            L {
                max_output_records: 0,
                ..L::default()
            },
            "output records",
        ),
        (
            L {
                max_derived_bytes: 16,
                ..L::default()
            },
            "derived view bytes",
        ),
        (
            L {
                max_scopes: 0,
                ..L::default()
            },
            "reader scopes",
        ),
    ];
    for (l, reason) in limits {
        assert_eq!(
            inspect_distribution_with_limits("bounded", COMBINED, l),
            Err(E::ResourceLimit(reason.into())),
            "{l:?}"
        );
    }
    let exact = L {
        max_input_bytes: COMBINED.len(),
        max_member_bytes: COMBINED.len() as u64,
        ..L::default()
    };
    assert!(inspect_distribution_with_limits("inclusive", COMBINED, exact).is_ok());
}
