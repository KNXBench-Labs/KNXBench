//! Verify captured generator binding through real regular CLI writes and restored executions.
use super::*;
use knx_app::device_backup::{prepare_restore, read_backup, StoredBackup, WriteSelection};
use knx_core::commissioning::memory_download::{MemoryDownloadPlan, MemoryDownloadStep};
use knx_core::commissioning::partial_memory_download::PartialDownloadParts;

const NEW: WriteSelection = WriteSelection::SourceDefaultInactiveWholeBytesV1;
const LEGACY: WriteSelection = WriteSelection::SourceMaskOnlyV1;
const OLD: [u8; 6] = [1, 0xD4, 0xD5, 0xD6, 0xD7, 0xD8];

fn products(dir: &std::path::Path) -> knx_productdb::Connection {
    let conn = knx_productdb::open_and_migrate(&dir.join("products.sqlite")).unwrap();
    knx_productdb::ingest_file(&conn, "M-FFF6/A.xml", source(1).as_bytes()).unwrap();
    knx_productdb::ingest_file(&conn, "M-FFF6/Hardware.xml", HARDWARE_XML.as_bytes()).unwrap();
    conn
}

fn device() -> SimulatedDevice {
    let device = SimulatedDevice::with_config_at(
        TARGET.parse().unwrap(),
        SimulatorConfig {
            mask_version: 0x0701,
            serial_number: Some(SERIAL),
            ..SimulatorConfig::default()
        },
    );
    device.preset_property(0, PID_MANUFACTURER_ID, &[0xFF, 0xF6]);
    device.preset_property(3, PID_PROGRAM_VERSION, &APPLICATION);
    for machine in MemoryLoadStateMachine::ALL {
        device.preset_load_state(ObjectIndex::new(machine.type_number()), LoadState::Loaded);
    }
    device.preset_memory(0x4000, &[0xB4; 8]);
    device.preset_memory(0x4400, &OLD);
    device
}

fn execute(
    device: &SimulatedDevice,
    plan: &MemoryDownloadPlan,
    selection: WriteSelection,
    partial: Option<PartialDownloadParts>,
    dir: &std::path::Path,
) -> (StoredBackup, std::path::PathBuf) {
    let log = Arc::new(OneShotLog::for_run(dir.join("history.sqlite")).unwrap());
    let mut activity = log.start_download(1, device.address()).unwrap();
    let backups = dir.join("backups");
    let backup = BackupTarget {
        write_selection: selection,
        dir: &backups,
        application: PROGRAM_ID,
        partial,
        taken: "2026-10-30T06:00:00Z".into(),
    };
    let mut output = Vec::new();
    let result = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap()
        .block_on(execute_regular_download(
            device,
            WriteAuthorisation::for_simulator(device.address(), WriteScope::Download).unwrap(),
            download_keying(plan.mask, None, KeySource::None),
            fast(),
            DownloadRun {
                plan,
                backup: &backup,
            },
            &mut activity,
            &mut output,
        ));
    assert!(
        matches!(result.result, DownloadResult::Finished { .. }),
        "{}",
        String::from_utf8_lossy(&output)
    );
    assert!(result.recorded && result.check_recorded);
    let files: Vec<_> = std::fs::read_dir(&backups)
        .unwrap()
        .map(|f| f.unwrap().path())
        .collect();
    assert_eq!(files.len(), 1);
    // Retain the actual closed keeper file for private execution evidence, not a synthesized fixture.
    static CAPTURE: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let retained =
        std::path::PathBuf::from(std::env::var_os("HOME").unwrap()).join("inactive-captures");
    std::fs::create_dir_all(&retained).unwrap();
    let index = CAPTURE.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    std::fs::copy(
        &files[0],
        retained.join(format!("{index}-{}.backup.json", selection.as_str())),
    )
    .unwrap();
    (read_backup(&files[0]).unwrap(), files[0].clone())
}

fn assert_restore_guards(conn: &knx_productdb::Connection, stored: &StoredBackup) {
    use knx_app::device_backup::RestorePrepareError as Error;
    use knx_core::commissioning::device_backup::{
        ExemplarEvidence, PriorApplication, RestoreError,
    };
    use knx_core::commissioning::memory_download::ExemplarSerial;
    let mut wrong = stored.clone();
    wrong.write_selection = if stored.write_selection == NEW {
        LEGACY
    } else {
        NEW
    };
    assert!(
        matches!(
            prepare_restore(conn, &wrong),
            Err(Error::DifferentProcedure)
        ),
        "no try-both or shape fallback"
    );
    let mut wrong = stored.clone();
    wrong.backup.regions[0].address += 1;
    assert!(matches!(
        prepare_restore(conn, &wrong),
        Err(Error::Shape(RestoreError::OtherShape { .. }))
    ));
    let mut wrong = stored.clone();
    wrong.backup.regions.swap(0, 1);
    assert!(matches!(
        prepare_restore(conn, &wrong),
        Err(Error::Shape(RestoreError::OtherShape { .. }))
    ));
    let mut wrong = stored.clone();
    wrong.backup.prior_application = PriorApplication::Recorded([0xFF, 0xF6, 0, 1, 2]);
    assert!(matches!(
        prepare_restore(conn, &wrong),
        Err(Error::Shape(RestoreError::OtherApplication { .. }))
    ));
    let mut wrong = stored.clone();
    wrong.backup.exemplar_serial = ExemplarEvidence::NotRecorded;
    assert!(matches!(
        prepare_restore(conn, &wrong),
        Err(Error::Shape(RestoreError::ExemplarNotRecorded))
    ));
    let mut wrong = stored.clone();
    wrong.backup.exemplar_serial = ExemplarEvidence::Recorded(ExemplarSerial::new([0; 6]));
    assert!(matches!(
        prepare_restore(conn, &wrong),
        Err(Error::Shape(RestoreError::ExemplarNotBindable))
    ));
    let mut wrong = stored.clone();
    wrong.backup.load_states[0].1 = LoadState::Unloaded;
    assert!(matches!(
        prepare_restore(conn, &wrong),
        Err(Error::NotLoaded)
    ));
    let mut wrong = stored.clone();
    wrong.backup.manufacturer ^= 1;
    assert!(matches!(
        prepare_restore(conn, &wrong),
        Err(Error::Shape(RestoreError::OtherProduct))
    ));
}

fn app_writes(plan: &MemoryDownloadPlan) -> Vec<(u16, Vec<u8>)> {
    plan.steps
        .iter()
        .filter_map(|s| match s {
            MemoryDownloadStep::WriteMemory { address, octets } if *address >= 0x4400 => {
                Some((*address, octets.clone()))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn new_capture_requires_format_four_and_explicit_generator() {
    for scope in [
        ScopeRequest::Complete,
        ScopeRequest::Partial(PartialDownloadParts {
            parameters: true,
            group_addresses: false,
        }),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let conn = products(dir.path());
        let mut project = project();
        for (index, (reference, raw)) in [("U1", "89"), ("Tail", "106"), ("I", "238")]
            .into_iter()
            .enumerate()
        {
            project.installations[0]
                .parameters
                .push(knx_core::ParameterInstance {
                    id: knx_core::ParameterInstanceId(index as u32 + 1),
                    device: DeviceId(1),
                    source: SourceRef {
                        path: "synthetic".into(),
                        ets_id: format!("{PROGRAM_ID}_P-{reference}_R-{reference}"),
                    },
                    raw: raw.into(),
                });
        }
        let prepared = prepare_scoped_download(&conn, &project, TARGET.parse().unwrap(), scope)
            .unwrap()
            .prepared
            .unwrap();
        assert_eq!(prepared.write_selection, NEW);
        assert_eq!(
            prepared
                .octets_to_write()
                .iter()
                .find(|(_, address, _, _)| *address == 0x4400)
                .unwrap()
                .3,
            4,
            "counts follow selected writes, not the source image"
        );
        let partial = prepared.partial.as_ref().map(|(p, _)| *p);
        let device = device();
        let (stored, path) = execute(
            &device,
            &prepared.plan,
            prepared.write_selection,
            partial,
            dir.path(),
        );
        let bytes = std::fs::read(&path).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            value["format"], 4,
            "actual writer capture must carry the explicit versioned generator"
        );
        assert_eq!(
            value["write_selection"],
            "source-default-inactive-whole-bytes-v1"
        );
        assert_eq!(stored.write_selection, NEW);
        assert_restore_guards(&conn, &stored);
        assert_eq!(
            application_writes(&device),
            vec![
                (0x4400, vec![1]),
                (0x4402, vec![0xC7, 0x59]),
                (0x4405, vec![0x6A])
            ]
        );
        assert!(stored.backup.was_loaded());
        let restored = prepare_restore(&conn, &read_backup(&path).unwrap()).unwrap();
        assert_eq!(
            app_writes(&restored),
            vec![
                (0x4400, vec![1]),
                (0x4402, vec![0xD5, 0xD6]),
                (0x4405, vec![0xD8])
            ]
        );
        let before = device.seen().len();
        let next = tempfile::tempdir().unwrap();
        let (recaptured, next_path) = execute(
            &device,
            &restored,
            stored.write_selection,
            stored.partial,
            next.path(),
        );
        let frames: Vec<_> = device
            .seen()
            .into_iter()
            .skip(before)
            .filter_map(|seen| match seen {
                Seen::MemoryWrite { address, data, .. } if address >= 0x4400 => {
                    Some((address, data))
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            frames,
            vec![
                (0x4400, vec![1]),
                (0x4402, vec![0xD5, 0xD6]),
                (0x4405, vec![0xD8])
            ]
        );
        assert_eq!(recaptured.write_selection, NEW);
        assert_eq!(
            app_writes(&prepare_restore(&conn, &read_backup(&next_path).unwrap()).unwrap()),
            vec![
                (0x4400, vec![1]),
                (0x4402, vec![0xC7, 0x59]),
                (0x4405, vec![0x6A])
            ]
        );
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }
}

#[test]
fn fallback_capture_binds_the_historical_generator_and_restores() {
    // Mode 0 against a source default of 1 changes the topology: the proof falls back to the
    // regular plan, so the capture must name the generator that rebuilds exactly that plan.
    for scope in [
        ScopeRequest::Complete,
        ScopeRequest::Partial(PartialDownloadParts {
            parameters: true,
            group_addresses: false,
        }),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let conn = products(dir.path());
        let mut project = project();
        project.installations[0]
            .parameters
            .push(knx_core::ParameterInstance {
                id: knx_core::ParameterInstanceId(1),
                device: DeviceId(1),
                source: SourceRef {
                    path: "synthetic".into(),
                    ets_id: format!("{PROGRAM_ID}_P-Mode_R-Mode"),
                },
                raw: "0".into(),
            });
        let prepared = prepare_scoped_download(&conn, &project, TARGET.parse().unwrap(), scope)
            .unwrap()
            .prepared
            .unwrap();
        let partial = prepared.partial.as_ref().map(|(p, _)| *p);
        let device = device();
        let (stored, path) = execute(
            &device,
            &prepared.plan,
            prepared.write_selection,
            partial,
            dir.path(),
        );
        let written: Vec<_> = application_writes(&device);
        assert!(
            written
                .iter()
                .any(|(a, d)| *a <= 0x4401 && 0x4401 < *a + d.len() as u32),
            "the fallback writes the regular plan: {written:?}"
        );
        let restored = prepare_restore(&conn, &read_backup(&path).unwrap())
            .expect("a fallback download's own backup restores");
        assert_eq!(
            app_writes(&restored),
            vec![(0x4400, OLD[..4].to_vec()), (0x4405, vec![OLD[5]])]
        );
        assert_eq!(
            (prepared.write_selection, stored.write_selection),
            (LEGACY, LEGACY),
            "nothing omitted, nothing to bind"
        );
        assert_restore_guards(&conn, &stored);
    }
}

#[test]
fn genuine_format_three_full_and_partial_restore_and_recapture_keep_legacy_identity() {
    for fixture in [
        include_str!("../tests/fixtures/inactive-selection/legacy-complete.backup.json"),
        include_str!("../tests/fixtures/inactive-selection/legacy-parameters.backup.json"),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let conn = products(dir.path());
        let path = dir.path().join("original.backup.json");
        std::fs::write(&path, fixture).unwrap();
        let stored = read_backup(&path).unwrap();
        assert_eq!(stored.write_selection, LEGACY);
        assert_restore_guards(&conn, &stored);
        let plan = prepare_restore(&conn, &stored).unwrap();
        assert_eq!(
            app_writes(&plan),
            vec![(0x4400, vec![1, 0xD4, 0xD5, 0xD6]), (0x4405, vec![0xD8])]
        );
        let device = device();
        device.preset_memory(0x4400, &[1, 0x9A, 0xC7, 0x53, 0x6D, 0x68]);
        let (new, new_path) = execute(
            &device,
            &plan,
            stored.write_selection,
            stored.partial,
            dir.path(),
        );
        assert_eq!(
            application_writes(&device),
            vec![(0x4400, vec![1, 0xD4, 0xD5, 0xD6]), (0x4405, vec![0xD8])]
        );
        let value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&new_path).unwrap()).unwrap();
        assert_eq!(value["format"], 4);
        assert_eq!(value["write_selection"], "source-mask-only-v1");
        assert_eq!(new.write_selection, LEGACY);
        let repeated = prepare_restore(&conn, &read_backup(&new_path).unwrap()).unwrap();
        assert_eq!(
            app_writes(&repeated),
            vec![(0x4400, vec![1, 0x9A, 0xC7, 0x53]), (0x4405, vec![0x68])]
        );
        assert_eq!(
            std::fs::read(&path).unwrap(),
            fixture.as_bytes(),
            "the original legacy capture is immutable"
        );
    }
}
