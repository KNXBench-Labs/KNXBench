//! `Languages`/`Language`/`TranslationUnit`/`TranslationElement`/
//! `Translation`: per-language overrides of one attribute on one element,
//! keyed by `(scope, scope_id, language, ref_id, attribute_name)`. `scope`
//! names which table `scope_id` refers into, so translations that do not
//! belong to an application program (catalog, hardware, master data) can
//! share this same table.

use quick_xml::events::Event;
use quick_xml::Reader;
use rusqlite::Connection;

use crate::xml::{attrs, local_name};
use crate::ProductDbError;

/// Which table `insert_translations`' `scope_id` refers into. `Master`
/// covers translations that are not attached to any particular program,
/// catalog item or piece of hardware; its rows use `scope_id = ""` (see
/// `migrate_v3_to_v4`'s doc comment in `migration.rs` for why that sentinel
/// is required rather than `NULL`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranslationScope {
    Program,
    Catalog,
    Hardware,
    Master,
}

impl TranslationScope {
    pub fn as_str(self) -> &'static str {
        match self {
            TranslationScope::Program => "Program",
            TranslationScope::Catalog => "Catalog",
            TranslationScope::Hardware => "Hardware",
            TranslationScope::Master => "Master",
        }
    }
}

/// Returns `true` when the row was actually written. `INSERT OR IGNORE`
/// silently drops a row whose key already exists, so the caller cannot tell
/// a fresh row from a repeat without asking SQLite how many rows it
/// changed — which is exactly what this reports (R3: the import report
/// counts what was captured, not what was merely parsed).
pub fn insert_translations(
    conn: &Connection,
    scope: TranslationScope,
    scope_id: &str,
    language: &str,
    ref_id: &str,
    attribute_name: &str,
    text: &str,
) -> Result<bool, ProductDbError> {
    let changed = conn.execute(
        "INSERT OR IGNORE INTO translation (scope, scope_id, language, ref_id, attribute_name, text)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![scope.as_str(), scope_id, language, ref_id, attribute_name, text],
    )?;
    Ok(changed > 0)
}

/// A standalone pass over `Languages`/`Language`/`TranslationUnit`/
/// `TranslationElement`/`Translation`, run as a second pass over the same
/// bytes an entity parser already consumed — mirroring how
/// `dynamic::parse::parse_dynamic_trees` is a second pass over an
/// `ApplicationProgram`'s bytes (`ingest.rs`). `program.rs` keeps its own
/// inline handling and is never routed through this function (see its own
/// doc comment for why).
///
/// The scope id is `Manufacturer/@RefId` for `Catalog`/`Hardware`,
/// `ApplicationProgram/@Id` for `Program`, and `""` for `Master` (which has
/// no owning element at all — see `TranslationScope::Master`'s doc comment).
/// Returns the number of `Translation` elements actually written — rows
/// where `INSERT OR IGNORE` took effect, not the number merely walked past
/// (R3: a repeat key contributes nothing to this count even though the
/// parser still visits it).
pub fn ingest_translations(
    conn: &Connection,
    scope: TranslationScope,
    source_path: &str,
    bytes: &[u8],
) -> Result<usize, ProductDbError> {
    let mut reader = Reader::from_reader(bytes);
    let mut buf = Vec::new();
    let mut scope_id = String::new();
    let mut language: Option<String> = None;
    let mut ref_id: Option<String> = None;
    let mut count = 0usize;

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
            Event::End(e) => match e.local_name().as_ref() {
                "Language" => language = None,
                "TranslationElement" => ref_id = None,
                _ => {}
            },
            Event::Start(e) | Event::Empty(e) => {
                let name = local_name(&e);
                let a = attrs(&e, source_path)?;
                match name.as_str() {
                    "Manufacturer"
                        if matches!(
                            scope,
                            TranslationScope::Catalog | TranslationScope::Hardware
                        ) =>
                    {
                        scope_id = a.get("RefId").unwrap_or_default().to_string();
                    }
                    "ApplicationProgram" if scope == TranslationScope::Program => {
                        scope_id = a.get("Id").unwrap_or_default().to_string();
                    }
                    "Language" => {
                        language = a.get("Identifier").map(str::to_string);
                    }
                    "TranslationElement" => {
                        ref_id = a.get("RefId").map(str::to_string);
                    }
                    "Translation"
                        if insert_translations(
                            conn,
                            scope,
                            &scope_id,
                            language.as_deref().unwrap_or_default(),
                            ref_id.as_deref().unwrap_or_default(),
                            a.get("AttributeName").unwrap_or_default(),
                            a.get("Text").unwrap_or_default(),
                        )? =>
                    {
                        count += 1;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    Ok(count)
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
        <ApplicationProgram Id="A-1" Name="P" ApplicationVersion="22" MaskVersion="MV-0701">
          <Static>
            <ComObjectTable>
              <ComObject Id="A-1_O-0" Number="0" Text="Ausgang" ObjectSize="1 Bit" />
            </ComObjectTable>
          </Static>
          <Languages>
            <Language Identifier="en-US">
              <TranslationUnit RefId="A-1">
                <TranslationElement RefId="A-1_O-0">
                  <Translation AttributeName="Text" Text="Output" />
                  <Translation AttributeName="FunctionText" Text="Switch" />
                </TranslationElement>
              </TranslationUnit>
            </Language>
            <Language Identifier="de-DE">
              <TranslationUnit RefId="A-1">
                <TranslationElement RefId="A-1_O-0">
                  <Translation AttributeName="Text" Text="Ausgang" />
                </TranslationElement>
              </TranslationUnit>
            </Language>
          </Languages>
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
    fn a_translation_is_keyed_by_language_ref_and_attribute() {
        let (_dir, conn) = db();
        ingest_program(&conn, "sha-1", "M-006A/A.xml", PROGRAM.as_bytes()).unwrap();
        let text: String = conn
            .query_row(
                "SELECT text FROM translation
                 WHERE scope = 'Program' AND scope_id = 'A-1' AND language = 'en-US'
                   AND ref_id = 'A-1_O-0' AND attribute_name = 'Text'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(text, "Output");
    }

    const CATALOG: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11">
  <ManufacturerData>
    <Manufacturer RefId="M-0083">
      <Catalog>
        <CatalogSection Id="M-0083_CG-1" Name="Sensors" Number="1">
          <CatalogItem Id="M-0083_CI-1" Name="Sensor" Number="1" />
        </CatalogSection>
      </Catalog>
      <Languages>
        <Language Identifier="en-US">
          <TranslationUnit RefId="M-0083_CI-1">
            <TranslationElement RefId="M-0083_CI-1">
              <Translation AttributeName="Name" Text="Sensor" />
            </TranslationElement>
          </TranslationUnit>
        </Language>
        <Language Identifier="de-DE">
          <TranslationUnit RefId="M-0083_CI-1">
            <TranslationElement RefId="M-0083_CI-1">
              <Translation AttributeName="Name" Text="Sensor DE" />
            </TranslationElement>
          </TranslationUnit>
        </Language>
      </Languages>
    </Manufacturer>
  </ManufacturerData>
</KNX>"#;

    const HARDWARE: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11">
  <ManufacturerData>
    <Manufacturer RefId="M-0083">
      <Hardware>
        <Hardware Id="M-0083_H-1" Name="Sensor" SerialNumber="S1" VersionNumber="1">
          <Products>
            <Product Id="M-0083_H-1_P-1" Text="Sensor" OrderNumber="N1" />
          </Products>
        </Hardware>
      </Hardware>
      <Languages>
        <Language Identifier="en-US">
          <TranslationUnit RefId="M-0083_H-1_P-1">
            <TranslationElement RefId="M-0083_H-1_P-1">
              <Translation AttributeName="Text" Text="Sensor" />
            </TranslationElement>
          </TranslationUnit>
        </Language>
        <Language Identifier="de-DE">
          <TranslationUnit RefId="M-0083_H-1_P-1">
            <TranslationElement RefId="M-0083_H-1_P-1">
              <Translation AttributeName="Text" Text="Sensor DE" />
            </TranslationElement>
          </TranslationUnit>
        </Language>
      </Languages>
    </Manufacturer>
  </ManufacturerData>
</KNX>"#;

    #[test]
    fn a_catalogs_languages_block_lands_scoped_to_its_manufacturer() {
        let (_dir, conn) = db();
        crate::ingest_file(&conn, "M-0083/Catalog.xml", CATALOG.as_bytes()).unwrap();
        let rows: Vec<(String, String)> = conn
            .prepare(
                "SELECT language, text FROM translation
                 WHERE scope = 'Catalog' AND scope_id = 'M-0083'
                   AND ref_id = 'M-0083_CI-1' AND attribute_name = 'Name'
                 ORDER BY language",
            )
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert_eq!(
            rows,
            vec![
                ("de-DE".to_string(), "Sensor DE".to_string()),
                ("en-US".to_string(), "Sensor".to_string()),
            ]
        );
    }

    #[test]
    fn a_hardware_files_languages_block_lands_scoped_to_its_manufacturer() {
        let (_dir, conn) = db();
        crate::ingest_file(&conn, "M-0083/Hardware.xml", HARDWARE.as_bytes()).unwrap();
        let rows: Vec<(String, String)> = conn
            .prepare(
                "SELECT language, text FROM translation
                 WHERE scope = 'Hardware' AND scope_id = 'M-0083'
                   AND ref_id = 'M-0083_H-1_P-1' AND attribute_name = 'Text'
                 ORDER BY language",
            )
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert_eq!(
            rows,
            vec![
                ("de-DE".to_string(), "Sensor DE".to_string()),
                ("en-US".to_string(), "Sensor".to_string()),
            ]
        );
    }

    #[test]
    fn a_catalog_translation_does_not_collide_with_a_program_translation() {
        let (_dir, conn) = db();
        // Both files translate the same (scope_id, language, ref_id,
        // attribute_name) triple — the catalog's `Manufacturer/@RefId` is
        // deliberately set to the program's own id, "A-1", so only `scope`
        // tells the two rows apart. If `scope` were not part of the primary
        // key, the second `INSERT OR IGNORE` would silently drop this row.
        crate::ingest_file(&conn, "M-006A/A.xml", PROGRAM.as_bytes()).unwrap();
        let catalog = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11">
  <ManufacturerData>
    <Manufacturer RefId="A-1">
      <Catalog>
        <CatalogSection Id="A-1_CG-1" Name="Sensors" Number="1">
          <CatalogItem Id="A-1_CI-1" Name="Sensor" Number="1" />
        </CatalogSection>
      </Catalog>
      <Languages>
        <Language Identifier="en-US">
          <TranslationUnit RefId="A-1_O-0">
            <TranslationElement RefId="A-1_O-0">
              <Translation AttributeName="Text" Text="Catalog Text" />
            </TranslationElement>
          </TranslationUnit>
        </Language>
      </Languages>
    </Manufacturer>
  </ManufacturerData>
</KNX>"#;
        crate::ingest_file(&conn, "A-1/Catalog.xml", catalog.as_bytes()).unwrap();

        let rows: Vec<(String, String)> = conn
            .prepare(
                "SELECT scope, text FROM translation
                 WHERE scope_id = 'A-1' AND language = 'en-US'
                   AND ref_id = 'A-1_O-0' AND attribute_name = 'Text'
                 ORDER BY scope",
            )
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert_eq!(
            rows,
            vec![
                ("Catalog".to_string(), "Catalog Text".to_string()),
                ("Program".to_string(), "Output".to_string()),
            ]
        );
    }

    #[test]
    fn the_same_element_translates_into_several_languages() {
        let (_dir, conn) = db();
        ingest_program(&conn, "sha-1", "M-006A/A.xml", PROGRAM.as_bytes()).unwrap();
        let rows: i64 = conn
            .query_row(
                "SELECT count(*) FROM translation WHERE ref_id = 'A-1_O-0'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(rows, 3);
    }

    // A `Translation` sitting directly under `TranslationUnit`, with no
    // `TranslationElement` wrapping it, never fires a fresh `TranslationElement`
    // `Start`/`Empty` event — so the only thing that can stop it inheriting
    // the previous sibling's `ref_id` is the `</TranslationElement>` reset.
    // Not a shape the real schema produces, but the right shape to pin the
    // reset itself rather than the `RefId` attribute lookup, which already
    // overwrites `ref_id` unconditionally on every `TranslationElement` open
    // tag regardless of whether the reset ever ran.
    const A_LOOSE_TRANSLATION_AFTER_A_CLOSED_ELEMENT: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11">
  <Languages>
    <Language Identifier="en-US">
      <TranslationUnit RefId="X">
        <TranslationElement RefId="EL-1">
          <Translation AttributeName="Name" Text="First" />
        </TranslationElement>
        <Translation AttributeName="Name" Text="Loose" />
      </TranslationUnit>
    </Language>
  </Languages>
</KNX>"#;

    #[test]
    fn a_translation_elements_ref_id_does_not_leak_past_its_own_end_tag() {
        let (_dir, conn) = db();
        ingest_translations(
            &conn,
            TranslationScope::Master,
            "t.xml",
            A_LOOSE_TRANSLATION_AFTER_A_CLOSED_ELEMENT.as_bytes(),
        )
        .unwrap();
        let first: String = conn
            .query_row(
                "SELECT text FROM translation WHERE ref_id = 'EL-1' AND attribute_name = 'Name'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(first, "First");
        // The "Loose" `Translation` is outside any `TranslationElement`, so
        // no `Start`/`Empty` event ever reassigns `ref_id` before it is
        // read — it must land under the empty-string ref_id. Without the
        // `"TranslationElement" => ref_id = None` reset on
        // `</TranslationElement>`, `ref_id` would still read `EL-1` here,
        // colliding with the row above; `INSERT OR IGNORE` would then
        // silently drop it and this query would find nothing.
        let second: String = conn
            .query_row(
                "SELECT text FROM translation WHERE ref_id = '' AND attribute_name = 'Name'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(second, "Loose");
    }

    // Same idea, one level up: a `TranslationUnit` sitting directly under
    // `Languages`, outside any `Language`, never fires a fresh `Language`
    // `Start`/`Empty` event, so only the `</Language>` reset can stop its
    // `Translation` inheriting the previous `Language`'s identifier.
    const A_LOOSE_TRANSLATION_UNIT_AFTER_A_CLOSED_LANGUAGE: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11">
  <Languages>
    <Language Identifier="en-US">
      <TranslationUnit RefId="X">
        <TranslationElement RefId="EL-2">
          <Translation AttributeName="Name" Text="English" />
        </TranslationElement>
      </TranslationUnit>
    </Language>
    <TranslationUnit RefId="X">
      <TranslationElement RefId="EL-2">
        <Translation AttributeName="Name" Text="Loose" />
      </TranslationElement>
    </TranslationUnit>
  </Languages>
</KNX>"#;

    #[test]
    fn a_languages_identifier_does_not_leak_past_its_own_end_tag() {
        let (_dir, conn) = db();
        ingest_translations(
            &conn,
            TranslationScope::Master,
            "t.xml",
            A_LOOSE_TRANSLATION_UNIT_AFTER_A_CLOSED_LANGUAGE.as_bytes(),
        )
        .unwrap();
        let first: String = conn
            .query_row(
                "SELECT text FROM translation WHERE language = 'en-US' AND ref_id = 'EL-2'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(first, "English");
        // The second `TranslationUnit` is outside any `Language`, so no
        // `Start`/`Empty` event ever reassigns `language` before its
        // `Translation` is read — it must land under the empty-string
        // language. Without the `"Language" => language = None` reset on
        // `</Language>`, `language` would still read `en-US` here, colliding
        // with the row above; `INSERT OR IGNORE` would then silently drop it
        // and this query would find nothing.
        let second: String = conn
            .query_row(
                "SELECT text FROM translation WHERE language = '' AND ref_id = 'EL-2'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(second, "Loose");
    }
}
