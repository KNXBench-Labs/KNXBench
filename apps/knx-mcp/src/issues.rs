//! `find_issues`: structural checks over one saved project.
//!
//! Pure function of the project and its read model; it reads no file and
//! asks no database. Each check reports what the project states, never a
//! guess about what was meant. These checks live here until a second
//! consumer needs them (ADR-0090).

use std::collections::{BTreeMap, HashSet};

use knx_core::{DeviceId, Project};
use knx_projection::{GroupAddressDptOutcome, ProjectTree};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

impl Severity {
    pub fn parse(text: &str) -> Result<Self, String> {
        match text {
            "error" => Ok(Self::Error),
            "warning" => Ok(Self::Warning),
            "info" => Ok(Self::Info),
            other => Err(format!(
                "unknown severity {other:?}; expected error, warning or info"
            )),
        }
    }
}

/// One thing the issue is about.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Subject {
    /// `device`, `groupAddress` or `comObject`.
    pub kind: &'static str,
    pub id: u32,
    /// The subject's name or address as the project states it.
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    pub severity: Severity,
    pub code: &'static str,
    pub installation: String,
    pub message: String,
    pub subjects: Vec<Subject>,
}

fn device_subject(project: &Project, id: DeviceId) -> Subject {
    let label = match project.devices.get(id) {
        Some(device) => match device.address {
            Some(address) => format!("{address} {}", device.name),
            None => device.name.clone(),
        },
        None => format!("device #{id}"),
    };
    Subject {
        kind: "device",
        id: id.0,
        label,
    }
}

/// Every issue, ordered by severity, then code, then first subject.
pub fn find_issues(project: &Project, tree: &ProjectTree) -> Vec<Issue> {
    let mut issues = Vec::new();
    for (installation, node) in project.installations.iter().zip(&tree.installations) {
        let name = installation.name.clone();
        let push = |issues: &mut Vec<Issue>, severity, code, message: String, subjects| {
            issues.push(Issue {
                severity,
                code,
                installation: name.clone(),
                message,
                subjects,
            })
        };

        // Devices this installation's topology holds.
        let mut devices: Vec<DeviceId> = Vec::new();
        for line in &installation.topology.lines {
            devices.extend(line.devices.iter().copied());
        }
        devices.extend(installation.topology.unassigned.iter().copied());

        let mut by_address: BTreeMap<String, Vec<DeviceId>> = BTreeMap::new();
        for &id in &devices {
            let Some(device) = project.devices.get(id) else {
                continue;
            };
            match device.address {
                Some(address) => by_address.entry(address.to_string()).or_default().push(id),
                None => push(
                    &mut issues,
                    Severity::Info,
                    "deviceWithoutAddress",
                    format!("device {:?} has no individual address", device.name),
                    vec![device_subject(project, id)],
                ),
            }
            if device.program_ref.is_empty() {
                push(
                    &mut issues,
                    Severity::Warning,
                    "deviceWithoutApplication",
                    format!("device {:?} names no application program", device.name),
                    vec![device_subject(project, id)],
                );
            }
            let unlinked = device
                .com_objects
                .iter()
                .filter_map(|com| project.devices.com_object(*com))
                .filter(|com| com.is_active && com.links.is_empty())
                .count();
            if unlinked > 0 {
                push(
                    &mut issues,
                    Severity::Info,
                    "activeComObjectsWithoutGroupAddress",
                    format!(
                        "device {:?} has {unlinked} active communication object(s) linked to no group address",
                        device.name
                    ),
                    vec![device_subject(project, id)],
                );
            }
        }
        for &id in &installation.topology.unassigned {
            if let Some(device) = project.devices.get(id) {
                push(
                    &mut issues,
                    Severity::Info,
                    "deviceNotInTopology",
                    format!("device {:?} is not placed on any line", device.name),
                    vec![device_subject(project, id)],
                );
            }
        }
        for (address, ids) in &by_address {
            if ids.len() > 1 {
                push(
                    &mut issues,
                    Severity::Error,
                    "duplicateIndividualAddress",
                    format!("{} devices share individual address {address}", ids.len()),
                    ids.iter().map(|id| device_subject(project, *id)).collect(),
                );
            }
        }

        for ga in &node.group_addresses {
            let subject = || Subject {
                kind: "groupAddress",
                id: ga.id,
                label: format!("{} {}", ga.address, ga.name),
            };
            if ga.links.is_empty() {
                push(
                    &mut issues,
                    Severity::Info,
                    "groupAddressWithoutLinks",
                    format!(
                        "group address {} is linked to no communication object",
                        ga.address
                    ),
                    vec![subject()],
                );
                continue;
            }
            let outcome = ga.dpt_detail.as_ref().map(|detail| detail.outcome);
            if outcome == Some(GroupAddressDptOutcome::SizeConflict) {
                push(
                    &mut issues,
                    Severity::Error,
                    "groupAddressDptSizeConflict",
                    format!(
                        "group address {}: declared and linked datapoint types differ in size ({})",
                        ga.address,
                        ga.dpts.join(", ")
                    ),
                    vec![subject()],
                );
            } else if ga.dpts.len() > 1
                || outcome == Some(GroupAddressDptOutcome::DeclaredDiffersFromLinked)
            {
                push(
                    &mut issues,
                    Severity::Warning,
                    "groupAddressDptDisagreement",
                    format!(
                        "group address {}: linked objects or declaration disagree on the datapoint type",
                        ga.address
                    ),
                    vec![subject()],
                );
            } else if ga.dpts.is_empty() {
                push(
                    &mut issues,
                    Severity::Info,
                    "groupAddressWithoutDpt",
                    format!(
                        "group address {}: neither the address nor a linked object states a datapoint type",
                        ga.address
                    ),
                    vec![subject()],
                );
            }
        }

        let style = project.info.group_address_style;
        for entry in &installation.group_addresses {
            let Some(range_id) = entry.range else {
                continue;
            };
            let Some(range) = installation.group_ranges.iter().find(|r| r.id == range_id) else {
                continue;
            };
            if !range.contains(entry.address) {
                push(
                    &mut issues,
                    Severity::Warning,
                    "groupAddressOutsideRange",
                    format!(
                        "group address {} lies outside its range {:?} ({}-{})",
                        entry.address.format(style),
                        range.name,
                        range.start.format(style),
                        range.end.format(style)
                    ),
                    vec![Subject {
                        kind: "groupAddress",
                        id: entry.id.0,
                        label: format!("{} {}", entry.address.format(style), entry.name),
                    }],
                );
            }
        }
    }

    // Links to an address no installation holds.
    let known: HashSet<u32> = project
        .installations
        .iter()
        .flat_map(|i| &i.group_addresses)
        .map(|g| g.id.0)
        .collect();
    for com in project.devices.com_objects() {
        for link in &com.links {
            if !known.contains(&link.ga.0) {
                issues.push(Issue {
                    severity: Severity::Error,
                    code: "danglingGroupLink",
                    installation: String::new(),
                    message: format!(
                        "communication object {} links to group address id {} that does not exist",
                        com.number, link.ga
                    ),
                    subjects: vec![
                        device_subject(project, com.device),
                        Subject {
                            kind: "comObject",
                            id: com.id.0,
                            label: format!("object {}", com.number),
                        },
                    ],
                });
            }
        }
    }

    issues.sort_by(|a, b| {
        (
            a.severity,
            a.code,
            a.subjects.first().map(|s| (s.kind, s.id)),
        )
            .cmp(&(
                b.severity,
                b.code,
                b.subjects.first().map(|s| (s.kind, s.id)),
            ))
    });
    issues
}
