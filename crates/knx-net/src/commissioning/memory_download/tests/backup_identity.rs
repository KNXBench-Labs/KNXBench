//! Backups must not outlive the application they were read from (restore derivation).

use std::cell::Cell;
use std::time::Duration;

use super::*;
use knx_core::commissioning::device_backup::{restore_plan, PriorApplication};
use knx_core::commissioning::mutation::TargetKind;
use knx_core::commissioning::partial_memory_download::PartialDownloadParts;
use knx_core::commissioning::properties::PID_SERIAL_NUMBER;
use tokio::sync::broadcast;

use crate::cemi::{ApplicationService, Destination, Tpci};
use crate::client::{BusError, TunnelEvent};

const OTHER: [u8; 5] = [0x00, 0x83, 0x00, 0x27, 0x14];

/// More frames than the simulator's event channel holds.
const FLOOD: usize = 400;

/// What the wrapped transport does to the one read of object 3, PID 13.
#[derive(Clone, Copy)]
enum Fault {
    None,
    /// Ask for PID 14 instead: the device acknowledges and answers a
    /// property nobody waits for, so the PID 13 read sees silence on a live
    /// connection.
    AckedSilence,
    /// Send nothing: no acknowledge, so the connection is released.
    Swallow,
    /// The transport refuses the frame.
    Refuse,
    /// Flood the event channel past its capacity before the read goes out,
    /// so the waiting session finds it lagged.
    Flood,
}

struct Wire<'a> {
    device: &'a SimulatedDevice,
    fault: Fault,
    frames: Cell<usize>,
    frames_at_prior_read: Cell<Option<usize>>,
}

impl<'a> Wire<'a> {
    fn new(device: &'a SimulatedDevice, fault: Fault) -> Self {
        Self {
            device,
            fault,
            frames: Cell::new(0),
            frames_at_prior_read: Cell::new(None),
        }
    }
}

impl ManagementTransport for Wire<'_> {
    fn assigned_address(&self) -> knx_core::IndividualAddress {
        ManagementTransport::assigned_address(self.device)
    }

    fn subscribe(&self) -> broadcast::Receiver<TunnelEvent> {
        ManagementTransport::subscribe(self.device)
    }

    async fn send_frame(
        &self,
        destination: Destination,
        transport: Tpci,
        service: ApplicationService,
    ) -> Result<(), BusError> {
        self.frames.set(self.frames.get() + 1);
        let mut service = service;
        if let ApplicationService::PropertyValueRead {
            object_index: 3,
            property_id: PID_PROGRAM_VERSION,
            ..
        } = &mut service
        {
            self.frames_at_prior_read.set(Some(self.frames.get()));
            match self.fault {
                Fault::None => {}
                Fault::Swallow => return Ok(()),
                Fault::Refuse => return Err(BusError::Timeout),
                Fault::Flood => {
                    // Harmless reads of the neighbouring PID: each one adds
                    // events to the channel the session already listens to.
                    for _ in 0..FLOOD {
                        let mut noise = service.clone();
                        if let ApplicationService::PropertyValueRead { property_id, .. } =
                            &mut noise
                        {
                            *property_id = PID_PROGRAM_VERSION + 1;
                        }
                        ManagementTransport::send_frame(self.device, destination, transport, noise)
                            .await?;
                    }
                }
                Fault::AckedSilence => {
                    if let ApplicationService::PropertyValueRead { property_id, .. } = &mut service
                    {
                        *property_id = PID_PROGRAM_VERSION + 1;
                    }
                }
            }
        }
        ManagementTransport::send_frame(self.device, destination, transport, service).await
    }

    fn target_kind(&self) -> TargetKind {
        ManagementTransport::target_kind(self.device)
    }
}

fn wire_session<'a>(wire: &'a Wire<'a>) -> ManagementSession<'a, Wire<'a>> {
    let authorisation =
        WriteAuthorisation::for_simulator(wire.device.address(), WriteScope::Download)
            .expect("not excluded");
    ManagementSession::authorised(wire, AuthorisationPlan::Skip, fast(), authorisation)
        .expect("a simulator session")
}

/// A device whose three machines are loaded and whose memory holds old data
/// where the complete plan writes; `prior` is what it answers for PID 13.
fn device_holding(prior: Option<&[u8]>, config: SimulatorConfig) -> SimulatedDevice {
    let device = device(config);
    if let Some(prior) = prior {
        device.preset_property(3, PID_PROGRAM_VERSION, prior);
    }
    for machine in MemoryLoadStateMachine::ALL {
        device.preset_load_state(ObjectIndex::new(machine.type_number()), LoadState::Loaded);
    }
    device.preset_memory(0x4000, &[0x07]);
    device.preset_memory(0x4003, &(100..127).collect::<Vec<u8>>());
    device.preset_memory(0x4400, &[0x5A; 8]);
    device
}

struct Taken {
    result: Result<MemoryDownloadReport, MemoryDownloadError>,
    kept: Option<DeviceBackup>,
}

async fn backed_up<T: ManagementTransport>(
    session: &mut ManagementSession<'_, T>,
    plan: &MemoryDownloadPlan,
) -> Taken {
    let mut kept = None;
    let result = run_memory_download_with_backup(session, plan, |_| {}, &mut |b: &DeviceBackup| {
        kept = Some(b.clone());
        Ok(())
    })
    .await;
    Taken { result, kept }
}

fn prior_reads(device: &SimulatedDevice) -> usize {
    device
        .seen()
        .iter()
        .filter(|seen| {
            matches!(
                seen,
                Seen::PropertyRead {
                    object_index: 3,
                    property_id: PID_PROGRAM_VERSION
                }
            )
        })
        .count()
}

#[tokio::test]
async fn a_backup_of_another_application_does_not_restore_under_this_plans_procedure() {
    let device = device_holding(Some(&OTHER), SimulatorConfig::default());
    let plan = complete_with_application();
    let taken = backed_up(&mut session(&device, WriteScope::Download), &plan).await;
    taken
        .result
        .expect("the complete download runs over another application");
    let backup = taken.kept.expect("the backup was kept");
    assert_eq!(backup.prior_application, PriorApplication::Recorded(OTHER));
    let restore = restore_plan(&plan, device.address(), &backup);
    assert_eq!(
        restore,
        Err(RestoreError::OtherApplication {
            plan: IDENTITY,
            backup: OTHER
        })
    );
}

#[tokio::test]
async fn the_same_application_restores_the_original_octets() {
    let device = device_holding(Some(&IDENTITY), SimulatorConfig::default());
    device.preset_property(0, PID_SERIAL_NUMBER, &SERIAL);
    let before = device.memory(0x4000, 0x4408 - 0x4000);
    let plan = complete_with_application();
    let taken = backed_up(&mut session(&device, WriteScope::Download), &plan).await;
    taken.result.expect("the download runs");
    let backup = taken.kept.expect("kept");
    assert_eq!(
        backup.prior_application,
        PriorApplication::Recorded(IDENTITY)
    );
    assert_eq!(prior_reads(&device), 1, "one read, no retry");
    assert_ne!(device.memory(0x4000, 0x4408 - 0x4000), before);

    let restore = restore_plan(&plan, device.address(), &backup).expect("same application");
    run_memory_download(&mut session(&device, WriteScope::Download), &restore)
        .await
        .expect("the restore runs");
    assert_eq!(device.memory(0x4000, 0x4408 - 0x4000), before);
}

#[tokio::test]
async fn an_unusable_answer_still_downloads_but_the_backup_never_restores() {
    let wrong_width = |len: usize| Some([&IDENTITY[..], &[0; 4]].concat()[..len].to_vec());
    let cases: [(&str, Option<Vec<u8>>, Fault); 5] = [
        ("absent", None, Fault::None),
        ("truncated", wrong_width(4), Fault::None),
        ("oversized", wrong_width(6), Fault::None),
        ("empty", Some(vec![]), Fault::None),
        ("silent", Some(IDENTITY.to_vec()), Fault::AckedSilence),
    ];
    for (name, prior, fault) in cases {
        let device = device_holding(prior.as_deref(), SimulatorConfig::default());
        let wire = Wire::new(&device, fault);
        let plan = complete_with_application();
        let taken = backed_up(&mut wire_session(&wire), &plan).await;
        taken.result.unwrap_or_else(|e| panic!("{name}: {e}"));
        let backup = taken.kept.expect("kept");
        assert_eq!(
            backup.prior_application,
            PriorApplication::NotRecorded,
            "{name}"
        );
        assert_eq!(
            restore_plan(&plan, device.address(), &backup),
            Err(RestoreError::PriorApplicationNotRecorded { plan: IDENTITY }),
            "{name}"
        );
    }
}

#[tokio::test]
async fn a_lost_connection_during_the_identity_read_aborts_before_keep_and_write() {
    // Dry run to find the frame that carries the PID 13 read.
    let probe_device = device_holding(Some(&IDENTITY), SimulatorConfig::default());
    let probe = Wire::new(&probe_device, Fault::None);
    backed_up(&mut wire_session(&probe), &complete_with_application())
        .await
        .result
        .expect("probe run");
    let at = probe.frames_at_prior_read.get().expect("the read was sent");

    for (name, fault, config) in [
        (
            "dropped",
            Fault::None,
            SimulatorConfig {
                drop_connection_after: Some(at - 1),
                ..SimulatorConfig::default()
            },
        ),
        ("released", Fault::Swallow, SimulatorConfig::default()),
    ] {
        let device = device_holding(Some(&IDENTITY), config);
        let wire = Wire::new(&device, fault);
        let taken = backed_up(&mut wire_session(&wire), &complete_with_application()).await;
        let error = taken.result.expect_err(name);
        assert!(
            matches!(
                error,
                MemoryDownloadError::BackupRead {
                    error: SessionError::ConnectionLost { .. }
                        | SessionError::ConnectionReleased { .. },
                    ..
                }
            ),
            "{name}: {error}"
        );
        assert!(taken.kept.is_none(), "{name}: keep must not run");
        assert!(
            writes_of_any_kind(&device).is_empty(),
            "{name}: nothing written"
        );
    }
}

/// The PID 13 read fails on the wire in a way that is neither a lost
/// connection nor a refusal: the run aborts with the original error, keeps
/// nothing and writes nothing. (The local `Encode` variant cannot come out
/// of a transport, see `an_encode_failure_is_not_swallowed_as_unrecorded`.)
#[tokio::test]
async fn a_transport_failure_or_lag_during_the_identity_read_aborts_before_keep_and_write() {
    for (name, fault) in [("transport", Fault::Refuse), ("lagged", Fault::Flood)] {
        let device = device_holding(Some(&IDENTITY), SimulatorConfig::default());
        let wire = Wire::new(&device, fault);
        let taken = backed_up(&mut wire_session(&wire), &complete_with_application()).await;
        let error = taken.result.expect_err(name);
        let kind = match &error {
            MemoryDownloadError::BackupRead {
                error: SessionError::Transport(BusError::Timeout),
                ..
            } => "transport",
            MemoryDownloadError::BackupRead {
                error: SessionError::Lagged { .. },
                ..
            } => "lagged",
            other => panic!("{name}: {other}"),
        };
        assert_eq!(kind, name, "the original error variant is kept");
        assert!(taken.kept.is_none(), "{name}: keep must not run");
        assert!(
            writes_of_any_kind(&device).is_empty(),
            "{name}: nothing written"
        );
    }
}

/// `Encode` is a programming error of this crate and no transport can
/// produce it, so the classifier is exercised on its result directly: only
/// a refusal, a wrong width and an acknowledged silence on a live
/// connection are tolerated; every other error comes back unchanged.
#[test]
fn an_encode_failure_is_not_swallowed_as_unrecorded() {
    let encode = || SessionError::Encode(crate::cemi::CemiError::UnsupportedMessageCode(0));
    assert!(matches!(
        prior_from_read(Err(encode()), true),
        Err(SessionError::Encode(_))
    ));
    assert!(matches!(
        prior_from_read(Err(encode()), false),
        Err(SessionError::Encode(_))
    ));
    let silent = || SessionError::NoAnswer {
        waiting_for: "x",
        each: Duration::ZERO,
        attempts: 1,
    };
    assert!(matches!(
        prior_from_read(Err(silent()), true),
        Ok(PriorApplication::NotRecorded)
    ));
    assert!(matches!(
        prior_from_read(Err(silent()), false),
        Err(SessionError::NoAnswer { .. })
    ));
    assert!(matches!(
        prior_from_read(Ok(IDENTITY.to_vec()), true),
        Ok(PriorApplication::Recorded(found)) if found == IDENTITY
    ));
}

/// What the device saw, in order, with the moment `keep` ran marked.
#[tokio::test]
async fn the_identity_is_read_after_the_plans_checks_and_before_keep_and_the_first_write() {
    let device = device_holding(Some(&IDENTITY), SimulatorConfig::default());
    let mut plan = complete_with_application();
    plan.steps.insert(
        2,
        MemoryDownloadStep::RequireLoaded(MemoryLoadStateMachine::AddressTable),
    );
    let mut seen_at_keep = None;
    let result = run_memory_download_with_backup(
        &mut session(&device, WriteScope::Download),
        &plan,
        |_| {},
        &mut |_: &DeviceBackup| {
            seen_at_keep = Some(device.seen().len());
            Ok(())
        },
    )
    .await;
    result.expect("the download runs");
    let seen = device.seen();
    let kept_after = seen_at_keep.expect("keep ran");
    let compare = seen
        .iter()
        .position(|s| {
            matches!(s, Seen::PropertyRead { object_index: 0, property_id } if *property_id == PID_HARDWARE_TYPE)
        })
        .expect("the plan's own compare");
    let identity = seen
        .iter()
        .position(|s| {
            matches!(
                s,
                Seen::PropertyRead {
                    object_index: 3,
                    property_id: PID_PROGRAM_VERSION
                }
            )
        })
        .expect("the identity read");
    let first_write = seen
        .iter()
        .position(|s| {
            matches!(
                s,
                Seen::MemoryWrite { .. } | Seen::PropertyWrite { .. } | Seen::Restart { .. }
            )
        })
        .expect("a write");
    let reads_between: Vec<_> = seen[compare + 1..identity].to_vec();
    let after_identity = &seen[identity + 1..kept_after];
    let load_state_reads = |part: &[Seen]| {
        part.iter()
            .filter(|s| matches!(s, Seen::MemoryRead { .. } | Seen::PropertyRead { .. }))
            .count()
    };
    // Plan compare, then its RequireLoaded read, then the identity.
    assert!(compare < identity);
    assert!(
        load_state_reads(&reads_between) >= 1,
        "RequireLoaded ran first"
    );
    // Then the backup's load states and regions, then keep, then writes.
    assert!(
        load_state_reads(after_identity) >= 2,
        "load states and regions"
    );
    assert!(
        identity < kept_after,
        "the backup follows the identity read"
    );
    assert!(kept_after <= first_write, "no mutation before keep ran");
}

/// The executor, not the derivation alone, keeps a late identity compare
/// from ever counting: the same malformed plan the core refuses as a
/// precondition is refused here before any traffic, backup or write.
#[tokio::test]
async fn an_identity_compare_after_the_first_write_is_refused_before_any_traffic_or_keep() {
    let device = device_holding(Some(&IDENTITY), SimulatorConfig::default());
    let mut plan = complete_with_application();
    let first_write = plan
        .steps
        .iter()
        .position(MemoryDownloadStep::changes_device)
        .expect("the plan writes");
    plan.steps.insert(
        first_write + 1,
        MemoryDownloadStep::CompareProperty {
            object_index: 3,
            property_id: PID_PROGRAM_VERSION,
            inline_data: IDENTITY.to_vec(),
        },
    );
    let taken = backed_up(&mut session(&device, WriteScope::Download), &plan).await;
    let error = taken.result.expect_err("refused");
    assert!(
        matches!(error, MemoryDownloadError::CheckAfterWrite { index } if index == first_write + 1),
        "{error}"
    );
    assert!(taken.kept.is_none(), "keep must not run");
    assert!(device.seen().is_empty(), "nothing sent at all");
}

#[tokio::test]
async fn unbacked_and_read_only_runs_keep_their_traffic_and_record_nothing() {
    let device = device_holding(Some(&IDENTITY), SimulatorConfig::default());
    run_memory_download(
        &mut session(&device, WriteScope::Download),
        &complete_with_application(),
    )
    .await
    .expect("unbacked run");
    assert_eq!(prior_reads(&device), 0, "an unbacked run reads no PID 13");

    let device = device_holding(Some(&IDENTITY), SimulatorConfig::default());
    let held = read_what_the_plan_overwrites(&mut read_only(&device), &complete_with_application())
        .await
        .expect("preview read");
    assert_eq!(held.prior_application, PriorApplication::NotRecorded);
    assert_eq!(prior_reads(&device), 0, "a preview reads no PID 13");
}

#[tokio::test]
async fn a_legacy_partial_backup_restores_through_its_executed_identity_check() {
    let parts = PartialDownloadParts {
        parameters: true,
        group_addresses: true,
    };
    let device = device_holding(Some(&IDENTITY), SimulatorConfig::default());
    device.preset_property(0, PID_SERIAL_NUMBER, &SERIAL);
    let plan = partial(parts);
    let taken = backed_up(&mut session(&device, WriteScope::Download), &plan).await;
    taken.result.expect("the partial download runs");
    let mut legacy = taken.kept.expect("kept");
    legacy.prior_application = PriorApplication::NotRecorded;
    let restore = restore_plan(&plan, device.address(), &legacy).expect("its own precondition");

    // The device now holds another application: the restore's own check stops it.
    device.preset_property(3, PID_PROGRAM_VERSION, &OTHER);
    let writes = writes_of_any_kind(&device).len();
    let error = run_memory_download(&mut session(&device, WriteScope::Download), &restore)
        .await
        .expect_err("the wrong application is refused by the plan");
    assert!(
        matches!(
            error,
            MemoryDownloadError::PropertyMismatch { property_id: PID_PROGRAM_VERSION, ref found, .. }
                if found == &OTHER
        ),
        "{error}"
    );
    assert_eq!(writes_of_any_kind(&device).len(), writes, "nothing written");

    // A recorded identity that conflicts cannot hide behind the precondition.
    let mut forged = legacy;
    forged.prior_application = PriorApplication::Recorded(OTHER);
    assert!(matches!(
        restore_plan(&plan, device.address(), &forged),
        Err(RestoreError::OtherApplication { .. })
    ));
}
