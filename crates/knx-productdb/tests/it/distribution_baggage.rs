//! Tests source-bound distribution baggage joins and passive-only preview reports.

use super::distribution_inspection::{inspect_distribution, COMBINED};
use crate::standalone_packages::wp16::{raw_zip, RawMember};

#[test]
fn archive_maps_literal_adjacent_stem_and_exact_member_identity() {
    let bytes = raw_zip(&[
        RawMember::stored(b"dir/product.xml", COMBINED),
        RawMember::stored(b"dir/product.baggages/docs/help.txt", b"passive metadata"),
        RawMember::stored(b"other/help.txt", b"not the payload"),
    ]);
    let report = inspect_distribution("authored.zip", &bytes).unwrap();
    let json = serde_json::to_value(&report).unwrap();
    let d = &json["resourceDeclarations"][0];
    assert_eq!(d["resolution"], "resolved");
    assert_eq!(
        d["expectedLogicalPath"],
        "dir/product.baggages/docs/help.txt"
    );
    assert_eq!(d["memberCandidates"], serde_json::json!([1]));
    assert_eq!(
        d["media"]["sha256"],
        knx_productdb::sha256_hex(b"passive metadata")
    );
    assert_eq!(d["passivePresentation"]["reason"], "unsupportedMedia");
    assert_eq!(json["resourcePasses"][0]["association"], "performed");
    assert!(json["limitations"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("nativePackageBaggageNotPerformed")));
}

#[test]
fn canonical_consumers_resolve_across_members_without_borrowing_manufacturers() {
    let declarations = br#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-1"><Baggages><Baggage Id="B" Name="pixel.png" TargetPath=""/></Baggages></Manufacturer></ManufacturerData></KNX>"#;
    let applications = br#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-1"><ApplicationPrograms><ApplicationProgram Id="A" ContextHelpFile="B" IconFile=""><Static><ParameterTypes><ParameterType Id="T"><TypePicture RefId="B"/></ParameterType></ParameterTypes><Extension><Baggage RefId="B"/></Extension></Static></ApplicationProgram></ApplicationPrograms></Manufacturer><Manufacturer RefId="M-2"><ApplicationPrograms><ApplicationProgram Id="Other" IconFile="B"/></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;
    let bytes = raw_zip(&[
        RawMember::stored(b"decl.xml", declarations),
        RawMember::stored(b"apps.xml", applications),
    ]);
    let r = inspect_distribution("split.zip", &bytes).unwrap();
    let j = serde_json::to_value(&r).unwrap();
    let rows = j["resourceConsumers"]
        .as_array()
        .expect("typed resource consumer rows");
    assert_eq!(rows.len(), 5);
    assert_eq!(rows[0]["referenceState"], "resolved");
    assert_eq!(
        rows[0]["declarationCandidates"][0],
        j["resourceDeclarations"][0]["source"]
    );
    assert_eq!(rows[1]["referenceState"], "dangling");
    assert_eq!(rows[1]["declaredBaggageRef"], "");
    assert_eq!(rows[2]["role"], "rasterPicture");
    assert_eq!(rows[2]["referenceState"], "resolved");
    assert_eq!(rows[3]["role"], "resourceReference");
    assert_eq!(rows[3]["referenceState"], "resolved");
    assert_eq!(rows[4]["referenceState"], "dangling");
}

#[test]
fn help_zip_is_metadata_only_and_preserves_file_info_without_nested_extraction() {
    let xml = br#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-1"><Baggages><Baggage Id="H" Name="help.zip" TargetPath="" InstallOnImport="true"><FileInfo TimeInfo="2001-01-01"/></Baggage></Baggages></Manufacturer></ManufacturerData></KNX>"#;
    let nested = raw_zip(&[
        RawMember::stored(b"help.html", b"<script>NEVER_RENDER_HELP</script>"),
        RawMember::stored(b"inner.zip", b"not opened"),
    ]);
    let bytes = raw_zip(&[
        RawMember::stored(b"product.xml", xml),
        RawMember::stored(b"product.baggages/help.zip", &nested),
    ]);
    let r = inspect_distribution("help.zip", &bytes).unwrap();
    let j = serde_json::to_value(r).unwrap();
    let d = &j["resourceDeclarations"][0];
    assert_eq!(d["passivePresentation"]["state"], "metadataOnly");
    assert_eq!(d["passivePresentation"]["reason"], "helpArchiveAvailable");
    assert_eq!(d["media"]["entries"], 2);
    assert_eq!(d["media"]["archiveNamedEntries"], 1);
    assert_eq!(d["media"]["encryptedEntries"], 0);
    assert_eq!(d["installOnImport"], "true");
    assert_eq!(d["fileInfo"][0]["attributes"][0]["value"], "2001-01-01");
    assert!(!j.to_string().contains("NEVER_RENDER_HELP"));
}

pub(super) fn png_chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let mut crc = crc32fast::Hasher::new();
    crc.update(kind);
    crc.update(data);
    out.extend_from_slice(&crc.finalize().to_be_bytes());
}
pub(super) fn authored_png() -> Vec<u8> {
    use std::io::Write;
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut header = Vec::new();
    header.extend_from_slice(&1u32.to_be_bytes());
    header.extend_from_slice(&1u32.to_be_bytes());
    header.extend_from_slice(&[8, 6, 0, 0, 0]);
    png_chunk(&mut out, b"IHDR", &header);
    png_chunk(&mut out, b"tEXt", b"Comment\0DO_NOT_RETAIN_METADATA");
    let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    z.write_all(&[0, 255, 0, 0, 255]).unwrap();
    png_chunk(&mut out, b"IDAT", &z.finish().unwrap());
    png_chunk(&mut out, b"IEND", &[]);
    out
}
pub(super) fn image_archive(png: &[u8]) -> Vec<u8> {
    let xml = br#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-1"><Baggages><Baggage Id="PNG" Name="pixel.png" TargetPath=""/></Baggages></Manufacturer></ManufacturerData></KNX>"#;
    raw_zip(&[
        RawMember::stored(b"product.xml", xml),
        RawMember::stored(b"product.baggages/pixel.png", png),
    ])
}
#[test]
fn png_preview_is_decoded_reencoded_and_bound_to_original_and_derived_hashes() {
    use base64::Engine;
    let png = authored_png();
    let r = inspect_distribution("picture.zip", &image_archive(&png)).unwrap();
    let j = serde_json::to_value(&r).unwrap();
    assert_eq!(
        j["resourceDeclarations"][0]["passivePresentation"]["state"],
        "decodedPreview"
    );
    let p = &j["resourcePreviews"][0];
    assert_eq!(p["memberOrdinal"], 1);
    assert_eq!(p["originalSha256"], knx_productdb::sha256_hex(&png));
    assert_eq!(p["width"], 1);
    assert_eq!(p["height"], 1);
    let derived = base64::engine::general_purpose::STANDARD
        .decode(p["pngBase64"].as_str().unwrap())
        .unwrap();
    assert_eq!(p["derivedSha256"], knx_productdb::sha256_hex(&derived));
    assert_ne!(png, derived);
    assert!(!derived
        .windows(b"DO_NOT_RETAIN_METADATA".len())
        .any(|w| w == b"DO_NOT_RETAIN_METADATA"));
    assert_eq!(&derived[..8], b"\x89PNG\r\n\x1a\n");
    let mut reader = png::Decoder::new(std::io::Cursor::new(&derived))
        .read_info()
        .unwrap();
    let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
    let frame = reader.next_frame(&mut pixels).unwrap();
    assert_eq!(&pixels[..frame.buffer_size()], &[255, 0, 0, 255]);
    reader.finish().unwrap();
    assert_eq!(p["transformation"], "decoded-pixels-to-png-v1");
}

#[test]
fn metadata_work_refusal_has_a_distinct_reason_from_encoded_size() {
    use knx_productdb::distribution::{inspect_distribution_with_limits, DistributionLimits};
    let png = authored_png();
    let mut limits = DistributionLimits::default();
    limits.resource.max_work_bytes = png.len() - 1;
    let report =
        inspect_distribution_with_limits("work.zip", &image_archive(&png), limits).unwrap();
    let j = serde_json::to_value(report).unwrap();
    assert_eq!(
        j["resourceDeclarations"][0]["passivePresentation"]["reason"],
        "resourceLimitWorkBytes"
    );
    assert_eq!(j["resourcePreviews"], serde_json::json!([]));
}

#[test]
fn loose_xml_keeps_declaration_with_explicit_no_archive_fallback() {
    let report = inspect_distribution("product.xml", COMBINED).unwrap();
    let json = serde_json::to_value(&report).unwrap();
    assert_eq!(
        json["resourceDeclarations"][0]["resolution"],
        "noArchiveEnvelope"
    );
    assert_eq!(json["resourceDeclarations"][0]["id"], "B-1");
    assert_eq!(json["resourceDeclarations"][0]["name"], "help.txt");
    assert_eq!(json["formatVersion"], 5);
    assert_eq!(json["complete"], false);
}
