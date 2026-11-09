//! Which hidden choose controllers an active `Assign` sets, and whether it agrees (ADR-0168).

use std::collections::HashSet;
use std::rc::Rc;

use super::{Diagnostic, ModuleScope, ScopeKey, ValueMap};

/// What one `Assign` element declares, read from its stored attributes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assignment {
    /// `@TargetParamRefRef`: the `ParameterRef` the assignment sets.
    pub target: String,
    /// Where the assigned value comes from.
    pub source: AssignSource,
}

/// `@SourceParamRefRef` or `@Value`; the schema allows exactly one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssignSource {
    /// The value another `ParameterRef` holds.
    Parameter(String),
    /// A literal value.
    Value(String),
}

/// Reads an `Assign` row's `dynamic_node.extra`. That column is an audit
/// trail, not an encoding (`parse.rs`), so the reading is strict: exactly
/// `SourceParamRefRef` + `TargetParamRefRef`, or `TargetParamRefRef` +
/// `Value`, each once, in the sorted order the writer produces, with
/// identifier-shaped references. Anything else is unreadable (`None`).
pub(crate) fn parse_stored_assignment(extra: &str) -> Option<Assignment> {
    let mut lines = extra.split('\n');
    let first = lines.next()?.split_once('=')?;
    let second = lines.next()?.split_once('=')?;
    if lines.next().is_some() {
        return None;
    }
    let assignment = match (first, second) {
        (("SourceParamRefRef", source), ("TargetParamRefRef", target)) => Assignment {
            target: identifier(target)?,
            source: AssignSource::Parameter(identifier(source)?),
        },
        // `Value` is the last attribute, so a newline inside it would have
        // produced a third line and been refused above.
        (("TargetParamRefRef", target), ("Value", value)) => Assignment {
            target: identifier(target)?,
            source: AssignSource::Value(value.to_owned()),
        },
        _ => return None,
    };
    Some(assignment)
}

fn identifier(text: &str) -> Option<String> {
    let shaped = !text.is_empty()
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'));
    shaped.then(|| text.to_owned())
}

/// One `Assign` element a pass reached in an active branch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ActiveAssignment {
    pub(super) node_id: i64,
    pub(super) scope: Option<Rc<ModuleScope>>,
    /// `None`: the stored declaration could not be read.
    pub(super) assignment: Option<Assignment>,
}

/// What the active assignments say about a controller the pass found
/// placed but inactive.
#[derive(Debug, PartialEq)]
pub(super) enum Verdict {
    /// No active assignment sets it: ADR-0160 applies, it selects nothing.
    Unassigned,
    /// Every active assignment setting it yields the value the evaluator
    /// already read for it, so its `choose` is evaluated as it stands.
    Agrees,
    /// An assignment yields another (or an unknown) value: the evaluator
    /// does not execute `Assign`, so the result would not be ETS's.
    Differs(Option<Rc<ModuleScope>>, Diagnostic),
}

/// The verdict for `controller` (a [`ScopeKey`]) under `assignments`.
pub(super) fn verdict(
    controller: &ScopeKey,
    assignments: &[ActiveAssignment],
    values: &ValueMap,
) -> Verdict {
    let mut setting = assignments.iter().filter_map(|active| {
        let assignment = active.assignment.as_ref()?;
        let key = (
            super::scope_key_chain(active.scope.as_ref()),
            assignment.target.clone(),
        );
        (key == *controller).then_some((active, assignment))
    });
    let Some(first) = setting.next() else {
        return Verdict::Unassigned;
    };
    for (active, assignment) in std::iter::once(first).chain(setting) {
        let scope = active.scope.as_deref();
        let observed = values.get(scope, &assignment.target);
        let assigned = match &assignment.source {
            AssignSource::Parameter(source) => values.get(scope, source),
            AssignSource::Value(value) => Some(value.as_str()),
        };
        if !matches!((assigned, observed), (Some(a), Some(o)) if same_value(a, o)) {
            return Verdict::Differs(
                active.scope.clone(),
                Diagnostic::AssignedControllerDiffers {
                    assign_node: active.node_id,
                    param_ref: assignment.target.clone(),
                    assigned_value: assigned.map(str::to_owned),
                    observed_value: observed.map(str::to_owned),
                },
            );
        }
    }
    Verdict::Agrees
}

/// The unreadable active assignments, once each: whether one sets a given
/// controller is unknown, so each is reported as unrecognized.
pub(super) fn unreadable(
    assignments: &[ActiveAssignment],
) -> Vec<(Option<Rc<ModuleScope>>, Diagnostic)> {
    let mut seen = HashSet::new();
    assignments
        .iter()
        .filter(|active| active.assignment.is_none())
        .filter(|active| {
            seen.insert((
                super::scope_key_chain(active.scope.as_ref()),
                active.node_id,
            ))
        })
        .map(|active| {
            (
                active.scope.clone(),
                Diagnostic::UnrecognizedNode {
                    node_id: active.node_id,
                    kind: "Assign".to_owned(),
                },
            )
        })
        .collect()
}

/// Equal as integers when both read as one, else equal as written.
fn same_value(a: &str, b: &str) -> bool {
    match (a.trim().parse::<i64>(), b.trim().parse::<i64>()) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_two_stored_shapes() {
        assert_eq!(
            parse_stored_assignment(
                "SourceParamRefRef=M-1_A-2_P-3_R-4\nTargetParamRefRef=M-1_A-2_P-5_R-6"
            ),
            Some(Assignment {
                target: "M-1_A-2_P-5_R-6".into(),
                source: AssignSource::Parameter("M-1_A-2_P-3_R-4".into()),
            })
        );
        assert_eq!(
            parse_stored_assignment("TargetParamRefRef=M-1_A-2_P-5_R-6\nValue=a=b"),
            Some(Assignment {
                target: "M-1_A-2_P-5_R-6".into(),
                source: AssignSource::Value("a=b".into()),
            })
        );
    }

    #[test]
    fn refuses_anything_ambiguous() {
        for extra in [
            "",
            "TargetParamRefRef=M-1_P-5",
            "TargetParamRefRef=M-1_P-5\nValue=1\nValue=2",
            "TargetParamRefRef=M-1_P-5\nValue=1\nx",
            "SourceParamRefRef=M-1_P-3\nTargetParamRefRef=M-1_P-5\nValue=1",
            "Value=1\nTargetParamRefRef=M-1_P-5",
            "SourceParamRefRef=M 1\nTargetParamRefRef=M-1_P-5",
            "SourceParamRefRef=M-1_P-3\nTargetParamRefRef=",
        ] {
            assert_eq!(parse_stored_assignment(extra), None, "{extra:?}");
        }
    }

    #[test]
    fn compares_numbers_as_numbers_and_text_as_written() {
        assert!(same_value("0", "00"));
        assert!(same_value(" 3", "3"));
        assert!(!same_value("1", "2"));
        assert!(same_value("abc", "abc"));
        assert!(!same_value("abc", "ABC"));
    }
}
