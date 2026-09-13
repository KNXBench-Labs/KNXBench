//! The application program's `Static` tree: head, parameter types,
//! parameters (including `Union` members and their `Memory` layout) and
//! parameter refs.
//!
//! `Dynamic` is skipped here deliberately, not because it is unread: its
//! grammar was unresearched at RESEARCH R3, but that research is now done
//! (RESEARCH §4.3, 2026-09-11) and `crate::dynamic::parse` reads and stores
//! it losslessly as a second pass over the same bytes, in the same
//! ingest transaction (see `ingest::ingest_file_in_transaction`). This
//! parser stays `Static`-only so the two passes stay independently
//! testable; nothing is lost by keeping them apart, since the bytes also
//! survive whole in `source_file` regardless (ADR-0011).
//!
//! `ModuleDefs/ModuleDef/Static/{ComObjectTable,ComObjectRefs}` (schema
//! ≥21, ADR-0013) needs no dedicated handling here: `ComObject`/
//! `ComObjectRef` are matched by local element name only, with no path
//! context, so a `ModuleDef`'s own `ComObject`/`ComObjectRef` children hit
//! the exact same match arms as the top-level `ApplicationProgram/Static`
//! ones and are inserted under whichever `program_id` is currently open —
//! always the owning `ApplicationProgram`'s id, since `ModuleDef` never
//! reassigns it. Verified directly (Task 9, 2026-09), not assumed.

use quick_xml::events::Event;
use quick_xml::Reader;
use rusqlite::{params, Connection, OptionalExtension};

use super::comobject::{
    insert_com_object, insert_com_object_ref, COM_OBJECT_ATTRS, COM_OBJECT_REF_ATTRS,
};
use super::translation::{insert_translations, TranslationScope};
use super::{bool_flag, report_unknown_attrs};
use crate::report::{IdConflict, UnknownCollector, UnknownConstruct};
use crate::xml::{attrs, local_name, skip_subtree, Attrs};
use crate::ProductDbError;

const PROGRAM_ATTRS: &[&str] = &[
    "Id",
    "Name",
    "ApplicationNumber",
    "ApplicationVersion",
    "ProgramType",
    "MaskVersion",
    "PeiType",
    "LoadProcedureStyle",
    "DefaultLanguage",
    "Hash",
    "Linkable",
    "OriginalManufacturer",
];

pub struct ProgramIngest {
    pub program_id: String,
    pub unknown: Vec<UnknownConstruct>,
    pub conflicts: Vec<IdConflict>,
    /// `Translation` rows this pass actually wrote (R3) — counted the same
    /// way `translation::ingest_translations` counts its own, since
    /// `Program` scope never goes through that function (see its doc
    /// comment for why).
    pub translations: usize,
}

/// The currently open `<Union>`: its sequence number, its `@SizeInBit`, and
/// a `Memory` seen before any member `Parameter` — which then applies to
/// every member (spec §1 / Task 5 rule).
struct UnionState {
    id: i64,
    size_in_bit: Option<i64>,
    memory: Option<(Option<String>, Option<i64>, Option<i64>)>,
}

/// Where in the `Languages` tree the reader currently is: the open
/// `Language/@Identifier` and, nested inside it, the open
/// `TranslationElement/@RefId` — the two keys every `Translation` row needs
/// besides its own `AttributeName`.
#[derive(Default)]
struct TranslationState {
    language: Option<String>,
    ref_id: Option<String>,
}

fn parse_i64(v: Option<&str>) -> Option<i64> {
    v.and_then(|v| v.parse::<i64>().ok())
}

pub fn ingest_program(
    conn: &Connection,
    source_sha256: &str,
    source_path: &str,
    bytes: &[u8],
) -> Result<ProgramIngest, ProductDbError> {
    let mut reader = Reader::from_reader(bytes);
    let mut buf = Vec::new();
    let mut unknown = UnknownCollector::default();
    let mut conflicts = Vec::new();
    let mut manufacturer_id = String::new();
    let mut program_id = String::new();
    let mut already_present = false;

    let mut union_seq: i64 = 0;
    let mut current_union: Option<UnionState> = None;
    // (id, name) of the `ParameterType` currently open; cleared on its
    // `</ParameterType>`.
    let mut current_parameter_type: Option<(String, Option<String>)> = None;
    // True for exactly the one element expected right after
    // `<ParameterType>` opens — the child that decides `kind`.
    let mut expecting_type_child = false;
    let mut current_parameter: Option<String> = None;
    let mut translation_state = TranslationState::default();
    let mut translations_written = 0usize;

    loop {
        buf.clear();
        let event = reader
            .read_event_into(&mut buf)
            .map_err(|e| ProductDbError::Xml {
                source_path: source_path.to_string(),
                cause: e.to_string(),
            })?;
        match event {
            Event::Eof => break,
            Event::Start(e) if local_name(&e) == "Dynamic" => {
                skip_subtree(&mut reader, e.name().as_ref(), source_path)?;
            }
            Event::End(e) => match e.local_name().as_ref() {
                "Union" => current_union = None,
                "ParameterType" => current_parameter_type = None,
                "Parameter" => current_parameter = None,
                "Language" => translation_state.language = None,
                "TranslationElement" => translation_state.ref_id = None,
                _ => {}
            },
            Event::Empty(e) => {
                let name = local_name(&e);
                let a = attrs(&e, source_path)?;
                handle_start_or_empty(
                    conn,
                    &name,
                    &a,
                    false,
                    source_sha256,
                    &mut unknown,
                    &mut conflicts,
                    &mut manufacturer_id,
                    &mut program_id,
                    &mut already_present,
                    &mut union_seq,
                    &mut current_union,
                    &mut current_parameter_type,
                    &mut expecting_type_child,
                    &mut current_parameter,
                    &mut translation_state,
                    &mut translations_written,
                )?;
            }
            Event::Start(e) => {
                let name = local_name(&e);
                let a = attrs(&e, source_path)?;
                handle_start_or_empty(
                    conn,
                    &name,
                    &a,
                    true,
                    source_sha256,
                    &mut unknown,
                    &mut conflicts,
                    &mut manufacturer_id,
                    &mut program_id,
                    &mut already_present,
                    &mut union_seq,
                    &mut current_union,
                    &mut current_parameter_type,
                    &mut expecting_type_child,
                    &mut current_parameter,
                    &mut translation_state,
                    &mut translations_written,
                )?;
            }
            _ => {}
        }
    }

    Ok(ProgramIngest {
        program_id,
        unknown: unknown.into_vec(),
        conflicts,
        translations: translations_written,
    })
}

#[allow(clippy::too_many_arguments)]
fn handle_start_or_empty(
    conn: &Connection,
    name: &str,
    a: &Attrs,
    is_start: bool,
    source_sha256: &str,
    unknown: &mut UnknownCollector,
    conflicts: &mut Vec<IdConflict>,
    manufacturer_id: &mut String,
    program_id: &mut String,
    already_present: &mut bool,
    union_seq: &mut i64,
    current_union: &mut Option<UnionState>,
    current_parameter_type: &mut Option<(String, Option<String>)>,
    expecting_type_child: &mut bool,
    current_parameter: &mut Option<String>,
    translation_state: &mut TranslationState,
    translations_written: &mut usize,
) -> Result<(), ProductDbError> {
    if *expecting_type_child {
        *expecting_type_child = false;
        if !*already_present {
            if let Some((pt_id, pt_name)) = current_parameter_type.clone() {
                insert_parameter_type(conn, program_id, &pt_id, &pt_name, name, a, unknown)?;
            }
        }
        return Ok(());
    }

    match name {
        "Manufacturer" => {
            *manufacturer_id = a.get("RefId").unwrap_or_default().to_string();
            conn.execute(
                "INSERT OR IGNORE INTO manufacturer (id, name) VALUES (?1, NULL)",
                [manufacturer_id.as_str()],
            )?;
        }
        "ApplicationProgram" => {
            *program_id = a.get("Id").unwrap_or_default().to_string();
            let existing: Option<String> = conn
                .query_row(
                    "SELECT source_sha256 FROM application_program WHERE id = ?1",
                    [program_id.as_str()],
                    |r| r.get(0),
                )
                .optional()?;
            if let Some(kept) = existing {
                *already_present = true;
                if kept != source_sha256 {
                    conflicts.push(IdConflict {
                        table: "application_program".into(),
                        id: program_id.clone(),
                        kept_sha256: kept,
                        other_sha256: source_sha256.to_string(),
                    });
                }
            } else {
                report_unknown_attrs(
                    unknown,
                    "/KNX/ManufacturerData/Manufacturer/ApplicationPrograms/ApplicationProgram",
                    a,
                    PROGRAM_ATTRS,
                );
                conn.execute(
                    "INSERT INTO application_program
                     (id, manufacturer_id, name, application_number,
                      application_version, program_type, mask_version, pei_type,
                      load_procedure_style, default_language, hash, linkable,
                      original_manufacturer, source_sha256)
                     VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
                    params![
                        program_id.as_str(),
                        manufacturer_id.as_str(),
                        a.get("Name"),
                        a.get("ApplicationNumber"),
                        a.get("ApplicationVersion"),
                        a.get("ProgramType"),
                        a.get("MaskVersion"),
                        a.get("PeiType"),
                        a.get("LoadProcedureStyle"),
                        a.get("DefaultLanguage"),
                        a.get("Hash"),
                        bool_flag(
                            unknown,
                            "/KNX/ManufacturerData/Manufacturer/ApplicationPrograms/ApplicationProgram",
                            a,
                            "Linkable",
                        ),
                        a.get("OriginalManufacturer"),
                        source_sha256,
                    ],
                )?;
            }
        }
        "ParameterType" => {
            *current_parameter_type = Some((
                a.get("Id").unwrap_or_default().to_string(),
                a.get("Name").map(str::to_string),
            ));
            *expecting_type_child = true;
        }
        "Enumeration" => {
            if !*already_present {
                if let Some((pt_id, _)) = current_parameter_type.as_ref() {
                    conn.execute(
                        "INSERT INTO parameter_type_enum
                         (program_id, parameter_type_id, id, value, text, display_order)
                         VALUES (?1,?2,?3,?4,?5,?6)",
                        params![
                            program_id.as_str(),
                            pt_id,
                            a.get("Id"),
                            a.get("Value"),
                            a.get("Text"),
                            parse_i64(a.get("DisplayOrder")),
                        ],
                    )?;
                }
            }
        }
        "Union" => {
            *union_seq += 1;
            *current_union = Some(UnionState {
                id: *union_seq,
                size_in_bit: parse_i64(a.get("SizeInBit")),
                memory: None,
            });
            *current_parameter = None;
        }
        "Parameter" => {
            let id = a.get("Id").unwrap_or_default().to_string();
            if !*already_present {
                let (union_id, union_size, seg, off, bit) = match current_union.as_ref() {
                    Some(u) => {
                        let (seg, off, bit) = u.memory.clone().unwrap_or((None, None, None));
                        (Some(u.id), u.size_in_bit, seg, off, bit)
                    }
                    None => (None, None, None, None, None),
                };
                conn.execute(
                    "INSERT INTO parameter
                     (program_id, id, name, text, parameter_type_id, access, value, suffix,
                      code_segment, offset, bit_offset, union_id, union_size_in_bit)
                     VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
                    params![
                        program_id.as_str(),
                        id,
                        a.get("Name"),
                        a.get("Text"),
                        a.get("ParameterType"),
                        a.get("Access"),
                        a.get("Value"),
                        a.get("Suffix"),
                        seg,
                        off,
                        bit,
                        union_id,
                        union_size,
                    ],
                )?;
            }
            // A self-closing `<Parameter/>` has no children, so no `Memory`
            // can follow as its child — only a `Start` leaves it open to
            // receive one.
            *current_parameter = if is_start { Some(id) } else { None };
        }
        "Memory" => {
            let seg = a.get("CodeSegment").map(str::to_string);
            let off = parse_i64(a.get("Offset"));
            let bit = parse_i64(a.get("BitOffset"));
            if !*already_present {
                if let Some(pid) = current_parameter.as_ref() {
                    conn.execute(
                        "UPDATE parameter SET code_segment = ?1, offset = ?2, bit_offset = ?3
                         WHERE program_id = ?4 AND id = ?5",
                        params![seg, off, bit, program_id.as_str(), pid],
                    )?;
                } else if let Some(u) = current_union.as_mut() {
                    u.memory = Some((seg, off, bit));
                }
            }
        }
        "Language" => {
            translation_state.language = a.get("Identifier").map(str::to_string);
        }
        "TranslationElement" => {
            translation_state.ref_id = a.get("RefId").map(str::to_string);
        }
        "Translation" => {
            if let (Some(language), Some(ref_id)) =
                (&translation_state.language, &translation_state.ref_id)
            {
                if insert_translations(
                    conn,
                    TranslationScope::Program,
                    program_id,
                    language,
                    ref_id,
                    a.get("AttributeName").unwrap_or_default(),
                    a.get("Text").unwrap_or_default(),
                )? {
                    *translations_written += 1;
                }
            }
        }
        "ComObject" if !*already_present => {
            report_unknown_attrs(
                unknown,
                "/KNX/ManufacturerData/Manufacturer/ApplicationPrograms/ApplicationProgram/Static/ComObjectTable/ComObject",
                a,
                COM_OBJECT_ATTRS,
            );
            insert_com_object(conn, program_id, a)?;
        }
        "ComObjectRef" if !*already_present => {
            report_unknown_attrs(
                unknown,
                "/KNX/ManufacturerData/Manufacturer/ApplicationPrograms/ApplicationProgram/Static/ComObjectRefs/ComObjectRef",
                a,
                COM_OBJECT_REF_ATTRS,
            );
            insert_com_object_ref(conn, program_id, a)?;
        }
        "ParameterRef" if !*already_present => {
            conn.execute(
                "INSERT INTO parameter_ref
                 (program_id, id, parameter_id, display_order, tag, text, value)
                 VALUES (?1,?2,?3,?4,?5,?6,?7)",
                params![
                    program_id.as_str(),
                    a.get("Id"),
                    a.get("RefId"),
                    parse_i64(a.get("DisplayOrder")),
                    a.get("Tag"),
                    a.get("Text"),
                    a.get("Value"),
                ],
            )?;
        }
        _ => {}
    }
    Ok(())
}

/// `(kind, size_in_bit, base, min_inclusive, max_inclusive, number_type)`,
/// as decided by the `ParameterType`'s type-deciding child element.
type TypeFields<'a> = (
    &'a str,
    Option<i64>,
    Option<&'a str>,
    Option<&'a str>,
    Option<&'a str>,
    Option<&'a str>,
);

#[allow(clippy::too_many_arguments)]
fn insert_parameter_type(
    conn: &Connection,
    program_id: &str,
    pt_id: &str,
    pt_name: &Option<String>,
    child_name: &str,
    a: &Attrs,
    unknown: &mut UnknownCollector,
) -> Result<(), ProductDbError> {
    let (kind, size_in_bit, base, min_inclusive, max_inclusive, number_type): TypeFields =
        match child_name {
            "TypeRestriction" => (
                "Restriction",
                parse_i64(a.get("SizeInBit")),
                a.get("Base"),
                None,
                None,
                None,
            ),
            "TypeNumber" => (
                "Number",
                parse_i64(a.get("SizeInBit")),
                None,
                a.get("minInclusive"),
                a.get("maxInclusive"),
                a.get("Type"),
            ),
            "TypeText" => ("Text", None, None, None, None, None),
            "TypeNone" => ("None", None, None, None, None, None),
            "TypeFloat" => ("Float", None, None, None, None, None),
            "TypeIPAddress" => ("IPAddress", None, None, None, None, None),
            "TypePicture" => ("Picture", None, None, None, None, None),
            "TypeRawData" => ("Raw", None, None, None, None, None),
            other => {
                unknown.element(
                "/KNX/ManufacturerData/Manufacturer/ApplicationPrograms/ApplicationProgram/Static/ParameterTypes/ParameterType",
                other,
            );
                ("Other", None, None, None, None, None)
            }
        };
    conn.execute(
        "INSERT INTO parameter_type
         (program_id, id, name, kind, size_in_bit, base, min_inclusive, max_inclusive, number_type)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
        params![
            program_id,
            pt_id,
            pt_name,
            kind,
            size_in_bit,
            base,
            min_inclusive,
            max_inclusive,
            number_type,
        ],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_and_migrate;

    const PROGRAM: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11">
  <ManufacturerData>
    <Manufacturer RefId="M-006A">
      <ApplicationPrograms>
        <ApplicationProgram Id="M-006A_A-0001-22-26C0-O0079" Name="Presence" ApplicationNumber="1"
                            ApplicationVersion="22" ProgramType="ApplicationProgram"
                            MaskVersion="MV-0701" PeiType="0" LoadProcedureStyle="ProductProcedure"
                            DefaultLanguage="de-DE" Hash="Dyd1CfXJEmqKsHKWZeApTA==">
          <Static>
            <ParameterTypes>
              <ParameterType Id="PT-Base" Name="baseOfSignal">
                <TypeRestriction Base="Value" SizeInBit="8">
                  <Enumeration Id="PT-Base_EN-0" Text="0.0 V" Value="0" />
                  <Enumeration Id="PT-Base_EN-5" Text="0.5 V" Value="5" DisplayOrder="1" />
                </TypeRestriction>
              </ParameterType>
              <ParameterType Id="PT-Num" Name="delay">
                <TypeNumber maxInclusive="255" minInclusive="0" SizeInBit="8" Type="unsignedInt" />
              </ParameterType>
            </ParameterTypes>
            <Parameters>
              <Parameter Id="P-1" Name="Delay" Text="Delay" ParameterType="PT-Num"
                         Access="ReadWrite" Value="7">
                <Memory CodeSegment="AS-40F4" Offset="116" BitOffset="0" />
              </Parameter>
              <Union SizeInBit="8">
                <Memory CodeSegment="AS-40F4" Offset="160" BitOffset="0" />
                <Parameter Id="P-2" Name="Mode" Text="Mode" ParameterType="PT-Base"
                           Access="ReadWrite" Value="0" />
              </Union>
            </Parameters>
            <ParameterRefs>
              <ParameterRef Id="P-1_R-1" RefId="P-1" DisplayOrder="1000" Tag="1" />
            </ParameterRefs>
          </Static>
          <Dynamic>
            <Channel Id="CH-1">
              <choose ParamRefId="P-1_R-1">
                <when test="1"><ParameterRefRef RefId="P-1_R-1" /></when>
              </choose>
            </Channel>
          </Dynamic>
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
    fn the_program_head_is_stored() {
        let (_dir, conn) = db();
        let out = ingest_program(&conn, "sha-1", "M-006A/A.xml", PROGRAM.as_bytes()).unwrap();
        assert_eq!(out.program_id, "M-006A_A-0001-22-26C0-O0079");
        let (number, version, mask): (String, String, String) = conn
            .query_row(
                "SELECT application_number, application_version, mask_version
                 FROM application_program WHERE id = ?1",
                ["M-006A_A-0001-22-26C0-O0079"],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(
            (number.as_str(), version.as_str(), mask.as_str()),
            ("1", "22", "MV-0701")
        );
    }

    /// The defect this task fixes, at the level it was measured: schema
    /// 20/21 write `Linkable="false"`, and `bool_flag` used to fall through
    /// its `_ => None` arm on that spelling, storing `NULL` even though the
    /// attribute was present. It must land as `0`.
    #[test]
    fn linkable_false_is_stored_as_zero_not_null() {
        let (_dir, conn) = db();
        let xml = PROGRAM.replace(
            "Hash=\"Dyd1CfXJEmqKsHKWZeApTA==\"",
            "Hash=\"Dyd1CfXJEmqKsHKWZeApTA==\" Linkable=\"false\"",
        );
        ingest_program(&conn, "sha-1", "M-006A/A.xml", xml.as_bytes()).unwrap();
        let linkable: Option<i64> = conn
            .query_row(
                "SELECT linkable FROM application_program WHERE id = ?1",
                ["M-006A_A-0001-22-26C0-O0079"],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(linkable, Some(0));
    }

    #[test]
    fn a_restriction_type_stores_its_enumeration_values() {
        let (_dir, conn) = db();
        ingest_program(&conn, "sha-1", "M-006A/A.xml", PROGRAM.as_bytes()).unwrap();
        let kind: String = conn
            .query_row(
                "SELECT kind FROM parameter_type WHERE id = 'PT-Base'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(kind, "Restriction");
        let values: Vec<(String, String)> = conn
            .prepare("SELECT value, text FROM parameter_type_enum WHERE parameter_type_id = 'PT-Base' ORDER BY value")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert_eq!(
            values,
            vec![
                ("0".to_string(), "0.0 V".to_string()),
                ("5".to_string(), "0.5 V".to_string())
            ]
        );
    }

    #[test]
    fn a_number_type_stores_its_bounds_and_size() {
        let (_dir, conn) = db();
        ingest_program(&conn, "sha-1", "M-006A/A.xml", PROGRAM.as_bytes()).unwrap();
        let (kind, min, max, size): (String, String, String, i64) = conn
            .query_row(
                "SELECT kind, min_inclusive, max_inclusive, size_in_bit
                 FROM parameter_type WHERE id = 'PT-Num'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .unwrap();
        assert_eq!(
            (kind.as_str(), min.as_str(), max.as_str(), size),
            ("Number", "0", "255", 8)
        );
    }

    #[test]
    fn a_parameter_stores_its_memory_layout() {
        let (_dir, conn) = db();
        ingest_program(&conn, "sha-1", "M-006A/A.xml", PROGRAM.as_bytes()).unwrap();
        let (segment, offset, bit): (String, i64, i64) = conn
            .query_row(
                "SELECT code_segment, offset, bit_offset FROM parameter WHERE id = 'P-1'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!((segment.as_str(), offset, bit), ("AS-40F4", 116, 0));
    }

    #[test]
    fn a_union_member_carries_the_unions_size_and_memory() {
        let (_dir, conn) = db();
        ingest_program(&conn, "sha-1", "M-006A/A.xml", PROGRAM.as_bytes()).unwrap();
        let (union_size, segment, offset): (i64, String, i64) = conn
            .query_row(
                "SELECT union_size_in_bit, code_segment, offset FROM parameter WHERE id = 'P-2'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!((union_size, segment.as_str(), offset), (8, "AS-40F4", 160));
    }

    #[test]
    fn a_parameter_ref_points_back_at_its_parameter() {
        let (_dir, conn) = db();
        ingest_program(&conn, "sha-1", "M-006A/A.xml", PROGRAM.as_bytes()).unwrap();
        let (param, order): (String, i64) = conn
            .query_row(
                "SELECT parameter_id, display_order FROM parameter_ref WHERE id = 'P-1_R-1'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!((param.as_str(), order), ("P-1", 1000));
    }

    #[test]
    fn the_dynamic_subtree_is_skipped_and_stores_nothing() {
        let (_dir, conn) = db();
        ingest_program(&conn, "sha-1", "M-006A/A.xml", PROGRAM.as_bytes()).unwrap();
        // Nothing from <Dynamic> reaches any table: no channel, no choose,
        // no when. It survives as the source_file blob instead (ADR-0011).
        let params: i64 = conn
            .query_row("SELECT count(*) FROM parameter", [], |r| r.get(0))
            .unwrap();
        assert_eq!(params, 2);
        let refs: i64 = conn
            .query_row("SELECT count(*) FROM parameter_ref", [], |r| r.get(0))
            .unwrap();
        assert_eq!(refs, 1);
    }

    #[test]
    fn the_same_id_from_a_different_hash_is_a_recorded_conflict_not_an_overwrite() {
        let (_dir, conn) = db();
        ingest_program(&conn, "sha-1", "M-006A/A.xml", PROGRAM.as_bytes()).unwrap();
        let changed = PROGRAM.replace("Name=\"Presence\"", "Name=\"Presence v2\"");
        let out = ingest_program(&conn, "sha-2", "M-006A/A.xml", changed.as_bytes()).unwrap();
        let name: String = conn
            .query_row(
                "SELECT name FROM application_program WHERE id = ?1",
                ["M-006A_A-0001-22-26C0-O0079"],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(name, "Presence", "the first ingest's rows win");
        assert!(out
            .conflicts
            .iter()
            .any(|c| c.id == "M-006A_A-0001-22-26C0-O0079" && c.other_sha256 == "sha-2"));
    }

    const MODULE_PROGRAM: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/21"><ManufacturerData><Manufacturer RefId="M-00FA">
<ApplicationPrograms><ApplicationProgram Id="M-00FA_A-2504-10-C071" Name="P" ApplicationVersion="10" MaskVersion="MV-0701">
<Static><ComObjectTable/><ComObjectRefs/></Static>
<ModuleDefs><ModuleDef Id="M-00FA_A-2504-10-C071_MD-2" Name="module">
<Static>
<ComObjectTable>
  <ComObject Id="M-00FA_A-2504-10-C071_MD-2_O-2-0" Number="0" Text="OnOff" ObjectSize="1 Bit" DatapointType="DPST-1-1" WriteFlag="Enabled" />
</ComObjectTable>
<ComObjectRefs>
  <ComObjectRef Id="M-00FA_A-2504-10-C071_MD-2_O-2-0_R-1" RefId="M-00FA_A-2504-10-C071_MD-2_O-2-0" />
</ComObjectRefs>
</Static>
</ModuleDef></ModuleDefs>
</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

    #[test]
    fn a_module_defs_com_object_is_ingested_under_the_owning_program_id() {
        let (_dir, conn) = db();
        ingest_program(&conn, "sha-mod", "M-00FA/A.xml", MODULE_PROGRAM.as_bytes()).unwrap();
        let view = crate::query::com_object_view(
            &conn,
            "M-00FA_A-2504-10-C071",
            "M-00FA_A-2504-10-C071_MD-2_O-2-0_R-1",
            None,
        )
        .unwrap();
        assert!(view.is_some());
        assert_eq!(view.unwrap().text.as_deref(), Some("OnOff"));
    }
}
