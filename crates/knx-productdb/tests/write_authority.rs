//! ADR-0080: the product database records the write authority a program
//! declares — `ParameterRef/@Access` and which `ParameterRef`s a
//! `ParameterCalculation` names — at install and by the v19 -> v20 backfill.
//!
//! Synthetic fixture; attribute shapes follow the corpus census in the ADR,
//! no manufacturer data is copied.

use knx_productdb::query::{parameter_views, write_authority, CalculationSide};
use rusqlite::Connection;

const PROGRAM_ID: &str = "M-00FA_A-0009-10-ABCD";

const PROGRAM: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/20">
  <ManufacturerData>
    <Manufacturer RefId="M-00FA">
      <ApplicationPrograms>
        <ApplicationProgram Id="M-00FA_A-0009-10-ABCD" Name="Authority" ApplicationNumber="9"
                            ApplicationVersion="16" ProgramType="ApplicationProgram"
                            MaskVersion="MV-07B0" PeiType="0" LoadProcedureStyle="MergedProcedure"
                            DefaultLanguage="en-US">
          <Static>
            <ParameterTypes>
              <ParameterType Id="M-00FA_A-0009-10-ABCD_PT-N" Name="n">
                <TypeNumber SizeInBit="8" Type="unsignedInt" minInclusive="0" maxInclusive="99" />
              </ParameterType>
            </ParameterTypes>
            <Parameters>
              <Parameter Id="M-00FA_A-0009-10-ABCD_P-1" Name="open" ParameterType="M-00FA_A-0009-10-ABCD_PT-N" Text="Open" Value="1" Access="None" />
              <Parameter Id="M-00FA_A-0009-10-ABCD_P-2" Name="hidden" ParameterType="M-00FA_A-0009-10-ABCD_PT-N" Text="Hidden" Value="2" Access="None" />
              <Parameter Id="M-00FA_A-0009-10-ABCD_P-3" Name="plain" ParameterType="M-00FA_A-0009-10-ABCD_PT-N" Text="Plain" Value="3" />
              <Parameter Id="M-00FA_A-0009-10-ABCD_P-4" Name="minutes" ParameterType="M-00FA_A-0009-10-ABCD_PT-N" Text="Minutes" Value="4" />
              <Parameter Id="M-00FA_A-0009-10-ABCD_P-5" Name="seconds" ParameterType="M-00FA_A-0009-10-ABCD_PT-N" Text="Seconds" Value="5" Access="None" />
            </Parameters>
            <ParameterRefs>
              <ParameterRef Id="M-00FA_A-0009-10-ABCD_P-1_R-1" RefId="M-00FA_A-0009-10-ABCD_P-1" Access="ReadWrite" />
              <ParameterRef Id="M-00FA_A-0009-10-ABCD_P-2_R-1" RefId="M-00FA_A-0009-10-ABCD_P-2" />
              <ParameterRef Id="M-00FA_A-0009-10-ABCD_P-3_R-1" RefId="M-00FA_A-0009-10-ABCD_P-3" Access="Read" />
              <ParameterRef Id="M-00FA_A-0009-10-ABCD_P-4_R-1" RefId="M-00FA_A-0009-10-ABCD_P-4" />
              <ParameterRef Id="M-00FA_A-0009-10-ABCD_P-5_R-1" RefId="M-00FA_A-0009-10-ABCD_P-5" />
            </ParameterRefs>
            <ParameterCalculations>
              <ParameterCalculation Id="M-00FA_A-0009-10-ABCD_PC-1" Name="toSeconds" Language="JavaScript"
                                    LRTransformationFunc="lr" RLTransformationFunc="rl">
                <LParameters><ParameterRefRef RefId="M-00FA_A-0009-10-ABCD_P-4_R-1" /></LParameters>
                <RParameters><ParameterRefRef RefId="M-00FA_A-0009-10-ABCD_P-5_R-1" /></RParameters>
              </ParameterCalculation>
            </ParameterCalculations>
          </Static>
          <Dynamic><ChannelIndependentBlock><ParameterBlock Id="M-00FA_A-0009-10-ABCD_PB-1" Text="B">
            <ParameterRefRef RefId="M-00FA_A-0009-10-ABCD_P-1_R-1" />
            <ParameterRefRef RefId="M-00FA_A-0009-10-ABCD_P-2_R-1" />
            <ParameterRefRef RefId="M-00FA_A-0009-10-ABCD_P-3_R-1" />
            <ParameterRefRef RefId="M-00FA_A-0009-10-ABCD_P-4_R-1" />
            <ParameterRefRef RefId="M-00FA_A-0009-10-ABCD_P-5_R-1" />
          </ParameterBlock></ChannelIndependentBlock></Dynamic>
        </ApplicationProgram>
      </ApplicationPrograms>
    </Manufacturer>
  </ManufacturerData>
</KNX>"#;

fn r(n: u32) -> String {
    format!("{PROGRAM_ID}_P-{n}_R-1")
}

fn installed() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let conn = knx_productdb::open_and_migrate(&path).unwrap();
    knx_productdb::ingest_file(&conn, "M-00FA/authority.xml", PROGRAM.as_bytes()).unwrap();
    (dir, path)
}

fn ref_access(conn: &Connection) -> Vec<(String, Option<String>, Option<String>)> {
    let mut views: Vec<_> = parameter_views(conn, PROGRAM_ID, None)
        .unwrap()
        .into_iter()
        .map(|v| (v.id, v.access, v.ref_access))
        .collect();
    views.sort();
    views
}

fn assert_recorded(conn: &Connection) {
    assert_eq!(
        ref_access(conn),
        vec![
            (r(1), Some("None".into()), Some("ReadWrite".into())),
            (r(2), Some("None".into()), None),
            (r(3), None, Some("Read".into())),
            (r(4), None, None),
            (r(5), Some("None".into()), None),
        ]
    );
    let authority = write_authority(conn, PROGRAM_ID).unwrap();
    assert!(authority.recorded);
    let calculated: Vec<_> = authority
        .calculated
        .iter()
        .map(|(id, sides)| (id.clone(), sides.clone()))
        .collect();
    assert_eq!(
        calculated,
        vec![
            (r(4), vec![CalculationSide::Left]),
            (r(5), vec![CalculationSide::Right]),
        ]
    );
}

#[test]
fn install_records_parameter_ref_access_and_calculation_members() {
    let (_dir, path) = installed();
    let conn = Connection::open(&path).unwrap();
    assert_recorded(&conn);
}

#[test]
fn the_calculation_itself_stays_a_reported_unknown_construct() {
    // ADR-0080 rule 1: the index is not an interpretation.
    let (_dir, path) = installed();
    let conn = Connection::open(&path).unwrap();
    let reported: i64 = conn
        .query_row(
            "SELECT count(*) FROM ingest_unknown WHERE kind = 'Element' AND name = 'ParameterCalculation'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(reported, 1);
}

#[test]
fn an_unknown_program_has_no_recorded_authority() {
    let (_dir, path) = installed();
    let conn = Connection::open(&path).unwrap();
    let authority = write_authority(&conn, "M-00FA_A-FFFF-FF-FFFF").unwrap();
    assert!(!authority.recorded);
    assert!(authority.calculated.is_empty());
}

/// A genuine v19 database: no column, no table, no flag.
fn rewind_to_v19(conn: &Connection) {
    conn.execute_batch(
        "DROP TABLE parameter_calculation_ref;
         ALTER TABLE parameter_ref DROP COLUMN access;
         ALTER TABLE application_program DROP COLUMN write_authority_recorded;
         PRAGMA user_version = 19;",
    )
    .unwrap();
}

#[test]
fn v19_to_v20_backfills_access_and_calculation_members_from_the_retained_blob() {
    let (_dir, path) = installed();
    rewind_to_v19(&Connection::open(&path).unwrap());
    let conn = knx_productdb::open_and_migrate(&path).unwrap();
    assert_recorded(&conn);
}

#[test]
fn v19_to_v20_leaves_a_damaged_blob_unrecorded_and_says_so() {
    let (_dir, path) = installed();
    let sha = knx_productdb::sha256_hex(PROGRAM.as_bytes());
    {
        let conn = Connection::open(&path).unwrap();
        rewind_to_v19(&conn);
        let mut damaged = PROGRAM.as_bytes().to_vec();
        let at = damaged.len() - 10;
        damaged[at] ^= 0x20;
        conn.execute(
            "UPDATE source_file SET bytes = ?1 WHERE sha256 = ?2",
            rusqlite::params![damaged, sha],
        )
        .unwrap();
    }
    let conn = knx_productdb::open_and_migrate(&path).unwrap();
    let authority = write_authority(&conn, PROGRAM_ID).unwrap();
    assert!(!authority.recorded, "fail closed: nothing was re-read");
    assert!(authority.calculated.is_empty());
    let failure: String = conn
        .query_row(
            "SELECT sample FROM ingest_unknown WHERE source_sha256 = ?1 AND kind = 'WriteAuthorityBackfillError'",
            [&sha],
            |r| r.get(0),
        )
        .unwrap();
    assert!(
        failure.contains("do not match their SHA-256 key"),
        "{failure}"
    );
}
