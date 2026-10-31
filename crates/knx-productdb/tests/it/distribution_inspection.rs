//! Tests useful read-only inspection of independently authored manufacturer distributions.

pub(super) use knx_productdb::distribution::{
    inspect_distribution, DistributionFallbackOrigin, DistributionLimitation,
    DistributionPassState, DistributionSectionKind, DistributionSourceKind,
};

// Authored for this test; no vendor XML, baggage, scripts or master data are copied.
pub(super) const COMBINED: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<KNX xmlns="http://knx.org/xml/project/20" CreatedBy="Independent Fixture">
  <ManufacturerData>
    <Manufacturer RefId="M-0001">
      <Catalog>
        <CatalogSection Id="S-1" Name="Examples">
          <CatalogItem Id="C-1" Name="Example item"
            ProductRefId="P-1" Hardware2ProgramRefId="H2P-1"/>
        </CatalogSection>
      </Catalog>
      <ApplicationPrograms>
        <ApplicationProgram Id="A-1" Name="Example application"
          ApplicationNumber="1" ApplicationVersion="1" MaskVersion="MV-0701">
          <Static>
            <ParameterTypes>
              <ParameterType Id="T-1" Name="Selector">
                <TypeNumber Type="unsignedInt" SizeInBit="8" minInclusive="0" maxInclusive="255"/>
              </ParameterType>
            </ParameterTypes>
            <Parameters>
              <Parameter Id="P-Control" Name="Control" ParameterType="T-1" Value="1"/>
            </Parameters>
            <ParameterRefs>
              <ParameterRef Id="R-Control" RefId="P-Control"/>
            </ParameterRefs>
          </Static>
          <Dynamic>
            <choose ParamRefId="R-Control">
              <when test="1"><ParameterRefRef RefId="R-Control"/></when>
            </choose>
          </Dynamic>
        </ApplicationProgram>
      </ApplicationPrograms>
      <Baggages><Baggage Id="B-1" TargetPath="docs" Name="help.txt"/></Baggages>
      <Hardware>
        <Hardware Id="H-1" Name="Example hardware" HasApplicationProgram="true">
          <Products><Product Id="P-1" Text="Example product" OrderNumber="EX-1"/></Products>
          <Hardware2Programs>
            <Hardware2Program Id="H2P-1">
              <ApplicationProgramRef RefId="A-1"/>
            </Hardware2Program>
          </Hardware2Programs>
        </Hardware>
      </Hardware>
    </Manufacturer>
  </ManufacturerData>
</KNX>"#;

#[test]
fn combined_source_exposes_typed_catalogue_hardware_and_dynamic_fallback_without_installing() {
    let original = COMBINED.to_vec();
    // Neither the filename nor a known producer string is an admission oracle.
    let result = inspect_distribution("independently-authored.data", &original);
    assert!(
        result.is_ok(),
        "a recognized combined manufacturer source must provide useful read-only inspection: {result:?}"
    );
    let report = result.unwrap();

    assert_eq!(original, COMBINED);
    assert_eq!(report.format_version, 2);
    assert_eq!(report.source_kind, DistributionSourceKind::ManufacturerXml);
    assert_eq!(report.source.name, "independently-authored.data");
    assert_eq!(report.source.size, COMBINED.len() as u64);
    assert_eq!(report.source.sha256, knx_productdb::sha256_hex(COMBINED));
    assert_eq!(
        report
            .sections
            .iter()
            .map(|section| section.kind)
            .collect::<Vec<_>>(),
        [
            DistributionSectionKind::Catalog,
            DistributionSectionKind::ApplicationPrograms,
            DistributionSectionKind::Baggages,
            DistributionSectionKind::Hardware,
        ]
    );
    for section in &report.sections {
        assert_eq!(section.declared_manufacturer_id.as_deref(), Some("M-0001"));
        let coordinate = report.resolve_coordinate(section.source).unwrap();
        assert!(coordinate.start_byte < coordinate.end_byte);
        assert!(coordinate.end_byte <= COMBINED.len() as u64);
        assert!(coordinate
            .path
            .starts_with("/KNX[1]/ManufacturerData[1]/Manufacturer[1]/"));
        assert_eq!(section.source.member_ordinal, None);
    }

    assert_eq!(report.catalog_items.len(), 1);
    let item = &report.catalog_items[0];
    assert_eq!(item.id.as_deref(), Some("C-1"));
    assert_eq!(item.name.as_deref(), Some("Example item"));
    assert_eq!(item.product_ref_id.as_deref(), Some("P-1"));
    assert_eq!(item.hardware2program_ref_id.as_deref(), Some("H2P-1"));
    assert_eq!(report.hardware.len(), 1);
    let hardware = &report.hardware[0];
    assert_eq!(hardware.id.as_deref(), Some("H-1"));
    assert_eq!(hardware.product_ids, ["P-1"]);
    assert_eq!(hardware.program_links.len(), 1);
    assert_eq!(hardware.program_links[0].id.as_deref(), Some("H2P-1"));
    assert_eq!(
        hardware.program_links[0].application_program_ref_ids,
        ["A-1"]
    );

    assert_eq!(report.applications.len(), 1);
    let application = &report.applications[0];
    assert_eq!(application.id.as_deref(), Some("A-1"));
    assert_eq!(application.name.as_deref(), Some("Example application"));
    assert_eq!(
        application.declared_manufacturer_id.as_deref(),
        Some("M-0001")
    );
    assert_eq!(application.parameter_fallbacks.len(), 1);
    let fallback = &application.parameter_fallbacks[0];
    assert_eq!(fallback.parameter_ref_id, "R-Control");
    assert_eq!(fallback.parameter_id, "P-Control");
    assert_eq!(fallback.value.as_deref(), Some("1"));
    assert_eq!(fallback.origin, DistributionFallbackOrigin::Parameter);
    assert_eq!(
        application.dynamic_state,
        DistributionPassState::PartiallyEvaluated
    );
    assert_eq!(application.active_parameter_ref_ids, ["R-Control"]);
    for limitation in [
        DistributionLimitation::MissingMaster,
        DistributionLimitation::XsdNotPerformed,
        DistributionLimitation::AuthenticityNotVerified,
        DistributionLimitation::WriteAuthorityNotPerformed,
        DistributionLimitation::InitializationNotPerformed,
    ] {
        assert!(
            report.limitations.contains(&limitation),
            "missing {limitation:?}"
        );
    }

    let json = serde_json::to_value(&report).unwrap();
    assert_eq!(json["formatVersion"], 2);
    assert_eq!(json["sourceKind"], "manufacturerXml");
    assert_eq!(
        json["applications"][0]["parameterFallbacks"][0]["origin"],
        "parameter"
    );
}
