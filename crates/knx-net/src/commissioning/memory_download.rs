//! Runs a mask-`070nh` memory download plan against one device, step by step.
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

/// What a completed run did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryDownloadReport {
    /// Every step, in order.
    pub steps: Vec<StepDone>,
    /// The state each machine was last seen in.
    pub final_states: Vec<(MemoryLoadStateMachine, LoadState)>,
    /// Octets written as segment data, every one of them read back.
    pub data_octets: usize,
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
    check_shape(plan)?;
    // The session decides whether to set Verify Mode from the mask, and
    // must not on this one (MP §3.31.2). The device's own mask is read and
    // compared right after connecting, before anything is written.
    session.adopt_mask(plan.mask);
    let mut report = MemoryDownloadReport {
        steps: Vec::new(),
        final_states: Vec::new(),
        data_octets: 0,
    };
    let mut limit = None;
    for (index, step) in plan.steps.iter().enumerate() {
        let at = |error| MemoryDownloadError::Session {
            index: Some(index),
            error,
        };
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
                    .write_memory_region(u32::from(*address), octets, limit)
                    .await
                    .map_err(at)?;
                report.data_octets += octets.len();
                Some(format!("{chunks} writes, each read back"))
            }
            MemoryDownloadStep::Restart => {
                session
                    .restart_basic_as(WriteScope::Download)
                    .await
                    .map_err(at)?;
                None
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
        report.steps.push(StepDone {
            index,
            step: step.to_string(),
            observed,
        });
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
