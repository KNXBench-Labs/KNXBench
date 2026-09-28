//! `code::load_program_code` against a real product file (ADR-0044).
//!
//! The private corpus is not in the repository; like the other corpus tests
//! this one is `#[ignore]`d and runs with `KNXBENCH_PRODUCT_CORPUS` (default:
//! `OriginalData/ProductDatabases` of the root checkout).

use knx_productdb::code::{load_program_code, LoadStep, TablePlacement};
use knx_productdb::{install_package, open_and_migrate, sha256_hex};

const FILE: &str = "MDT_KP_BE_01_Push_Button_V15a.knxprod";
const PROGRAM: &str = "M-0083_A-0027-15-0BAC";

fn segment_id(short: &str) -> String {
    format!("{PROGRAM}_{short}")
}

#[test]
#[ignore = "requires the private product corpus; set KNXBENCH_PRODUCT_CORPUS"]
fn mdt_push_button_download_data_is_read_whole_from_the_stored_file() {
    let root = std::env::var_os("KNXBENCH_PRODUCT_CORPUS")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../OriginalData/ProductDatabases")
        });
    let path = knx_testsupport::find_corpus_file(&root, FILE).unwrap_or_else(|| {
        panic!(
            "corpus fixture {FILE} unavailable under {}; set KNXBENCH_PRODUCT_CORPUS",
            root.display()
        )
    });
    let bytes = std::fs::read(&path).expect("readable fixture");
    let dir = tempfile::tempdir().expect("tempdir");
    let conn = open_and_migrate(&dir.path().join("products.sqlite")).expect("database");
    install_package(&conn, FILE, &bytes).expect("installs");

    let code = load_program_code(&conn, PROGRAM)
        .expect("readable")
        .expect("the program is installed");

    assert_eq!(code.mask_version.as_deref(), Some("MV-0701"));
    assert_eq!(
        code.load_procedure_style.as_deref(),
        Some("ProductProcedure")
    );
    assert_eq!(
        code.options
            .get("LegacyAllowPartialDownloadIfAp2Mismatch")
            .map(String::as_str),
        Some("true")
    );
    assert_eq!(
        code.options.len(),
        1,
        "no ParameterByteOrder: {:?}",
        code.options
    );

    let layout: Vec<(String, u32, u32, bool, bool)> = code
        .segments
        .iter()
        .map(|s| {
            (
                s.id.clone(),
                s.address,
                s.size,
                s.data.is_some(),
                s.mask.is_some(),
            )
        })
        .collect();
    assert_eq!(
        layout,
        vec![
            (segment_id("AS-4000"), 0x4000, 513, true, true),
            (segment_id("AS-4201"), 0x4201, 511, true, false),
            (segment_id("AS-0700"), 0x0700, 152, false, false),
            (segment_id("AS-0798"), 0x0798, 1, false, false),
            (segment_id("AS-4400"), 0x4400, 394, true, false),
        ]
    );

    // The base images, pinned by content hash. AS-4400's first 259 octets
    // are the group object table `knx-core`'s `group_object_table` rebuilds
    // a real device's read-back from, octet for octet.
    let data = |short: &str| {
        code.segment(&segment_id(short))
            .and_then(|s| s.data.as_deref())
            .expect("has data")
    };
    assert_eq!(&sha256_hex(data("AS-4000"))[..16], "99bd7093e75e38b9");
    assert_eq!(&sha256_hex(data("AS-4201"))[..16], "86a1ffb5ee9cdb55");
    assert_eq!(&sha256_hex(data("AS-4400"))[..16], "4a66c9550a882268");
    assert_eq!(&data("AS-4400")[..4], &[0x40, 0x07, 0x00, 0x07]);

    // The one Mask marks exactly octets 1–2 of the group address table: the
    // slot a mask-0701h device keeps its own individual address in.
    let mask = code
        .segment(&segment_id("AS-4000"))
        .and_then(|s| s.mask.as_deref())
        .expect("AS-4000 has a mask");
    let marked: Vec<usize> = (0..mask.len()).filter(|&i| mask[i] != 0xFF).collect();
    assert_eq!(marked, vec![1, 2]);

    let placement = |segment: &str, max: Option<u32>| TablePlacement {
        code_segment: Some(segment_id(segment)),
        offset: Some(0),
        max_entries: max,
    };
    assert_eq!(code.address_table, Some(placement("AS-4000", Some(255))));
    assert_eq!(
        code.association_table,
        Some(placement("AS-4201", Some(255)))
    );
    assert_eq!(code.com_object_table, Some(placement("AS-4400", None)));

    assert_eq!(code.load_procedures.len(), 1);
    let procedure = &code.load_procedures[0];
    assert_eq!(procedure.merge_id, None);
    assert_eq!(procedure.unmodelled().count(), 0, "{:?}", procedure.steps);
    let abs = |lsm, segment_type, address, size, access, memory_type, flags| LoadStep::AbsSegment {
        lsm,
        segment_type,
        address,
        size,
        access,
        memory_type,
        flags,
    };
    assert_eq!(
        procedure.steps,
        vec![
            LoadStep::Connect,
            LoadStep::CompareProp {
                object_index: 0,
                property_id: 78,
                data: vec![0, 0, 0, 0, 0x01, 0x27, 0, 0, 0, 0],
            },
            LoadStep::Unload { lsm: 1 },
            LoadStep::Unload { lsm: 2 },
            LoadStep::Unload { lsm: 3 },
            LoadStep::Load { lsm: 1 },
            abs(1, 0, 0x4000, 513, 255, 3, 128),
            LoadStep::TaskSegment {
                lsm: 1,
                address: 0x4000
            },
            LoadStep::LoadCompleted { lsm: 1 },
            LoadStep::Load { lsm: 2 },
            abs(2, 0, 0x4201, 511, 255, 3, 128),
            LoadStep::TaskSegment {
                lsm: 2,
                address: 0x4201
            },
            LoadStep::LoadCompleted { lsm: 2 },
            LoadStep::Load { lsm: 3 },
            abs(3, 0, 0x0700, 152, 0, 2, 0),
            abs(3, 1, 0x0798, 1, 0, 2, 0),
            abs(3, 0, 0x4400, 394, 255, 3, 128),
            LoadStep::TaskSegment {
                lsm: 3,
                address: 0x4400
            },
            LoadStep::LoadCompleted { lsm: 3 },
            LoadStep::Restart,
            LoadStep::Disconnect,
        ]
    );
}

#[test]
fn an_unknown_program_is_none_not_an_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    let conn = open_and_migrate(&dir.path().join("products.sqlite")).expect("database");
    assert!(load_program_code(&conn, "M-0000_A-NONE")
        .expect("no error")
        .is_none());
}
