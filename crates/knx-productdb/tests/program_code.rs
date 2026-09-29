//! `code::load_program_code` against a real product file (ADR-0044).
//!
//! The private corpus is not in the repository; like the other corpus tests
//! this one is `#[ignore]`d and runs with `KNXBENCH_PRODUCT_CORPUS` (default:
//! `OriginalData/ProductDatabases` of the root checkout).

use knx_core::commissioning::memory_download::MemoryDownloadStep;
use knx_core::{GroupAddress, IndividualAddress};
use knx_productdb::code::{load_program_code, LoadStep, TablePlacement};
use knx_productdb::download_plan::plan_memory_download;
use knx_productdb::image::{build_download_image, ImageRequest, Link};
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

/// `AS-4400` of `1.1.67` (`A-0027-15-0BAC`) as read back from the device on
/// 2026-09-28 (read-only; docs/IMPLEMENTATION_STATUS.md), 394 octets: its
/// group object table and parameters for two shutter buttons and a status
/// LED.
const DEVICE_AS_4400: &str = concat!(
    "40070007404f0007484f000750db000758db000760db000744db03074cdb0307",
    "54db00075cdb000761db000762db000764db000766db000763db000765db0007",
    "67db000768db000769db00076ad700076bdb00076cdb00076ddb00076edb0007",
    "6fdb000770db000771db000772db000773db000774db000775db000776db0007",
    "77db000778db000779db00077adb00077bdb00077cdb00077ddb00077edb0007",
    "7fdb000780db000781db000782db000783db000784db000785db000786db0007",
    "87db000788db000789db00078adb00078bdb00078cdb00078ddb00078edb0007",
    "8fdb000790db000791db000792db000793db000794db000795db000796db0007",
    "97db000000320190000200000000000001000000010000000100000000000000",
    "0000000000000007000001000100000000000000000000020001000000000100",
    "0000010000000100000000000000000000000000000700000100010000000000",
    "000000000101ff0000000000000000ff00000000000000000000010001060001",
    "00000000000000000200",
);

fn hex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).expect("hex"))
        .collect()
}

fn installed() -> (tempfile::TempDir, rusqlite::Connection) {
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
    (dir, conn)
}

fn image_request(values: &[(&str, &str)], links: Vec<Link>) -> ImageRequest {
    ImageRequest {
        program_id: PROGRAM.to_string(),
        individual_address: IndividualAddress::new(1, 1, 67).expect("valid"),
        values: values
            .iter()
            .map(|(short, value)| (format!("{PROGRAM}_{short}"), value.to_string()))
            .collect(),
        links,
        flag_overrides: Default::default(),
    }
}

fn link(object: u8, raw: u16, sending: bool) -> Link {
    Link {
        object,
        group_address: GroupAddress::from_raw(raw),
        sending,
    }
}

/// The device's configuration as ETS left it: buttons 1/2 as one shutter
/// pair on 2/1/15 and 2/1/16, the LED object 18 on 0/4/6 and 0/4/7. The
/// five values are the ones that differ from the product defaults.
const DEVICE_VALUES: &[(&str, &str)] = &[
    ("P-1007_R-1007", "1"),
    ("P-1014_R-1014", "2"),
    ("P-27_R-27", "2"),
    ("P-326_R-326", "1"),
    ("P-95_R-95", "1"),
];

#[test]
#[ignore = "requires the private product corpus; set KNXBENCH_PRODUCT_CORPUS"]
fn the_image_rebuilds_the_devices_parameter_segment_octet_for_octet() {
    let (_dir, conn) = installed();
    let links = vec![
        link(0, 0x110F, true),
        link(1, 0x1110, true),
        link(18, 0x0406, true),
        link(18, 0x0407, false),
    ];
    let image = build_download_image(&conn, &image_request(DEVICE_VALUES, links)).expect("builds");

    let parameters = &image
        .segment(&segment_id("AS-4400"))
        .expect("AS-4400")
        .octets;
    let device = hex(DEVICE_AS_4400);
    let differing: Vec<usize> = (0..device.len())
        .filter(|&i| parameters[i] != device[i])
        .collect();
    assert_eq!(
        differing,
        Vec::<usize>::new(),
        "octets differing from the device"
    );

    // The tables, as far as they reach; the device keeps stale octets
    // after them (docs/RESEARCH.md §19.1).
    let addresses = &image
        .segment(&segment_id("AS-4000"))
        .expect("AS-4000")
        .octets;
    assert_eq!(&addresses[..11], &hex("05114304060407110f1110")[..]);
    let associations = &image
        .segment(&segment_id("AS-4201"))
        .expect("AS-4201")
        .octets;
    assert_eq!(&associations[..9], &hex("040300040101120212")[..]);
}

#[test]
#[ignore = "requires the private product corpus; set KNXBENCH_PRODUCT_CORPUS"]
fn option_c_toggles_2_0_53_from_button_1_and_changes_only_what_it_must() {
    let (_dir, conn) = installed();
    // Button 1 a single toggle switch, button 2 inactive; 2/0/53 = 1035h.
    let request = image_request(
        &[
            ("P-1007_R-1007", "2"),
            ("UP-5500_R-5500", "0"),
            ("UP-5501_R-5501", "1"),
        ],
        vec![link(0, 0x1035, true)],
    );
    let image = build_download_image(&conn, &request).expect("builds");

    let numbers: Vec<u8> = image.objects.iter().map(|object| object.number).collect();
    assert_eq!(numbers, vec![0, 1], "Switch and Value for toggle");

    let parameters = &image
        .segment(&segment_id("AS-4400"))
        .expect("AS-4400")
        .octets;
    let device = hex(DEVICE_AS_4400);
    let changed: Vec<(usize, u8)> = (0..device.len())
        .filter(|&i| parameters[i] != device[i])
        .map(|i| (i, parameters[i]))
        .collect();
    assert_eq!(
        changed,
        vec![
            (9, 0xD7),   // object 1: now written by the bus, no longer sending
            (77, 0xDB),  // object 18 (LED) inactive
            (265, 0x00), // button 1: switch
            (267, 0x01), // subfunction: toggle
            (311, 0xFF), // button 2: inactive
            (313, 0x04),
            (381, 0x00),
            (392, 0x00),
        ]
    );

    let addresses = &image
        .segment(&segment_id("AS-4000"))
        .expect("AS-4000")
        .octets;
    assert_eq!(&addresses[..5], &hex("0211431035")[..]);
    let associations = &image
        .segment(&segment_id("AS-4201"))
        .expect("AS-4201")
        .octets;
    assert_eq!(&associations[..3], &hex("010100")[..]);
}

#[test]
#[ignore = "requires the private product corpus; set KNXBENCH_PRODUCT_CORPUS"]
fn option_c_plans_the_products_own_procedure_and_keeps_the_individual_address() {
    let (_dir, conn) = installed();
    let request = image_request(
        &[
            ("P-1007_R-1007", "2"),
            ("UP-5500_R-5500", "0"),
            ("UP-5501_R-5501", "1"),
        ],
        vec![link(0, 0x1035, true)],
    );
    let image = build_download_image(&conn, &request).expect("builds");
    let plan = plan_memory_download(&image).expect("plans");
    let steps: Vec<String> = plan.steps.iter().map(ToString::to_string).collect();
    assert_eq!(
        steps,
        [
            "connect; check mask and manufacturer",
            "compare property 0/78 with 00 00 00 00 01 27 00 00 00 00",
            "A_Memory_Write 0104h: 14 00 00 00 00 00 00 00 00 00 00 (unload address table)",
            "A_Memory_Write 0104h: 24 00 00 00 00 00 00 00 00 00 00 (unload association table)",
            "A_Memory_Write 0104h: 34 00 00 00 00 00 00 00 00 00 00 (unload application program)",
            "A_Memory_Write 0104h: 11 00 00 00 00 00 00 00 00 00 00 (load address table)",
            "A_Memory_Write 0104h: 13 00 00 40 00 02 01 FF 03 80 00 (segment address table)",
            // 4001h-4002h, the individual address, are masked out.
            "A_Memory_Write 4000h..4000h, 1 octets",
            "A_Memory_Write 4003h..4200h, 510 octets",
            "A_Memory_Write 0104h: 13 02 00 40 00 00 00 00 00 00 00 (segment address table)",
            "A_Memory_Write 0104h: 12 00 00 00 00 00 00 00 00 00 00 (load completed address table)",
            "A_Memory_Write 0104h: 21 00 00 00 00 00 00 00 00 00 00 (load association table)",
            "A_Memory_Write 0104h: 23 00 00 42 01 01 FF FF 03 80 00 (segment association table)",
            "A_Memory_Write 4201h..43FFh, 511 octets",
            "A_Memory_Write 0104h: 23 02 00 42 01 00 00 00 00 00 00 (segment association table)",
            "A_Memory_Write 0104h: 22 00 00 00 00 00 00 00 00 00 00 (load completed association table)",
            "A_Memory_Write 0104h: 31 00 00 00 00 00 00 00 00 00 00 (load application program)",
            "A_Memory_Write 0104h: 33 00 00 07 00 00 98 00 02 00 00 (segment application program)",
            "A_Memory_Write 0104h: 33 01 00 07 98 00 01 00 02 00 00 (segment application program)",
            "A_Memory_Write 0104h: 33 00 00 44 00 01 8A FF 03 80 00 (segment application program)",
            "A_Memory_Write 4400h..4589h, 394 octets",
            "A_Memory_Write 0104h: 33 02 00 44 00 01 00 83 00 27 15 (segment application program)",
            "A_Memory_Write 0104h: 32 00 00 00 00 00 00 00 00 00 00 (load completed application program)",
            "A_Restart (basic)",
            "disconnect",
        ]
    );
    assert_eq!(plan.data_octets(), 1 + 510 + 511 + 394);

    let parameters = &image
        .segment(&segment_id("AS-4400"))
        .expect("AS-4400")
        .octets;
    let written = plan.steps.iter().find_map(|step| match step {
        MemoryDownloadStep::WriteMemory {
            address: 0x4400,
            octets,
        } => Some(octets),
        _ => None,
    });
    assert_eq!(
        written,
        Some(parameters),
        "the parameter segment goes out as built"
    );
}
