//! Protects v29 promotion when retained TypeDate evidence depends on the historical ingest mode.

use super::*;

fn mode_program(ns: u8, sensitive: bool, id: u8) -> Vec<u8> {
    let attr = if sensitive {
        r#" xmlns:y="urn:example""#
    } else {
        ""
    };
    format!(r#"<KNX xmlns="http://knx.org/xml/project/{ns}"><ManufacturerData><Manufacturer RefId="M-0001"><ApplicationPrograms><ApplicationProgram Id="M-0001_A-{id}" Name="Mode"><Static><ParameterTypes><ParameterType Id="M-0001_A-{id}_PT-D"><TypeDate Encoding="DPT 11"{attr}/></ParameterType></ParameterTypes></Static></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#).into_bytes()
}

fn mode_package(ns: u8, program: &[u8]) -> Vec<u8> {
    let master = std::str::from_utf8(MASTER)
        .unwrap()
        .replace("project/14", &format!("project/{ns}"));
    let hardware = format!(
        r#"<KNX xmlns="http://knx.org/xml/project/{ns}"><ManufacturerData><Manufacturer RefId="M-0001"><Hardware><Hardware Id="H-1" Name="Example"/></Hardware></Manufacturer></ManufacturerData></KNX>"#
    );
    archive(&[
        ("knx_master.xml", master.as_bytes()),
        ("M-0001/Hardware.xml", hardware.as_bytes()),
        ("M-0001/M-0001_A-1.xml", program),
    ])
}

/// Recreates the genuine v28 rows for the synthetic two-mode witness: the
/// targeted package reports the whole TypeDate only, while the extended
/// package additionally reports the expanded namespace declaration. Checked
/// against an actual v28 writer during the independent review.
fn rewind_modes(conn: &Connection, program: &[u8], sensitive: bool) {
    let sha = sha256_hex(program);
    let child = format!("{TYPE_PATH}/TypeDate");
    conn.execute(
        "DELETE FROM ingest_unknown WHERE source_sha256 = ?1 AND xpath = ?2",
        params![sha, child],
    )
    .unwrap();
    conn.execute("INSERT INTO ingest_unknown (source_sha256,xpath,kind,name,occurrences) VALUES (?1,?2,'Element','TypeDate',1)", params![sha, TYPE_PATH]).unwrap();
    if sensitive {
        conn.execute("INSERT INTO ingest_unknown (source_sha256,xpath,kind,name,occurrences,sample) VALUES (?1,?2,'Attribute','{http://www.w3.org/2000/xmlns/}y',1,'urn:example')", params![sha, child]).unwrap();
    }
    let packages: Vec<(String, i64)> = conn
        .prepare("SELECT sha256, scheme FROM package ORDER BY sha256")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    for (package, scheme) in packages {
        let count = 1 + i64::from(sensitive && scheme != 14);
        conn.execute(
            "DELETE FROM package_install_unknown WHERE package_sha256=?1",
            [&package],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO package_install_unknown VALUES (?1,0,?2,'Element','TypeDate',1,NULL)",
            params![package, TYPE_PATH],
        )
        .unwrap();
        if count == 2 {
            conn.execute("INSERT INTO package_install_unknown VALUES (?1,1,?2,'Attribute','{http://www.w3.org/2000/xmlns/}y',1,'urn:example')", params![package, child]).unwrap();
        }
        conn.execute("UPDATE package_install_report SET unknown_distinct=?2,unknown_occurrences=?2 WHERE package_sha256=?1", params![package,count]).unwrap();
        conn.execute("UPDATE package_install_count SET count=?2 WHERE package_sha256=?1 AND category='unknown_construct' AND disposition IN ('read','stored')",params![package,count]).unwrap();
        conn.execute(
            "UPDATE package SET unknown_count=?2 WHERE sha256=?1",
            params![package, count],
        )
        .unwrap();
    }
    conn.execute_batch(
        "UPDATE parameter_type SET kind='Other' WHERE kind='Date'; PRAGMA user_version=28;",
    )
    .unwrap();
}

#[test]
fn ambiguous_namespace_modes_preserve_original_source_and_package_counts() {
    for ns in [21, 23] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("products.sqlite");
        let conn = open_and_migrate(&path).unwrap();
        let program = mode_program(ns, true, 1);
        install_package(&conn, "targeted.knxprod", &mode_package(14, &program)).unwrap();
        install_package(&conn, "extended.knxprod", &mode_package(ns, &program)).unwrap();
        rewind_modes(&conn, &program, true);
        let unknown = type_rows(&conn, "ingest_unknown");
        let counters = table(&conn, "SELECT sha256,unknown_count FROM package ORDER BY 1");
        drop(conn);
        let conn = open_and_migrate(&path).unwrap();
        assert_eq!(
            kinds(&conn)[0].1,
            "Other",
            "ambiguous promotion must not invent mode ownership"
        );
        assert_eq!(
            type_rows(&conn, "ingest_unknown"),
            unknown,
            "all original evidence stays"
        );
        assert_eq!(
            table(&conn, "SELECT sha256,unknown_count FROM package ORDER BY 1"),
            counters
        );
        let unavailable: i64 = conn
            .query_row(
                "SELECT count(*) FROM package_install_report WHERE status='unavailable'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(unavailable, 2);
        let failures:i64 = conn.query_row("SELECT count(*) FROM ingest_unknown WHERE kind='ParameterKindBackfillError' AND name='backfill_date_kind'",[],|r|r.get(0)).unwrap();
        assert_eq!(failures, 1);
        let recorded = snapshot(&conn);
        drop(conn);
        assert_eq!(
            snapshot(&open_and_migrate(&path).unwrap()),
            recorded,
            "reopen is idempotent"
        );
    }
}

#[test]
fn invariant_namespace_modes_migrate_instead_of_blanket_refusal() {
    for ns in [21, 23] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("products.sqlite");
        let conn = open_and_migrate(&path).unwrap();
        let program = mode_program(ns, false, 1);
        install_package(&conn, "targeted.knxprod", &mode_package(14, &program)).unwrap();
        install_package(&conn, "extended.knxprod", &mode_package(ns, &program)).unwrap();
        let fresh = snapshot(&conn);
        rewind_modes(&conn, &program, false);
        drop(conn);
        assert_eq!(snapshot(&open_and_migrate(&path).unwrap()), fresh);
    }
}

#[test]
fn an_ambiguous_blob_does_not_stop_an_independent_plain_blob() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let conn = open_and_migrate(&path).unwrap();
    let ambiguous = mode_program(21, true, 1);
    install_package(&conn, "extended.knxprod", &mode_package(21, &ambiguous)).unwrap();
    rewind_modes(&conn, &ambiguous, true);
    let plain = mode_program(14, false, 2);
    knx_productdb::ingest_file(&conn, "M-0001/plain.xml", &plain).unwrap();
    let sha = sha256_hex(&plain);
    conn.execute(
        "DELETE FROM ingest_unknown WHERE source_sha256=?1 AND xpath=?2",
        params![sha, format!("{TYPE_PATH}/TypeDate")],
    )
    .unwrap();
    conn.execute("INSERT INTO ingest_unknown (source_sha256,xpath,kind,name,occurrences) VALUES (?1,?2,'Element','TypeDate',1)",params![sha,TYPE_PATH]).unwrap();
    conn.execute_batch("UPDATE parameter_type SET kind='Other' WHERE kind='Date';")
        .unwrap();
    drop(conn);
    let conn = open_and_migrate(&path).unwrap();
    assert_eq!(
        kinds(&conn),
        vec![
            ("M-0001_A-1_PT-D".into(), "Other".into()),
            ("M-0001_A-2_PT-D".into(), "Date".into())
        ]
    );
}
