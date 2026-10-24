//! Synthetic mask-`0701h` program for association link adoption tests.
//!
//! Address table at 4000h (4 entries), association table at 4100h (4 entries),
//! group object table with objects 0, 1 and 2 at 4400h and one parameter at
//! 440Fh. Object 2 is active only when that parameter is 1. Application
//! `M-0001_A-0002-01-0000` (manufacturer 0001h, number 2, version 1) behind
//! `Hardware2Program` `HP`.

/// The application program.
pub const LINK_ADOPTION_PROGRAM_XML: &str = r#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0001"><ApplicationPrograms>
<ApplicationProgram Id="M-0001_A-0002-01-0000" Name="Links" ApplicationNumber="2" ApplicationVersion="1" PeiType="0" MaskVersion="MV-0701" LoadProcedureStyle="ProductProcedure"><Static>
<Code><AbsoluteSegment Id="AT" Address="16384" Size="11"><Data>AAAAAAAAAAAAAAA=</Data></AbsoluteSegment>
<AbsoluteSegment Id="AS" Address="16640" Size="9"><Data>AAAAAAAAAAAA</Data></AbsoluteSegment>
<AbsoluteSegment Id="PS" Address="17408" Size="16"><Data>AwAAAAAAAAAAAAAAAAAAAA==</Data></AbsoluteSegment></Code>
<ParameterTypes><ParameterType Id="T"><TypeNumber SizeInBit="8" Type="unsignedInt" minInclusive="0" maxInclusive="255" /></ParameterType></ParameterTypes>
<Parameters><Parameter Id="M-0001_A-0002-01-0000_P-1" Name="Third object" ParameterType="T" Value="0"><Memory CodeSegment="PS" Offset="15" BitOffset="0" /></Parameter></Parameters>
<ParameterRefs><ParameterRef Id="M-0001_A-0002-01-0000_P-1_R-1" RefId="M-0001_A-0002-01-0000_P-1" /></ParameterRefs>
<ComObjectTable CodeSegment="PS" Offset="0">
<ComObject Id="M-0001_A-0002-01-0000_O-0" Name="Zero" Number="0" ObjectSize="1 Bit" ReadFlag="Disabled" WriteFlag="Enabled" CommunicationFlag="Enabled" TransmitFlag="Enabled" UpdateFlag="Disabled" ReadOnInitFlag="Disabled" />
<ComObject Id="M-0001_A-0002-01-0000_O-1" Name="One" Number="1" ObjectSize="1 Bit" ReadFlag="Disabled" WriteFlag="Enabled" CommunicationFlag="Enabled" TransmitFlag="Enabled" UpdateFlag="Disabled" ReadOnInitFlag="Disabled" />
<ComObject Id="M-0001_A-0002-01-0000_O-2" Name="Two" Number="2" ObjectSize="1 Bit" ReadFlag="Disabled" WriteFlag="Enabled" CommunicationFlag="Enabled" TransmitFlag="Enabled" UpdateFlag="Disabled" ReadOnInitFlag="Disabled" />
</ComObjectTable>
<ComObjectRefs><ComObjectRef Id="M-0001_A-0002-01-0000_O-0_R-1" RefId="M-0001_A-0002-01-0000_O-0" />
<ComObjectRef Id="M-0001_A-0002-01-0000_O-1_R-2" RefId="M-0001_A-0002-01-0000_O-1" />
<ComObjectRef Id="M-0001_A-0002-01-0000_O-2_R-3" RefId="M-0001_A-0002-01-0000_O-2" /></ComObjectRefs>
<AddressTable CodeSegment="AT" Offset="0" MaxEntries="4" />
<AssociationTable CodeSegment="AS" Offset="0" MaxEntries="4" />
<LoadProcedures><LoadProcedure><LdCtrlConnect /><LdCtrlUnload LsmIdx="1" /><LdCtrlUnload LsmIdx="2" /><LdCtrlUnload LsmIdx="3" />
<LdCtrlLoad LsmIdx="1" /><LdCtrlAbsSegment LsmIdx="1" SegType="0" Address="16384" Size="11" Access="255" MemType="3" SegFlags="128" /><LdCtrlLoadCompleted LsmIdx="1" />
<LdCtrlLoad LsmIdx="2" /><LdCtrlAbsSegment LsmIdx="2" SegType="0" Address="16640" Size="9" Access="255" MemType="3" SegFlags="128" /><LdCtrlLoadCompleted LsmIdx="2" />
<LdCtrlLoad LsmIdx="3" /><LdCtrlAbsSegment LsmIdx="3" SegType="0" Address="17408" Size="16" Access="255" MemType="3" SegFlags="128" />
<LdCtrlTaskSegment LsmIdx="3" Address="17408" /><LdCtrlLoadCompleted LsmIdx="3" /><LdCtrlDisconnect /></LoadProcedure></LoadProcedures>
</Static><Dynamic><ChannelIndependentBlock><ParameterBlock Id="PB" Text="B">
<ParameterRefRef RefId="M-0001_A-0002-01-0000_P-1_R-1" />
<ComObjectRefRef RefId="M-0001_A-0002-01-0000_O-0_R-1" /><ComObjectRefRef RefId="M-0001_A-0002-01-0000_O-1_R-2" />
<choose ParamRefId="M-0001_A-0002-01-0000_P-1_R-1"><when test="1"><ComObjectRefRef RefId="M-0001_A-0002-01-0000_O-2_R-3" /></when></choose>
</ParameterBlock></ChannelIndependentBlock></Dynamic>
</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

/// Its hardware, product `P` and `Hardware2Program` `HP`.
pub const LINK_ADOPTION_HARDWARE_XML: &str = r#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0001"><Hardware><Hardware Id="H" Name="Synthetic" SerialNumber="synthetic" VersionNumber="1"><Products><Product Id="P" /></Products><Hardware2Programs><Hardware2Program Id="HP" MediumTypes="MT-0"><ApplicationProgramRef RefId="M-0001_A-0002-01-0000" /></Hardware2Program></Hardware2Programs></Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;
