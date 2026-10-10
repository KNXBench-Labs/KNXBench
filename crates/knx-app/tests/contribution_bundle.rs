//! Regression tests for consented community evidence.
use knx_app::contribution_bundle::{build_bundle, Audience, BundleOptions};
use std::io::{Cursor, Read};
fn source(xml: &str) -> Vec<u8> {
    knx_testsupport::zip_with_entries(&[
        (
            "knx_master.xml",
            br#"<KNX xmlns="http://knx.org/xml/project/24"/>"#,
        ),
        ("M-0001/Application.xml", xml.as_bytes()),
    ])
}
fn options() -> BundleOptions {
    serde_json::from_str(
        r#"{"audience":"public","sampleIds":["member-2"],"includeOriginal":false,"consent":true}"#,
    )
    .unwrap()
}
#[test]
fn selected_context_is_byte_exact_and_hash_bound_not_claimed_anonymous() {
    let xml = "<KNX xmlns=\"http://knx.org/xml/project/24\"><Future Name=\"Private room\"/></KNX>";
    let bundle = build_bundle(&source(xml), "Private house.knxprod", &options()).unwrap();
    let mut zip = zip::ZipArchive::new(Cursor::new(bundle.bytes)).unwrap();
    let mut data = Vec::new();
    zip.by_name("samples/member-2.xml")
        .unwrap()
        .read_to_end(&mut data)
        .unwrap();
    assert_eq!(data, xml.as_bytes());
    let file = bundle
        .manifest
        .files
        .iter()
        .find(|f| f.path == "samples/member-2.xml")
        .unwrap();
    assert_eq!(file.sha256, knx_productdb::sha256_hex(xml.as_bytes()));
    assert_eq!(bundle.manifest.disclosure, "context-samples");
    assert_eq!(bundle.manifest.original_sha256, None);
}
#[test]
fn original_requires_private_audience_and_separate_consent() {
    let bytes = source("<KNX/>");
    let mut o = options();
    o.sample_ids.clear();
    o.include_original = true;
    assert!(build_bundle(&bytes, "source.knxprod", &o).is_err());
    o.audience = Audience::Private;
    assert!(build_bundle(&bytes, "source.knxprod", &o).is_err());
    let o: BundleOptions = serde_json::from_str(r#"{"audience":"private","sampleIds":[],"includeOriginal":true,"consent":true,"originalConsent":true}"#).unwrap();
    let bundle = build_bundle(&bytes, "source.knxprod", &o).unwrap();
    let mut zip = zip::ZipArchive::new(Cursor::new(bundle.bytes)).unwrap();
    let mut original = Vec::new();
    zip.by_name("original/source.knxprod")
        .unwrap()
        .read_to_end(&mut original)
        .unwrap();
    assert_eq!(original, bytes);
    assert_eq!(
        bundle.manifest.original_sha256,
        Some(knx_productdb::sha256_hex(&bytes))
    );
}
#[test]
fn key_material_blocks_samples_and_original_even_with_consent() {
    let bytes = source("<KNX BCUKey=\"synthetic-secret\"/>");
    assert!(build_bundle(&bytes, "source.knxprod", &options()).is_err());
    let o: BundleOptions = serde_json::from_str(r#"{"audience":"private","sampleIds":[],"includeOriginal":true,"consent":true,"originalConsent":true}"#).unwrap();
    assert!(build_bundle(&bytes, "source.knxprod", &o).is_err());
}
#[test]
fn invalid_selection_and_missing_consent_fail_closed() {
    let bytes = source("<KNX/>");
    for ids in [
        vec!["missing".into()],
        vec!["member-2".into(), "member-2".into()],
    ] {
        let mut o = options();
        o.sample_ids = ids;
        assert!(build_bundle(&bytes, "source.knxprod", &o).is_err());
    }
    let mut o = options();
    o.consent = false;
    assert!(build_bundle(&bytes, "source.knxprod", &o).is_err());
}

#[test]
fn export_refuses_a_manifest_changed_since_preview() {
    let bytes = source("<KNX/>");
    let o: BundleOptions = serde_json::from_str(
        r#"{"audience":"public","sampleIds":[],"consent":true,"expectedManifestSha256":"wrong"}"#,
    )
    .unwrap();
    assert!(build_bundle(&bytes, "source.knxprod", &o).is_err());
}

#[test]
fn preview_manifest_and_every_text_artifact_equal_deterministic_export() {
    let bytes = source("<KNX><X Name=\"private\"/></KNX>");
    let o = options();
    let preview =
        knx_app::contribution_bundle::preview_bundle(&bytes, "private-name.knxprod", &o).unwrap();
    let mut o = o;
    o.expected_manifest_sha256 = Some(preview.manifest_sha256.clone());
    let first = build_bundle(&bytes, "private-name.knxprod", &o).unwrap();
    let second = build_bundle(&bytes, "private-name.knxprod", &o).unwrap();
    assert_eq!(first.bytes, second.bytes);
    let mut zip = zip::ZipArchive::new(Cursor::new(first.bytes)).unwrap();
    for f in preview.files {
        let mut bytes = Vec::new();
        zip.by_name(&f.path)
            .unwrap()
            .read_to_end(&mut bytes)
            .unwrap();
        assert_eq!(Some(String::from_utf8(bytes).unwrap()), f.text);
    }
    let mut manifest = Vec::new();
    zip.by_name("manifest.json")
        .unwrap()
        .read_to_end(&mut manifest)
        .unwrap();
    assert_eq!(
        knx_productdb::sha256_hex(&manifest),
        preview.manifest_sha256
    );
}

/// KL-106: a reduced bundle withholds retained endpoints, MACs and user
/// names; the README names them for the unmodified samples and original.
#[test]
fn reduced_bundle_withholds_retained_network_and_user_values() {
    let bytes = knx_testsupport::retained_privacy_knxproj_bytes();
    let o: BundleOptions =
        serde_json::from_str(r#"{"audience":"public","sampleIds":[],"consent":true}"#).unwrap();
    let bundle = build_bundle(&bytes, "synthetic.knxproj", &o).unwrap();
    assert_eq!(bundle.manifest.disclosure, "reduced");
    let mut zip = zip::ZipArchive::new(Cursor::new(bundle.bytes)).unwrap();
    let mut readme = String::new();
    for i in 0..zip.len() {
        let mut file = zip.by_index(i).unwrap();
        let mut text = String::new();
        file.read_to_string(&mut text).unwrap();
        for planted in knx_testsupport::RETAINED_PRIVACY_PLANTED.iter().chain([
            &knx_testsupport::RETAINED_PRIVACY_UNKNOWN_MAC,
            &knx_testsupport::RETAINED_PRIVACY_UNKNOWN_OWNER,
        ]) {
            assert!(
                !text.contains(planted),
                "{planted} leaked into {}",
                file.name()
            );
        }
        if file.name() == "README.md" {
            readme = text;
        }
    }
    for named in ["network endpoints", "MAC addresses", "user names"] {
        assert!(readme.contains(named), "README must name {named}: {readme}");
    }
}
