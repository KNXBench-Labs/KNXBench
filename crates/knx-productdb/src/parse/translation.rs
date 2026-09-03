//! `Languages`/`Language`/`TranslationUnit`/`TranslationElement`/
//! `Translation`: per-language overrides of one attribute on one program
//! element, keyed by `(program_id, language, ref_id, attribute_name)`.

use rusqlite::Connection;

use crate::ProductDbError;

pub fn insert_translations(
    conn: &Connection,
    program_id: &str,
    language: &str,
    ref_id: &str,
    attribute_name: &str,
    text: &str,
) -> Result<(), ProductDbError> {
    conn.execute(
        "INSERT OR IGNORE INTO translation (program_id, language, ref_id, attribute_name, text)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![program_id, language, ref_id, attribute_name, text],
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
                 WHERE program_id = 'A-1' AND language = 'en-US'
                   AND ref_id = 'A-1_O-0' AND attribute_name = 'Text'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(text, "Output");
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
}
