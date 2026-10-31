//! Checks bounded report work and immutable text/metadata provenance, not executable semantics.

use crate::distribution_inspection::*;
use knx_productdb::distribution::{
    inspect_distribution_with_limits, DistributionInspectionError as E, DistributionLimits as L,
};
// Generated grammar and values are independent of manufacturer payloads.
fn generated_source(parameters: usize, refs: usize, extensions: usize) -> Vec<u8> {
    use std::fmt::Write;
    let mut body = String::from("<Catalog/><ApplicationPrograms><ApplicationProgram Id='A' Name='Generated'><Static><Parameters>");
    for i in 0..parameters {
        write!(body, "<Parameter Id='P{i}' Name='Generated {i}' Value='{i}'><Memory CodeSegment='SEG' Offset='{i}' BitOffset='0'/></Parameter>").unwrap();
    }
    body.push_str("</Parameters><ParameterRefs>");
    for i in 0..refs {
        write!(
            body,
            "<ParameterRef Id='R{i}' RefId='P{}'/>",
            i % parameters
        )
        .unwrap();
    }
    body.push_str("</ParameterRefs></Static><Dynamic>");
    for i in 0..refs {
        write!(body, "<ParameterRefRef RefId='R{i}'/>").unwrap();
    }
    body.push_str("</Dynamic></ApplicationProgram></ApplicationPrograms><Baggages>");
    for _ in 0..parameters * 2 {
        body.push_str("<Baggage/>");
    }
    body.push_str("</Baggages><Hardware/><Extension>");
    for _ in 0..extensions {
        body.push_str("<Opaque marker='retained'/>");
    }
    body.push_str("</Extension>");
    source(&body)
}
fn assert_generated(parameters: usize, refs: usize, extensions: usize) {
    let bytes = generated_source(parameters, refs, extensions);
    let result = inspect_distribution("generated", &bytes);
    assert!(
        result.is_ok(),
        "bounded realistic source must retain useful facts: {result:?}"
    );
    let r = result.unwrap();
    assert_eq!(r.source.sha256, knx_productdb::sha256_hex(&bytes));
    assert_eq!(r.sections.len(), 4);
    assert_eq!(r.applications.len(), 1);
    let app = &r.applications[0];
    assert_eq!(app.id.as_deref(), Some("A"));
    assert_eq!(app.name.as_deref(), Some("Generated"));
    assert_eq!(app.parameter_fallbacks.len(), refs);
    for (i, f) in app.parameter_fallbacks.iter().enumerate() {
        assert_eq!(f.parameter_ref_id, format!("R{i}"));
        assert_eq!(f.parameter_id, format!("P{}", i % parameters));
        assert_eq!(f.value, Some((i % parameters).to_string()));
        assert_eq!(f.origin, DistributionFallbackOrigin::Parameter);
    }
    assert_eq!(
        r.identities
            .iter()
            .filter(|i| i.kind == "Parameter")
            .count(),
        parameters
    );
    assert_eq!(
        r.identities
            .iter()
            .filter(|i| i.kind == "ParameterRef")
            .count(),
        refs
    );
    assert_eq!(
        r.references
            .iter()
            .filter(|r| r.attribute == "RefId")
            .count(),
        refs * 2
    );
    assert!(r
        .references
        .iter()
        .all(|r| r.state == knx_productdb::distribution::DistributionReferenceState::Resolved));
    assert_eq!(
        r.unknowns.iter().filter(|u| u.raw_name == "Opaque").count(),
        extensions
    );
    assert_eq!(
        r.unknowns.iter().filter(|u| u.raw_name == "marker").count(),
        extensions
    );
    assert!(!r.complete);
}
#[test]
fn compact_small_source_retains_every_generated_fact() {
    assert_generated(2, 3, 4);
}
#[test]
fn compact_corpus_sized_source_retains_every_generated_fact() {
    assert_generated(50_402, 55_748, 60_000);
}

#[test]
fn compact_json_byte_bound_is_exact_inclusive_and_independent_of_working_limits() {
    let bytes = source("<Catalog/>");
    let r = inspect_distribution("exact", &bytes).unwrap();
    let size = serde_json::to_vec(&r).unwrap().len();
    assert_eq!(size, r.serialized_size().unwrap());
    for (cap, positive) in [(size - 1, false), (size, true), (size + 1, true)] {
        let result = inspect_distribution_with_limits(
            "exact",
            &bytes,
            L {
                max_report_bytes: cap,
                ..L::default()
            },
        );
        assert_eq!(
            result.is_ok(),
            positive,
            "cap={cap}, size={size}, result={result:?}"
        );
    }
    assert!(matches!(
        inspect_distribution_with_limits(
            "exact",
            &bytes,
            L {
                max_xml_working_bytes: 0,
                ..L::default()
            }
        ),
        Err(E::ResourceLimit(_))
    ));
    let dynamic = source("<ApplicationPrograms><ApplicationProgram Id='A'><Static/><Dynamic><ParameterRefRef RefId='missing'/></Dynamic></ApplicationProgram></ApplicationPrograms>");
    assert!(matches!(
        inspect_distribution_with_limits(
            "dynamic",
            &dynamic,
            L {
                max_dynamic_work_bytes: 0,
                ..L::default()
            }
        ),
        Err(E::ResourceLimit(_))
    ));
}
#[test]
fn compact_source_map_preserves_names_siblings_ranges_and_rejects_untrusted_links() {
    use knx_productdb::distribution::DistributionSourceRef;
    let bytes = source("<Catalog/><x:Extension xmlns:x='urn:foreign'><x:Item/><x:Item>original&amp;text</x:Item></x:Extension>");
    let r = inspect_distribution("map", &bytes).unwrap();
    let d = &r.documents[0];
    let mut item_paths = Vec::new();
    for (i, l) in d.locations.iter().enumerate() {
        let reference = DistributionSourceRef {
            member_ordinal: None,
            location: i as u32 + 1,
        };
        let coordinate = r.resolve_coordinate(reference).unwrap();
        assert_eq!(coordinate.start_byte, l.start);
        assert_eq!(coordinate.end_byte, l.end);
        if d.names[l.name as usize].raw == "x:Item" && l.text_token.is_none() {
            assert_eq!(d.names[l.name as usize].namespace, "urn:foreign");
            assert_eq!(coordinate.element_ordinal, i as u64 + 1);
            assert!(bytes[coordinate.start_byte as usize..].starts_with(b"<x:Item"));
            item_paths.push(coordinate.path);
        }
    }
    assert_eq!(
        item_paths,
        [
            "/KNX[1]/ManufacturerData[1]/Manufacturer[1]/Extension[1]/Item[1]",
            "/KNX[1]/ManufacturerData[1]/Manufacturer[1]/Extension[1]/Item[2]"
        ]
    );
    let reference = DistributionSourceRef {
        member_ordinal: None,
        location: 2,
    };
    for mutation in 0..7 {
        let mut bad = r.clone();
        match mutation {
            0 => bad.documents[0].locations[1].parent = Some(1),
            1 => bad.documents[0].locations[1].parent = Some(u32::MAX),
            2 => bad.documents[0].locations[1].name = u32::MAX,
            3 => bad.documents[0].locations[1].end = u64::MAX,
            4 => bad.documents[0].locations[1].start = u64::MAX,
            5 => bad.documents[0].locations[1].sibling = 0,
            _ => bad.documents.push(bad.documents[0].clone()),
        }
        let decoded: knx_productdb::distribution::DistributionInspection =
            serde_json::from_slice(&serde_json::to_vec(&bad).unwrap()).unwrap();
        assert!(
            decoded.resolve_coordinate(reference).is_err(),
            "mutation {mutation}"
        );
    }
    for reference in [
        DistributionSourceRef {
            member_ordinal: Some(999),
            location: 1,
        },
        DistributionSourceRef {
            member_ordinal: None,
            location: 0,
        },
        DistributionSourceRef {
            member_ordinal: None,
            location: u32::MAX,
        },
    ] {
        assert!(r.resolve_coordinate(reference).is_err());
    }
    let json = serde_json::to_vec(&r).unwrap();
    assert_eq!(
        json,
        serde_json::to_vec(&inspect_distribution("map", &bytes).unwrap()).unwrap()
    );
}
#[test]
fn utf8_bom_keeps_original_byte_coordinates_and_whitespace_out_of_preserved_content() {
    use knx_productdb::distribution::DistributionSourceRef;
    let plain = source("<Catalog/><Extension>\n  <Item>kept</Item>\n  <Item/>\n</Extension>");
    let mut bom = vec![0xef, 0xbb, 0xbf];
    bom.extend_from_slice(&plain);
    for (label, bytes) in [("plain", &plain), ("bom", &bom)] {
        let r = inspect_distribution(label, bytes).unwrap();
        let d = &r.documents[0];
        for (i, l) in d.locations.iter().enumerate() {
            let reference = DistributionSourceRef {
                member_ordinal: None,
                location: i as u32 + 1,
            };
            let c = r.resolve_coordinate(reference).unwrap();
            let original = &bytes[c.start_byte as usize..c.end_byte as usize];
            if l.text_token.is_none() {
                let open = format!("<{}", d.names[l.name as usize].raw);
                assert!(
                    original.starts_with(open.as_bytes()),
                    "{label}: {} at {}",
                    c.path,
                    c.start_byte
                );
                assert!(original.ends_with(b">"), "{label}: {}", c.path);
            }
        }
        assert_eq!(
            r.preserved_content.len(),
            1,
            "{label}: only non-whitespace text is content"
        );
        let c = r.resolve_coordinate(r.preserved_content[0].source).unwrap();
        assert_eq!(
            &bytes[c.start_byte as usize..c.end_byte as usize],
            b"kept",
            "{label}"
        );
        assert_eq!(
            r.preserved_content[0].sha256,
            knx_productdb::sha256_hex(b"kept")
        );
    }
}
#[test]
fn compact_empty_reference_never_resolves_an_empty_identity() {
    let bytes = source("<ApplicationPrograms><ApplicationProgram Id='A'><Static><Parameters><Parameter Id='' Value='not a target'/></Parameters><ParameterRefs><ParameterRef Id='R' RefId=''/></ParameterRefs></Static><Dynamic/></ApplicationProgram></ApplicationPrograms>");
    let r = inspect_distribution("empty-ref", &bytes).unwrap();
    let reference = r
        .references
        .iter()
        .find(|r| r.attribute == "RefId")
        .unwrap();
    assert_eq!(
        reference.state,
        knx_productdb::distribution::DistributionReferenceState::Dangling
    );
    assert!(reference.candidates.is_empty());
    assert!(r.identities.iter().any(|i| i.declared_id.is_empty()));
    assert!(r.resolve_coordinate(reference.source).is_ok());
}

fn source(body: &str) -> Vec<u8> {
    format!(r#"<KNX xmlns="http://knx.org/xml/project/20" CreatedBy="Independent"><ManufacturerData><Manufacturer RefId="M-0001">{body}</Manufacturer></ManufacturerData></KNX>"#).into_bytes()
}
#[test]
fn script_text_cdata_and_declared_metadata_have_exact_original_ranges_without_echoing_code() {
    let bytes = source(
        r#"<Catalog Strange="preserve"/><ApplicationPrograms><ApplicationProgram Id="A" ApplicationNumber="17" ApplicationVersion="3"><Static><Script><![CDATA[throw new Error('must never run');]]></Script><Extension>opaque &amp; literal</Extension></Static><Dynamic/></ApplicationProgram></ApplicationPrograms>"#,
    );
    let r = inspect_distribution("metadata", &bytes).unwrap();
    assert!(r
        .declared_metadata
        .iter()
        .any(|m| m.attribute == "CreatedBy" && m.value == "Independent"));
    assert!(r
        .declared_metadata
        .iter()
        .any(|m| m.attribute == "ApplicationNumber" && m.value == "17"));
    assert!(r
        .unknowns
        .iter()
        .any(|u| u.kind == "attribute" && u.raw_name == "Strange"));
    assert!(r.preserved_content.iter().any(|c| c.kind == "cdata"));
    for c in &r.preserved_content {
        let coordinate = r.resolve_coordinate(c.source).unwrap();
        let original = &bytes[coordinate.start_byte as usize..coordinate.end_byte as usize];
        assert_eq!(c.sha256, knx_productdb::sha256_hex(original));
        assert!(coordinate.path.contains("/Script[") || coordinate.path.contains("/Extension["));
    }
    let json = serde_json::to_string(&r).unwrap();
    assert!(!json.contains("must never run"));
    assert!(!json.contains("opaque &amp; literal"));
}
#[test]
fn callers_cannot_raise_default_security_profile_or_disable_report_work_bound() {
    let deep = source(&format!(
        "<Catalog>{}{}</Catalog>",
        "<Extension>".repeat(130),
        "</Extension>".repeat(130)
    ));
    let l = L {
        max_depth: usize::MAX,
        ..L::default()
    };
    assert!(matches!(
        inspect_distribution_with_limits("clamped", &deep, l),
        Err(E::ResourceLimit(_))
    ));
    let l = L {
        max_report_bytes: 0,
        ..L::default()
    };
    assert!(matches!(
        inspect_distribution_with_limits("output", COMBINED, l),
        Err(E::ResourceLimit(_))
    ));
    let many = source(&format!(
        "<Catalog>{}</Catalog>",
        "<Extension Strange='x'/>".repeat(100)
    ));
    let l = L {
        max_output_records: 20,
        ..L::default()
    };
    assert!(matches!(
        inspect_distribution_with_limits("records", &many, l),
        Err(E::ResourceLimit(_))
    ));
}
#[test]
fn duplicate_parameter_refs_in_other_module_scopes_do_not_supply_root_defaults() {
    let bytes = source(
        r#"<ApplicationPrograms><ApplicationProgram Id="A"><Static><Parameters><Parameter Id="P" Value="root"/></Parameters><ParameterRefs><ParameterRef Id="R" RefId="P"/></ParameterRefs><ModuleDefs><ModuleDef Id="MD"><Static><Parameters><Parameter Id="P" Value="module"/></Parameters><ParameterRefs><ParameterRef Id="R" RefId="P"/></ParameterRefs></Static><Dynamic/></ModuleDef></ModuleDefs></Static><Dynamic><ParameterRefRef RefId="R"/></Dynamic></ApplicationProgram></ApplicationPrograms>"#,
    );
    let r = inspect_distribution("module", &bytes).unwrap();
    assert_eq!(
        r.applications[0].parameter_fallbacks[0].value,
        Some("root".into()),
        "root ref must not bind module-local declaration"
    );
    assert_eq!(
        r.applications[0].parameter_fallbacks[1].value,
        Some("module".into())
    );
}
