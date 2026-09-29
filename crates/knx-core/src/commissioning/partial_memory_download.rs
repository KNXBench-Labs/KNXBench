//! A mask-`070nh` partial download, derived from the complete one by CP's rules.
//!
//! `[D]` *Configuration Procedures* (`03_05_03` v02.01.01) §3.9.2.4, pp. 69–70,
//! *"Default Partial Download Procedure"*, which follows the complete download
//! of §3.9.2.2 that [`super::memory_download`] already takes its order from.
//! The Standard does not write the partial download out a second time. It
//! says: *"This is generated from the complete download procedure by applying
//! the following transformations"*, and lists them:
//!
//! 1. *"Remove all UNLOAD(OIDX_APPLICATION_PROGRAM) and
//!    UNLOAD(OIDX_PEI_PROGRAM) load controls."*
//! 2. *"If the download does not include the group communication part"*,
//!    remove every load control of the Group Address Table and Group Object
//!    Association Table machines, and their segment allocations.
//! 3. *"Change all OIDX_APPLICATION_PROGRAM or OIDX_PEI_PROGRAM segment
//!    allocations to memory writes (absolute data or stack segments in EEPROM
//!    only, all others are simply ignored)."*
//!
//! The additional input is the *"Partial Download Type (Parameters and/or
//! Group Addresses)"*; the parameters part is the `If (Partial Download Type
//! & Parameters)` block, which loads and writes the application program. So
//! a group-addresses-only download drops the application program's steps too.
//!
//! [`derive_partial_plan`] applies exactly those rules to a complete
//! [`MemoryDownloadPlan`], so the partial download can never write anything
//! the complete one would not have written.
//!
//! `[A]` Two things are this project's, not the Standard's:
//!
//! - Before anything is written, the partial plan checks that the device
//!   carries *this* application ([`PID_PROGRAM_VERSION`] of the Application
//!   Program Object against the plan's own task segment) and that every part
//!   the complete plan loads is `Loaded`. CP's procedure only calls
//!   `DMP_Identify_RCo2`, which checks the hardware, not the application; a
//!   parameter block written into another application's memory would be
//!   silent corruption. On the MDT `1.1.67` (mask `0701h`), object index 3,
//!   PID 13 answered `00 83 00 27 15` for `A-0027-15-0BAC` (device dump of
//!   2026-09-29), so the check has been seen to read what it compares.
//! - An ignored write (rule 3's *"all others are simply ignored"*) is not
//!   dropped in silence: [`PartialPlan::ignored_writes`] lists every one, so
//!   the operator sees what the partial download will not write.

use std::fmt;

use super::load_control_memory::MemoryLoadStateMachine;
use super::load_state::LoadEvent;
use super::memory_download::{MemoryDownloadPlan, MemoryDownloadStep};
use super::properties::{ObjectIndex, PID_PROGRAM_VERSION};

/// Which parts a partial download writes: CP's *"Partial Download Type
/// (Parameters and/or Group Addresses)"*.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PartialDownloadParts {
    /// The application program's memory: parameters, and code where the
    /// product ships it.
    pub parameters: bool,
    /// The Group Address Table and the Group Object Association Table.
    pub group_addresses: bool,
}

/// A derived partial download.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartialPlan {
    /// The steps to run.
    pub plan: MemoryDownloadPlan,
    /// Writes of the complete plan that CP rule 3 ignores, as `(address,
    /// octets)`: application or PEI data in a segment that is not EEPROM.
    pub ignored_writes: Vec<(u16, usize)>,
}

/// Why no partial plan was derived.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartialPlanError {
    /// Neither part was selected.
    NothingSelected,
    /// The complete plan allocates no application program task segment, so
    /// the application the device must already carry is unknown.
    NoApplicationIdentity,
    /// A data write does not follow a segment allocation, so it cannot be
    /// told apart as application or table data.
    UnattributedWrite {
        /// The write's first address.
        address: u16,
    },
    /// After the rules, nothing is left that writes the device.
    NothingToWrite,
}

impl std::error::Error for PartialPlanError {}

impl fmt::Display for PartialPlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PartialPlanError::NothingSelected => {
                f.write_str("a partial download needs parameters, group addresses or both")
            }
            PartialPlanError::NoApplicationIdentity => f.write_str(
                "the complete download allocates no application program task segment, so the \
                 application the device must already carry is unknown",
            ),
            PartialPlanError::UnattributedWrite { address } => write!(
                f,
                "the write at {address:04X}h follows no segment allocation; it cannot be told \
                 apart as application or table data"
            ),
            PartialPlanError::NothingToWrite => {
                f.write_str("after CP §3.9.2.4's rules the partial download writes nothing")
            }
        }
    }
}

/// `[D]` MP §3.31.2: the segment type octet of an `AllocAbs*Seg` record.
const SEGMENT_TYPE_TASK: u8 = 0x02;
/// `[D]` MP §3.31.2: memory type `3`, EEPROM.
const MEMORY_TYPE_EEPROM: u8 = 3;

/// Derives the partial download of `parts` from `complete`.
pub fn derive_partial_plan(
    complete: &MemoryDownloadPlan,
    parts: PartialDownloadParts,
) -> Result<PartialPlan, PartialPlanError> {
    if !parts.parameters && !parts.group_addresses {
        return Err(PartialPlanError::NothingSelected);
    }
    let identity = application_identity(complete).ok_or(PartialPlanError::NoApplicationIdentity)?;

    let mut steps = Vec::new();
    let mut ignored_writes = Vec::new();
    // Whether the writes after the last allocation are kept; `None` while
    // no allocation has been seen since the last non-write step.
    let mut writes_kept: Option<bool> = None;
    let mut checks_inserted = false;

    for step in &complete.steps {
        if step.changes_device() && !checks_inserted {
            steps.extend(preconditions(complete, identity));
            checks_inserted = true;
        }
        match step {
            MemoryDownloadStep::WriteMemory { address, octets } => {
                let kept =
                    writes_kept.ok_or(PartialPlanError::UnattributedWrite { address: *address })?;
                if kept {
                    steps.push(step.clone());
                } else if parts.parameters && is_ignored_application_data(complete, *address) {
                    ignored_writes.push((*address, octets.len()));
                }
                continue;
            }
            MemoryDownloadStep::LoadRecord(record) => {
                let application = is_application(record.machine());
                let selected = if application {
                    parts.parameters
                } else {
                    parts.group_addresses
                };
                let octets = record.octets();
                let allocation = record.event() == LoadEvent::AdditionalLoadControls;
                writes_kept = match (allocation, octets[1]) {
                    (true, SEGMENT_TYPE_TASK) | (false, _) => None,
                    // Rule 3: application data survives as a memory write
                    // only in EEPROM.
                    (true, _) if application => Some(selected && octets[8] == MEMORY_TYPE_EEPROM),
                    (true, _) => Some(selected),
                };
                let keep = selected
                    && if application {
                        // Rule 1 drops the unloads, rule 3 the allocations.
                        !matches!(
                            record.event(),
                            LoadEvent::Unload | LoadEvent::AdditionalLoadControls
                        )
                    } else {
                        true
                    };
                if keep {
                    steps.push(step.clone());
                }
                continue;
            }
            MemoryDownloadStep::Connect
            | MemoryDownloadStep::CompareProperty { .. }
            | MemoryDownloadStep::RequireLoaded(_)
            | MemoryDownloadStep::Restart
            | MemoryDownloadStep::Disconnect => steps.push(step.clone()),
        }
        writes_kept = None;
    }

    if !steps.iter().any(|step| {
        matches!(
            step,
            MemoryDownloadStep::LoadRecord(_) | MemoryDownloadStep::WriteMemory { .. }
        )
    }) {
        return Err(PartialPlanError::NothingToWrite);
    }
    Ok(PartialPlan {
        plan: MemoryDownloadPlan {
            mask: complete.mask,
            manufacturer: complete.manufacturer,
            steps,
        },
        ignored_writes,
    })
}

fn is_application(machine: MemoryLoadStateMachine) -> bool {
    matches!(
        machine,
        MemoryLoadStateMachine::ApplicationProgram | MemoryLoadStateMachine::PeiProgram
    )
}

/// Whether the write at `address` lies in an application or PEI segment the
/// complete plan allocates outside EEPROM: CP rule 3's *"simply ignored"*.
fn is_ignored_application_data(complete: &MemoryDownloadPlan, address: u16) -> bool {
    complete.steps.iter().any(|step| match step {
        MemoryDownloadStep::LoadRecord(record)
            if is_application(record.machine())
                && record.event() == LoadEvent::AdditionalLoadControls
                && record.octets()[1] != SEGMENT_TYPE_TASK
                && record.octets()[8] != MEMORY_TYPE_EEPROM =>
        {
            let octets = record.octets();
            let start = u32::from(u16::from_be_bytes([octets[3], octets[4]]));
            let length = u32::from(u16::from_be_bytes([octets[5], octets[6]]));
            (start..start + length).contains(&u32::from(address))
        }
        _ => false,
    })
}

/// The application's `MM MM TT TT VV`, from the complete plan's application
/// program task segment (`[D]` MP §3.31.2: `L3 02h 00h SSSS PP MMMM TTTT VV`).
fn application_identity(complete: &MemoryDownloadPlan) -> Option<[u8; 5]> {
    complete.steps.iter().find_map(|step| match step {
        MemoryDownloadStep::LoadRecord(record)
            if record.machine() == MemoryLoadStateMachine::ApplicationProgram
                && record.event() == LoadEvent::AdditionalLoadControls
                && record.octets()[1] == SEGMENT_TYPE_TASK =>
        {
            let octets = record.octets();
            Some([octets[6], octets[7], octets[8], octets[9], octets[10]])
        }
        _ => None,
    })
}

/// The `[A]` checks run before the first write.
fn preconditions(complete: &MemoryDownloadPlan, identity: [u8; 5]) -> Vec<MemoryDownloadStep> {
    let mut checks = vec![MemoryDownloadStep::CompareProperty {
        object_index: ObjectIndex::APPLICATION_PROGRAM.octet(),
        property_id: PID_PROGRAM_VERSION,
        inline_data: identity.to_vec(),
    }];
    for machine in MemoryLoadStateMachine::ALL {
        let loaded_by_plan = complete.steps.iter().any(|step| {
            matches!(step, MemoryDownloadStep::LoadRecord(record)
                if record.machine() == machine && record.event() == LoadEvent::LoadCompleted)
        });
        if loaded_by_plan {
            checks.push(MemoryDownloadStep::RequireLoaded(machine));
        }
    }
    checks
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commissioning::load_control_memory::{
        abs_data_segment, abs_task_segment, event_record, AbsoluteSegment, SegmentMemoryType,
        TaskSegment,
    };
    use crate::commissioning::load_state::MaskVersion;

    use MemoryLoadStateMachine::{AddressTable, ApplicationProgram, AssociationTable};

    fn record(machine: MemoryLoadStateMachine, event: LoadEvent) -> MemoryDownloadStep {
        MemoryDownloadStep::LoadRecord(event_record(machine, event).expect("an event record"))
    }

    fn alloc(
        machine: MemoryLoadStateMachine,
        start: u16,
        length: u16,
        memory_type: SegmentMemoryType,
    ) -> MemoryDownloadStep {
        MemoryDownloadStep::LoadRecord(
            abs_data_segment(
                machine,
                AbsoluteSegment {
                    start,
                    length,
                    access: 0xFF,
                    memory_type,
                    checksum_control: false,
                },
            )
            .expect("a segment"),
        )
    }

    fn task(machine: MemoryLoadStateMachine, start: u16, identity: bool) -> MemoryDownloadStep {
        MemoryDownloadStep::LoadRecord(abs_task_segment(
            machine,
            TaskSegment {
                start,
                pei_type: if identity { 1 } else { 0 },
                manufacturer: if identity { 0x0083 } else { 0 },
                application: if identity { 0x0027 } else { 0 },
                version: if identity { 0x15 } else { 0 },
            },
        ))
    }

    fn write(address: u16, len: u8) -> MemoryDownloadStep {
        MemoryDownloadStep::WriteMemory {
            address,
            octets: (0..len).collect(),
        }
    }

    /// The shape `A-0027-15-0BAC`'s product procedure has: unload all, then
    /// per part load, allocate, write, task segment, load completed. The
    /// application has an EEPROM parameter segment and a RAM segment.
    fn complete() -> MemoryDownloadPlan {
        MemoryDownloadPlan {
            mask: MaskVersion(0x0701),
            manufacturer: 0x0083,
            steps: vec![
                MemoryDownloadStep::Connect,
                MemoryDownloadStep::CompareProperty {
                    object_index: 0,
                    property_id: 78,
                    inline_data: vec![0, 0, 0, 0, 1, 0x27],
                },
                record(AddressTable, LoadEvent::Unload),
                record(AssociationTable, LoadEvent::Unload),
                record(ApplicationProgram, LoadEvent::Unload),
                record(AddressTable, LoadEvent::StartLoading),
                alloc(AddressTable, 0x4000, 16, SegmentMemoryType::Eeprom),
                write(0x4000, 16),
                task(AddressTable, 0x4000, false),
                record(AddressTable, LoadEvent::LoadCompleted),
                record(AssociationTable, LoadEvent::StartLoading),
                alloc(AssociationTable, 0x4201, 8, SegmentMemoryType::Eeprom),
                write(0x4201, 8),
                record(AssociationTable, LoadEvent::LoadCompleted),
                record(ApplicationProgram, LoadEvent::StartLoading),
                alloc(ApplicationProgram, 0x4400, 32, SegmentMemoryType::Eeprom),
                write(0x4400, 20),
                write(0x4418, 8),
                alloc(ApplicationProgram, 0x00C0, 4, SegmentMemoryType::Ram),
                write(0x00C0, 4),
                task(ApplicationProgram, 0x4400, true),
                record(ApplicationProgram, LoadEvent::LoadCompleted),
                MemoryDownloadStep::Restart,
                MemoryDownloadStep::Disconnect,
            ],
        }
    }

    fn text(plan: &MemoryDownloadPlan) -> Vec<String> {
        plan.steps.iter().map(ToString::to_string).collect()
    }

    const BOTH: PartialDownloadParts = PartialDownloadParts {
        parameters: true,
        group_addresses: true,
    };

    #[test]
    fn both_parts_drop_only_the_application_unload_and_allocations() {
        let partial = derive_partial_plan(&complete(), BOTH).expect("a partial plan");
        let expected: Vec<MemoryDownloadStep> = vec![
            MemoryDownloadStep::Connect,
            MemoryDownloadStep::CompareProperty {
                object_index: 0,
                property_id: 78,
                inline_data: vec![0, 0, 0, 0, 1, 0x27],
            },
            MemoryDownloadStep::CompareProperty {
                object_index: 3,
                property_id: 13,
                inline_data: vec![0x00, 0x83, 0x00, 0x27, 0x15],
            },
            MemoryDownloadStep::RequireLoaded(AddressTable),
            MemoryDownloadStep::RequireLoaded(AssociationTable),
            MemoryDownloadStep::RequireLoaded(ApplicationProgram),
            record(AddressTable, LoadEvent::Unload),
            record(AssociationTable, LoadEvent::Unload),
            record(AddressTable, LoadEvent::StartLoading),
            alloc(AddressTable, 0x4000, 16, SegmentMemoryType::Eeprom),
            write(0x4000, 16),
            task(AddressTable, 0x4000, false),
            record(AddressTable, LoadEvent::LoadCompleted),
            record(AssociationTable, LoadEvent::StartLoading),
            alloc(AssociationTable, 0x4201, 8, SegmentMemoryType::Eeprom),
            write(0x4201, 8),
            record(AssociationTable, LoadEvent::LoadCompleted),
            record(ApplicationProgram, LoadEvent::StartLoading),
            write(0x4400, 20),
            write(0x4418, 8),
            record(ApplicationProgram, LoadEvent::LoadCompleted),
            MemoryDownloadStep::Restart,
            MemoryDownloadStep::Disconnect,
        ];
        assert_eq!(
            text(&partial.plan),
            text(&MemoryDownloadPlan {
                steps: expected.clone(),
                ..complete()
            })
        );
        assert_eq!(partial.plan.steps, expected);
        assert_eq!(partial.ignored_writes, vec![(0x00C0, 4)]);
    }

    #[test]
    fn parameters_only_leave_the_tables_alone() {
        let partial = derive_partial_plan(
            &complete(),
            PartialDownloadParts {
                parameters: true,
                group_addresses: false,
            },
        )
        .expect("a partial plan");
        let touched: Vec<_> = partial
            .plan
            .steps
            .iter()
            .filter_map(|step| match step {
                MemoryDownloadStep::LoadRecord(record) => Some(record.machine()),
                _ => None,
            })
            .collect();
        assert_eq!(touched, vec![ApplicationProgram, ApplicationProgram]);
        let writes: Vec<_> = partial
            .plan
            .steps
            .iter()
            .filter_map(|step| match step {
                MemoryDownloadStep::WriteMemory { address, .. } => Some(*address),
                _ => None,
            })
            .collect();
        assert_eq!(writes, vec![0x4400, 0x4418]);
        assert_eq!(partial.ignored_writes, vec![(0x00C0, 4)]);
    }

    #[test]
    fn group_addresses_only_never_touch_the_application_program() {
        let partial = derive_partial_plan(
            &complete(),
            PartialDownloadParts {
                parameters: false,
                group_addresses: true,
            },
        )
        .expect("a partial plan");
        for step in &partial.plan.steps {
            match step {
                MemoryDownloadStep::LoadRecord(record) => {
                    assert!(!is_application(record.machine()), "{step}")
                }
                MemoryDownloadStep::WriteMemory { address, .. } => {
                    assert!(*address < 0x4400, "{step}")
                }
                _ => {}
            }
        }
        assert!(partial.ignored_writes.is_empty());
        // The application check still runs: tables of another application
        // would be as wrong as its parameters.
        assert!(partial
            .plan
            .steps
            .contains(&MemoryDownloadStep::CompareProperty {
                object_index: 3,
                property_id: 13,
                inline_data: vec![0x00, 0x83, 0x00, 0x27, 0x15],
            }));
    }

    #[test]
    fn the_checks_come_before_the_first_write() {
        let partial = derive_partial_plan(&complete(), BOTH).expect("a partial plan");
        let first_write = partial
            .plan
            .steps
            .iter()
            .position(MemoryDownloadStep::changes_device)
            .expect("a write");
        let last_check = partial
            .plan
            .steps
            .iter()
            .rposition(|step| {
                matches!(
                    step,
                    MemoryDownloadStep::CompareProperty { .. }
                        | MemoryDownloadStep::RequireLoaded(_)
                )
            })
            .expect("a check");
        assert!(last_check < first_write);
    }

    #[test]
    fn a_partial_plan_writes_nothing_the_complete_one_does_not() {
        let complete = complete();
        for parts in [
            BOTH,
            PartialDownloadParts {
                parameters: true,
                group_addresses: false,
            },
            PartialDownloadParts {
                parameters: false,
                group_addresses: true,
            },
        ] {
            let partial = derive_partial_plan(&complete, parts).expect("a partial plan");
            for step in partial
                .plan
                .steps
                .iter()
                .filter(|step| step.changes_device())
            {
                assert!(complete.steps.contains(step), "{parts:?}: {step}");
            }
        }
    }

    #[test]
    fn nothing_selected_is_refused() {
        assert_eq!(
            derive_partial_plan(
                &complete(),
                PartialDownloadParts {
                    parameters: false,
                    group_addresses: false,
                }
            ),
            Err(PartialPlanError::NothingSelected)
        );
    }

    #[test]
    fn a_plan_without_an_application_task_segment_is_refused() {
        let mut plan = complete();
        plan.steps
            .retain(|step| *step != task(ApplicationProgram, 0x4400, true));
        assert_eq!(
            derive_partial_plan(&plan, BOTH),
            Err(PartialPlanError::NoApplicationIdentity)
        );
    }

    #[test]
    fn a_write_after_no_allocation_is_refused() {
        let mut plan = complete();
        plan.steps.insert(2, write(0x5000, 2));
        assert_eq!(
            derive_partial_plan(&plan, BOTH),
            Err(PartialPlanError::UnattributedWrite { address: 0x5000 })
        );
    }

    #[test]
    fn a_selection_that_leaves_nothing_to_write_is_refused() {
        let mut plan = complete();
        plan.steps.retain(|step| match step {
            MemoryDownloadStep::LoadRecord(record) => is_application(record.machine()),
            MemoryDownloadStep::WriteMemory { address, .. } => {
                *address >= 0x4400 || *address < 0x4000
            }
            _ => true,
        });
        assert_eq!(
            derive_partial_plan(
                &plan,
                PartialDownloadParts {
                    parameters: false,
                    group_addresses: true,
                }
            ),
            Err(PartialPlanError::NothingToWrite)
        );
    }
}
