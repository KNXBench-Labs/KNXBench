//! Structural resolver from a program-side `Module/@Id` to its one imported `ModuleInstance`.
//!
//! Grammar (ADR-0189): `<program prefix>_<RefId>[_MI-<k>]`, where `<RefId>` is the instance's
//! `source.ets_id` (e.g. `MD-2_M-4`). The `RefId` must end the id (before an optional `MI-`
//! tail) and start at an `_` component boundary, so `MD-2_M-4` never owns `…_MD-2_M-41`. A
//! carried counter must equal the instance's own `instance_counter()`. Anything that is not
//! exactly one such instance is a typed refusal; input order never changes the outcome.

use crate::ids::ModuleInstanceId;
use crate::module::ModuleInstance;

/// Outcome of [`resolve_module_owner`]. Only `Resolved` may be acted on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModuleOwner<'a> {
    /// Exactly one instance owns the id.
    Resolved(&'a ModuleInstance),
    /// No instance's `RefId` ends the id on a component boundary.
    NoMatch,
    /// The id carries an `_MI-` tail that is not all ASCII digits (or overflows); holds the
    /// instances whose `RefId` ends the id before that tail (the refused set, sorted).
    MalformedModuleId(Vec<ModuleInstanceId>),
    /// A candidate's own `instance_ets_id` does not decompose as `<RefId>_MI-<digits>`.
    MalformedInstance(Vec<ModuleInstanceId>),
    /// Several instances with different `RefId`s match the same id.
    ConflictingRefs(Vec<ModuleInstanceId>),
    /// The id carries `MI-<k>` but no candidate has counter `k`.
    CounterMismatch(Vec<ModuleInstanceId>),
    /// More than one candidate remains (no counter in the id, or duplicate owners).
    Ambiguous(Vec<ModuleInstanceId>),
}

/// Split an optional trailing `_MI-<digits>` off `module_id`: `(base, counter)`.
fn split_counter(module_id: &str) -> Result<(&str, Option<u32>), ()> {
    let Some(marker) = module_id.rfind("_MI-") else {
        return Ok((module_id, None));
    };
    let digits = &module_id[marker + 4..];
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(());
    }
    digits
        .parse()
        .map(|k| (&module_id[..marker], Some(k)))
        .map_err(|_| ())
}

/// True when `base` ends with `RefId` preceded by an `_` component boundary.
fn ends_with_component(base: &str, ref_id: &str) -> bool {
    !ref_id.is_empty()
        && base
            .strip_suffix(ref_id)
            .is_some_and(|head| head.ends_with('_'))
}

fn sorted_ids(found: &[&ModuleInstance]) -> Vec<ModuleInstanceId> {
    let mut ids: Vec<ModuleInstanceId> = found.iter().map(|m| m.id).collect();
    ids.sort();
    ids
}

/// Resolve `module_id` against a device's `instances`; see the module docs for the grammar.
pub fn resolve_module_owner<'a>(
    module_id: &str,
    instances: &'a [ModuleInstance],
) -> ModuleOwner<'a> {
    let Ok((base, counter)) = split_counter(module_id) else {
        let base = module_id
            .rfind("_MI-")
            .map_or(module_id, |i| &module_id[..i]);
        let found: Vec<&ModuleInstance> = instances
            .iter()
            .filter(|m| ends_with_component(base, &m.source.ets_id))
            .collect();
        return ModuleOwner::MalformedModuleId(sorted_ids(&found));
    };
    let candidates: Vec<&ModuleInstance> = instances
        .iter()
        .filter(|m| ends_with_component(base, &m.source.ets_id))
        .collect();
    if candidates.is_empty() {
        return ModuleOwner::NoMatch;
    }
    let malformed: Vec<&ModuleInstance> = candidates
        .iter()
        .copied()
        .filter(|m| m.instance_counter().is_none())
        .collect();
    if !malformed.is_empty() {
        return ModuleOwner::MalformedInstance(sorted_ids(&malformed));
    }
    if candidates
        .iter()
        .any(|m| m.source.ets_id != candidates[0].source.ets_id)
    {
        return ModuleOwner::ConflictingRefs(sorted_ids(&candidates));
    }
    let remaining: Vec<&ModuleInstance> = match counter {
        Some(k) => candidates
            .iter()
            .copied()
            .filter(|m| m.instance_counter() == Some(k))
            .collect(),
        None => candidates.clone(),
    };
    match remaining.as_slice() {
        [] => ModuleOwner::CounterMismatch(sorted_ids(&candidates)),
        [one] => ModuleOwner::Resolved(one),
        many => ModuleOwner::Ambiguous(sorted_ids(many)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{DeviceId, SourceRef};

    fn inst(id: u32, ref_id: &str, instance: &str) -> ModuleInstance {
        ModuleInstance {
            id: ModuleInstanceId(id),
            device: DeviceId(1),
            source: SourceRef {
                path: "t".into(),
                ets_id: ref_id.into(),
            },
            repeat_index: String::new(),
            instance_ets_id: instance.into(),
            arguments: vec![],
        }
    }

    fn owner_id(o: ModuleOwner<'_>) -> Option<u32> {
        match o {
            ModuleOwner::Resolved(m) => Some(m.id.0),
            _ => None,
        }
    }

    #[test]
    fn m4_never_owns_m41_and_vice_versa() {
        let only4 = [inst(1, "MD-2_M-4", "MD-2_M-4_MI-1")];
        assert_eq!(
            resolve_module_owner("A-1_MD-2_M-41_MI-1", &only4),
            ModuleOwner::NoMatch
        );
        assert_eq!(
            resolve_module_owner("A-1_MD-2_M-41", &only4),
            ModuleOwner::NoMatch
        );
        let both = [
            inst(1, "MD-2_M-4", "MD-2_M-4_MI-1"),
            inst(2, "MD-2_M-41", "MD-2_M-41_MI-1"),
        ];
        assert_eq!(
            owner_id(resolve_module_owner("A-1_MD-2_M-41_MI-1", &both)),
            Some(2)
        );
        assert_eq!(
            owner_id(resolve_module_owner("A-1_MD-2_M-4_MI-1", &both)),
            Some(1)
        );
        // No leading boundary: `XMD-2_M-4` is not component `MD-2_M-4`.
        assert_eq!(
            resolve_module_owner("A-1_XMD-2_M-4", &only4),
            ModuleOwner::NoMatch
        );
    }

    #[test]
    fn mi_counter_must_agree_even_for_a_single_candidate() {
        let one = [inst(1, "MD-1_M-1", "MD-1_M-1_MI-1")];
        assert_eq!(
            owner_id(resolve_module_owner("A_MD-1_M-1_MI-1", &one)),
            Some(1)
        );
        assert_eq!(owner_id(resolve_module_owner("A_MD-1_M-1", &one)), Some(1));
        assert!(matches!(
            resolve_module_owner("A_MD-1_M-1_MI-99", &one),
            ModuleOwner::CounterMismatch(_)
        ));
        // MI-1 must not match MI-99 by prefix either.
        let ninety = [inst(9, "MD-1_M-1", "MD-1_M-1_MI-99")];
        assert!(matches!(
            resolve_module_owner("A_MD-1_M-1_MI-1", &ninety),
            ModuleOwner::CounterMismatch(_)
        ));
    }

    #[test]
    fn repeated_instances_are_told_apart_only_by_a_carried_counter() {
        let two = [
            inst(1, "MD-1_M-1", "MD-1_M-1_MI-1"),
            inst(2, "MD-1_M-1", "MD-1_M-1_MI-2"),
        ];
        assert_eq!(
            owner_id(resolve_module_owner("A_MD-1_M-1_MI-2", &two)),
            Some(2)
        );
        assert_eq!(
            owner_id(resolve_module_owner("A_MD-1_M-1_MI-1", &two)),
            Some(1)
        );
        assert_eq!(
            resolve_module_owner("A_MD-1_M-1", &two),
            ModuleOwner::Ambiguous(vec![ModuleInstanceId(1), ModuleInstanceId(2)])
        );
        assert!(matches!(
            resolve_module_owner("A_MD-1_M-1_MI-3", &two),
            ModuleOwner::CounterMismatch(_)
        ));
    }

    #[test]
    fn duplicate_owners_refuse() {
        let dup = [
            inst(1, "MD-1_M-1", "MD-1_M-1_MI-1"),
            inst(2, "MD-1_M-1", "MD-1_M-1_MI-1"),
        ];
        assert_eq!(
            resolve_module_owner("A_MD-1_M-1_MI-1", &dup),
            ModuleOwner::Ambiguous(vec![ModuleInstanceId(1), ModuleInstanceId(2)])
        );
    }

    #[test]
    fn conflicting_refs_refuse() {
        // `M-1` is a component suffix of `A_MD-1_M-1` too.
        let conflict = [
            inst(1, "MD-1_M-1", "MD-1_M-1_MI-1"),
            inst(2, "M-1", "M-1_MI-1"),
        ];
        assert_eq!(
            resolve_module_owner("A_MD-1_M-1_MI-1", &conflict),
            ModuleOwner::ConflictingRefs(vec![ModuleInstanceId(1), ModuleInstanceId(2)])
        );
    }

    #[test]
    fn malformed_ids_refuse() {
        let one = [inst(1, "MD-1_M-1", "MD-1_M-1_MI-1")];
        for bad in [
            "A_MD-1_M-1_MI-",
            "A_MD-1_M-1_MI-x",
            "A_MD-1_M-1_MI-1x",
            "A_MD-1_M-1_MI-99999999999",
        ] {
            assert_eq!(
                resolve_module_owner(bad, &one),
                ModuleOwner::MalformedModuleId(vec![ModuleInstanceId(1)]),
                "{bad}"
            );
        }
        for bad_instance in ["MD-1_M-1", "MD-1_M-1_MI-", "MD-1_M-1_MI-a", "MD-1_M-2_MI-1"] {
            let bad = [inst(7, "MD-1_M-1", bad_instance)];
            assert_eq!(
                resolve_module_owner("A_MD-1_M-1_MI-1", &bad),
                ModuleOwner::MalformedInstance(vec![ModuleInstanceId(7)]),
                "{bad_instance}"
            );
        }
        // One malformed sibling poisons the resolution: never skip it.
        let mixed = [
            inst(1, "MD-1_M-1", "MD-1_M-1_MI-1"),
            inst(2, "MD-1_M-1", "garbage"),
        ];
        assert_eq!(
            resolve_module_owner("A_MD-1_M-1_MI-1", &mixed),
            ModuleOwner::MalformedInstance(vec![ModuleInstanceId(2)])
        );
        assert_eq!(
            resolve_module_owner("A_MD-1_M-1", &[]),
            ModuleOwner::NoMatch
        );
    }

    #[test]
    fn outcome_is_independent_of_input_order() {
        let items = [
            inst(1, "MD-1_M-1", "MD-1_M-1_MI-1"),
            inst(2, "MD-1_M-1", "MD-1_M-1_MI-2"),
            inst(3, "MD-1_M-1", "MD-1_M-1_MI-2"),
            inst(4, "MD-1_M-2", "MD-1_M-2_MI-1"),
        ];
        let perms: [[usize; 4]; 6] = [
            [0, 1, 2, 3],
            [3, 2, 1, 0],
            [1, 3, 0, 2],
            [2, 0, 3, 1],
            [1, 0, 3, 2],
            [3, 0, 2, 1],
        ];
        for id in [
            "A_MD-1_M-1",
            "A_MD-1_M-1_MI-1",
            "A_MD-1_M-1_MI-2",
            "A_MD-1_M-2_MI-1",
            "A_MD-1_M-3",
        ] {
            let expect = resolve_module_owner(id, &items).clone();
            let expect = describe(&expect);
            for p in perms {
                let shuffled: Vec<ModuleInstance> = p.iter().map(|&i| items[i].clone()).collect();
                assert_eq!(
                    describe(&resolve_module_owner(id, &shuffled)),
                    expect,
                    "{id} {p:?}"
                );
            }
        }
    }

    /// Variant plus owner/candidate ids, comparable across differently ordered slices.
    fn describe(o: &ModuleOwner<'_>) -> String {
        match o {
            ModuleOwner::Resolved(m) => format!("resolved {}", m.id.0),
            other => format!("{other:?}"),
        }
    }
}
