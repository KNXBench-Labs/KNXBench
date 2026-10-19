//! Checks explicit private System-1 source refusals without admitting device downloads.

use std::collections::BTreeMap;
use std::path::PathBuf;

use knx_productdb::download_plan::{check_program_kind, DownloadPlanError};
use knx_productdb::image::{build_system1_image, ImageError, ImageRequest};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    package: PathBuf,
    program_id: String,
    source_sha256: String,
    expected_refusal: Refusal,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Refusal {
    FixupList,
    LegacyPatch,
    DefaultBits,
    MidOctetUnion,
}

impl Refusal {
    fn matches(&self, error: &ImageError) -> bool {
        let ImageError::Unsupported { what } = error else {
            return false;
        };
        match self {
            Self::FixupList => {
                what == "System1 FixupList requires a separate source interpretation"
            }
            Self::LegacyPatch => {
                what == "System1 LegacyPatchAlways requires a separate source interpretation"
            }
            Self::DefaultBits => what.ends_with(": System1 default disagrees with source bits"),
            Self::MidOctetUnion => {
                what.ends_with(": a union member offset inside a union that starts mid-octet")
            }
        }
    }
}

#[test]
#[ignore = "requires KNXBENCH_SYSTEM1_SOURCE_CASES private source/refusal manifest; no hardware"]
fn configured_system1_sources_keep_named_refusals_and_public_download_boundaries() {
    let path = PathBuf::from(
        std::env::var_os("KNXBENCH_SYSTEM1_SOURCE_CASES")
            .expect("explicit private source manifest required"),
    );
    assert!(std::fs::metadata(&path).unwrap().len() <= 65536);
    let cases: Vec<Case> = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert!(!cases.is_empty() && cases.len() <= 16);
    let temp = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&temp.path().join("products.sqlite")).unwrap();
    let mut packages = BTreeMap::new();
    for case in &cases {
        if packages.contains_key(&case.package) {
            continue;
        }
        assert!(
            std::fs::metadata(&case.package).unwrap().len()
                <= knx_productdb::MAX_PACKAGE_INPUT_BYTES
        );
        let bytes = std::fs::read(&case.package).unwrap();
        let digest = knx_productdb::sha256_hex(&bytes);
        let report = knx_productdb::install_package_with_limits(
            &conn,
            "configured-system1-source.knxprod",
            &bytes,
            knx_productdb::PackageLimits::LARGE,
        )
        .unwrap();
        assert_eq!(report.sha256, digest);
        packages.insert(case.package.clone(), digest);
    }
    let before = conn.total_changes();
    for case in &cases {
        let hash: String = conn
            .query_row(
                "SELECT source_sha256 FROM application_program WHERE id = ?1",
                [&case.program_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(hash, case.source_sha256);
        let code = knx_productdb::code::load_program_code(&conn, &case.program_id)
            .unwrap()
            .unwrap();
        assert_eq!(code.mask_version.as_deref(), Some("MV-0012"));
        assert_eq!(
            check_program_kind(&code),
            Err(DownloadPlanError::NotMemoryMapped(Some("MV-0012".into())))
        );
        let request = ImageRequest {
            program_id: case.program_id.clone(),
            individual_address: knx_core::IndividualAddress::from_raw(0x1234),
            values: Default::default(),
            links: Vec::new(),
            flag_overrides: Default::default(),
        };
        let result = build_system1_image(&conn, &request);
        let Err(error) = result else {
            panic!("configured unsupported source unexpectedly returned an image");
        };
        assert!(
            case.expected_refusal.matches(&error),
            "unexpected configured refusal: {error}"
        );
    }
    assert_eq!(conn.total_changes(), before);
    for (path, digest) in &packages {
        assert_eq!(
            knx_productdb::sha256_hex(&std::fs::read(path).unwrap()),
            *digest
        );
    }
    println!(
        "configured System1 source refusals verified: {} in {} package(s); no executable images",
        cases.len(),
        packages.len()
    );
}
