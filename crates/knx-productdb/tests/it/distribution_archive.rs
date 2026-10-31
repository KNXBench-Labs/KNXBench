//! Verifies logical Windows distribution names reuse native checked ZIP structure and decoding.

use crate::distribution_inspection::*;
use crate::standalone_packages::wp16::{raw_zip, RawMember};
use knx_productdb::distribution::{
    inspect_distribution_with_limits, DistributionInspectionError as E, DistributionLimits as L,
};

#[test]
fn windows_distribution_is_positive_without_weakening_native_package_rejection() {
    let bytes = raw_zip(&[
        RawMember::stored(b"data\\product.xml", COMBINED),
        RawMember::stored(b"data\\inert.js", b"never execute me"),
    ]);
    let original = bytes.clone();
    let result = inspect_distribution("arbitrary", &bytes);
    assert!(
        result.is_ok(),
        "checked logical Windows paths must be positively inspectable: {result:?}"
    );
    let r = result.unwrap();
    assert_eq!(r.source_kind, DistributionSourceKind::DistributionZip);
    assert_eq!(r.source.sha256, knx_productdb::sha256_hex(&bytes));
    assert_eq!(r.members.len(), 2);
    assert_eq!(r.members[0].raw_name, b"data\\product.xml");
    assert_eq!(r.members[0].decoded_name, "data\\product.xml");
    assert_eq!(r.members[0].logical_path, "data/product.xml");
    assert_eq!(r.members[0].ordinal, 0);
    assert_eq!(r.members[0].size, COMBINED.len() as u64);
    assert_eq!(r.members[0].sha256, knx_productdb::sha256_hex(COMBINED));
    assert_eq!(r.sections.len(), 4);
    assert_eq!(r.applications[0].active_parameter_ref_ids, ["R-Control"]);
    assert!(r
        .sections
        .iter()
        .all(|s| s.source.member_ordinal == Some(0)));
    assert_eq!(bytes, original);
    let db = knx_productdb::open_and_migrate(std::path::Path::new(":memory:")).unwrap();
    assert!(matches!(
        knx_productdb::install_package(&db, "source.zip", &bytes),
        Err(knx_productdb::PackageError::UnsafeMember { .. })
    ));
    assert_eq!(
        db.query_row("SELECT count(*) FROM package", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
}

#[test]
fn malformed_optional_manifest_does_not_repair_or_hide_the_valid_product_member() {
    let mut conflict = vec![0xff, 0xfe];
    conflict.extend(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><content/>"
            .encode_utf16()
            .flat_map(u16::to_le_bytes),
    );
    let bytes = raw_zip(&[
        RawMember::stored(b"data/content.xml", &conflict),
        RawMember::stored(b"data/product.xml", COMBINED),
        RawMember::stored(b"script.sh", b"exit 42"),
    ]);
    let r = inspect_distribution("mixed", &bytes).unwrap();
    assert_eq!(r.members.len(), 3);
    assert_eq!(r.members[0].error, Some(E::EncodingConflict));
    assert_eq!(r.members[0].sha256, knx_productdb::sha256_hex(&conflict));
    assert_eq!(r.applications.len(), 1);
    assert_eq!(r.applications[0].source.member_ordinal, Some(1));
    assert!(!r.complete);
}

#[test]
fn unsafe_logical_paths_collisions_and_structural_contradictions_are_refused() {
    for name in [
        b"../escape".as_slice(),
        b"data\\..\\escape",
        b"/absolute",
        b"\\absolute",
        b"C:\\drive",
        b"\\\\server\\share",
        b"a//b",
        b"a/./b",
        b"a/%2e%2e/b",
        b"a%2fb",
        b"a\0b",
    ] {
        let bytes = raw_zip(&[
            RawMember::stored(b"product.xml", COMBINED),
            RawMember::stored(name, b"inert"),
        ]);
        assert!(
            matches!(inspect_distribution("unsafe", &bytes), Err(E::Archive(_))),
            "{name:?}"
        );
    }
    for names in [
        (b"a/b".as_slice(), b"a\\b".as_slice()),
        (b"a", b"a/b"),
        (b"same", b"same"),
    ] {
        let bytes = raw_zip(&[
            RawMember::stored(b"product.xml", COMBINED),
            RawMember::stored(names.0, b"a"),
            RawMember::stored(names.1, b"b"),
        ]);
        assert!(matches!(
            inspect_distribution("collision", &bytes),
            Err(E::Archive(_))
        ));
    }
    let mut bytes = raw_zip(&[RawMember::stored(b"product.xml", COMBINED)]);
    bytes[30] = b'x';
    assert!(matches!(
        inspect_distribution("contradiction", &bytes),
        Err(E::Archive(_))
    ));
    let mut symlink = raw_zip(&[RawMember::stored(b"product.xml", COMBINED)]);
    let central = symlink.windows(4).position(|b| b == b"PK\x01\x02").unwrap();
    symlink[central + 5] = 3;
    symlink[central + 38..central + 42].copy_from_slice(&(0o120777u32 << 16).to_le_bytes());
    assert!(matches!(
        inspect_distribution("symlink", &symlink),
        Err(E::Archive(_))
    ));
}

#[test]
fn crc_and_deflate_extent_and_directory_data_are_checked_before_success() {
    let mut corrupted = raw_zip(&[RawMember::stored(b"product.xml", COMBINED)]);
    corrupted[30 + b"product.xml".len()] = b'!';
    assert!(matches!(
        inspect_distribution("crc", &corrupted),
        Err(E::Archive(_))
    ));
    let bytes = raw_zip(&[
        RawMember::stored(b"product.xml", COMBINED),
        RawMember::directory(b"data\\empty\\", b"\x03\x00hidden"),
    ]);
    assert!(matches!(
        inspect_distribution("trailing", &bytes),
        Err(E::Archive(_))
    ));
    let bytes = raw_zip(&[
        RawMember::stored(b"product.xml", COMBINED),
        RawMember::stored(b"data\\directory\\", b"hidden"),
    ]);
    assert!(matches!(
        inspect_distribution("dirdata", &bytes),
        Err(E::Archive(_))
    ));
    let bytes = raw_zip(&[
        RawMember::stored(b"product.xml", COMBINED),
        RawMember::stored(b"data\\empty\\", b""),
    ]);
    assert!(inspect_distribution("emptydir", &bytes).is_ok());
}

#[test]
fn unix_special_file_modes_are_not_regular_distribution_sources() {
    for mode in [0o010644u32, 0o020644, 0o060644, 0o140644] {
        let mut bytes = raw_zip(&[RawMember::stored(b"product.xml", COMBINED)]);
        let central = bytes.windows(4).position(|b| b == b"PK\x01\x02").unwrap();
        bytes[central + 5] = 3;
        bytes[central + 38..central + 42].copy_from_slice(&(mode << 16).to_le_bytes());
        assert!(
            matches!(inspect_distribution("special", &bytes), Err(E::Archive(_))),
            "special Unix file type must be refused: {mode:o}"
        );
    }
}

#[test]
fn archive_expansion_and_member_limits_are_inclusive_and_typed() {
    let bytes = raw_zip(&[
        RawMember::stored(b"product.xml", COMBINED),
        RawMember::stored(b"inert.bin", b"1234"),
    ]);
    let mut l = L {
        max_expanded_bytes: COMBINED.len() as u64 + 3,
        ..L::default()
    };
    assert!(matches!(
        inspect_distribution_with_limits("budget", &bytes, l),
        Err(E::ResourceLimit(_))
    ));
    l.max_expanded_bytes += 1;
    assert!(inspect_distribution_with_limits("inclusive", &bytes, l).is_ok());
    l.max_member_bytes = COMBINED.len() as u64 - 1;
    assert!(matches!(
        inspect_distribution_with_limits("member", &bytes, l),
        Err(E::ResourceLimit(_))
    ));
}
