//! Synthetic KL-006 witness: one plannable program that may declare an ETS download plug-in.
//!
//! Manufacturer `M-FFF6`, mask `0701h`, a complete `ProductProcedure` that
//! loads an address table segment (LSM 1) and an application segment with
//! its task segment (LSM 3), so the complete download and every partial
//! scope derive. [`declaration`] adds `Static/Extension/@EtsDownloadPlugin`
//! with a baggage reference, as a product file declares a vendor download
//! plug-in. The GUID is made up: the refusal rule is the declaration.

/// The application program's id.
pub const PROGRAM_ID: &str = "M-FFF6_A-0001-01-0000";
/// The declared plug-in: a made-up GUID.
pub const PLUGIN_GUID: &str = "{00000000-0000-4000-8000-00000000F006}";
/// The product a project device references.
pub const PRODUCT_REF: &str = "M-FFF6_H-1_P-1";
/// The `Hardware2Program` a project device references.
pub const HARDWARE2PROGRAM_REF: &str = "M-FFF6_H-1_HP-1";
/// The individual address the tests give their one device.
pub const TARGET: &str = "1.1.7";

/// The program, with `extension` placed inside its `Static`.
pub fn program_xml(extension: &str) -> String {
    format!(
        r#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-FFF6"><ApplicationPrograms>
<ApplicationProgram Id="{PROGRAM_ID}" Name="Synthetic plug-in witness" ApplicationNumber="1" ApplicationVersion="1" PeiType="0"
  MaskVersion="MV-0701" LoadProcedureStyle="ProductProcedure"><Static>
<Code>
  <AbsoluteSegment Id="{PROGRAM_ID}_AS-4000" Address="16384" Size="8"><Data>AAAAAAAAAAA=</Data></AbsoluteSegment>
  <AbsoluteSegment Id="{PROGRAM_ID}_AS-4400" Address="17408" Size="4"><Data>AQIDBA==</Data></AbsoluteSegment>
</Code>
<AddressTable CodeSegment="{PROGRAM_ID}_AS-4000" Offset="0" MaxEntries="2" />
<LoadProcedures><LoadProcedure>
  <LdCtrlConnect /><LdCtrlUnload LsmIdx="1" /><LdCtrlUnload LsmIdx="3" />
  <LdCtrlLoad LsmIdx="1" />
  <LdCtrlAbsSegment LsmIdx="1" SegType="0" Address="16384" Size="8" Access="255" MemType="3" SegFlags="128" />
  <LdCtrlLoadCompleted LsmIdx="1" />
  <LdCtrlLoad LsmIdx="3" />
  <LdCtrlAbsSegment LsmIdx="3" SegType="0" Address="17408" Size="4" Access="255" MemType="3" SegFlags="128" />
  <LdCtrlTaskSegment LsmIdx="3" Address="17408" />
  <LdCtrlLoadCompleted LsmIdx="3" />
  <LdCtrlRestart /><LdCtrlDisconnect />
</LoadProcedure></LoadProcedures>
{extension}
</Static><Dynamic /></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#
    )
}

/// The declaration as a product file carries it: the plug-in's GUID and
/// its DLL as a baggage reference.
pub fn declaration() -> String {
    format!(
        r#"<Extension EtsDownloadPlugin="{PLUGIN_GUID}" RequiresExternalSoftware="0"><Baggage RefId="M-FFF6_BG-plugin.dll" /></Extension>"#
    )
}

/// The hardware file that makes [`PRODUCT_REF`] and
/// [`HARDWARE2PROGRAM_REF`] resolve to [`PROGRAM_ID`].
pub const HARDWARE_XML: &str = r#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-FFF6"><Hardware>
<Hardware Id="M-FFF6_H-1" Name="Synthetic" SerialNumber="synthetic" VersionNumber="1"><Products><Product Id="M-FFF6_H-1_P-1" /></Products>
<Hardware2Programs><Hardware2Program Id="M-FFF6_H-1_HP-1" MediumTypes="MT-0"><ApplicationProgramRef RefId="M-FFF6_A-0001-01-0000" /></Hardware2Program></Hardware2Programs>
</Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;
