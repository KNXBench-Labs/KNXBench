//! Runs a mask-`070nh` memory download plan against one device, step by step.
//!
//! **Direction.** *Download* here means one thing only: KNXBench writes to the
//! device, over the bus (the KNX sense: *"downloaded into the device"*, KNX
//! Architecture v03.00.02). It never means saving a file from the KNXBench
//! server to the user's computer; that is `/api/project/download`, an
//! unrelated HTTP route. See `docs/GLOSSARY.md`.
//!
//! The plan is `knx_core::commissioning::memory_download`'s and is decided
//! in full before this module sends anything. This module adds only what a
//! plan cannot know in advance: whether the device is the one the plan was
//! built for, and whether each step landed.
//!
//! - `[D]` *Configuration Procedures* (`03_05_03` v02.01.01) §3.9.2.2.2,
//!   pp. 67–68: connect, identify, then unload, load, write and complete.
//!   The identity checks all happen before the first write; a plan that asks
//!   for a check after a write is refused before anything is sent.
//! - `[D]` The load records and their state reads are MP §3.31.2's,
//!   [`ManagementSession::write_memory_load_record`].
//! - `[D]` Every data write is read back ([`ManagementSession::write_memory_region`]);
//!   MP §3.31.2 forbids Verify Mode on this mask, so the read-back is an
//!   explicit `A_Memory_Read`.
//! - `[A]` Each load record must end in the one state its event aims at:
//!   `Unloaded` after an unload, `Loading` after a load or a segment,
//!   `Loaded` after load completed. RES Table 94 permits more (an `Error`
//!   state is a legal answer), but a download that went anywhere else did
//!   not do what the plan says.
//!
//! A failure stops the run at once and nothing is undone. The machines are
//! left where the failure found them; the report says which step that was.
//!
//! One silence is not a failure: the closing Basic Restart's. `[D]` MP
//! §3.7.1.1.3, p. 80: *"The Application Layer of the Management Server
//! shall not confirm the A_Restart-service if a Basic Restart is called"*,
//! and §3.7.1.1.2, p. 78, lets the server between `t0` and `t1` *"not react
//! at all"*. §3.7.3, p. 89, adds that the reset of its communication system
//! may keep even the `T_Disconnect` off the bus. So a restart that went out
//! and heard nothing, after every machine reached `Loaded` and with nothing
//! but the disconnect left to do, is reported as
//! [`RestartOutcome::Unconfirmed`] inside an `Ok`. It is never retried here:
//! a second restart is the operator's decision (`goal-commission.md` K2).
//! `[V]` `1.1.67`, 2026-09-28, run 3 is that case.

use std::fmt;

use knx_core::commissioning::load_control_memory::MemoryLoadStateMachine;
use knx_core::commissioning::load_state::{LoadEvent, LoadState, MaskVersion};
use knx_core::commissioning::memory_download::{
    property_matches, MemoryDownloadPlan, MemoryDownloadStep,
};
use knx_core::commissioning::mutation::WriteScope;
use knx_core::commissioning::properties::ObjectIndex;

use super::{ManagementSession, SessionError};
use crate::management::ManagementTransport;

/// One step that ran, as the report tells it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepDone {
    /// The step's index in the plan.
    pub index: usize,
    /// The step, as [`MemoryDownloadStep`]'s `Display` reads it.
    pub step: String,
    /// What the device answered, if the step reads an answer.
    pub observed: Option<String>,
}

/// One thing a running download tells its observer, in the order it
/// happens. Everything here travels from KNXBench to the device, except
/// what a step reports back from it.
///
/// Carried to the operator as it happens rather than in the final report,
/// because a download is long (`1.1.67`: three minutes) and a silent one is
/// indistinguishable from a hung one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Progress {
    /// The run is about to start.
    Started {
        /// The device being written to.
        target: knx_core::IndividualAddress,
        /// Steps in the plan.
        steps: usize,
        /// Segment octets the plan writes to the device.
        data_octets: usize,
    },
    /// A step is about to run.
    StepStarted {
        /// The step's index.
        index: usize,
        /// Steps in the plan.
        of: usize,
        /// The step, as its `Display` reads it.
        step: String,
    },
    /// One `A_Memory_Write` went to the device and was read back unchanged.
    DataWritten {
        /// The step it belongs to.
        index: usize,
        /// Where it went.
        address: u32,
        /// What went there.
        octets: Vec<u8>,
        /// Segment octets written so far, this chunk included.
        written: usize,
        /// Segment octets the plan writes in all.
        of: usize,
    },
    /// A step finished.
    StepDone(StepDone),
}

/// What a completed run did.
///
/// Not `Clone` or `PartialEq`: [`RestartOutcome::Unconfirmed`] carries the
/// [`SessionError`] itself, which is neither (see its own note).
#[derive(Debug)]
pub struct MemoryDownloadReport {
    /// Every step, in order.
    pub steps: Vec<StepDone>,
    /// The state each machine was last seen in.
    pub final_states: Vec<(MemoryLoadStateMachine, LoadState)>,
    /// Octets written as segment data, every one of them read back.
    pub data_octets: usize,
    /// What became of the plan's Basic Restart. Read this before telling
    /// anyone the device runs the new program.
    pub restart: RestartOutcome,
}

/// What became of the plan's Basic Restart.
#[derive(Debug)]
pub enum RestartOutcome {
    /// The plan has no restart step.
    NotInPlan,
    /// The device acknowledged the `A_Restart` at the Transport Layer.
    /// That is all a Basic Restart ever confirms (MP §3.7.1.1.3, p. 80):
    /// that the request arrived, not that the device restarted.
    Acknowledged {
        /// The restart's step.
        index: usize,
    },
    /// The `A_Restart` went out and nothing acknowledged it. Every machine
    /// was `Loaded` and only the disconnect was left. Whether the device
    /// restarted is unknown: power-cycle it or send a restart on the
    /// operator's say-so, never automatically.
    Unconfirmed {
        /// The restart's step.
        index: usize,
        /// The silence, as the session reported it.
        error: SessionError,
    },
}

impl RestartOutcome {
    /// Whether the device acknowledged the restart request.
    pub fn is_confirmed(&self) -> bool {
        matches!(self, RestartOutcome::Acknowledged { .. })
    }
}

impl fmt::Display for RestartOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RestartOutcome::NotInPlan => f.write_str("no restart in the plan"),
            RestartOutcome::Acknowledged { index } => {
                write!(f, "step {index}: restart acknowledged")
            }
            RestartOutcome::Unconfirmed { index, error } => write!(
                f,
                "step {index}: data loaded, restart unconfirmed ({error}); \
                 power-cycle the device or restart it on purpose"
            ),
        }
    }
}

/// Why a run stopped.
#[derive(Debug)]
pub enum MemoryDownloadError {
    /// The plan does not begin with [`MemoryDownloadStep::Connect`].
    DoesNotConnectFirst,
    /// The plan checks the device after it has already written to it.
    CheckAfterWrite {
        /// The late check.
        index: usize,
    },
    /// The device reports another mask.
    MaskMismatch {
        /// The plan's.
        expected: MaskVersion,
        /// The device's.
        found: MaskVersion,
    },
    /// The device reports another manufacturer.
    ManufacturerMismatch {
        /// The plan's.
        expected: u16,
        /// The device's.
        found: u16,
    },
    /// A property did not match the product's `InlineData`.
    PropertyMismatch {
        /// The step.
        index: usize,
        /// `ObjIdx`.
        object_index: u8,
        /// `PropId`.
        property_id: u8,
        /// What the device answered.
        found: Vec<u8>,
    },
    /// A load record ended in another state than the one its event aims at.
    UnexpectedState {
        /// The step.
        index: usize,
        /// The machine.
        machine: MemoryLoadStateMachine,
        /// The event.
        event: LoadEvent,
        /// The state it should have reached.
        expected: LoadState,
        /// The state it reached.
        found: LoadState,
    },
    /// The session failed.
    Session {
        /// The step, or `None` before the first one.
        index: Option<usize>,
        /// The failure.
        error: SessionError,
    },
}

impl std::error::Error for MemoryDownloadError {}

impl fmt::Display for MemoryDownloadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemoryDownloadError::DoesNotConnectFirst => {
                f.write_str("the plan does not begin by connecting")
            }
            MemoryDownloadError::CheckAfterWrite { index } => write!(
                f,
                "step {index} checks the device after the plan has written to it"
            ),
            MemoryDownloadError::MaskMismatch { expected, found } => write!(
                f,
                "the device has mask {:04X}h, the plan is for {:04X}h",
                found.0, expected.0
            ),
            MemoryDownloadError::ManufacturerMismatch { expected, found } => write!(
                f,
                "the device is from manufacturer {found:04X}h, the plan is for {expected:04X}h"
            ),
            MemoryDownloadError::PropertyMismatch {
                index,
                object_index,
                property_id,
                found,
            } => write!(
                f,
                "step {index}: property {object_index}/{property_id} is {found:02X?}, which the product does not accept"
            ),
            MemoryDownloadError::UnexpectedState {
                index,
                machine,
                event,
                expected,
                found,
            } => write!(
                f,
                "step {index}: the {machine} went to {found:?} on {event:?}, not {expected:?}"
            ),
            MemoryDownloadError::Session { index, error } => match index {
                Some(index) => write!(f, "step {index}: {error}"),
                None => write!(f, "{error}"),
            },
        }
    }
}

/// The state an event's record must leave its machine in.
fn aimed_at(event: LoadEvent) -> LoadState {
    match event {
        LoadEvent::Unload => LoadState::Unloaded,
        LoadEvent::LoadCompleted => LoadState::Loaded,
        LoadEvent::StartLoading | LoadEvent::AdditionalLoadControls | LoadEvent::NoOperation => {
            LoadState::Loading
        }
    }
}

/// Whether a restart that ended in `error` may have reached the device.
///
/// Only the silences qualify: the request was sent and nothing came back.
/// A refusal, a missing connection or an encoding failure means nothing
/// left this machine, and that is a failure like any other.
pub(super) fn restart_may_have_gone_out(error: &SessionError) -> bool {
    matches!(
        error,
        SessionError::ConnectionReleased { .. }
            | SessionError::ConnectionLost { .. }
            | SessionError::NoAnswer { .. }
            | SessionError::Lagged { .. }
    )
}

/// Whether an unacknowledged restart at `index` closes a loaded download:
/// every machine seen so far is `Loaded`, and nothing but a disconnect
/// follows it in the plan.
fn closes_a_loaded_download(
    plan: &MemoryDownloadPlan,
    index: usize,
    states: &[(MemoryLoadStateMachine, LoadState)],
) -> bool {
    !states.is_empty()
        && states.iter().all(|(_, state)| *state == LoadState::Loaded)
        && plan.steps[index + 1..]
            .iter()
            .all(|step| *step == MemoryDownloadStep::Disconnect)
}

/// Refuses a plan whose shape this module will not run.
fn check_shape(plan: &MemoryDownloadPlan) -> Result<(), MemoryDownloadError> {
    if plan.steps.first() != Some(&MemoryDownloadStep::Connect) {
        return Err(MemoryDownloadError::DoesNotConnectFirst);
    }
    let first_write = plan.steps.iter().position(|step| {
        matches!(
            step,
            MemoryDownloadStep::LoadRecord(_)
                | MemoryDownloadStep::WriteMemory { .. }
                | MemoryDownloadStep::Restart
        )
    });
    let late_check = plan.steps.iter().enumerate().position(|(index, step)| {
        matches!(step, MemoryDownloadStep::CompareProperty { .. })
            && first_write.is_some_and(|first| index > first)
    });
    match late_check {
        Some(index) => Err(MemoryDownloadError::CheckAfterWrite { index }),
        None => Ok(()),
    }
}

/// Runs `plan` through `session`, which must be authorised for
/// [`WriteScope::Download`] to the plan's device.
pub async fn run_memory_download<T: ManagementTransport>(
    session: &mut ManagementSession<'_, T>,
    plan: &MemoryDownloadPlan,
) -> Result<MemoryDownloadReport, MemoryDownloadError> {
    run_memory_download_observed(session, plan, |_| {}).await
}

/// [`run_memory_download`], telling `observe` each [`Progress`] as it
/// happens. The observer only watches: it cannot change, skip or stop a
/// step, so what is sent is the same with or without one.
pub async fn run_memory_download_observed<T: ManagementTransport>(
    session: &mut ManagementSession<'_, T>,
    plan: &MemoryDownloadPlan,
    mut observe: impl FnMut(Progress),
) -> Result<MemoryDownloadReport, MemoryDownloadError> {
    check_shape(plan)?;
    let of = plan.steps.len();
    let total_octets = plan.data_octets();
    let mut written_octets = 0usize;
    observe(Progress::Started {
        target: session.target(),
        steps: of,
        data_octets: total_octets,
    });
    // The session decides whether to set Verify Mode from the mask, and
    // must not on this one (MP §3.31.2). The device's own mask is read and
    // compared right after connecting, before anything is written.
    session.adopt_mask(plan.mask);
    let mut report = MemoryDownloadReport {
        steps: Vec::new(),
        final_states: Vec::new(),
        data_octets: 0,
        restart: RestartOutcome::NotInPlan,
    };
    let mut limit = None;
    for (index, step) in plan.steps.iter().enumerate() {
        let at = |error| MemoryDownloadError::Session {
            index: Some(index),
            error,
        };
        observe(Progress::StepStarted {
            index,
            of,
            step: step.to_string(),
        });
        let observed = match step {
            MemoryDownloadStep::Connect => {
                session.connect().await.map_err(at)?;
                let found = session.read_mask_version().await.map_err(at)?;
                if found != plan.mask {
                    session.disconnect().await;
                    return Err(MemoryDownloadError::MaskMismatch {
                        expected: plan.mask,
                        found,
                    });
                }
                let manufacturer = session.read_manufacturer_id().await.map_err(at)?;
                if manufacturer != plan.manufacturer {
                    session.disconnect().await;
                    return Err(MemoryDownloadError::ManufacturerMismatch {
                        expected: plan.manufacturer,
                        found: manufacturer,
                    });
                }
                let negotiated = session.write_limit(None).await.map_err(at)?;
                limit = Some(negotiated);
                Some(format!(
                    "mask {:04X}h, manufacturer {manufacturer:04X}h, {} octets per write",
                    found.0,
                    negotiated.max_octets()
                ))
            }
            MemoryDownloadStep::CompareProperty {
                object_index,
                property_id,
                inline_data,
            } => {
                let found = session
                    .read_property(ObjectIndex::new(*object_index), *property_id, 1, 1)
                    .await
                    .map_err(at)?;
                if !property_matches(&found, inline_data) {
                    session.disconnect().await;
                    return Err(MemoryDownloadError::PropertyMismatch {
                        index,
                        object_index: *object_index,
                        property_id: *property_id,
                        found,
                    });
                }
                Some(hex(&found))
            }
            MemoryDownloadStep::LoadRecord(record) => {
                let found = session
                    .write_memory_load_record(*record)
                    .await
                    .map_err(at)?;
                let expected = aimed_at(record.event());
                if found != expected {
                    return Err(MemoryDownloadError::UnexpectedState {
                        index,
                        machine: record.machine(),
                        event: record.event(),
                        expected,
                        found,
                    });
                }
                report
                    .final_states
                    .retain(|(machine, _)| *machine != record.machine());
                report.final_states.push((record.machine(), found));
                Some(format!("{found:?}"))
            }
            MemoryDownloadStep::WriteMemory { address, octets } => {
                let limit = limit.expect("check_shape puts Connect first");
                let chunks = session
                    .write_memory_region_observed(
                        u32::from(*address),
                        octets,
                        limit,
                        WriteScope::Download,
                        |address, chunk| {
                            written_octets += chunk.len();
                            observe(Progress::DataWritten {
                                index,
                                address,
                                octets: chunk.to_vec(),
                                written: written_octets,
                                of: total_octets,
                            });
                        },
                    )
                    .await
                    .map_err(at)?;
                report.data_octets += octets.len();
                Some(format!("{chunks} writes, each read back"))
            }
            MemoryDownloadStep::Restart => {
                // `restart_basic_as` disconnects on every path out (MP
                // §3.7.3 exception (5)), so an unconfirmed restart leaves
                // no connection behind for the disconnect step to close.
                match session.restart_basic_as(WriteScope::Download).await {
                    Ok(()) => {
                        report.restart = RestartOutcome::Acknowledged { index };
                        Some("acknowledged".to_owned())
                    }
                    Err(error)
                        if restart_may_have_gone_out(&error)
                            && closes_a_loaded_download(plan, index, &report.final_states) =>
                    {
                        let observed = format!("unconfirmed: {error}");
                        report.restart = RestartOutcome::Unconfirmed { index, error };
                        Some(observed)
                    }
                    Err(error) => return Err(at(error)),
                }
            }
            MemoryDownloadStep::Disconnect => {
                // A restart has already disconnected (MP §3.7.3 exception
                // (5)); a second `T_Disconnect` would go to a device that is
                // starting up.
                if session.connection().is_some() {
                    session.disconnect().await;
                }
                None
            }
        };
        let done = StepDone {
            index,
            step: step.to_string(),
            observed,
        };
        observe(Progress::StepDone(done.clone()));
        report.steps.push(done);
    }
    report.final_states.sort();
    Ok(report)
}

fn hex(octets: &[u8]) -> String {
    octets
        .iter()
        .map(|octet| format!("{octet:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use knx_core::commissioning::authorisation::AuthorisationPlan;
    use knx_core::commissioning::load_control_memory::{
        abs_data_segment, abs_task_segment, event_record, AbsoluteSegment, SegmentMemoryType,
        TaskSegment,
    };
    use knx_core::commissioning::mutation::WriteAuthorisation;
    use knx_core::commissioning::properties::{PID_HARDWARE_TYPE, PID_MANUFACTURER_ID};

    use super::*;
    use crate::commissioning::simulator::{Seen, SimulatedDevice, SimulatorConfig};
    use crate::commissioning::SessionTiming;

    const MANUFACTURER: u16 = 0x0083;
    const HARDWARE: [u8; 6] = [0, 0, 0, 0, 0x01, 0x27];

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

    fn device(config: SimulatorConfig) -> SimulatedDevice {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            mask_version: 0x0701,
            ..config
        });
        device.preset_property(0, PID_MANUFACTURER_ID, &MANUFACTURER.to_be_bytes());
        device.preset_property(0, PID_HARDWARE_TYPE, &HARDWARE);
        device
    }

    fn session(
        device: &SimulatedDevice,
        scope: WriteScope,
    ) -> ManagementSession<'_, SimulatedDevice> {
        let authorisation =
            WriteAuthorisation::for_simulator(device.address(), scope).expect("not excluded");
        ManagementSession::authorised(device, AuthorisationPlan::Skip, fast(), authorisation)
            .expect("a simulator session")
    }

    fn record(machine: MemoryLoadStateMachine, event: LoadEvent) -> MemoryDownloadStep {
        MemoryDownloadStep::LoadRecord(event_record(machine, event).expect("an event record"))
    }

    /// The address table's download: the product's procedure, cut down to
    /// one machine and a 30-octet table.
    fn plan() -> MemoryDownloadPlan {
        let table = MemoryLoadStateMachine::AddressTable;
        let segment = AbsoluteSegment {
            start: 0x4000,
            length: 30,
            access: 0xFF,
            memory_type: SegmentMemoryType::Eeprom,
            checksum_control: true,
        };
        MemoryDownloadPlan {
            mask: MaskVersion(0x0701),
            manufacturer: MANUFACTURER,
            steps: vec![
                MemoryDownloadStep::Connect,
                MemoryDownloadStep::CompareProperty {
                    object_index: 0,
                    property_id: PID_HARDWARE_TYPE,
                    inline_data: [&HARDWARE[..], &[0, 0, 0, 0]].concat(),
                },
                record(table, LoadEvent::Unload),
                record(table, LoadEvent::StartLoading),
                MemoryDownloadStep::LoadRecord(
                    abs_data_segment(table, segment).expect("a segment record"),
                ),
                MemoryDownloadStep::WriteMemory {
                    address: 0x4000,
                    octets: vec![0x01],
                },
                MemoryDownloadStep::WriteMemory {
                    address: 0x4003,
                    octets: (0..27).collect(),
                },
                MemoryDownloadStep::LoadRecord(abs_task_segment(
                    table,
                    TaskSegment {
                        start: 0x4000,
                        pei_type: 0,
                        manufacturer: 0,
                        application: 0,
                        version: 0,
                    },
                )),
                record(table, LoadEvent::LoadCompleted),
                MemoryDownloadStep::Restart,
                MemoryDownloadStep::Disconnect,
            ],
        }
    }

    fn memory_writes(device: &SimulatedDevice) -> Vec<(u32, Vec<u8>)> {
        device
            .seen()
            .into_iter()
            .filter_map(|seen| match seen {
                Seen::MemoryWrite { address, data, .. } => Some((address, data)),
                _ => None,
            })
            .collect()
    }

    #[tokio::test]
    async fn a_plan_runs_to_loaded_and_leaves_the_masked_octets_alone() {
        let device = device(SimulatorConfig::default());
        // The individual address the plan must not touch.
        device.preset_memory(0x4001, &[0x11, 0x43]);
        let mut session = session(&device, WriteScope::Download);

        let report = run_memory_download(&mut session, &plan())
            .await
            .expect("the download runs");

        assert_eq!(
            report.final_states,
            vec![(MemoryLoadStateMachine::AddressTable, LoadState::Loaded)]
        );
        assert_eq!(report.data_octets, 28);
        assert_eq!(report.steps.len(), plan().steps.len());
        assert_eq!(device.load_state(ObjectIndex::new(1)), LoadState::Loaded);
        let memory: Vec<u8> = device
            .memory(0x4000, 30)
            .into_iter()
            .map(|octet| octet.expect("written or preset"))
            .collect();
        let mut expected = vec![0x01, 0x11, 0x43];
        expected.extend(0..27);
        assert_eq!(memory, expected);
        assert!(
            memory_writes(&device).iter().all(|(address, data)| {
                let end = address + data.len() as u32;
                end <= 0x4001 || *address >= 0x4003
            }),
            "nothing written to 4001h-4002h"
        );
        assert!(device.seen().iter().any(|seen| matches!(
            seen,
            Seen::Restart {
                restart_type: 0,
                ..
            }
        )));
    }

    #[tokio::test]
    async fn verify_mode_is_never_set() {
        let device = device(SimulatorConfig::default());
        let mut session = session(&device, WriteScope::Download);
        run_memory_download(&mut session, &plan())
            .await
            .expect("the download runs");
        assert!(!device.verify_mode());
        assert_eq!(device.device_control_reads(), 0);
    }

    #[tokio::test]
    async fn another_hardware_type_stops_the_run_before_any_write() {
        let device = device(SimulatorConfig::default());
        device.preset_property(0, PID_HARDWARE_TYPE, &[0, 0, 0, 0, 0x02, 0x39]);
        let mut session = session(&device, WriteScope::Download);

        let error = run_memory_download(&mut session, &plan())
            .await
            .expect_err("refused");

        assert!(
            matches!(
                error,
                MemoryDownloadError::PropertyMismatch { index: 1, ref found, .. }
                    if found == &[0, 0, 0, 0, 0x02, 0x39]
            ),
            "{error}"
        );
        assert!(!device.memory_was_written());
    }

    #[tokio::test]
    async fn another_manufacturer_stops_the_run_before_any_write() {
        let device = device(SimulatorConfig::default());
        device.preset_property(0, PID_MANUFACTURER_ID, &[0x00, 0x02]);
        let mut session = session(&device, WriteScope::Download);

        let error = run_memory_download(&mut session, &plan())
            .await
            .expect_err("refused");

        assert!(
            matches!(
                error,
                MemoryDownloadError::ManufacturerMismatch {
                    expected: MANUFACTURER,
                    found: 0x0002
                }
            ),
            "{error}"
        );
        assert!(!device.memory_was_written());
    }

    #[tokio::test]
    async fn another_mask_stops_the_run_before_any_write() {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            mask_version: 0x0705,
            ..SimulatorConfig::default()
        });
        device.preset_property(0, PID_MANUFACTURER_ID, &MANUFACTURER.to_be_bytes());
        let mut session = session(&device, WriteScope::Download);

        let error = run_memory_download(&mut session, &plan())
            .await
            .expect_err("refused");

        assert!(
            matches!(
                error,
                MemoryDownloadError::MaskMismatch {
                    expected: MaskVersion(0x0701),
                    found: MaskVersion(0x0705)
                }
            ),
            "{error}"
        );
        assert!(!device.memory_was_written());
    }

    #[tokio::test]
    async fn a_write_the_device_does_not_keep_stops_the_run() {
        let device = device(SimulatorConfig {
            corrupt_memory_writes: true,
            ..SimulatorConfig::default()
        });
        let mut session = session(&device, WriteScope::Download);

        let error = run_memory_download(&mut session, &plan())
            .await
            .expect_err("the read-back notices");

        // The first corrupted write is a load record at 0104h, which is not
        // read back; the first data write is, at step 5.
        assert!(
            matches!(
                error,
                MemoryDownloadError::Session {
                    index: Some(_),
                    error: SessionError::ReadBackMismatch { .. }
                        | SessionError::MemoryLoadNotSettled { .. }
                        | SessionError::MemoryLoadIllegalTransition { .. }
                }
            ),
            "{error}"
        );
        assert_ne!(device.load_state(ObjectIndex::new(1)), LoadState::Loaded);
    }

    #[tokio::test]
    async fn a_protected_octet_in_a_write_stops_the_run() {
        let device = device(SimulatorConfig {
            protected_memory: Some((0x4003, 0x4004)),
            ..SimulatorConfig::default()
        });
        let mut session = session(&device, WriteScope::Download);

        let error = run_memory_download(&mut session, &plan())
            .await
            .expect_err("the read-back notices");

        assert!(
            matches!(error, MemoryDownloadError::Session { index: Some(6), .. }),
            "{error}"
        );
        assert_eq!(
            device.load_state(ObjectIndex::new(1)),
            LoadState::Loading,
            "left where the failure found it"
        );
    }

    #[tokio::test]
    async fn a_session_for_another_scope_writes_nothing() {
        let device = device(SimulatorConfig::default());
        let mut session = session(&device, WriteScope::Restart);

        let error = run_memory_download(&mut session, &plan())
            .await
            .expect_err("refused");

        assert!(
            matches!(
                error,
                MemoryDownloadError::Session {
                    index: Some(2),
                    error: SessionError::Refused(_)
                }
            ),
            "{error}"
        );
        assert!(!device.memory_was_written());
    }

    #[tokio::test]
    async fn a_read_only_session_writes_nothing() {
        let device = device(SimulatorConfig::default());
        let mut session = ManagementSession::read_only(
            &device,
            device.address(),
            AuthorisationPlan::Skip,
            fast(),
        )
        .expect("a session");

        let error = run_memory_download(&mut session, &plan())
            .await
            .expect_err("refused");

        assert!(
            matches!(
                error,
                MemoryDownloadError::Session {
                    index: Some(2),
                    error: SessionError::NoAuthorisation { .. }
                }
            ),
            "{error}"
        );
        assert!(!device.memory_was_written());
    }

    #[tokio::test]
    async fn a_plan_that_checks_after_writing_is_refused_before_connecting() {
        let device = device(SimulatorConfig::default());
        let mut session = session(&device, WriteScope::Download);
        let mut plan = plan();
        let check = plan.steps.remove(1);
        plan.steps.insert(3, check);

        let error = run_memory_download(&mut session, &plan)
            .await
            .expect_err("refused");

        assert!(
            matches!(error, MemoryDownloadError::CheckAfterWrite { index: 3 }),
            "{error}"
        );
        assert!(device.seen().is_empty(), "nothing sent at all");
    }

    #[tokio::test]
    async fn a_plan_that_does_not_connect_first_is_refused() {
        let device = device(SimulatorConfig::default());
        let mut session = session(&device, WriteScope::Download);
        let mut plan = plan();
        plan.steps.remove(0);

        let error = run_memory_download(&mut session, &plan)
            .await
            .expect_err("refused");

        assert!(matches!(error, MemoryDownloadError::DoesNotConnectFirst));
        assert!(device.seen().is_empty());
    }

    /// Recovery after an interrupted download is running the whole download
    /// again, and the plan's opening unload is what makes that work.
    ///
    /// `[D]` MP §3.1, p. 68: *"if an error is detected, the download shall be
    /// interrupted and an error-message shall be raised"*. There is no
    /// separate recovery procedure for it. RES Table 94, p. 296: a *Device
    /// Restart* in `Loading` leads to `Error` (recommended) or stays in
    /// `Loading` (optional), and `Unload` leads from every state, `Error`
    /// and `Loading` included, to `Unloaded`. CP §3.4.1.2.1 step 6, p. 38,
    /// opens the complete download with *"Unload all loadable Objects"*.
    /// So an interruption leaves `Loading` or `Error`, and the same plan run
    /// again ends `Loaded`.
    #[tokio::test]
    async fn the_same_plan_run_again_recovers_a_part_left_loading_or_in_error() {
        for left_in in [LoadState::Loading, LoadState::Error] {
            let device = device(SimulatorConfig::default());
            device.preset_load_state(ObjectIndex::new(1), left_in);
            let mut session = session(&device, WriteScope::Download);

            let report = run_memory_download(&mut session, &plan())
                .await
                .unwrap_or_else(|e| panic!("left in {left_in:?}: {e}"));

            assert_eq!(
                device.load_state(ObjectIndex::new(1)),
                LoadState::Loaded,
                "left in {left_in:?}: {report:?}"
            );
            assert_eq!(
                device.memory(0x4003, 27),
                (0..27).map(Some).collect::<Vec<_>>(),
                "left in {left_in:?}: the table is written again in full"
            );
        }
    }

    #[tokio::test]
    async fn a_permitted_state_that_is_not_the_aim_stops_the_run() {
        // RES Table 94: `Start Loading` from `Error` stays in `Error`. The
        // session accepts that as the correct answer; the download must not.
        let device = device(SimulatorConfig::default());
        device.preset_load_state(ObjectIndex::new(1), LoadState::Error);
        let mut session = session(&device, WriteScope::Download);
        let mut plan = plan();
        plan.steps.remove(2); // no unload first

        let error = run_memory_download(&mut session, &plan)
            .await
            .expect_err("refused");

        assert!(
            matches!(
                error,
                MemoryDownloadError::UnexpectedState {
                    index: 2,
                    machine: MemoryLoadStateMachine::AddressTable,
                    event: LoadEvent::StartLoading,
                    expected: LoadState::Loading,
                    found: LoadState::Error,
                }
            ),
            "{error}"
        );
        assert!(
            memory_writes(&device)
                .iter()
                .all(|(address, _)| *address == 0x0104),
            "no segment data after a failed load record"
        );
        assert!(
            !device
                .seen()
                .iter()
                .any(|seen| matches!(seen, Seen::Restart { .. })),
            "no restart after a failed load"
        );
    }

    /// K2: `1.1.67`, 2026-09-28, run 3. Every data write read back, every
    /// load state `Loaded`, and the closing Basic Restart never got its
    /// `T_ACK`. MP §3.7.1.1.2, p. 78 lets the server *"not react at all"*
    /// and §3.7.1.1.3, p. 80 forbids it an AL confirmation, so the silence
    /// is not a failed download. It is not a confirmed restart either.
    #[tokio::test]
    async fn an_unacknowledged_closing_restart_is_loaded_but_unconfirmed() {
        let device = device(SimulatorConfig {
            restart_unanswered: true,
            ..SimulatorConfig::default()
        });
        let mut session = session(&device, WriteScope::Download);

        let report = run_memory_download(&mut session, &plan())
            .await
            .expect("the data is loaded, so this is not a failed download");

        assert_eq!(
            report.final_states,
            vec![(MemoryLoadStateMachine::AddressTable, LoadState::Loaded)]
        );
        assert_eq!(report.data_octets, 28);
        assert!(
            matches!(
                report.restart,
                RestartOutcome::Unconfirmed {
                    index: 9,
                    error: SessionError::ConnectionReleased { .. }
                }
            ),
            "{:?}",
            report.restart
        );
        assert!(!report.restart.is_confirmed());
        // `1.1.67`'s trace (2026-09-28) printed "no T_ACK for T_ACK for
        // A_Restart": the message must name the request once.
        let shown = report.restart.to_string();
        assert!(
            shown.contains("no T_ACK for A_Restart (Basic Restart)"),
            "{shown}"
        );
        assert!(!shown.contains("T_ACK for T_ACK"), "{shown}");
        let seqs = device.unanswered_restart_seqs();
        assert_eq!(
            seqs.len(),
            usize::from(crate::commissioning::MAX_TRANSMISSIONS)
        );
        assert!(
            seqs.iter().all(|seq| *seq == seqs[0]),
            "TL's repetitions of one request, never a second request: {seqs:?}"
        );
        assert_eq!(
            report.steps.len(),
            plan().steps.len(),
            "disconnect still ran"
        );
        assert!(session.connection().is_none());
    }

    #[tokio::test]
    async fn an_acknowledged_closing_restart_is_confirmed() {
        let device = device(SimulatorConfig::default());
        let mut session = session(&device, WriteScope::Download);
        let report = run_memory_download(&mut session, &plan())
            .await
            .expect("the download runs");
        assert!(
            matches!(report.restart, RestartOutcome::Acknowledged { index: 9 }),
            "{:?}",
            report.restart
        );
        assert!(report.restart.is_confirmed());
        assert!(device.unanswered_restart_seqs().is_empty());
    }

    /// The unconfirmed outcome is only for a restart after a complete load.
    /// A plan with anything but a disconnect after its restart has not
    /// finished, and the run stops as it always did.
    #[tokio::test]
    async fn an_unacknowledged_restart_before_the_end_still_fails() {
        let device = device(SimulatorConfig {
            restart_unanswered: true,
            ..SimulatorConfig::default()
        });
        let mut session = session(&device, WriteScope::Download);
        let mut plan = plan();
        plan.steps.insert(
            10,
            MemoryDownloadStep::WriteMemory {
                address: 0x4003,
                octets: vec![0],
            },
        );

        let error = run_memory_download(&mut session, &plan)
            .await
            .expect_err("a step after the restart is left undone");

        assert!(
            matches!(
                error,
                MemoryDownloadError::Session {
                    index: Some(9),
                    error: SessionError::ConnectionReleased { .. }
                }
            ),
            "{error}"
        );
    }

    /// Nor for a restart whose machines are not all `Loaded`.
    #[tokio::test]
    async fn an_unacknowledged_restart_without_loaded_machines_still_fails() {
        let device = device(SimulatorConfig {
            restart_unanswered: true,
            ..SimulatorConfig::default()
        });
        let mut session = session(&device, WriteScope::Download);
        let mut plan = plan();
        plan.steps.remove(8); // no LoadCompleted: the table stays Loading

        let error = run_memory_download(&mut session, &plan)
            .await
            .expect_err("a table left in Loading is not a loaded download");

        assert!(
            matches!(
                error,
                MemoryDownloadError::Session {
                    index: Some(8),
                    error: SessionError::ConnectionReleased { .. }
                }
            ),
            "{error}"
        );
    }

    /// A refused restart never left the machine: that is a failure, not an
    /// unconfirmed restart.
    #[test]
    fn a_restart_that_was_never_sent_is_not_unconfirmed() {
        assert!(!restart_may_have_gone_out(&SessionError::NotConnected));
        assert!(!restart_may_have_gone_out(&SessionError::Refused(
            knx_core::commissioning::mutation::AuthorisationRefused::WrongScope {
                authorised: WriteScope::Download,
                attempted: WriteScope::Restart,
            }
        )));
        assert!(restart_may_have_gone_out(
            &SessionError::ConnectionReleased {
                waiting_for: "x",
                each: Duration::from_millis(1),
                attempts: 4,
            }
        ));
        assert!(restart_may_have_gone_out(&SessionError::ConnectionLost {
            during: "x"
        }));
        assert!(restart_may_have_gone_out(&SessionError::Lagged {
            waiting_for: "x"
        }));
        assert!(restart_may_have_gone_out(&SessionError::NoAnswer {
            waiting_for: "x",
            each: Duration::from_millis(1),
            attempts: 1,
        }));
    }

    /// The operator sees every step start and finish, and every octet that
    /// goes to the device, in order, while the run is still going.
    #[tokio::test]
    async fn progress_shows_every_step_and_every_octet_sent() {
        let device = device(SimulatorConfig::default());
        let mut session = session(&device, WriteScope::Download);
        let mut seen = Vec::new();

        let report = run_memory_download_observed(&mut session, &plan(), |p| seen.push(p))
            .await
            .expect("the download runs");

        let steps = plan().steps.len();
        assert_eq!(
            seen.first(),
            Some(&Progress::Started {
                target: device.address(),
                steps,
                data_octets: 28,
            })
        );
        let started: Vec<usize> = seen
            .iter()
            .filter_map(|p| match p {
                Progress::StepStarted { index, of, .. } => {
                    assert_eq!(*of, steps);
                    Some(*index)
                }
                _ => None,
            })
            .collect();
        assert_eq!(started, (0..steps).collect::<Vec<_>>());
        let done: Vec<&StepDone> = seen
            .iter()
            .filter_map(|p| match p {
                Progress::StepDone(done) => Some(done),
                _ => None,
            })
            .collect();
        assert_eq!(done, report.steps.iter().collect::<Vec<_>>());

        // Exactly the octets the device received, at the addresses it
        // received them, with a running total that ends at the plan's.
        let data: Vec<(u32, Vec<u8>, usize)> = seen
            .iter()
            .filter_map(|p| match p {
                Progress::DataWritten {
                    address,
                    octets,
                    written,
                    of,
                    ..
                } => {
                    assert_eq!(*of, 28);
                    Some((*address, octets.clone(), *written))
                }
                _ => None,
            })
            .collect();
        let shown: Vec<(u32, Vec<u8>)> = data.iter().map(|(a, o, _)| (*a, o.clone())).collect();
        let sent: Vec<(u32, Vec<u8>)> = memory_writes(&device)
            .into_iter()
            .filter(|(address, _)| *address != 0x0104)
            .collect();
        assert_eq!(shown, sent);
        assert_eq!(data.last().map(|(_, _, total)| *total), Some(28));
        assert!(data.windows(2).all(|w| w[0].2 < w[1].2));

        // Each data chunk is shown inside its own step, after it starts and
        // before it finishes.
        let position = |wanted: &dyn Fn(&Progress) -> bool| seen.iter().position(wanted).unwrap();
        let first_data = position(&|p| matches!(p, Progress::DataWritten { index: 5, .. }));
        assert!(position(&|p| matches!(p, Progress::StepStarted { index: 5, .. })) < first_data);
        assert!(first_data < position(&|p| matches!(p, Progress::StepDone(d) if d.index == 5)));
    }

    /// A failing run has told the operator everything up to the failure,
    /// and nothing after it.
    #[tokio::test]
    async fn progress_stops_where_the_run_stops() {
        let device = device(SimulatorConfig {
            protected_memory: Some((0x4003, 0x4004)),
            ..SimulatorConfig::default()
        });
        let mut session = session(&device, WriteScope::Download);
        let mut seen = Vec::new();

        run_memory_download_observed(&mut session, &plan(), |p| seen.push(p))
            .await
            .expect_err("the read-back notices");

        assert!(matches!(
            seen.last(),
            Some(Progress::StepStarted { index: 6, .. })
        ));
        assert!(!seen
            .iter()
            .any(|p| matches!(p, Progress::StepDone(d) if d.index >= 6)));
    }

    /// Watching changes nothing: the same frames go out with or without an
    /// observer.
    #[tokio::test]
    async fn an_observer_does_not_change_what_is_sent() {
        let quiet = device(SimulatorConfig::default());
        run_memory_download(&mut session(&quiet, WriteScope::Download), &plan())
            .await
            .expect("runs");
        let watched = device(SimulatorConfig::default());
        run_memory_download_observed(
            &mut session(&watched, WriteScope::Download),
            &plan(),
            |_| {},
        )
        .await
        .expect("runs");
        assert_eq!(quiet.seen(), watched.seen());
    }

    #[test]
    fn every_event_aims_at_one_state() {
        assert_eq!(aimed_at(LoadEvent::Unload), LoadState::Unloaded);
        assert_eq!(aimed_at(LoadEvent::StartLoading), LoadState::Loading);
        assert_eq!(
            aimed_at(LoadEvent::AdditionalLoadControls),
            LoadState::Loading
        );
        assert_eq!(aimed_at(LoadEvent::LoadCompleted), LoadState::Loaded);
    }
}
