//! Shared `#[cfg(test)]` fixtures reused from Task 9 onward: the same
//! minimal hand-written document (Task 6's `MINIMAL` fixture, restated here
//! since that constant is private to `parse::installation`'s own test
//! module) and the same real reference project, parsed once per call rather
//! than re-derived ad hoc in every module that needs a `SourceDocument` to
//! exercise.

#![cfg(test)]

use crate::known::known_schema;
use crate::parse::parse_installation;
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

fn workspace_root() -> PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("crate lives at <root>/crates/<name>")
        .to_path_buf()
}

pub(crate) fn reference_ets4_bytes() -> Vec<u8> {
    std::fs::read(reference_ets4_path())
        .expect("reference ETS4 project is committed at the workspace root")
}

pub(crate) fn reference_ets4_path() -> PathBuf {
    workspace_root().join("Unser Zuhause ets4 - 2025-12-15.knxproj")
}

pub(crate) fn reference_ets6_path() -> PathBuf {
    workspace_root().join("Unser Zuhause ets 6.3.0 - 2026-09-02.knxproj")
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
