//! Datapoint type inference for group addresses (IMPORT_EXPORT §7).
//!
//! `GroupAddressEntry` deliberately carries no datapoint type field
//! (DATA_MODEL §9) — a datapoint type is a property of the communication
//! objects linked to an address, not of the address itself — so inference
//! produces a side table rather than mutating the model. That also makes
//! the "never exported" rule structural: there is nothing on the entity for
//! export to write back.
//!
//! Where every linked object that states a datapoint type states the same
//! one, that value is inferred (`Layer::Inferred` conceptually, though
//! there is no field to attach the layer to — the layer lives entirely in
//! this module not applying at all to anything on `GroupAddressEntry`).
//! Where they disagree, that is a [`Conflict`]: no majority vote, no
//! first-wins rule, the address simply keeps no inferred type. A linked
//! object that states no *usable* datapoint type (`Override::Empty`,
//! `Override::Absent`, or an `Override::Malformed` value the mapper could
//! not parse) contributes no candidate either way — it neither supports
//! nor contradicts what the others say.

use std::collections::{BTreeMap, HashMap};

use knx_core::{ComObjectInstanceId, DptRef, GroupAddressId, Project};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InferenceOutput {
    pub inferred: Vec<InferredValue>,
    pub conflicts: Vec<Conflict>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InferredValue {
    pub group_address: GroupAddressId,
    pub dpt: DptRef,
    pub from: Vec<ComObjectInstanceId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conflict {
    pub group_address: GroupAddressId,
    pub candidates: Vec<(DptRef, Vec<ComObjectInstanceId>)>,
}

// Hand-written rather than derived: `GroupAddressId`, `DptRef` and
// `ComObjectInstanceId` are `knx_core` types, and `knx_core` stays free of
// `serde` (Task 14's own scope is `knx-etsproj` only — see its Cargo.toml
// change). Each field converts to a plain, always-serializable primitive:
// an id's raw integer, a `DptRef` through its existing `Display` impl.
impl serde::Serialize for InferredValue {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("InferredValue", 3)?;
        s.serialize_field("group_address", &self.group_address.0)?;
        s.serialize_field("dpt", &self.dpt.to_string())?;
        s.serialize_field("from", &self.from.iter().map(|id| id.0).collect::<Vec<_>>())?;
        s.end()
    }
}

impl serde::Serialize for Conflict {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let candidates: Vec<(String, Vec<u32>)> = self
            .candidates
            .iter()
            .map(|(dpt, ids)| (dpt.to_string(), ids.iter().map(|id| id.0).collect()))
            .collect();
        let mut s = serializer.serialize_struct("Conflict", 2)?;
        s.serialize_field("group_address", &self.group_address.0)?;
        s.serialize_field("candidates", &candidates)?;
        s.end()
    }
}

pub fn infer_group_address_dpts(project: &Project) -> InferenceOutput {
    // Every (dpt, com object) candidate pair a link contributes, keyed by
    // the group address it links to. `BTreeMap` gives deterministic
    // iteration order (by `GroupAddressId`) without a separate sort.
    let mut per_address: BTreeMap<GroupAddressId, Vec<(DptRef, ComObjectInstanceId)>> =
        BTreeMap::new();

    for device in project.devices.iter() {
        for &com_id in &device.com_objects {
            let Some(com) = project.devices.com_object(com_id) else {
                continue;
            };
            let Some(resolved) = com.dpt.value() else {
                continue; // Empty, Absent or Malformed: no candidate.
            };
            for link in &com.links {
                per_address
                    .entry(link.ga)
                    .or_default()
                    .push((resolved.value, com_id));
            }
        }
    }

    let mut inferred = Vec::new();
    let mut conflicts = Vec::new();

    for (ga, candidates) in per_address {
        let mut by_dpt: HashMap<DptRef, Vec<ComObjectInstanceId>> = HashMap::new();
        for (dpt, com_id) in candidates {
            by_dpt.entry(dpt).or_default().push(com_id);
        }
        let mut grouped: Vec<(DptRef, Vec<ComObjectInstanceId>)> = by_dpt.into_iter().collect();
        for (_, ids) in &mut grouped {
            ids.sort();
        }
        grouped.sort_by_key(|(dpt, _)| (dpt.main, dpt.sub));

        match grouped.len() {
            0 => {} // every linked object left its dpt empty or absent
            1 => {
                let (dpt, from) = grouped.into_iter().next().expect("length checked above");
                inferred.push(InferredValue {
                    group_address: ga,
                    dpt,
                    from,
                });
            }
            _ => conflicts.push(Conflict {
                group_address: ga,
                candidates: grouped,
            }),
        }
    }

    InferenceOutput {
        inferred,
        conflicts,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::reference_project;
    use knx_core::provenance::{Layer, Override, Resolved};
    use knx_core::{
        ComObjectInstance, CommissioningState, DeviceId, DeviceInstance, Direction, GroupAddress,
        GroupAddressEntry, GroupLink, IdAllocators, Installation, InstallationId, Language,
        ResolvedFlags, SourceRef, Topology,
    };

    fn source() -> SourceRef {
        SourceRef {
            path: "t".into(),
            ets_id: "t".into(),
        }
    }

    fn device_with_com_objects(
        project: &mut Project,
        device_id: DeviceId,
        com_objects: Vec<(ComObjectInstanceId, Option<DptRef>, GroupAddressId)>,
    ) {
        let mut ids = Vec::new();
        for (com_id, dpt, ga) in com_objects {
            ids.push(com_id);
            project.devices.insert_com_object(ComObjectInstance {
                id: com_id,
                source: source(),
                device: device_id,
                number: 0,
                text: Override::Absent,
                description: Override::Absent,
                dpt: dpt
                    .map(|value| {
                        Override::Value(Resolved {
                            value,
                            layer: Layer::Instance,
                        })
                    })
                    .unwrap_or(Override::Absent),
                flags: ResolvedFlags::none(),
                size: None,
                is_active: true,
                links: vec![GroupLink {
                    ga,
                    direction: Direction::Send,
                }],
                module_instance: None,
            });
        }
        project.devices.insert(DeviceInstance {
            id: device_id,
            source: source(),
            name: "D".into(),
            description: None,
            address: None,
            product_ref: String::new(),
            program_ref: String::new(),
            commissioning: CommissioningState::default(),
            visibility_calculated: true,
            com_objects: ids,
            binary_data: vec![],
        });
    }

    fn empty_project() -> Project {
        let mut project = Project::new(Language("en".into()));
        project.installations.push(Installation {
            id: InstallationId(0),
            name: "I".into(),
            default_line: None,
            multicast_address: None,
            completion: Default::default(),
            topology: Topology {
                areas: vec![],
                lines: vec![],
                unassigned: vec![],
            },
            buildings: vec![],
            group_ranges: vec![],
            group_addresses: vec![GroupAddressEntry {
                id: GroupAddressId(1),
                source: source(),
                name: "GA".into(),
                address: GroupAddress::from_raw(1),
                central: false,
                unfiltered: false,
                range: None,
            }],
            parameters: vec![],
        });
        project.ids = IdAllocators::default();
        project
    }

    fn project_with_two_links(a: DptRef, b: DptRef) -> Project {
        let mut project = empty_project();
        device_with_com_objects(
            &mut project,
            DeviceId(1),
            vec![
                (ComObjectInstanceId(1), Some(a), GroupAddressId(1)),
                (ComObjectInstanceId(2), Some(b), GroupAddressId(1)),
            ],
        );
        project
    }

    fn project_with_orphan_address() -> Project {
        // `empty_project` already has one `GroupAddressEntry` with nothing
        // linked to it — that is the orphan this fixture needs.
        empty_project()
    }

    #[test]
    fn an_address_whose_linked_objects_agree_gets_an_inferred_type() {
        let project = project_with_two_links(
            DptRef {
                main: 1,
                sub: Some(1),
            },
            DptRef {
                main: 1,
                sub: Some(1),
            },
        );
        let out = infer_group_address_dpts(&project);
        assert_eq!(out.inferred.len(), 1);
        assert_eq!(
            out.inferred[0].dpt,
            DptRef {
                main: 1,
                sub: Some(1)
            }
        );
        assert_eq!(out.inferred[0].from.len(), 2);
        assert_eq!(out.conflicts, vec![]);
    }

    #[test]
    fn an_address_whose_linked_objects_disagree_is_a_conflict_with_no_winner() {
        let project = project_with_two_links(
            DptRef {
                main: 1,
                sub: Some(1),
            },
            DptRef {
                main: 5,
                sub: Some(1),
            },
        );
        let out = infer_group_address_dpts(&project);
        assert_eq!(out.inferred, vec![]);
        assert_eq!(out.conflicts.len(), 1);
        assert_eq!(out.conflicts[0].candidates.len(), 2);
    }

    #[test]
    fn an_orphan_address_infers_nothing_and_is_not_a_conflict() {
        let project = project_with_orphan_address();
        let out = infer_group_address_dpts(&project);
        assert_eq!(out.inferred, vec![]);
        assert_eq!(out.conflicts, vec![]);
    }

    #[test]
    #[ignore = "requires the gitignored OriginalData/ corpus; run with --ignored"]
    fn the_reference_project_has_no_datapoint_type_conflicts() {
        assert!(
            crate::testutil::corpus_available(),
            "OriginalData/ corpus not present (gitignored, local-only); this test is #[ignore]d and must be run explicitly on a machine that has it"
        );
        let out = infer_group_address_dpts(&reference_project());
        assert_eq!(
            out.conflicts,
            vec![],
            "conflicts are not present in this sample"
        );
        // 110 of 514 addresses have no linked object at all (RESEARCH §6.1), so
        // at most 404 can be inferred, and only those whose links state a type
        // are.
        assert!(out.inferred.len() <= 404);
    }
}
