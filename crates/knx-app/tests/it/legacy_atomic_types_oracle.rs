//! Legacy string and long-enum types match ETS4's own conversion (KL-128 acceptance).
//!
//! KL-128 acceptance (Goal A5): legacy atomic types 3 (`string`) and 5
//! (`long enum`) map like ETS4's own conversion of the same database.
//!
//! Oracle: the Siemens November 2016 database as `.vd5` (legacy path) and as
//! ETS4's `.knxprod` (XML path, `ConvertedFromPreEts4Data`), both private
//! corpus files, both read only. Programs pair by manufacturer, application
//! number and version; types by name; parameters by `ParameterRef/@Tag`
//! (`PARAMETER_NUMBER`). Differences are printed by id and field, never with
//! product content. `#[ignore]`d: run with `KNXBENCH_PRODUCT_CORPUS` and
//! `KNXBENCH_VD_PASSWORD_FILE`.

use std::collections::BTreeMap;
use std::path::PathBuf;

use knx_app::legacy::{import_legacy_file, LegacyPassword};
use knx_productdb::Connection;

const VD5: &str = "SIEMENS_KNX_PDB_Nov_2016_ETS3.vd5";
const ETS4: &str = "SIEMENS_KNX_PDB_Nov_2016_ETS4.knxprod";

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

/// (kind, size in bit, base, min, max) of a type.
type Shape = (
    String,
    Option<i64>,
    Option<String>,
    Option<String>,
    Option<String>,
);

fn shape(conn: &Connection, program: &str, name: &str) -> Vec<(String, Shape)> {
    conn.prepare(
        "SELECT id, kind, size_in_bit, base, min_inclusive, max_inclusive
         FROM parameter_type WHERE program_id = ?1 AND name = ?2",
    )
    .unwrap()
    .query_map([program, name], |r| {
        Ok((
            r.get(0)?,
            (r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?),
        ))
    })
    .unwrap()
    .collect::<Result<_, _>>()
    .unwrap()
}

/// Enumeration values in display order, with each id's `_EN-…` suffix.
fn enumeration(conn: &Connection, program: &str, type_id: &str) -> Vec<(String, String)> {
    conn.prepare(
        "SELECT value, id FROM parameter_type_enum
         WHERE program_id = ?1 AND parameter_type_id = ?2 ORDER BY display_order, rowid",
    )
    .unwrap()
    .query_map([program, type_id], |r| {
        let id: String = r.get(1)?;
        Ok((
            r.get::<_, String>(0)?,
            id.rsplit('_').next().unwrap_or_default().to_string(),
        ))
    })
    .unwrap()
    .collect::<Result<_, _>>()
    .unwrap()
}

/// Tag → (effective value, effective access) for parameters of `type_id`;
/// an absent `Access` is the schema default `ReadWrite`.
fn parameters(
    conn: &Connection,
    program: &str,
    type_id: &str,
) -> BTreeMap<String, (Option<String>, Option<String>)> {
    conn.prepare(
        "SELECT r.tag, COALESCE(r.value, p.value), COALESCE(r.access, p.access, 'ReadWrite')
         FROM parameter_ref r JOIN parameter p
           ON p.program_id = r.program_id AND p.id = r.parameter_id
         WHERE r.program_id = ?1 AND p.parameter_type_id = ?2",
    )
    .unwrap()
    .query_map([program, type_id], |r| {
        Ok((r.get(0)?, (r.get(1)?, r.get(2)?)))
    })
    .unwrap()
    .collect::<Result<_, _>>()
    .unwrap()
}

#[test]
#[ignore = "needs the private product corpus and the legacy password"]
fn string_and_long_enum_types_map_like_the_ets4_conversion() {
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    knx_productdb::install_package(&conn, ETS4, &read(ETS4)).unwrap();
    import_legacy_file(&conn, VD5, &read(VD5), Some(&password())).unwrap();

    // Every legacy type that atomic types 3 and 5 produce, with its
    // program's ETS4 counterpart where the package has one.
    let types: Vec<(String, String, String, Option<String>)> = conn
        .prepare(
            "SELECT l.exim_program_id, t.program_id, t.id, t.name
             FROM parameter_type t
             JOIN legacy_program l ON l.program_id = t.program_id
             WHERE t.kind = 'Text' OR t.base = 'BinaryValue'
             ORDER BY CAST(l.exim_program_id AS INTEGER), t.id",
        )
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    let counterpart = |legacy: &str| -> Option<String> {
        conn.query_row(
            "SELECT e.id FROM application_program a JOIN application_program e
               ON e.manufacturer_id = a.manufacturer_id
              AND e.application_number = a.application_number
              AND e.application_version = a.application_version
              AND e.id NOT IN (SELECT program_id FROM legacy_program)
             WHERE a.id = ?1",
            [legacy],
            |r| r.get(0),
        )
        .ok()
    };

    let mut wrong = Vec::new();
    let mut counts = BTreeMap::<&str, usize>::new();
    for (exim, legacy, type_id, name) in &types {
        let ours = shape(&conn, legacy, name.as_deref().unwrap_or_default());
        let ours = ours.into_iter().find(|(id, _)| id == type_id).unwrap().1;
        let kind = if ours.0 == "Text" {
            "string"
        } else {
            "long enum"
        };
        *counts.entry(kind).or_default() += 1;
        let Some(ets) = counterpart(legacy) else {
            *counts.entry("without ETS4 conversion").or_default() += 1;
            continue;
        };
        let theirs = shape(&conn, &ets, name.as_deref().unwrap_or_default());
        let [(ets_type, theirs)] = theirs.as_slice() else {
            wrong.push(format!(
                "{exim}/{type_id}: {} ETS types of that name",
                theirs.len()
            ));
            continue;
        };
        *counts.entry("compared").or_default() += 1;
        if &ours != theirs {
            wrong.push(format!("{exim}/{type_id}: shape {ours:?} vs {theirs:?}"));
        }
        if kind == "long enum" {
            let (a, b) = (
                enumeration(&conn, legacy, type_id),
                enumeration(&conn, &ets, ets_type),
            );
            if a != b {
                wrong.push(format!(
                    "{exim}/{type_id}: enumeration values or keys differ ({} vs {})",
                    a.len(),
                    b.len()
                ));
            }
        }
        let (a, b) = (
            parameters(&conn, legacy, type_id),
            parameters(&conn, &ets, ets_type),
        );
        *counts.entry("parameters compared").or_default() += b.len();
        for (tag, theirs) in &b {
            match a.get(tag) {
                Some(ours) if ours == theirs => {}
                Some(_) => wrong.push(format!("{exim}/{type_id}: parameter {tag} value/access")),
                None => wrong.push(format!("{exim}/{type_id}: parameter {tag} missing")),
            }
        }
    }
    println!("{counts:?}");
    assert!(wrong.is_empty(), "{wrong:#?}");
    // Measured 2026-10-13: 49 of the 56 types have an ETS4 conversion
    // (7 belong to programs the package does not hold); 1,490 of their
    // parameters compare.
    assert_eq!(counts.get("compared"), Some(&49), "{counts:?}");
    assert_eq!(
        counts.get("without ETS4 conversion"),
        Some(&7),
        "{counts:?}"
    );
    assert_eq!(counts.get("parameters compared"), Some(&1490), "{counts:?}");
    assert_eq!(counts.get("string"), Some(&27), "{counts:?}");
    assert_eq!(counts.get("long enum"), Some(&29), "{counts:?}");
}
