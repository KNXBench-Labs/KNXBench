//! Independent selector/provenance and full-caller controls for authored caption packages.
use super::*;
use knx_app::system2_validation_preparation::{
    ProjectProductInput, ProjectProductSelector, ProjectValidationProfile,
};
#[tokio::test]
async fn review_bad_second_candidate_cannot_be_discarded() {
    let input = f::inputs(20, false);
    let bundle = observed(&input.conn, &input.project, &input.profile).await;
    let changed = f::caption(&input.package);
    let input = input.replace(&changed, true);
    let current = knx_productdb::sha256_hex(&changed);
    let old = knx_productdb::sha256_hex(&input.package);
    let affected = input
        .conn
        .execute(
            "UPDATE package SET bytes=x'00' WHERE sha256!=?1 AND sha256!=?2",
            [&current, &old],
        )
        .unwrap();
    assert_eq!(affected, 1);
    let source: String = input
        .conn
        .query_row(
            "SELECT source_sha256 FROM application_program WHERE id=?1",
            [f::PROGRAM],
            |r| r.get(0),
        )
        .unwrap();
    let count: i64 = input
        .conn
        .query_row(
            "SELECT COUNT(DISTINCT package_sha256) FROM package_member WHERE source_sha256=?1",
            [&source],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 2);
    assert_eq!(parts(&input.conn, &input.project, &bundle), None);
}
#[tokio::test]
async fn review_qualified_caption_and_namespace_change_full_caller_refuse() {
    for retained in [false, true] {
        for attack in 0..2 {
            let input = f::inputs(20, retained);
            let bundle = observed(&input.conn, &input.project, &input.profile).await;
            let changed = f::changed(&input.package, |s| {
                if attack == 0 {
                    s.replace(
                        "Name=\"Synthetic\"",
                        "xmlns:x=\"urn:x\" x:Name=\"Revised caption\"",
                    )
                } else {
                    s.replace("Name=\"Synthetic\"", "Name=\"Revised caption\"")
                        .replace("project/20", "project/23")
                }
            });
            let input = input.replace(&changed, false);
            assert_eq!(
                parts(&input.conn, &input.project, &bundle),
                None,
                "{retained}/{attack}"
            );
        }
    }
}
#[tokio::test]
async fn review_changed_source_extra_application_version_refuses() {
    for retained in [false, true] {
        let input = f::inputs(20, retained);
        let bundle = observed(&input.conn, &input.project, &input.profile).await;
        let changed = f::changed(&input.package, |s| {
            let revised = s.replace("Name=\"Synthetic\"", "Name=\"Revised caption\"");
            assert!(revised.contains("ApplicationVersion=\"4\""));
            revised.replace("ApplicationVersion=\"4\"", "ApplicationVersion=\"5\"")
        });
        let input = input.replace(&changed, false);
        assert_eq!(parts(&input.conn, &input.project, &bundle), None);
    }
}
fn orphan_package(good: &[u8]) -> Vec<u8> {
    let tiny = f::package(
        &[("unindexed-plugin.bin".into(), b"unknown bytes".to_vec())],
        "",
    );
    let e = tiny.len() - 22;
    let c = u32::from_le_bytes(tiny[e + 16..e + 20].try_into().unwrap()) as usize;
    let mut bad = good.to_vec();
    let e = bad.len() - 22;
    let start = u32::from_le_bytes(bad[e + 16..e + 20].try_into().unwrap()) as usize;
    bad.splice(start..start, tiny[..c].iter().copied());
    let e = bad.len() - 22;
    bad[e + 16..e + 20].copy_from_slice(&((start + c) as u32).to_le_bytes());
    bad
}
#[tokio::test]
async fn review_added_unindexed_local_member_should_refuse() {
    let input = f::inputs(20, false);
    let bundle = observed(&input.conn, &input.project, &input.profile).await;
    let changed = orphan_package(&f::caption(&input.package));
    let input = input.replace(&changed, false);
    assert_eq!(
        parts(&input.conn, &input.project, &bundle),
        None,
        "a complete physical inventory must not silently omit an added local file record"
    );
}

#[tokio::test]
async fn review_original_exact_pin_with_unindexed_bytes_keeps_strict_success() {
    let input = f::inputs(20, false);
    let original_package = orphan_package(&input.package);
    let mut input = input.replace(&original_package, false);
    let mut profile: ProjectValidationProfile = serde_json::from_slice(&input.profile).unwrap();
    profile.product_input = ProjectProductSelector::StandalonePackage {
        package_sha256: knx_productdb::sha256_hex(&original_package),
    };
    input.profile = serde_json::to_vec(&profile).unwrap();
    let bundle = observed(&input.conn, &input.project, &input.profile).await;
    match &bundle.project_preparation.as_ref().unwrap().product_input {
        ProjectProductInput::StandalonePackage { package_bytes, .. } => {
            assert_eq!(package_bytes, &original_package);
        }
        _ => panic!("fixture must really capture the original standalone container"),
    }
    admit_current_project_hardware(&input.conn, &input.project, f::DEVICE, &bundle).unwrap();
    assert_eq!(
        parts(&input.conn, &input.project, &bundle),
        Some(ProgrammingParts::ALL)
    );
}
