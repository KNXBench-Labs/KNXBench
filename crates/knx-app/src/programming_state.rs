//! What a finished or failed bus write means for a device's programming belief (P3, ADR-0129).
//!
//! The server reports how a download or an individual-address programming
//! ended; this module decides which parts become programmed and which become
//! pending, and applies that to the project outside the undo history (a write
//! to a device is not an edit and cannot be undone). Pure apart from the
//! project and stack it is handed.

use chrono::{DateTime, Utc};
use knx_core::commissioning::partial_memory_download::PartialDownloadParts;
use knx_core::commissioning::pending::{ProgrammingChange, ProgrammingParts};
use knx_core::{CommandStack, DeviceId, IndividualAddress, Project};
use knx_productdb::image::ImageRequest;

/// How a download ended, as far as the device is concerned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadEnd {
    /// Every step ran and every block was read back.
    Completed,
    /// The run stopped after a write may have reached the device.
    Interrupted(ProgrammingParts),
    /// Nothing reached the device.
    NothingWritten,
}

/// How an individual-address programming ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressEnd {
    /// The device answers at the new address (written now or already held).
    Programmed,
    /// The address may have been written but the device was not confirmed
    /// at it.
    Uncertain,
    /// Nothing was written.
    NothingWritten,
}

/// The belief change one bus write leads to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Settlement {
    /// Parts the device now holds as the project states them.
    pub programmed: ProgrammingParts,
    /// Parts the device may hold something else for.
    pub pending: ProgrammingParts,
    /// Parts whose device content this write changed or confirmed: no undo
    /// or redo may restore an older \"programmed\" claim about them.
    pub touched: ProgrammingParts,
}

/// The parts a download writes into device memory: a complete download the
/// application with its parameters and tables, a partial one what it names.
pub fn written_parts(partial: Option<PartialDownloadParts>) -> ProgrammingParts {
    match partial {
        None => ProgrammingParts {
            address: false,
            application: true,
            parameters: true,
            group_addresses: true,
        },
        Some(parts) => ProgrammingParts {
            parameters: parts.parameters,
            group_addresses: parts.group_addresses,
            ..ProgrammingParts::NONE
        },
    }
}

/// Parts a run may have changed through its one-based last started step.
/// The failing step counts too: a missing reply does not prove no write.
pub fn attempted_parts(
    plan: &knx_core::commissioning::memory_download::MemoryDownloadPlan,
    partial: Option<PartialDownloadParts>,
    through: Option<usize>,
) -> ProgrammingParts {
    use knx_core::commissioning::load_control_memory::MemoryLoadStateMachine as Machine;
    use knx_core::commissioning::memory_download::MemoryDownloadStep as Step;

    let scheduled = written_parts(partial);
    let Some(through) = through.filter(|step| *step <= plan.steps.len()) else {
        return scheduled;
    };
    let mut active = None;
    let mut attempted = ProgrammingParts::NONE;
    for step in plan.steps.iter().take(through) {
        let changed = match step {
            Step::LoadRecord(record) => {
                let parts = match record.machine() {
                    Machine::AddressTable | Machine::AssociationTable => ProgrammingParts {
                        group_addresses: true,
                        ..ProgrammingParts::NONE
                    },
                    Machine::ApplicationProgram | Machine::PeiProgram => ProgrammingParts {
                        application: partial.is_none(),
                        parameters: true,
                        ..ProgrammingParts::NONE
                    },
                };
                active = Some(parts);
                parts
            }
            Step::WriteMemory { .. } => active.unwrap_or(scheduled),
            Step::Connect
            | Step::CompareProperty { .. }
            | Step::RequireLoaded(_)
            | Step::Restart
            | Step::Disconnect => {
                active = None;
                ProgrammingParts::NONE
            }
        };
        attempted = attempted.union(changed);
    }
    attempted.intersection(scheduled)
}

/// The parts whose content in `now` equals what `written` sent. A part is
/// only current if everything it is built from is: parameters need the same
/// application, the group address tables also the same parameters (they
/// decide which objects exist) and the same individual address.
pub fn still_current(written: &ImageRequest, now: &ImageRequest) -> ProgrammingParts {
    let address = written.individual_address == now.individual_address;
    let application = written.program_id == now.program_id;
    let parameters = application && written.values == now.values;
    let group_addresses = parameters
        && address
        && written.links == now.links
        && written.flag_overrides == now.flag_overrides;
    ProgrammingParts {
        address,
        application,
        parameters,
        group_addresses,
    }
}

/// The settlement for a download. `now` is the request the project yields
/// when the run ends (`None` if it yields none any more); only a completed
/// run makes anything programmed, and only what is still current. A
/// complete download also confirms the address it reached the device at.
pub fn settle_download(
    partial: Option<PartialDownloadParts>,
    end: DownloadEnd,
    written: &ImageRequest,
    now: Option<&ImageRequest>,
) -> Settlement {
    let memory = written_parts(partial);
    match end {
        DownloadEnd::NothingWritten => Settlement::default(),
        DownloadEnd::Interrupted(attempted) => Settlement {
            programmed: ProgrammingParts::NONE,
            pending: attempted.intersection(memory),
            touched: attempted.intersection(memory),
        },
        DownloadEnd::Completed => {
            // The device answered at `written.individual_address`, the
            // address the run was started for.
            let confirmed = if partial.is_none() {
                ProgrammingParts::ALL
            } else {
                memory
            };
            let current = now.map_or(ProgrammingParts::NONE, |now| still_current(written, now));
            let programmed = confirmed.intersection(current);
            Settlement {
                programmed,
                pending: confirmed.intersection(complement(programmed)),
                touched: confirmed,
            }
        }
    }
}

fn complement(parts: ProgrammingParts) -> ProgrammingParts {
    ProgrammingParts {
        address: !parts.address,
        application: !parts.application,
        parameters: !parts.parameters,
        group_addresses: !parts.group_addresses,
    }
}

/// The settlement for an individual-address programming.
pub fn settle_address(end: AddressEnd) -> Settlement {
    let address = ProgrammingParts {
        address: true,
        ..ProgrammingParts::NONE
    };
    match end {
        AddressEnd::Programmed => Settlement {
            programmed: address,
            pending: ProgrammingParts::NONE,
            touched: address,
        },
        AddressEnd::Uncertain => Settlement {
            programmed: ProgrammingParts::NONE,
            pending: address,
            touched: address,
        },
        AddressEnd::NothingWritten => Settlement::default(),
    }
}

/// The one project device at `address`, or `None` when none or several
/// have it (the write does not say which of them it was).
pub fn sole_device_at(project: &Project, address: IndividualAddress) -> Option<DeviceId> {
    let mut at = project
        .devices
        .iter()
        .filter(|device| device.address == Some(address));
    match (at.next(), at.next()) {
        (Some(device), None) => Some(device.id),
        _ => None,
    }
}

/// Applies `settlement` to `device`. Returns whether anything changed; a
/// missing device changes nothing. `downloaded_at` records the time of a
/// download that made something programmed.
pub fn apply_settlement(
    project: &mut Project,
    stack: &mut CommandStack,
    device: DeviceId,
    settlement: &Settlement,
    downloaded_at: Option<DateTime<Utc>>,
) -> bool {
    let Some(instance) = project.devices.get_mut(device) else {
        return false;
    };
    let state = &mut instance.commissioning;
    let before = state.clone();
    ProgrammingChange::mark(settlement.programmed, true).apply_to(state);
    ProgrammingChange::mark(settlement.pending, false).apply_to(state);
    if let Some(at) = downloaded_at.filter(|_| !settlement.programmed.is_empty()) {
        state.last_download = Some(at);
    }
    let changed = *state != before;
    if !settlement.touched.is_empty() {
        stack.forget_programmed_claims(device, settlement.touched);
    }
    changed
}

#[cfg(test)]
mod tests;
