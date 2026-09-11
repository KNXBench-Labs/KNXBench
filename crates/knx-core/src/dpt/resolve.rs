//! Which datapoint type a group address carries, inferred from the
//! communication objects linked to it.
//!
//! This is a different question from `codec.rs`'s: that module turns a
//! known `DptRef` into and out of wire bytes; this one decides *which*
//! `DptRef`, if any, applies to a given `GroupAddressId` — a group address
//! has no DPT field of its own (`group.rs`), only the com objects linked to
//! it do, and they can disagree. Resolution never guesses: where the
//! project model does not state a DPT, or states more than one, the answer
//! is `None` or `Conflict`, never a pick.

use std::collections::HashMap;

use crate::ids::GroupAddressId;
use crate::project::Project;

use super::DptRef;

/// The datapoint type a group address resolves to, from the communication
/// objects linked to it.
///
/// `None` is the ordinary case, not a failure: 194 of the reference
/// project's 514 group addresses (38%) resolve this way, 110 of those
/// because nothing links to them at all (RESEARCH.md §6.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupAddressDpt {
    /// No linked communication object states a usable DPT.
    None,
    /// Every linked communication object that states a DPT states the same
    /// one.
    Single(DptRef),
    /// Linked communication objects disagree. Sorted and deduplicated,
    /// holding at least two distinct entries — resolution reports the
    /// disagreement, it does not settle it.
    Conflict(Vec<DptRef>),
}

/// Resolves the datapoint type of `ga`, from every communication object in
/// `project` that links to it, in either direction.
///
/// - Only `Override::Value` counts as "this instance states a DPT".
///   `Override::Absent` (the attribute was never present) and
///   `Override::Malformed` (present but unparsable) both mean nothing usable
///   was stated. `Override::Empty` means the same for a different reason: it
///   is ETS recording that the attribute was present and *deliberately
///   cleared* (ADR-0010), not that it was never set. Treating a cleared
///   value as if it still applied would invent a DPT the project does not
///   state.
/// - `Direction` is not filtered on: a `GroupLink` in either direction
///   describes the same value on the same address, per RESEARCH.md §6.1.
/// - `is_active` is deliberately **not** filtered on. An inactive
///   communication object still documents what the address carries, and
///   nothing in the project's research record rules otherwise — a reviewer
///   who disagrees should do so on purpose, which is why this sentence
///   exists.
/// - A `ga` that names no group address in `project` at all resolves to
///   `None`, the same as one nothing links to. This function never panics
///   on an unknown id; it has no way to tell "unknown" from "known but
///   unlinked" apart, and no caller listed in the spec needs it to.
pub fn resolve_group_address_dpt(project: &Project, ga: GroupAddressId) -> GroupAddressDpt {
    let dpts: Vec<DptRef> = project
        .devices
        .com_objects()
        // `.any()`, not a count: an object with two links to `ga` (one per
        // `Direction`) still contributes its DPT once — one object states
        // one DPT, and direction says nothing about the type. Do not turn
        // this into a per-link count.
        .filter(|com| com.links.iter().any(|link| link.ga == ga))
        .filter_map(|com| com.dpt.value().map(|resolved| resolved.value))
        .collect();
    group_address_dpt_from(dpts)
}

/// Resolves every group address in `project` to its `GroupAddressDpt`,
/// keyed on the raw 16-bit address a telegram actually carries.
///
/// Spec E4-D8 has a bus monitor resolve once at startup rather than per
/// telegram; this is that one pass. The map holds an entry only for
/// addresses that resolved to `Single` or `Conflict` — a missing key means
/// "no DPT resolved", which is what a caller wants to print anyway, so
/// `GroupAddressDpt::None` is never inserted.
///
/// A project holds several installations, each with its own
/// `GroupAddressId`s, so two entries — even from different installations —
/// can share one raw address. A telegram only ever carries the raw address,
/// so if those entries disagree, the map reports the merged `Conflict` of
/// everything any of them state; a reading that is ambiguous on the bus is
/// reported as ambiguous, not resolved by picking one installation over the
/// other.
pub fn resolve_project_group_address_dpts(project: &Project) -> HashMap<u16, GroupAddressDpt> {
    let mut merged: HashMap<u16, Vec<DptRef>> = HashMap::new();
    for installation in &project.installations {
        for entry in &installation.group_addresses {
            let dpts = match resolve_group_address_dpt(project, entry.id) {
                GroupAddressDpt::None => continue,
                GroupAddressDpt::Single(dpt) => vec![dpt],
                GroupAddressDpt::Conflict(dpts) => dpts,
            };
            merged.entry(entry.address.raw()).or_default().extend(dpts);
        }
    }
    merged
        .into_iter()
        .map(|(raw, dpts)| (raw, group_address_dpt_from(dpts)))
        .collect()
}

/// Sorts and deduplicates `dpts`, then classifies the result: no entries is
/// `None`, one distinct entry is `Single`, more than one is `Conflict`. The
/// sort order is `DptRef`'s own derived `Ord` (main type, then subtype), so
/// two runs over the same input always produce the same `Conflict` vector.
fn group_address_dpt_from(mut dpts: Vec<DptRef>) -> GroupAddressDpt {
    dpts.sort();
    dpts.dedup();
    match dpts.len() {
        0 => GroupAddressDpt::None,
        1 => GroupAddressDpt::Single(dpts[0]),
        _ => GroupAddressDpt::Conflict(dpts),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::address::GroupAddress;
    use crate::commissioning::CompletionStatus;
    use crate::device::ComObjectInstance;
    use crate::flags::{Direction, GroupLink, ResolvedFlags};
    use crate::group::GroupAddressEntry;
    use crate::ids::{ComObjectInstanceId, DeviceId, InstallationId, SourceRef};
    use crate::installation::Installation;
    use crate::provenance::{Layer, Override, Resolved};
    use crate::string_table::Language;
    use crate::topology::Topology;

    fn source() -> SourceRef {
        SourceRef {
            path: "t".into(),
            ets_id: "t".into(),
        }
    }

    fn dpt(main: u16, sub: u16) -> DptRef {
        DptRef {
            main,
            sub: Some(sub),
        }
    }

    fn stated(main: u16, sub: u16) -> Override<DptRef> {
        Override::Value(Resolved {
            value: dpt(main, sub),
            layer: Layer::Instance,
        })
    }

    /// Builds a `ComObjectInstance` with just the fields these tests vary:
    /// its stated DPT, the group links it carries, and whether it is
    /// active. Every other field is a fixed, irrelevant placeholder — this
    /// exists so a test reads as "this com object states X and links to Y",
    /// not twenty lines of boilerplate repeated per case.
    fn com_object(
        id: u32,
        dpt: Override<DptRef>,
        links: Vec<GroupLink>,
        is_active: bool,
    ) -> ComObjectInstance {
        ComObjectInstance {
            id: ComObjectInstanceId(id),
            source: source(),
            device: DeviceId(1),
            number: 0,
            text: Override::Absent,
            description: Override::Absent,
            dpt,
            flags: ResolvedFlags::none(),
            size: None,
            is_active,
            links,
            module_instance: None,
        }
    }

    fn send(ga: GroupAddressId) -> GroupLink {
        GroupLink {
            ga,
            direction: Direction::Send,
        }
    }

    fn receive(ga: GroupAddressId) -> GroupLink {
        GroupLink {
            ga,
            direction: Direction::Receive,
        }
    }

    /// A project whose `Devices` owns exactly `com_objects` and nothing
    /// else. `resolve_group_address_dpt` only ever reads `Devices`'
    /// com-object map, so no `DeviceInstance` or installation needs to
    /// exist for these tests to be meaningful.
    fn project_with(com_objects: Vec<ComObjectInstance>) -> Project {
        let mut project = Project::new(Language("en".into()));
        for com in com_objects {
            project.devices.insert_com_object(com);
        }
        project
    }

    fn group_address_entry(id: u32, raw: u16) -> GroupAddressEntry {
        GroupAddressEntry {
            id: GroupAddressId(id),
            source: source(),
            name: "GA".into(),
            address: GroupAddress::from_raw(raw),
            central: false,
            unfiltered: false,
            range: None,
        }
    }

    fn installation(id: u8, group_addresses: Vec<GroupAddressEntry>) -> Installation {
        Installation {
            id: InstallationId(id),
            name: "I".into(),
            default_line: None,
            multicast_address: None,
            completion: CompletionStatus::FinishedDesign,
            topology: Topology {
                areas: vec![],
                lines: vec![],
                unassigned: vec![],
            },
            buildings: vec![],
            group_ranges: vec![],
            group_addresses,
            parameters: vec![],
        }
    }

    #[test]
    fn no_linked_communication_object_resolves_to_none() {
        let project = project_with(vec![]);
        assert_eq!(
            resolve_group_address_dpt(&project, GroupAddressId(1)),
            GroupAddressDpt::None
        );
    }

    #[test]
    fn one_linked_object_stating_a_dpt_resolves_to_single() {
        let ga = GroupAddressId(1);
        let project = project_with(vec![com_object(1, stated(1, 1), vec![send(ga)], true)]);
        assert_eq!(
            resolve_group_address_dpt(&project, ga),
            GroupAddressDpt::Single(dpt(1, 1))
        );
    }

    #[test]
    fn two_linked_objects_stating_the_same_dpt_resolve_to_single_not_a_one_element_conflict() {
        let ga = GroupAddressId(1);
        let project = project_with(vec![
            com_object(1, stated(1, 1), vec![send(ga)], true),
            com_object(2, stated(1, 1), vec![receive(ga)], true),
        ]);
        assert_eq!(
            resolve_group_address_dpt(&project, ga),
            GroupAddressDpt::Single(dpt(1, 1))
        );
    }

    #[test]
    fn two_linked_objects_stating_different_dpts_resolve_to_a_sorted_deduplicated_conflict() {
        let ga = GroupAddressId(1);
        // Three objects, one DPT repeated, prove both that both states
        // survive and that the repeat does not produce a third entry.
        let project = project_with(vec![
            com_object(1, stated(5, 1), vec![send(ga)], true),
            com_object(2, stated(1, 1), vec![send(ga)], true),
            com_object(3, stated(5, 1), vec![receive(ga)], true),
        ]);
        assert_eq!(
            resolve_group_address_dpt(&project, ga),
            GroupAddressDpt::Conflict(vec![dpt(1, 1), dpt(5, 1)])
        );
    }

    #[test]
    fn override_absent_contributes_nothing() {
        let ga = GroupAddressId(1);
        let project = project_with(vec![com_object(1, Override::Absent, vec![send(ga)], true)]);
        assert_eq!(
            resolve_group_address_dpt(&project, ga),
            GroupAddressDpt::None
        );
    }

    #[test]
    fn override_empty_contributes_nothing_because_ets_cleared_it_not_left_it_unset() {
        // ADR-0010: `Override::Empty` is ETS recording that the attribute
        // was present with an empty value — deliberately cleared, not
        // unset. Resolution must not treat a cleared DPT as a stated one.
        let ga = GroupAddressId(1);
        let project = project_with(vec![com_object(1, Override::Empty, vec![send(ga)], true)]);
        assert_eq!(
            resolve_group_address_dpt(&project, ga),
            GroupAddressDpt::None
        );
    }

    #[test]
    fn override_malformed_contributes_nothing() {
        let ga = GroupAddressId(1);
        let project = project_with(vec![com_object(
            1,
            Override::Malformed("bogus".into()),
            vec![send(ga)],
            true,
        )]);
        assert_eq!(
            resolve_group_address_dpt(&project, ga),
            GroupAddressDpt::None
        );
    }

    #[test]
    fn a_link_in_each_direction_counts() {
        let ga = GroupAddressId(1);
        let project = project_with(vec![
            com_object(1, stated(1, 1), vec![send(ga)], true),
            com_object(2, stated(9, 1), vec![receive(ga)], true),
        ]);
        // Both contributed, or this would be `Single`, not `Conflict`.
        assert_eq!(
            resolve_group_address_dpt(&project, ga),
            GroupAddressDpt::Conflict(vec![dpt(1, 1), dpt(9, 1)])
        );
    }

    #[test]
    fn one_object_linked_to_the_same_address_in_both_directions_counts_once_not_twice() {
        let ga = GroupAddressId(1);
        // One object, two links to the same `ga` — one Send, one Receive.
        // It still states one DPT, so this must resolve to `Single`, never
        // a two-element `Conflict` manufactured out of counting the same
        // object's DPT twice.
        let project = project_with(vec![com_object(
            1,
            stated(1, 1),
            vec![send(ga), receive(ga)],
            true,
        )]);
        assert_eq!(
            resolve_group_address_dpt(&project, ga),
            GroupAddressDpt::Single(dpt(1, 1))
        );
    }

    #[test]
    fn an_inactive_communication_object_still_counts() {
        let ga = GroupAddressId(1);
        let project = project_with(vec![com_object(1, stated(1, 1), vec![send(ga)], false)]);
        assert_eq!(
            resolve_group_address_dpt(&project, ga),
            GroupAddressDpt::Single(dpt(1, 1))
        );
    }

    #[test]
    fn an_unknown_group_address_id_resolves_to_none_without_panicking() {
        let ga = GroupAddressId(1);
        let project = project_with(vec![com_object(1, stated(1, 1), vec![send(ga)], true)]);
        assert_eq!(
            resolve_group_address_dpt(&project, GroupAddressId(999)),
            GroupAddressDpt::None
        );
    }

    #[test]
    fn the_map_omits_none_addresses_and_retains_conflict_ones() {
        let unlinked = group_address_entry(1, 100); // nothing links to it
        let single = group_address_entry(2, 200);
        let conflicting = group_address_entry(3, 300);
        let mut project = project_with(vec![
            com_object(1, stated(1, 1), vec![send(single.id)], true),
            com_object(2, stated(1, 1), vec![send(conflicting.id)], true),
            com_object(3, stated(5, 1), vec![send(conflicting.id)], true),
        ]);
        project.installations = vec![installation(0, vec![unlinked, single, conflicting])];

        let map = resolve_project_group_address_dpts(&project);

        assert_eq!(map.get(&100), None);
        assert_eq!(map.get(&200), Some(&GroupAddressDpt::Single(dpt(1, 1))));
        assert_eq!(
            map.get(&300),
            Some(&GroupAddressDpt::Conflict(vec![dpt(1, 1), dpt(5, 1)]))
        );
    }

    #[test]
    fn two_installations_sharing_a_raw_address_with_different_dpts_merge_into_one_conflict() {
        let entry_in_installation_0 = group_address_entry(1, 500);
        let entry_in_installation_1 = group_address_entry(2, 500); // same raw address
        let mut project = project_with(vec![
            com_object(
                1,
                stated(1, 1),
                vec![send(entry_in_installation_0.id)],
                true,
            ),
            com_object(
                2,
                stated(5, 1),
                vec![send(entry_in_installation_1.id)],
                true,
            ),
        ]);
        project.installations = vec![
            installation(0, vec![entry_in_installation_0]),
            installation(1, vec![entry_in_installation_1]),
        ];

        let map = resolve_project_group_address_dpts(&project);

        assert_eq!(
            map.get(&500),
            Some(&GroupAddressDpt::Conflict(vec![dpt(1, 1), dpt(5, 1)]))
        );
    }

    #[test]
    fn an_installation_resolving_single_merges_with_another_resolving_conflict_on_the_same_raw_address(
    ) {
        // Installation 0's own group address has exactly one linked object,
        // so `resolve_group_address_dpt` resolves it to a plain `Single`.
        // Installation 1's own group address, same raw value, already has
        // two disagreeing linked objects and resolves to `Conflict` all by
        // itself, before the map ever combines anything. The map must not
        // let the `Single` side win, or drop either of the `Conflict`
        // side's entries — it pools everything stated for that raw address
        // and reclassifies once.
        let single_entry = group_address_entry(1, 700);
        let conflict_entry = group_address_entry(2, 700);
        let mut project = project_with(vec![
            com_object(1, stated(1, 1), vec![send(single_entry.id)], true),
            com_object(2, stated(5, 1), vec![send(conflict_entry.id)], true),
            com_object(3, stated(9, 1), vec![send(conflict_entry.id)], true),
        ]);
        project.installations = vec![
            installation(0, vec![single_entry]),
            installation(1, vec![conflict_entry]),
        ];

        let map = resolve_project_group_address_dpts(&project);

        assert_eq!(
            map.get(&700),
            Some(&GroupAddressDpt::Conflict(vec![
                dpt(1, 1),
                dpt(5, 1),
                dpt(9, 1)
            ]))
        );
    }
}
