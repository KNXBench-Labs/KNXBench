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

/// D39 rules 2-3, verbatim: the instance-matching rule is
/// `module_id.ends_with("_" + instance.source.ets_id)` — the leading
/// underscore is what keeps `MD-1_M-2` from matching a `..._MD-11_M-2`
/// module id. `digits` must be all-ASCII (`\d+`), matching
/// `decompose_module_qualified`'s own definition of a valid `MI-`.
pub fn resolve_mi_authority(
    instances: &[knx_core::ModuleInstance],
    module_id: &str,
) -> MiAuthority {
    let matches: Vec<&knx_core::ModuleInstance> = instances
        .iter()
        .filter(|m| module_id.ends_with(&format!("_{}", m.source.ets_id)))
        .collect();
    match matches.as_slice() {
        [] => MiAuthority::NoMatch,
        [one] => {
            let expected_prefix = format!("{}_MI-", one.source.ets_id);
            match one
                .instance_ets_id
                .strip_prefix(expected_prefix.as_str())
                .filter(|digits| !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()))
            {
                Some(digits) => MiAuthority::Found(digits.to_string()),
                None => MiAuthority::Malformed {
                    source_ets_id: one.source.ets_id.clone(),
                    instance_ets_id: one.instance_ets_id.clone(),
                },
            }
        }
        many => MiAuthority::Ambiguous {
            source_ets_id: many[0].source.ets_id.clone(),
            instance_ets_ids: many.iter().map(|m| m.instance_ets_id.clone()).collect(),
        },
    }
}

/// One device's evaluated `Dynamic` tree, from its stored parameter values
/// (design D21/D42). Shared by the parameter panel and, since ISSUE-08, by
/// the device detail's communication objects, so both read the same
/// activation instead of two implementations drifting apart.
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
        // walk its iterations or decide activation (ADR-0108).
        let matches: Vec<_> = module_instances
            .iter()
            .filter(|m| {
                module_id.ends_with(&format!("_{}", m.source.ets_id))
                    && m.instance_ets_id == format!("{}_MI-{mi_digits}", m.source.ets_id)
            })
            .collect();
        let declared: Vec<_> = declarations
            .iter()
            .filter(|(id, _)| id == &module_id)
            .collect();
        let has_unique_declaration = declared.len() == 1;
        // Namespace association is required for the new read-only map. The
        // existing evaluator also supports legacy declaration aliases; don't
        // silently change that single-instance write/read contract here.
        let has_matching_definition = has_unique_declaration && declared[0].1 == prefix;
        let has_imported_owner = module_instances
            .iter()
            .any(|m| module_id.ends_with(&format!("_{}", m.source.ets_id)));
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
