//! `knx_master.xml`: `Manufacturers` (id → display name), `DatapointTypes`
//! (main/sub numbers → id), `FunctionTypes`/`FunctionPoint` (the function-type
//! catalogue, each type's per-role datapoints nested under it) and
//! `SpaceUsages` (the standard list of room/space purposes), ingested
//! minimally — the file itself stays in the project's opaque store
//! (`OpaqueKind::MasterData`), so the export path is unchanged. Every other
//! `MasterData` child (`DatapointRoles`, `InterfaceObjectTypes`,
//! `InterfaceObjectProperties`, `PropertyDataTypes`, `MediumTypes`,
//! `MaskVersions`, `FunctionalBlocks`, `ProductLanguages`) stays unparsed —
//! [V], `knx_master.xml`'s own top-level section list, corpus-wide — and,
//! like everything else this function does not recognize, falls through to
//! `_ => {}` unreported rather than into `unknown` (KNOWN_LIMITATIONS.md
//! §64 tracks only the two families that actually carry a `Master`-scope
//! translation; the rest is a pre-existing, wider gap this slice does not
//! close).

use quick_xml::events::Event;
use quick_xml::Reader;
use rusqlite::{params, Connection};

use super::report_unknown_attrs;
use super::translation::{ingest_translations, TranslationScope};
use crate::report::{UnknownCollector, UnknownConstruct};
use crate::xml::{attrs, local_name};
use crate::ProductDbError;

const MANUFACTURER_ATTRS: &[&str] = &["Id", "Name"];
const DATAPOINT_TYPE_ATTRS: &[&str] = &["Id", "Number", "Name", "Text"];
const DATAPOINT_SUBTYPE_ATTRS: &[&str] = &["Id", "Number", "Name", "Text"];
/// [V], corpus-wide (`Dummy_Applikation_Secure`, `MDT_KP_AMI_AMS_03_Switch_Actuator_V31a`
/// — the two sampled packages whose `knx_master.xml` carries this section at
/// all). No published schema for `knx_master.xml` is available to this
/// project (it is ETS's own, not part of the KNX Standard corpus), so this
/// allowlist is corpus-observed, the same convention `DATAPOINT_TYPE_ATTRS`
/// already uses.
const FUNCTION_TYPE_ATTRS: &[&str] = &["Id", "Number", "Text", "Status"];
/// [V], same two packages as `FUNCTION_TYPE_ATTRS`.
const FUNCTION_POINT_ATTRS: &[&str] = &["Id", "Text", "DatapointType", "Role", "Characteristics"];
/// [V], same two packages as `FUNCTION_TYPE_ATTRS`.
const SPACE_USAGE_ATTRS: &[&str] = &["Id", "Number", "Text"];

fn parse_i64(v: Option<&str>) -> Option<i64> {
    v.and_then(|v| v.parse::<i64>().ok())
}

/// What one `knx_master.xml` pass found: the unrecognized constructs
/// `ingest_master_data`'s doc comment already reported before this slice,
/// plus (R3) how many `Master`-scope `translation` rows it actually wrote —
/// measured the same way `ingest_translations` measures its own, never
/// predicted from the XML.
///
/// `dropped_datapoint_types` is KNOWN_LIMITATIONS.md §86's counter: `id` is
/// `datapoint_type`'s whole primary key (`migration.rs`), no
/// `source_sha256` column exists on it at all, and every manufacturer's
/// `knx_master.xml` restates the entire KNX-standard DPT catalogue rather
/// than only the DPTs its own products use. So every package after the
/// first one installed collides on nearly every id it declares, `INSERT OR
/// IGNORE` drops the second copy, and — unlike `hardware`/`product`/
/// `catalog_item`/`application_program`, which at least get an `IdConflict`
/// when the collision crosses a file — nothing recorded that a collision
/// happened at all before this field existed. It counts drops, not
/// mismatches: two files declaring the identical id with different
/// `Name`/`Text` would drop just as silently and this field cannot tell
/// that case from an exact repeat, because the table never kept either
/// candidate's source to compare.
pub struct MasterIngest {
    pub unknown: Vec<UnknownConstruct>,
    pub translations: usize,
    pub dropped_datapoint_types: usize,
}

pub fn ingest_master_data(conn: &Connection, bytes: &[u8]) -> Result<MasterIngest, ProductDbError> {
    let source_path = "knx_master.xml";
    let mut reader = Reader::from_reader(bytes);
    let mut buf = Vec::new();
    let mut unknown = UnknownCollector::default();
    let mut current_main: Option<i64> = None;
    let mut current_function_type: Option<String> = None;
    let mut dropped_datapoint_types = 0usize;

    loop {
        buf.clear();
        let event = reader
            .read_event_into(&mut buf)
            .map_err(|e| ProductDbError::Xml {
                source_path: source_path.to_string(),
                cause: e.to_string(),
            })?;
        // Self-closing elements fire no matching `Event::End`, so any
        // "current" scope opened for one must be closed again before the
        // match arm below returns — nothing downstream gets a second
        // chance to notice.
        let is_self_closing = matches!(&event, Event::Empty(_));
        match event {
            Event::Eof => break,
            Event::End(e) if e.local_name().as_ref() == "DatapointType" => {
                current_main = None;
            }
            Event::End(e) if e.local_name().as_ref() == "FunctionType" => {
                current_function_type = None;
            }
            Event::Start(e) | Event::Empty(e) => {
                let name = local_name(&e);
                let a = attrs(&e, source_path)?;
                match name.as_str() {
                    "Manufacturer" => {
                        report_unknown_attrs(
                            &mut unknown,
                            "/KNX/MasterData/Manufacturers/Manufacturer",
                            &a,
                            MANUFACTURER_ATTRS,
                        );
                        conn.execute(
                            "INSERT INTO manufacturer (id, name) VALUES (?1, ?2)
                             ON CONFLICT(id) DO UPDATE SET name = excluded.name",
                            params![a.get("Id"), a.get("Name")],
                        )?;
                    }
                    "DatapointType" => {
                        report_unknown_attrs(
                            &mut unknown,
                            "/KNX/MasterData/DatapointTypes/DatapointType",
                            &a,
                            DATAPOINT_TYPE_ATTRS,
                        );
                        let main = parse_i64(a.get("Number")).unwrap_or_default();
                        current_main = Some(main);
                        let written = conn.execute(
                            "INSERT OR IGNORE INTO datapoint_type (id, main, sub, name, text)
                             VALUES (?1, ?2, NULL, ?3, ?4)",
                            params![a.get("Id"), main, a.get("Name"), a.get("Text")],
                        )?;
                        if written == 0 {
                            dropped_datapoint_types += 1;
                        }
                    }
                    "DatapointSubtype" => {
                        report_unknown_attrs(
                            &mut unknown,
                            "/KNX/MasterData/DatapointTypes/DatapointType/DatapointSubtypes/DatapointSubtype",
                            &a,
                            DATAPOINT_SUBTYPE_ATTRS,
                        );
                        if let Some(main) = current_main {
                            let written = conn.execute(
                                "INSERT OR IGNORE INTO datapoint_type (id, main, sub, name, text)
                                 VALUES (?1, ?2, ?3, ?4, ?5)",
                                params![
                                    a.get("Id"),
                                    main,
                                    parse_i64(a.get("Number")),
                                    a.get("Name"),
                                    a.get("Text"),
                                ],
                            )?;
                            if written == 0 {
                                dropped_datapoint_types += 1;
                            }
                        }
                    }
                    "FunctionType" => {
                        report_unknown_attrs(
                            &mut unknown,
                            "/KNX/MasterData/FunctionTypes/FunctionType",
                            &a,
                            FUNCTION_TYPE_ATTRS,
                        );
                        // A self-closing `FunctionType` (the corpus's own
                        // `<FunctionType Text="custom" Id="FT-0" Number="0" />`
                        // shape) fires no `Event::End` to clear this again,
                        // so it must not open a scope that outlives itself
                        // — otherwise a later sibling `FunctionPoint` would
                        // silently inherit an id that was never really its
                        // parent.
                        current_function_type = if is_self_closing {
                            None
                        } else {
                            a.get("Id").map(str::to_string)
                        };
                        conn.execute(
                            "INSERT OR IGNORE INTO function_type (id, number, text, status)
                             VALUES (?1, ?2, ?3, ?4)",
                            params![
                                a.get("Id"),
                                parse_i64(a.get("Number")),
                                a.get("Text"),
                                a.get("Status"),
                            ],
                        )?;
                    }
                    "FunctionPoint" => {
                        report_unknown_attrs(
                            &mut unknown,
                            "/KNX/MasterData/FunctionTypes/FunctionType/FunctionPoint",
                            &a,
                            FUNCTION_POINT_ATTRS,
                        );
                        // Mirrors `DatapointSubtype` below: a `FunctionPoint`
                        // met outside any `FunctionType` (not a shape the
                        // schema produces, going by every sampled package)
                        // has no parent to key its foreign key on, so it is
                        // skipped rather than stored half-addressed.
                        if let Some(function_type_id) = current_function_type.as_deref() {
                            conn.execute(
                                "INSERT OR IGNORE INTO function_point
                                     (id, function_type_id, datapoint_type, role, characteristics, text)
                                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                                params![
                                    a.get("Id"),
                                    function_type_id,
                                    a.get("DatapointType"),
                                    a.get("Role"),
                                    a.get("Characteristics"),
                                    a.get("Text"),
                                ],
                            )?;
                        }
                    }
                    "SpaceUsage" => {
                        report_unknown_attrs(
                            &mut unknown,
                            "/KNX/MasterData/SpaceUsages/SpaceUsage",
                            &a,
                            SPACE_USAGE_ATTRS,
                        );
                        conn.execute(
                            "INSERT OR IGNORE INTO space_usage (id, number, text)
                             VALUES (?1, ?2, ?3)",
                            params![a.get("Id"), parse_i64(a.get("Number")), a.get("Text")],
                        )?;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    // A second pass over the same bytes, in the same spirit as `Catalog.xml`
    // and `Hardware.xml`'s own `Languages` blocks (`ingest.rs`):
    // `knx_master.xml` carries no owning element to key its translations to,
    // so `TranslationScope::Master` uses the empty-string sentinel instead.
    let translations = ingest_translations(conn, TranslationScope::Master, source_path, bytes)?;
    Ok(MasterIngest {
        unknown: unknown.into_vec(),
        translations,
        dropped_datapoint_types,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_and_migrate;

    const MASTER: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11">
  <MasterData>
    <Manufacturers>
      <Manufacturer Id="M-0001" Name="Siemens" />
      <Manufacturer Id="M-0083" Name="MDT technologies" />
    </Manufacturers>
    <DatapointTypes>
      <DatapointType Id="DPT-1" Number="1" Name="1.xxx" Text="1-bit">
        <DatapointSubtypes>
          <DatapointSubtype Id="DPST-1-1" Number="1" Name="DPT_Switch" Text="switch" />
        </DatapointSubtypes>
      </DatapointType>
    </DatapointTypes>
  </MasterData>
</KNX>"#;

    fn db() -> (tempfile::TempDir, Connection) {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
        (dir, conn)
    }

    #[test]
    fn manufacturer_names_are_filled_in() {
        let (_dir, conn) = db();
        ingest_master_data(&conn, MASTER.as_bytes()).unwrap();
        let name: String = conn
            .query_row(
                "SELECT name FROM manufacturer WHERE id = 'M-0083'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(name, "MDT technologies");
    }

    #[test]
    fn a_manufacturer_seen_during_ingest_first_gets_its_name_later() {
        // Hardware.xml creates the row with a NULL name (Task 4); master
        // data fills it in whichever order the two arrive.
        let (_dir, conn) = db();
        conn.execute(
            "INSERT INTO manufacturer (id, name) VALUES ('M-0083', NULL)",
            [],
        )
        .unwrap();
        ingest_master_data(&conn, MASTER.as_bytes()).unwrap();
        let name: String = conn
            .query_row(
                "SELECT name FROM manufacturer WHERE id = 'M-0083'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(name, "MDT technologies");
    }

    #[test]
    fn a_later_ingested_master_file_updates_the_name_the_earlier_one_wrote() {
        // Deliberately the mirror image of `first_winner`
        // (KNOWN_LIMITATIONS.md § 88): real manufacturers get renamed across
        // ETS editions (`M-0007` is either `"Busch-Jaeger Elektro"` or
        // `"ABB AG - BUSCH-JAEGER"`, and 51 further ids are the same shape,
        // both spellings quoted as the corpus writes them, per the
        // 69-file corpus sweep behind that entry), and nothing in
        // knx_master.xml says which spelling is newer except ingest order.
        // First-writer-wins would leave a package's old spelling stuck
        // forever; this asserts the current, chosen behaviour is the other
        // way round.
        let (_dir, conn) = db();
        ingest_master_data(&conn, MASTER.as_bytes()).unwrap();
        let renamed = MASTER.replace(
            r#"Name="MDT technologies""#,
            r#"Name="MDT Technologies GmbH""#,
        );
        ingest_master_data(&conn, renamed.as_bytes()).unwrap();
        let name: String = conn
            .query_row(
                "SELECT name FROM manufacturer WHERE id = 'M-0083'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            name, "MDT Technologies GmbH",
            "the later ingest's name wins"
        );
    }

    const MASTER_WITH_LANGUAGES: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11">
  <MasterData>
    <Manufacturers>
      <Manufacturer Id="M-0001" Name="Siemens" />
    </Manufacturers>
  </MasterData>
  <Languages>
    <Language Identifier="de-DE">
      <TranslationUnit RefId="LOC-1">
        <TranslationElement RefId="LOC-1">
          <Translation AttributeName="Text" Text="Übersetzt" />
        </TranslationElement>
      </TranslationUnit>
    </Language>
  </Languages>
</KNX>"#;

    #[test]
    fn master_translations_are_ingested_with_the_empty_scope_id() {
        let (_dir, conn) = db();
        let report = ingest_master_data(&conn, MASTER_WITH_LANGUAGES.as_bytes()).unwrap();
        let (scope, scope_id, text): (String, String, String) = conn
            .query_row(
                "SELECT scope, scope_id, text FROM translation
                 WHERE ref_id = 'LOC-1' AND attribute_name = 'Text'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(scope, "Master");
        assert_eq!(scope_id, "");
        assert_eq!(text, "Übersetzt");
        assert_eq!(report.translations, 1);
    }

    #[test]
    fn a_repeated_master_translation_is_parsed_but_not_counted_twice() {
        // The second ingest sees the same `Translation` element again but
        // `INSERT OR IGNORE` writes nothing new — R3 counts writes, not
        // sightings.
        let (_dir, conn) = db();
        ingest_master_data(&conn, MASTER_WITH_LANGUAGES.as_bytes()).unwrap();
        let report = ingest_master_data(&conn, MASTER_WITH_LANGUAGES.as_bytes()).unwrap();
        assert_eq!(report.translations, 0);
    }

    #[test]
    fn datapoint_main_and_subtypes_are_stored_with_their_numbers() {
        let (_dir, conn) = db();
        ingest_master_data(&conn, MASTER.as_bytes()).unwrap();
        let (main, sub, name): (i64, Option<i64>, String) = conn
            .query_row(
                "SELECT main, sub, name FROM datapoint_type WHERE id = 'DPST-1-1'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!((main, sub, name.as_str()), (1, Some(1), "DPT_Switch"));
        let main_only: (i64, Option<i64>) = conn
            .query_row(
                "SELECT main, sub FROM datapoint_type WHERE id = 'DPT-1'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(main_only, (1, None));
    }

    const MASTER_WITH_FUNCTIONS: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11">
  <MasterData>
    <Manufacturers>
      <Manufacturer Id="M-0001" Name="Siemens" />
    </Manufacturers>
    <FunctionTypes>
      <FunctionType Id="FT-1" Number="1" Text="Switch" Status="Certified">
        <FunctionPoint Id="FP-1_DR-1" Text="Switch" DatapointType="DPST-1-1" Role="Control" Characteristics="W" />
      </FunctionType>
    </FunctionTypes>
    <SpaceUsages>
      <SpaceUsage Id="SU-1" Number="1" Text="Office" />
    </SpaceUsages>
  </MasterData>
  <Languages>
    <Language Identifier="de-DE">
      <TranslationUnit RefId="FT-1">
        <TranslationElement RefId="FT-1">
          <Translation AttributeName="Text" Text="Schalten" />
        </TranslationElement>
      </TranslationUnit>
      <TranslationUnit RefId="SU-1">
        <TranslationElement RefId="SU-1">
          <Translation AttributeName="Text" Text="Büro" />
        </TranslationElement>
      </TranslationUnit>
    </Language>
  </Languages>
</KNX>"#;

    #[test]
    fn function_types_and_their_points_are_stored() {
        let (_dir, conn) = db();
        ingest_master_data(&conn, MASTER_WITH_FUNCTIONS.as_bytes()).unwrap();
        let (number, text, status): (i64, String, String) = conn
            .query_row(
                "SELECT number, text, status FROM function_type WHERE id = 'FT-1'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(
            (number, text.as_str(), status.as_str()),
            (1, "Switch", "Certified")
        );
        let (function_type_id, datapoint_type, role): (String, String, String) = conn
            .query_row(
                "SELECT function_type_id, datapoint_type, role FROM function_point WHERE id = 'FP-1_DR-1'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(
            (
                function_type_id.as_str(),
                datapoint_type.as_str(),
                role.as_str()
            ),
            ("FT-1", "DPST-1-1", "Control")
        );
    }

    #[test]
    fn a_function_point_outside_any_function_type_is_dropped_not_misparented() {
        // A `FunctionPoint` cannot be stored without the parent id its
        // foreign key names — there is no sentinel value for "no parent"
        // that would not silently misattribute a real one. Two ways a
        // trailing `FunctionPoint` could end up misparented instead of
        // dropped: `FT-9` is opened and closed properly with its own real
        // child, exercising the `Event::End` reset; `FT-0` is self-closing
        // — the corpus's own `<FunctionType Text="custom" Id="FT-0"
        // Number="0" />` shape — exercising the leak-on-`Event::Empty` path
        // instead. Either bug would attribute the final `FunctionPoint` to
        // whichever id is still lingering; a correct parser drops it,
        // leaving exactly one row: `FT-9`'s own.
        let (_dir, conn) = db();
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11">
  <MasterData>
    <FunctionTypes>
      <FunctionType Id="FT-9" Number="9" Text="Dimming" Status="Certified">
        <FunctionPoint Id="FP-9_DR-1" Text="Dim" DatapointType="DPST-1-1" Role="Control" Characteristics="W" />
      </FunctionType>
      <FunctionType Id="FT-0" Number="0" Text="custom" Status="Certified" />
      <FunctionPoint Id="FP-1_DR-1" Text="Switch" DatapointType="DPST-1-1" Role="Control" Characteristics="W" />
    </FunctionTypes>
  </MasterData>
</KNX>"#;
        ingest_master_data(&conn, xml.as_bytes()).unwrap();
        let points: i64 = conn
            .query_row("SELECT count(*) FROM function_point", [], |r| r.get(0))
            .unwrap();
        assert_eq!(points, 1);
        let function_type_id: String = conn
            .query_row("SELECT function_type_id FROM function_point", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(function_type_id, "FT-9");
    }

    #[test]
    fn space_usages_are_stored() {
        let (_dir, conn) = db();
        ingest_master_data(&conn, MASTER_WITH_FUNCTIONS.as_bytes()).unwrap();
        let (number, text): (i64, String) = conn
            .query_row(
                "SELECT number, text FROM space_usage WHERE id = 'SU-1'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!((number, text.as_str()), (1, "Office"));
    }

    #[test]
    fn function_type_and_space_usage_translations_join_against_their_new_tables() {
        let (_dir, conn) = db();
        ingest_master_data(&conn, MASTER_WITH_FUNCTIONS.as_bytes()).unwrap();
        let function_type_text: String = conn
            .query_row(
                "SELECT t.text FROM translation t
                 JOIN function_type f ON f.id = t.ref_id
                 WHERE t.scope = 'Master' AND t.language = 'de-DE' AND t.attribute_name = 'Text'
                   AND f.id = 'FT-1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(function_type_text, "Schalten");
        let space_usage_text: String = conn
            .query_row(
                "SELECT t.text FROM translation t
                 JOIN space_usage s ON s.id = t.ref_id
                 WHERE t.scope = 'Master' AND t.language = 'de-DE' AND t.attribute_name = 'Text'
                   AND s.id = 'SU-1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(space_usage_text, "Büro");
    }
}
