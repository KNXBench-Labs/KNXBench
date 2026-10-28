//! Shared synthetic source/project builders; no device or protocol behavior.
use knx_core::IndividualAddress;
const PROFILE: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../data/system2-validation/up258d12-920504-v1.json"
));

// Entirely synthetic installed-package → Project fixture, with independent OLD state.
pub(crate) const SYN_ID: &str = "M-0000_A-0001-01-0000";
pub(crate) fn syn_ref(suffix: &str) -> String {
    format!("{SYN_ID}_{suffix}")
}
pub(crate) fn syn_source(id: &str) -> knx_core::ids::SourceRef {
    knx_core::ids::SourceRef {
        path: "synthetic/0.xml".into(),
        ets_id: id.into(),
    }
}
pub(crate) fn synthetic_package(namespace: u8) -> (Vec<u8>, Vec<u8>) {
    use std::io::{Cursor, Write};
    let application = r#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0000"><ApplicationPrograms>
<ApplicationProgram Id="M-0000_A-0001-01-0000" Name="Synthetic" ApplicationNumber="1" ApplicationVersion="1" PeiType="0" MaskVersion="MV-0025" LoadProcedureStyle="ProductProcedure"><Static>
<Options DownloadInvisibleParameters="Background" />
<Code>
<AbsoluteSegment Id="GA" Address="278" Size="16"><Data>AAAAAAAAAAAAAAAAAAAAAA==</Data></AbsoluteSegment>
<AbsoluteSegment Id="AA" Address="608" Size="16"><Data>AAAAAAAAAAAAAAAAAAAAAA==</Data></AbsoluteSegment>
<AbsoluteSegment Id="OBJ" Address="1024" Size="8"><Data>AqcSJQA0pQg=</Data></AbsoluteSegment>
<AbsoluteSegment Id="PAR" Address="15360" Size="8"><Data>AAAAAAAAAAA=</Data><Mask>//8A//////8=</Mask></AbsoluteSegment>
</Code>
<ParameterTypes><ParameterType Id="PT16" Name="Level"><TypeNumber SizeInBit="16" Type="unsignedInt" minInclusive="0" maxInclusive="65535" /></ParameterType><ParameterType Id="PTSEL" Name="Selector"><TypeNumber SizeInBit="8" Type="unsignedInt" minInclusive="0" maxInclusive="1" /></ParameterType></ParameterTypes>
<Parameters><Parameter Id="P" Name="Level" ParameterType="PT16" Value="4660"><Memory CodeSegment="PAR" Offset="0" BitOffset="0" /></Parameter><Parameter Id="S" Name="Selector" ParameterType="PTSEL" Value="1" /></Parameters>
<ParameterRefs><ParameterRef Id="R" RefId="P" /><ParameterRef Id="RS" RefId="S" /></ParameterRefs>
<ComObjectTable CodeSegment="OBJ" Offset="0"><ComObject Id="O" Number="1" ObjectSize="1 Byte" Priority="Low" ReadFlag="Enabled" WriteFlag="Disabled" CommunicationFlag="Enabled" TransmitFlag="Enabled" UpdateFlag="Disabled" ReadOnInitFlag="Disabled" /></ComObjectTable>
<ComObjectRefs><ComObjectRef Id="OR1" RefId="O" /><ComObjectRef Id="OR2" RefId="O" /></ComObjectRefs>
<AddressTable CodeSegment="GA" Offset="0" MaxEntries="6" /><AssociationTable CodeSegment="AA" Offset="0" MaxEntries="7" />
<LoadProcedures><LoadProcedure>
<LdCtrlConnect /><LdCtrlCompareProp ObjIdx="0" PropId="78" InlineData="F00000000001" />
<LdCtrlUnload LsmIdx="1" /><LdCtrlUnload LsmIdx="2" /><LdCtrlUnload LsmIdx="3" />
<LdCtrlLoad LsmIdx="1" /><LdCtrlAbsSegment LsmIdx="1" SegType="0" Address="278" Size="16" Access="51" MemType="3" SegFlags="0" /><LdCtrlTaskSegment LsmIdx="1" Address="278" />
<LdCtrlLoad LsmIdx="2" /><LdCtrlAbsSegment LsmIdx="2" SegType="0" Address="608" Size="16" Access="51" MemType="3" SegFlags="0" /><LdCtrlTaskSegment LsmIdx="2" Address="608" />
<LdCtrlLoad LsmIdx="3" /><LdCtrlAbsSegment LsmIdx="3" SegType="0" Address="1024" Size="8" Access="51" MemType="3" SegFlags="0" /><LdCtrlAbsSegment LsmIdx="3" SegType="0" Address="15360" Size="8" Access="51" MemType="3" SegFlags="0" /><LdCtrlTaskSegment LsmIdx="3" Address="0" /><LdCtrlTaskCtrl2 LsmIdx="3" Callback="0" Address="1024" Seg0="0" Seg1="0" /><LdCtrlTaskSegment LsmIdx="5" Address="1024" />
<LdCtrlLoadCompleted LsmIdx="1" /><LdCtrlLoadCompleted LsmIdx="2" /><LdCtrlLoadCompleted LsmIdx="3" /><LdCtrlDisconnect />
</LoadProcedure></LoadProcedures>
</Static><Dynamic><Channel Id="CH"><ParameterBlock Id="PB"><ParameterRefRef RefId="R" /><ParameterRefRef RefId="RS" /><choose ParamRefId="RS"><when test="1"><ComObjectRefRef RefId="OR1" /></when><when test="0"><ComObjectRefRef RefId="OR2" /></when></choose></ParameterBlock></Channel></Dynamic>
</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;
    let mut application = application.replace("project/20", &format!("project/{namespace}"));
    for id in [
        "PT16", "PTSEL", "OR1", "OR2", "RS", "CH", "PB", "P", "S", "R", "O",
    ] {
        application = application.replace(&format!("=\"{id}\""), &format!("=\"{}\"", syn_ref(id)));
    }
    let master = format!(
        r#"<KNX xmlns="http://knx.org/xml/project/{namespace}"><MasterData><Manufacturers><Manufacturer Id="M-0000" Name="Synthetic" /></Manufacturers></MasterData></KNX>"#
    );
    let hardware = format!(
        r#"<KNX xmlns="http://knx.org/xml/project/{namespace}"><ManufacturerData><Manufacturer RefId="M-0000"><Hardware><Hardware Id="M-0000_H-1" Name="Synthetic"><Products><Product Id="M-0000_H-1_P-1" Text="Synthetic" /></Products><Hardware2Programs><Hardware2Program Id="M-0000_H-1_HP-1" MediumTypes="MT-0"><ApplicationProgramRef RefId="{SYN_ID}" /></Hardware2Program></Hardware2Programs></Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#
    );
    let catalog = format!(
        r#"<KNX xmlns="http://knx.org/xml/project/{namespace}"><ManufacturerData><Manufacturer RefId="M-0000"><Catalog><CatalogSection Id="M-0000_CS-1" Name="Synthetic"><CatalogItem Id="M-0000_CI-1" Name="Synthetic" ProductRefId="M-0000_H-1_P-1" Hardware2ProgramRefId="M-0000_H-1_HP-1" /></CatalogSection></Catalog></Manufacturer></ManufacturerData></KNX>"#
    );
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (path, bytes) in [
        ("knx_master.xml", master.as_bytes()),
        ("M-0000/Hardware.xml", hardware.as_bytes()),
        ("M-0000/Application.xml", application.as_bytes()),
        ("M-0000/Catalog.xml", catalog.as_bytes()),
    ] {
        writer
            .start_file(path, zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(bytes).unwrap();
    }
    (
        writer.finish().unwrap().into_inner(),
        application.into_bytes(),
    )
}
pub(crate) fn synthetic_profile(namespace: u8, package: &[u8], source: &[u8]) -> Vec<u8> {
    let mut profile: serde_json::Value = serde_json::from_slice(PROFILE).unwrap();
    profile["id"] = "synthetic-project-v2".into();
    profile["format"] = 2.into();
    profile["version"] = 2.into();
    profile["configuration"] = "project-parameters-links-flags-v1".into();
    profile["program_id"] = SYN_ID.into();
    profile["schema_namespace"] = format!("http://knx.org/xml/project/{namespace}").into();
    profile["source_sha256"] = knx_productdb::sha256_hex(source).into();
    profile.as_object_mut().unwrap().remove("package_sha256");
    profile["product_input"] = serde_json::json!({"kind":"standalone-package-v1", "package_sha256":knx_productdb::sha256_hex(package)});
    profile["manufacturer"] = 0.into();
    profile["hardware"] = serde_json::json!([240, 0, 0, 0, 0, 1]);
    profile["application"] = serde_json::json!([0, 0, 0, 1, 1]);
    profile["invisible_parameters"] =
        "source-template-inactive-legacy-patch-active-overlay-v1".into();
    profile["procedure_policy"] = "source-capacity-preserving-independent-data-v1".into();
    serde_json::to_vec(&profile).unwrap()
}
pub(crate) fn synthetic_project() -> knx_core::Project {
    use knx_core::{
        device::{ComObjectInstance, DeviceInstance},
        flags::{Direction, GroupLink, ResolvedFlags},
        group::GroupAddressEntry,
        ids::*,
        installation::Installation,
        parameter::ParameterInstance,
        provenance::{Layer, Override, Resolved},
        topology::Topology,
        GroupAddress,
    };
    let mut project = knx_core::Project::new(knx_core::string_table::Language("en".into()));
    let device = project.ids.next_device_id().unwrap();
    let mut objects = Vec::new();
    for suffix in ["OR1", "OR2"] {
        let id = project.ids.next_com_object_instance_id().unwrap();
        objects.push(id);
        let mut flags = ResolvedFlags::none();
        if suffix == "OR1" {
            flags.read = Override::Value(Resolved {
                value: false,
                layer: Layer::Instance,
            });
            flags.write = Override::Value(Resolved {
                value: true,
                layer: Layer::UserEdit,
            });
            flags.update = Override::Value(Resolved {
                value: true,
                layer: Layer::Instance,
            });
        }
        project.devices.insert_com_object(ComObjectInstance {
            id,
            source: syn_source(&syn_ref(suffix)),
            device,
            number: 1,
            text: Override::Absent,
            description: Override::Absent,
            dpt: Override::Absent,
            flags,
            size: None,
            is_active: suffix == "OR1",
            links: if suffix == "OR1" {
                vec![
                    GroupLink {
                        ga: GroupAddressId(1),
                        direction: Direction::Send,
                    },
                    GroupLink {
                        ga: GroupAddressId(2),
                        direction: Direction::Receive,
                    },
                ]
            } else {
                vec![]
            },
            module_instance: None,
        });
    }
    project.devices.insert(DeviceInstance {
        id: device,
        source: syn_source("KB-DEV-1"),
        name: "Selected synthetic catalogue product".into(),
        description: None,
        address: Some(IndividualAddress::from_raw(0x1234)),
        product_ref: "M-0000_H-1_P-1".into(),
        program_ref: "M-0000_H-1_HP-1".into(),
        commissioning: Default::default(),
        visibility_calculated: true,
        com_objects: objects,
        binary_data: vec![],
    });
    let parameters = [("R", "258"), ("RS", "1")]
        .into_iter()
        .map(|(id, raw)| ParameterInstance {
            id: project.ids.next_parameter_instance_id().unwrap(),
            device,
            source: syn_source(&syn_ref(id)),
            raw: raw.into(),
        })
        .collect();
    let group_addresses = [0x1035, 0x1036]
        .into_iter()
        .map(|raw| {
            let id = project.ids.next_group_address_id().unwrap();
            GroupAddressEntry {
                id,
                source: syn_source(&format!("GA-{}", id.0)),
                name: "Synthetic group".into(),
                address: GroupAddress::from_raw(raw),
                central: false,
                unfiltered: false,
                range: None,
                declared_dpt: Override::Absent,
            }
        })
        .collect();
    project.installations.push(Installation {
        id: InstallationId(0),
        name: "Fresh installation".into(),
        default_line: None,
        multicast_address: None,
        completion: Default::default(),
        topology: Topology {
            areas: vec![],
            lines: vec![],
            unassigned: vec![device],
        },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses,
        parameters,
    });
    project
}
pub(crate) const PROGRAM: &str = "M-0001_A-9205-04-9270";
pub(crate) fn configured_inputs() -> (
    tempfile::TempDir,
    knx_productdb::Connection,
    knx_core::Project,
    Vec<u8>,
) {
    use std::io::{Cursor, Read, Write};
    let (old_package, _) = synthetic_package(20);
    let mut zip = zip::ZipArchive::new(Cursor::new(old_package)).unwrap();
    let mut out = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let convert = |s: &str| {
        s.replace(SYN_ID, PROGRAM)
            .replace("M-0000", "M-0001")
            .replace("ApplicationNumber=\"1\"", "ApplicationNumber=\"37381\"")
            .replace("ApplicationVersion=\"1\"", "ApplicationVersion=\"4\"")
            .replace("F00000000001", "00010000005B")
    };
    let mut source = Vec::new();
    for i in 0..zip.len() {
        let mut member = zip.by_index(i).unwrap();
        let name = convert(member.name());
        let mut text = String::new();
        member.read_to_string(&mut text).unwrap();
        let bytes = convert(&text).into_bytes();
        if name.ends_with("Application.xml") {
            source = bytes.clone();
        }
        out.start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        out.write_all(&bytes).unwrap();
    }
    let package = out.finish().unwrap().into_inner();
    let mut profile: serde_json::Value =
        serde_json::from_slice(&synthetic_profile(20, &package, &source)).unwrap();
    profile["program_id"] = PROGRAM.into();
    profile["manufacturer"] = 1.into();
    profile["hardware"] = serde_json::json!([0, 1, 0, 0, 0, 91]);
    profile["application"] = serde_json::json!([0, 1, 146, 5, 4]);
    let mut project = synthetic_project();
    let device = project.devices.get_mut(knx_core::DeviceId(1)).unwrap();
    device.product_ref = convert(&device.product_ref);
    device.program_ref = convert(&device.program_ref);
    let objects = device.com_objects.clone();
    for id in objects {
        let object = project.devices.com_object_mut(id).unwrap();
        object.source.ets_id = convert(&object.source.ets_id);
    }
    for parameter in &mut project.installations[0].parameters {
        parameter.source.ets_id = convert(&parameter.source.ets_id);
    }
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    knx_productdb::install_package(&conn, "synthetic-exact-identity.knxprod", &package).unwrap();
    (dir, conn, project, serde_json::to_vec(&profile).unwrap())
}
