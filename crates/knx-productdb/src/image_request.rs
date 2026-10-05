//! The [`ImageRequest`] for one project device, read from the project and nothing else.
//!
//! K3 of the commissioning goal: a download to a device takes its
//! configuration from the project, not from a hand-written request. The
//! mapping is pure. It reads a [`Project`] and returns a request, or a
//! refusal that names what is missing or contradictory. It never fills a
//! gap with a default. A value the project does not carry is left out of
//! [`ImageRequest::values`], and the image builder then takes the
//! product's own default for it, which is what an absent `ParameterInstance`
//! means in the project model.
//!
//! What goes where:
//!
//! - `program_id`: the application program behind the device's
//!   `program_ref` (a `Hardware2Program` id), resolved by the caller;
//!   [`image_request_for_device`] does it through the product database.
//! - `individual_address`: the device's own address. A device without one
//!   is refused, since the address table's first slot needs it.
//! - `values`: every `ParameterInstance` of the device, `ets_id` → `raw`.
//!   An id that belongs to another program is refused, and so are two
//!   different values for one id.
//! - `links`: every group link of every communication object of the
//!   device, as `(object number, group address, sending)`. `sending` is
//!   [`Direction::Send`]. A program may declare several `ComObjectRef`s for
//!   one object number (`A-0027-15-0BAC` has 23 for number 0), so a device
//!   created from the catalog has as many instances with that number; only
//!   the instances that carry links count. Refused: links on two instances
//!   with one number, two sending links on one number, an object number
//!   above `FFh` (the group object table holds one octet), and a link to a
//!   group address the project does not have.
//!
//! Whether an object is active is decided twice. The pure mapping does not
//! know the program. [`image_request_for_device`] evaluates the program's
//! `Dynamic` tree for the values above and refuses a link on an object
//! *instance* it does not activate. The builder then refuses a link on an
//! object *number* nothing activates
//! ([`crate::image::ImageError::InactiveObject`]).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use knx_core::{
    DeviceId, Direction, GroupAddress, GroupAddressId, GroupAddressStyle, IndividualAddress,
    Override, Project,
};
use rusqlite::Connection;

use crate::dynamic::{evaluate, load_program_trees, resolve_values};
use crate::enrich::com_object_lookup_id;
use crate::image::{FlagOverrides, ImageRequest, Link};
use crate::query::resolve_program;
use crate::ProductDbError;

/// Why a project device does not yield an [`ImageRequest`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectRequestError {
    /// No device with this id.
    UnknownDevice(DeviceId),
    /// The device has no individual address.
    NoIndividualAddress(DeviceId),
    /// The device names no application program (`program_ref` is empty).
    NoProgram(DeviceId),
    /// The device's `program_ref` resolves to no installed program.
    UnresolvedProgram {
        device: DeviceId,
        program_ref: String,
    },
    /// A parameter value whose id is not one of this program's.
    ForeignParameter {
        device: DeviceId,
        ets_id: String,
        program_id: String,
    },
    /// Two different values for one parameter.
    ConflictingParameter {
        device: DeviceId,
        ets_id: String,
        values: [String; 2],
    },
    /// A communication object listed by the device is not in the project.
    MissingComObject { device: DeviceId, com_object: u32 },
    /// Links on more than one communication object with the same number.
    /// A program may declare several `ComObjectRef`s for one number, of
    /// which its `Dynamic` tree activates one; links on two of them leave
    /// open which links apply.
    LinksOnSeveralObjects {
        device: DeviceId,
        number: u16,
        com_objects: Vec<u32>,
    },
    /// An object number the one-octet group object table cannot hold.
    ObjectNumberTooLarge { device: DeviceId, number: u16 },
    /// More than one sending link on one object.
    SeveralSendingLinks {
        device: DeviceId,
        number: u16,
        group_addresses: Vec<GroupAddress>,
    },
    /// A link to a group address the project does not have.
    DanglingGroupAddress {
        device: DeviceId,
        number: u16,
        group_address: GroupAddressId,
    },
    /// A communication flag the project states, but empty or unreadable
    /// (`Override::Empty`/`Override::Malformed`). Neither says whether the
    /// flag is on, and the product's own value would silently replace
    /// what the file said.
    UnreadableFlag {
        device: DeviceId,
        number: u16,
        flag: &'static str,
    },
}

impl fmt::Display for ProjectRequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownDevice(d) => write!(f, "device {} is not in the project", d.0),
            Self::NoIndividualAddress(d) => {
                write!(f, "device {} has no individual address", d.0)
            }
            Self::NoProgram(d) => write!(f, "device {} names no application program", d.0),
            Self::UnresolvedProgram {
                device,
                program_ref,
            } => write!(
                f,
                "device {}: application program {program_ref} is not in the product database",
                device.0
            ),
            Self::ForeignParameter {
                device,
                ets_id,
                program_id,
            } => write!(
                f,
                "device {}: parameter {ets_id} does not belong to program {program_id}",
                device.0
            ),
            Self::ConflictingParameter {
                device,
                ets_id,
                values,
            } => write!(
                f,
                "device {}: parameter {ets_id} has two values, {:?} and {:?}",
                device.0, values[0], values[1]
            ),
            Self::MissingComObject { device, com_object } => write!(
                f,
                "device {}: communication object {com_object} is not in the project",
                device.0
            ),
            Self::LinksOnSeveralObjects {
                device,
                number,
                com_objects,
            } => write!(
                f,
                "device {}: links on {} communication objects with number {number} (ids {:?})",
                device.0,
                com_objects.len(),
                com_objects
            ),
            Self::ObjectNumberTooLarge { device, number } => write!(
                f,
                "device {}: object number {number} does not fit the group object table (max 255)",
                device.0
            ),
            Self::SeveralSendingLinks {
                device,
                number,
                group_addresses,
            } => write!(
                f,
                "device {}: object {number} sends on {} group addresses ({})",
                device.0,
                group_addresses.len(),
                group_addresses
                    .iter()
                    .map(|ga| ga.format(GroupAddressStyle::ThreeLevel))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::DanglingGroupAddress {
                device,
                number,
                group_address,
            } => write!(
                f,
                "device {}: object {number} links group address id {}, which the project does not have",
                device.0, group_address.0
            ),
            Self::UnreadableFlag {
                device,
                number,
                flag,
            } => write!(
                f,
                "device {}: object {number} states {flag}, but not as Enabled or Disabled",
                device.0
            ),
        }
    }
}

impl std::error::Error for ProjectRequestError {}

/// Why [`image_request_for_device`] failed.
#[derive(Debug)]
pub enum DeviceRequestError {
    /// The product database could not be read.
    Database(ProductDbError),
    /// The project does not yield a request.
    Project(ProjectRequestError),
    /// A communication object carries links, but the device's parameters
    /// do not activate it. The program activates another object with the
    /// same number, or none; the link belongs to the object the user sees,
    /// so it is not moved silently.
    LinkOnInactiveObject {
        device: DeviceId,
        number: u16,
        com_object_ref: String,
    },
}

impl fmt::Display for DeviceRequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Database(e) => write!(f, "product database: {e}"),
            Self::Project(e) => e.fmt(f),
            Self::LinkOnInactiveObject {
                device,
                number,
                com_object_ref,
            } => write!(
                f,
                "device {}: object {number} ({com_object_ref}) has links, but the parameters do not activate it",
                device.0
            ),
        }
    }
}

impl std::error::Error for DeviceRequestError {}

impl From<ProjectRequestError> for DeviceRequestError {
    fn from(e: ProjectRequestError) -> Self {
        Self::Project(e)
    }
}

impl From<ProductDbError> for DeviceRequestError {
    fn from(e: ProductDbError) -> Self {
        Self::Database(e)
    }
}

/// The request for `device`, with its `program_ref` resolved through the
/// product database.
pub fn image_request_for_device(
    conn: &Connection,
    project: &Project,
    device: DeviceId,
) -> Result<ImageRequest, DeviceRequestError> {
    let instance = project
        .devices
        .get(device)
        .ok_or(ProjectRequestError::UnknownDevice(device))?;
    if instance.program_ref.is_empty() {
        return Err(ProjectRequestError::NoProgram(device).into());
    }
    let program_id = resolve_program(conn, &instance.program_ref)?.ok_or_else(|| {
        ProjectRequestError::UnresolvedProgram {
            device,
            program_ref: instance.program_ref.clone(),
        }
    })?;
    let request = image_request_from_project(project, device, &program_id)?;
    check_linked_objects_are_active(conn, project, device, &program_id, &request)?;
    Ok(request)
}

/// Refuses a link on an object instance the request's values do not
/// activate. [`crate::image::build_download_image`] checks an object's
/// *number* only, and a program may declare many `ComObjectRef`s for one
/// number (23 for number 0 in `A-0027-15-0BAC`). Without this check a link
/// on an inactive one would land on the active one.
fn check_linked_objects_are_active(
    conn: &Connection,
    project: &Project,
    device: DeviceId,
    program_id: &str,
    request: &ImageRequest,
) -> Result<(), DeviceRequestError> {
    let instance = project
        .devices
        .get(device)
        .ok_or(ProjectRequestError::UnknownDevice(device))?;
    let supplied = request.values.clone().into_iter().collect();
    let values = resolve_values(conn, program_id, &supplied)?;
    let activation = evaluate(&load_program_trees(conn, program_id)?, &values);
    let active: BTreeSet<&str> = activation
        .com_object_refs
        .iter()
        .filter(|active| active.scope.is_none())
        .map(|active| active.ref_id.as_str())
        .collect();
    for id in &instance.com_objects {
        let Some(object) = project.devices.com_object(*id) else {
            continue;
        };
        if object.links.is_empty() {
            continue;
        }
        let com_object_ref = com_object_lookup_id(
            program_id,
            &object.source.ets_id,
            object.module_instance.is_some(),
        );
        if !active.contains(com_object_ref.as_str()) {
            return Err(DeviceRequestError::LinkOnInactiveObject {
                device,
                number: object.number,
                com_object_ref,
            });
        }
    }
    Ok(())
}

/// The request for `device`, built for `program_id`. Pure: reads `project`
/// only.
pub fn image_request_from_project(
    project: &Project,
    device: DeviceId,
    program_id: &str,
) -> Result<ImageRequest, ProjectRequestError> {
    let instance = project
        .devices
        .get(device)
        .ok_or(ProjectRequestError::UnknownDevice(device))?;
    let individual_address: IndividualAddress = instance
        .address
        .ok_or(ProjectRequestError::NoIndividualAddress(device))?;
    Ok(ImageRequest {
        program_id: program_id.to_string(),
        individual_address,
        values: parameter_values(project, device, program_id)?,
        links: links(project, device)?,
        flag_overrides: flag_overrides(project, device, program_id)?,
    })
}

/// Every flag an object instance states itself, at `Layer::Instance` (read
/// from the project file) or `Layer::UserEdit`, by the instance's
/// `ComObjectRef` id. `Program`/`ProgramRef` values are enrichment's copy
/// of the product's own flags and are left out, so the builder keeps
/// taking those from the product database.
fn flag_overrides(
    project: &Project,
    device: DeviceId,
    program_id: &str,
) -> Result<BTreeMap<String, FlagOverrides>, ProjectRequestError> {
    let instance = project
        .devices
        .get(device)
        .ok_or(ProjectRequestError::UnknownDevice(device))?;
    let mut overrides = BTreeMap::new();
    for id in &instance.com_objects {
        let object =
            project
                .devices
                .com_object(*id)
                .ok_or(ProjectRequestError::MissingComObject {
                    device,
                    com_object: id.0,
                })?;
        let stated = |flag: &'static str, slot: &Override<bool>| match slot {
            Override::Absent => Ok(None),
            Override::Value(resolved) if resolved.layer.is_exported() => Ok(Some(resolved.value)),
            Override::Value(_) => Ok(None),
            Override::Empty | Override::Malformed(_) => Err(ProjectRequestError::UnreadableFlag {
                device,
                number: object.number,
                flag,
            }),
        };
        let flags = &object.flags;
        let object_overrides = FlagOverrides {
            read: stated("ReadFlag", &flags.read)?,
            write: stated("WriteFlag", &flags.write)?,
            transmit: stated("TransmitFlag", &flags.transmit)?,
            update: stated("UpdateFlag", &flags.update)?,
            communication: stated("CommunicationFlag", &flags.communication)?,
        };
        if object_overrides.is_empty() {
            continue;
        }
        let com_object_ref = com_object_lookup_id(
            program_id,
            &object.source.ets_id,
            object.module_instance.is_some(),
        );
        overrides.insert(com_object_ref, object_overrides);
    }
    Ok(overrides)
}

fn parameter_values(
    project: &Project,
    device: DeviceId,
    program_id: &str,
) -> Result<BTreeMap<String, String>, ProjectRequestError> {
    let prefix = format!("{program_id}_");
    let mut values: BTreeMap<String, String> = BTreeMap::new();
    for parameter in project
        .installations
        .iter()
        .flat_map(|i| &i.parameters)
        .filter(|p| p.device == device)
    {
        let ets_id = &parameter.source.ets_id;
        if !ets_id.starts_with(&prefix) {
            return Err(ProjectRequestError::ForeignParameter {
                device,
                ets_id: ets_id.clone(),
                program_id: program_id.to_string(),
            });
        }
        match values.get(ets_id) {
            Some(existing) if existing != &parameter.raw => {
                return Err(ProjectRequestError::ConflictingParameter {
                    device,
                    ets_id: ets_id.clone(),
                    values: [existing.clone(), parameter.raw.clone()],
                });
            }
            Some(_) => {}
            None => {
                values.insert(ets_id.clone(), parameter.raw.clone());
            }
        }
    }
    Ok(values)
}

fn links(project: &Project, device: DeviceId) -> Result<Vec<Link>, ProjectRequestError> {
    let instance = project
        .devices
        .get(device)
        .ok_or(ProjectRequestError::UnknownDevice(device))?;
    let addresses: BTreeMap<GroupAddressId, GroupAddress> = project
        .installations
        .iter()
        .flat_map(|i| &i.group_addresses)
        .map(|entry| (entry.id, entry.address))
        .collect();
    let mut linked: BTreeMap<u16, BTreeSet<u32>> = BTreeMap::new();
    let mut links = Vec::new();
    for id in &instance.com_objects {
        let object =
            project
                .devices
                .com_object(*id)
                .ok_or(ProjectRequestError::MissingComObject {
                    device,
                    com_object: id.0,
                })?;
        let number = object.number;
        if object.links.is_empty() {
            continue;
        }
        let holders = linked.entry(number).or_default();
        holders.insert(id.0);
        if holders.len() > 1 {
            return Err(ProjectRequestError::LinksOnSeveralObjects {
                device,
                number,
                com_objects: holders.iter().copied().collect(),
            });
        }
        let object_octet = u8::try_from(number)
            .map_err(|_| ProjectRequestError::ObjectNumberTooLarge { device, number })?;
        let mut sending = Vec::new();
        for link in &object.links {
            let group_address =
                *addresses
                    .get(&link.ga)
                    .ok_or(ProjectRequestError::DanglingGroupAddress {
                        device,
                        number,
                        group_address: link.ga,
                    })?;
            let is_sending = link.direction == Direction::Send;
            if is_sending {
                sending.push(group_address);
            }
            links.push(Link {
                object: object_octet,
                group_address,
                sending: is_sending,
            });
        }
        if sending.len() > 1 {
            return Err(ProjectRequestError::SeveralSendingLinks {
                device,
                number,
                group_addresses: sending,
            });
        }
    }
    links.sort_by_key(|l| (l.object, !l.sending, l.group_address.raw()));
    Ok(links)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::FlagOverrides;
    use knx_core::{
        ComObjectInstance, ComObjectInstanceId, CompletionStatus, DeviceInstance,
        GroupAddressEntry, GroupLink, Installation, InstallationId, Language, Layer, Override,
        ParameterInstance, ParameterInstanceId, Resolved, ResolvedFlags, SourceRef, Topology,
    };

    const PROGRAM: &str = "M-0083_A-0027-15-0BAC";

    fn source(ets_id: &str) -> SourceRef {
        SourceRef {
            path: "test".into(),
            ets_id: ets_id.into(),
        }
    }

    /// One device `1.1.67` with no parameters and no objects, and one group
    /// address `2/0/53` (id 1).
    fn project() -> Project {
        let mut p = Project::new(Language("en".into()));
        p.installations.push(Installation {
            id: InstallationId(0),
            name: "I".into(),
            default_line: None,
            multicast_address: None,
            completion: CompletionStatus::FinishedDesign,
            topology: Topology {
                areas: vec![],
                lines: vec![],
                unassigned: vec![DeviceId(1)],
            },
            buildings: vec![],
            group_ranges: vec![],
            group_addresses: vec![GroupAddressEntry {
                id: GroupAddressId(1),
                source: source("GA-1"),
                name: "Button 1".into(),
                address: GroupAddress::from_raw(0x1035),
                central: false,
                unfiltered: false,
                range: None,
                declared_dpt: Default::default(),
            }],
            parameters: vec![],
        });
        p.devices.insert(DeviceInstance {
            id: DeviceId(1),
            source: source("KB-DEV-1"),
            name: "Push button".into(),
            description: None,
            address: Some("1.1.67".parse().unwrap()),
            product_ref: "M-0083_H-39-1_P-BE.2DTA55P2.2E01".into(),
            program_ref: "M-0083_H-39-1_HP-0027-15-0BAC".into(),
            commissioning: Default::default(),
            visibility_calculated: true,
            com_objects: vec![],
            binary_data: vec![],
        });
        p
    }

    fn set(p: &mut Project, id: u32, short: &str, raw: &str) {
        p.installations[0].parameters.push(ParameterInstance {
            id: ParameterInstanceId(id),
            device: DeviceId(1),
            source: source(&format!("{PROGRAM}_{short}")),
            raw: raw.into(),
        });
    }

    fn object(p: &mut Project, id: u32, number: u16, links: Vec<GroupLink>) {
        p.devices.insert_com_object(ComObjectInstance {
            id: ComObjectInstanceId(id),
            source: source(&format!("{PROGRAM}_O-{number}_R-{id}")),
            device: DeviceId(1),
            number,
            text: Override::Absent,
            description: Override::Absent,
            dpt: Override::Absent,
            flags: ResolvedFlags::none(),
            size: None,
            is_active: true,
            links,
            module_instance: None,
        });
        p.devices
            .get_mut(DeviceId(1))
            .unwrap()
            .com_objects
            .push(ComObjectInstanceId(id));
    }

    fn send(ga: u32) -> GroupLink {
        GroupLink {
            ga: GroupAddressId(ga),
            direction: Direction::Send,
        }
    }

    fn receive(ga: u32) -> GroupLink {
        GroupLink {
            ga: GroupAddressId(ga),
            direction: Direction::Receive,
        }
    }

    fn map(p: &Project) -> Result<ImageRequest, ProjectRequestError> {
        image_request_from_project(p, DeviceId(1), PROGRAM)
    }

    #[test]
    fn option_c_maps_to_the_hand_written_request() {
        let mut p = project();
        set(&mut p, 1, "P-1007_R-1007", "2");
        set(&mut p, 2, "UP-5500_R-5500", "0");
        set(&mut p, 3, "UP-5501_R-5501", "1");
        // 23 instances share number 0 in this program; the unlinked ones
        // do not count.
        object(&mut p, 10, 0, vec![]);
        object(&mut p, 11, 0, vec![send(1)]);
        object(&mut p, 12, 1, vec![]);
        let request = map(&p).unwrap();
        assert_eq!(request.program_id, PROGRAM);
        assert_eq!(request.individual_address, "1.1.67".parse().unwrap());
        assert_eq!(
            request.values,
            [
                ("P-1007_R-1007", "2"),
                ("UP-5500_R-5500", "0"),
                ("UP-5501_R-5501", "1"),
            ]
            .into_iter()
            .map(|(short, raw)| (format!("{PROGRAM}_{short}"), raw.to_string()))
            .collect()
        );
        assert_eq!(
            request.links,
            vec![Link {
                object: 0,
                group_address: GroupAddress::from_raw(0x1035),
                sending: true,
            }]
        );
    }

    #[test]
    fn an_absent_value_stays_absent_rather_than_defaulted() {
        let request = map(&project()).unwrap();
        assert!(request.values.is_empty());
        assert!(request.links.is_empty());
    }

    #[test]
    fn a_device_without_an_address_is_refused() {
        let mut p = project();
        p.devices.get_mut(DeviceId(1)).unwrap().address = None;
        assert_eq!(
            map(&p),
            Err(ProjectRequestError::NoIndividualAddress(DeviceId(1)))
        );
    }

    #[test]
    fn an_unknown_device_is_refused() {
        assert_eq!(
            image_request_from_project(&project(), DeviceId(9), PROGRAM),
            Err(ProjectRequestError::UnknownDevice(DeviceId(9)))
        );
    }

    #[test]
    fn a_parameter_of_another_program_is_refused() {
        let mut p = project();
        p.installations[0].parameters.push(ParameterInstance {
            id: ParameterInstanceId(1),
            device: DeviceId(1),
            source: source("M-0083_A-0026-15-7565_P-1007_R-1007"),
            raw: "2".into(),
        });
        assert!(matches!(
            map(&p),
            Err(ProjectRequestError::ForeignParameter { .. })
        ));
    }

    #[test]
    fn another_devices_parameter_is_not_taken() {
        let mut p = project();
        p.installations[0].parameters.push(ParameterInstance {
            id: ParameterInstanceId(1),
            device: DeviceId(2),
            source: source(&format!("{PROGRAM}_P-1007_R-1007")),
            raw: "2".into(),
        });
        assert!(map(&p).unwrap().values.is_empty());
    }

    #[test]
    fn two_different_values_for_one_parameter_are_refused() {
        let mut p = project();
        set(&mut p, 1, "P-1007_R-1007", "2");
        set(&mut p, 2, "P-1007_R-1007", "1");
        assert_eq!(
            map(&p),
            Err(ProjectRequestError::ConflictingParameter {
                device: DeviceId(1),
                ets_id: format!("{PROGRAM}_P-1007_R-1007"),
                values: ["2".into(), "1".into()],
            })
        );
    }

    #[test]
    fn the_same_value_twice_is_one_value() {
        let mut p = project();
        set(&mut p, 1, "P-1007_R-1007", "2");
        set(&mut p, 2, "P-1007_R-1007", "2");
        assert_eq!(map(&p).unwrap().values.len(), 1);
    }

    #[test]
    fn links_on_two_objects_with_one_number_are_refused() {
        let mut p = project();
        object(&mut p, 10, 0, vec![send(1)]);
        object(&mut p, 11, 0, vec![receive(1)]);
        assert_eq!(
            map(&p),
            Err(ProjectRequestError::LinksOnSeveralObjects {
                device: DeviceId(1),
                number: 0,
                com_objects: vec![10, 11],
            })
        );
    }

    #[test]
    fn two_sending_links_on_one_object_are_refused() {
        let mut p = project();
        p.installations[0].group_addresses.push(GroupAddressEntry {
            id: GroupAddressId(2),
            source: source("GA-2"),
            name: "Other".into(),
            address: GroupAddress::from_raw(0x1036),
            central: false,
            unfiltered: false,
            range: None,
            declared_dpt: Default::default(),
        });
        object(&mut p, 10, 0, vec![send(1), send(2)]);
        assert!(matches!(
            map(&p),
            Err(ProjectRequestError::SeveralSendingLinks { number: 0, .. })
        ));
    }

    #[test]
    fn a_link_to_a_missing_group_address_is_refused() {
        let mut p = project();
        object(&mut p, 10, 0, vec![send(7)]);
        assert_eq!(
            map(&p),
            Err(ProjectRequestError::DanglingGroupAddress {
                device: DeviceId(1),
                number: 0,
                group_address: GroupAddressId(7),
            })
        );
    }

    #[test]
    fn an_object_number_above_one_octet_is_refused_only_when_linked() {
        let mut p = project();
        object(&mut p, 10, 256, vec![]);
        assert!(map(&p).is_ok());
        object(&mut p, 11, 300, vec![receive(1)]);
        assert_eq!(
            map(&p),
            Err(ProjectRequestError::ObjectNumberTooLarge {
                device: DeviceId(1),
                number: 300,
            })
        );
    }

    #[test]
    fn a_listed_object_missing_from_the_project_is_refused() {
        let mut p = project();
        p.devices
            .get_mut(DeviceId(1))
            .unwrap()
            .com_objects
            .push(ComObjectInstanceId(42));
        assert_eq!(
            map(&p),
            Err(ProjectRequestError::MissingComObject {
                device: DeviceId(1),
                com_object: 42,
            })
        );
    }

    fn stated(value: bool, layer: Layer) -> Override<bool> {
        Override::Value(Resolved { value, layer })
    }

    /// Object 0 (instance 10) with `flags`.
    fn object_with_flags(flags: ResolvedFlags) -> Project {
        let mut p = project();
        object(&mut p, 10, 0, vec![send(1)]);
        p.devices
            .com_object_mut(ComObjectInstanceId(10))
            .unwrap()
            .flags = flags;
        p
    }

    #[test]
    fn an_instance_flag_becomes_an_override_for_its_com_object_ref() {
        // The house's 1.1.20 object 0: write and update enabled by the
        // instance, the other flags left to the product.
        let p = object_with_flags(ResolvedFlags {
            write: stated(true, Layer::Instance),
            update: stated(true, Layer::Instance),
            ..ResolvedFlags::none()
        });
        let request = map(&p).unwrap();
        assert_eq!(
            request.flag_overrides,
            [(
                format!("{PROGRAM}_O-0_R-10"),
                FlagOverrides {
                    write: Some(true),
                    update: Some(true),
                    ..FlagOverrides::default()
                }
            )]
            .into_iter()
            .collect()
        );
    }

    #[test]
    fn a_user_edit_is_an_override_too_and_can_switch_a_flag_off() {
        let p = object_with_flags(ResolvedFlags {
            transmit: stated(false, Layer::UserEdit),
            ..ResolvedFlags::none()
        });
        let overrides = &map(&p).unwrap().flag_overrides[&format!("{PROGRAM}_O-0_R-10")];
        assert_eq!(overrides.transmit, Some(false));
    }

    #[test]
    fn product_flags_filled_in_by_enrichment_are_not_overrides() {
        // Enrichment writes the program's own flags into absent slots at
        // `Program`/`ProgramRef`; they are the product's, not the project's.
        let p = object_with_flags(ResolvedFlags {
            read: stated(true, Layer::Program),
            write: stated(false, Layer::ProgramRef),
            ..ResolvedFlags::none()
        });
        assert!(map(&p).unwrap().flag_overrides.is_empty());
    }

    #[test]
    fn an_unlinked_objects_instance_flag_is_carried_as_well() {
        let mut p = project();
        object(&mut p, 10, 0, vec![]);
        p.devices
            .com_object_mut(ComObjectInstanceId(10))
            .unwrap()
            .flags = ResolvedFlags {
            read: stated(true, Layer::Instance),
            ..ResolvedFlags::none()
        };
        assert_eq!(map(&p).unwrap().flag_overrides.len(), 1);
    }

    #[test]
    fn an_empty_or_unreadable_flag_is_refused_rather_than_guessed() {
        for state in [Override::Empty, Override::Malformed("Perhaps".into())] {
            let p = object_with_flags(ResolvedFlags {
                write: state.clone(),
                ..ResolvedFlags::none()
            });
            assert_eq!(
                map(&p),
                Err(ProjectRequestError::UnreadableFlag {
                    device: DeviceId(1),
                    number: 0,
                    flag: "WriteFlag",
                }),
                "{state:?}"
            );
        }
    }

    #[test]
    fn links_come_out_in_object_order_sending_first() {
        let mut p = project();
        p.installations[0].group_addresses.push(GroupAddressEntry {
            id: GroupAddressId(2),
            source: source("GA-2"),
            name: "Other".into(),
            address: GroupAddress::from_raw(0x0001),
            central: false,
            unfiltered: false,
            range: None,
            declared_dpt: Default::default(),
        });
        object(&mut p, 12, 1, vec![receive(2)]);
        object(&mut p, 10, 0, vec![receive(2), send(1)]);
        let objects: Vec<(u8, bool, u16)> = map(&p)
            .unwrap()
            .links
            .iter()
            .map(|l| (l.object, l.sending, l.group_address.raw()))
            .collect();
        assert_eq!(
            objects,
            vec![(0, true, 0x1035), (0, false, 0x0001), (1, false, 0x0001)]
        );
    }
}
