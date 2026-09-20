//! Shared `#[cfg(test)]` fixtures reused from Task 9 onward: the same
//! minimal hand-written document (Task 6's `MINIMAL` fixture, restated here
//! since that constant is private to `parse::installation`'s own test
//! module) and the same real reference project, parsed once per call rather
//! than re-derived ad hoc in every module that needs a `SourceDocument` to
//! exercise.

#![cfg(test)]

use crate::known::known_schema;
use crate::parse::{parse_installation, parse_installation_v21, parse_project_info};
use crate::source::SourceDocument;
use crate::Container;
use std::path::PathBuf;

const MINIMAL: &[u8] = br#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11" CreatedBy="ETS4" ToolVersion="ETS 4.1.8">
  <Project Id="P-0001">
    <Installations>
      <Installation InstallationId="0" Name="" DefaultLine="P-0001-0_L-2" CompletionStatus="Undefined">
        <Topology>
          <Area Id="P-0001-0_A-1" Name="A" Address="1" CompletionStatus="Undefined">
            <Line Id="P-0001-0_L-2" Name="L" Address="1" MediumTypeRefId="MT-0" CompletionStatus="Accepted">
              <DeviceInstance Id="P-0001-0_DI-1" Name="D" ProductRefId="M-0001_H-1_P-1"
                              Hardware2ProgramRefId="M-0001_H-1_HP-1" Address="1"
                              LastModified="2023-07-14T11:55:33" CompletionStatus="FinishedDesign"
                              IndividualAddressLoaded="1" ApplicationProgramLoaded="1"
                              ParametersLoaded="1" CommunicationPartLoaded="1"
                              MediumConfigLoaded="1" IsCommunicationObjectVisibilityCalculated="1"
                              Broken="0">
                <ComObjectInstanceRefs>
                  <ComObjectInstanceRef RefId="M-0001_A-1_O-0_R-1" DatapointType="" IsActive="1">
                    <Connectors><Send GroupAddressRefId="P-0001-0_GA-1" /></Connectors>
                  </ComObjectInstanceRef>
                </ComObjectInstanceRefs>
              </DeviceInstance>
            </Line>
          </Area>
        </Topology>
        <GroupAddresses>
          <GroupRanges>
            <GroupRange Id="P-0001-0_GR-1" Name="Licht" RangeStart="1" RangeEnd="255">
              <GroupRange Id="P-0001-0_GR-2" Name="An/Aus" RangeStart="1" RangeEnd="127">
                <GroupAddress Id="P-0001-0_GA-1" Address="1" Name="GA" />
              </GroupRange>
            </GroupRange>
          </GroupRanges>
        </GroupAddresses>
      </Installation>
    </Installations>
  </Project>
</KNX>"#;

pub(crate) fn minimal_source_document() -> SourceDocument {
    parse_installation(MINIMAL, "P-0001/0.xml", known_schema(11).unwrap())
        .unwrap()
        .document
}

pub(crate) fn reference_ets4_bytes() -> Vec<u8> {
    std::fs::read(reference_ets4_path())
        .expect("reference ETS4 project lives under OriginalData/ (gitignored); see knx_testsupport::corpus_available")
}

pub(crate) fn reference_ets4_path() -> PathBuf {
    knx_testsupport::reference_ets4_path()
}

/// True when the gitignored `OriginalData/` fixture corpus is present
/// locally. It holds the maintainer's own real KNX installation and
/// manufacturer files — never committed, so CI (and any contributor
/// without a copy) has none of it. Every test that needs the corpus must
/// check this first and skip, not panic, or CI is permanently red.
pub(crate) fn corpus_available() -> bool {
    knx_testsupport::corpus_available()
}

pub(crate) fn reference_ets6_path() -> PathBuf {
    knx_testsupport::reference_ets6_path()
}

/// A second, genuinely independent installation (KNX Association demo
/// project, not a re-export of the "Unser Zuhause" reference project) —
/// schema 21, ETS 5.7. Session 7 evidence: schema 21 already carries the
/// `Segment`/`GroupObjectTree`/`ModuleInstances`/`Locations` deltas
/// previously attributed to schema 23 alone (RESEARCH §2.5/§3.4).
pub(crate) fn reference_kv_schema21_path() -> PathBuf {
    knx_testsupport::reference_kv_schema21_path()
}

pub(crate) fn reference_source_document() -> SourceDocument {
    let mut c = Container::open(reference_ets4_bytes()).unwrap();
    let bytes = c.read("P-0512/0.xml").unwrap();
    parse_installation(&bytes, "P-0512/0.xml", known_schema(11).unwrap())
        .unwrap()
        .document
}

pub(crate) fn reference_project() -> knx_core::Project {
    crate::map::map(&reference_source_document(), "P-0512/0.xml").project
}

/// Schema-≥21 counterpart of [`reference_source_document`]: the KV demo
/// project's `0.xml`, parsed through [`parse_installation_v21`], with its
/// `project.xml` folded into `document.info` exactly as
/// `import_knxproj_bytes` does for schema 11's `Project.xml` — `lib.rs`
/// itself does not dispatch to `parse_installation_v21` yet (later task), so
/// this helper calls it directly rather than going through
/// `import_knxproj`/`import_knxproj_bytes`.
pub(crate) fn reference_kv_source_document() -> SourceDocument {
    let schema = known_schema(21).unwrap();
    let mut c = Container::open(std::fs::read(reference_kv_schema21_path()).unwrap()).unwrap();
    let topology_bytes = c.read("P-03DE/0.xml").unwrap();
    let mut document = parse_installation_v21(&topology_bytes, "P-03DE/0.xml", schema)
        .unwrap()
        .document;
    let info_bytes = c.read("P-03DE/project.xml").unwrap();
    let (info, _) = parse_project_info(&info_bytes, "P-03DE/project.xml", schema).unwrap();
    document.info = info;
    document
}
