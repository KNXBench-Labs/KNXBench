//! PDB-9: each observed `ParameterType` kind is typed, the rest reported.
//!
//! Every observed kind lands as its own typed row, and what the parser does
//! not model is reported, not dropped.
//!
//! Corpus basis (304 distinct application programs, read-only aggregate
//! scan): `TypeRestriction` 20,759, `TypeNumber` 4,153, `TypePicture` 1,118,
//! `TypeFloat` 579, `TypeText` 554, `TypeColor` 115, `TypeNone` 87,
//! `TypeIPAddress` 19, `TypeTime` 17, `TypeRawData` 3. Before PDB-9
//! `TypeColor` and `TypeTime` fell into the `Other` bucket. The fixture
//! below is synthetic; attribute *shapes* follow the corpus, values do not
//! copy any manufacturer's data.

use rusqlite::Connection;

mod v16_rewind;
mod v20_rewind;

fn db() -> (tempfile::TempDir, Connection) {
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    (dir, conn)
}

const PROGRAM_ID: &str = "M-00FA_A-0009-10-ABCD";

const PROGRAM: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/20">
  <ManufacturerData>
    <Manufacturer RefId="M-00FA">
      <ApplicationPrograms>
        <ApplicationProgram Id="M-00FA_A-0009-10-ABCD" Name="Kinds" ApplicationNumber="9"
                            ApplicationVersion="16" ProgramType="ApplicationProgram"
                            MaskVersion="MV-07B0" PeiType="0" LoadProcedureStyle="MergedProcedure"
                            DefaultLanguage="en-US">
          <Static>
            <ParameterTypes>
              <ParameterType Id="PT-Restriction" Name="mode">
                <TypeRestriction Base="Value" SizeInBit="8" UIHint="RadioButton">
                  <Enumeration Id="PT-Restriction_EN-0" Text="Off" Value="0" />
                  <Enumeration Id="PT-Restriction_EN-1" Text="On" Value="1" />
                </TypeRestriction>
              </ParameterType>
              <ParameterType Id="PT-Number" Name="count">
                <TypeNumber SizeInBit="16" Type="unsignedInt" minInclusive="0"
                            maxInclusive="1000" Increment="10" UIHint="Slider" />
              </ParameterType>
              <ParameterType Id="PT-Float" Name="threshold">
                <TypeFloat Encoding="DPT 9" minInclusive="-20" maxInclusive="60" Increment="0.5" />
              </ParameterType>
              <ParameterType Id="PT-Text" Name="label">
                <TypeText SizeInBit="112" Pattern="[A-Z]*" />
              </ParameterType>
              <ParameterType Id="PT-None" Name="info">
                <TypeNone />
              </ParameterType>
              <ParameterType Id="PT-IP" Name="gateway">
                <TypeIPAddress AddressType="HostAddress" />
              </ParameterType>
              <ParameterType Id="PT-Picture" Name="diagram">
                <TypePicture RefId="M-00FA_BG-diagram.png" HorizontalAlignment="Center" />
              </ParameterType>
              <ParameterType Id="PT-Raw" Name="blob">
                <TypeRawData MaxSize="4" />
              </ParameterType>
              <ParameterType Id="PT-Color" Name="light colour">
                <TypeColor Space="RGB" />
              </ParameterType>
              <ParameterType Id="PT-Time" Name="delay">
                <TypeTime SizeInBit="16" Unit="Seconds" minInclusive="0" maxInclusive="3600"
                          UIHint="Duration_hhmmss" />
              </ParameterType>
              <ParameterType Id="PT-Time2" Name="long delay">
                <TypeTime SizeInBit="32" Unit="Minutes" minInclusive="1" maxInclusive="1440" />
              </ParameterType>
              <ParameterType Id="PT-Color2" Name="accent">
                <TypeColor Space="HSV" />
              </ParameterType>
            </ParameterTypes>
            <Parameters />
            <ParameterRefs />
          </Static>
        </ApplicationProgram>
      </ApplicationPrograms>
    </Manufacturer>
  </ManufacturerData>
</KNX>"#;

const TYPE_PATH: &str = "/KNX/ManufacturerData/Manufacturer/ApplicationPrograms/ApplicationProgram/Static/ParameterTypes/ParameterType";

type TypeRow = (
    String,
    Option<i64>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

fn type_row(conn: &Connection, id: &str) -> TypeRow {
    conn.query_row(
        "SELECT kind, size_in_bit, base, min_inclusive, max_inclusive, number_type
         FROM parameter_type WHERE program_id = ?1 AND id = ?2",
        [PROGRAM_ID, id],
        |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
            ))
        },
    )
    .unwrap()
}

fn s(v: &str) -> Option<String> {
    Some(v.to_string())
}

/// `(xpath, kind, name, occurrences, sample)` of every unknown row this
/// source produced, sorted, so two databases can be compared exactly.
fn unknown_rows(conn: &Connection) -> Vec<(String, String, String, i64, Option<String>)> {
    let mut stmt = conn
        .prepare(
            "SELECT xpath, kind, name, occurrences, sample FROM ingest_unknown
             WHERE kind IN ('Element', 'Attribute')
             ORDER BY xpath, kind, name",
        )
        .unwrap();
    stmt.query_map([], |r| {
        Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
    })
    .unwrap()
    .collect::<Result<_, _>>()
    .unwrap()
}

#[test]
fn every_observed_parameter_kind_is_stored_as_its_own_kind() {
    let (_dir, conn) = db();
    knx_productdb::ingest_file(&conn, "M-00FA/kinds.xml", PROGRAM.as_bytes()).unwrap();

    assert_eq!(
        type_row(&conn, "PT-Restriction"),
        ("Restriction".into(), Some(8), s("Value"), None, None, None)
    );
    assert_eq!(
        type_row(&conn, "PT-Number"),
        (
            "Number".into(),
            Some(16),
            None,
            s("0"),
            s("1000"),
            s("unsignedInt")
        )
    );
    assert_eq!(
        type_row(&conn, "PT-Float"),
        ("Float".into(), None, None, s("-20"), s("60"), None)
    );
    assert_eq!(
        type_row(&conn, "PT-Text"),
        ("Text".into(), Some(112), None, None, None, None)
    );
    assert_eq!(type_row(&conn, "PT-None").0, "None");
    assert_eq!(type_row(&conn, "PT-IP").0, "IPAddress");
    assert_eq!(type_row(&conn, "PT-Picture").0, "Picture");
    assert_eq!(type_row(&conn, "PT-Raw").0, "Raw");
    assert_eq!(
        type_row(&conn, "PT-Color"),
        ("Color".into(), None, None, None, None, None),
        "TypeColor is its own kind, not the Other bucket"
    );
    assert_eq!(
        type_row(&conn, "PT-Time"),
        ("Time".into(), Some(16), None, s("0"), s("3600"), None),
        "TypeTime keeps its size and integer bounds, like TypeNumber"
    );
    let other: i64 = conn
        .query_row(
            "SELECT count(*) FROM parameter_type WHERE kind = 'Other'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(other, 0);
}

/// What is not a column is reported with a sample, at the type child's own
/// path. No type child is reported as an unknown *element* any more.
#[test]
fn unmodelled_parameter_kind_attributes_are_reported_not_dropped() {
    let (_dir, conn) = db();
    knx_productdb::ingest_file(&conn, "M-00FA/kinds.xml", PROGRAM.as_bytes()).unwrap();
    let rows = unknown_rows(&conn);
    let reported = |child: &str, name: &str, sample: &str| {
        let xpath = format!("{TYPE_PATH}/{child}");
        assert!(
            rows.iter().any(|(x, k, n, _, smp)| x == &xpath
                && k == "Attribute"
                && n == name
                && smp.as_deref() == Some(sample)),
            "{child}/@{name} must be reported with its sample: {rows:#?}"
        );
    };
    reported("TypeRestriction", "UIHint", "RadioButton");
    reported("TypeNumber", "Increment", "10");
    reported("TypeNumber", "UIHint", "Slider");
    reported("TypeFloat", "Encoding", "DPT 9");
    reported("TypeFloat", "Increment", "0.5");
    reported("TypeText", "Pattern", "[A-Z]*");
    reported("TypeIPAddress", "AddressType", "HostAddress");
    reported("TypePicture", "RefId", "M-00FA_BG-diagram.png");
    reported("TypePicture", "HorizontalAlignment", "Center");
    reported("TypeRawData", "MaxSize", "4");
    reported("TypeColor", "Space", "RGB");
    reported("TypeTime", "Unit", "Seconds");
    reported("TypeTime", "UIHint", "Duration_hhmmss");
    assert!(
        !rows
            .iter()
            .any(|(x, k, _, _, _)| x == TYPE_PATH && k == "Element"),
        "no ParameterType child is an unknown element: {rows:#?}"
    );
    for read in ["SizeInBit", "minInclusive", "maxInclusive", "Base", "Type"] {
        assert!(
            !rows
                .iter()
                .any(|(_, k, n, _, _)| k == "Attribute" && n == read),
            "{read} is stored in a column and must not also be reported"
        );
    }
}

/// A `ParameterType` child nobody has seen is still reported as an element
/// and kept under the explicit `Other` kind — never guessed into a known one.
#[test]
fn an_unknown_parameter_kind_stays_other_and_is_reported() {
    let (_dir, conn) = db();
    let xml = PROGRAM.replace(
        r#"<TypeColor Space="RGB" />"#,
        r#"<TypeHologram Depth="3" />"#,
    );
    knx_productdb::ingest_file(&conn, "M-00FA/kinds.xml", xml.as_bytes()).unwrap();
    assert_eq!(type_row(&conn, "PT-Color").0, "Other");
    assert!(unknown_rows(&conn)
        .iter()
        .any(|(x, k, n, _, _)| x == TYPE_PATH && k == "Element" && n == "TypeHologram"));
}

/// Rewinds a fresh v15 database to what a v14 parser left behind for the
/// two kinds PDB-9 adds: `kind = 'Other'` with no size/bounds, the child
/// element reported as unknown (with no sample, as `UnknownCollector::element`
/// writes it) and none of its attributes reported.
fn rewind_to_v14(conn: &Connection) {
    v16_rewind::rewind_to_v15(conn);
    for (child, kind) in [("TypeColor", "Color"), ("TypeTime", "Time")] {
        let attr_path = format!("{TYPE_PATH}/{child}");
        let source: String = conn
            .query_row(
                "SELECT source_sha256 FROM ingest_unknown WHERE xpath = ?1 LIMIT 1",
                [&attr_path],
                |r| r.get(0),
            )
            .unwrap();
        conn.execute("DELETE FROM ingest_unknown WHERE xpath = ?1", [&attr_path])
            .unwrap();
        // v14's `UnknownCollector::element` counted every occurrence.
        let occurrences: i64 = conn
            .query_row(
                "SELECT count(*) FROM parameter_type WHERE kind = ?1",
                [kind],
                |r| r.get(0),
            )
            .unwrap();
        // …into the one aggregated row per `(xpath, name)`: a v15 ingest
        // may already hold one for a *non*-type-deciding occurrence at the
        // same path, and v14 counted both kinds of occurrence together.
        let merged = conn
            .execute(
                "UPDATE ingest_unknown SET occurrences = occurrences + ?4
                 WHERE source_sha256 = ?1 AND xpath = ?2 AND kind = 'Element' AND name = ?3",
                rusqlite::params![source, TYPE_PATH, child, occurrences],
            )
            .unwrap();
        if merged == 0 {
            conn.execute(
                "INSERT INTO ingest_unknown (source_sha256, program_id, xpath, kind, name, occurrences, sample)
                 VALUES (?1, NULL, ?2, 'Element', ?3, ?4, NULL)",
                rusqlite::params![source, TYPE_PATH, child, occurrences],
            )
            .unwrap();
        }
    }
    v20_rewind::drop_v20_objects(conn);
    conn.execute_batch(
        "UPDATE parameter_type
         SET kind = 'Other', size_in_bit = NULL, min_inclusive = NULL, max_inclusive = NULL
         WHERE kind IN ('Color', 'Time');
         PRAGMA user_version = 14;",
    )
    .unwrap();
}

#[test]
fn v14_to_v15_rederives_color_and_time_kinds_exactly_like_a_fresh_ingest() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let (fresh_types, fresh_unknown) = {
        let conn = knx_productdb::open_and_migrate(&path).unwrap();
        knx_productdb::ingest_file(&conn, "M-00FA/kinds.xml", PROGRAM.as_bytes()).unwrap();
        let types = ["PT-Color", "PT-Time", "PT-Color2", "PT-Time2"].map(|id| type_row(&conn, id));
        let unknown = unknown_rows(&conn);
        let time_units: i64 = conn
            .query_row(
                "SELECT occurrences FROM ingest_unknown WHERE xpath = ?1 AND name = 'Unit'",
                [format!("{TYPE_PATH}/TypeTime")],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(time_units, 2, "the fixture aggregates two TypeTime/@Unit");
        rewind_to_v14(&conn);
        assert_eq!(type_row(&conn, "PT-Time").0, "Other", "rewind took effect");
        (types, unknown)
    };
    let conn = knx_productdb::open_and_migrate(&path).unwrap();
    let version: i64 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap();
    assert_eq!(version, knx_productdb::CURRENT_PRODUCTDB_VERSION);
    assert_eq!(
        ["PT-Color", "PT-Time", "PT-Color2", "PT-Time2"].map(|id| type_row(&conn, id)),
        fresh_types
    );
    assert_eq!(
        unknown_rows(&conn),
        fresh_unknown,
        "the stale element rows are retired and the attribute rows a fresh ingest writes appear"
    );
}

/// A blob that lost its program id to another file (ADR-0011) must not
/// rewrite the winner's `Other` row during the backfill.
#[test]
fn v14_to_v15_does_not_let_a_losing_blob_rewrite_the_winners_row() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    {
        let conn = knx_productdb::open_and_migrate(&path).unwrap();
        // The winner declares PT-Time as something else entirely.
        let winner = PROGRAM.replace(
            r#"<TypeTime SizeInBit="16" Unit="Seconds" minInclusive="0" maxInclusive="3600"
                          UIHint="Duration_hhmmss" />"#,
            r#"<TypeHologram />"#,
        );
        knx_productdb::ingest_file(&conn, "M-00FA/winner.xml", winner.as_bytes()).unwrap();
        knx_productdb::ingest_file(&conn, "M-00FA/loser.xml", PROGRAM.as_bytes()).unwrap();
        assert_eq!(type_row(&conn, "PT-Time").0, "Other");
        // Make the loser look like a v14 ingest of it: its TypeTime was an
        // unknown element then, so the migration does select this blob.
        let loser: String = conn
            .query_row(
                "SELECT sha256 FROM source_file WHERE source_path = 'M-00FA/loser.xml'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        conn.execute(
            "DELETE FROM ingest_unknown WHERE source_sha256 = ?1 AND xpath = ?2",
            rusqlite::params![loser, format!("{TYPE_PATH}/TypeTime")],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO ingest_unknown (source_sha256, program_id, xpath, kind, name, occurrences, sample)
             VALUES (?1, NULL, ?2, 'Element', 'TypeTime', 2, NULL)",
            rusqlite::params![loser, TYPE_PATH],
        )
        .unwrap();
        v16_rewind::rewind_to_v15(&conn);
        conn.execute_batch("PRAGMA user_version = 14").unwrap();
    }
    let conn = knx_productdb::open_and_migrate(&path).unwrap();
    assert_eq!(
        type_row(&conn, "PT-Time"),
        ("Other".into(), None, None, None, None, None),
        "first writer wins, in the migration exactly as at ingest"
    );
    // The loser's own report is still brought to v15 shape, as a fresh
    // ingest of it would report its constructs too.
    let loser_rows = |kind: &str, name: &str| -> i64 {
        conn.query_row(
            "SELECT count(*) FROM ingest_unknown AS u
             JOIN source_file AS s ON s.sha256 = u.source_sha256
             WHERE s.source_path = 'M-00FA/loser.xml' AND u.kind = ?1 AND u.name = ?2",
            [kind, name],
            |r| r.get(0),
        )
        .unwrap()
    };
    assert_eq!(loser_rows("Element", "TypeTime"), 0, "stale row retired");
    assert_eq!(loser_rows("Attribute", "Unit"), 1, "attribute row written");
}

/// Review follow-ups: (a) a schema-14 program's namespace-prefixed
/// attribute on `TypeTime` leaves the *same* rows after the backfill as a
/// fresh ingest (the backfill runs the scheme-evidence reconciliation
/// ingest runs); (b) a `TypeTime` that is *not* a `ParameterType`'s
/// type-deciding child keeps its generic element report — only the
/// type-deciding occurrences are retired. Both a stray one elsewhere and,
/// the case that shares the retired row's `(xpath, name)`, a second child
/// of a `ParameterType` whose first child already decided its kind.
#[test]
fn v14_to_v15_matches_fresh_ingest_for_prefixed_attributes_and_non_deciding_occurrences() {
    let program = PROGRAM
        .replace(
            r#"xmlns="http://knx.org/xml/project/20""#,
            r#"xmlns="http://knx.org/xml/project/14" xmlns:e="urn:example""#,
        )
        .replace(
            r#"<TypeTime SizeInBit="16" Unit="Seconds""#,
            r#"<TypeTime e:Precision="ms" SizeInBit="16" Unit="Seconds""#,
        )
        .replace(
            "<Parameters />",
            r#"<Parameters /><Stray><TypeTime Unit="Hours" /></Stray>"#,
        )
        .replace(
            r#"<TypeColor Space="HSV" />"#,
            r#"<TypeColor Space="HSV" /><TypeTime Unit="Days" />"#,
        );
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let (fresh_types, fresh_unknown) = {
        let conn = knx_productdb::open_and_migrate(&path).unwrap();
        knx_productdb::ingest_file(&conn, "M-00FA/kinds.xml", program.as_bytes()).unwrap();
        let unknown = unknown_rows(&conn);
        assert!(
            unknown
                .iter()
                .any(|(_, k, n, _, _)| k == "Attribute" && n.contains("Precision")),
            "the fixture really does exercise a prefixed attribute: {unknown:#?}"
        );
        assert!(
            unknown.iter().any(|(x, k, n, _, _)| k == "Element"
                && n == "TypeTime"
                && x.ends_with("/Static/Stray")),
            "the stray TypeTime is a generic unknown element: {unknown:#?}"
        );
        assert!(
            unknown.iter().any(|(x, k, n, c, _)| x == TYPE_PATH
                && k == "Element"
                && n == "TypeTime"
                && *c == 1),
            "the second child of PT-Color2 is reported at the type path: {unknown:#?}"
        );
        let types = ["PT-Color", "PT-Time", "PT-Color2", "PT-Time2"].map(|id| type_row(&conn, id));
        rewind_to_v14(&conn);
        // v14's generic arm did report the non-deciding child's own
        // attribute at the type-child path; the rewind removed it with the
        // rest, so put back exactly that row.
        conn.execute(
            "INSERT INTO ingest_unknown (source_sha256, program_id, xpath, kind, name, occurrences, sample)
             SELECT sha256, NULL, ?1, 'Attribute', 'Unit', 1, 'Days' FROM source_file",
            [format!("{TYPE_PATH}/TypeTime")],
        )
        .unwrap();
        (types, unknown)
    };
    let conn = knx_productdb::open_and_migrate(&path).unwrap();
    assert_eq!(
        ["PT-Color", "PT-Time", "PT-Color2", "PT-Time2"].map(|id| type_row(&conn, id)),
        fresh_types
    );
    assert_eq!(unknown_rows(&conn), fresh_unknown);
}

/// A stored blob that no longer parses must not keep the database from
/// opening: the backfill rolls back that blob alone and records a named
/// `ParameterKindBackfillError`, leaving its row `Other` rather than
/// guessing.
#[test]
fn v14_to_v15_records_a_corrupt_blob_and_still_opens() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    {
        let conn = knx_productdb::open_and_migrate(&path).unwrap();
        knx_productdb::ingest_file(&conn, "M-00FA/kinds.xml", PROGRAM.as_bytes()).unwrap();
        rewind_to_v14(&conn);
        let mut corrupt = PROGRAM.as_bytes().to_vec();
        corrupt.extend_from_slice(b"</Unbalanced>");
        conn.execute("UPDATE source_file SET bytes = ?1", [corrupt])
            .unwrap();
    }
    let conn = knx_productdb::open_and_migrate(&path).unwrap();
    assert_eq!(type_row(&conn, "PT-Time").0, "Other");
    let failures: i64 = conn
        .query_row(
            "SELECT count(*) FROM ingest_unknown
             WHERE kind = 'ParameterKindBackfillError' AND name = 'backfill_color_time_kinds'
               AND sample IS NOT NULL",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(failures, 1);
}

/// Characterization, goal §2.9 "transformations and allocator arguments":
/// `ParameterCalculation` (corpus 1,236 in 91 programs) and `Allocator`
/// (94 in 14) are neither typed nor evaluated. They must be reported —
/// elements and attributes — and their bytes retained, not skipped.
#[test]
fn parameter_calculations_and_allocators_are_reported_and_retained() {
    let (_dir, conn) = db();
    let xml = PROGRAM.replace(
        "<Parameters />",
        r#"<Parameters />
            <Allocators>
              <Allocator Id="M-00FA_A-0009-10-ABCD_L-1" Name="Channels" Start="1" maxInclusive="8" />
            </Allocators>
            <ParameterCalculations>
              <ParameterCalculation Id="M-00FA_A-0009-10-ABCD_PC-1" Name="scale" Language="JavaScript"
                                    LRTransformationFunc="toRight" RLTransformationFunc="toLeft">
                <LParameters><ParameterRefRef RefId="P-1_R-1" /></LParameters>
                <RParameters><ParameterRefRef RefId="P-2_R-1" /></RParameters>
              </ParameterCalculation>
            </ParameterCalculations>"#,
    );
    knx_productdb::ingest_file(&conn, "M-00FA/kinds.xml", xml.as_bytes()).unwrap();
    let rows = unknown_rows(&conn);
    let static_path =
        "/KNX/ManufacturerData/Manufacturer/ApplicationPrograms/ApplicationProgram/Static";
    for (parent, element) in [
        (static_path.to_string(), "Allocators"),
        (format!("{static_path}/Allocators"), "Allocator"),
        (static_path.to_string(), "ParameterCalculations"),
        (
            format!("{static_path}/ParameterCalculations"),
            "ParameterCalculation",
        ),
    ] {
        assert!(
            rows.iter()
                .any(|(x, k, n, _, _)| x == &parent && k == "Element" && n == element),
            "{element} must be reported as an unknown element: {rows:#?}"
        );
    }
    for (element, attr) in [
        ("Allocators/Allocator", "Start"),
        ("Allocators/Allocator", "maxInclusive"),
        (
            "ParameterCalculations/ParameterCalculation",
            "LRTransformationFunc",
        ),
        ("ParameterCalculations/ParameterCalculation", "Language"),
    ] {
        let xpath = format!("{static_path}/{element}");
        assert!(
            rows.iter()
                .any(|(x, k, n, _, _)| x == &xpath && k == "Attribute" && n == attr),
            "{element}/@{attr} must be reported: {rows:#?}"
        );
    }
    let retained: Vec<u8> = conn
        .query_row(
            "SELECT bytes FROM source_file WHERE source_path = 'M-00FA/kinds.xml'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        retained,
        xml.as_bytes(),
        "the source bytes are retained verbatim"
    );
}
