//! Stage 6 (reverse direction): writing `knx_core::Project` back out to a
//! `.knxproj` container. `schema11` is the schema-11 XML writer (Task 18);
//! this module packs its output together with every opaque container entry
//! into a fresh ZIP archive (Task 19).

pub mod schema11;

pub use schema11::{write_installation_xml, write_project_xml, ExportError};

use std::io::{Cursor, Write};

use knx_core::Project;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

use crate::opaque::{OpaqueEntry, OpaqueKind};

pub struct ExportOutcome {
    pub bytes: Vec<u8>,
    pub warnings: Vec<ExportWarning>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportWarning {
    /// Always present. Every export this application produces is unsigned.
    Unsigned {
        detail: String,
    },
    /// A signature entry was copied through unchanged; it no longer matches
    /// the content it signs.
    StaleSignature {
        source_path: String,
    },
    ManufacturerDataFromOpaqueStore {
        entries: usize,
    },
}

/// Writes `project` and every opaque entry back out as a `.knxproj` ZIP
/// archive: a freshly generated `0.xml`/`Project.xml` (Task 18) plus every
/// other opaque entry copied through unchanged at its own `source_path`.
/// `RetainedAttribute`/`RetainedElement` entries are not written as
/// container files — they already went back into the generated XML — only
/// whole-file entries (`ContainerEntry`, `ManufacturerData`, `Baggage`,
/// `BinaryData`, `ExtraData`, `Signature`, `MasterData`) are.
pub fn export_knxproj(
    project: &Project,
    opaque: &[OpaqueEntry],
) -> Result<ExportOutcome, ExportError> {
    // Constructed before anything below can fail, so no code path produces
    // an export without this warning.
    let mut warnings = vec![ExportWarning::Unsigned {
        detail: "this export is not signed; whether ETS re-imports an unsigned \
                 third-party .knxproj file is untested against a real ETS \
                 installation (risk R9)"
            .to_string(),
    }];

    let installation_xml = write_installation_xml(project, opaque)?;
    let project_xml = write_project_xml(project, opaque)?;
    let project_id = &project.info.project_id;
    let installation_path = format!("{project_id}/0.xml");
    let project_info_path = format!("{project_id}/Project.xml");

    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default();

    write_entry(&mut writer, options, &installation_path, &installation_xml)?;
    write_entry(&mut writer, options, &project_info_path, &project_xml)?;

    let mut manufacturer_data_entries = 0usize;
    for entry in opaque {
        match entry.kind {
            OpaqueKind::RetainedAttribute | OpaqueKind::RetainedElement => continue,
            OpaqueKind::Signature => warnings.push(ExportWarning::StaleSignature {
                source_path: entry.source_path.clone(),
            }),
            OpaqueKind::ManufacturerData => manufacturer_data_entries += 1,
            _ => {}
        }
        write_entry(&mut writer, options, &entry.source_path, &entry.bytes)?;
    }
    if manufacturer_data_entries > 0 {
        warnings.push(ExportWarning::ManufacturerDataFromOpaqueStore {
            entries: manufacturer_data_entries,
        });
    }

    let bytes = writer
        .finish()
        .map_err(|e| ExportError::Xml(e.to_string()))?
        .into_inner();

    Ok(ExportOutcome { bytes, warnings })
}

fn write_entry(
    writer: &mut ZipWriter<Cursor<Vec<u8>>>,
    options: SimpleFileOptions,
    path: &str,
    bytes: &[u8],
) -> Result<(), ExportError> {
    writer
        .start_file(path, options)
        .map_err(|e| ExportError::Xml(e.to_string()))?;
    writer
        .write_all(bytes)
        .map_err(|e| ExportError::Xml(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container::Container;
    use crate::testutil::reference_ets4_path;

    #[test]
    fn every_export_is_unsigned_and_says_so() {
        let out = crate::import_knxproj(&reference_ets4_path()).unwrap();
        let exported = export_knxproj(&out.project, &out.opaque).unwrap();
        let unsigned = exported
            .warnings
            .iter()
            .find(|w| matches!(w, ExportWarning::Unsigned { .. }))
            .expect("no export may be produced without the unsigned warning");
        let ExportWarning::Unsigned { detail } = unsigned else {
            unreachable!()
        };
        assert!(detail.contains("untested"));
    }

    #[test]
    fn the_exported_container_holds_every_entry_the_source_had() {
        let out = crate::import_knxproj(&reference_ets4_path()).unwrap();
        let exported = export_knxproj(&out.project, &out.opaque).unwrap();
        let container = Container::open(exported.bytes).unwrap();
        assert_eq!(container.entries().len(), 38);
        assert!(container.find("P-0512/0.xml").is_some());
        assert!(container.find("M-0008/Baggages/econEts3.dll").is_some());
    }

    #[test]
    fn copied_signatures_are_reported_as_stale() {
        let out = crate::import_knxproj(&reference_ets4_path()).unwrap();
        let exported = export_knxproj(&out.project, &out.opaque).unwrap();
        let stale = exported
            .warnings
            .iter()
            .filter(|w| matches!(w, ExportWarning::StaleSignature { .. }))
            .count();
        assert_eq!(stale, 5); // four manufacturer signatures and one project signature
    }
}
