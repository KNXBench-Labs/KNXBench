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
//! What a backup does not hold, and a restore therefore cannot bring back:
//! memory outside the regions the plan writes (masked octets are never
//! written, so they need no backup), properties, and a load state other
//! than `Loaded` — a restore always ends `Loaded`, like any download.

use std::fmt;

use super::load_control_memory::MemoryLoadStateMachine;
use super::load_state::{LoadState, MaskVersion};
use super::memory_download::{machines, MemoryDownloadPlan, MemoryDownloadStep};
use crate::IndividualAddress;

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
    let mut regions = backup.regions.iter();
    let steps = plan
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
    Ok(MemoryDownloadPlan {
        mask: plan.mask,
        manufacturer: plan.manufacturer,
        steps,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commissioning::load_control_memory::event_record;
    use crate::commissioning::load_state::LoadEvent;

    fn address(text: &str) -> IndividualAddress {
        text.parse().unwrap()
    }

    fn plan() -> MemoryDownloadPlan {
        MemoryDownloadPlan {
            mask: MaskVersion(0x0701),
            manufacturer: 0x0083,
            steps: vec![
                MemoryDownloadStep::Connect,
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
        }
    }

    #[test]
    fn a_restore_is_the_plan_with_the_old_octets() {
        let restored = restore_plan(&plan(), address("1.1.67"), &backup()).unwrap();
        let expected: Vec<MemoryDownloadStep> = plan()
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
}
