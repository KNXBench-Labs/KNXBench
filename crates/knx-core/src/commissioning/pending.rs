//! Which parts of its programming a device still owes the project (P3, ADR-0129).
//!
//! The project's belief about a device lives in the four `*_loaded` flags of
//! [`CommissioningState`](super::CommissioningState) that ETS projects carry
//! (`IndividualAddressLoaded`, `ApplicationProgramLoaded`, `ParametersLoaded`,
//! `CommunicationPartLoaded`). A part whose flag is `false` is *pending*: the
//! project holds something the device has not been sent. The fifth flag,
//! `MediumConfigLoaded`, has no editor in KNXBench and is left alone.
//!
//! This module only describes parts, changes to the belief and the project
//! content each part is programmed from. Deciding *when* a part becomes
//! pending (an edit) or programmed (a download) belongs to the command stack
//! and the application layer.

use std::collections::{BTreeMap, BTreeSet};

use super::{CommissioningState, LoadPart};
use crate::flags::{Direction, ResolvedFlags};
use crate::ids::{ComObjectInstanceId, DeviceId, GroupAddressId};
use crate::provenance::Override;
use crate::{IndividualAddress, Project};

/// A set of the four parts shown to the user: Address, Application,
/// Parameters, Group addresses (the communication part).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ProgrammingParts {
    pub address: bool,
    pub application: bool,
    pub parameters: bool,
    pub group_addresses: bool,
}

impl ProgrammingParts {
    pub const NONE: Self = Self {
        address: false,
        application: false,
        parameters: false,
        group_addresses: false,
    };
    pub const ALL: Self = Self {
        address: true,
        application: true,
        parameters: true,
        group_addresses: true,
    };

    pub fn is_empty(self) -> bool {
        self == Self::NONE
    }

    pub fn union(self, other: Self) -> Self {
        Self {
            address: self.address || other.address,
            application: self.application || other.application,
            parameters: self.parameters || other.parameters,
            group_addresses: self.group_addresses || other.group_addresses,
        }
    }

    pub fn intersection(self, other: Self) -> Self {
        Self {
            address: self.address && other.address,
            application: self.application && other.application,
            parameters: self.parameters && other.parameters,
            group_addresses: self.group_addresses && other.group_addresses,
        }
    }

    /// The parts in this set, as the load parts the flags are named after.
    pub fn load_parts(self) -> impl Iterator<Item = LoadPart> {
        [
            (self.address, LoadPart::IndividualAddress),
            (self.application, LoadPart::ApplicationProgram),
            (self.parameters, LoadPart::Parameters),
            (self.group_addresses, LoadPart::CommunicationPart),
        ]
        .into_iter()
        .filter_map(|(member, part)| member.then_some(part))
    }
}

impl CommissioningState {
    /// The four parts the project believes the device does not hold yet.
    pub fn pending_parts(&self) -> ProgrammingParts {
        ProgrammingParts {
            address: !self.individual_address_loaded,
            application: !self.application_program_loaded,
            parameters: !self.parameters_loaded,
            group_addresses: !self.communication_part_loaded,
        }
    }
}

/// A change to the project's belief, part by part: `Some(true)` programmed,
/// `Some(false)` pending, `None` left as it is. Applying one returns the
/// change that restores the previous belief, so it is its own undo form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ProgrammingChange {
    pub address: Option<bool>,
    pub application: Option<bool>,
    pub parameters: Option<bool>,
    pub group_addresses: Option<bool>,
}

impl ProgrammingChange {
    /// Sets every part in `parts` to `programmed`, leaves the others.
    pub fn mark(parts: ProgrammingParts, programmed: bool) -> Self {
        let value = |member: bool| member.then_some(programmed);
        Self {
            address: value(parts.address),
            application: value(parts.application),
            parameters: value(parts.parameters),
            group_addresses: value(parts.group_addresses),
        }
    }

    pub fn is_empty(self) -> bool {
        self == Self::default()
    }

    /// Applies the change and returns the one that undoes it.
    pub fn apply_to(self, state: &mut CommissioningState) -> Self {
        let swap = |value: Option<bool>, slot: &mut bool| {
            value.map(|value| std::mem::replace(slot, value))
        };
        Self {
            address: swap(self.address, &mut state.individual_address_loaded),
            application: swap(self.application, &mut state.application_program_loaded),
            parameters: swap(self.parameters, &mut state.parameters_loaded),
            group_addresses: swap(self.group_addresses, &mut state.communication_part_loaded),
        }
    }

    /// The same change, except that it never claims a part of `parts` is
    /// programmed: such a claim becomes \"pending\". Used on history entries
    /// once the device was written, so no undo or redo can bring back a
    /// claim about content the device no longer holds.
    pub fn without_programmed_claims(self, parts: ProgrammingParts) -> Self {
        let pending = |value: Option<bool>, member: bool| {
            if member {
                value.map(|_| false)
            } else {
                value
            }
        };
        Self {
            address: pending(self.address, parts.address),
            application: pending(self.application, parts.application),
            parameters: pending(self.parameters, parts.parameters),
            group_addresses: pending(self.group_addresses, parts.group_addresses),
        }
    }
}

/// The project content one device is programmed from, part by part. Two
/// contents compare equal exactly where a download would send the same.
/// Datapoint types are not part of it: the group object table carries the
/// product's object types, not the project's DPT choice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgrammingContent {
    address: Option<IndividualAddress>,
    application: (String, String),
    /// `(ParameterRef id, raw value)`, sorted.
    parameters: Vec<(String, String)>,
    /// Per communication object, in id order: its effective flags and its
    /// links with the linked group address's numeric value.
    group_addresses: Vec<ObjectContent>,
}

/// The six flags as the effective value each states, if any. The layer a
/// value came from is provenance, not content.
type FlagValues = [Option<bool>; 6];

/// One object's links: the linked group address's number (`None` if the
/// link names no known address) and the direction.
type LinkContent = Vec<(Option<u16>, Direction)>;

/// One communication object's share of the group address part.
type ObjectContent = (ComObjectInstanceId, FlagValues, LinkContent);

impl ProgrammingContent {
    /// The parts whose content differs between `self` and `other`.
    pub fn changed_parts(&self, other: &Self) -> ProgrammingParts {
        ProgrammingParts {
            address: self.address != other.address,
            application: self.application != other.application,
            parameters: self.parameters != other.parameters,
            group_addresses: self.group_addresses != other.group_addresses,
        }
    }
}

/// The programming content of each of `devices` that exists in `project`.
/// One pass over the parameters and group addresses, however many devices.
pub fn programming_content(
    project: &Project,
    devices: &BTreeSet<DeviceId>,
) -> BTreeMap<DeviceId, ProgrammingContent> {
    let mut contents: BTreeMap<DeviceId, ProgrammingContent> = devices
        .iter()
        .filter_map(|id| project.devices.get(*id))
        .map(|device| {
            (
                device.id,
                ProgrammingContent {
                    address: device.address,
                    application: (device.product_ref.clone(), device.program_ref.clone()),
                    parameters: Vec::new(),
                    group_addresses: Vec::new(),
                },
            )
        })
        .collect();
    if contents.is_empty() {
        return contents;
    }
    for parameter in project.installations.iter().flat_map(|i| &i.parameters) {
        if let Some(content) = contents.get_mut(&parameter.device) {
            content
                .parameters
                .push((parameter.source.ets_id.clone(), parameter.raw.clone()));
        }
    }
    let mut numbers: BTreeMap<GroupAddressId, u16> = BTreeMap::new();
    for entry in project
        .installations
        .iter()
        .flat_map(|i| &i.group_addresses)
    {
        numbers.entry(entry.id).or_insert(entry.address.raw());
    }
    for (id, content) in contents.iter_mut() {
        content.parameters.sort();
        let device = project.devices.get(*id).expect("collected above");
        let mut objects: Vec<_> = device
            .com_objects
            .iter()
            .filter_map(|co| project.devices.com_object(*co))
            .map(|co| {
                let links = co
                    .links
                    .iter()
                    .map(|link| (numbers.get(&link.ga).copied(), link.direction))
                    .collect();
                (co.id, flag_values(&co.flags), links)
            })
            .collect();
        objects.sort_by_key(|(co, _, _)| *co);
        content.group_addresses = objects;
    }
    contents
}

fn flag_values(flags: &ResolvedFlags) -> FlagValues {
    let value = |flag: &Override<bool>| flag.value().map(|resolved| resolved.value);
    [
        value(&flags.read),
        value(&flags.write),
        value(&flags.transmit),
        value(&flags.update),
        value(&flags.communication),
        value(&flags.read_on_init),
    ]
}

/// The devices whose communication objects link to `ga`.
pub fn devices_linked_to(project: &Project, ga: GroupAddressId) -> BTreeSet<DeviceId> {
    project
        .devices
        .com_objects()
        .filter(|co| co.links.iter().any(|link| link.ga == ga))
        .map(|co| co.device)
        .collect()
}

#[cfg(test)]
pub(crate) mod fixture;
#[cfg(test)]
mod tests;
