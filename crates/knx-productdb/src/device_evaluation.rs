//! One device's stored parameter values evaluated against its program's `Dynamic` tree.
//!
//! Moved here from `apps/knx-server` (ADR-0090) so the server's parameter
//! panel and the read-only MCP adapter share one evaluation instead of two
//! drifting implementations. Pure product-data logic over values the
//! project stores: it decides which `ParameterRef`s are active, which stored
//! values are module-scoped, and which are stale, and reports findings as
//! data. Wording for a UI stays with the caller.

use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use rusqlite::Connection;

use crate::dynamic::ModuleScope;
use crate::ProductDbError;

/// A stored value no declared parameter accepts (unknown id, a
/// module-scoped id that does not validate, or the loser of a duplicate).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaleValue {
    pub ets_id: String,
    pub raw: String,
}

/// Something the evaluation noticed about the stored values, in order.
#[derive(Debug, Clone)]
pub enum EvaluationFinding {
    /// Two stored rows target the same unscoped parameter; `kept_raw` won.
    DuplicateUnscopedValue { ets_id: String, kept_raw: String },
    /// Two stored rows resolve to the same module-scoped parameter;
    /// `winner_ets_id` won, `ets_id` became stale.
    DuplicateModuleScopedValue {
        scope: Option<Rc<ModuleScope>>,
        winner_ets_id: String,
        ets_id: String,
        module_id: String,
        declared_id: String,
    },
}

/// Takes the longest run of ASCII digits at the start of `s`, returning
/// `None` for zero digits (`\d+` needs at least one) — `(digits, rest)`.
pub fn take_digits(s: &str) -> Option<(&str, &str)> {
    let end = s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
    if end == 0 {
        None
    } else {
        Some((&s[..end], &s[end..]))
    }
}

/// Hand-rolled stand-in for `^(.*)_M-(\d+)_MI-(\d+)_(.*)$` — no `regex`
/// crate exists anywhere in this workspace (checked; see the parameter
/// editor design D21). A regex engine's greedy `.*` for the first group
/// backtracks only as far as it must, which — since the trailing `.*`
/// matches anything, including empty — is equivalent to picking the
/// *rightmost* position in `ets_id` where the literal
/// `_M-<digits>_MI-<digits>_` shape occurs. This scans left to right and
/// keeps overwriting its candidate on every syntactically valid match, so
/// whatever is left standing after the scan is that rightmost one.
/// Returns `(prefix, module_digits, mi_digits, suffix)`.
pub fn decompose_module_qualified(ets_id: &str) -> Option<(String, String, String, String)> {
    const MARKER: &str = "_M-";
    let mut best: Option<(usize, String, String, String, String)> = None;
    let mut search_from = 0;
    while let Some(relative) = ets_id.get(search_from..).and_then(|tail| tail.find(MARKER)) {
        let start = search_from + relative;
        let after_marker = &ets_id[start + MARKER.len()..];
        if let Some((module_digits, rest)) = take_digits(after_marker) {
            if let Some(rest) = rest.strip_prefix("_MI-") {
                if let Some((mi_digits, rest)) = take_digits(rest) {
                    if let Some(suffix) = rest.strip_prefix('_') {
                        best = Some((
                            start,
                            ets_id[..start].to_string(),
                            module_digits.to_string(),
                            mi_digits.to_string(),
                            suffix.to_string(),
                        ));
                    }
                }
            }
        }
        search_from = start + 1;
    }
    best.map(|(_, prefix, module_digits, mi_digits, suffix)| {
        (prefix, module_digits, mi_digits, suffix)
    })
}

/// D39 rules 2-3: whether one imported `ModuleInstance` can serve as the
/// `MI-` authority for a program-side `module_id`, and if not, exactly
/// why — never a guess, never a default (D40).
///
/// ADR-0177: with the accepted mapping, the `Module/@Id` itself carries the
/// instance counter (`…_<RefId>_M-<m>_MI-<k>`), so a repeated module is
/// resolved by that counter instead of being ambiguous by construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MiAuthority {
    /// Exactly one imported `ModuleInstance` matches, and its
    /// `instance_ets_id` decomposes cleanly — these are the `MI-` digits
    /// a write target uses.
    Found(String),
    /// No imported `ModuleInstance`'s `source.ets_id` is the trailing
    /// component of `module_id` (D39 rule 2, zero matches).
    NoMatch,
    /// Two or more imported `ModuleInstance`s match one `module_id` — a
    /// genuinely repeated module (`MI-` > 1) whose channels this slice
    /// cannot tell apart on the read side (D40). Carries the shared
    /// `RefId` and every matching `instance_ets_id`, for the diagnostic.
    Ambiguous {
        source_ets_id: String,
        instance_ets_ids: Vec<String>,
    },
    /// Exactly one match, but its `instance_ets_id` is empty or does not
    /// decompose as `<source.ets_id>_MI-<digits>` (D39 rule 3) — D38's
    /// migration note treats empty exactly like a missing instance.
    Malformed {
        source_ets_id: String,
        instance_ets_id: String,
    },
}

/// D39 rules 2-3 through the one structural resolver
/// [`knx_core::resolve_module_owner`] (ADR-0177, amended 2026-11-21): the
/// `RefId` must end the program-side `Module/@Id` on a component boundary,
/// an `MI-` tail the id carries must equal the instance counter, and only
/// exactly one such instance is `Found` (carried as its decimal counter).
/// Duplicate, conflicting, counter-mismatching or malformed candidates are
/// never first-matched: they map to `Ambiguous`/`Malformed`.
pub fn resolve_mi_authority(
    instances: &[knx_core::ModuleInstance],
    module_id: &str,
) -> MiAuthority {
    use knx_core::ModuleOwner;
    let describe = |ids: &[knx_core::ModuleInstanceId]| -> Vec<&knx_core::ModuleInstance> {
        instances.iter().filter(|m| ids.contains(&m.id)).collect()
    };
    let ambiguous = |found: Vec<&knx_core::ModuleInstance>| {
        let mut instance_ets_ids: Vec<String> =
            found.iter().map(|m| m.instance_ets_id.clone()).collect();
        instance_ets_ids.sort();
        MiAuthority::Ambiguous {
            source_ets_id: found
                .iter()
                .map(|m| m.source.ets_id.clone())
                .min()
                .unwrap_or_default(),
            instance_ets_ids,
        }
    };
    match knx_core::resolve_module_owner(module_id, instances) {
        ModuleOwner::Resolved(m) => match m.instance_counter() {
            Some(k) => MiAuthority::Found(k.to_string()),
            None => malformed_of(m),
        },
        ModuleOwner::NoMatch => MiAuthority::NoMatch,
        ModuleOwner::MalformedInstance(ids) => {
            let mut found = describe(&ids);
            found.sort_by(|a, b| a.instance_ets_id.cmp(&b.instance_ets_id));
            found
                .first()
                .map_or(MiAuthority::NoMatch, |m| malformed_of(m))
        }
        // A malformed program-side id has no structural owner; report every
        // instance whose RefId it ends with as the refused set.
        ModuleOwner::MalformedModuleId => {
            let base = module_id
                .rfind("_MI-")
                .map_or(module_id, |i| &module_id[..i]);
            let found: Vec<_> = instances
                .iter()
                .filter(|m| {
                    base.strip_suffix(m.source.ets_id.as_str())
                        .is_some_and(|h| h.ends_with('_'))
                })
                .collect();
            if found.is_empty() {
                MiAuthority::NoMatch
            } else {
                ambiguous(found)
            }
        }
        ModuleOwner::ConflictingRefs(ids)
        | ModuleOwner::CounterMismatch(ids)
        | ModuleOwner::Ambiguous(ids) => ambiguous(describe(&ids)),
    }
}

fn malformed_of(m: &knx_core::ModuleInstance) -> MiAuthority {
    MiAuthority::Malformed {
        source_ets_id: m.source.ets_id.clone(),
        instance_ets_id: m.instance_ets_id.clone(),
    }
}

/// One device's evaluated `Dynamic` tree, from its stored parameter values
/// (design D21/D42). Shared by the parameter panel and, since ISSUE-08, by
/// the device detail's communication objects, so both read the same
/// activation instead of two implementations drifting apart.
#[derive(Debug, Clone, Default)]
pub struct DeviceEvaluation {
    pub ref_ids: HashSet<String>,
    pub supplied: HashMap<String, String>,
    pub validated_scoped: HashMap<(String, String), String>,
    /// The stored id behind each `validated_scoped` key, inverted: the
    /// `(module_id, declared_id)` a stored module-scoped id resolved to.
    pub scoped_ids: HashMap<String, (String, String)>,
    /// Stored id -> (declared module, verbatim imported instance id, declared ref).
    /// Read-only evidence, independent of Repeat expansion and write authority.
    pub instance_scoped_ids: HashMap<String, (String, String, String)>,
    pub stale: Vec<StaleValue>,
    /// Pass A/B findings, in the order the parameter panel lists them.
    pub findings: Vec<EvaluationFinding>,
    pub values: crate::dynamic::ValueMap,
    pub activation: crate::dynamic::Activation,
}

pub fn evaluate_device(
    products: &Connection,
    program_id: &str,
    stored: Vec<(String, String)>,
    module_instances: &[knx_core::ModuleInstance],
) -> Result<DeviceEvaluation, ProductDbError> {
    let ref_ids = crate::query::parameter_ref_ids(products, program_id)?;

    let mut findings: Vec<EvaluationFinding> = Vec::new();

    // Pass A (design D21): sort every stored value into unscoped-supplied,
    // a regex candidate awaiting module-id validation, or outright
    // undecomposable (no verbatim match, no regex match at all).
    let mut supplied: HashMap<String, String> = HashMap::new();
    let mut candidates: Vec<(String, String, String, String, String, String)> = Vec::new();
    let mut stale: Vec<StaleValue> = Vec::new();
    for (ets_id, raw) in stored {
        if ref_ids.contains(&ets_id) {
            // I1 (fix round 2): a second stored row for the same
            // unscoped id must not vanish the way the first committed
            // round let it -- named in a diagnostic and kept in `stale`,
            // the same loud treatment Pass B already gives a module-
            // scoped collision (D41) below.
            if let Some(previous_raw) = supplied.get(&ets_id) {
                findings.push(EvaluationFinding::DuplicateUnscopedValue {
                    ets_id: ets_id.clone(),
                    kept_raw: previous_raw.clone(),
                });
                stale.push(StaleValue { ets_id, raw });
            } else {
                supplied.insert(ets_id, raw);
            }
        } else if let Some((prefix, module_digits, mi_digits, suffix)) =
            decompose_module_qualified(&ets_id)
        {
            candidates.push((ets_id, raw, prefix, module_digits, mi_digits, suffix));
        } else {
            stale.push(StaleValue { ets_id, raw });
        }
    }

    // D42, step 1 of 2: the unscoped-only `ValueMap`, evaluated once to
    // learn which `Module/@Id`s this program's `choose` chain actually
    // reaches — Pass B needs that set before it can validate a single
    // scoped candidate, and `evaluate` is the only place that set is
    // computed (E3: no parallel module-expansion implementation).
    let mut values = crate::dynamic::resolve_values(products, program_id, &supplied)?;
    let trees = crate::dynamic::load_program_trees(products, program_id)?;
    let provisional_activation = crate::dynamic::evaluate(&trees, &values);

    // The declared `Module/@Id` set this provisional activation reached —
    // D21's second half of candidate validation.
    let module_ids: HashSet<String> = provisional_activation
        .parameter_refs
        .iter()
        .filter_map(|r| r.scope.as_ref().and_then(|s| s.module_id.clone()))
        .collect();

    // Pass B (D21, D41): validate every regex candidate against
    // `module_ids` and `ref_ids` as before, plus two new conditions —
    // its `MI-` digits must agree with the one authoritative
    // `ModuleInstance` when one exists (no authority: not checked, so a
    // pre-migration project displays exactly as it did before this
    // slice), and it must not collide with an already-validated row on
    // the same `(module_id, declared_id)` key (no silent overwrite: the
    // loser is `stale`, named alongside the winner in a diagnostic).
    let mut validated_scoped: HashMap<(String, String), String> = HashMap::new();
    let mut validated_scoped_ets_id: HashMap<(String, String), String> = HashMap::new();
    let mut instance_scoped_ids = HashMap::new();
    let declarations: Vec<(String, String)> = products.prepare(
        "SELECT element_id, ref_id FROM dynamic_node WHERE program_id = ?1 AND kind = 'Module' AND element_id IS NOT NULL AND ref_id IS NOT NULL"
    )?.query_map([program_id], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<Result<_, _>>()?;
    let mut instance_winners: HashMap<(String, String, String), String> = HashMap::new();
    for (ets_id, raw, prefix, module_digits, mi_digits, suffix) in candidates {
        let module_id = format!("{prefix}_M-{module_digits}");
        let declared_id = format!("{prefix}_{suffix}");
        // Resolve stored identity even below a skipped Repeat. This does not
        // walk its iterations or decide activation (ADR-0108). The stored
        // `MI-<k>` comparison now goes through the shared authority
        // (ADR-0177): the instance's counter must equal the candidate's
        // `mi_digits`, which is the same all-ASCII-digit check expressed once.
        let carried_id = format!("{module_id}_MI-{mi_digits}");
        let owner = knx_core::resolve_module_owner(&carried_id, module_instances);
        // The stored digits must be the counter's canonical spelling.
        let matches: Vec<&knx_core::ModuleInstance> = match owner {
            knx_core::ModuleOwner::Resolved(m)
                if m.instance_counter().map(|k| k.to_string()) == Some(mi_digits.clone()) =>
            {
                vec![m]
            }
            _ => Vec::new(),
        };
        let declared: Vec<_> = declarations
            .iter()
            .filter(|(id, _)| id == &module_id)
            .collect();
        let has_unique_declaration = declared.len() == 1;
        // Namespace association is required for the new read-only map. The
        // existing evaluator also supports legacy declaration aliases; don't
        // silently change that single-instance write/read contract here.
        let has_matching_definition = has_unique_declaration && declared[0].1 == prefix;
        let has_imported_owner = !matches!(owner, knx_core::ModuleOwner::NoMatch);
        if has_imported_owner && (matches.len() != 1 || !has_unique_declaration) {
            stale.push(StaleValue { ets_id, raw });
            continue;
        }
        if matches.len() == 1 && has_matching_definition && ref_ids.contains(&declared_id) {
            let instance_id = matches[0].instance_ets_id.clone();
            let key = (module_id.clone(), instance_id.clone(), declared_id.clone());
            if let Some(winner) = instance_winners.get(&key) {
                findings.push(EvaluationFinding::DuplicateModuleScopedValue {
                    scope: None,
                    winner_ets_id: winner.clone(),
                    ets_id: ets_id.clone(),
                    module_id: module_id.clone(),
                    declared_id: declared_id.clone(),
                });
                stale.push(StaleValue { ets_id, raw });
                continue;
            }
            instance_winners.insert(key.clone(), ets_id.clone());
            values.insert_instance(key.0, key.1, key.2, raw.clone());
            instance_scoped_ids.insert(
                ets_id.clone(),
                (module_id.clone(), instance_id, declared_id.clone()),
            );
            if !module_ids.contains(&module_id)
                || !matches!(
                    resolve_mi_authority(module_instances, &module_id),
                    MiAuthority::Found(_)
                )
            {
                continue;
            }
        }
        if !module_ids.contains(&module_id) || !ref_ids.contains(&declared_id) {
            stale.push(StaleValue { ets_id, raw });
            continue;
        }
        if let MiAuthority::Found(authoritative_digits) =
            resolve_mi_authority(module_instances, &module_id)
        {
            if authoritative_digits != mi_digits {
                stale.push(StaleValue { ets_id, raw });
                continue;
            }
        }
        let key = (module_id, declared_id);
        if let Some(winner_ets_id) = validated_scoped_ets_id.get(&key) {
            // I2 (fix round 2): every other section-scoped diagnostic
            // carries a real `scope` the UI can filter by; this one used
            // to say `None` despite naming one specific module. The
            // provisional activation already resolved this exact
            // `module_id` (that is what `module_ids.contains` above just
            // checked), so its own `ModuleScope` is looked up rather
            // than reinvented.
            let scope = provisional_activation.parameter_refs.iter().find_map(|r| {
                r.scope
                    .as_ref()
                    .filter(|s| s.module_id.as_deref() == Some(key.0.as_str()))
                    .cloned()
            });
            findings.push(EvaluationFinding::DuplicateModuleScopedValue {
                scope,
                winner_ets_id: winner_ets_id.clone(),
                ets_id: ets_id.clone(),
                module_id: key.0.clone(),
                declared_id: key.1.clone(),
            });
            stale.push(StaleValue { ets_id, raw });
            continue;
        }
        validated_scoped_ets_id.insert(key.clone(), ets_id);
        validated_scoped.insert(key, raw);
    }

    // D42, step 2 of 2: feed the validated scoped values back into the
    // same `ValueMap` and evaluate again, so a module-scoped `choose`
    // sees its own channel's value instead of the program default (D16).
    // Skipped entirely when there is nothing to feed — every corpus
    // project except KV (E2) — since a second `evaluate` over an
    // unchanged `ValueMap` can only reproduce the first activation.
    let activation = if validated_scoped.is_empty() {
        provisional_activation
    } else {
        for ((module_id, ref_id), raw) in validated_scoped.clone() {
            values.insert_scoped(module_id, ref_id, raw);
        }
        crate::dynamic::evaluate(&trees, &values)
    };

    Ok(DeviceEvaluation {
        ref_ids,
        supplied,
        validated_scoped,
        scoped_ids: validated_scoped_ets_id
            .into_iter()
            .map(|(key, ets_id)| (ets_id, key))
            .collect(),
        instance_scoped_ids,
        stale,
        findings,
        values,
        activation,
    })
}

/// Whether one stored value currently counts on the device, as the
/// program's `Dynamic` tree decides it from the stored values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueStatus {
    /// The tree activates the parameter this value belongs to. `Access`
    /// is not applied here and may still hide it from display.
    Active,
    /// A declared parameter the tree does not activate with these values:
    /// the value stays stored, but the parameter panel does not show it.
    Inactive,
    /// No declared parameter accepts the value (unknown id, an
    /// unvalidated module-scoped id, or the loser of a duplicate).
    Stale,
    /// Not decidable: the traversal stopped at a resource budget, or the id
    /// is not among this evaluation's stored values.
    Unknown,
}

impl DeviceEvaluation {
    /// False when the traversal stopped at a work or module-expansion
    /// budget (ADR-0062): an activated ref is then still certain, but a
    /// missing one cannot be called inactive.
    pub fn traversal_complete(&self) -> bool {
        !self.activation.diagnostics.iter().any(|scoped| {
            matches!(
                scoped.diagnostic,
                crate::dynamic::Diagnostic::EvaluationWorkBudgetExhausted { .. }
                    | crate::dynamic::Diagnostic::ModuleExpansionBudgetExhausted { .. }
            )
        })
    }

    /// The status of the value stored under `ets_id`. When two stored rows
    /// share an id, this is the status of the row that counts (the first);
    /// a later duplicate row is stale.
    pub fn value_status(&self, ets_id: &str) -> ValueStatus {
        let active = if self.supplied.contains_key(ets_id) {
            self.activation
                .parameter_refs
                .iter()
                .any(|r| r.scope.is_none() && r.ref_id == ets_id)
        } else if let Some((module_id, declared_id)) = self.scoped_ids.get(ets_id) {
            self.activation.parameter_refs.iter().any(|r| {
                &r.ref_id == declared_id
                    && r.scope
                        .as_ref()
                        .is_some_and(|s| s.module_id.as_deref() == Some(module_id.as_str()))
            })
        } else if self.instance_scoped_ids.contains_key(ets_id) {
            // Identity resolved, but no accepted iteration/activation rule.
            return ValueStatus::Unknown;
        } else if self.stale.iter().any(|s| s.ets_id == ets_id) {
            return ValueStatus::Stale;
        } else {
            return ValueStatus::Unknown;
        };
        match (active, self.traversal_complete()) {
            (true, _) => ValueStatus::Active,
            (false, true) => ValueStatus::Inactive,
            (false, false) => ValueStatus::Unknown,
        }
    }
}

#[cfg(test)]
mod owner_tests {
    use super::*;
    use knx_core::{DeviceId, ModuleInstance, ModuleInstanceId, SourceRef};

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

    /// F01: exact boundary, counter agreement and refusal through the shared resolver.
    #[test]
    fn resolve_mi_authority_uses_the_structural_resolver() {
        let m4 = [inst(1, "MD-2_M-4", "MD-2_M-4_MI-1")];
        assert_eq!(
            resolve_mi_authority(&m4, "A-1_MD-2_M-41"),
            MiAuthority::NoMatch
        );
        assert_eq!(
            resolve_mi_authority(&m4, "A-1_MD-2_M-4"),
            MiAuthority::Found("1".into())
        );
        assert!(matches!(
            resolve_mi_authority(&m4, "A-1_MD-2_M-4_MI-99"),
            MiAuthority::Ambiguous { .. }
        ));
        let dup = [
            inst(1, "MD-2_M-4", "MD-2_M-4_MI-1"),
            inst(2, "MD-2_M-4", "MD-2_M-4_MI-1"),
        ];
        let reversed = [dup[1].clone(), dup[0].clone()];
        assert_eq!(
            resolve_mi_authority(&dup, "A-1_MD-2_M-4_MI-1"),
            resolve_mi_authority(&reversed, "A-1_MD-2_M-4_MI-1")
        );
        assert!(matches!(
            resolve_mi_authority(&dup, "A-1_MD-2_M-4_MI-1"),
            MiAuthority::Ambiguous { .. }
        ));
        let bad = [inst(3, "MD-2_M-4", "MD-2_M-4_MI-x")];
        assert!(matches!(
            resolve_mi_authority(&bad, "A-1_MD-2_M-4"),
            MiAuthority::Malformed { .. }
        ));
    }
}
