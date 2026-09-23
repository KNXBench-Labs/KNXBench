//! The generic, entity-agnostic matching engine: given two slices of some
//! entity type `E`, decide which items on the left correspond to which
//! items on the right. This module knows nothing about `knx_core` — every
//! per-entity extraction (which field is the ETS id, what the natural key
//! is) is supplied by the caller as a closure, so the same engine matches
//! devices, group addresses, or anything else a later task points it at.

use std::collections::{BTreeMap, BTreeSet};

/// How a matched pair of entities was found to correspond.
#[derive(Debug, Clone, PartialEq)]
pub enum MatchKind {
    /// Both sides carry the same ETS-assigned identifier.
    EtsId,
    /// Neither side shares an ETS id, but a caller-supplied natural key
    /// (e.g. an address) uniquely identifies the pair among the leftovers.
    NaturalKey,
}

/// The final disposition of an entity once a diff has resolved ambiguity.
#[derive(Debug, Clone, PartialEq)]
pub enum EntityStatus {
    Added,
    Removed,
    Matched,
}

/// One matched pair of entities plus the field-level differences between
/// them. `F` is whatever per-entity field-snapshot type a later task
/// extracts (Task 3 onward) — this layer never inspects it.
#[derive(Debug, Clone, PartialEq)]
pub struct EntityChange<K, F> {
    pub key: K,
    pub matched_by: MatchKind,
    pub left: F,
    pub right: F,
    pub changed_fields: Vec<&'static str>,
}

/// A natural key that had more than one candidate on at least one side,
/// recorded so a caller can report it rather than guess.
#[derive(Debug, Clone, PartialEq)]
pub struct AmbiguityNote<K> {
    pub key: K,
    pub left_candidates: usize,
    pub right_candidates: usize,
}

/// A finished, per-entity-type diff: additions, removals, changes and the
/// ambiguities a later task could not resolve on its own.
#[derive(Debug, Clone, PartialEq)]
pub struct EntityTable<K, F> {
    pub added: Vec<(K, F)>,
    pub removed: Vec<(K, F)>,
    pub changed: Vec<EntityChange<K, F>>,
    pub ambiguous: Vec<AmbiguityNote<K>>,
}

/// Two or more candidates on at least one side shared the same natural
/// key, so `match_entities` could not pick a pair on its own.
#[derive(Debug, Clone, PartialEq)]
pub struct AmbiguityGroup<'a, E> {
    pub left: Vec<&'a E>,
    pub right: Vec<&'a E>,
}

/// The raw result of matching `left` against `right`: pairs the engine is
/// confident about, plus everything it could not place. `left_leftover`/
/// `right_leftover` are deliberately not called `removed`/`added` — an
/// entity here may still be one half of an `AmbiguityGroup`; only a
/// caller that has looked at `ambiguous` too (Task 3) may decide an entity
/// is truly added or removed.
pub struct MatchOutcome<'a, E> {
    pub matched: Vec<(&'a E, &'a E, MatchKind)>,
    pub left_leftover: Vec<&'a E>,
    pub right_leftover: Vec<&'a E>,
    pub ambiguous: Vec<AmbiguityGroup<'a, E>>,
}

/// Matches `left` against `right` in two passes.
///
/// Pass 1 matches by `ets_id`: an id present on both sides is a match
/// (`MatchKind::EtsId`) **only if it identifies exactly one entity on each
/// side**. An `ets_id` is not a usable identity when it is duplicated
/// within its own side — a duplicated id is never guessed at, so every
/// entity carrying it falls straight through to pass 2 as an ordinary
/// leftover instead (where the natural key may still resolve it, or the
/// existing ambiguity rule may catch it). Match order follows `left`'s own
/// order, not any hash map's — this function must be deterministic run to
/// run.
///
/// Pass 2 groups each side's pass-1 leftovers by `natural_key` (entities
/// for which it returns `None` skip this pass). A key with exactly one
/// candidate on each side is a match (`MatchKind::NaturalKey`); a key with
/// more than one candidate on either side becomes an `AmbiguityGroup`
/// instead, and none of its members count as matched — but they are not
/// removed from `left_leftover`/`right_leftover` either, since this layer
/// does not decide their final disposition. A key present in only one
/// side's leftovers is left untouched: its members simply remain leftover.
pub fn match_entities<'a, E, NK: Ord + Clone>(
    left: &'a [E],
    right: &'a [E],
    ets_id: impl Fn(&E) -> &str,
    natural_key: impl Fn(&E) -> Option<NK>,
) -> MatchOutcome<'a, E> {
    // Pass 1: ets_id. Indexed by BTreeMap (not HashMap) so a duplicated id
    // within one side is visible as `.len() > 1` rather than silently
    // overwriting an earlier entity — CLAUDE.md forbids discarding data
    // that quietly.
    let mut left_by_id: BTreeMap<&str, Vec<&'a E>> = BTreeMap::new();
    for e in left {
        let id = ets_id(e);
        if !id.is_empty() {
            left_by_id.entry(id).or_default().push(e);
        }
    }
    let mut right_by_id: BTreeMap<&str, Vec<&'a E>> = BTreeMap::new();
    for e in right {
        let id = ets_id(e);
        if !id.is_empty() {
            right_by_id.entry(id).or_default().push(e);
        }
    }

    // Iterate `left` itself, not either map, so match order is the input's
    // order rather than a map's iteration order (map iteration order is
    // deterministic here since both are BTreeMaps, but it is alphabetical
    // by id, not input order — the latter is what callers should see).
    let mut matched: Vec<(&'a E, &'a E, MatchKind)> = Vec::new();
    let mut matched_ets_ids: BTreeSet<&str> = BTreeSet::new();
    for e in left {
        let id = ets_id(e);
        if id.is_empty() {
            continue;
        }
        if matched_ets_ids.contains(id) {
            continue;
        }
        let lv = &left_by_id[id];
        if lv.len() != 1 {
            continue;
        }
        if let Some(rv) = right_by_id.get(id) {
            if rv.len() == 1 {
                matched.push((lv[0], rv[0], MatchKind::EtsId));
                matched_ets_ids.insert(id);
            }
        }
    }

    let left_leftover1: Vec<&'a E> = left
        .iter()
        .filter(|e| ets_id(e).is_empty() || !matched_ets_ids.contains(ets_id(e)))
        .collect();
    let right_leftover1: Vec<&'a E> = right
        .iter()
        .filter(|e| ets_id(e).is_empty() || !matched_ets_ids.contains(ets_id(e)))
        .collect();

    // Pass 2: natural key, over pass-1 leftovers only.
    let mut left_groups: BTreeMap<NK, Vec<&'a E>> = BTreeMap::new();
    for e in left_leftover1.iter().copied() {
        if let Some(k) = natural_key(e) {
            left_groups.entry(k).or_default().push(e);
        }
    }
    let mut right_groups: BTreeMap<NK, Vec<&'a E>> = BTreeMap::new();
    for e in right_leftover1.iter().copied() {
        if let Some(k) = natural_key(e) {
            right_groups.entry(k).or_default().push(e);
        }
    }

    let mut ambiguous: Vec<AmbiguityGroup<'a, E>> = Vec::new();
    let mut resolved_left: BTreeSet<usize> = BTreeSet::new();
    let mut resolved_right: BTreeSet<usize> = BTreeSet::new();

    let mut keys: BTreeSet<NK> = left_groups.keys().cloned().collect();
    keys.extend(right_groups.keys().cloned());

    // A key present on only one side is not touched here: its members
    // simply remain in that side's leftover list below.
    for key in keys {
        if let (Some(lv), Some(rv)) = (left_groups.get(&key), right_groups.get(&key)) {
            if lv.len() == 1 && rv.len() == 1 {
                matched.push((lv[0], rv[0], MatchKind::NaturalKey));
                resolved_left.insert(lv[0] as *const E as usize);
                resolved_right.insert(rv[0] as *const E as usize);
            } else {
                ambiguous.push(AmbiguityGroup {
                    left: lv.clone(),
                    right: rv.clone(),
                });
            }
        }
    }

    let left_leftover: Vec<&'a E> = left_leftover1
        .into_iter()
        .filter(|e| !resolved_left.contains(&(*e as *const E as usize)))
        .collect();
    let right_leftover: Vec<&'a E> = right_leftover1
        .into_iter()
        .filter(|e| !resolved_right.contains(&(*e as *const E as usize)))
        .collect();

    MatchOutcome {
        matched,
        left_leftover,
        right_leftover,
        ambiguous,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    struct Item {
        ets_id: &'static str,
        code: Option<u8>,
    }

    fn id(item: &Item) -> &str {
        item.ets_id
    }

    fn key(item: &Item) -> Option<u8> {
        item.code
    }

    #[test]
    fn ets_id_match_wins_even_when_every_other_field_differs() {
        let left = [Item {
            ets_id: "a",
            code: Some(1),
        }];
        let right = [Item {
            ets_id: "a",
            code: Some(2),
        }];

        let outcome = match_entities(&left, &right, id, key);

        assert_eq!(outcome.matched.len(), 1);
        assert_eq!(outcome.matched[0].2, MatchKind::EtsId);
        assert!(outcome.left_leftover.is_empty());
        assert!(outcome.right_leftover.is_empty());
        assert!(outcome.ambiguous.is_empty());
    }

    #[test]
    fn natural_key_match_is_found_only_among_leftovers() {
        let left = [
            Item {
                ets_id: "a",
                code: Some(5),
            },
            Item {
                ets_id: "b",
                code: Some(5),
            },
        ];
        let right = [
            Item {
                ets_id: "a",
                code: Some(9),
            },
            Item {
                ets_id: "c",
                code: Some(5),
            },
        ];

        let outcome = match_entities(&left, &right, id, key);

        assert_eq!(outcome.matched.len(), 2);
        assert!(outcome
            .matched
            .iter()
            .any(|(l, r, k)| l.ets_id == "a" && r.ets_id == "a" && *k == MatchKind::EtsId));
        assert!(outcome
            .matched
            .iter()
            .any(|(l, r, k)| l.ets_id == "b" && r.ets_id == "c" && *k == MatchKind::NaturalKey));
        assert!(outcome.left_leftover.is_empty());
        assert!(outcome.right_leftover.is_empty());
        assert!(outcome.ambiguous.is_empty());
    }

    #[test]
    fn two_leftover_candidates_sharing_a_natural_key_produce_one_ambiguity_group() {
        let left = [
            Item {
                ets_id: "a",
                code: Some(5),
            },
            Item {
                ets_id: "b",
                code: Some(5),
            },
        ];
        let right = [Item {
            ets_id: "c",
            code: Some(5),
        }];

        let outcome = match_entities(&left, &right, id, key);

        assert!(outcome.matched.is_empty());
        assert_eq!(outcome.ambiguous.len(), 1);
        assert_eq!(outcome.ambiguous[0].left.len(), 2);
        assert_eq!(outcome.ambiguous[0].right.len(), 1);

        // An ambiguity group's members are not subtracted from the
        // leftover lists here — that decision belongs to whichever caller
        // looks at `ambiguous` too (Task 3). Pin that so a later "fix"
        // that starts subtracting them cannot pass silently.
        assert_eq!(outcome.left_leftover.len(), 2);
        assert_eq!(outcome.right_leftover.len(), 1);
        assert!(outcome.left_leftover.iter().any(|e| e.ets_id == "a"));
        assert!(outcome.left_leftover.iter().any(|e| e.ets_id == "b"));
        assert!(outcome.right_leftover.iter().any(|e| e.ets_id == "c"));
    }

    #[test]
    fn a_duplicate_ets_id_on_one_side_falls_through_to_the_natural_key_pass() {
        let left = [
            Item {
                ets_id: "x",
                code: None,
            },
            Item {
                ets_id: "x",
                code: None,
            },
        ];
        let right = [Item {
            ets_id: "x",
            code: None,
        }];

        let outcome = match_entities(&left, &right, id, key);

        // A duplicated ets_id is not a usable identity, so pass 1 must not
        // guess a match for it.
        assert!(outcome
            .matched
            .iter()
            .all(|(_, _, k)| *k != MatchKind::EtsId));
        // Nothing vanishes: all three land somewhere accounted for. None
        // of them carries a natural key here, so pass 2 leaves them as
        // plain leftovers rather than matching or grouping them.
        assert_eq!(outcome.left_leftover.len(), 2);
        assert_eq!(outcome.right_leftover.len(), 1);
        assert!(outcome.ambiguous.is_empty());
    }

    #[test]
    fn a_natural_key_with_candidates_on_only_one_side_is_a_plain_leftover_not_an_ambiguity() {
        let left = [
            Item {
                ets_id: "a",
                code: Some(5),
            },
            Item {
                ets_id: "b",
                code: Some(5),
            },
        ];
        let right: [Item; 0] = [];

        let outcome = match_entities(&left, &right, id, key);

        assert!(outcome.matched.is_empty());
        assert!(outcome.ambiguous.is_empty());
        assert_eq!(outcome.left_leftover.len(), 2);
        assert!(outcome.right_leftover.is_empty());
    }

    #[test]
    fn an_item_with_no_natural_key_and_no_ets_id_match_is_an_unrelated_leftover() {
        let left = [Item {
            ets_id: "a",
            code: None,
        }];
        let right = [Item {
            ets_id: "b",
            code: None,
        }];

        let outcome = match_entities(&left, &right, id, key);

        assert!(outcome.matched.is_empty());
        assert!(outcome.ambiguous.is_empty());
        assert_eq!(outcome.left_leftover.len(), 1);
        assert_eq!(outcome.right_leftover.len(), 1);
    }

    #[test]
    fn empty_ets_ids_are_absent_identity_not_a_match() {
        let left = [Item {
            ets_id: "",
            code: None,
        }];
        let right = [Item {
            ets_id: "",
            code: None,
        }];

        let outcome = match_entities(&left, &right, id, key);

        assert!(outcome.matched.is_empty());
        assert_eq!(outcome.left_leftover.len(), 1);
        assert_eq!(outcome.right_leftover.len(), 1);
        assert!(outcome.ambiguous.is_empty());
    }

    #[test]
    fn identical_collections_produce_only_matches() {
        let left = [
            Item {
                ets_id: "a",
                code: Some(1),
            },
            Item {
                ets_id: "b",
                code: Some(2),
            },
        ];
        let right = left.clone();

        let outcome = match_entities(&left, &right, id, key);

        assert_eq!(outcome.matched.len(), 2);
        assert!(outcome
            .matched
            .iter()
            .all(|(_, _, k)| *k == MatchKind::EtsId));
        assert!(outcome.left_leftover.is_empty());
        assert!(outcome.right_leftover.is_empty());
        assert!(outcome.ambiguous.is_empty());
    }
}
