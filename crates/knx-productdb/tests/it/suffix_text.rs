//! `SuffixText` on `Parameter` and `ParameterRef` is stored, translated and resolved (WP-02).
//!
//! Synthetic fixture: attribute names follow the product corpus, every value
//! is invented. `Suffix` and `SuffixText` are separate declarations: both are
//! stored, only `SuffixText` reaches a view (a stored `Suffix` was never seen
//! in the corpus and has no evidenced meaning).

use crate::v20_rewind;
use std::io::{Cursor, Write};

use knx_productdb::query::{parameter_views, ParameterView};
use knx_productdb::{install_package, open_and_migrate, sha256_hex};
use rusqlite::{params, Connection};
use zip::write::SimpleFileOptions;

const MASTER: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/14"><MasterData><Manufacturers><Manufacturer Id="M-0001" Name="Example"/></Manufacturers></MasterData></KNX>"#;
const HARDWARE: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/14"><ManufacturerData><Manufacturer RefId="M-0001"><Hardware><Hardware Id="H-1" Name="Example"/></Hardware></Manufacturer></ManufacturerData></KNX>"#;

/// P-1: own suffix, overridden by its ref. P-2: `Suffix` and `SuffixText`
/// together, inherited by its ref. P-3: `Suffix` only. P-4: none. P-5: a
/// prefixed `SuffixText`, which `xml::attrs` reads like every other column
/// attribute while its expanded-name evidence row stays. P-6: a union member
/// whose ref declares an empty suffix. A `ModuleDef` parameter and ref, and a `ComObject` carrying
/// `SuffixText` (a host no column models).
const PROGRAM: &[u8] = r#"<KNX xmlns="http://knx.org/xml/project/14" xmlns:x="urn:example:extension"><ManufacturerData><Manufacturer RefId="M-0001"><ApplicationPrograms><ApplicationProgram Id="M-0001_A-1" Name="Program">
<Static>
<ParameterTypes><ParameterType Id="M-0001_A-1_PT-1" Name="n"><TypeNumber SizeInBit="8" Type="unsignedInt" minInclusive="0" maxInclusive="99"/></ParameterType></ParameterTypes>
<Parameters>
<Parameter Id="M-0001_A-1_P-1" Name="delay" ParameterType="M-0001_A-1_PT-1" Text="Delay" Value="5" SuffixText="s"/>
<Parameter Id="M-0001_A-1_P-2" Name="time" ParameterType="M-0001_A-1_PT-1" Text="Time" Value="0" Suffix="x" SuffixText="min"/>
<Parameter Id="M-0001_A-1_P-3" Name="level" ParameterType="M-0001_A-1_PT-1" Text="Level" Value="0" Suffix="pct"/>
<Parameter Id="M-0001_A-1_P-4" Name="plain" ParameterType="M-0001_A-1_PT-1" Text="Plain" Value="0"/>
<Parameter Id="M-0001_A-1_P-5" Name="foreign" ParameterType="M-0001_A-1_PT-1" Text="Foreign" Value="0" x:SuffixText="ext"/>
<Union SizeInBit="8"><Memory CodeSegment="M-0001_A-1_RS-1" Offset="0" BitOffset="0"/>
<Parameter Id="M-0001_A-1_P-6" Name="member" ParameterType="M-0001_A-1_PT-1" Text="Member" Value="0" Offset="0" BitOffset="0" SuffixText="ms"/>
</Union>
</Parameters>
<ParameterRefs>
<ParameterRef Id="M-0001_A-1_P-1_R-1" RefId="M-0001_A-1_P-1" SuffixText="sec" DisplayOrder="1"/>
<ParameterRef Id="M-0001_A-1_P-2_R-1" RefId="M-0001_A-1_P-2" DisplayOrder="2"/>
<ParameterRef Id="M-0001_A-1_P-3_R-1" RefId="M-0001_A-1_P-3" DisplayOrder="3"/>
<ParameterRef Id="M-0001_A-1_P-4_R-1" RefId="M-0001_A-1_P-4" DisplayOrder="4"/>
<ParameterRef Id="M-0001_A-1_P-5_R-1" RefId="M-0001_A-1_P-5" DisplayOrder="5"/>
<ParameterRef Id="M-0001_A-1_P-6_R-1" RefId="M-0001_A-1_P-6" SuffixText="" DisplayOrder="6"/>
</ParameterRefs>
<ComObjectTable><ComObject Id="M-0001_A-1_O-1" Number="1" Text="Switch" ObjectSize="1 Bit" SuffixText="obj"/></ComObjectTable>
</Static>
<ModuleDefs><ModuleDef Id="M-0001_A-1_MD-1" Name="Module"><Static><Parameters>
<Parameter Id="M-0001_A-1_MD-1_P-1" Name="inner" ParameterType="M-0001_A-1_PT-1" Text="Inner" Value="0" SuffixText="°C"/>
</Parameters><ParameterRefs>
<ParameterRef Id="M-0001_A-1_MD-1_P-1_R-1" RefId="M-0001_A-1_MD-1_P-1" SuffixText="K"/>
</ParameterRefs></Static></ModuleDef></ModuleDefs>
<Languages>
  <Language Identifier="de-DE">
    <TranslationUnit RefId="M-0001_A-1">
      <TranslationElement RefId="M-0001_A-1_P-1"><Translation AttributeName="SuffixText" Text="Sek"/></TranslationElement>
      <TranslationElement RefId="M-0001_A-1_P-1_R-1"><Translation AttributeName="SuffixText" Text="Sekunden"/></TranslationElement>
      <TranslationElement RefId="M-0001_A-1_P-2"><Translation AttributeName="SuffixText" Text="Min."/></TranslationElement>
    </TranslationUnit>
  </Language>
</Languages>
</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#
    .as_bytes();

const PROGRAMS: &str = "/KNX/ManufacturerData/Manufacturer/ApplicationPrograms/ApplicationProgram";

fn archive(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (path, bytes) in entries {
        writer
            .start_file(*path, SimpleFileOptions::default())
            .unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn package() -> Vec<u8> {
    archive(&[
        ("knx_master.xml", MASTER),
        ("M-0001/Hardware.xml", HARDWARE),
        ("M-0001/M-0001_A-1.xml", PROGRAM),
    ])
}

fn installed() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let conn = open_and_migrate(&path).unwrap();
    install_package(&conn, "sample.knxprod", &package()).unwrap();
    (dir, path)
}

type Columns = Vec<(String, Option<String>, Option<String>)>;

/// `(id, suffix, suffix_text)` of every parameter, `(id, NULL, suffix_text)`
/// of every ref.
fn columns(conn: &Connection) -> (Columns, Columns) {
    let read = |sql: &str| -> Columns {
        conn.prepare(sql)
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    };
    (
        read("SELECT id, suffix, suffix_text FROM parameter ORDER BY id"),
        read("SELECT id, NULL, suffix_text FROM parameter_ref ORDER BY id"),
    )
}

/// `(xpath, name, occurrences, sample)` of every `SuffixText` unknown row.
fn suffix_rows(conn: &Connection, table: &str) -> Vec<(String, String, i64, Option<String>)> {
    conn.prepare(&format!(
        "SELECT xpath, name, occurrences, sample FROM {table}
         WHERE kind = 'Attribute' AND name LIKE '%SuffixText' ORDER BY 1, 2"
    ))
    .unwrap()
    .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
    .unwrap()
    .collect::<Result<_, _>>()
    .unwrap()
}

fn view<'a>(views: &'a [ParameterView], id: &str) -> &'a ParameterView {
    views.iter().find(|v| v.id == id).unwrap()
}

fn some(value: &str) -> Option<String> {
    Some(value.to_string())
}

#[test]
fn both_declarations_are_stored_separately_and_verbatim() {
    let (_dir, path) = installed();
    let conn = Connection::open(&path).unwrap();
    let (parameters, refs) = columns(&conn);
    let p = |id: &str, suffix: Option<&str>, text: Option<&str>| {
        (
            format!("M-0001_A-1_{id}"),
            suffix.map(str::to_string),
            text.map(str::to_string),
        )
    };
    assert_eq!(
        parameters,
        vec![
            p("MD-1_P-1", None, Some("°C")),
            p("P-1", None, Some("s")),
            p("P-2", Some("x"), Some("min")),
            p("P-3", Some("pct"), None),
            p("P-4", None, None),
            p("P-5", None, Some("ext")),
            p("P-6", None, Some("ms")),
        ]
    );
    assert_eq!(
        refs,
        vec![
            p("MD-1_P-1_R-1", None, Some("K")),
            p("P-1_R-1", None, Some("sec")),
            p("P-2_R-1", None, None),
            p("P-3_R-1", None, None),
            p("P-4_R-1", None, None),
            p("P-5_R-1", None, None),
            p("P-6_R-1", None, Some("")),
        ]
    );
}

#[test]
fn only_the_unmodelled_suffix_declarations_stay_reported() {
    let (_dir, path) = installed();
    let conn = Connection::open(&path).unwrap();
    let expected = vec![
        (
            format!("{PROGRAMS}/Static/ComObjectTable/ComObject"),
            "SuffixText".to_string(),
            1,
            some("obj"),
        ),
        (
            format!("{PROGRAMS}/Static/Parameters/Parameter"),
            "{urn:example:extension}SuffixText".to_string(),
            1,
            some("ext"),
        ),
    ];
    assert_eq!(suffix_rows(&conn, "ingest_unknown"), expected);
    assert_eq!(suffix_rows(&conn, "package_install_unknown"), expected);
}

#[test]
fn the_view_takes_the_ref_suffix_over_the_parameter_one_like_text() {
    let (_dir, path) = installed();
    let conn = Connection::open(&path).unwrap();
    let views = parameter_views(&conn, "M-0001_A-1", None).unwrap();
    let suffix = |id: &str| {
        let v = view(&views, id);
        (v.suffix_text.clone(), v.suffix_text_language.clone())
    };
    assert_eq!(suffix("M-0001_A-1_P-1_R-1"), (some("sec"), None));
    assert_eq!(suffix("M-0001_A-1_P-2_R-1"), (some("min"), None));
    // A stored `Suffix` is not a display suffix.
    assert_eq!(suffix("M-0001_A-1_P-3_R-1"), (None, None));
    assert_eq!(suffix("M-0001_A-1_P-4_R-1"), (None, None));
    assert_eq!(suffix("M-0001_A-1_P-5_R-1"), (some("ext"), None));
    // The ref's own empty declaration wins, verbatim.
    assert_eq!(suffix("M-0001_A-1_P-6_R-1"), (some(""), None));
    // The value is untouched by its suffix.
    let delay = view(&views, "M-0001_A-1_P-1_R-1");
    assert_eq!(delay.text.as_deref(), Some("Delay"));
}

#[test]
fn a_translated_suffix_follows_the_layer_that_supplies_it() {
    let (_dir, path) = installed();
    let conn = Connection::open(&path).unwrap();
    let views = parameter_views(&conn, "M-0001_A-1", Some("de")).unwrap();
    let suffix = |id: &str| {
        let v = view(&views, id);
        (v.suffix_text.clone(), v.suffix_text_language.clone())
    };
    // The ref's own translation, not the parameter's.
    assert_eq!(
        suffix("M-0001_A-1_P-1_R-1"),
        (some("Sekunden"), some("de-DE"))
    );
    // Inherited from the parameter, with the parameter's translation.
    assert_eq!(suffix("M-0001_A-1_P-2_R-1"), (some("Min."), some("de-DE")));
    // No translation: the package's own text, no language.
    assert_eq!(suffix("M-0001_A-1_P-6_R-1"), (some(""), None));
    let english = parameter_views(&conn, "M-0001_A-1", Some("fr")).unwrap();
    let delay = view(&english, "M-0001_A-1_P-1_R-1");
    assert_eq!(
        (
            delay.suffix_text.as_deref(),
            delay.suffix_text_language.as_deref()
        ),
        (Some("sec"), None)
    );
}

/// What a v25 parser reported for the fixture's one program blob and that
/// v26 no longer does: `(xpath below the program, occurrences, first sample)`.
const V25_ONLY: &[(&str, i64, &str)] = &[
    ("/Static/Parameters/Parameter", 2, "s"),
    ("/Static/Parameters/Union/Parameter", 1, "ms"),
    ("/Static/ParameterRefs/ParameterRef", 2, "sec"),
    ("/ModuleDefs/ModuleDef/Static/Parameters/Parameter", 1, "°C"),
    (
        "/ModuleDefs/ModuleDef/Static/ParameterRefs/ParameterRef",
        1,
        "K",
    ),
];

/// Rewinds a v26 install of this fixture to a genuine v25 database: the
/// columns go, and the `SuffixText` rows a v25 parser wrote come back in
/// `ingest_unknown`, the measured report and `package.unknown_count`.
fn rewind_to_v25(conn: &Connection) {
    v20_rewind::drop_v26_objects(conn);
    let blob = sha256_hex(PROGRAM);
    let package: String = conn
        .query_row(
            "SELECT package_sha256 FROM package_member WHERE source_sha256 = ?1",
            [&blob],
            |r| r.get(0),
        )
        .unwrap();
    for (xpath, occurrences, sample) in V25_ONLY {
        conn.execute(
            "INSERT INTO ingest_unknown (source_sha256, program_id, xpath, kind, name, occurrences, sample)
             VALUES (?1, NULL, ?2, 'Attribute', 'SuffixText', ?3, ?4)",
            params![blob, format!("{PROGRAMS}{xpath}"), occurrences, sample],
        )
        .unwrap();
    }
    let mut rows: Vec<(String, String, String, i64, Option<String>)> = conn
        .prepare(
            "SELECT xpath, kind, name, occurrences, sample FROM package_install_unknown
             WHERE package_sha256 = ?1 ORDER BY ordinal",
        )
        .unwrap()
        .query_map([&package], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    for (xpath, occurrences, sample) in V25_ONLY {
        rows.push((
            format!("{PROGRAMS}{xpath}"),
            "Attribute".into(),
            "SuffixText".into(),
            *occurrences,
            some(sample),
        ));
    }
    rows.sort_by(|a, b| (&a.0, &a.1, &a.2).cmp(&(&b.0, &b.1, &b.2)));
    conn.execute(
        "DELETE FROM package_install_unknown WHERE package_sha256 = ?1",
        [&package],
    )
    .unwrap();
    for (ordinal, (xpath, kind, name, occurrences, sample)) in rows.iter().enumerate() {
        conn.execute(
            "INSERT INTO package_install_unknown VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                package,
                ordinal as i64,
                xpath,
                kind,
                name,
                occurrences,
                sample
            ],
        )
        .unwrap();
    }
    let distinct = rows.len() as i64;
    let total: i64 = rows.iter().map(|r| r.3).sum();
    conn.execute(
        "UPDATE package_install_report SET unknown_distinct = ?2, unknown_occurrences = ?3
         WHERE package_sha256 = ?1",
        params![package, distinct, total],
    )
    .unwrap();
    for (disposition, count) in [("read", total), ("stored", distinct)] {
        conn.execute(
            "UPDATE package_install_count SET count = ?3
             WHERE package_sha256 = ?1 AND category = 'unknown_construct' AND disposition = ?2",
            params![package, disposition, count],
        )
        .unwrap();
    }
    conn.execute(
        "UPDATE package SET unknown_count = unknown_count + ?2 WHERE sha256 = ?1",
        params![package, V25_ONLY.len() as i64],
    )
    .unwrap();
    conn.execute_batch("PRAGMA user_version = 25;").unwrap();
}

type Table = Vec<Vec<Option<String>>>;

fn table(conn: &Connection, sql: &str) -> Table {
    let mut stmt = conn.prepare(sql).unwrap();
    let width = stmt.column_count();
    stmt.query_map([], |r| {
        (0..width)
            .map(|i| {
                let value: rusqlite::types::Value = r.get(i)?;
                Ok(match value {
                    rusqlite::types::Value::Null => None,
                    rusqlite::types::Value::Integer(v) => Some(v.to_string()),
                    rusqlite::types::Value::Real(v) => Some(v.to_string()),
                    rusqlite::types::Value::Text(v) => Some(v),
                    rusqlite::types::Value::Blob(v) => Some(sha256_hex(&v)),
                })
            })
            .collect()
    })
    .unwrap()
    .collect::<Result<_, _>>()
    .unwrap()
}

/// Everything v26 touches, as a fresh install leaves it.
fn snapshot(conn: &Connection) -> Vec<Table> {
    vec![
        table(conn, "PRAGMA user_version"),
        table(conn, "SELECT * FROM parameter ORDER BY program_id, id"),
        table(conn, "SELECT * FROM parameter_ref ORDER BY program_id, id"),
        table(
            conn,
            "SELECT source_sha256, program_id, xpath, kind, name, occurrences, sample
             FROM ingest_unknown ORDER BY 1, 3, 4, 5, 6, 7",
        ),
        table(conn, "SELECT * FROM package_install_report ORDER BY 1"),
        table(conn, "SELECT * FROM package_install_count ORDER BY 1, 2"),
        table(conn, "SELECT * FROM package_install_unknown ORDER BY 1, 2"),
        table(conn, "SELECT sha256, unknown_count FROM package ORDER BY 1"),
    ]
}

#[test]
fn the_v25_rewind_is_genuine() {
    let (_dir, path) = installed();
    let conn = Connection::open(&path).unwrap();
    rewind_to_v25(&conn);
    for (table, column) in [
        ("parameter", "suffix_text"),
        ("parameter_ref", "suffix_text"),
    ] {
        let present: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM pragma_table_info(?1) WHERE name = ?2)",
                [table, column],
                |r| r.get(0),
            )
            .unwrap();
        assert!(!present, "{table}.{column}");
    }
    assert_eq!(
        suffix_rows(&conn, "ingest_unknown").len(),
        2 + V25_ONLY.len()
    );
    assert_eq!(
        suffix_rows(&conn, "package_install_unknown").len(),
        2 + V25_ONLY.len()
    );
}

#[test]
fn v25_to_v26_reproduces_a_fresh_install() {
    let (_dir, path) = installed();
    let fresh = snapshot(&Connection::open(&path).unwrap());
    rewind_to_v25(&Connection::open(&path).unwrap());
    let conn = open_and_migrate(&path).unwrap();
    assert_eq!(snapshot(&conn), fresh);
    let retry = install_package(&conn, "sample.knxprod", &package()).unwrap();
    assert!(retry.skipped);
}

#[test]
fn v25_to_v26_names_a_damaged_blob_keeps_its_rows_and_downgrades_its_report() {
    let (_dir, path) = installed();
    let blob = sha256_hex(PROGRAM);
    {
        let conn = Connection::open(&path).unwrap();
        rewind_to_v25(&conn);
        let mut damaged = PROGRAM.to_vec();
        let at = damaged.len() - 10;
        damaged[at] ^= 0x20;
        conn.execute(
            "UPDATE source_file SET bytes = ?1 WHERE sha256 = ?2",
            params![damaged, blob],
        )
        .unwrap();
    }
    let conn = open_and_migrate(&path).unwrap();
    let failures: i64 = conn
        .query_row(
            "SELECT count(*) FROM ingest_unknown
             WHERE source_sha256 = ?1 AND kind = 'SuffixTextBackfillError'",
            [&blob],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(failures, 1);
    // Nothing retired, nothing invented.
    assert_eq!(
        suffix_rows(&conn, "ingest_unknown").len(),
        2 + V25_ONLY.len()
    );
    let filled: i64 = conn
        .query_row(
            "SELECT (SELECT count(*) FROM parameter WHERE suffix_text IS NOT NULL)
                  + (SELECT count(*) FROM parameter_ref WHERE suffix_text IS NOT NULL)",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(filled, 0);
    let status: String = conn
        .query_row("SELECT status FROM package_install_report", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(status, "unavailable");
}
