//! An ambiguous retained attribute is dropped and warned about, never guessed at (§34).

use std::io::{Cursor, Write};

use knx_etsproj::export::{export_knxproj, ExportWarning};
use knx_etsproj::opaque::OpaqueEntry;
use knx_etsproj::{import_knxproj_bytes, Container};

const PROJECT_INFO_21: &[u8] = br#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/21">
  <Project Id="P-0001">
    <ProjectInformation Name="T" GroupAddressStyle="ThreeLevel" />
  </Project>
</KNX>"#;

/// One line with *two* `Segment`s, each carrying its own `Puid`. A segment
/// has no identity in the domain model — `knx_core` has lines, not segments
/// — so both retained `Puid`s key on the same line, and which one belongs
/// to which segment is no longer recoverable. This is the shape the rule
/// exists for; the ETS files measured so far carry one segment per line.
const TWO_SEGMENTS: &[u8] = br#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/21" CreatedBy="ETS6" ToolVersion="ETS 6.1.0">
  <Project Id="P-0001">
    <Installations>
      <Installation Name="T" DefaultLine="P-0001-0_L-2">
        <Topology>
          <Area Id="P-0001-0_A-1" Address="1">
            <Line Id="P-0001-0_L-2" Address="1" Puid="11">
              <Segment Id="P-0001-0_L-2_S-1" Number="0" MediumTypeRefId="MT-0" Puid="101">
                <DeviceInstance Id="P-0001-0_DI-1" Name="D1" ProductRefId="M-0001_H-1_P-1"
                                Hardware2ProgramRefId="M-0001_H-1_HP-1" Address="1" Puid="201" />
              </Segment>
              <Segment Id="P-0001-0_L-2_S-2" Number="1" MediumTypeRefId="MT-0" Puid="102">
                <DeviceInstance Id="P-0001-0_DI-2" Name="D2" ProductRefId="M-0001_H-1_P-1"
                                Hardware2ProgramRefId="M-0001_H-1_HP-1" Address="2" Puid="202" />
              </Segment>
            </Line>
          </Area>
        </Topology>
      </Installation>
    </Installations>
  </Project>
</KNX>"#;

fn knxproj(installation_xml: &[u8]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for (name, bytes) in [
        ("P-0001.signature", b"x" as &[u8]),
        ("P-0001/0.xml", installation_xml),
        ("P-0001/Project.xml", PROJECT_INFO_21),
    ] {
        writer.start_file(name, options).unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn round_trip(installation_xml: &[u8]) -> (String, Vec<ExportWarning>) {
    let outcome = import_knxproj_bytes(knxproj(installation_xml), "ambiguity.knxproj").unwrap();
    let entries: Vec<OpaqueEntry> = outcome.opaque.to_vec();
    let exported = export_knxproj(&outcome.project, &entries).unwrap();
    let mut written = Container::open(exported.bytes).unwrap();
    let part = written.project_part().unwrap().to_string();
    let xml = String::from_utf8(written.read(&format!("{part}/0.xml")).unwrap()).unwrap();
    (xml, exported.warnings)
}

/// The ambiguous value is dropped, not guessed at, and the drop is
/// announced. §34: writing one segment's `Puid` onto both would be silent
/// corruption, and corruption is worse than loss.
#[test]
fn an_ambiguous_retained_attribute_is_omitted_and_warned_about() {
    let (xml, warnings) = round_trip(TWO_SEGMENTS);

    assert!(
        !xml.contains("Puid=\"101\"") && !xml.contains("Puid=\"102\""),
        "neither segment's Puid may be written when the two cannot be told \
         apart:\n{xml}"
    );

    let warning = warnings
        .iter()
        .find(|w| {
            matches!(
                w,
                ExportWarning::RetainedAttributeNotExported { element, attribute, .. }
                    if element == "Segment" && attribute == "Puid"
            )
        })
        .unwrap_or_else(|| panic!("no warning named the dropped Segment/@Puid: {warnings:?}"));
    let ExportWarning::RetainedAttributeNotExported {
        instances, detail, ..
    } = warning
    else {
        unreachable!("matched above")
    };
    assert_eq!(*instances, 2, "both segments should be counted");
    assert!(
        detail.contains("shared one `Segment` identity"),
        "the warning must say why, not merely that: {detail}"
    );
}

/// The unambiguous values on the same document are unaffected: each device
/// keeps its own `Puid`, and the line keeps its own. A rule that dropped
/// everything nearby would be safe and useless.
#[test]
fn unambiguous_neighbours_still_come_back() {
    let (xml, warnings) = round_trip(TWO_SEGMENTS);

    for puid in ["\"201\"", "\"202\"", "\"11\""] {
        assert!(
            xml.contains(&format!("Puid={puid}")),
            "Puid={puid} has an unambiguous owner and must be written back:\n{xml}"
        );
    }
    assert!(
        !warnings.iter().any(|w| matches!(
            w,
            ExportWarning::RetainedAttributeNotExported { element, .. }
                if element == "DeviceInstance"
        )),
        "no device attribute was dropped, so nothing should warn about one: {warnings:?}"
    );
}

/// One line, two `Segment`s, each with its own `BusAccess` — the element
/// flavour of the same collision. Both key on
/// `…/Line[@Id='…']/Segment/BusAccess`, because the exporter synthesizes
/// one segment per line and there is nothing else to key by.
const TWO_BUS_ACCESS: &[u8] = br#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/21" CreatedBy="ETS6" ToolVersion="ETS 6.1.0">
  <Project Id="P-0001">
    <Installations>
      <Installation Name="T" DefaultLine="P-0001-0_L-2">
        <Topology>
          <Area Id="P-0001-0_A-1" Address="1">
            <Line Id="P-0001-0_L-2" Address="1">
              <Segment Id="P-0001-0_L-2_S-1" Number="0" MediumTypeRefId="MT-0">
                <DeviceInstance Id="P-0001-0_DI-1" Name="D1" ProductRefId="M-0001_H-1_P-1"
                                Hardware2ProgramRefId="M-0001_H-1_HP-1" Address="1" />
                <BusAccess Name="alpha" Edi="one" />
              </Segment>
              <Segment Id="P-0001-0_L-2_S-2" Number="1" MediumTypeRefId="MT-0">
                <DeviceInstance Id="P-0001-0_DI-2" Name="D2" ProductRefId="M-0001_H-1_P-1"
                                Hardware2ProgramRefId="M-0001_H-1_HP-1" Address="2" />
                <BusAccess Name="beta" Edi="two" />
              </Segment>
            </Line>
          </Area>
        </Topology>
      </Installation>
    </Installations>
  </Project>
</KNX>"#;

/// The retained-element entries an import of `TWO_BUS_ACCESS` produces,
/// with the collision the store is meant to survive actually present.
///
/// The importer alone cannot produce it today: `SourceLine::bus_access` is
/// an `Option`, so the second `<BusAccess>` overwrites the first while the
/// document is still being parsed and only one entry ever reaches the
/// opaque store. That is an import-side loss of its own — recorded in this
/// task's report — and it is not what this test is about. The store is the
/// boundary under test here, and a store can hold the pair: any importer
/// that stops dropping the first element, and any project file written by
/// one that already does, hands the exporter two blobs under one key.
fn entries_with_a_colliding_twin(opaque: &[OpaqueEntry]) -> Vec<OpaqueEntry> {
    let mut entries = opaque.to_vec();
    let survivor = entries
        .iter()
        .find(|e| e.xpath.ends_with("/Segment/BusAccess"))
        .cloned()
        .expect("the import retained one of the two BusAccess elements");
    entries.push(OpaqueEntry {
        bytes: br#"<BusAccess Name="alpha" Edi="one" />"#.to_vec(),
        // Never read by the export; the store checks its own hashes.
        sha256: String::new(),
        ..survivor
    });
    entries
}

/// Two blobs under one key: neither is written, and the export says how
/// many elements it is talking about. Writing the one the store happened
/// to see last would hand one segment's bus access to the other, which is
/// §34's forbidden trade — and it is what this store did before the rule
/// reached its element half.
#[test]
fn an_ambiguous_retained_element_is_omitted_and_warned_about() {
    let outcome = import_knxproj_bytes(knxproj(TWO_BUS_ACCESS), "ambiguity.knxproj").unwrap();
    let entries = entries_with_a_colliding_twin(&outcome.opaque);
    let exported = export_knxproj(&outcome.project, &entries).unwrap();
    let mut written = Container::open(exported.bytes).unwrap();
    let part = written.project_part().unwrap().to_string();
    let xml = String::from_utf8(written.read(&format!("{part}/0.xml")).unwrap()).unwrap();

    assert!(
        !xml.contains("BusAccess"),
        "neither segment's BusAccess may be written when the two cannot be \
         told apart:\n{xml}"
    );

    let warnings: Vec<_> = exported
        .warnings
        .iter()
        .filter(|w| {
            matches!(
                w,
                ExportWarning::RetainedElementNotExported { element, .. } if element == "BusAccess"
            )
        })
        .collect();
    assert_eq!(
        warnings.len(),
        1,
        "one warning per element class, not one per instance: {:?}",
        exported.warnings
    );
    let ExportWarning::RetainedElementNotExported {
        instances, detail, ..
    } = warnings[0]
    else {
        unreachable!("filtered above")
    };
    assert_eq!(*instances, 2, "both segments should be counted");
    assert!(
        detail.contains("shared one `<BusAccess>` identity"),
        "the warning must say why, not merely that: {detail}"
    );
}

/// The control: one `BusAccess` under one segment still comes back
/// verbatim, and nothing warns. A rule that dropped every retained element
/// would pass the test above and be useless.
#[test]
fn an_unambiguous_retained_element_still_comes_back() {
    let (xml, warnings) = round_trip(ONE_BUS_ACCESS);
    assert!(
        xml.contains(r#"<BusAccess Name="alpha" Edi="one" />"#),
        "the only BusAccess in the document has an unambiguous owner:\n{xml}"
    );
    assert!(
        !warnings
            .iter()
            .any(|w| matches!(w, ExportWarning::RetainedElementNotExported { .. })),
        "nothing was left behind, so nothing should warn: {warnings:?}"
    );
}

/// `TWO_BUS_ACCESS` with the second segment removed. Both fixtures carry a
/// device per segment on purpose: the schema-≥21 writer spells a
/// device-less `Segment` as an empty element and never asks the store for
/// its `BusAccess` at all, which would make the assertion below pass
/// without the rule under test ever running.
const ONE_BUS_ACCESS: &[u8] = br#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/21" CreatedBy="ETS6" ToolVersion="ETS 6.1.0">
  <Project Id="P-0001">
    <Installations>
      <Installation Name="T" DefaultLine="P-0001-0_L-2">
        <Topology>
          <Area Id="P-0001-0_A-1" Address="1">
            <Line Id="P-0001-0_L-2" Address="1">
              <Segment Id="P-0001-0_L-2_S-1" Number="0" MediumTypeRefId="MT-0">
                <DeviceInstance Id="P-0001-0_DI-1" Name="D1" ProductRefId="M-0001_H-1_P-1"
                                Hardware2ProgramRefId="M-0001_H-1_HP-1" Address="1" />
                <BusAccess Name="alpha" Edi="one" />
              </Segment>
            </Line>
          </Area>
        </Topology>
      </Installation>
    </Installations>
  </Project>
</KNX>"#;

/// A file that breaks the one assumption the Unique/Ambiguous rule rests
/// on: two `DeviceInstance` elements sharing an `@Id`, only the first of
/// them carrying a `Comment`. ETS does not write such a file; nothing
/// stops one existing.
const DUPLICATE_DEVICE_ID: &[u8] = br#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/21" CreatedBy="ETS6" ToolVersion="ETS 6.1.0">
  <Project Id="P-0001">
    <Installations>
      <Installation Name="T" DefaultLine="P-0001-0_L-2">
        <Topology>
          <Area Id="P-0001-0_A-1" Address="1">
            <Line Id="P-0001-0_L-2" Address="1">
              <Segment Id="P-0001-0_L-2_S-1" Number="0" MediumTypeRefId="MT-0">
                <DeviceInstance Id="P-0001-0_DI-1" Name="D1" ProductRefId="M-0001_H-1_P-1"
                                Hardware2ProgramRefId="M-0001_H-1_HP-1" Address="1"
                                Comment="only on the first" />
                <DeviceInstance Id="P-0001-0_DI-1" Name="D2" ProductRefId="M-0001_H-1_P-1"
                                Hardware2ProgramRefId="M-0001_H-1_HP-1" Address="2" />
              </Segment>
            </Line>
          </Area>
        </Topology>
      </Installation>
    </Installations>
  </Project>
</KNX>"#;

/// **Pinned, not approved.** Two elements sharing one ETS `@Id` collapse
/// into one retained key, and because only one of them carried the
/// `Comment`, the store sees one value, calls it unambiguous, and writes it
/// onto both copies — a retained value on an element that never had it.
///
/// The rule cannot catch this on its own: ambiguity is derived from the
/// values in the store, and one value looks the same whether it came from
/// one element or from the only one of two that bothered to carry it.
/// Carrying an element-instance count instead would mean carrying it
/// through the opaque store, i.e. a stored-project format change, for a
/// shape no ETS file has (see `crate::xpath`'s module doc).
///
/// Note what this test also shows: the far larger damage is upstream of
/// the retained store. Both exported devices come back as the *second*
/// device — same `Name`, same `Address` — because the model is keyed by
/// ETS id too, so the first device's own identity is gone before export
/// ever runs. A duplicate `@Id` is an import-validation problem (T06);
/// fixing it here would leave the bigger half of the corruption in place.
#[test]
fn a_duplicate_ets_id_puts_a_retained_value_on_both_copies() {
    let (xml, _warnings) = round_trip(DUPLICATE_DEVICE_ID);
    assert_eq!(
        xml.matches("<DeviceInstance ").count(),
        2,
        "the export writes one element per modeled device:\n{xml}"
    );
    assert_eq!(
        xml.matches(r#"Comment="only on the first""#).count(),
        2,
        "pinning today's behaviour: the retained Comment lands on both \
         copies. If this count drops to 1, the element-instance count \
         arrived after all and this test should become an assertion that \
         nothing is written at all:\n{xml}"
    );
    assert_eq!(
        xml.matches(r#"Name="D1""#).count(),
        0,
        "pinning the upstream collapse: the first device's own name does \
         not survive the model's id table either:\n{xml}"
    );
}

/// The same asymmetry one element up, in the shape that *is* reachable
/// from a well-formed ETS file: two `Segment`s under one `Line`, only one
/// of them carrying a `Puid`. The exporter synthesizes a single `Segment`
/// per line — the merge of both — and the lone `Puid` is written onto it.
///
/// Pinned rather than celebrated. Nothing here can tell which segment the
/// value belongs to; what saves it from being wrong is that the element it
/// lands on is not a third party but the merge of the two candidates, the
/// same merge the parser already performed on `MediumTypeRefId` and the
/// domain-address attributes. When both segments carry the attribute, the
/// values differ and the rule drops both — the test above.
#[test]
fn an_asymmetric_segment_attribute_lands_on_the_merged_segment() {
    let (xml, warnings) = round_trip(ONE_SIDED_SEGMENT_PUID);
    assert_eq!(
        xml.matches("<Segment ").count(),
        1,
        "one segment per line is what the writer synthesizes:\n{xml}"
    );
    assert!(
        xml.contains(r#"Puid="101""#),
        "pinning today's behaviour: a value only one of the two segments \
         carried is written onto the merged segment:\n{xml}"
    );
    assert!(
        !warnings.iter().any(|w| matches!(
            w,
            ExportWarning::RetainedAttributeNotExported { element, attribute, .. }
                if element == "Segment" && attribute == "Puid"
        )),
        "and nothing warns about it, because from the store's side it was \
         never ambiguous: {warnings:?}"
    );
}

/// `TWO_SEGMENTS` with the second segment's `Puid` taken away.
const ONE_SIDED_SEGMENT_PUID: &[u8] = br#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/21" CreatedBy="ETS6" ToolVersion="ETS 6.1.0">
  <Project Id="P-0001">
    <Installations>
      <Installation Name="T" DefaultLine="P-0001-0_L-2">
        <Topology>
          <Area Id="P-0001-0_A-1" Address="1">
            <Line Id="P-0001-0_L-2" Address="1">
              <Segment Id="P-0001-0_L-2_S-1" Number="0" MediumTypeRefId="MT-0" Puid="101">
                <DeviceInstance Id="P-0001-0_DI-1" Name="D1" ProductRefId="M-0001_H-1_P-1"
                                Hardware2ProgramRefId="M-0001_H-1_HP-1" Address="1" />
              </Segment>
              <Segment Id="P-0001-0_L-2_S-2" Number="1" MediumTypeRefId="MT-0">
                <DeviceInstance Id="P-0001-0_DI-2" Name="D2" ProductRefId="M-0001_H-1_P-1"
                                Hardware2ProgramRefId="M-0001_H-1_HP-1" Address="2" />
              </Segment>
            </Line>
          </Area>
        </Topology>
      </Installation>
    </Installations>
  </Project>
</KNX>"#;
