//! `ComObjectTable`/`ComObject` (program layer) and `ComObjectRefs`/
//! `ComObjectRef` (program-ref layer) — the two layers enrichment resolves
//! onto `ComObjectInstance` (Task 11, DATA_MODEL §3).
//!
//! Flags are stored as the source's own `"Enabled"`/`"Disabled"` strings,
//! matching RESEARCH §3's amendment that schema 11 spells these five flags
//! differently from every other boolean. The conversion to `bool` happens
//! once, in enrichment, where `knx-core`'s types are in scope.

use rusqlite::{params, Connection};

use crate::xml::Attrs;
use crate::ProductDbError;

pub const COM_OBJECT_ATTRS: &[&str] = &[
    "Id",
    "Name",
    "Text",
    "FunctionText",
    "Number",
    "ObjectSize",
    "Priority",
    "DatapointType",
    "ReadFlag",
    "WriteFlag",
    "TransmitFlag",
    "UpdateFlag",
    "CommunicationFlag",
    "ReadOnInitFlag",
    "VisibleDescription",
];

pub const COM_OBJECT_REF_ATTRS: &[&str] = &[
    "Id",
    "RefId",
    "Tag",
    "Text",
    "FunctionText",
    "ObjectSize",
    "Priority",
    "DatapointType",
    "ReadFlag",
    "WriteFlag",
    "TransmitFlag",
    "UpdateFlag",
    "CommunicationFlag",
    "ReadOnInitFlag",
    "VisibleDescription",
];

fn parse_i64(v: Option<&str>) -> Option<i64> {
    v.and_then(|v| v.parse::<i64>().ok())
}

pub fn insert_com_object(
    conn: &Connection,
    program_id: &str,
    a: &Attrs,
) -> Result<(), ProductDbError> {
    conn.execute(
        "INSERT INTO com_object
         (program_id, id, number, name, text, function_text, visible_description,
          object_size, priority, dpt_list, read_flag, write_flag, transmit_flag,
          update_flag, communication_flag, read_on_init_flag)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)",
        params![
            program_id,
            a.get("Id"),
            parse_i64(a.get("Number")),
            a.get("Name"),
            a.get("Text"),
            a.get("FunctionText"),
            a.get("VisibleDescription"),
            a.get("ObjectSize"),
            a.get("Priority"),
            a.get("DatapointType"),
            a.get("ReadFlag"),
            a.get("WriteFlag"),
            a.get("TransmitFlag"),
            a.get("UpdateFlag"),
            a.get("CommunicationFlag"),
            a.get("ReadOnInitFlag"),
        ],
    )?;
    Ok(())
}

pub fn insert_com_object_ref(
    conn: &Connection,
    program_id: &str,
    a: &Attrs,
) -> Result<(), ProductDbError> {
    conn.execute(
        "INSERT INTO com_object_ref
         (program_id, id, com_object_id, tag, text, function_text, visible_description,
          object_size, priority, dpt_list, read_flag, write_flag, transmit_flag,
          update_flag, communication_flag, read_on_init_flag)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)",
        params![
            program_id,
            a.get("Id"),
            a.get("RefId"),
            a.get("Tag"),
            a.get("Text"),
            a.get("FunctionText"),
            a.get("VisibleDescription"),
            a.get("ObjectSize"),
            a.get("Priority"),
            a.get("DatapointType"),
            a.get("ReadFlag"),
            a.get("WriteFlag"),
            a.get("TransmitFlag"),
            a.get("UpdateFlag"),
            a.get("CommunicationFlag"),
            a.get("ReadOnInitFlag"),
        ],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_and_migrate;
    use crate::parse::program::ingest_program;

    const PROGRAM: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11">
  <ManufacturerData>
    <Manufacturer RefId="M-006A">
      <ApplicationPrograms>
        <ApplicationProgram Id="A-1" Name="P" ApplicationNumber="1" ApplicationVersion="22"
                            MaskVersion="MV-0701">
          <Static>
            <ComObjectTable>
              <ComObject Id="A-1_O-0" Name="Ausgang - Licht" Text="Ausgang - Licht"
                         FunctionText="Dimmen absolut" Number="0" ObjectSize="1 Byte"
                         Priority="Low" ReadFlag="Enabled" WriteFlag="Disabled"
                         CommunicationFlag="Enabled" TransmitFlag="Enabled"
                         UpdateFlag="Disabled" ReadOnInitFlag="Disabled" VisibleDescription="" />
              <ComObject Id="A-1_O-1" Name="Schalten" Text="Schalten" Number="1"
                         ObjectSize="1 Bit" DatapointType="DPST-1-1" WriteFlag="Enabled" />
            </ComObjectTable>
            <ComObjectRefs>
              <ComObjectRef Id="A-1_O-1_R-10003" RefId="A-1_O-1" Tag="10003" />
              <ComObjectRef Id="A-1_O-0_R-10004" RefId="A-1_O-0" Text="Ausgang - Dimmen"
                            DatapointType="DPST-9-21 DPST-9-1" WriteFlag="Enabled" />
            </ComObjectRefs>
          </Static>
        </ApplicationProgram>
      </ApplicationPrograms>
    </Manufacturer>
  </ManufacturerData>
</KNX>"#;

    fn db() -> (tempfile::TempDir, Connection) {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
        (dir, conn)
    }

    #[test]
    fn com_objects_store_number_texts_size_and_flags() {
        let (_dir, conn) = db();
        ingest_program(&conn, "sha-1", "M-006A/A.xml", PROGRAM.as_bytes()).unwrap();
        let (number, text, function, size, read, write): (
            i64,
            String,
            String,
            String,
            String,
            String,
        ) = conn
            .query_row(
                "SELECT number, text, function_text, object_size, read_flag, write_flag
                 FROM com_object WHERE id = 'A-1_O-0'",
                [],
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
            .unwrap();
        assert_eq!(number, 0);
        assert_eq!(text, "Ausgang - Licht");
        assert_eq!(function, "Dimmen absolut");
        assert_eq!(size, "1 Byte");
        assert_eq!((read.as_str(), write.as_str()), ("Enabled", "Disabled"));
    }

    #[test]
    fn a_com_object_ref_without_overrides_stores_nulls_not_defaults() {
        // A ProgramRef-layer row states only what the variant overrides.
        // Copying the program's own values down would make every attribute
        // look overridden, which is exactly the distinction DATA_MODEL §3
        // exists to keep.
        let (_dir, conn) = db();
        ingest_program(&conn, "sha-1", "M-006A/A.xml", PROGRAM.as_bytes()).unwrap();
        let (com, text, dpt): (String, Option<String>, Option<String>) = conn
            .query_row(
                "SELECT com_object_id, text, dpt_list FROM com_object_ref WHERE id = 'A-1_O-1_R-10003'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(com, "A-1_O-1");
        assert_eq!(text, None);
        assert_eq!(dpt, None);
    }

    #[test]
    fn a_datapoint_type_list_is_stored_verbatim() {
        // RESEARCH §4.2: the attribute can hold several space-separated
        // alternatives. Stored as written; deciding between them is the
        // enrichment's problem, and it refuses to guess (Task 11).
        let (_dir, conn) = db();
        ingest_program(&conn, "sha-1", "M-006A/A.xml", PROGRAM.as_bytes()).unwrap();
        let dpt: String = conn
            .query_row(
                "SELECT dpt_list FROM com_object_ref WHERE id = 'A-1_O-0_R-10004'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(dpt, "DPST-9-21 DPST-9-1");
    }

    #[test]
    fn a_program_level_datapoint_type_is_stored_on_the_com_object() {
        let (_dir, conn) = db();
        ingest_program(&conn, "sha-1", "M-006A/A.xml", PROGRAM.as_bytes()).unwrap();
        let dpt: String = conn
            .query_row(
                "SELECT dpt_list FROM com_object WHERE id = 'A-1_O-1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(dpt, "DPST-1-1");
    }
}
