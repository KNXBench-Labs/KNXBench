//! Acceptance for L4: a legacy program downloads like ETS's own conversion of it (ADR-0094).
//!
//! Two oracles, both private corpus files:
//!
//! 1. **The house.** `N000520_IRBM_20` from the Eibmarkt `.vd4` against ETS
//!    6.3's conversion of the same program in the maintainer's house
//!    project, which runs it on nine presence detectors (1.1.1–1.1.9). The
//!    program code (segments, base images, masks, load procedure, table
//!    placements, identity, parameter memory) must be equal, and for each
//!    of the nine devices, with the project's own values and links, the
//!    download plan built from the legacy program must equal the one built
//!    from ETS's conversion step for step and octet for octet.
//! 2. **Siemens.** Every `070nh` program of the November 2016 `.vd5` that
//!    the ETS4 package of the same database also holds (same manufacturer,
//!    application number and version) must yield the same program code.
//!
//! Differences are printed by id and field, never with product content.
//! `#[ignore]`d: run with `KNXBENCH_PRODUCT_CORPUS` and
//! `KNXBENCH_VD_PASSWORD_FILE` (and the house project under
//! `OriginalData/DemoProjects`).

use std::collections::BTreeMap;
use std::path::PathBuf;

use knx_app::legacy::{import_legacy_file, LegacyPassword};
use knx_core::commissioning::memory_download::MemoryDownloadStep;
use knx_productdb::code::{load_program_code, ParameterPlacement, ProgramCode};
use knx_productdb::download_plan::plan_memory_download;
use knx_productdb::image::{build_download_image, ImageRequest};
use knx_productdb::image_request::image_request_for_device;
use knx_productdb::Connection;

const VD4: &str = "Eibmarkt Motion Sensor N520_IRBM_N530_IRBM.vd4";
const VD5: &str = "SIEMENS_KNX_PDB_Nov_2016_ETS3.vd5";
const ETS4: &str = "SIEMENS_KNX_PDB_Nov_2016_ETS4.knxprod";
const HOUSE_PROGRAM: &str = "M-006A_A-0001-22-617E-O0079";
const VD4_PROGRAM: &str = "63558";
const PRESENCE_DETECTORS: [&str; 9] = [
    "1.1.1", "1.1.2", "1.1.3", "1.1.4", "1.1.5", "1.1.6", "1.1.7", "1.1.8", "1.1.9",
];

fn root() -> PathBuf {
    std::env::var_os("KNXBENCH_PRODUCT_CORPUS")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../OriginalData/ProductDatabases")
        })
}

fn read(name: &str) -> Vec<u8> {
    let path = knx_testsupport::find_corpus_file(&root(), name)
        .unwrap_or_else(|| panic!("SKIP: set KNXBENCH_PRODUCT_CORPUS; {name} unavailable"));
    std::fs::read(path).unwrap()
}

fn password() -> LegacyPassword {
    let path = std::env::var_os("KNXBENCH_VD_PASSWORD_FILE")
        .expect("SKIP: set KNXBENCH_VD_PASSWORD_FILE to the local legacy password file");
    let raw = std::fs::read_to_string(&path).expect("KNXBENCH_VD_PASSWORD_FILE unreadable");
    let line = raw.lines().next().unwrap_or_default().to_string();
    assert!(
        !line.is_empty(),
        "KNXBENCH_VD_PASSWORD_FILE holds no password"
    );
    LegacyPassword::new(line)
}

fn legacy_program(conn: &Connection, exim_program_id: &str) -> String {
    conn.query_row(
        "SELECT program_id FROM legacy_program WHERE exim_program_id = ?1",
        [exim_program_id],
        |r| r.get(0),
    )
    .unwrap()
}

/// Address, size, base image and mask.
type Segment = (u32, u32, Option<Vec<u8>>, Option<Vec<u8>>);

/// A program's code with its ids taken out: segments by address, tables by
/// absolute address, parameters by id suffix and absolute bit.
#[derive(Debug, PartialEq)]
struct Comparable {
    segments: Vec<Segment>,
    steps: Vec<String>,
    tables: Vec<Option<(u32, Option<u32>)>>,
    identity: Vec<Option<String>>,
    /// By parameter number: the import groups a shared address as a union
    /// (`UP-`) where ETS's conversion may not, and the other way round.
    parameters: BTreeMap<String, Result<u32, String>>,
}

fn comparable(code: &ProgramCode) -> Comparable {
    let start = |segment: &str| code.segment(segment).map(|s| s.address);
    let table = |placement: &Option<knx_productdb::code::TablePlacement>| {
        placement.as_ref().map(|p| {
            let at = p
                .code_segment
                .as_deref()
                .and_then(start)
                .unwrap_or(u32::MAX);
            (at + p.offset.unwrap_or(0), p.max_entries)
        })
    };
    let prefix = format!("{}_", code.program_id);
    let mut segments: Vec<_> = code
        .segments
        .iter()
        .map(|s| (s.address, s.size, s.data.clone(), s.mask.clone()))
        .collect();
    segments.sort_by_key(|s| s.0);
    Comparable {
        segments,
        steps: code
            .load_procedures
            .iter()
            .filter(|p| p.merge_id.is_none())
            .flat_map(|p| &p.steps)
            .map(|step| format!("{step:?}"))
            .collect(),
        tables: vec![
            table(&code.address_table),
            table(&code.association_table),
            table(&code.com_object_table),
        ],
        identity: ["ApplicationNumber", "ApplicationVersion", "PeiType"]
            .iter()
            .map(|name| code.program_attributes.get(*name).cloned())
            .chain([code.mask_version.clone(), code.load_procedure_style.clone()])
            .collect(),
        parameters: code
            .parameters
            .iter()
            .map(|(id, placement)| {
                let bit = match placement {
                    ParameterPlacement::Memory(m) => start(&m.code_segment)
                        .map(|s| (s + m.offset) * 8 + u32::from(m.bit_offset))
                        .ok_or_else(|| m.code_segment.clone()),
                    ParameterPlacement::UnionMember {
                        union,
                        offset,
                        bit_offset,
                    } => start(&union.code_segment)
                        .map(|s| {
                            (s + union.offset + offset) * 8
                                + u32::from(union.bit_offset)
                                + u32::from(*bit_offset)
                        })
                        .ok_or_else(|| union.code_segment.clone()),
                    ParameterPlacement::Unmodelled { name, attributes } => {
                        Err(format!("{name} {attributes:?}"))
                    }
                };
                let local = id.strip_prefix(&prefix).unwrap_or(id);
                let number = local.trim_start_matches("UP-").trim_start_matches("P-");
                (number.to_string(), bit)
            })
            .collect(),
    }
}

/// Names every field that differs; empty when equal.
fn differences(
    name: &str,
    ours: &Comparable,
    theirs: &Comparable,
    unmapped: &std::collections::BTreeSet<String>,
) -> Vec<String> {
    let mut out = Vec::new();
    if ours.segments.len() != theirs.segments.len() {
        out.push(format!(
            "{name}: {} segments, ETS {}",
            ours.segments.len(),
            theirs.segments.len()
        ));
    }
    for (o, t) in ours.segments.iter().zip(&theirs.segments) {
        for (field, differs) in [
            ("address/size", (o.0, o.1) != (t.0, t.1)),
            ("data", o.2 != t.2),
            ("mask", o.3 != t.3),
        ] {
            if differs {
                out.push(format!("{name}: segment {:04X}h {field}", t.0));
            }
        }
    }
    if ours.steps != theirs.steps {
        let at = ours
            .steps
            .iter()
            .zip(&theirs.steps)
            .position(|(o, t)| o != t);
        out.push(format!(
            "{name}: procedure ({} steps, ETS {}; first difference at {at:?}: {:?} / {:?})",
            ours.steps.len(),
            theirs.steps.len(),
            at.map(|i| &ours.steps[i]),
            at.map(|i| &theirs.steps[i]),
        ));
    }
    for (table, (o, t)) in ["address", "association", "group object"]
        .iter()
        .zip(ours.tables.iter().zip(&theirs.tables))
    {
        if o != t {
            out.push(format!("{name}: {table} table {o:?}, ETS {t:?}"));
        }
    }
    if ours.identity != theirs.identity {
        out.push(format!(
            "{name}: identity {:?}, ETS {:?}",
            ours.identity, theirs.identity
        ));
    }
    // A parameter number both sides place must sit at the same bit.
    for (id, t) in &theirs.parameters {
        if let Some(o) = ours.parameters.get(id) {
            if o != t {
                out.push(format!("{name}: parameter {id} at {o:?}, ETS {t:?}"));
            }
        }
    }
    // The import names a group of parameters sharing one cell after one
    // member, ETS's conversion sometimes after another (ADR-0094, L2), so
    // numbers placed on one side only are not compared by number. Every
    // placed bit must still be placed on both sides.
    // Parameters of a type the import does not map (atomic types 3 and 5,
    // KNOWN_LIMITATIONS §128) are absent from the legacy program, so ETS
    // alone places them; they are counted, not compared.
    let bits = |c: &Comparable, skip: &std::collections::BTreeSet<String>| {
        c.parameters
            .iter()
            .filter(|(number, _)| !skip.contains(*number))
            .map(|(_, bit)| bit.clone())
            .collect::<std::collections::BTreeSet<_>>()
    };
    let (our_bits, their_bits) = (bits(ours, &Default::default()), bits(theirs, unmapped));
    for bit in their_bits.symmetric_difference(&our_bits) {
        let side = if their_bits.contains(bit) {
            "ETS"
        } else {
            "legacy"
        };
        out.push(format!(
            "{name}: only the {side} program places a parameter at {bit:?}"
        ));
    }
    out
}

/// Every parameter number the legacy program declares, placed or not.
fn numbers(conn: &Connection, program: &str) -> std::collections::BTreeSet<String> {
    let prefix = format!("{program}_");
    conn.prepare("SELECT id FROM parameter WHERE program_id = ?1")
        .unwrap()
        .query_map([program], |r| r.get::<_, String>(0))
        .unwrap()
        .map(|id| {
            let id = id.unwrap();
            let local = id.strip_prefix(&prefix).unwrap_or(&id).to_string();
            local
                .trim_start_matches("UP-")
                .trim_start_matches("P-")
                .to_string()
        })
        .collect()
}

/// `request` for the legacy program: the same values, links and flags, by
/// the same id suffixes.
fn translated(request: &ImageRequest, legacy: &str) -> ImageRequest {
    let from = format!("{}_", request.program_id);
    let to = format!("{legacy}_");
    let rename = |id: &String| id.replacen(&from, &to, 1);
    ImageRequest {
        program_id: legacy.to_string(),
        individual_address: request.individual_address,
        values: request
            .values
            .iter()
            .map(|(k, v)| (rename(k), v.clone()))
            .collect(),
        links: request.links.clone(),
        flag_overrides: request
            .flag_overrides
            .iter()
            .map(|(k, v)| (rename(k), *v))
            .collect(),
    }
}

#[test]
#[ignore = "needs the private product corpus, the house project and the legacy password"]
fn the_houses_presence_detectors_download_the_same_from_the_legacy_program() {
    assert!(
        knx_testsupport::corpus_available(),
        "OriginalData/ corpus not present (gitignored, local-only)"
    );
    let dir = tempfile::tempdir().unwrap();
    let store = knx_store::open_and_migrate(&dir.path().join("p.knxdb")).unwrap();
    let products = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    let project = knx_app::import_ets_project_with(
        &knx_testsupport::reference_ets6_path(),
        &store,
        knx_app::ImportOptions {
            product_db: Some(&products),
        },
    )
    .unwrap()
    .project;
    import_legacy_file(&products, VD4, &read(VD4), Some(&password())).unwrap();
    let legacy = legacy_program(&products, VD4_PROGRAM);

    let ours = load_program_code(&products, &legacy).unwrap().unwrap();
    let theirs = load_program_code(&products, HOUSE_PROGRAM)
        .unwrap()
        .unwrap();
    let wrong = differences(
        "N000520",
        &comparable(&ours),
        &comparable(&theirs),
        &Default::default(),
    );
    assert!(wrong.is_empty(), "{wrong:#?}");

    let mut wrong = Vec::new();
    for address in PRESENCE_DETECTORS {
        let device = project
            .devices
            .iter()
            .find(|d| d.address == Some(address.parse().unwrap()))
            .unwrap_or_else(|| panic!("{address} not in the project"));
        let request = image_request_for_device(&products, &project, device.id)
            .unwrap_or_else(|e| panic!("{address}: {e}"));
        assert_eq!(request.program_id, HOUSE_PROGRAM, "{address}");
        let build = |request: &ImageRequest| {
            let image = build_download_image(&products, request)
                .unwrap_or_else(|e| panic!("{address} {}: {e}", request.program_id));
            let plan = plan_memory_download(&image)
                .unwrap_or_else(|e| panic!("{address} {}: {e}", request.program_id));
            (image, plan)
        };
        let (their_image, theirs) = build(&request);
        let (our_image, ours) = build(&translated(&request, &legacy));
        // Named deviation (L2 deviation 3, docs/research/legacy-vd-mapping.md):
        // the file hangs P-5008 (`brightnessThresholdPIR_1`, 16 bit at
        // 4196h, default 0) on the first page, so the legacy tree activates
        // it and its default is written; ETS's conversion leaves it
        // inactive, and the base image's 03E8h stays, which all nine
        // devices hold (house read-back, RESEARCH §19.13).
        let mut octets = Vec::new();
        for (o, t) in our_image.segments.iter().zip(&their_image.segments) {
            assert_eq!((o.address, o.octets.len()), (t.address, t.octets.len()));
            for i in 0..o.octets.len() {
                if o.octets[i] != t.octets[i] {
                    octets.push((o.address as usize + i, o.octets[i], t.octets[i]));
                }
            }
        }
        if octets != [(0x4196, 0x00, 0x03), (0x4197, 0x00, 0xE8)] {
            wrong.push(format!("{address}: image octets differ at {octets:02X?}"));
        }
        let steps: Vec<usize> = (0..ours.steps.len().min(theirs.steps.len()))
            .filter(|&i| ours.steps[i] != theirs.steps[i])
            .collect();
        let only_that_write = steps.iter().all(|&i| {
            matches!(
                &theirs.steps[i],
                MemoryDownloadStep::WriteMemory { address, octets }
                    if (*address as usize..*address as usize + octets.len()).contains(&0x4196)
            )
        });
        if ours.steps.len() != theirs.steps.len() || steps.len() != 1 || !only_that_write {
            wrong.push(format!("{address}: plan steps {steps:?} differ"));
        }
        if (ours.mask, ours.manufacturer) != (theirs.mask, theirs.manufacturer) {
            wrong.push(format!("{address}: identity"));
        }
        println!(
            "{address}: {} steps, {} data octets; differing octets {octets:02X?}",
            ours.steps.len(),
            ours.data_octets(),
        );
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

#[test]
#[ignore = "needs the private product corpus and the legacy password"]
fn the_siemens_vd5_programs_yield_the_code_of_their_ets4_conversion() {
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    knx_productdb::install_package(&conn, ETS4, &read(ETS4)).unwrap();
    import_legacy_file(&conn, VD5, &read(VD5), Some(&password())).unwrap();

    let pairs: Vec<(String, String, String)> = conn
        .prepare(
            "SELECT l.exim_program_id, l.program_id, e.id
             FROM legacy_program l
             JOIN application_program a ON a.id = l.program_id
             JOIN application_program e
               ON e.manufacturer_id = a.manufacturer_id
              AND e.application_number = a.application_number
              AND e.application_version = a.application_version
              AND e.id NOT IN (SELECT program_id FROM legacy_program)
             WHERE a.mask_version LIKE 'MV-070_'
             ORDER BY CAST(l.exim_program_id AS INTEGER)",
        )
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    let mut wrong = Vec::new();
    let mut unmapped_count = 0;
    for (exim, legacy, ets) in &pairs {
        let ours = load_program_code(&conn, legacy).unwrap().unwrap();
        let theirs = load_program_code(&conn, ets).unwrap().unwrap();
        let (ours, theirs) = (comparable(&ours), comparable(&theirs));
        let imported = numbers(&conn, legacy);
        let unmapped: std::collections::BTreeSet<String> = theirs
            .parameters
            .keys()
            .filter(|number| !imported.contains(*number))
            .cloned()
            .collect();
        unmapped_count += unmapped.len();
        wrong.extend(differences(exim, &ours, &theirs, &unmapped));
    }
    println!(
        "{} 070nh programs with an ETS4 conversion compared; {unmapped_count} placed ETS \
         parameters have no legacy counterpart (unmapped types)",
        pairs.len()
    );
    assert_eq!(
        unmapped_count, 4,
        "string parameters of 24774, 24815 and 24858"
    );
    assert_eq!(pairs.len(), 9, "the measured pairs");
    assert!(wrong.is_empty(), "{wrong:#?}");
}
