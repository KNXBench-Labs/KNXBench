//! Pre-write memory snapshots and their restore plans.
//!
//! A download overwrites memory the operator may want back: the previous
//! configuration of a device nobody documented, or the one that worked.
//! [`DeviceBackup`] is what the executor reads, in the same connection,
//! after the identity checks and before the first step that changes the
//! device: every region the plan writes, octet for octet, and the load
//! state of every machine the plan touches.
//!
//! [`restore_plan`] turns a backup and a plan *of the same shape* back into
//! a plan: the same steps, the same load procedure, the backed-up octets in
//! place of the new ones. It refuses when the shapes differ (another
//! application, another partial selection, another device), because then
//! the load procedure's allocations would not describe the old memory.
//!
//! [`download_changes`] compares a backup with the plan it was read for:
//! the octets a download would change, before anything is written. Read
//! without a download, a backup is that preview.
//!
//! A restore also needs the application the bytes belonged to. The plan's
//! load procedure describes the *new* application, so [`restore_plan`]
//! refuses unless the backup [`PriorApplication::Recorded`] the same five
//! octets (`MM MM TT TT VV`, object index 3, PID 13) the plan names. The
//! one exception is a plan that already carries an exact five-octet
//! identity precondition: its executor checks the device itself. This is a
//! necessary guard only; it does not prove that the bytes fit the plan's
//! allocations, that a physical device is the same exemplar, or that a
//! restore recovers a device.
//!
//! What a backup does not hold, and a restore therefore cannot bring back:
//! memory outside the regions the plan writes (masked octets are never
//! written, so they need no backup), properties, and a load state other
//! than `Loaded` — a restore always ends `Loaded`, like any download.

use std::fmt;

use super::load_control_memory::MemoryLoadStateMachine;
use super::load_state::{LoadState, MaskVersion};
use super::memory_download::{machines, ExemplarSerial, MemoryDownloadPlan, MemoryDownloadStep};
use super::partial_memory_download::application_identity;
use super::properties::{ObjectIndex, PID_PROGRAM_VERSION, PROGRAM_VERSION_OCTETS};
use crate::IndividualAddress;

/// The application a device held before the backup, as one exact
/// observation of object index 3, PID 13.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriorApplication {
    /// Exactly five octets, `MM MM TT TT VV`, answered before any write.
    Recorded([u8; PROGRAM_VERSION_OCTETS]),
    /// No usable observation: not read (preview, unbacked run, older
    /// file), refused, unsupported, or not five octets. Never a guess.
    NotRecorded,
}

/// The Device Object serial (PID 11) a device reported before the backup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExemplarEvidence {
    /// Exactly six octets, answered with count 1 before any write.
    Recorded(ExemplarSerial),
    /// No usable observation: not read (preview, unbacked run, format 1/2 file), refused,
    /// malformed or unanswered. Such a backup is never restorable.
    NotRecorded,
}

/// One region a plan writes, as the device held it before.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupRegion {
    /// The first address.
    pub address: u16,
    /// The octets read there, as many as the plan writes.
    pub octets: Vec<u8>,
}

/// What a device held where a download was about to write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceBackup {
    /// The device.
    pub target: IndividualAddress,
    /// Its mask, as read and checked before the backup.
    pub mask: MaskVersion,
    /// Its manufacturer, as read and checked before the backup.
    pub manufacturer: u16,
    /// Each machine the plan touches, and the state it was in.
    pub load_states: Vec<(MemoryLoadStateMachine, LoadState)>,
    /// Every region the plan writes, in plan order.
    pub regions: Vec<BackupRegion>,
    /// The application the device held when the backup was taken.
    pub prior_application: PriorApplication,
    /// The serial the device reported when the backup was taken.
    pub exemplar_serial: ExemplarEvidence,
}

impl DeviceBackup {
    /// Octets held in the backup.
    pub fn octets(&self) -> usize {
        self.regions.iter().map(|region| region.octets.len()).sum()
    }

    /// Whether every machine was `Loaded` when the backup was taken. A
    /// backup of a device that was not is a record of a broken state, and
    /// restoring it restores that memory, not a working application.
    pub fn was_loaded(&self) -> bool {
        !self.load_states.is_empty()
            && self
                .load_states
                .iter()
                .all(|(_, state)| *state == LoadState::Loaded)
    }
}

/// The regions a plan writes, as `(address, length)`, in plan order.
pub fn written_regions(plan: &MemoryDownloadPlan) -> Vec<(u16, usize)> {
    plan.steps
        .iter()
        .filter_map(|step| match step {
            MemoryDownloadStep::WriteMemory { address, octets } => Some((*address, octets.len())),
            _ => None,
        })
        .collect()
}

/// Why no restore plan was derived.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RestoreError {
    /// The backup is of another device.
    OtherDevice {
        /// The plan's.
        plan: IndividualAddress,
        /// The backup's.
        backup: IndividualAddress,
    },
    /// The backup's mask or manufacturer is not the plan's.
    OtherProduct,
    /// The backup's regions are not the regions the plan writes.
    OtherShape {
        /// The plan's regions, `(address, length)`.
        plan: Vec<(u16, usize)>,
        /// The backup's.
        backup: Vec<(u16, usize)>,
    },
    /// A state is missing, duplicated or for a machine outside this plan.
    OtherLoadStates {
        /// The machines addressed by the plan.
        plan: Vec<MemoryLoadStateMachine>,
        /// The machines recorded in the backup (duplicates preserved).
        backup: Vec<MemoryLoadStateMachine>,
    },
    /// The plan names no single five-octet application identity, so no
    /// backup can be shown to belong to it.
    PlanApplicationUnknown,
    /// The backup holds no exact observation of the application it was
    /// taken from, and the plan has no identity precondition of its own.
    PriorApplicationNotRecorded {
        /// The plan's application identity.
        plan: [u8; 5],
    },
    /// The backup was taken from another application than the plan's.
    OtherApplication {
        /// The plan's.
        plan: [u8; 5],
        /// The backup's.
        backup: [u8; 5],
    },
    /// The backup holds no serial observation, so no replacement can be told from the exemplar.
    ExemplarNotRecorded,
    /// The recorded serial is all-zero or all-FF: a placeholder that identifies nothing.
    ExemplarNotBindable,
    /// The plan already carries an exemplar precondition; a caller-supplied guard is not trusted.
    PlanAlreadyGuarded,
    /// The plan changes nothing, so there is no point to guard before.
    PlanChangesNothing,
}

impl std::error::Error for RestoreError {}

impl fmt::Display for RestoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OtherDevice { plan, backup } => {
                write!(
                    f,
                    "the backup is of device {backup}, the plan is for {plan}"
                )
            }
            Self::OtherProduct => {
                f.write_str("the backup's mask or manufacturer is not the plan's")
            }
            Self::OtherShape { plan, backup } => write!(
                f,
                "the backup holds regions {} but the plan writes {}; restore only with the \
                 plan of the same application and the same partial selection",
                regions(backup),
                regions(plan)
            ),
            Self::OtherLoadStates { plan, backup } => write!(
                f,
                "the backup records load states for {backup:?} but the plan addresses {plan:?}"
            ),
            Self::PlanApplicationUnknown => f.write_str(
                "the plan names no single five-octet application identity; a backup cannot be \
                 shown to belong to it",
            ),
            Self::PriorApplicationNotRecorded { plan } => write!(
                f,
                "the backup does not record which application it was taken from (plan: \
                 {plan:02X?}); restore refused. Backups taken before this was recorded, or \
                 whose device did not answer it, cannot be restored",
            ),
            Self::OtherApplication { plan, backup } => write!(
                f,
                "the backup was taken from application {backup:02X?} but the plan loads \
                 {plan:02X?}; its octets do not fit that load procedure"
            ),
            Self::ExemplarNotRecorded => f.write_str(
                "the backup does not record the device's serial number; restore refused. Backups \
                 taken before this was recorded, or whose device did not answer it, cannot be \
                 restored",
            ),
            Self::ExemplarNotBindable => f.write_str(
                "the backup's recorded serial number is all-zero or all-FF and identifies no \
                 device; restore refused",
            ),
            Self::PlanAlreadyGuarded => f.write_str(
                "the plan already carries an exemplar check; restore derivation adds its own",
            ),
            Self::PlanChangesNothing => f.write_str(
                "the plan changes nothing on the device, so there is nothing to restore",
            ),
        }
    }
}

fn regions(list: &[(u16, usize)]) -> String {
    list.iter()
        .map(|(address, length)| format!("{address:04X}h+{length}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// One run of consecutive octets a download would change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OctetChange {
    /// The first address.
    pub address: u16,
    /// What the device holds there now.
    pub device: Vec<u8>,
    /// What the plan would write.
    pub planned: Vec<u8>,
}

/// What a download of `plan` would change on a device that holds `held`:
/// every run of consecutive octets where the two differ, in plan order.
/// `held` is read the way a backup is ([`DeviceBackup`]), so the same
/// device, product and shape are required as for [`restore_plan`].
pub fn download_changes(
    plan: &MemoryDownloadPlan,
    target: IndividualAddress,
    held: &DeviceBackup,
) -> Result<Vec<OctetChange>, RestoreError> {
    check_same_shape(plan, target, held)?;
    let mut changes = Vec::new();
    let planned_runs = plan.steps.iter().filter_map(|step| match step {
        MemoryDownloadStep::WriteMemory { address, octets } => Some((*address, octets)),
        _ => None,
    });
    for ((address, planned), region) in planned_runs.zip(&held.regions) {
        let mut open: Option<OctetChange> = None;
        for (offset, (&want, &have)) in planned.iter().zip(&region.octets).enumerate() {
            if want == have {
                changes.extend(open.take());
                continue;
            }
            let change = open.get_or_insert_with(|| OctetChange {
                address: address + offset as u16,
                device: Vec::new(),
                planned: Vec::new(),
            });
            change.device.push(have);
            change.planned.push(want);
        }
        changes.extend(open);
    }
    Ok(changes)
}

/// The device, product and written regions `held` must share with `plan`
/// before its octets can stand in for (or be compared with) the plan's.
fn check_same_shape(
    plan: &MemoryDownloadPlan,
    target: IndividualAddress,
    held: &DeviceBackup,
) -> Result<(), RestoreError> {
    if held.target != target {
        return Err(RestoreError::OtherDevice {
            plan: target,
            backup: held.target,
        });
    }
    if held.mask != plan.mask || held.manufacturer != plan.manufacturer {
        return Err(RestoreError::OtherProduct);
    }
    let wanted = written_regions(plan);
    let have: Vec<(u16, usize)> = held
        .regions
        .iter()
        .map(|region| (region.address, region.octets.len()))
        .collect();
    if wanted != have {
        return Err(RestoreError::OtherShape {
            plan: wanted,
            backup: have,
        });
    }
    Ok(())
}

/// Whether the plan itself compares the device's application against its
/// own identity before any change: object 3, PID 13, exactly five octets,
/// equal to `identity`, ahead of the first step that changes the device. A
/// padded or longer compare proves nothing here, and neither does one that
/// only runs after a write.
fn has_identity_precondition(plan: &MemoryDownloadPlan, identity: [u8; 5]) -> bool {
    plan.steps
        .iter()
        .take_while(|step| !step.changes_device())
        .any(|step| {
            matches!(step, MemoryDownloadStep::CompareProperty {
                object_index, property_id, inline_data,
            } if *object_index == ObjectIndex::APPLICATION_PROGRAM.octet()
                && *property_id == PID_PROGRAM_VERSION
                && inline_data.as_slice() == identity)
        })
}

/// The backed-up octets belong to the application the plan loads. A
/// recorded identity must always equal the plan's, whatever the plan's
/// own checks say; an unrecorded one is accepted only behind the plan's
/// exact identity precondition (a partial plan), never on a flag.
fn check_prior_application(
    plan: &MemoryDownloadPlan,
    backup: &DeviceBackup,
) -> Result<(), RestoreError> {
    let identity = application_identity(plan).ok_or(RestoreError::PlanApplicationUnknown)?;
    match backup.prior_application {
        PriorApplication::Recorded(prior) if prior == identity => Ok(()),
        PriorApplication::Recorded(prior) => Err(RestoreError::OtherApplication {
            plan: identity,
            backup: prior,
        }),
        PriorApplication::NotRecorded if has_identity_precondition(plan, identity) => Ok(()),
        PriorApplication::NotRecorded => {
            Err(RestoreError::PriorApplicationNotRecorded { plan: identity })
        }
    }
}

/// `plan` with the backed-up octets in place of its own. The steps, their
/// order and every check stay exactly the plan's.
pub fn restore_plan(
    plan: &MemoryDownloadPlan,
    target: IndividualAddress,
    backup: &DeviceBackup,
) -> Result<MemoryDownloadPlan, RestoreError> {
    check_same_shape(plan, target, backup)?;
    let plan_machines = machines(plan);
    let mut backup_machines: Vec<_> = backup
        .load_states
        .iter()
        .map(|(machine, _)| *machine)
        .collect();
    backup_machines.sort();
    if plan_machines != backup_machines {
        return Err(RestoreError::OtherLoadStates {
            plan: plan_machines,
            backup: backup_machines,
        });
    }
    check_prior_application(plan, backup)?;
    let serial = match backup.exemplar_serial {
        ExemplarEvidence::Recorded(serial) if serial.is_bindable() => serial,
        ExemplarEvidence::Recorded(_) => return Err(RestoreError::ExemplarNotBindable),
        ExemplarEvidence::NotRecorded => return Err(RestoreError::ExemplarNotRecorded),
    };
    if plan
        .steps
        .iter()
        .any(|step| matches!(step, MemoryDownloadStep::RequireExemplar(_)))
    {
        return Err(RestoreError::PlanAlreadyGuarded);
    }
    let first_change = plan
        .steps
        .iter()
        .position(MemoryDownloadStep::changes_device)
        .ok_or(RestoreError::PlanChangesNothing)?;
    let mut regions = backup.regions.iter();
    let mut steps: Vec<MemoryDownloadStep> = plan
        .steps
        .iter()
        .map(|step| match step {
            MemoryDownloadStep::WriteMemory { address, .. } => MemoryDownloadStep::WriteMemory {
                address: *address,
                octets: regions
                    .next()
                    .expect("the shapes were compared")
                    .octets
                    .clone(),
            },
            other => other.clone(),
        })
        .collect();
    steps.insert(first_change, MemoryDownloadStep::RequireExemplar(serial));
    Ok(MemoryDownloadPlan {
        mask: plan.mask,
        manufacturer: plan.manufacturer,
        steps,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commissioning::load_control_memory::{abs_task_segment, event_record, TaskSegment};
    use crate::commissioning::load_state::LoadEvent;
    use crate::commissioning::memory_download::source_procedure_descriptions;

    const IDENTITY: [u8; 5] = [0x00, 0x83, 0x00, 0x27, 0x15];
    const SERIAL: [u8; 6] = [0x00, 0x83, 0x12, 0x34, 0x56, 0x78];

    fn address(text: &str) -> IndividualAddress {
        text.parse().unwrap()
    }

    fn plan() -> MemoryDownloadPlan {
        MemoryDownloadPlan {
            mask: MaskVersion(0x0701),
            manufacturer: 0x0083,
            steps: vec![
                MemoryDownloadStep::Connect,
                MemoryDownloadStep::CompareProperty {
                    object_index: 3,
                    property_id: 13,
                    inline_data: IDENTITY.to_vec(),
                },
                MemoryDownloadStep::LoadRecord(
                    event_record(
                        MemoryLoadStateMachine::AddressTable,
                        LoadEvent::StartLoading,
                    )
                    .unwrap(),
                ),
                MemoryDownloadStep::WriteMemory {
                    address: 0x4000,
                    octets: vec![1, 2],
                },
                MemoryDownloadStep::Restart,
                MemoryDownloadStep::WriteMemory {
                    address: 0x4400,
                    octets: vec![3, 4, 5],
                },
                MemoryDownloadStep::Disconnect,
            ],
        }
    }

    fn backup() -> DeviceBackup {
        DeviceBackup {
            target: address("1.1.67"),
            mask: MaskVersion(0x0701),
            manufacturer: 0x0083,
            load_states: vec![(MemoryLoadStateMachine::AddressTable, LoadState::Loaded)],
            regions: vec![
                BackupRegion {
                    address: 0x4000,
                    octets: vec![9, 8],
                },
                BackupRegion {
                    address: 0x4400,
                    octets: vec![7, 6, 5],
                },
            ],
            prior_application: PriorApplication::Recorded(IDENTITY),
            exemplar_serial: ExemplarEvidence::Recorded(ExemplarSerial::new(SERIAL)),
        }
    }

    /// [`plan`] as a complete download: no precondition, the identity only
    /// in the application's task segment.
    fn complete_plan(application: u16) -> MemoryDownloadPlan {
        let mut complete = plan();
        complete.steps.remove(1);
        complete.steps.insert(
            2,
            MemoryDownloadStep::LoadRecord(abs_task_segment(
                MemoryLoadStateMachine::ApplicationProgram,
                TaskSegment {
                    start: 0x4400,
                    pei_type: 1,
                    manufacturer: 0x0083,
                    application,
                    version: 0x15,
                },
            )),
        );
        complete
    }

    fn complete_backup() -> DeviceBackup {
        let mut backup = backup();
        backup.load_states.push((
            MemoryLoadStateMachine::ApplicationProgram,
            LoadState::Loaded,
        ));
        backup
    }

    #[test]
    fn a_restore_is_the_plan_with_the_old_octets() {
        let restored = restore_plan(&plan(), address("1.1.67"), &backup()).unwrap();
        let mut expected: Vec<MemoryDownloadStep> = plan()
            .steps
            .into_iter()
            .map(|step| match step {
                MemoryDownloadStep::WriteMemory {
                    address: 0x4000, ..
                } => MemoryDownloadStep::WriteMemory {
                    address: 0x4000,
                    octets: vec![9, 8],
                },
                MemoryDownloadStep::WriteMemory {
                    address: 0x4400, ..
                } => MemoryDownloadStep::WriteMemory {
                    address: 0x4400,
                    octets: vec![7, 6, 5],
                },
                other => other,
            })
            .collect();
        // The exemplar guard sits right before the first step that changes the device.
        expected.insert(
            2,
            MemoryDownloadStep::RequireExemplar(ExemplarSerial::new(SERIAL)),
        );
        assert_eq!(restored.steps, expected);
        assert_eq!(restored.data_octets(), plan().data_octets());
    }

    #[test]
    fn a_backup_of_another_device_or_product_is_refused() {
        assert!(matches!(
            restore_plan(&plan(), address("1.1.68"), &backup()),
            Err(RestoreError::OtherDevice { .. })
        ));
        let mut other = backup();
        other.manufacturer = 0x0002;
        assert_eq!(
            restore_plan(&plan(), address("1.1.67"), &other),
            Err(RestoreError::OtherProduct)
        );
        let mut other = backup();
        other.mask = MaskVersion(0x0705);
        assert_eq!(
            restore_plan(&plan(), address("1.1.67"), &other),
            Err(RestoreError::OtherProduct)
        );
    }

    #[test]
    fn a_backup_of_another_shape_is_refused_by_region() {
        let mut shorter = backup();
        shorter.regions[1].octets.pop();
        let error = restore_plan(&plan(), address("1.1.67"), &shorter).unwrap_err();
        assert!(error.to_string().contains("4400h+2"), "{error}");
        let mut fewer = backup();
        fewer.regions.pop();
        assert!(matches!(
            restore_plan(&plan(), address("1.1.67"), &fewer),
            Err(RestoreError::OtherShape { .. })
        ));
        let mut moved = backup();
        moved.regions[0].address = 0x4001;
        assert!(matches!(
            restore_plan(&plan(), address("1.1.67"), &moved),
            Err(RestoreError::OtherShape { .. })
        ));
    }

    #[test]
    fn restore_refuses_missing_load_state_even_when_memory_regions_match() {
        let mut second_machine = plan();
        second_machine.steps.insert(
            2,
            MemoryDownloadStep::LoadRecord(
                event_record(
                    MemoryLoadStateMachine::AssociationTable,
                    LoadEvent::StartLoading,
                )
                .unwrap(),
            ),
        );
        assert!(matches!(
            restore_plan(&second_machine, address("1.1.67"), &backup()),
            Err(RestoreError::OtherLoadStates { .. })
        ));
    }

    #[test]
    fn restore_refuses_duplicate_load_state() {
        let mut duplicate = backup();
        duplicate.load_states.push(duplicate.load_states[0]);
        assert!(matches!(
            restore_plan(&plan(), address("1.1.67"), &duplicate),
            Err(RestoreError::OtherLoadStates { .. })
        ));
    }

    #[test]
    fn restore_refuses_extra_load_state() {
        let mut extra = backup();
        extra.load_states.push((
            MemoryLoadStateMachine::ApplicationProgram,
            LoadState::Loaded,
        ));
        assert!(matches!(
            restore_plan(&plan(), address("1.1.67"), &extra),
            Err(RestoreError::OtherLoadStates { .. })
        ));
    }

    #[test]
    fn the_changes_are_the_runs_where_plan_and_device_differ() {
        let mut held = backup();
        held.regions[0].octets = vec![1, 2]; // 4000h: as planned
        held.regions[1].octets = vec![3, 9, 9]; // 4400h: two octets differ
        let changes = download_changes(&plan(), address("1.1.67"), &held).unwrap();
        assert_eq!(
            changes,
            vec![OctetChange {
                address: 0x4401,
                device: vec![9, 9],
                planned: vec![4, 5],
            }]
        );
    }

    #[test]
    fn separate_differences_stay_separate_runs() {
        let mut held = backup();
        held.regions[0].octets = vec![0, 2];
        held.regions[1].octets = vec![0, 4, 0];
        let changes = download_changes(&plan(), address("1.1.67"), &held).unwrap();
        let at: Vec<(u16, usize)> = changes
            .iter()
            .map(|change| (change.address, change.planned.len()))
            .collect();
        assert_eq!(at, vec![(0x4000, 1), (0x4400, 1), (0x4402, 1)]);
    }

    #[test]
    fn a_device_that_holds_the_plan_has_no_changes() {
        let mut held = backup();
        held.regions[0].octets = vec![1, 2];
        held.regions[1].octets = vec![3, 4, 5];
        assert_eq!(
            download_changes(&plan(), address("1.1.67"), &held),
            Ok(vec![])
        );
    }

    #[test]
    fn changes_are_only_computed_for_the_same_device_product_and_shape() {
        assert!(matches!(
            download_changes(&plan(), address("1.1.68"), &backup()),
            Err(RestoreError::OtherDevice { .. })
        ));
        let mut other = backup();
        other.mask = MaskVersion(0x0705);
        assert_eq!(
            download_changes(&plan(), address("1.1.67"), &other),
            Err(RestoreError::OtherProduct)
        );
        let mut shorter = backup();
        shorter.regions[1].octets.pop();
        assert!(matches!(
            download_changes(&plan(), address("1.1.67"), &shorter),
            Err(RestoreError::OtherShape { .. })
        ));
    }

    #[test]
    fn a_backup_knows_its_size_and_whether_it_was_loaded() {
        assert_eq!(backup().octets(), 5);
        assert!(backup().was_loaded());
        let mut unloaded = backup();
        unloaded.load_states[0].1 = LoadState::Unloaded;
        assert!(!unloaded.was_loaded());
        let mut none = backup();
        none.load_states.clear();
        assert!(!none.was_loaded());
        assert_eq!(written_regions(&plan()), vec![(0x4000, 2), (0x4400, 3)]);
    }

    #[test]
    fn a_complete_plan_restores_only_the_application_it_was_read_from() {
        let target = address("1.1.67");
        let restored = restore_plan(&complete_plan(0x0027), target, &complete_backup());
        assert!(restored.is_ok(), "{restored:?}");

        let mut other = complete_backup();
        other.prior_application = PriorApplication::Recorded([0x00, 0x83, 0x00, 0x27, 0x14]);
        assert_eq!(
            restore_plan(&complete_plan(0x0027), target, &other),
            Err(RestoreError::OtherApplication {
                plan: IDENTITY,
                backup: [0x00, 0x83, 0x00, 0x27, 0x14],
            })
        );

        let mut unrecorded = complete_backup();
        unrecorded.prior_application = PriorApplication::NotRecorded;
        let error = restore_plan(&complete_plan(0x0027), target, &unrecorded).unwrap_err();
        assert_eq!(
            error,
            RestoreError::PriorApplicationNotRecorded { plan: IDENTITY }
        );
        assert!(error.to_string().contains("does not record"), "{error}");
    }

    #[test]
    fn a_plan_without_a_single_identity_refuses_every_backup() {
        let target = address("1.1.67");
        let mut none = plan();
        none.steps.remove(1);
        assert_eq!(
            restore_plan(&none, target, &backup()),
            Err(RestoreError::PlanApplicationUnknown)
        );
        // Two identities that disagree are not guessed between.
        let mut conflicting = plan();
        conflicting.steps.insert(
            2,
            MemoryDownloadStep::CompareProperty {
                object_index: 3,
                property_id: 13,
                inline_data: vec![0x00, 0x83, 0x00, 0x27, 0x16],
            },
        );
        assert_eq!(
            restore_plan(&conflicting, target, &backup()),
            Err(RestoreError::PlanApplicationUnknown)
        );
        // A padded compare is not a five-octet identity, and no prefix rule
        // turns it into one.
        let mut padded = plan();
        padded.steps[1] = MemoryDownloadStep::CompareProperty {
            object_index: 3,
            property_id: 13,
            inline_data: [&IDENTITY[..], &[0, 0, 0]].concat(),
        };
        assert_eq!(
            restore_plan(&padded, target, &backup()),
            Err(RestoreError::PlanApplicationUnknown)
        );
    }

    #[test]
    fn an_identity_precondition_admits_an_unrecorded_backup_but_never_a_conflicting_one() {
        let target = address("1.1.67");
        let mut legacy = backup();
        legacy.prior_application = PriorApplication::NotRecorded;
        assert!(restore_plan(&plan(), target, &legacy).is_ok());
        let mut forged = backup();
        forged.prior_application = PriorApplication::Recorded([0, 0x83, 0, 0x27, 0x14]);
        assert!(matches!(
            restore_plan(&plan(), target, &forged),
            Err(RestoreError::OtherApplication { .. })
        ));
    }

    #[test]
    fn an_identity_compare_after_the_first_change_is_no_precondition() {
        let target = address("1.1.67");
        let mut legacy = backup();
        legacy.prior_application = PriorApplication::NotRecorded;
        // The same exact compare, moved behind the first write: whatever it
        // would prove no longer protects the bytes that write replaced.
        let mut late = plan();
        let compare = late.steps.remove(1);
        let first_write = late
            .steps
            .iter()
            .position(MemoryDownloadStep::changes_device)
            .expect("the plan writes");
        late.steps.insert(first_write + 1, compare);
        assert_eq!(
            restore_plan(&late, target, &legacy),
            Err(RestoreError::PriorApplicationNotRecorded { plan: IDENTITY })
        );
        // Recorded evidence does not depend on the compare's position.
        assert!(restore_plan(&late, target, &backup()).is_ok());
    }

    #[test]
    fn matching_identity_does_not_bypass_the_shape_checks() {
        let mut moved = complete_backup();
        moved.regions[0].address = 0x4001;
        assert!(matches!(
            restore_plan(&complete_plan(0x0027), address("1.1.67"), &moved),
            Err(RestoreError::OtherShape { .. })
        ));
        assert!(matches!(
            restore_plan(
                &complete_plan(0x0027),
                address("1.1.68"),
                &complete_backup()
            ),
            Err(RestoreError::OtherDevice { .. })
        ));
    }

    #[test]
    fn a_restore_needs_a_bindable_recorded_serial() {
        let mut missing = backup();
        missing.exemplar_serial = ExemplarEvidence::NotRecorded;
        assert_eq!(
            restore_plan(&plan(), address("1.1.67"), &missing),
            Err(RestoreError::ExemplarNotRecorded)
        );
        for placeholder in [[0x00; 6], [0xFF; 6]] {
            let mut sentinel = backup();
            sentinel.exemplar_serial = ExemplarEvidence::Recorded(ExemplarSerial::new(placeholder));
            assert_eq!(
                restore_plan(&plan(), address("1.1.67"), &sentinel),
                Err(RestoreError::ExemplarNotBindable)
            );
        }
    }

    #[test]
    fn existing_guards_still_come_before_the_serial() {
        let mut other = backup();
        other.exemplar_serial = ExemplarEvidence::NotRecorded;
        assert!(matches!(
            restore_plan(&plan(), address("1.1.68"), &other),
            Err(RestoreError::OtherDevice { .. })
        ));
        other.prior_application = PriorApplication::Recorded([0, 0x83, 0, 0x27, 0x14]);
        assert!(matches!(
            restore_plan(&plan(), address("1.1.67"), &other),
            Err(RestoreError::OtherApplication { .. })
        ));
    }

    #[test]
    fn a_plan_with_a_guard_or_without_a_change_is_refused() {
        let restored = restore_plan(&plan(), address("1.1.67"), &backup()).unwrap();
        assert_eq!(
            restore_plan(&restored, address("1.1.67"), &backup()),
            Err(RestoreError::PlanAlreadyGuarded)
        );
        let mut inert = plan();
        inert.steps.retain(|step| {
            !step.changes_device() && !matches!(step, MemoryDownloadStep::WriteMemory { .. })
        });
        let mut empty_backup = backup();
        empty_backup.regions.clear();
        empty_backup.load_states.clear();
        assert_eq!(
            restore_plan(&inert, address("1.1.67"), &empty_backup),
            Err(RestoreError::PlanChangesNothing)
        );
    }

    #[test]
    fn the_guard_and_its_errors_never_show_the_serial() {
        let restored = restore_plan(&plan(), address("1.1.67"), &backup()).unwrap();
        let guard = restored.steps[2].clone();
        let shown = format!("{guard} {guard:?} {:?} {:?}", backup(), restored);
        assert!(
            !shown.contains("12 34") && !shown.to_uppercase().contains("1234"),
            "{shown}"
        );
        let mut text = String::new();
        for error in [
            RestoreError::ExemplarNotRecorded,
            RestoreError::ExemplarNotBindable,
            RestoreError::PlanAlreadyGuarded,
            RestoreError::PlanChangesNothing,
        ] {
            text.push_str(&error.to_string());
        }
        assert!(!text.contains("1234"));
    }

    #[test]
    fn the_source_procedure_drops_only_the_exemplar_guard() {
        let source = source_procedure_descriptions(&plan());
        assert_eq!(source.len(), plan().steps.len());
        let restored = restore_plan(&plan(), address("1.1.67"), &backup()).unwrap();
        assert_eq!(restored.steps.len(), plan().steps.len() + 1);
        assert_eq!(source_procedure_descriptions(&restored), source);
        assert!(source
            .iter()
            .any(|step| step.contains("compare property 3/13")));
    }

    #[test]
    fn a_plan_that_only_changes_load_states_is_guarded_too() {
        let mut task_only = plan();
        task_only
            .steps
            .retain(|step| !matches!(step, MemoryDownloadStep::WriteMemory { .. }));
        let mut held = backup();
        held.regions.clear();
        let restored = restore_plan(&task_only, address("1.1.67"), &held).unwrap();
        let guard = restored
            .steps
            .iter()
            .position(|step| matches!(step, MemoryDownloadStep::RequireExemplar(_)))
            .expect("guarded");
        let first_change = restored
            .steps
            .iter()
            .position(MemoryDownloadStep::changes_device)
            .unwrap();
        assert_eq!(guard + 1, first_change, "right before the first change");
        assert_eq!(
            restored
                .steps
                .iter()
                .filter(|step| matches!(step, MemoryDownloadStep::RequireExemplar(_)))
                .count(),
            1
        );
    }
}
