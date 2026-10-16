//! Matches relative source metadata and bytes against an explicitly configured private oracle.

use std::collections::BTreeMap;
use std::path::PathBuf;

use knx_productdb::code::load_program_code;
use knx_productdb::image::{build_download_image, build_system2_image, ImageError, ImageRequest};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    package: PathBuf,
    program_id: String,
    relative_segments: Vec<ExpectedSegment>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpectedSegment {
    id: String,
    offset: u32,
    size: u32,
    load_state_machine: u8,
    other: BTreeMap<String, String>,
    data_sha256: Option<String>,
    mask_sha256: Option<String>,
}

#[test]
#[ignore = "requires KNXBENCH_RELATIVE_SOURCE_CASES private source/oracle manifest; no hardware"]
fn configured_relative_sources_match_independent_oracles_and_stay_not_executable() {
    let input = PathBuf::from(std::env::var_os("KNXBENCH_RELATIVE_SOURCE_CASES").expect(
        "set KNXBENCH_RELATIVE_SOURCE_CASES to an explicit private source/oracle manifest",
    ));
    assert!(std::fs::metadata(&input).unwrap().len() <= 64 * 1024);
    let cases: Vec<Case> = serde_json::from_slice(&std::fs::read(input).unwrap()).unwrap();
    assert!(!cases.is_empty() && cases.len() <= 16);
    let directory = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&directory.path().join("products.sqlite")).unwrap();
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
            "configured-private-source.knxprod",
            &bytes,
            knx_productdb::PackageLimits::LARGE,
        )
        .unwrap();
        assert_eq!(report.sha256, digest);
        packages.insert(case.package.clone(), digest);
    }
    let before = conn.total_changes();
    for case in &cases {
        assert!(!case.relative_segments.is_empty());
        let code = load_program_code(&conn, &case.program_id).unwrap().unwrap();
        assert_eq!(code.relative_segments.len(), case.relative_segments.len());
        for (actual, expected) in code.relative_segments.iter().zip(&case.relative_segments) {
            assert_eq!(actual.id, expected.id);
            assert_eq!(actual.offset, expected.offset);
            assert_eq!(actual.size, expected.size);
            assert_eq!(actual.load_state_machine, expected.load_state_machine);
            assert_eq!(actual.other, expected.other);
            assert_eq!(
                actual.data.as_ref().map(|v| knx_productdb::sha256_hex(v)),
                expected.data_sha256
            );
            assert_eq!(
                actual.mask.as_ref().map(|v| knx_productdb::sha256_hex(v)),
                expected.mask_sha256
            );
        }
        let request = ImageRequest {
            program_id: case.program_id.clone(),
            individual_address: knx_core::IndividualAddress::new(1, 1, 1).unwrap(),
            values: BTreeMap::new(),
            links: Vec::new(),
            flag_overrides: BTreeMap::new(),
        };
        for build in [build_download_image, build_system2_image] {
            assert!(
                matches!(build(&conn, &request), Err(ImageError::Unsupported { what }) if what == "relative source segments require a relative image builder")
            );
        }
    }
    assert_eq!(conn.total_changes(), before);
    for (path, digest) in &packages {
        assert_eq!(
            &knx_productdb::sha256_hex(&std::fs::read(path).unwrap()),
            digest
        );
    }
    println!(
        "configured relative sources verified: {} in {} package(s); no executable images",
        cases.len(),
        packages.len()
    );
}
