//! Checked outside-walk text admission: public synthetic scalar invariants.
//!
//! One projection budget, unchanged scope-local substitution, no partial text.
//!

use super::*;

fn scope(value: &str) -> ModuleScope {
    ModuleScope {
        module_node: 1,
        module_id: None,
        module_def_id: "public".into(),
        arguments: vec![BoundArgument {
            name: "name".into(),
            value: value.into(),
        }],
        parent: None,
    }
}

fn remaining(units: usize) -> TextProjectionBudget {
    TextProjectionBudget {
        units_used: MAX_TEXT_PROJECTION_WORK - units,
        exhausted: false,
    }
}

#[test]
fn checked_literal_charges_exact_utf8_input_and_output() {
    let raw = "é🙂";
    let mut budget = remaining(raw.len() * 2);
    assert_eq!(
        substitute_text_checked(raw, None, &mut budget),
        Ok(raw.into())
    );
    assert_eq!(budget.units_used, MAX_TEXT_PROJECTION_WORK);
    assert_eq!(budget.copy_text("x"), Err(TextProjectionError));
    assert_eq!(
        substitute_text_checked("", None, &mut budget),
        Err(TextProjectionError)
    );
}

#[test]
fn checked_named_text_charges_local_lookup_and_exact_utf8_output() {
    let raw = "é{{name}}";
    let scope = scope("🙂");
    let result = "é🙂";
    let cost = raw.len() + scope.arguments.len() + 1 + result.len();
    let mut budget = remaining(cost);
    assert_eq!(
        substitute_text_checked(raw, Some(&scope), &mut budget),
        Ok(result.into())
    );
    assert_eq!(budget.units_used, MAX_TEXT_PROJECTION_WORK);
    let mut refused = remaining(cost - 1);
    assert_eq!(
        substitute_text_checked(raw, Some(&scope), &mut refused),
        Err(TextProjectionError)
    );
}

#[test]
fn checked_names_are_not_rescanned_unknown_numeric_and_incomplete_tokens_stay_visible() {
    let scope = scope("{{later}}");
    let before = scope.clone();
    let mut budget = TextProjectionBudget::default();
    assert_eq!(
        substitute_text_checked(
            "A{{name}}{{0}}{{unknown}}{{unfinished",
            Some(&scope),
            &mut budget
        ),
        Ok("A{{later}}{{0}}{{unknown}}{{unfinished".into()),
    );
    assert_eq!(scope, before);
}

#[test]
fn checked_child_scope_does_not_inherit_parent_arguments() {
    let parent = Rc::new(scope("must-not-appear"));
    let child = ModuleScope {
        module_node: 2,
        module_id: None,
        module_def_id: "public-child".into(),
        arguments: vec![],
        parent: Some(parent),
    };
    let before = child.clone();
    assert_eq!(
        substitute_text_checked(
            "{{name}}",
            Some(&child),
            &mut TextProjectionBudget::default()
        ),
        Ok("{{name}}".into()),
    );
    assert_eq!(child, before);
}

#[test]
fn one_budget_is_shared_across_rendered_strings_and_cached_text_copies() {
    let mut budget = remaining(6);
    assert_eq!(budget.copy_text("ab"), Ok("ab".into()));
    assert_eq!(
        substitute_text_checked("a", None, &mut budget),
        Ok("a".into())
    );
    assert_eq!(budget.admit_copy("ab"), Ok(()));
    assert_eq!(budget.units_used, MAX_TEXT_PROJECTION_WORK);
    assert_eq!(budget.admit_copy("x"), Err(TextProjectionError));
    assert_eq!(budget.admit_copy(""), Err(TextProjectionError));
}

#[test]
fn checked_named_amplification_refuses_whole_text_and_keeps_scope() {
    let scope = scope(&"X".repeat(1024));
    let before = scope.clone();
    let raw = "{{name}}".repeat(4096);
    let mut budget = TextProjectionBudget::default();
    assert_eq!(
        substitute_text_checked(&raw, Some(&scope), &mut budget),
        Err(TextProjectionError)
    );
    assert!(budget.exhausted);
    assert_eq!(scope, before);
}

#[test]
fn checked_raw_allowance_is_not_only_an_output_limit() {
    let mut budget = remaining(3);
    assert_eq!(
        substitute_text_checked("ab", None, &mut budget),
        Err(TextProjectionError)
    );
    assert_eq!(budget.units_used, MAX_TEXT_PROJECTION_WORK - 1);
}

#[test]
fn checked_local_lookup_work_is_admitted_even_for_empty_replacements() {
    let scope = scope("");
    let raw = "{{name}}";
    let mut budget = remaining(raw.len());
    assert_eq!(
        substitute_text_checked(raw, Some(&scope), &mut budget),
        Err(TextProjectionError)
    );
    assert!(budget.exhausted);
}
