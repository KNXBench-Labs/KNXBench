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
use crate::provenance::Override;

use super::codec::format_width_bits;
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
///
/// # Before you widen the inputs
///
/// This function reads communication-object links and their resolved DPTs,
/// and nothing else. One thing downstream depends on that narrowness in a
/// way no compiler will notice.
///
/// The web client's bus monitor shows a "stale project" lock, driven by a
/// fingerprint over exactly the facts this function consumes
/// (`apps/knx-web/src/busContext.ts`, `fingerprintProjectContext`). A
/// parameter edit now republishes a `ProjectTree` too, so the publish
/// channel this comment used to warn about is closed. What remains is the
/// fingerprint's own input set: `fingerprintProjectContext` hashes
/// `schema_version` plus, per group address, `address`/`name`/`dpts` —
/// parameters are not among them.
///
/// So: if you make a parameter value influence a com object's DPT, its
/// links or its activity, you have made the bus monitor report
/// `"synced"` over a decode that has silently changed. Widening this
/// function's inputs requires widening the fingerprint too.
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

/// What a group address states about its own type, set beside what its
/// linked communication objects state, and how the two were weighed
/// (ADR-0078). Neither side is discarded: a caller can always show both.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupAddressType {
    /// `GroupAddress/@DatapointType`, exactly as the model holds it.
    pub declared: Override<DptRef>,
    /// What the linked communication objects state
    /// ([`resolve_group_address_dpt`]).
    pub linked: GroupAddressDpt,
    pub outcome: GroupAddressTypeOutcome,
}

/// How [`group_address_type_from`] weighed a declaration against the
/// linked objects. Project Schema23 §1.2.7 allows the two to differ in
/// subtype but requires the same size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupAddressTypeOutcome {
    /// The declaration applies; every linked object that states a type
    /// states the same one, or none states any.
    Declared,
    /// The declaration applies; at least one linked object states a
    /// different type of the same width. The difference stays visible in
    /// `linked`.
    DeclaredDiffersFromLinked,
    /// The declaration and at least one linked object differ in width,
    /// which the schema forbids. No type applies.
    SizeConflict,
    /// The declaration applies, but the width of the declared or a linked
    /// type is unknown (variable length, or a main type outside the codec),
    /// so the size rule could not be checked.
    Unverifiable,
    /// No declaration in the model, but the project's store holds
    /// declarations it could not attribute to an address (ADR-0078 D3). The
    /// linked objects' answer applies and may be incomplete.
    DeclarationNotLifted,
    /// No usable declaration; the linked objects' answer applies.
    Inferred,
}

impl GroupAddressType {
    /// The type a decoder or writer should use, in the existing
    /// `GroupAddressDpt` shape. A `SizeConflict` is a `Conflict` naming the
    /// declaration and every linked type, so callers that already refuse a
    /// conflict refuse this one too.
    pub fn effective(&self) -> GroupAddressDpt {
        let declared = self.declared.value().map(|resolved| resolved.value);
        match (self.outcome, declared) {
            (
                GroupAddressTypeOutcome::Declared
                | GroupAddressTypeOutcome::DeclaredDiffersFromLinked
                | GroupAddressTypeOutcome::Unverifiable,
                Some(dpt),
            ) => GroupAddressDpt::Single(dpt),
            (GroupAddressTypeOutcome::SizeConflict, Some(dpt)) => {
                let mut all = linked_list(&self.linked);
                all.push(dpt);
                group_address_dpt_from(all)
            }
            _ => self.linked.clone(),
        }
    }
}

fn linked_list(linked: &GroupAddressDpt) -> Vec<DptRef> {
    match linked {
        GroupAddressDpt::None => Vec::new(),
        GroupAddressDpt::Single(dpt) => vec![*dpt],
        GroupAddressDpt::Conflict(dpts) => dpts.clone(),
    }
}

/// Weighs a group address's own declaration against what its linked
/// objects state (ADR-0078 D2). `declarations_unlifted` is whether the
/// project still holds declarations it could not attribute
/// (`ProjectInfo::unlifted_group_address_dpt_declarations > 0`).
///
/// `pub` for the same reason as [`group_address_dpt_from`]: a caller that
/// gathered the linked types its own way classifies by this one rule.
pub fn group_address_type_from(
    declared: &Override<DptRef>,
    linked: GroupAddressDpt,
    declarations_unlifted: bool,
) -> GroupAddressType {
    let outcome = match declared.value().map(|resolved| resolved.value) {
        Some(dpt) => {
            let others = linked_list(&linked);
            let width = format_width_bits(dpt.main);
            let mismatch = others.iter().any(|other| {
                matches!((width, format_width_bits(other.main)), (Some(a), Some(b)) if a != b)
            });
            if mismatch {
                GroupAddressTypeOutcome::SizeConflict
            } else if !others.is_empty()
                && (width.is_none()
                    || others
                        .iter()
                        .any(|other| format_width_bits(other.main).is_none()))
            {
                GroupAddressTypeOutcome::Unverifiable
            } else if others.iter().any(|other| *other != dpt) {
                GroupAddressTypeOutcome::DeclaredDiffersFromLinked
            } else {
                GroupAddressTypeOutcome::Declared
            }
        }
        None if matches!(declared, Override::Absent) && declarations_unlifted => {
            GroupAddressTypeOutcome::DeclarationNotLifted
        }
        None => GroupAddressTypeOutcome::Inferred,
    };
    GroupAddressType {
        declared: declared.clone(),
        linked,
        outcome,
    }
}

/// Resolves `ga`'s declaration against its linked objects. An id that names
/// no group address in `project` is treated as undeclared, matching
/// [`resolve_group_address_dpt`]'s treatment of unknown ids.
pub fn resolve_group_address_type(project: &Project, ga: GroupAddressId) -> GroupAddressType {
    let absent = Override::Absent;
    let declared = project
        .installations
        .iter()
        .flat_map(|installation| &installation.group_addresses)
        .find(|entry| entry.id == ga)
        .map_or(&absent, |entry| &entry.declared_dpt);
    group_address_type_from(
        declared,
        resolve_group_address_dpt(project, ga),
        project.info.unlifted_group_address_dpt_declarations > 0,
    )
}

/// Resolves every group address in `project` to its effective
/// `GroupAddressDpt` ([`GroupAddressType::effective`], ADR-0078), keyed on
/// the raw 16-bit address a telegram actually carries.
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
    let unlifted = project.info.unlifted_group_address_dpt_declarations > 0;
    let mut merged: HashMap<u16, Vec<DptRef>> = HashMap::new();
    for installation in &project.installations {
        for entry in &installation.group_addresses {
            let effective = group_address_type_from(
                &entry.declared_dpt,
                resolve_group_address_dpt(project, entry.id),
                unlifted,
            )
            .effective();
            let dpts = match effective {
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
///
/// `pub` so a caller that has already gathered the linked communication
/// objects for its own reasons can classify them by the same rule instead
/// of re-deriving it. `knx-projection` builds one reverse index over every
/// communication object to project both a group address's links and its
/// DPT; calling [`resolve_group_address_dpt`] per address instead would
/// rescan every communication object once per group address, and
/// open-coding the three-way classification there would be a second copy
/// of this rule free to drift from this one.
///
/// Deduplication is what makes it safe to feed this one entry *per link*
/// rather than per communication object: an object linked to the same
/// address in both directions states its DPT once either way. It is not a
/// licence to count links — see [`resolve_group_address_dpt`].
pub fn group_address_dpt_from(mut dpts: Vec<DptRef>) -> GroupAddressDpt {
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
            declared_dpt: Default::default(),
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
    fn one_object_linked_to_the_same_address_in_both_directions_resolves_to_single() {
        let ga = GroupAddressId(1);
        // One object, two links to the same `ga` — one Send, one Receive.
        // It still states one DPT, so this resolves to `Single`.
        //
        // Note what this test does *not* prove: counting per link instead
        // of per object would give the same answer here, because both
        // links carry the identical `DptRef` and `group_address_dpt_from`
        // dedups before it classifies. The per-object rule is therefore
        // unobservable through this function's return type, and is stated
        // at the `.filter()` for the reader rather than pinned by a test
        // that cannot distinguish it.
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

    // --- ADR-0078: declared group-address DPT against the linked objects ---

    fn declared_entry(id: u32, raw: u16, declared: Override<DptRef>) -> GroupAddressEntry {
        GroupAddressEntry {
            declared_dpt: declared,
            ..group_address_entry(id, raw)
        }
    }

    fn outcome(declared: Override<DptRef>, linked: GroupAddressDpt) -> GroupAddressTypeOutcome {
        group_address_type_from(&declared, linked, false).outcome
    }

    #[test]
    fn a_declaration_with_nothing_linked_applies() {
        let ty = group_address_type_from(&stated(9, 1), GroupAddressDpt::None, false);
        assert_eq!(ty.outcome, GroupAddressTypeOutcome::Declared);
        assert_eq!(ty.effective(), GroupAddressDpt::Single(dpt(9, 1)));
    }

    #[test]
    fn a_declaration_matching_every_linked_object_applies() {
        assert_eq!(
            outcome(stated(9, 1), GroupAddressDpt::Single(dpt(9, 1))),
            GroupAddressTypeOutcome::Declared
        );
    }

    #[test]
    fn a_same_width_difference_follows_the_declaration_and_keeps_the_linked_side() {
        let linked = GroupAddressDpt::Conflict(vec![dpt(9, 1), dpt(9, 4)]);
        let ty = group_address_type_from(&stated(9, 1), linked.clone(), false);
        assert_eq!(
            ty.outcome,
            GroupAddressTypeOutcome::DeclaredDiffersFromLinked
        );
        assert_eq!(ty.linked, linked);
        assert_eq!(ty.effective(), GroupAddressDpt::Single(dpt(9, 1)));
    }

    #[test]
    fn a_width_conflict_resolves_to_no_type_and_names_both_sides() {
        // Schema23 §1.2.7: "the sizes must match". 1.001 is 1 bit, 5.001 one
        // octet; neither side may win.
        let ty = group_address_type_from(&stated(1, 1), GroupAddressDpt::Single(dpt(5, 1)), false);
        assert_eq!(ty.outcome, GroupAddressTypeOutcome::SizeConflict);
        assert_eq!(
            ty.effective(),
            GroupAddressDpt::Conflict(vec![dpt(1, 1), dpt(5, 1)])
        );
    }

    #[test]
    fn a_width_conflict_wins_over_an_unknown_width_elsewhere() {
        let linked = GroupAddressDpt::Conflict(vec![dpt(5, 1), dpt(24, 1)]);
        assert_eq!(
            outcome(stated(1, 1), linked),
            GroupAddressTypeOutcome::SizeConflict
        );
    }

    #[test]
    fn an_unknown_width_on_either_side_is_unverifiable_not_a_pass() {
        assert_eq!(
            outcome(stated(24, 1), GroupAddressDpt::Single(dpt(16, 0))),
            GroupAddressTypeOutcome::Unverifiable
        );
        assert_eq!(
            outcome(stated(16, 0), GroupAddressDpt::Single(dpt(28, 1))),
            GroupAddressTypeOutcome::Unverifiable
        );
        let ty = group_address_type_from(&stated(232, 600), GroupAddressDpt::None, false);
        assert_eq!(ty.outcome, GroupAddressTypeOutcome::Declared);
        assert_eq!(ty.effective(), GroupAddressDpt::Single(dpt(232, 600)));
    }

    #[test]
    fn missing_empty_or_malformed_declarations_fall_back_to_the_linked_objects() {
        let linked = GroupAddressDpt::Conflict(vec![dpt(1, 1), dpt(5, 1)]);
        for declared in [
            Override::Absent,
            Override::Empty,
            Override::Malformed("DPST-1-1 DPST-1-2".into()),
        ] {
            let ty = group_address_type_from(&declared, linked.clone(), false);
            assert_eq!(ty.outcome, GroupAddressTypeOutcome::Inferred);
            assert_eq!(ty.declared, declared, "the source declaration is kept");
            assert_eq!(ty.effective(), linked);
        }
    }

    #[test]
    fn only_an_absent_declaration_reports_unlifted_store_data() {
        assert_eq!(
            group_address_type_from(&Override::Absent, GroupAddressDpt::None, true).outcome,
            GroupAddressTypeOutcome::DeclarationNotLifted
        );
        assert_eq!(
            group_address_type_from(&Override::Empty, GroupAddressDpt::None, true).outcome,
            GroupAddressTypeOutcome::Inferred
        );
        assert_eq!(
            group_address_type_from(&stated(1, 1), GroupAddressDpt::None, true).outcome,
            GroupAddressTypeOutcome::Declared
        );
    }

    #[test]
    fn the_project_resolvers_use_the_declaration() {
        let declared = GroupAddressId(1);
        let conflicting = GroupAddressId(2);
        let mut project = project_with(vec![
            com_object(1, stated(9, 4), vec![send(declared)], true),
            com_object(2, stated(5, 1), vec![send(conflicting)], true),
        ]);
        project.installations.push(installation(
            0,
            vec![
                declared_entry(1, 0x0801, stated(9, 1)),
                declared_entry(2, 0x0802, stated(1, 1)),
            ],
        ));
        let ty = resolve_group_address_type(&project, declared);
        assert_eq!(
            ty.outcome,
            GroupAddressTypeOutcome::DeclaredDiffersFromLinked
        );
        assert_eq!(
            resolve_group_address_dpt(&project, declared),
            GroupAddressDpt::Single(dpt(9, 4)),
            "the inference-only function keeps its contract"
        );
        let map = resolve_project_group_address_dpts(&project);
        assert_eq!(map[&0x0801], GroupAddressDpt::Single(dpt(9, 1)));
        assert_eq!(
            map[&0x0802],
            GroupAddressDpt::Conflict(vec![dpt(1, 1), dpt(5, 1)])
        );
    }

    #[test]
    fn the_project_counter_switches_an_undeclared_address_to_not_lifted() {
        let mut project = project_with(vec![]);
        project
            .installations
            .push(installation(0, vec![group_address_entry(1, 0x0801)]));
        assert_eq!(
            resolve_group_address_type(&project, GroupAddressId(1)).outcome,
            GroupAddressTypeOutcome::Inferred
        );
        project.info.unlifted_group_address_dpt_declarations = 3;
        assert_eq!(
            resolve_group_address_type(&project, GroupAddressId(1)).outcome,
            GroupAddressTypeOutcome::DeclarationNotLifted
        );
    }
}
