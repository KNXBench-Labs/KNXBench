//! Option C for `1.1.67`, downloaded end to end into the simulator.
//!
//! The whole offline path, in one place: the stored MDT product file, the
//! option-C values and link, the segment images, the product's own load
//! procedure turned into a plan, and that plan run against a simulated
//! mask-`0701h` device. Nothing here touches a bus; the simulator is the
//! only transport. `#[ignore]`d: it needs the private product corpus
//! (`KNXBENCH_PRODUCT_CORPUS`).

use std::time::Duration;

use knx_core::commissioning::authorisation::AuthorisationPlan;
use knx_core::commissioning::load_control_memory::MemoryLoadStateMachine;
use knx_core::commissioning::load_state::LoadState;
use knx_core::commissioning::mutation::{WriteAuthorisation, WriteScope};
use knx_core::commissioning::properties::{ObjectIndex, PID_HARDWARE_TYPE, PID_MANUFACTURER_ID};
use knx_core::{GroupAddress, IndividualAddress};
use knx_net::commissioning::memory_download::{
    run_memory_download, MemoryDownloadReport, RestartOutcome,
};
use knx_net::commissioning::simulator::{Seen, SimulatedDevice, SimulatorConfig};
use knx_net::commissioning::{ManagementSession, SessionTiming};
use knx_productdb::download_plan::plan_memory_download;
use knx_productdb::image::{build_download_image, DownloadImage, ImageRequest, Link};
use knx_productdb::{install_package, open_and_migrate};

const FILE: &str = "MDT_KP_BE_01_Push_Button_V15a.knxprod";
const PROGRAM: &str = "M-0083_A-0027-15-0BAC";
/// The device's individual address as it sits in `4001h`–`4002h`.
const INDIVIDUAL_ADDRESS: [u8; 2] = [0x11, 0x43];

fn option_c() -> DownloadImage {
    let root = std::env::var_os("KNXBENCH_PRODUCT_CORPUS")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../OriginalData/ProductDatabases")
        });
    let path = knx_testsupport::find_corpus_file(&root, FILE)
        .unwrap_or_else(|| panic!("{FILE} not under {}", root.display()));
    let bytes = std::fs::read(path).expect("readable");
    let dir = tempfile::tempdir().expect("tempdir");
    let conn = open_and_migrate(&dir.path().join("products.sqlite")).expect("database");
    install_package(&conn, FILE, &bytes).expect("installs");
    let request = ImageRequest {
        program_id: PROGRAM.to_string(),
        individual_address: IndividualAddress::new(1, 1, 67).expect("valid"),
        values: [
            ("P-1007_R-1007", "2"),
            ("UP-5500_R-5500", "0"),
            ("UP-5501_R-5501", "1"),
        ]
        .into_iter()
        .map(|(short, value)| (format!("{PROGRAM}_{short}"), value.to_string()))
        .collect(),
        links: vec![Link {
            object: 0,
            group_address: GroupAddress::from_raw(0x1035), // 2/0/53
            sending: true,
        }],
        flag_overrides: Default::default(),
    };
    build_download_image(&conn, &request).expect("builds")
}

fn fast() -> SessionTiming {
    SessionTiming {
        connection_timeout: Duration::from_millis(50),
        response_timeout: Duration::from_millis(50),
        poll_interval: Duration::from_millis(1),
        max_transition: Duration::from_millis(40),
        programming_delay: Duration::from_millis(0),
        restart_basic_t1: Duration::from_millis(1),
        restart_responsive_again: Duration::from_millis(5),
        post_restart_disconnect_wait: Duration::from_millis(5),
        programming_mode_broadcast_timeout: Duration::from_millis(20),
    }
}

/// A simulated `1.1.67`: mask `0701h`, MDT, hardware type `0127h`, its
/// individual address in memory, and the shutter configuration's first
/// octets still in the tables.
fn mdt_device() -> SimulatedDevice {
    mdt_device_with(SimulatorConfig::default())
}

fn mdt_device_with(config: SimulatorConfig) -> SimulatedDevice {
    let device = SimulatedDevice::with_config(SimulatorConfig {
        mask_version: 0x0701,
        ..config
    });
    device.preset_property(0, PID_MANUFACTURER_ID, &[0x00, 0x83]);
    device.preset_property(0, PID_HARDWARE_TYPE, &[0, 0, 0, 0, 0x01, 0x27]);
    device.preset_memory(0x4000, &[0x05]);
    device.preset_memory(0x4001, &INDIVIDUAL_ADDRESS);
    device.preset_memory(0x4003, &[0x04, 0x06, 0x04, 0x07]);
    for machine in [1, 2, 3] {
        device.preset_load_state(ObjectIndex::new(machine), LoadState::Loaded);
    }
    device
}

fn run(device: &SimulatedDevice, image: &DownloadImage) {
    let report = download(device, image);
    assert!(report.restart.is_confirmed(), "{}", report.restart);
}

fn download(device: &SimulatedDevice, image: &DownloadImage) -> MemoryDownloadReport {
    let plan = plan_memory_download(image).expect("plans");
    let authorisation = WriteAuthorisation::for_simulator(device.address(), WriteScope::Download)
        .expect("not excluded");
    let mut session =
        ManagementSession::authorised(device, AuthorisationPlan::Skip, fast(), authorisation)
            .expect("a simulator session");
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .expect("a runtime");
    let report = runtime
        .block_on(run_memory_download(&mut session, &plan))
        .expect("the download runs");
    assert_eq!(report.steps.len(), plan.steps.len());
    assert_eq!(report.data_octets, plan.data_octets());
    assert_eq!(
        report.final_states,
        vec![
            (MemoryLoadStateMachine::AddressTable, LoadState::Loaded),
            (MemoryLoadStateMachine::AssociationTable, LoadState::Loaded),
            (
                MemoryLoadStateMachine::ApplicationProgram,
                LoadState::Loaded
            ),
        ]
    );
    report
}

#[test]
#[ignore = "requires the private product corpus; set KNXBENCH_PRODUCT_CORPUS"]
fn option_c_lands_in_the_simulator_octet_for_octet_and_keeps_the_address() {
    let image = option_c();
    let device = mdt_device();
    run(&device, &image);

    for segment in &image.segments {
        let stored: Vec<u8> = device
            .memory(segment.address, segment.octets.len())
            .into_iter()
            .map(|octet| octet.unwrap_or_default())
            .collect();
        let mut expected = segment.octets.clone();
        if segment.address == 0x4000 {
            // Masked: the device's own address, never written.
            expected[1..3].copy_from_slice(&INDIVIDUAL_ADDRESS);
        }
        assert_eq!(stored, expected, "segment {}", segment.id);
    }
    // 2/0/53 is the one group address; the shutter's 0/4/6 is gone.
    assert_eq!(&device.memory(0x4000, 5)[3..], &[Some(0x10), Some(0x35)]);

    let writes: Vec<(u32, usize)> = device
        .seen()
        .into_iter()
        .filter_map(|seen| match seen {
            Seen::MemoryWrite { address, data, .. } => Some((address, data.len())),
            _ => None,
        })
        .collect();
    assert!(
        writes
            .iter()
            .all(|&(address, length)| address + length as u32 <= 0x4001 || address >= 0x4003),
        "nothing written to the individual address"
    );
    assert!(!device.verify_mode(), "MP §3.31.2: no Verify Mode");
    let restarts = device
        .seen()
        .into_iter()
        .filter(|seen| matches!(seen, Seen::Restart { .. }))
        .count();
    assert_eq!(restarts, 1);
}

#[test]
#[ignore = "requires the private product corpus; set KNXBENCH_PRODUCT_CORPUS"]
fn the_same_image_twice_leaves_the_same_memory() {
    let image = option_c();
    let device = mdt_device();
    run(&device, &image);
    let first: Vec<_> = image
        .segments
        .iter()
        .map(|segment| device.memory(segment.address, segment.octets.len()))
        .collect();
    run(&device, &image);
    let second: Vec<_> = image
        .segments
        .iter()
        .map(|segment| device.memory(segment.address, segment.octets.len()))
        .collect();
    assert_eq!(first, second);
}

/// `1.1.67`, run 3, in the simulator: every octet lands, every machine is
/// `Loaded`, and the closing Basic Restart goes unacknowledged. That is a
/// loaded download with an unconfirmed restart, not a failed one, and the
/// restart is sent as one request (TL's own repetitions of it aside),
/// never again on the executor's initiative.
#[test]
#[ignore = "requires the private product corpus; set KNXBENCH_PRODUCT_CORPUS"]
fn option_c_with_an_unacknowledged_restart_is_loaded_but_unconfirmed() {
    let image = option_c();
    let device = mdt_device_with(SimulatorConfig {
        restart_unanswered: true,
        ..SimulatorConfig::default()
    });

    let report = download(&device, &image);

    assert!(
        matches!(report.restart, RestartOutcome::Unconfirmed { .. }),
        "{}",
        report.restart
    );
    for segment in &image.segments {
        let stored: Vec<u8> = device
            .memory(segment.address, segment.octets.len())
            .into_iter()
            .map(|octet| octet.unwrap_or_default())
            .collect();
        let mut expected = segment.octets.clone();
        if segment.address == 0x4000 {
            expected[1..3].copy_from_slice(&INDIVIDUAL_ADDRESS);
        }
        assert_eq!(stored, expected, "segment {}", segment.id);
    }
    let seqs = device.unanswered_restart_seqs();
    assert!(!seqs.is_empty());
    assert!(
        seqs.iter().all(|seq| *seq == seqs[0]),
        "one restart request, repeated only by TL: {seqs:?}"
    );
}
