//! Source-authenticated successor repairs only proven released Color/Time contamination.

use rusqlite::{types::Value, Connection};

fn program(id: &str, types: &str) -> String {
    format!(
        r#"<ApplicationProgram Id="{id}" Name="winner"><Static><ParameterTypes>{types}</ParameterTypes></Static></ApplicationProgram>"#
    )
}

fn fixture(id: &str) -> String {
    let first = program(
        id,
        r#"<ParameterType Id="C" Name="original color"><TypeHologram/></ParameterType><ParameterType Id="T" Name="original time"><TypeHologram/></ParameterType>"#,
    );
    let loser = program(
        id,
        r#"<ParameterType Id="C" Name="loser color"><TypeColor Space="RGB"/></ParameterType><ParameterType Id="T" Name="loser time"><TypeTime SizeInBit="16" minInclusive="1" maxInclusive="3600"/></ParameterType>"#,
    );
    format!(
        r#"<KNX xmlns="http://knx.org/xml/project/14"><ManufacturerData><Manufacturer RefId="M-00FA"><ApplicationPrograms>{first}{loser}</ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#
    )
}

fn rows(conn: &Connection) -> Vec<Vec<Value>> {
    let mut stmt = conn
        .prepare("SELECT * FROM parameter_type ORDER BY program_id,id")
        .unwrap();
    let n = stmt.column_count();
    stmt.query_map([], |r| (0..n).map(|i| r.get(i)).collect())
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

/// Authored known-state fixture, not an authentic historical producer database.
fn contaminate(conn: &Connection) {
    conn.execute_batch("UPDATE parameter_type SET kind='Color' WHERE id='C'; UPDATE parameter_type SET kind='Time',size_in_bit=16,min_inclusive='1',max_inclusive='3600' WHERE id='T'; PRAGMA user_version=31;").unwrap();
}

#[test]
fn contaminated_v31_reopens_with_source_winner_and_null_time_fields() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let expected = {
        let conn = knx_productdb::open_and_migrate(&path).unwrap();
        knx_productdb::ingest_file(&conn, "M-00FA/source.xml", fixture("A").as_bytes()).unwrap();
        let expected = rows(&conn);
        contaminate(&conn);
        assert_ne!(rows(&conn), expected);
        expected
    };
    let conn = knx_productdb::open_and_migrate(&path).unwrap();
    assert_eq!(
        rows(&conn),
        expected,
        "public opener must undo proven losing promotions including non-NULL Time fields"
    );
    let version: i64 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap();
    assert_eq!(version, knx_productdb::CURRENT_PRODUCTDB_VERSION);
    drop(conn);
    let conn = knx_productdb::open_and_migrate(&path).unwrap();
    assert_eq!(rows(&conn), expected);
}

fn query(conn: &Connection, sql: &str) -> Vec<Vec<Value>> {
    let mut stmt = conn.prepare(sql).unwrap();
    let n = stmt.column_count();
    stmt.query_map([], |r| (0..n).map(|i| r.get(i)).collect())
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

/// Compare all unrelated persisted tables, including complete original history.
fn untouched(conn: &Connection) -> Vec<(String, Vec<Vec<Value>>)> {
    let names = query(conn, "SELECT name FROM sqlite_schema WHERE type='table' AND name NOT IN ('parameter_type','ingest_unknown') ORDER BY name");
    names
        .into_iter()
        .map(|row| {
            let Value::Text(name) = &row[0] else { panic!() };
            let mut data = query(
                conn,
                &format!("SELECT * FROM \"{}\"", name.replace('"', "\"\"")),
            );
            data.sort_by_key(|row| format!("{row:?}"));
            (name.clone(), data)
        })
        .collect()
}

fn package(xml: &str) -> Vec<u8> {
    use std::io::{Cursor, Write};
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let master = r#"<KNX xmlns="http://knx.org/xml/project/14"><MasterData><Manufacturers><Manufacturer Id="M-00FA" Name="Fictional"/></Manufacturers></MasterData></KNX>"#;
    for (name, bytes) in [("knx_master.xml", master), ("M-00FA/A.xml", xml)] {
        zip.start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(bytes.as_bytes()).unwrap();
    }
    zip.finish().unwrap().into_inner()
}

#[test]
fn date_winners_legitimate_color_time_and_unique_controls_preserve_every_other_row() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let conn = knx_productdb::open_and_migrate(&path).unwrap();
    let date = fixture("A").replace("<TypeHologram/>", "<TypeDate Encoding=\"DPT 11\"/>");
    let positive = program(
        "B",
        r#"<ParameterType Id="C"><TypeColor/></ParameterType><ParameterType Id="T"><TypeTime SizeInBit="32" minInclusive="2" maxInclusive="7200"/></ParameterType>"#,
    );
    let xml = date.replace(
        "</ApplicationPrograms>",
        &format!("{positive}</ApplicationPrograms>"),
    );
    knx_productdb::install_package(&conn, "fictional.knxprod", &package(&xml)).unwrap();
    assert!(!query(&conn, "SELECT * FROM package_install_count").is_empty());
    assert!(!query(&conn, "SELECT * FROM package_conflict").is_empty());
    let expected = rows(&conn);
    conn.execute_batch("UPDATE parameter_type SET kind='Color' WHERE program_id='A' AND id='C'; UPDATE parameter_type SET kind='Time',size_in_bit=16,min_inclusive='1',max_inclusive='3600' WHERE program_id='A' AND id='T'; PRAGMA user_version=31").unwrap();
    let preserved = untouched(&conn);
    let unknown = query(&conn, "SELECT * FROM ingest_unknown ORDER BY id");
    drop(conn);
    let conn = knx_productdb::open_and_migrate(&path).unwrap();
    assert_eq!(rows(&conn), expected);
    assert_eq!(untouched(&conn), preserved);
    assert_eq!(
        query(&conn, "SELECT * FROM ingest_unknown ORDER BY id"),
        unknown
    );
    let version: i64 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap();
    assert_eq!(version, knx_productdb::CURRENT_PRODUCTDB_VERSION);
}

#[test]
fn cross_file_loser_cannot_change_foreign_winner_and_correct_database_is_noop() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let conn = knx_productdb::open_and_migrate(&path).unwrap();
    let first = fixture("A");
    knx_productdb::ingest_file(&conn, "M-00FA/winner.xml", first.as_bytes()).unwrap();
    let loser = first.replace("<TypeHologram/>", "<TypeColor/>");
    knx_productdb::ingest_file(&conn, "M-00FA/loser.xml", loser.as_bytes()).unwrap();
    let own = fixture("B");
    let own = own
        .split("<ApplicationPrograms>")
        .nth(1)
        .unwrap()
        .split("</ApplicationPrograms>")
        .next()
        .unwrap();
    // One source owns B but loses A to the first file. It must still be
    // reparsed with A pre-seeded as foreign, not admitted as a new winner.
    let loser = loser.replace(
        "</ApplicationPrograms>",
        &format!("{own}</ApplicationPrograms>"),
    );
    knx_productdb::ingest_file(&conn, "M-00FA/mixed.xml", loser.as_bytes()).unwrap();
    let expected = rows(&conn);
    conn.execute_batch("UPDATE parameter_type SET kind='Color' WHERE program_id='B' AND id='C'")
        .unwrap();
    let preserved = untouched(&conn);
    let unknown = query(&conn, "SELECT * FROM ingest_unknown ORDER BY id");
    conn.execute_batch("PRAGMA user_version=31").unwrap();
    drop(conn);
    let conn = knx_productdb::open_and_migrate(&path).unwrap();
    assert_eq!(rows(&conn), expected);
    assert_eq!(untouched(&conn), preserved);
    assert_eq!(
        query(&conn, "SELECT * FROM ingest_unknown ORDER BY id"),
        unknown
    );
}

fn authenticated_replacement(conn: &Connection, old: &str, bytes: &[u8]) {
    let new = knx_productdb::sha256_hex(bytes);
    conn.execute(
        "UPDATE source_file SET sha256=?1,bytes=?2,len=?3 WHERE sha256=?4",
        rusqlite::params![new, bytes, bytes.len() as i64, old],
    )
    .unwrap();
    conn.execute(
        "UPDATE application_program SET source_sha256=?1 WHERE source_sha256=?2",
        [&new, old],
    )
    .unwrap();
    conn.execute(
        "UPDATE package_member SET source_sha256=?1 WHERE source_sha256=?2",
        [&new, old],
    )
    .unwrap();
}

#[test]
fn unproven_sources_refuse_atomically_with_named_diagnostic_while_independent_source_repairs() {
    for fault in [
        "hash",
        "length",
        "parse",
        "duplicate-type",
        "depth",
        "classification",
        "missing-source",
        "owner",
        "missing-owner",
        "missing-key",
        "extra-key",
        "metadata",
        "tuple",
    ] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("products.sqlite");
        let conn = knx_productdb::open_and_migrate(&path).unwrap();
        let bad = fixture("A");
        knx_productdb::install_package(&conn, "fictional.knxprod", &package(&bad)).unwrap();
        let independent = fixture("B");
        knx_productdb::ingest_file(&conn, "M-00FA/independent.xml", independent.as_bytes())
            .unwrap();
        let good_expected = query(
            &conn,
            "SELECT * FROM parameter_type WHERE program_id='B' ORDER BY id",
        );
        let old = knx_productdb::sha256_hex(bad.as_bytes());
        contaminate(&conn);
        // Deliberately corrupt persisted ownership/source states independently
        // of the installer's foreign-key admission, before the real reopen.
        conn.pragma_update(None, "foreign_keys", false).unwrap();
        match fault {
            "hash" => {
                // A losing, non-normalized lexeme changes without affecting
                // authoritative rows or the v15-bad tuple. Without the hash
                // guard, tuple proof would admit the forbidden repair.
                let changed = bad.replace("Space=\"RGB\"", "Space=\"HSV\"");
                assert_ne!(changed, bad);
                assert_eq!(changed.len(), bad.len());
                conn.execute(
                    "UPDATE source_file SET bytes=?1 WHERE sha256=?2",
                    rusqlite::params![changed.as_bytes(), old],
                )
                .unwrap();
            }
            "length" => {
                conn.execute("UPDATE source_file SET len=len+1 WHERE sha256=?1", [&old])
                    .unwrap();
            }
            "parse" => {
                authenticated_replacement(&conn, &old, format!("{bad}</Mismatched>").as_bytes())
            }
            "duplicate-type" => {
                let changed = bad.replacen(
                    "</ParameterTypes>",
                    "<ParameterType Id=\"C\"><TypeColor/></ParameterType></ParameterTypes>",
                    1,
                );
                assert_ne!(changed, bad);
                authenticated_replacement(&conn, &old, changed.as_bytes());
            }
            "depth" => {
                let changed = bad
                    .replace("<Static>", &format!("<Static>{}", "<N>".repeat(1024)))
                    .replace("</Static>", &format!("{}</Static>", "</N>".repeat(1024)));
                assert_ne!(changed, bad);
                authenticated_replacement(&conn, &old, changed.as_bytes());
            }
            "classification" => {
                authenticated_replacement(&conn, &old, b"not an application program")
            }
            "missing-source" => {
                conn.execute("DELETE FROM source_file WHERE sha256=?1", [&old])
                    .unwrap();
            }
            "owner" => {
                conn.execute(
                    "UPDATE application_program SET id='unmatched' WHERE id='A'",
                    [],
                )
                .unwrap();
            }
            "missing-owner" => {
                conn.execute("DELETE FROM application_program WHERE id='A'", [])
                    .unwrap();
            }
            "missing-key" => {
                conn.execute(
                    "DELETE FROM parameter_type WHERE program_id='A' AND id='T'",
                    [],
                )
                .unwrap();
            }
            "extra-key" => {
                conn.execute(
                    "INSERT INTO parameter_type(program_id,id,kind) VALUES('A','ZZ','Other')",
                    [],
                )
                .unwrap();
            }
            "metadata" => {
                conn.execute("UPDATE parameter_type SET name='not source-derived' WHERE program_id='A' AND id='T'", []).unwrap();
            }
            "tuple" => {
                conn.execute("UPDATE parameter_type SET max_inclusive='unproven' WHERE program_id='A' AND id='T'", []).unwrap();
            }
            _ => unreachable!(),
        }
        let bad_before = query(
            &conn,
            "SELECT * FROM parameter_type WHERE program_id<>'B' ORDER BY program_id,id",
        );
        let old_unknown = query(&conn, "SELECT * FROM ingest_unknown ORDER BY id");
        let preserved = untouched(&conn);
        drop(conn);
        let conn = knx_productdb::open_and_migrate(&path).unwrap();
        assert_eq!(query(&conn, "SELECT * FROM parameter_type WHERE program_id<>'B' ORDER BY program_id,id"), bad_before, "{fault}: entire refused source remains unchanged, even when an earlier C row was repairable");
        assert_eq!(
            query(
                &conn,
                "SELECT * FROM parameter_type WHERE program_id='B' ORDER BY id"
            ),
            good_expected,
            "{fault}: valid independent source repairs"
        );
        assert_eq!(
            untouched(&conn),
            preserved,
            "{fault}: original persisted history/counters/sources stay exact"
        );
        assert_eq!(
            query(
                &conn,
                // v33's own refusal of a blob it cannot re-read is a separate, named record.
                "SELECT * FROM ingest_unknown
                 WHERE name NOT IN ('repair_color_time_kinds','reparse_rename_leaves') ORDER BY id"
            ),
            old_unknown
        );
        let diagnostic = query(&conn, "SELECT source_sha256,kind,name,occurrences,sample FROM ingest_unknown WHERE name='repair_color_time_kinds'");
        assert_eq!(diagnostic.len(), 1, "{fault}: one named source refusal");
        assert_eq!(diagnostic[0][1], Value::Text("PromotionRepairError".into()));
        assert_eq!(diagnostic[0][3], Value::Integer(1));
        let Value::Text(sha) = &diagnostic[0][0] else {
            panic!()
        };
        let public = knx_productdb::source_diagnostics(&conn, sha);
        if matches!(
            fault,
            "parse" | "duplicate-type" | "depth" | "classification"
        ) {
            assert!(
                matches!(public, Err(knx_productdb::ProductDbError::Xml { cause, .. }) if cause.contains("unmeasured source") && cause.contains("v32 Color/Time repair")),
                "a named refusal must not manufacture measured source evidence"
            );
        } else {
            assert!(
                public
                    .unwrap()
                    .unwrap()
                    .iter()
                    .any(|d| d.name == "repair_color_time_kinds"),
                "named refusal must be readable through public diagnostics"
            );
        }
        let all_unknown = query(&conn, "SELECT * FROM ingest_unknown ORDER BY id");
        drop(conn);
        let conn = knx_productdb::open_and_migrate(&path).unwrap();
        assert_eq!(
            query(&conn, "SELECT * FROM ingest_unknown ORDER BY id"),
            all_unknown
        );
        assert_eq!(untouched(&conn), preserved);
    }
}

#[test]
fn native_storage_failure_is_fatal_and_rolls_back_migration_version_and_rows() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let conn = knx_productdb::open_and_migrate(&path).unwrap();
    knx_productdb::ingest_file(&conn, "M-00FA/source.xml", fixture("A").as_bytes()).unwrap();
    contaminate(&conn);
    conn.execute_batch("CREATE TRIGGER refuse_repair BEFORE UPDATE ON parameter_type WHEN OLD.id='T' BEGIN SELECT RAISE(ABORT,'storage refused'); END").unwrap();
    let original = rows(&conn);
    let unknown = query(&conn, "SELECT * FROM ingest_unknown ORDER BY id");
    drop(conn);
    assert!(matches!(
        knx_productdb::open_and_migrate(&path),
        Err(knx_productdb::ProductDbError::Sqlite(_))
    ));
    let conn = Connection::open(&path).unwrap();
    assert_eq!(rows(&conn), original);
    assert_eq!(
        query(&conn, "SELECT * FROM ingest_unknown ORDER BY id"),
        unknown
    );
    let version: i64 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap();
    assert_eq!(version, 31);
}

#[test]
fn first_color_time_remain_unchanged_despite_losing_same_blob_unknown_types() {
    let first = program(
        "A",
        r#"<ParameterType Id="C" Name="good color"><TypeColor/></ParameterType><ParameterType Id="T" Name="good time"><TypeTime SizeInBit="32" minInclusive="2" maxInclusive="7200"/></ParameterType>"#,
    );
    let second = program(
        "A",
        r#"<ParameterType Id="C"><TypeHologram/></ParameterType><ParameterType Id="T"><TypeDate/></ParameterType>"#,
    );
    let xml = format!(
        r#"<KNX xmlns="http://knx.org/xml/project/14"><ManufacturerData><Manufacturer RefId="M-00FA"><ApplicationPrograms>{first}{second}</ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#
    );
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let conn = knx_productdb::open_and_migrate(&path).unwrap();
    knx_productdb::install_package(&conn, "fictional.knxprod", &package(&xml)).unwrap();
    let expected = rows(&conn);
    let preserved = untouched(&conn);
    let unknown = query(&conn, "SELECT * FROM ingest_unknown ORDER BY id");
    conn.execute_batch("PRAGMA user_version=31").unwrap();
    drop(conn);
    let conn = knx_productdb::open_and_migrate(&path).unwrap();
    assert_eq!(rows(&conn), expected);
    assert_eq!(untouched(&conn), preserved);
    assert_eq!(
        query(&conn, "SELECT * FROM ingest_unknown ORDER BY id"),
        unknown
    );
    let version: i64 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap();
    assert_eq!(version, knx_productdb::CURRENT_PRODUCTDB_VERSION);
}

#[test]
fn successor_refusal_reader_admits_only_the_named_error_shape_without_inventing_measurements() {
    for measured in [false, true] {
        for corruption in ["path", "name", "count", "sample-null", "sample-empty"] {
            let dir = tempfile::tempdir().unwrap();
            let conn =
                knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
            let xml = fixture("A");
            let sha = knx_productdb::sha256_hex(xml.as_bytes());
            knx_productdb::ingest_file(&conn, "M-00FA/source.xml", xml.as_bytes()).unwrap();
            if !measured {
                conn.execute("DELETE FROM source_parse_evidence WHERE sha256=?1", [&sha])
                    .unwrap();
            }
            conn.execute("INSERT INTO ingest_unknown(source_sha256,program_id,xpath,kind,name,occurrences,sample) VALUES(?1,NULL,'source.xml','PromotionRepairError','repair_color_time_kinds',1,'v32 Color/Time repair: source refused')", [&sha]).unwrap();
            let public = knx_productdb::source_diagnostics(&conn, &sha);
            if measured {
                assert!(public
                    .unwrap()
                    .unwrap()
                    .iter()
                    .any(|d| d.name == "repair_color_time_kinds"));
            } else {
                assert!(
                    matches!(public, Err(knx_productdb::ProductDbError::Xml{cause,..}) if cause.contains("unmeasured source"))
                );
            }
            let update = match corruption {
                "path" => "xpath=''",
                "name" => "name='unrecognized-repair'",
                "count" => "occurrences=2",
                "sample-null" => "sample=NULL",
                "sample-empty" => "sample=''",
                _ => unreachable!(),
            };
            conn.execute_batch(&format!(
                "UPDATE ingest_unknown SET {update} WHERE kind='PromotionRepairError'"
            ))
            .unwrap();
            assert!(
                matches!(knx_productdb::source_diagnostics(&conn, &sha), Err(knx_productdb::ProductDbError::Xml{cause,..}) if cause=="invalid persisted diagnostic kind or count"),
                "{measured}/{corruption}"
            );
        }
    }
}

#[test]
fn orphan_does_not_invalidate_unrelated_type_free_unmeasured_source() {
    for orphan in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("products.sqlite");
        let conn = knx_productdb::open_and_migrate(&path).unwrap();
        let target = fixture("A");
        let target_sha = knx_productdb::sha256_hex(target.as_bytes());
        knx_productdb::install_package(&conn, "fictional.knxprod", &package(&target)).unwrap();
        let other = format!(
            r#"<KNX xmlns="http://knx.org/xml/project/14"><ManufacturerData><Manufacturer RefId="M-00FA"><ApplicationPrograms>{}</ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#,
            program("B", "")
        );
        let other_sha = knx_productdb::sha256_hex(other.as_bytes());
        knx_productdb::ingest_file(&conn, "M-00FA/unrelated.xml", other.as_bytes()).unwrap();
        assert!(query(&conn, "SELECT * FROM parameter_type WHERE program_id='B'").is_empty());
        let changed = other.replace("Name=\"winner\"", "Name=\"edited\"");
        assert_ne!(changed, other);
        assert_eq!(changed.len(), other.len());
        conn.execute(
            "UPDATE source_file SET bytes=?1 WHERE sha256=?2",
            rusqlite::params![changed.as_bytes(), other_sha],
        )
        .unwrap();
        conn.execute(
            "DELETE FROM source_parse_evidence WHERE sha256=?1",
            [&other_sha],
        )
        .unwrap();
        assert!(matches!(
            knx_productdb::source_diagnostics(&conn, &other_sha),
            Ok(None)
        ));
        contaminate(&conn);
        if orphan {
            conn.pragma_update(None, "foreign_keys", false).unwrap();
            conn.execute("DELETE FROM application_program WHERE id='A'", [])
                .unwrap();
        }
        let history = untouched(&conn);
        drop(conn);
        let conn = knx_productdb::open_and_migrate(&path).unwrap();
        let public = knx_productdb::source_diagnostics(&conn, &other_sha);
        assert!(matches!(public, Ok(None)), "orphan={orphan}: unrelated type-free unmeasured source must not gain a migration error; actual={public:?}");
        assert_eq!(untouched(&conn), history);
        let refused_sources = query(
            &conn,
            "SELECT source_sha256 FROM ingest_unknown WHERE name='repair_color_time_kinds'",
        );
        let expected = if orphan {
            vec![vec![Value::Text(target_sha)]]
        } else {
            vec![]
        };
        assert_eq!(
            refused_sources, expected,
            "the attributable orphan must still get exactly its own refusal"
        );
    }
}

#[test]
fn ambiguous_or_unmeasured_orphan_declarations_do_not_invent_source_ownership() {
    for ambiguity in ["multiple-sources", "unmeasured-scan"] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("products.sqlite");
        let conn = knx_productdb::open_and_migrate(&path).unwrap();
        let target = fixture("A");
        let target_sha = knx_productdb::sha256_hex(target.as_bytes());
        knx_productdb::install_package(&conn, "fictional.knxprod", &package(&target)).unwrap();
        if ambiguity == "multiple-sources" {
            let other = target.replace(
                "</ApplicationPrograms>",
                &format!("{}</ApplicationPrograms>", program("B", "")),
            );
            assert_ne!(other, target);
            knx_productdb::ingest_file(&conn, "M-00FA/other-declaration.xml", other.as_bytes())
                .unwrap();
            assert_eq!(query(&conn, "SELECT DISTINCT source_sha256 FROM source_identity WHERE table_name='application_program' AND logical_id='A'").len(), 2);
        } else {
            conn.execute("UPDATE source_identity_scan SET status='unavailable',reason='authored missing measurement' WHERE source_sha256=?1", [&target_sha]).unwrap();
        }
        contaminate(&conn);
        conn.pragma_update(None, "foreign_keys", false).unwrap();
        conn.execute("DELETE FROM application_program WHERE id='A'", [])
            .unwrap();
        let types = rows(&conn);
        let history = untouched(&conn);
        let unknown = query(&conn, "SELECT * FROM ingest_unknown ORDER BY id");
        drop(conn);
        let conn = knx_productdb::open_and_migrate(&path).unwrap();
        assert_eq!(rows(&conn), types, "{ambiguity}: no orphan adoption");
        assert_eq!(untouched(&conn), history);
        assert_eq!(
            query(&conn, "SELECT * FROM ingest_unknown ORDER BY id"),
            unknown,
            "{ambiguity}: no unproven source-specific refusal"
        );
    }
}

#[test]
fn native_diagnostic_insert_failure_rolls_back_complete_migration() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let conn = knx_productdb::open_and_migrate(&path).unwrap();
    let bad = fixture("A");
    knx_productdb::ingest_file(&conn, "M-00FA/bad.xml", bad.as_bytes()).unwrap();
    knx_productdb::ingest_file(&conn, "M-00FA/good.xml", fixture("B").as_bytes()).unwrap();
    contaminate(&conn);
    conn.execute(
        "UPDATE source_file SET len=len+1 WHERE sha256=?1",
        [knx_productdb::sha256_hex(bad.as_bytes())],
    )
    .unwrap();
    conn.execute_batch("CREATE TRIGGER refuse_diagnostic BEFORE INSERT ON ingest_unknown WHEN NEW.name='repair_color_time_kinds' BEGIN SELECT RAISE(ABORT,'diagnostic storage refused'); END").unwrap();
    let before = rows(&conn);
    let history = untouched(&conn);
    let unknown = query(&conn, "SELECT * FROM ingest_unknown ORDER BY id");
    drop(conn);
    let error = knx_productdb::open_and_migrate(&path).unwrap_err();
    assert!(
        matches!(error, knx_productdb::ProductDbError::Sqlite(_)),
        "{error:?}"
    );
    let conn = Connection::open(&path).unwrap();
    assert_eq!(rows(&conn), before);
    assert_eq!(untouched(&conn), history);
    assert_eq!(
        query(&conn, "SELECT * FROM ingest_unknown ORDER BY id"),
        unknown
    );
    let version: i64 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap();
    assert_eq!(version, 31);
}
