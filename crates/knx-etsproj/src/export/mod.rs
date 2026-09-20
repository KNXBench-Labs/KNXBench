//! Stage 6 (reverse direction): writing `knx_core::Project` back out to a
//! `.knxproj` container. `schema11` is the schema-11 XML writer (Task 18);
//! this module packs its output together with every opaque container entry
//! into a fresh ZIP archive (Task 19).

pub mod schema11;
pub mod schema21;

pub use schema11::{write_installation_xml, write_project_xml, ExportError};

use std::io::{Cursor, Write};

use knx_core::{Override, Project};
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
    Unsigned { detail: String },
    /// A signature entry was copied through unchanged; it no longer matches
    /// the content it signs.
    StaleSignature { source_path: String },
    /// Manufacturer data was written from the shared product database
    /// rather than from the project file (ADR-0005). Always present when
    /// the export carried any.
    ManufacturerDataFromProductDb { entries: usize },
    /// The project's manifest named a manufacturer file the product
    /// database could not supply. The container is written without it, and
    /// this says which one — never a silently incomplete archive. Raised
    /// by `knx-app` (Task 14), which is the layer that knows the manifest.
    MissingManufacturerData { source_path: String, sha256: String },
    /// Read-on-Init is switched *on* at instance level in this project, and
    /// the written container does not carry it. No `.knxproj` measured here
    /// spells `ReadOnInitFlag` on a `ComObjectInstanceRef` — 2533
    /// occurrences across the local corpus, every one of them on an
    /// application program's `ComObject` — so writing one would mean
    /// inventing an attribute position ETS may well reject. The flag stays
    /// in KNXBench's own project file; this says out loud which objects
    /// leave it behind (KNOWN_LIMITATIONS §117).
    ///
    /// Counts only values that are `true`. A `false` at an exported layer
    /// re-resolves to `false` from the product database — every one of those
    /// 2533 program-level occurrences reads `"Disabled"` — so the exported
    /// container and the project agree about it, and warning about it would
    /// mean a warning nobody can clear: switching the flag on and off again
    /// leaves `Value(false)` behind, and no "clear to inherited" gesture
    /// exists in the UI. The narrow case this does not cover — a user `false`
    /// against an application program that states `Enabled`, never yet
    /// measured — is written down in KNOWN_LIMITATIONS §117 instead of being
    /// announced on every export forever.
    ReadOnInitNotExported { com_objects: usize },
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

    let (installation_xml, project_xml) = if project.info.ets_schema_version >= 21 {
        (
            schema21::write_installation_xml_v21(project, opaque)?,
            schema21::write_project_xml_v21(project, opaque)?,
        )
    } else {
        (
            write_installation_xml(project, opaque)?,
            write_project_xml(project, opaque)?,
        )
    };
    // `r.value` on purpose: see `ExportWarning::ReadOnInitNotExported`. An
    // exported-layer `false` survives the round trip through the product
    // database, so warning about it would only produce an unclearable
    // warning.
    let unexportable_read_on_init = project
        .devices
        .com_objects()
        .filter(|com| {
            matches!(com.flags.read_on_init, Override::Value(ref r) if r.layer.is_exported() && r.value)
        })
        .count();
    if unexportable_read_on_init > 0 {
        warnings.push(ExportWarning::ReadOnInitNotExported {
            com_objects: unexportable_read_on_init,
        });
    }

    let project_id = &project.info.project_id;
    let installation_path = format!("{project_id}/0.xml");
    // ETS itself spells this entry `Project.xml` at schema 11 (ETS4) but
    // lowercase `project.xml` at schema ≥21 (ETS5/6) — measured directly:
    // `unzip -l` on both the ETS4 reference project and `KV v2.5 -
    // demo.knxproj` (`crates/knx-etsproj/src/source.rs`'s own module doc
    // already notes this split). `Container::read`'s case-insensitive
    // lookup means our own reader tolerates either spelling, but a
    // case-sensitive ZIP consumer — including real ETS — would not find
    // `project.xml` under `Project.xml`, so the casing this writer emits
    // is not cosmetic.
    let project_info_path = if project.info.ets_schema_version >= 21 {
        format!("{project_id}/project.xml")
    } else {
        format!("{project_id}/Project.xml")
    };

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
        warnings.push(ExportWarning::ManufacturerDataFromProductDb {
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
    use crate::opaque::ManufacturerFile;
    use crate::testutil::reference_ets4_path;

    /// Reassembles the full entry list `export_knxproj` needs: every opaque
    /// entry plus every manufacturer file converted back into an
    /// `OpaqueEntry` at its own `source_path` — what `knx-app` does for
    /// real once the product database exists (Task 14).
    fn all_entries(opaque: &[OpaqueEntry], manufacturer: &[ManufacturerFile]) -> Vec<OpaqueEntry> {
        opaque
            .iter()
            .cloned()
            .chain(manufacturer.iter().map(|m| OpaqueEntry {
                source_path: m.source_path.clone(),
                xpath: String::new(),
                kind: m.kind,
                name: String::new(),
                bytes: m.bytes.clone(),
                sha256: m.sha256.clone(),
            }))
            .collect()
    }

    #[test]
    fn every_export_is_unsigned_and_says_so() {
        if !crate::testutil::corpus_available() {
            eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
            return;
        }
        let out = crate::import_knxproj(&reference_ets4_path()).unwrap();
        let entries = all_entries(&out.opaque, &out.manufacturer);
        let exported = export_knxproj(&out.project, &entries).unwrap();
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
        if !crate::testutil::corpus_available() {
            eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
            return;
        }
        let out = crate::import_knxproj(&reference_ets4_path()).unwrap();
        let entries = all_entries(&out.opaque, &out.manufacturer);
        let exported = export_knxproj(&out.project, &entries).unwrap();
        let container = Container::open(exported.bytes).unwrap();
        assert_eq!(container.entries().len(), 38);
        assert!(container.find("P-0512/0.xml").is_some());
        assert!(container.find("M-0008/Baggages/econEts3.dll").is_some());
    }

    #[test]
    fn copied_signatures_are_reported_as_stale() {
        if !crate::testutil::corpus_available() {
            eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
            return;
        }
        let out = crate::import_knxproj(&reference_ets4_path()).unwrap();
        let entries = all_entries(&out.opaque, &out.manufacturer);
        let exported = export_knxproj(&out.project, &entries).unwrap();
        let stale = exported
            .warnings
            .iter()
            .filter(|w| matches!(w, ExportWarning::StaleSignature { .. }))
            .count();
        assert_eq!(stale, 5); // four manufacturer signatures and one project signature
    }

    /// Literal, case-preserving ZIP entry names for `exported.bytes` —
    /// deliberately not `Container::find`/`Container::read`, which match
    /// case-insensitively (`container.rs`'s own doc comment) precisely so
    /// *our* reader tolerates either spelling. That tolerance would hide
    /// the exact regression these two tests exist to catch: a real ETS or
    /// any case-sensitive ZIP consumer does not get the same latitude.
    fn literal_entry_names(bytes: Vec<u8>) -> Vec<String> {
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        (0..archive.len())
            .map(|i| archive.by_index(i).unwrap().name().to_string())
            .collect()
    }

    #[test]
    fn a_schema_21_export_writes_lowercase_project_xml() {
        if !crate::testutil::corpus_available() {
            eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
            return;
        }
        // ETS itself spells this entry lowercase from schema ≥21 onward
        // (measured — `source.rs`'s own module doc, `unzip -l` on `KV v2.5
        // - demo.knxproj`), unlike schema 11's `Project.xml`.
        let document = crate::testutil::reference_kv_source_document();
        let mapped = crate::map::map(&document, "P-03DE/0.xml");
        let exported = export_knxproj(&mapped.project, &[]).unwrap();
        let names = literal_entry_names(exported.bytes);
        assert!(
            names.iter().any(|n| n == "P-03DE/project.xml"),
            "expected a literal lowercase P-03DE/project.xml entry, got: {names:?}"
        );
        assert!(!names.iter().any(|n| n == "P-03DE/Project.xml"));
    }

    #[test]
    fn a_schema_11_export_still_writes_capitalized_project_xml() {
        if !crate::testutil::corpus_available() {
            eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
            return;
        }
        let out = crate::import_knxproj(&reference_ets4_path()).unwrap();
        let entries = all_entries(&out.opaque, &out.manufacturer);
        let exported = export_knxproj(&out.project, &entries).unwrap();
        let names = literal_entry_names(exported.bytes);
        assert!(names.iter().any(|n| n == "P-0512/Project.xml"));
        assert!(!names.iter().any(|n| n == "P-0512/project.xml"));
    }
}
