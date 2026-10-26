//! Synthetic System-2 product data and device memory for memory-profile diagnostics.
//!
//! The same shape as `knx-app`'s `device_diagnostics_memory` fixture: one PID 78
//! comparison and two declared absolute segments (30 + 12 octets). Shared by the
//! HTTP route tests and the loopback `diagnostic_simulator` example.
#![allow(dead_code)]

use knx_net::commissioning::simulator::SimulatedDevice;

pub const PROGRAM_ID: &str = "M-0001_A-0001-01-0000";
/// The project's hardware-to-program reference that resolves to [`PROGRAM_ID`].
pub const PROGRAM_REF: &str = "H_HP";
pub const PRODUCT_REF: &str = "M-0001_H-1_P-1";
pub const MASK: u16 = 0x0025;
pub const MANUFACTURER: [u8; 2] = [0x00, 0x01];
pub const HARDWARE_TYPE: [u8; 6] = [0, 1, 0, 0, 0, 0x5B];
/// `(address, length)` in source order.
pub const RANGES: [(u16, u16); 2] = [(0x0116, 30), (0x3C00, 12)];

pub const PROGRAM: &str = r#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0001">
<ApplicationPrograms><ApplicationProgram Id="M-0001_A-0001-01-0000" Name="Synthetic" ApplicationNumber="1"
ApplicationVersion="1" MaskVersion="MV-0025" LoadProcedureStyle="ProductProcedure"><Static>
<Code><AbsoluteSegment Id="OBJ" Address="1024" Size="8" /></Code>
<ComObjectTable CodeSegment="OBJ" Offset="0" />
<LoadProcedures><LoadProcedure><LdCtrlConnect />
<LdCtrlCompareProp ObjIdx="0" PropId="78" InlineData="00010000005B00000000" />
<LdCtrlAbsSegment LsmIdx="1" SegType="0" Address="278" Size="30" Access="51" MemType="3" SegFlags="0" />
<LdCtrlAbsSegment LsmIdx="3" SegType="0" Address="15360" Size="12" Access="51" MemType="3" SegFlags="0" />
<LdCtrlTaskCtrl2 LsmIdx="3" Callback="0" Address="1024" Seg0="0" Seg1="0" />
<LdCtrlTaskSegment LsmIdx="5" Address="1024" /><LdCtrlDisconnect /></LoadProcedure></LoadProcedures>
</Static></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

pub const HARDWARE: &str = r#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0001">
<Hardware><Hardware Id="H" Name="Synthetic" SerialNumber="synthetic" VersionNumber="1">
<Products><Product Id="M-0001_H-1_P-1" /></Products><Hardware2Programs><Hardware2Program Id="H_HP" MediumTypes="MT-0">
<ApplicationProgramRef RefId="M-0001_A-0001-01-0000" /></Hardware2Program></Hardware2Programs>
</Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;

/// A migrated product database at `path` holding `program` and the hardware mapping.
pub fn products_at(path: &std::path::Path, program: &str) -> knx_productdb::Connection {
    let db = knx_productdb::open_and_migrate(path).expect("synthetic product database");
    knx_productdb::ingest_file(&db, "synthetic.xml", program.as_bytes())
        .expect("synthetic program");
    knx_productdb::ingest_file(&db, "Hardware.xml", HARDWARE.as_bytes())
        .expect("synthetic hardware");
    db
}

/// The first declared range counts up from 1, the second is `EEh` throughout,
/// so a swapped or shifted chunk shows in its bytes.
pub fn preset_memory(device: &SimulatedDevice) {
    device.preset_memory(u32::from(RANGES[0].0), &(1..=30).collect::<Vec<u8>>());
    device.preset_memory(u32::from(RANGES[1].0), &[0xEE; 12]);
}

/// Identity values the synthetic program asserts.
pub fn preset_identity(device: &SimulatedDevice) {
    device.preset_property(0, 12, &MANUFACTURER);
    device.preset_property(0, 78, &HARDWARE_TYPE);
}
