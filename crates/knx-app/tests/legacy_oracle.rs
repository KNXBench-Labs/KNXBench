//! Acceptance: a real `.vd4` program evaluates like ETS's own conversion of it (ADR-0094, Q15).
//!
//! Imports `N000520_IRBM_20` from the private Eibmarkt `.vd4` and ingests
//! ETS 6.3's conversion of the same program (`ConvertedFromPreEts4Data`)
//! from the private house project into the same product database, then
//! compares both through KNXBench's own queries and evaluator: per
//! parameter ref the type, range, enumeration values, default, access,
//! memory, display order and texts; per object ref number, size, priority,
//! flags and texts; translations; and the visible refs under the defaults
//! and under every alternative value of every controlling parameter.
//!
//! Differences are printed structurally (ids, field names, numbers), never
//! with manufacturer text, and must equal the named deviations below.
//!
//! Run with `KNXBENCH_PRODUCT_CORPUS`, `KNXBENCH_VD_PASSWORD_FILE` and
//! optionally `KNXBENCH_LEGACY_ORACLE_KNXPROJ` (defaults to the house
//! project under `OriginalData/DemoProjects`).

use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::PathBuf;

use knx_app::legacy::{import_legacy_file, LegacyPassword};
use knx_productdb::device_evaluation::evaluate_device;
use knx_productdb::Connection;

const VD4: &str = "Eibmarkt Motion Sensor N520_IRBM_N530_IRBM.vd4";
const ORACLE_MEMBER: &str = "M-006A/M-006A_A-0001-22-617E-O0079.xml";
const ORACLE_PROGRAM: &str = "M-006A_A-0001-22-617E-O0079";
const VD_PROGRAM: &str = "63558";

/// Named deviations (docs/research/legacy-vd-mapping.md). Each is a
/// measured difference between the file and ETS's conversion that no
/// column of the file explains; they are reported, not adjusted away.
const KNOWN_DEVIATIONS: &[&str] = &[
    // VD access level 0; ETS makes it ReadWrite.
    "param P-5008_R-5008 access: None != ReadWrite",
    // The file translates the program name into en-US; ETS drops that
    // translation. KNXBench keeps it: no data is dropped to look alike.
    "translation en-US extra: 1",
    // The file places 5008 on page 1001 (inline); ETS hangs it under
    // 1008 = 1 / 1009, which its defaults never select. Either way it is
    // not editable (legacy: access None; ETS: inactive).
    "visibility P-5008_R-5008 active only in the legacy tree in 35 of 36 cases",
];

fn root() -> PathBuf {
    std::env::var_os("KNXBENCH_PRODUCT_CORPUS")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../OriginalData/ProductDatabases")
        })
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

fn oracle_xml() -> Vec<u8> {
    let path = std::env::var_os("KNXBENCH_LEGACY_ORACLE_KNXPROJ")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
                "../../OriginalData/DemoProjects/Unser Zuhause ets 6.3.0 - 2026-09-02.knxproj",
            )
        });
    let file = std::fs::File::open(&path).unwrap_or_else(|e| {
        panic!(
            "SKIP: set KNXBENCH_LEGACY_ORACLE_KNXPROJ; {} unreadable: {e}",
            path.display()
        )
    });
    let mut archive = zip::ZipArchive::new(file).expect("oracle project is a ZIP");
    let mut member = archive
        .by_name(ORACLE_MEMBER)
        .expect("oracle program member");
    let mut bytes = Vec::new();
    member.read_to_end(&mut bytes).unwrap();
    bytes
}

fn suffix(program: &str, id: &str) -> String {
    id.strip_prefix(&format!("{program}_"))
        .unwrap_or(id)
        .to_string()
}

#[derive(Debug, PartialEq)]
struct ParamFacts {
    tag: Option<String>,
    display_order: Option<i64>,
    value: Option<String>,
    access: Option<String>,
    kind: String,
    size_in_bit: Option<i64>,
    min: Option<String>,
    max: Option<String>,
    number_type: Option<String>,
    enum_values: Vec<String>,
    absolute_bit: Option<i64>,
    in_union: bool,
    text: Option<String>,
    name: Option<String>,
}

/// `AS-<hex>` is the segment's start address in ETS ids; the legacy
/// import uses `AS-0000` with absolute offsets.
fn segment_start(segment: &str) -> Option<i64> {
    i64::from_str_radix(segment.rsplit("AS-").next()?, 16).ok()
}

fn params(conn: &Connection, program: &str) -> BTreeMap<String, (ParamFacts, String)> {
    let mut stmt = conn
        .prepare(
            "SELECT pr.id, pr.tag, pr.display_order, COALESCE(pr.value, p.value),
                    COALESCE(pr.access, p.access), pt.kind, pt.size_in_bit, pt.min_inclusive,
                    pt.max_inclusive, pt.number_type, p.code_segment, p.offset, p.bit_offset,
                    p.union_id IS NOT NULL, COALESCE(pr.text, p.text), p.name, pt.id
             FROM parameter_ref pr
             JOIN parameter p ON p.program_id = pr.program_id AND p.id = pr.parameter_id
             JOIN parameter_type pt ON pt.program_id = p.program_id AND pt.id = p.parameter_type_id
             WHERE pr.program_id = ?1",
        )
        .unwrap();
    let rows: Vec<_> = stmt
        .query_map([program], |r| {
            let segment: Option<String> = r.get(10)?;
            let offset: Option<i64> = r.get(11)?;
            let bit: Option<i64> = r.get(12)?;
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(16)?,
                ParamFacts {
                    tag: r.get(1)?,
                    display_order: r.get(2)?,
                    value: r.get(3)?,
                    access: r.get(4)?,
                    kind: r.get(5)?,
                    size_in_bit: r.get(6)?,
                    min: r.get(7)?,
                    max: r.get(8)?,
                    number_type: r.get(9)?,
                    enum_values: Vec::new(),
                    absolute_bit: match (segment.as_deref().and_then(segment_start), offset) {
                        (Some(start), Some(offset)) => {
                            Some((start + offset) * 8 + bit.unwrap_or(0))
                        }
                        _ => None,
                    },
                    in_union: r.get(13)?,
                    text: r.get(14)?,
                    name: r.get(15)?,
                },
            ))
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    rows.into_iter()
        .map(|(id, type_id, mut facts)| {
            facts.enum_values = conn
                .prepare(
                    "SELECT value FROM parameter_type_enum
                     WHERE program_id = ?1 AND parameter_type_id = ?2 ORDER BY display_order",
                )
                .unwrap()
                .query_map([program, &type_id], |r| r.get(0))
                .unwrap()
                .collect::<Result<_, _>>()
                .unwrap();
            (suffix(program, &id), (facts, type_id))
        })
        .collect()
}

fn objects(conn: &Connection, program: &str) -> BTreeMap<String, Vec<Option<String>>> {
    let mut stmt = conn
        .prepare(
            "SELECT r.id, CAST(o.number AS TEXT), COALESCE(r.object_size, o.object_size),
                    COALESCE(r.priority, o.priority), COALESCE(r.read_flag, o.read_flag),
                    COALESCE(r.write_flag, o.write_flag), COALESCE(r.transmit_flag, o.transmit_flag),
                    COALESCE(r.update_flag, o.update_flag),
                    COALESCE(r.communication_flag, o.communication_flag),
                    COALESCE(r.read_on_init_flag, o.read_on_init_flag),
                    COALESCE(r.text, o.text), COALESCE(r.function_text, o.function_text)
             FROM com_object_ref r
             JOIN com_object o ON o.program_id = r.program_id AND o.id = r.com_object_id
             WHERE r.program_id = ?1",
        )
        .unwrap();
    stmt.query_map([program], |r| {
        let id: String = r.get(0)?;
        Ok((
            suffix(program, &id),
            (1..12).map(|i| r.get(i)).collect::<Result<Vec<_>, _>>()?,
        ))
    })
    .unwrap()
    .collect::<Result<_, _>>()
    .unwrap()
}

const OBJECT_FIELDS: [&str; 11] = [
    "number",
    "size",
    "priority",
    "read",
    "write",
    "transmit",
    "update",
    "communication",
    "read-on-init",
    "text",
    "function-text",
];

/// (language, element, attribute) → text, with parameter type ids
/// replaced by the parameter that uses them so both sides share keys.
fn translations(
    conn: &Connection,
    program: &str,
    type_of: &BTreeMap<String, String>,
) -> BTreeMap<(String, String, String), String> {
    let mut stmt = conn
        .prepare(
            "SELECT language, ref_id, attribute_name, text FROM translation
             WHERE scope = 'Program' AND scope_id = ?1 AND text IS NOT NULL",
        )
        .unwrap();
    stmt.query_map([program], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
        ))
    })
    .unwrap()
    .map(Result::unwrap)
    .filter_map(|(language, element, attribute, text)| {
        // The program name is translated on the program itself.
        let element = if element == program {
            "<program>".to_string()
        } else {
            suffix(program, &element)
        };
        let element = match element.split_once("_EN-") {
            Some((type_id, value)) => {
                let full = format!("{program}_{type_id}");
                format!("{}#EN-{value}", type_of.get(&full)?)
            }
            None => element,
        };
        Some(((language, element, attribute), text))
    })
    .collect()
}

fn active(conn: &Connection, program: &str, values: &[(String, String)]) -> BTreeSet<String> {
    let stored = values
        .iter()
        .map(|(r, v)| (format!("{program}_{r}"), v.clone()))
        .collect();
    let evaluation = evaluate_device(conn, program, stored, &[]).unwrap();
    evaluation
        .activation
        .parameter_refs
        .iter()
        .chain(&evaluation.activation.com_object_refs)
        .map(|r| suffix(program, &r.ref_id))
        .collect()
}

fn controllers(conn: &Connection, program: &str) -> BTreeSet<String> {
    conn.prepare(
        "SELECT DISTINCT ref_id FROM dynamic_node WHERE program_id = ?1 AND kind = 'choose'",
    )
    .unwrap()
    .query_map([program], |r| r.get::<_, String>(0))
    .unwrap()
    .map(|r| suffix(program, &r.unwrap()))
    .collect()
}

#[test]
#[ignore = "private corpus: KNXBENCH_PRODUCT_CORPUS, KNXBENCH_VD_PASSWORD_FILE, house project"]
fn n000520_evaluates_like_the_ets_conversion() {
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    let vd4 = knx_testsupport::find_corpus_file(&root(), VD4)
        .unwrap_or_else(|| panic!("SKIP: set KNXBENCH_PRODUCT_CORPUS; {VD4} unavailable"));
    let bytes = std::fs::read(vd4).unwrap();
    let report = import_legacy_file(&conn, VD4, &bytes, Some(&password())).unwrap();
    assert_eq!(report.programs.len(), 2);
    knx_productdb::ingest_file(&conn, ORACLE_MEMBER, &oracle_xml()).unwrap();
    let ours: String = conn
        .query_row(
            "SELECT program_id FROM legacy_program WHERE exim_program_id = ?1",
            [VD_PROGRAM],
            |r| r.get(0),
        )
        .unwrap();

    let mut differences = Vec::new();
    let (theirs_p, ours_p) = (params(&conn, ORACLE_PROGRAM), params(&conn, &ours));
    let keys =
        |m: &BTreeMap<String, (ParamFacts, String)>| m.keys().cloned().collect::<BTreeSet<_>>();
    for only in keys(&theirs_p).symmetric_difference(&keys(&ours_p)) {
        differences.push(format!("param {only}: only on one side"));
    }
    for (key, (theirs, _)) in &theirs_p {
        let Some((ours, _)) = ours_p.get(key) else {
            continue;
        };
        let fields: [(&str, String, String); 12] = [
            (
                "tag",
                format!("{:?}", theirs.tag),
                format!("{:?}", ours.tag),
            ),
            (
                "display-order",
                format!("{:?}", theirs.display_order),
                format!("{:?}", ours.display_order),
            ),
            (
                "value",
                format!("{:?}", theirs.value),
                format!("{:?}", ours.value),
            ),
            (
                "access",
                theirs.access.clone().unwrap_or_default(),
                ours.access.clone().unwrap_or_default(),
            ),
            ("kind", theirs.kind.clone(), ours.kind.clone()),
            (
                "size",
                format!("{:?}", theirs.size_in_bit),
                format!("{:?}", ours.size_in_bit),
            ),
            (
                "min",
                format!("{:?}", theirs.min),
                format!("{:?}", ours.min),
            ),
            (
                "max",
                format!("{:?}", theirs.max),
                format!("{:?}", ours.max),
            ),
            (
                "number-type",
                format!("{:?}", theirs.number_type),
                format!("{:?}", ours.number_type),
            ),
            (
                "enum-values",
                theirs.enum_values.join(","),
                ours.enum_values.join(","),
            ),
            (
                "memory",
                format!("{:?}", theirs.absolute_bit),
                format!("{:?}", ours.absolute_bit),
            ),
            (
                "union",
                theirs.in_union.to_string(),
                ours.in_union.to_string(),
            ),
        ];
        for (field, a, b) in fields {
            if a != b {
                differences.push(format!("param {key} {field}: {b} != {a}"));
            }
        }
        if theirs.text != ours.text {
            differences.push(format!("param {key} text differs"));
        }
        if theirs.name != ours.name {
            differences.push(format!("param {key} name differs"));
        }
    }

    let (theirs_o, ours_o) = (objects(&conn, ORACLE_PROGRAM), objects(&conn, &ours));
    for key in theirs_o
        .keys()
        .collect::<BTreeSet<_>>()
        .symmetric_difference(&ours_o.keys().collect())
    {
        differences.push(format!("object {key}: only on one side"));
    }
    for (key, theirs) in &theirs_o {
        let Some(ours) = ours_o.get(key) else {
            continue;
        };
        for (i, field) in OBJECT_FIELDS.iter().enumerate() {
            if theirs[i] != ours[i] {
                let shown = |v: &Option<String>| {
                    if i >= 9 {
                        "<text>".into()
                    } else {
                        format!("{v:?}")
                    }
                };
                differences.push(format!(
                    "object {key} {field}: {} != {}",
                    shown(&ours[i]),
                    shown(&theirs[i])
                ));
            }
        }
    }

    // Translations, keyed through the parameter that uses each type.
    let type_map = |m: &BTreeMap<String, (ParamFacts, String)>| {
        m.iter()
            .map(|(k, (_, t))| (t.clone(), k.clone()))
            .collect::<BTreeMap<_, _>>()
    };
    let (theirs_t, ours_t) = (
        translations(&conn, ORACLE_PROGRAM, &type_map(&theirs_p)),
        translations(&conn, &ours, &type_map(&ours_p)),
    );
    let mut translation_misses = BTreeMap::<String, usize>::new();
    for (key, text) in &theirs_t {
        let label = match ours_t.get(key) {
            None => "missing",
            Some(t) if t != text => "differs",
            Some(_) => continue,
        };
        *translation_misses
            .entry(format!("translation {} {label}", key.0))
            .or_default() += 1;
    }
    for key in ours_t.keys().filter(|k| !theirs_t.contains_key(*k)) {
        *translation_misses
            .entry(format!("translation {} extra", key.0))
            .or_default() += 1;
    }
    for (what, n) in translation_misses {
        differences.push(format!("{what}: {n}"));
    }

    // Visibility under the defaults and every alternative controlling value.
    let mut cases = vec![Vec::new()];
    let all_controllers: BTreeSet<_> = controllers(&conn, ORACLE_PROGRAM)
        .union(&controllers(&conn, &ours))
        .cloned()
        .collect();
    for controller in &all_controllers {
        if let Some((facts, _)) = theirs_p.get(controller) {
            for value in &facts.enum_values {
                cases.push(vec![(controller.clone(), value.clone())]);
            }
        }
    }
    let (mut visibility, mut p5008) = (0, 0);
    for case in &cases {
        let (theirs, ours_active) = (
            active(&conn, ORACLE_PROGRAM, case),
            active(&conn, &ours, case),
        );
        if theirs != ours_active {
            visibility += 1;
            let only_ours: Vec<_> = ours_active.difference(&theirs).collect();
            if theirs.is_subset(&ours_active) && only_ours == ["P-5008_R-5008"] {
                p5008 += 1;
                continue;
            }
            let only_theirs: Vec<_> = theirs.difference(&ours_active).collect();
            differences.push(format!(
                "visibility {case:?}: ETS only {only_theirs:?}, legacy only {only_ours:?}"
            ));
        }
    }
    if p5008 > 0 {
        differences.push(format!(
            "visibility P-5008_R-5008 active only in the legacy tree in {p5008} of {} cases",
            cases.len()
        ));
    }
    println!(
        "compared {} params, {} objects, {} translations, {} visibility cases ({visibility} differ)",
        theirs_p.len(),
        theirs_o.len(),
        theirs_t.len(),
        cases.len()
    );
    for d in &differences {
        println!("DIFF {d}");
    }
    assert_eq!(
        differences, KNOWN_DEVIATIONS,
        "differences from ETS's conversion"
    );
}
