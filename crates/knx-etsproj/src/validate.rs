//! Stage 4: structural validation of a [`SourceDocument`].
//!
//! Validation never modifies the document and never aborts the import — it
//! only reports. An **error** is data the mapper (Task 10) cannot use (a
//! dangling reference, a duplicate `Id`); a **warning** is data the mapper
//! can use but that is suspicious (two devices sharing one individual
//! address, a group address outside the range that contains it).
//!
//! Every check here stays inside the `SourceDocument` itself — no
//! manufacturer/application-program database exists yet (Session 4), so a
//! `ProductRefId` or a `ParameterInstanceRef/ComObjectInstanceRef`'s
//! `RefId` cannot be resolved here and is not checked. `BuildingPart`
//! device references and `Installation/@DefaultLine` are likewise left
//! unchecked for now: unlike group-address links, no measured case in the
//! reference project or a written test yet motivates the added surface —
//! add them when one does, per CLAUDE.md's "prefer documented evidence
//! over assumptions."
//!
//! Two `Installation`s are independent bus/address spaces — the domain
//! model already supports more than one (`Project::installations:
//! Vec<Installation>`), and two of them legitimately reusing the same
//! individual address or group address is normal, not a conflict. The two
//! checks that compare raw address *values* (not ETS id strings) are
//! therefore scoped per installation ([`DeviceAddress::installation`]/
//! [`GroupAddressOccurrence::installation`]). `check_duplicate_ids` and
//! `check_dangling_group_address_refs` stay global on purpose: they
//! compare ETS id *strings*, which already embed the owning installation's
//! number as part of the id itself (e.g. `P-0512-0_GA-1`), so a genuine
//! cross-installation string collision would be a real anomaly worth
//! flagging, not a false positive the way a bare address number is.

use crate::source::{SourceDocument, SourceGroupRange};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ValidationOutput {
    pub errors: Vec<SourceProblem>,
    pub warnings: Vec<SourceProblem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceProblem {
    pub xpath: String,
    pub id: Option<String>,
    pub detail: ProblemDetail,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProblemDetail {
    DuplicateId {
        kind: &'static str,
        id: String,
    },
    DanglingReference {
        kind: &'static str,
        from: String,
        to: String,
    },
    DuplicateIndividualAddress {
        address: String,
        devices: Vec<String>,
    },
    DuplicateGroupAddress {
        address: String,
        ids: Vec<String>,
    },
    GroupAddressOutsideItsRange {
        id: String,
        address: String,
        range: String,
    },
    /// Reserved for a required-but-optional field found missing in a real
    /// project. Not yet produced by [`validate`] — no measured case has
    /// needed it — kept so the interface has a place for one without a
    /// breaking change when it turns up.
    MissingRequiredAttribute {
        name: &'static str,
    },
}

/// One element carrying a project-unique `Id`, gathered while walking the
/// document, for the duplicate-id pass.
struct IdOccurrence {
    kind: &'static str,
    id: String,
    xpath: String,
}

/// One communication object's group-address links, gathered while walking
/// devices, for the dangling-reference pass.
struct GroupAddressLink {
    /// The `ComObjectInstance/@RefId` the link belongs to.
    from: String,
    /// The `GroupAddressRefId` value itself — the (possibly dangling) link target.
    to: String,
    xpath: String,
}

/// One device's composed individual address, for the duplicate-address pass.
struct DeviceAddress {
    /// Which `Installation` this device belongs to (its index in
    /// `document.installations`) — installations are independent bus/
    /// address spaces, so a duplicate check must not compare addresses
    /// across them.
    installation: usize,
    device_id: String,
    area: String,
    line: String,
    device: String,
}

/// One group address in its immediate enclosing range, for the duplicate-
/// address and outside-range passes.
struct GroupAddressOccurrence {
    /// Which `Installation` this address belongs to — see
    /// [`DeviceAddress::installation`] for why this matters.
    installation: usize,
    id: String,
    address: Option<String>,
    xpath: String,
    /// The immediate enclosing `GroupRange`'s name and bounds, for the
    /// outside-range check.
    range_name: String,
    range_start: Option<String>,
    range_end: Option<String>,
}

pub fn validate(document: &SourceDocument) -> ValidationOutput {
    let mut ids = Vec::new();
    let mut links = Vec::new();
    let mut device_addresses = Vec::new();
    let mut group_addresses = Vec::new();

    for (inst_idx, installation) in document.installations.iter().enumerate() {
        let inst_path = format!("/KNX/Project/Installations/Installation[{inst_idx}]");

        for area in &installation.areas {
            let area_path = format!("{inst_path}/Topology/Area[@Id='{}']", area.id);
            ids.push(IdOccurrence {
                kind: "Area",
                id: area.id.clone(),
                xpath: area_path.clone(),
            });

            for line in &area.lines {
                let line_path = format!("{area_path}/Line[@Id='{}']", line.id);
                ids.push(IdOccurrence {
                    kind: "Line",
                    id: line.id.clone(),
                    xpath: line_path.clone(),
                });

                for device in &line.devices {
                    walk_device(
                        device,
                        &line_path,
                        Some(&area.address),
                        Some(&line.address),
                        inst_idx,
                        &mut ids,
                        &mut links,
                        &mut device_addresses,
                    );
                }
            }
        }

        for device in &installation.unassigned_devices {
            let unassigned_path = format!("{inst_path}/Topology/UnassignedDevices");
            walk_device(
                device,
                &unassigned_path,
                None,
                None,
                inst_idx,
                &mut ids,
                &mut links,
                &mut device_addresses,
            );
        }

        for range in &installation.group_ranges {
            walk_group_range(range, &inst_path, inst_idx, &mut ids, &mut group_addresses);
        }
    }

    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    check_duplicate_ids(&ids, &mut errors);
    check_dangling_group_address_refs(&links, &group_addresses, &mut errors);
    check_duplicate_individual_addresses(&device_addresses, &mut warnings);
    check_duplicate_group_addresses(&group_addresses, &mut warnings);
    check_group_addresses_outside_their_range(&group_addresses, &mut warnings);

    ValidationOutput { errors, warnings }
}

#[allow(clippy::too_many_arguments)]
fn walk_device(
    device: &crate::source::SourceDevice,
    parent_path: &str,
    area_address: Option<&Option<String>>,
    line_address: Option<&Option<String>>,
    installation: usize,
    ids: &mut Vec<IdOccurrence>,
    links: &mut Vec<GroupAddressLink>,
    device_addresses: &mut Vec<DeviceAddress>,
) {
    let device_path = format!("{parent_path}/DeviceInstance[@Id='{}']", device.id);
    ids.push(IdOccurrence {
        kind: "DeviceInstance",
        id: device.id.clone(),
        xpath: device_path.clone(),
    });

    if let (Some(Some(area)), Some(Some(line)), Some(dev)) =
        (area_address, line_address, device.address.as_ref())
    {
        device_addresses.push(DeviceAddress {
            installation,
            device_id: device.id.clone(),
            area: area.clone(),
            line: line.clone(),
            device: dev.clone(),
        });
    }

    for binary_data in &device.binary_data {
        ids.push(IdOccurrence {
            kind: "BinaryData",
            id: binary_data.id.clone(),
            xpath: format!(
                "{device_path}/BinaryData/BinaryData[@Id='{}']",
                binary_data.id
            ),
        });
    }

    for com_object in &device.com_objects {
        let com_path = format!(
            "{device_path}/ComObjectInstanceRefs/ComObjectInstanceRef[@RefId='{}']",
            com_object.ref_id
        );
        for target in com_object.sends.iter().chain(com_object.receives.iter()) {
            links.push(GroupAddressLink {
                from: com_object.ref_id.clone(),
                to: target.clone(),
                xpath: com_path.clone(),
            });
        }
    }
}

fn walk_group_range(
    range: &SourceGroupRange,
    parent_path: &str,
    installation: usize,
    ids: &mut Vec<IdOccurrence>,
    group_addresses: &mut Vec<GroupAddressOccurrence>,
) {
    let range_path = format!("{parent_path}/GroupRanges/GroupRange[@Id='{}']", range.id);
    ids.push(IdOccurrence {
        kind: "GroupRange",
        id: range.id.clone(),
        xpath: range_path.clone(),
    });

    for address in &range.addresses {
        ids.push(IdOccurrence {
            kind: "GroupAddress",
            id: address.id.clone(),
            xpath: format!("{range_path}/GroupAddress[@Id='{}']", address.id),
        });
        group_addresses.push(GroupAddressOccurrence {
            installation,
            id: address.id.clone(),
            address: address.address.clone(),
            xpath: format!("{range_path}/GroupAddress[@Id='{}']", address.id),
            range_name: range.name.clone().unwrap_or_default(),
            range_start: range.range_start.clone(),
            range_end: range.range_end.clone(),
        });
    }

    for child in &range.children {
        walk_group_range(child, &range_path, installation, ids, group_addresses);
    }
}

fn check_duplicate_ids(ids: &[IdOccurrence], errors: &mut Vec<SourceProblem>) {
    use std::collections::HashMap;
    let mut seen: HashMap<(&'static str, &str), &IdOccurrence> = HashMap::new();
    for occurrence in ids {
        if seen
            .insert((occurrence.kind, occurrence.id.as_str()), occurrence)
            .is_some()
        {
            errors.push(SourceProblem {
                xpath: occurrence.xpath.clone(),
                id: Some(occurrence.id.clone()),
                detail: ProblemDetail::DuplicateId {
                    kind: occurrence.kind,
                    id: occurrence.id.clone(),
                },
            });
        }
    }
}

fn check_dangling_group_address_refs(
    links: &[GroupAddressLink],
    group_addresses: &[GroupAddressOccurrence],
    errors: &mut Vec<SourceProblem>,
) {
    use std::collections::HashSet;
    let known: HashSet<&str> = group_addresses.iter().map(|g| g.id.as_str()).collect();
    for link in links {
        if !known.contains(link.to.as_str()) {
            errors.push(SourceProblem {
                xpath: link.xpath.clone(),
                id: Some(link.from.clone()),
                detail: ProblemDetail::DanglingReference {
                    kind: "GroupAddressRefId",
                    from: link.from.clone(),
                    to: link.to.clone(),
                },
            });
        }
    }
}

fn check_duplicate_individual_addresses(
    device_addresses: &[DeviceAddress],
    warnings: &mut Vec<SourceProblem>,
) {
    use std::collections::HashMap;
    let mut groups: HashMap<(usize, String, String, String), Vec<&DeviceAddress>> = HashMap::new();
    for da in device_addresses {
        groups
            .entry((
                da.installation,
                da.area.clone(),
                da.line.clone(),
                da.device.clone(),
            ))
            .or_default()
            .push(da);
    }
    for ((_installation, area, line, device), occurrences) in groups {
        if occurrences.len() > 1 {
            let address = format!("{area}.{line}.{device}");
            warnings.push(SourceProblem {
                xpath: String::new(),
                id: None,
                detail: ProblemDetail::DuplicateIndividualAddress {
                    address,
                    devices: occurrences.iter().map(|d| d.device_id.clone()).collect(),
                },
            });
        }
    }
}

fn check_duplicate_group_addresses(
    group_addresses: &[GroupAddressOccurrence],
    warnings: &mut Vec<SourceProblem>,
) {
    use std::collections::HashMap;
    let mut groups: HashMap<(usize, &str), Vec<&GroupAddressOccurrence>> = HashMap::new();
    for ga in group_addresses {
        if let Some(address) = ga.address.as_deref() {
            groups
                .entry((ga.installation, address))
                .or_default()
                .push(ga);
        }
    }
    for ((_installation, address), occurrences) in groups {
        if occurrences.len() > 1 {
            warnings.push(SourceProblem {
                xpath: String::new(),
                id: None,
                detail: ProblemDetail::DuplicateGroupAddress {
                    address: address.to_string(),
                    ids: occurrences.iter().map(|g| g.id.clone()).collect(),
                },
            });
        }
    }
}

fn check_group_addresses_outside_their_range(
    group_addresses: &[GroupAddressOccurrence],
    warnings: &mut Vec<SourceProblem>,
) {
    for ga in group_addresses {
        let (Some(address), Some(start), Some(end)) = (
            ga.address.as_deref(),
            ga.range_start.as_deref(),
            ga.range_end.as_deref(),
        ) else {
            continue;
        };
        let (Ok(address_n), Ok(start_n), Ok(end_n)) = (
            address.parse::<i64>(),
            start.parse::<i64>(),
            end.parse::<i64>(),
        ) else {
            // Not numeric: a format problem for the mapper (Task 10) to
            // report as a `ValueError`, not a structural one for us.
            continue;
        };
        if address_n < start_n || address_n > end_n {
            warnings.push(SourceProblem {
                xpath: ga.xpath.clone(),
                id: Some(ga.id.clone()),
                detail: ProblemDetail::GroupAddressOutsideItsRange {
                    id: ga.id.clone(),
                    address: address.to_string(),
                    range: ga.range_name.clone(),
                },
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{minimal_source_document, reference_source_document};

    #[test]
    fn the_reference_project_validates_clean() {
        let doc = reference_source_document();
        let out = validate(&doc);
        assert_eq!(out.errors, vec![]);
        assert_eq!(out.warnings, vec![]);
    }

    #[test]
    fn a_dangling_group_address_reference_is_an_error_not_a_panic() {
        let mut doc = minimal_source_document();
        doc.installations[0].areas[0].lines[0].devices[0].com_objects[0].sends =
            vec!["P-0001-0_GA-999".into()];
        let out = validate(&doc);
        assert!(matches!(
            out.errors[0].detail,
            ProblemDetail::DanglingReference {
                kind: "GroupAddressRefId",
                ..
            }
        ));
    }

    #[test]
    fn two_devices_on_one_individual_address_is_a_warning() {
        let mut doc = minimal_source_document();
        let line = &mut doc.installations[0].areas[0].lines[0];
        let mut clone = line.devices[0].clone();
        clone.id = "P-0001-0_DI-2".into();
        line.devices.push(clone);
        let out = validate(&doc);
        assert!(matches!(
            out.warnings[0].detail,
            ProblemDetail::DuplicateIndividualAddress { .. }
        ));
        assert_eq!(out.errors, vec![]);
    }

    #[test]
    fn a_duplicate_element_id_is_an_error() {
        let mut doc = minimal_source_document();
        let inst = &mut doc.installations[0];
        let clone = inst.group_ranges[0].children[0].addresses[0].clone();
        inst.group_ranges[0].children[0].addresses.push(clone);
        let out = validate(&doc);
        assert!(matches!(
            out.errors[0].detail,
            ProblemDetail::DuplicateId {
                kind: "GroupAddress",
                ..
            }
        ));
    }

    #[test]
    fn an_address_outside_its_enclosing_range_is_a_warning() {
        let mut doc = minimal_source_document();
        doc.installations[0].group_ranges[0].children[0].addresses[0].address = Some("9999".into());
        let out = validate(&doc);
        assert!(matches!(
            out.warnings[0].detail,
            ProblemDetail::GroupAddressOutsideItsRange { .. }
        ));
    }

    #[test]
    fn two_installations_reusing_the_same_individual_and_group_address_is_not_a_duplicate() {
        // Installations are independent bus/address spaces: each one
        // legitimately has its own device at 1.1.1 and its own group
        // address "1" — a *different* Id, since ETS always embeds the
        // installation number in every Id it writes; the address *values*
        // coincide, the identities do not — and that is not a conflict
        // between them.
        let mut doc = minimal_source_document();
        let mut second = doc.installations[0].clone();
        second.areas[0].id = "P-0001-1_A-1".into();
        second.areas[0].lines[0].id = "P-0001-1_L-2".into();
        second.areas[0].lines[0].devices[0].id = "P-0001-1_DI-1".into();
        second.group_ranges[0].id = "P-0001-1_GR-1".into();
        second.group_ranges[0].children[0].id = "P-0001-1_GR-2".into();
        second.group_ranges[0].children[0].addresses[0].id = "P-0001-1_GA-1".into();
        second.areas[0].lines[0].devices[0].com_objects[0].sends = vec!["P-0001-1_GA-1".into()];
        doc.installations.push(second);

        let out = validate(&doc);
        assert_eq!(out.errors, vec![]);
        assert_eq!(out.warnings, vec![]);
    }
}
