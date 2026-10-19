//! A second project part in one archive is kept and named in the report, not passed over.
//
// AR18 independent review, finding M3 (2026-10-06): an archive with
// `P-0001` and `P-0002` imported only the first; the second's files became
// ordinary opaque entries without a single report line.

use std::io::{Cursor, Write};

use knx_etsproj::import_knxproj_bytes;

fn installation(id: &str) -> Vec<u8> {
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11" CreatedBy="ETS4" ToolVersion="ETS 4.1.8">
  <Project Id="{id}">
    <Installations>
      <Installation InstallationId="0" Name="" CompletionStatus="Undefined">
        <Topology />
        <GroupAddresses><GroupRanges /></GroupAddresses>
      </Installation>
    </Installations>
  </Project>
</KNX>"#
    )
    .into_bytes()
}

fn project_info(id: &str) -> Vec<u8> {
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11" CreatedBy="ETS4" ToolVersion="ETS 4.1.8">
  <Project Id="{id}"><ProjectInformation Name="Fictional {id}" /></Project>
</KNX>"#
    )
    .into_bytes()
}

fn archive(parts: &[&str]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for part in parts {
        for (name, bytes) in [
            (format!("{part}.signature"), b"x".to_vec()),
            (format!("{part}/0.xml"), installation(part)),
            (format!("{part}/Project.xml"), project_info(part)),
        ] {
            writer.start_file(name, options).unwrap();
            writer.write_all(&bytes).unwrap();
        }
    }
    writer.finish().unwrap().into_inner()
}

#[test]
fn a_second_project_part_is_named_in_the_report() {
    let outcome =
        import_knxproj_bytes(archive(&["P-0001", "P-0002"]), "two-parts.knxproj").unwrap();
    let lines: Vec<_> = outcome
        .report
        .unsupported
        .iter()
        .filter(|feature| feature.what.contains("P-0002"))
        .collect();
    assert_eq!(lines.len(), 1, "{:?}", outcome.report.unsupported);
    assert!(
        lines[0].what.contains("additional project part"),
        "{}",
        lines[0].what
    );
    assert!(
        lines[0].consequence.contains("P-0001"),
        "{}",
        lines[0].consequence
    );
    // Still kept: the second part's files are passthrough evidence.
    assert!(outcome
        .opaque
        .iter()
        .any(|entry| entry.source_path == "P-0002/0.xml"));
}

#[test]
fn a_single_part_archive_reports_no_extra_part() {
    let outcome = import_knxproj_bytes(archive(&["P-0001"]), "one-part.knxproj").unwrap();
    assert!(
        !outcome
            .report
            .unsupported
            .iter()
            .any(|feature| feature.what.contains("additional project part")),
        "{:?}",
        outcome.report.unsupported
    );
}
