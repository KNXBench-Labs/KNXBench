//! Proves Read-on-Init survives product enrichment, persistence, reload and export.
//!
//! §117's acceptance test: `knx-productdb` has always parsed
//! `ReadOnInitFlag`, and until this cycle `knx-core` had nowhere to put it,
//! so the value died at the crate boundary. Everything here is hand-built —
//! no `OriginalData/` corpus, so it runs everywhere.

use knx_core::{
    Area, AreaId, ComFlagKind, ComObjectInstance, ComObjectInstanceId, Command, CommandStack,
    CommissioningState, CompletionStatus, DeviceInstance, GroupAddressStyle, Installation,
    InstallationId, Language, Layer, Override, Project, ResolvedFlags, SourceRef, Topology,
};

const HARDWARE: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-006A">
<Hardware><Hardware Id="H-1" Name="X" SerialNumber="S" VersionNumber="1">
<Hardware2Programs><Hardware2Program Id="H-1_HP-1" MediumTypes="MT-0">
<ApplicationProgramRef RefId="A-1" /></Hardware2Program></Hardware2Programs>
</Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;

/// One communication object whose program-layer `ReadOnInitFlag` is
/// `"Enabled"` — the shape shipped packages actually use (measured: the
/// attribute appears on `ComObject`, 2533 times across the local corpus,
/// and on no other element).
const PROGRAM: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-006A">
<ApplicationPrograms><ApplicationProgram Id="A-1" Name="P" ApplicationVersion="22"
  MaskVersion="MV-0701"><Static>
<ComObjectTable>
  <ComObject Id="A-1_O-1" Number="1" Text="Schalten" ObjectSize="1 Bit"
             DatapointType="DPST-1-1" WriteFlag="Enabled" ReadOnInitFlag="Enabled" />
</ComObjectTable>
<ComObjectRefs>
  <ComObjectRef Id="A-1_O-1_R-1" RefId="A-1_O-1" />
</ComObjectRefs>
</Static></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

fn source(ets_id: &str) -> SourceRef {
    SourceRef {
        path: "P-0001/0.xml".into(),
        ets_id: ets_id.into(),
    }
}

/// One unassigned device carrying one communication object with no
/// instance-level overrides at all — every flag `Absent`, waiting for the
/// product database to speak.
fn project() -> Project {
    let mut p = Project::new(Language("de-DE".into()));
    p.info.project_id = "P-0001".into();
    p.info.group_address_style = GroupAddressStyle::ThreeLevel;

    let device_id = p.ids.next_device_id();
    let com_id = p.ids.next_com_object_instance_id();
    p.devices.insert(DeviceInstance {
        id: device_id,
        source: source("P-0001-0_DI-1"),
        name: "D".into(),
        description: None,
        address: None,
        product_ref: "H-1_P-1".into(),
        program_ref: "H-1_HP-1".into(),
        commissioning: CommissioningState::default(),
        visibility_calculated: false,
        com_objects: vec![com_id],
        binary_data: vec![],
    });
    p.devices.insert_com_object(ComObjectInstance {
        id: com_id,
        source: source("A-1_O-1_R-1"),
        device: device_id,
        number: 1,
        text: Override::Absent,
        description: Override::Absent,
        dpt: Override::Absent,
        flags: ResolvedFlags::none(),
        size: None,
        is_active: true,
        links: vec![],
        module_instance: None,
    });

    p.installations.push(Installation {
        id: InstallationId(0),
        name: String::new(),
        default_line: None,
        multicast_address: None,
        completion: CompletionStatus::default(),
        topology: Topology {
            areas: vec![Area {
                id: AreaId(1),
                source: source("P-0001-0_A-1"),
                name: "A".into(),
                address: 1,
                completion: CompletionStatus::default(),
                lines: vec![],
            }],
            lines: vec![],
            unassigned: vec![device_id],
        },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses: vec![],
        parameters: vec![],
    });
    p
}

fn products(dir: &std::path::Path) -> knx_productdb::Connection {
    let conn = knx_productdb::open_and_migrate(&dir.join("products.sqlite")).unwrap();
    knx_productdb::parse::hardware::ingest_hardware(
        &conn,
        "sha-h",
        "M-006A/Hardware.xml",
        HARDWARE.as_bytes(),
    )
    .unwrap();
    knx_productdb::parse::program::ingest_program(
        &conn,
        "sha-p",
        "M-006A/A.xml",
        PROGRAM.as_bytes(),
    )
    .unwrap();
    conn
}

#[test]
fn a_products_read_on_init_flag_survives_enrichment_persistence_and_reload() {
    let dir = tempfile::tempdir().unwrap();
    let store = knx_store::open_and_migrate(&dir.path().join("p.knxdb")).unwrap();
    let products = products(dir.path());

    let mut p = project();
    let report = knx_productdb::enrich(&mut p, &products).unwrap();
    assert_eq!(report.com_objects_enriched, 1);

    let com = p.devices.com_object(ComObjectInstanceId(1)).unwrap();
    let resolved = com
        .flags
        .read_on_init
        .value()
        .expect("the product's Read-on-Init flag reached the domain model");
    assert!(resolved.value);
    assert_eq!(resolved.layer, Layer::Program);

    knx_store::save_project(&store, &p).unwrap();
    let reloaded = knx_store::load_project(&store).unwrap();
    let com = reloaded.devices.com_object(ComObjectInstanceId(1)).unwrap();
    let resolved = com
        .flags
        .read_on_init
        .value()
        .expect("the flag survived the store round-trip");
    assert!(resolved.value);
    assert_eq!(resolved.layer, Layer::Program);
}

#[test]
fn a_user_set_read_on_init_flag_is_reported_rather_than_dropped_on_export() {
    let mut p = project();
    let mut stack = CommandStack::new();
    stack
        .do_command(
            &mut p,
            Command::SetComObjectFlag {
                com_object: ComObjectInstanceId(1),
                flag: ComFlagKind::ReadOnInit,
                value: true,
            },
        )
        .unwrap();

    // No measured `ComObjectInstanceRef` carries `ReadOnInitFlag`, so the
    // writer does not invent one. What it must not do is stay quiet about
    // it — that was §117's original sin.
    let xml =
        String::from_utf8(knx_etsproj::export::write_installation_xml(&p, &[]).unwrap()).unwrap();
    assert!(!xml.contains("ReadOnInitFlag"), "{xml}");

    let outcome = knx_etsproj::export::export_knxproj(&p, &[]).unwrap();
    assert!(
        outcome
            .warnings
            .contains(&knx_etsproj::export::ExportWarning::ReadOnInitNotExported {
                com_objects: 1
            }),
        "export dropped the user's Read-on-Init edit without a word: {:?}",
        outcome.warnings
    );
}

#[test]
fn a_program_layer_read_on_init_flag_raises_no_export_warning() {
    // Only `Layer::Instance`/`Layer::UserEdit` values are the project's own
    // to export, so a value that belongs to the product database is not a
    // loss and must not be reported as one.
    let dir = tempfile::tempdir().unwrap();
    let products = products(dir.path());
    let mut p = project();
    knx_productdb::enrich(&mut p, &products).unwrap();

    let outcome = knx_etsproj::export::export_knxproj(&p, &[]).unwrap();
    assert!(!outcome.warnings.iter().any(|w| matches!(
        w,
        knx_etsproj::export::ExportWarning::ReadOnInitNotExported { .. }
    )));
}
