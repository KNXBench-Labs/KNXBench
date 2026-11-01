//! Characterizes independent PNG budgets, integrity failures and report accounting through the API.

use super::distribution_baggage::{authored_png, image_archive, png_chunk};
use knx_productdb::distribution::{
    inspect_distribution, inspect_distribution_with_limits, DistributionLimits as L,
};

fn refuses(limits: L, reason: &str) {
    let r =
        inspect_distribution_with_limits("bounded.zip", &image_archive(&authored_png()), limits)
            .unwrap();
    let j = serde_json::to_value(r).unwrap();
    assert_eq!(
        j["resourceDeclarations"][0]["passivePresentation"]["reason"],
        reason
    );
    assert_eq!(j["resourcePreviews"], serde_json::json!([]));
}
macro_rules! budget {
    ($name:ident, $field:ident, $value:expr, $reason:literal) => {
        #[test]
        fn $name() {
            let mut l = L::default();
            l.resource.$field = $value;
            refuses(l, $reason);
        }
    };
}
budget!(
    png_encoded_bytes,
    max_encoded_bytes,
    authored_png().len() - 1,
    "resourceLimitEncodedBytes"
);
budget!(png_dimension, max_dimension, 0, "resourceLimitDimensions");
budget!(png_pixels, max_pixels, 0, "resourceLimitPixels");
budget!(
    png_decoded_bytes,
    max_decoded_bytes,
    3,
    "resourceLimitDecodedBytes"
);
budget!(
    png_decoder_internal_bytes,
    max_decoder_bytes,
    0,
    "resourceLimitDecoderBytes"
);
budget!(
    png_derived_bytes,
    max_preview_bytes,
    0,
    "resourceLimitPreviewBytes"
);
budget!(png_chunk_work, max_png_chunks, 3, "resourceLimitPngChunks");
budget!(
    png_cumulative_work,
    max_work_bytes,
    authored_png().len() * 3 + 15,
    "resourceLimitWorkBytes"
);
budget!(
    metadata_encoded_bytes,
    max_metadata_bytes,
    authored_png().len() - 1,
    "resourceLimitMetadataBytes"
);

#[test]
fn png_inclusive_caller_limits_accept_the_exact_next_reachable_budget() {
    let png = authored_png();
    let mut l = L::default();
    l.resource.max_encoded_bytes = png.len();
    l.resource.max_metadata_bytes = png.len();
    l.resource.max_dimension = 1;
    l.resource.max_pixels = 1;
    l.resource.max_decoded_bytes = 4;
    l.resource.max_png_chunks = 4;
    l.resource.max_work_bytes = png.len() * 3 + 16;
    let r = inspect_distribution_with_limits("exact.zip", &image_archive(&png), l).unwrap();
    assert_eq!(r.resource_previews.len(), 1);
    use base64::Engine;
    let size = base64::engine::general_purpose::STANDARD
        .decode(&r.resource_previews[0].png_base64)
        .unwrap()
        .len();
    l.resource.max_preview_bytes = size;
    assert_eq!(
        inspect_distribution_with_limits("exact.zip", &image_archive(&png), l)
            .unwrap()
            .resource_previews
            .len(),
        1
    );
    l.resource.max_preview_bytes = size - 1;
    refuses(l, "resourceLimitPreviewBytes");
}
fn bad(data: &[u8], reason: &str) {
    let r = inspect_distribution("bad.zip", &image_archive(data)).unwrap();
    assert_eq!(
        r.resource_declarations[0].passive_presentation.reason,
        reason
    );
    assert!(r.resource_previews.is_empty());
}
#[test]
fn truncated_magic_is_not_a_decoded_preview() {
    bad(b"\x89PNG\r\n\x1a\n", "pngDecodeFailed");
}
#[test]
fn corrupt_crc_is_not_a_decoded_preview() {
    let mut p = authored_png();
    p[29] ^= 1;
    bad(&p, "pngDecodeFailed");
}
#[test]
fn invalid_zlib_frame_is_not_a_decoded_preview() {
    let p = authored_png();
    let mut out = p[..33].to_vec();
    png_chunk(&mut out, b"IDAT", b"not-zlib");
    png_chunk(&mut out, b"IEND", &[]);
    bad(&out, "pngDecodeFailed");
}
#[test]
fn missing_iend_is_not_a_decoded_preview() {
    let mut p = authored_png();
    p.truncate(p.len() - 12);
    bad(&p, "pngDecodeFailed");
}
#[test]
fn trailing_corruption_is_not_a_decoded_preview() {
    let mut p = authored_png();
    p.extend_from_slice(b"unexpected");
    bad(&p, "pngDecodeFailed");
}
#[test]
fn animated_png_is_explicitly_unsupported() {
    let p = authored_png();
    let mut out = p[..33].to_vec();
    png_chunk(&mut out, b"acTL", &[0, 0, 0, 1, 0, 0, 0, 0]);
    out.extend_from_slice(&p[33..]);
    bad(&out, "animatedPngUnsupported");
}
#[test]
fn oversized_header_is_refused_before_pixel_allocation() {
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut h = Vec::new();
    h.extend_from_slice(&100_000u32.to_be_bytes());
    h.extend_from_slice(&100_000u32.to_be_bytes());
    h.extend_from_slice(&[8, 6, 0, 0, 0]);
    png_chunk(&mut out, b"IHDR", &h);
    png_chunk(&mut out, b"IEND", &[]);
    bad(&out, "resourceLimitDimensions");
}
#[test]
fn exact_report_cap_accepts_final_unsafe_path_fallbacks_without_temporary_inflation() {
    use crate::standalone_packages::wp16::{raw_zip, RawMember};
    let declarations = (0..64)
        .map(|i| format!("<Baggage Id=\"B{i}\" Name=\"x\"/>"))
        .collect::<String>();
    let xml = format!(
        r#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-1"><Baggages>{declarations}</Baggages></Manufacturer></ManufacturerData></KNX>"#
    );
    let zip = raw_zip(&[RawMember::stored(b"source.xml", xml.as_bytes())]);
    let r = inspect_distribution("unsafe.zip", &zip).unwrap();
    let exact = r.serialized_size().unwrap();
    assert_eq!(
        inspect_distribution_with_limits(
            "unsafe.zip",
            &zip,
            L {
                max_report_bytes: exact,
                ..L::default()
            }
        )
        .unwrap(),
        r
    );
}

#[test]
fn resource_identity_lookup_keys_are_bounded() {
    let xml = format!(
        r#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M"><Baggages><Baggage Id="{}" Name="x" TargetPath=""/></Baggages></Manufacturer></ManufacturerData></KNX>"#,
        "x".repeat(4097)
    );
    assert!(
        inspect_distribution("id.xml", xml.as_bytes()).is_err(),
        "oversized resource identity must be refused"
    );
}
#[test]
fn resource_consumer_lookup_keys_are_bounded() {
    let xml = format!(
        r#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M"><ApplicationPrograms><ApplicationProgram Id="A" IconFile="{}"/></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#,
        "x".repeat(4097)
    );
    assert!(
        inspect_distribution("ref.xml", xml.as_bytes()).is_err(),
        "oversized resource consumer must be refused"
    );
}

#[test]
fn format_five_requires_every_resource_array_instead_of_silently_losing_facts() {
    let r = inspect_distribution("format.xml", super::distribution_inspection::COMBINED).unwrap();
    for field in [
        "resourceDeclarations",
        "resourceConsumers",
        "resourcePasses",
        "resourcePreviews",
    ] {
        let mut j = serde_json::to_value(&r).unwrap();
        j.as_object_mut().unwrap().remove(field);
        j["formatVersion"] = 4.into();
        assert!(
            serde_json::from_value::<knx_productdb::distribution::DistributionInspection>(j)
                .is_err(),
            "{field}"
        );
    }
}
#[test]
fn new_resource_limits_cannot_raise_the_standard_profile() {
    let bytes = image_archive(&authored_png());
    let r = inspect_distribution("raise.zip", &bytes).unwrap();
    let l = L {
        resource: knx_productdb::distribution::DistributionResourceLimits {
            max_metadata_bytes: usize::MAX,
            max_encoded_bytes: usize::MAX,
            max_dimension: u32::MAX,
            max_pixels: u64::MAX,
            max_decoded_bytes: usize::MAX,
            max_decoder_bytes: usize::MAX,
            max_preview_bytes: usize::MAX,
            max_work_bytes: usize::MAX,
            max_png_chunks: usize::MAX,
        },
        ..L::default()
    };
    assert_eq!(
        inspect_distribution_with_limits("raise.zip", &bytes, l).unwrap(),
        r
    );
}
#[test]
fn existing_derived_view_limit_also_bounds_preview_output() {
    let l = L {
        max_derived_bytes: 0,
        ..L::default()
    };
    refuses(l, "resourceLimitPreviewBytes");
}
#[test]
fn reference_candidates_share_the_existing_reference_ceiling() {
    let xml=br#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M"><ApplicationPrograms><ApplicationProgram Id="A" IconFile="B"/></ApplicationPrograms><Baggages><Baggage Id="B" Name="x" TargetPath=""/></Baggages></Manufacturer></ManufacturerData></KNX>"#;
    assert!(inspect_distribution_with_limits(
        "refs.xml",
        xml,
        L {
            max_references: 1,
            ..L::default()
        }
    )
    .is_err());
    let r = inspect_distribution_with_limits(
        "refs.xml",
        xml,
        L {
            max_references: 2,
            ..L::default()
        },
    )
    .unwrap();
    assert_eq!(r.resource_consumers[0].declaration_candidates.len(), 1);
}
/// Source XML with one IconFile consumer, one Baggage and a body charging earlier reader passes.
fn combined(body: &str, icon: bool) -> String {
    format!(
        r#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M"><ApplicationPrograms><ApplicationProgram Id="A"{}>{body}</ApplicationProgram></ApplicationPrograms><Baggages><Baggage Id="B" Name="x" TargetPath=""/></Baggages></Manufacturer></ManufacturerData></KNX>"#,
        if icon { r#" IconFile="B""# } else { "" }
    )
}
/// Exact inclusive ceiling and the next refused one; `units` is the whole report's charge.
fn shares_ceiling(body: &str, units: usize) {
    let xml = combined(body, true);
    let at = |max_references| {
        inspect_distribution_with_limits(
            "shared.xml",
            xml.as_bytes(),
            L {
                max_references,
                ..L::default()
            },
        )
    };
    let r = at(units).unwrap();
    assert_eq!(r.resource_consumers[0].declaration_candidates.len(), 1);
    assert!(
        matches!(
            at(units - 1),
            Err(knx_productdb::distribution::DistributionInspectionError::ResourceLimit(_))
        ),
        "{units} - 1 references must refuse"
    );
}
#[test]
fn dependency_endpoints_and_resource_consumers_share_one_reference_ceiling() {
    let body =
        r#"<Static/><Dynamic><Assign SourceParamRefRef="S" TargetParamRefRef="T"/></Dynamic>"#;
    // Without a consumer, the readers and the Assign endpoints alone fit in four units.
    inspect_distribution_with_limits(
        "dependency.xml",
        combined(body, false).as_bytes(),
        L {
            max_references: 4,
            ..L::default()
        },
    )
    .unwrap();
    // Two ordinary references, two endpoints, then the consumer and its one candidate.
    shares_ceiling(body, 6);
}
#[test]
fn candidate_targets_and_resource_consumers_share_one_reference_ceiling() {
    let body = r#"<Static><Parameters><Parameter Id="P" Value="0"/></Parameters></Static>"#;
    inspect_distribution_with_limits(
        "candidate.xml",
        combined(body, false).as_bytes(),
        L {
            max_references: 1,
            ..L::default()
        },
    )
    .unwrap();
    // One candidate target, then the consumer and its one candidate.
    shares_ceiling(body, 3);
}
#[test]
fn file_info_attributes_member_candidates_and_previews_are_each_output_rows() {
    use crate::standalone_packages::wp16::{raw_zip, RawMember};
    let xml=br#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M"><Baggages><Baggage Id="B" Name="x.png" TargetPath=""><FileInfo TimeInfo="t" Extra="e"/></Baggage></Baggages></Manufacturer></ManufacturerData></KNX>"#;
    let png = authored_png();
    let zip = raw_zip(&[
        RawMember::stored(b"source.xml", xml),
        RawMember::stored(b"source.baggages/x.png", &png),
    ]);
    let r = inspect_distribution("rows.zip", &zip).unwrap();
    let structural = r
        .documents
        .iter()
        .map(|d| 1 + d.names.len() + d.locations.len())
        .sum::<usize>()
        + r.sections.len()
        + r.unknowns.len()
        + r.identities.len()
        + r.references.len()
        + r.reader_passes.len()
        + r.diagnostics.len()
        + r.members.len()
        + r.declared_metadata.len()
        + r.preserved_content.len();
    assert!(
        r.applications.is_empty()
            && r.hardware.is_empty()
            && r.catalog_items.is_empty()
            && r.dependency_evidence.is_empty()
            && r.source_candidates.is_empty()
    );
    // One declaration, one candidate, FileInfo plus two attributes, one preview and one pass.
    let exact = structural + 7;
    assert_eq!(
        inspect_distribution_with_limits(
            "rows.zip",
            &zip,
            L {
                max_output_records: exact,
                ..L::default()
            }
        )
        .unwrap(),
        r
    );
    assert!(inspect_distribution_with_limits(
        "rows.zip",
        &zip,
        L {
            max_output_records: exact - 1,
            ..L::default()
        }
    )
    .is_err());
}

#[test]
fn resource_report_bytes_are_exact_inclusive_and_nested_rows_are_charged() {
    let zip = image_archive(&authored_png());
    let r = inspect_distribution("report.zip", &zip).unwrap();
    let exact = r.serialized_size().unwrap();
    let mut l = L {
        max_report_bytes: exact,
        ..L::default()
    };
    assert_eq!(
        inspect_distribution_with_limits("report.zip", &zip, l).unwrap(),
        r
    );
    l.max_report_bytes -= 1;
    assert!(inspect_distribution_with_limits("report.zip", &zip, l).is_err());
    l = L {
        max_output_records: 1,
        ..L::default()
    };
    assert!(inspect_distribution_with_limits("report.zip", &zip, l).is_err());
}
