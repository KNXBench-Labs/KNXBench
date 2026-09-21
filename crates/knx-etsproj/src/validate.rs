//! Stage 4: structural validation of a [`SourceDocument`].
//!
//! Validation never modifies the document and never aborts the import — it
//! only reports. An **error** is data the mapper (Task 10) cannot use (a
//! dangling reference, a duplicate `Id`); a **warning** is data the mapper
//! can use but that is suspicious (two devices sharing one individual
//! address, a group address outside the range that contains it).
//!
//! There is no authoritative XSD to conform to (KNOWN_LIMITATIONS §2), so
//! this is a hand-built set of structural checks, and its scope is
//! deliberately bounded by what the installed corpus can attest. Every
//! check below is either a violation the corpus contains or a coverage
//! hole measured in the corpus; the list of checks *not* implemented, with
//! the measurement that argues against each, is just as load-bearing and
//! is kept below rather than left as a silence.
//!
//! # What is checked, and why the corpus says so
//!
//! Counts are from the three installed corpus projects — "Unser Zuhause"
//! at schema 11 (ETS 4.1.8) and schema 23 (ETS 6.3), and the KNX
//! Association "KV v2.5" demo at schema 21 — measured 2026-09-21.
//!
//! - **Duplicate `Id`.** Walks `Area`, `Line`, `DeviceInstance`,
//!   `BinaryData`, `GroupRange`, `GroupAddress` and `BuildingPart`. The
//!   `BuildingPart` arm is not decoration: `map.rs`'s `allocate_ids` pass
//!   inserts every part into one `BTreeMap<String, BuildingPartId>`, so a
//!   repeated `@Id` overwrites the earlier entry and both parts end up
//!   sharing one `BuildingPartId` — two rooms silently collapsed into one
//!   identity, with the first one's `DeviceInstanceRef`s landing on the
//!   second. 45 `BuildingPart/@Id`s across the corpus went entirely
//!   unwalked before this check existed (22 + 22 + 1).
//! - **Dangling `GroupAddressRefId`.** Covers both spellings. Schema 11
//!   writes `Connectors/Send|Receive/@GroupAddressRefId` (596 links in the
//!   corpus, all in the ETS4 project); schema ≥21 replaced it with a flat
//!   space-separated `Links` attribute (622 links, 26 in the KV project
//!   and 596 in the ETS6 one). Only the first spelling was walked before,
//!   which meant stage 4 validated 596 of the corpus's 1,218
//!   communication-object links and left the schema-≥21 half — i.e. every
//!   project a current ETS writes — with no referential check at all.
//! - **Duplicate individual address**, **duplicate group address**,
//!   **group address outside its enclosing range**: unchanged, all three
//!   warnings.
//!
//! # What is deliberately *not* checked
//!
//! - **`ModuleInstance/@Id` uniqueness.** Measured and rejected: the KV
//!   schema-21 project carries 32 `ModuleInstance/@Id`s of which only 16
//!   are distinct, because three of its four devices run the same
//!   application program and therefore repeat `MD-1_M-1_MI-1` and its
//!   siblings verbatim. The id is device-scoped, not project-unique, so
//!   feeding it to `check_duplicate_ids` would invent 16 errors on a
//!   valid project. This is why the id walk enumerates element kinds
//!   rather than taking every `@Id` it can reach.
//! - **`Installation/@DefaultLine`, `BuildingPart/@DefaultLine`,
//!   `BuildingPart`'s `DeviceInstanceRef`.** The corpus *does* now attest
//!   the first one — the KV project writes `DefaultLine=""`, which
//!   resolves to nothing — so the older "no measured case" reason has
//!   expired. It is still not checked here, for a different and better
//!   reason: `map.rs`'s `resolve_optional`/`resolve_many` already report
//!   all three as `MapProblemDetail::UnresolvedReference`, and a second
//!   channel for one class of problem is worse than a late one. If that
//!   reporting ever moves out of the mapper, this is where it comes.
//! - **Anything needing the manufacturer/application-program database**
//!   (`ProductRefId`, a `ParameterInstanceRef`/`ComObjectInstanceRef`
//!   `RefId` resolved against its program). Session 4 has happened and
//!   `knx-productdb` exists, so "there is no database yet" is no longer
//!   the reason. The reason now is structural: `knx-etsproj` does not
//!   depend on `knx-productdb` and must not start — stage 4 is a pure
//!   function of one `SourceDocument`, and the database is not an input to
//!   it (manufacturer files are not even collected until stage 7). Product
//!   resolution is also not a property of the file: the same `.knxproj` is
//!   resolvable or not depending on what happens to be installed, and a
//!   stage that classifies a *document* cannot own a verdict that depends
//!   on the machine. `knx_productdb::enrich` keeps it, and stays the only
//!   channel that reports it.
//!
//! # Scoping
//!
//! Two `Installation`s are independent bus/address spaces — the domain
//! model already supports more than one (`Project::installations:
//! Vec<Installation>`), and two of them legitimately reusing the same
//! individual address or group address is normal, not a conflict. The two
//! checks that compare raw address *values* (not ETS id strings) are
//! therefore scoped per installation ([`DeviceAddress::installation`]/
//! [`GroupAddressOccurrence::installation`]). `check_duplicate_ids` stays
//! global on purpose: it compares ETS id *strings*, which already embed
//! the owning installation's number as part of the id itself (e.g.
//! `P-0512-0_GA-1`), so a genuine cross-installation string collision
//! would be a real anomaly worth flagging, not a false positive the way a
//! bare address number is.
//!
//! `check_dangling_group_address_refs` applies that same rule to each of
//! its two spellings and lands in different places, which is the point of
//! stating the rule rather than the outcome. A schema-11
//! `@GroupAddressRefId` is fully qualified and embeds the installation, so
//! it resolves globally. Schema ≥21's `Links` targets are *short* ids
//! (`"GA-3"`) and embed nothing, so they resolve per installation.
//! `map.rs`'s `short_group_address_ids` builds its lookup table globally
//! instead; for a one-installation project — all three corpus projects —
//! the two agree exactly, and for a multi-installation one this stage will
//! report a cross-installation `Links` target the mapper quietly resolves
//! to the wrong installation's group address. No corpus sample has two
//! installations, so that divergence is reasoned, not measured.

use crate::source::{SourceBuildingPart, SourceDocument, SourceGroupRange};

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

/// How a link spells its target, which decides what it is resolved
/// against — see the module header's scoping section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LinkSpelling {
    /// Schema 11's `Connectors/Send|Receive/@GroupAddressRefId`: a fully
    /// qualified `GroupAddress/@Id` such as `P-0512-0_GA-1`.
    Qualified,
    /// Schema ≥21's flat `Links` attribute: a short id such as `GA-3`,
    /// carrying no installation of its own.
    Short,
}

/// One communication object's group-address links, gathered while walking
/// devices, for the dangling-reference pass.
struct GroupAddressLink {
    /// Which `Installation` the linking device belongs to. Only consulted
    /// for [`LinkSpelling::Short`], whose target says nothing about it.
    installation: usize,
    /// The `ComObjectInstance/@RefId` the link belongs to.
    from: String,
    /// The `GroupAddressRefId`/`Links` value itself — the (possibly
    /// dangling) link target.
    to: String,
    spelling: LinkSpelling,
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

    // Schema 11 nests the building tree as `Buildings/BuildingPart`,
    // schema ≥21 as `Locations/Space` — the same entity under two names,
    // so only the path spelling changes, never the walk.
    let (buildings_container, buildings_element) = if document.schema_version >= 21 {
        ("Locations", "Space")
    } else {
        ("Buildings", "BuildingPart")
    };

    for (inst_idx, installation) in document.installations.iter().enumerate() {
        let inst_path = format!("/KNX/Project/Installations/Installation[{inst_idx}]");
        let buildings_path = format!("{inst_path}/{buildings_container}");

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

        for part in &installation.buildings {
            walk_building_part(part, &buildings_path, buildings_element, &mut ids);
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
                installation,
                from: com_object.ref_id.clone(),
                to: target.clone(),
                spelling: LinkSpelling::Qualified,
                xpath: com_path.clone(),
            });
        }
        // Schema ≥21's replacement for the two `Connectors` lists above.
        // A document never uses both: the schema-11 parser leaves `links`
        // empty and the schema-≥21 parser leaves `sends`/`receives` empty,
        // so this loop and the one above are alternatives in practice, not
        // a double count.
        for target in &com_object.links {
            links.push(GroupAddressLink {
                installation,
                from: com_object.ref_id.clone(),
                to: target.clone(),
                spelling: LinkSpelling::Short,
                xpath: com_path.clone(),
            });
        }
    }
}

/// Collects `BuildingPart/@Id` for the duplicate-id pass, recursing into
/// nested parts. Nothing else on a `BuildingPart` is validated here — its
/// `@DefaultLine` and `DeviceInstanceRef`s are the mapper's to report (see
/// the module header).
///
/// `element` is the tree's name at this schema version: `BuildingPart`
/// under `Buildings` at schema 11, `Space` under `Locations` at schema
/// ≥21 (`xpath.rs`'s `building_part` draws the same distinction). The
/// `kind` recorded on the occurrence stays `"BuildingPart"` at both, since
/// it names the entity the duplicate-id check groups by, not the tag the
/// file happened to spell it with.
fn walk_building_part(
    part: &SourceBuildingPart,
    parent_path: &str,
    element: &'static str,
    ids: &mut Vec<IdOccurrence>,
) {
    let part_path = format!("{parent_path}/{element}[@Id='{}']", part.id);
    ids.push(IdOccurrence {
        kind: "BuildingPart",
        id: part.id.clone(),
        xpath: part_path.clone(),
    });
    for child in &part.children {
        walk_building_part(child, &part_path, element, ids);
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

/// The short form of a fully qualified group-address id, e.g.
/// `"P-03DE-0_GA-3"` → `"GA-3"` — the spelling schema ≥21's `Links`
/// attribute uses. Mirrors `map.rs`'s `short_group_address_ids` exactly,
/// including its refusal to shorten an id whose trailing segment is not a
/// `GA-…`: an id shaped some other way simply has no short form, and a
/// `Links` target naming it is dangling, which is the same conclusion the
/// mapper reaches.
fn short_group_address_id(full: &str) -> Option<&str> {
    full.rsplit_once('_')
        .map(|(_, short)| short)
        .filter(|short| short.starts_with("GA-"))
}

fn check_dangling_group_address_refs(
    links: &[GroupAddressLink],
    group_addresses: &[GroupAddressOccurrence],
    errors: &mut Vec<SourceProblem>,
) {
    use std::collections::HashSet;
    let qualified: HashSet<&str> = group_addresses.iter().map(|g| g.id.as_str()).collect();
    // Short ids carry no installation of their own, so the installation is
    // part of the key rather than assumed away — see the module header.
    let short: HashSet<(usize, &str)> = group_addresses
        .iter()
        .filter_map(|g| Some((g.installation, short_group_address_id(&g.id)?)))
        .collect();

    for link in links {
        let resolved = match link.spelling {
            LinkSpelling::Qualified => qualified.contains(link.to.as_str()),
            LinkSpelling::Short => short.contains(&(link.installation, link.to.as_str())),
        };
        if !resolved {
            errors.push(SourceProblem {
                xpath: link.xpath.clone(),
                id: Some(link.from.clone()),
                detail: ProblemDetail::DanglingReference {
                    // One `kind` for both spellings: the attribute is
                    // named `GroupAddressRefId` at schema 11 and `Links`
                    // at schema ≥21, but the broken thing is the same
                    // link, and a reader of the report should not have to
                    // know which schema wrote the file to search for it.
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
    use crate::testutil::{
        minimal_source_document, reference_kv_source_document, reference_source_document,
    };

    #[test]
    fn the_reference_project_validates_clean() {
        if !crate::testutil::corpus_available() {
            eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
            return;
        }
        let doc = reference_source_document();
        let out = validate(&doc);
        assert_eq!(out.errors, vec![]);
        assert_eq!(out.warnings, vec![]);
    }

    /// The other half of the corpus, and the half the dangling-link check
    /// was blind to until this task: the KV schema-21 project's 26
    /// `Links` targets and its 32 `ModuleInstance/@Id`s — of which only
    /// 16 are distinct, because three of its four devices run the same
    /// application program. If this ever reports anything, either the
    /// short-id resolution has drifted from the mapper's, or somebody has
    /// fed `ModuleInstance/@Id` to the project-wide duplicate-id check.
    #[test]
    fn the_schema_21_reference_project_validates_clean() {
        if !crate::testutil::corpus_available() {
            eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
            return;
        }
        let doc = reference_kv_source_document();
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

    /// The schema-≥21 shape of [`minimal_source_document`]: the same
    /// document with its `Connectors` lists emptied and the single link
    /// respelled the way a current ETS writes it — a flat, space-separated
    /// `Links` attribute of short ids. Built by mutation rather than by a
    /// second XML fixture so the two differ in exactly the one thing under
    /// test.
    fn minimal_v21_source_document() -> SourceDocument {
        let mut doc = minimal_source_document();
        doc.schema_version = 21;
        let com = &mut doc.installations[0].areas[0].lines[0].devices[0].com_objects[0];
        com.sends.clear();
        com.receives.clear();
        com.links = vec!["GA-1".into()];
        doc
    }

    #[test]
    fn a_resolvable_schema_21_short_link_is_not_reported() {
        let out = validate(&minimal_v21_source_document());
        assert_eq!(out.errors, vec![]);
        assert_eq!(out.warnings, vec![]);
    }

    #[test]
    fn a_dangling_schema_21_short_link_is_an_error() {
        let mut doc = minimal_v21_source_document();
        doc.installations[0].areas[0].lines[0].devices[0].com_objects[0].links =
            vec!["GA-999".into()];
        let out = validate(&doc);
        assert_eq!(out.errors.len(), 1);
        assert!(matches!(
            &out.errors[0].detail,
            ProblemDetail::DanglingReference {
                kind: "GroupAddressRefId",
                to,
                ..
            } if to == "GA-999"
        ));
    }

    /// Malformed input: `Links` is a free-text attribute, so anything at
    /// all can turn up in it. None of these resolves, every one of them is
    /// reported, and nothing panics — the stage's founding invariant is
    /// that a file that parses still imports.
    #[test]
    fn malformed_schema_21_link_targets_are_reported_not_panicked_on() {
        let mut doc = minimal_v21_source_document();
        doc.installations[0].areas[0].lines[0].devices[0].com_objects[0].links = vec![
            "GA-".into(),
            "GA-not-a-number".into(),
            "-".into(),
            "_".into(),
            "💡".into(),
            // A *qualified* id in an attribute that takes short ones: the
            // right group address, spelled the schema-11 way. It does not
            // resolve, and saying so is the point of the check.
            "P-0001-0_GA-1".into(),
        ];
        let out = validate(&doc);
        assert_eq!(out.errors.len(), 6);
        assert!(out
            .errors
            .iter()
            .all(|e| matches!(e.detail, ProblemDetail::DanglingReference { .. })));
    }

    /// Short ids carry no installation, so two installations that each own
    /// their own `GA-1` must not lend it to one another. The module header
    /// notes that `map.rs` resolves short links globally and would.
    #[test]
    fn a_short_link_does_not_resolve_across_installations() {
        let mut doc = minimal_v21_source_document();
        let mut second = doc.installations[0].clone();
        second.areas[0].id = "P-0001-1_A-1".into();
        second.areas[0].lines[0].id = "P-0001-1_L-2".into();
        second.areas[0].lines[0].devices[0].id = "P-0001-1_DI-1".into();
        second.group_ranges[0].id = "P-0001-1_GR-1".into();
        second.group_ranges[0].children[0].id = "P-0001-1_GR-2".into();
        // The second installation's only group address is GA-2, not GA-1,
        // while its device still links GA-1 — which exists, but in the
        // first installation.
        second.group_ranges[0].children[0].addresses[0].id = "P-0001-1_GA-2".into();
        second.group_ranges[0].children[0].addresses[0].address = Some("2".into());
        doc.installations.push(second);

        let out = validate(&doc);
        assert_eq!(out.errors.len(), 1);
        assert!(matches!(
            &out.errors[0].detail,
            ProblemDetail::DanglingReference { to, .. } if to == "GA-1"
        ));
    }

    /// Two building parts sharing one `@Id`. `map.rs`'s `allocate_ids`
    /// pass keys parts by that string, so the second insert overwrites the
    /// first and both rooms end up as one `BuildingPartId` — the first
    /// one's devices silently land on the second. `xpath.rs`'s header
    /// names this stage as the place to catch it.
    #[test]
    fn a_duplicate_building_part_id_is_an_error() {
        let mut doc = minimal_source_document();
        let room = crate::source::SourceBuildingPart {
            id: "P-0001-0_BP-1".into(),
            name: Some("Wohnzimmer".into()),
            kind: Some("Room".into()),
            ..Default::default()
        };
        let twin = crate::source::SourceBuildingPart {
            name: Some("Küche".into()),
            ..room.clone()
        };
        doc.installations[0].buildings = vec![crate::source::SourceBuildingPart {
            id: "P-0001-0_BP-0".into(),
            name: Some("Haus".into()),
            kind: Some("Building".into()),
            children: vec![room, twin],
            ..Default::default()
        }];

        let out = validate(&doc);
        assert_eq!(out.errors.len(), 1);
        assert!(matches!(
            &out.errors[0].detail,
            ProblemDetail::DuplicateId {
                kind: "BuildingPart",
                id,
            } if id == "P-0001-0_BP-1"
        ));
        assert_eq!(out.warnings, vec![]);
    }

    /// Distinct ids in the same tree stay silent, nesting included.
    #[test]
    fn a_nested_building_tree_with_distinct_ids_validates_clean() {
        let mut doc = minimal_source_document();
        doc.installations[0].buildings = vec![crate::source::SourceBuildingPart {
            id: "P-0001-0_BP-0".into(),
            children: vec![crate::source::SourceBuildingPart {
                id: "P-0001-0_BP-1".into(),
                children: vec![crate::source::SourceBuildingPart {
                    id: "P-0001-0_BP-2".into(),
                    ..Default::default()
                }],
                ..Default::default()
            }],
            ..Default::default()
        }];
        let out = validate(&doc);
        assert_eq!(out.errors, vec![]);
        assert_eq!(out.warnings, vec![]);
    }

    /// The corpus measurement that argues *against* a check, kept as a
    /// test so the next reader cannot re-add it by accident: the KV
    /// schema-21 project repeats `MD-1_M-1_MI-1` across three devices
    /// running one application program, so `ModuleInstance/@Id` is
    /// device-scoped, not project-unique.
    #[test]
    fn repeated_module_instance_ids_across_devices_are_not_a_duplicate() {
        let mut doc = minimal_v21_source_document();
        let module = crate::source::SourceModuleInstance {
            id: "MD-1_M-1_MI-1".into(),
            ref_id: "MD-1_M-1".into(),
            ..Default::default()
        };
        let line = &mut doc.installations[0].areas[0].lines[0];
        line.devices[0].module_instances = vec![module.clone()];
        let mut twin = line.devices[0].clone();
        twin.id = "P-0001-0_DI-2".into();
        twin.address = Some("2".into());
        twin.module_instances = vec![module];
        line.devices.push(twin);

        let out = validate(&doc);
        assert_eq!(out.errors, vec![]);
        assert_eq!(out.warnings, vec![]);
    }
}
