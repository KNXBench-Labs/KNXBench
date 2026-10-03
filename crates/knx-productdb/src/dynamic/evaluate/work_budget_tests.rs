//! Small deterministic boundaries for the actual evaluator, not a second engine.
use super::*;

fn node(id: i64, parent: Option<i64>, kind: &str) -> DynamicNode {
    DynamicNode {
        node_id: id,
        parent_id: parent,
        kind: kind.into(),
        element_id: None,
        ref_id: None,
        test: None,
        is_default: false,
        text: None,
        value: None,
        name: None,
        number: None,
        control_kind: None,
    }
}

fn with_remaining(trees: &ProgramTrees, values: &ValueMap, remaining: usize) -> Activation {
    assert!(remaining <= MAX_EVALUATION_WORK);
    let mut activation = Activation {
        work_units_used: MAX_EVALUATION_WORK - remaining,
        ..Activation::default()
    };
    let mut seen_params = HashSet::new();
    let mut seen_coms = HashSet::new();
    let mut expansions = 0;
    for &root in trees.program.roots() {
        if activation.work_budget_exhausted {
            break;
        }
        walk(
            trees,
            &trees.program,
            root,
            values,
            &mut activation,
            &mut seen_params,
            &mut seen_coms,
            &mut expansions,
            None,
            None,
        );
    }
    activation
}

fn refusal(activation: &Activation) -> &ScopedDiagnostic {
    let markers: Vec<_> = activation
        .diagnostics
        .iter()
        .filter(|d| {
            matches!(
                d.diagnostic,
                Diagnostic::EvaluationWorkBudgetExhausted { .. }
            )
        })
        .collect();
    assert_eq!(markers.len(), 1);
    assert!(markers[0].diagnostic.may_hide_refs());
    markers[0]
}

#[test]
fn exact_admission_and_overflow_refusal_keep_one_scoped_marker() {
    let scope = Rc::new(ModuleScope {
        module_node: 12,
        module_id: Some("PUBLIC-MODULE".into()),
        module_def_id: "PUBLIC-DEF".into(),
        arguments: vec![],
        parent: None,
    });
    let mut activation = Activation {
        work_units_used: MAX_EVALUATION_WORK - 1,
        ..Activation::default()
    };
    assert!(activation.admit_work(Some(&scope), 12, 1));
    assert!(activation.diagnostics.is_empty());
    assert!(!activation.admit_work(Some(&scope), 13, usize::MAX));
    assert!(!activation.admit_work(None, 14, 1));
    assert_eq!(activation.work_units_used, MAX_EVALUATION_WORK);
    assert_eq!(refusal(&activation).scope.as_ref(), Some(&scope));
    assert!(matches!(
        refusal(&activation).diagnostic,
        Diagnostic::EvaluationWorkBudgetExhausted {
            node_id: 13,
            budget: MAX_EVALUATION_WORK
        }
    ));
}

#[test]
fn duplicate_reference_visits_are_not_free() {
    let trees = ProgramTrees::single(DynamicTree::from_nodes(
        (0..3)
            .map(|id| DynamicNode {
                ref_id: Some("PUBLIC-REF".into()),
                ..node(id, None, "ParameterRefRef")
            })
            .collect(),
    ));
    let activation = with_remaining(&trees, &ValueMap::default(), 2);
    assert_eq!(activation.parameter_refs.len(), 1);
    assert_eq!(activation.parameter_refs[0].ref_id, "PUBLIC-REF");
    assert!(matches!(
        refusal(&activation).diagnostic,
        Diagnostic::EvaluationWorkBudgetExhausted { node_id: 2, .. }
    ));
}

#[test]
fn skipped_non_reference_visits_use_the_same_quota() {
    let trees = ProgramTrees::single(DynamicTree::from_nodes(vec![
        node(0, None, "Rows"),
        node(1, Some(0), "Assign"),
        node(2, Some(0), "Assign"),
    ]));
    let activation = with_remaining(&trees, &ValueMap::default(), 2);
    assert!(activation.parameter_refs.is_empty());
    assert!(matches!(
        refusal(&activation).diagnostic,
        Diagnostic::EvaluationWorkBudgetExhausted { node_id: 2, .. }
    ));
}

#[test]
fn incomplete_choice_scan_does_not_activate_an_earlier_default() {
    let trees = ProgramTrees::single(DynamicTree::from_nodes(vec![
        DynamicNode {
            ref_id: Some("CONTROL".into()),
            control_kind: Some(ControlKind::Comparable),
            ..node(0, None, "choose")
        },
        DynamicNode {
            is_default: true,
            ..node(1, Some(0), "when")
        },
        DynamicNode {
            ref_id: Some("DEFAULT".into()),
            ..node(2, Some(1), "ParameterRefRef")
        },
        DynamicNode {
            test: Some("9".into()),
            ..node(3, Some(0), "when")
        },
    ]));
    let values: ValueMap = HashMap::from([("CONTROL".to_string(), "0".to_string())]).into();
    let activation = with_remaining(&trees, &values, 2);
    assert!(activation.parameter_refs.is_empty());
    assert!(matches!(
        refusal(&activation).diagnostic,
        Diagnostic::EvaluationWorkBudgetExhausted { node_id: 3, .. }
    ));
    let complete = evaluate(&trees, &values);
    assert_eq!(complete.parameter_refs[0].ref_id, "DEFAULT");
    assert!(complete.diagnostics.is_empty());
}

#[test]
fn partial_argument_binding_does_not_create_an_activated_scope() {
    let program = DynamicTree::from_nodes(vec![
        DynamicNode {
            element_id: Some("PUBLIC-MODULE".into()),
            ref_id: Some("PUBLIC-DEF".into()),
            ..node(0, None, "Module")
        },
        DynamicNode {
            ref_id: Some("ARG-A".into()),
            value: Some(" a {{raw}} ".into()),
            ..node(1, Some(0), "TextArg")
        },
        DynamicNode {
            ref_id: Some("ARG-B".into()),
            value: Some("b".into()),
            ..node(2, Some(0), "TextArg")
        },
    ]);
    let module = DynamicTree::from_nodes(vec![DynamicNode {
        text: Some("{{A}}".into()),
        ..node(0, None, "ParameterBlock")
    }]);
    let trees = ProgramTrees::from_parts(program, HashMap::from([("PUBLIC-DEF".into(), module)]))
        .with_arguments(
            [("ARG-A", "A"), ("ARG-B", "B")].map(|(id, name)| ModuleDefArgument {
                id: id.into(),
                module_def_id: "PUBLIC-DEF".into(),
                name: Some(name.into()),
                arg_type: Some("Text".into()),
                allocates: None,
            }),
        );
    let activation = with_remaining(&trees, &ValueMap::default(), 2);
    assert!(activation.labels.is_empty());
    assert!(refusal(&activation).scope.is_none());
    let complete = evaluate(&trees, &ValueMap::default());
    assert_eq!(complete.labels[0].text, " a {{raw}} ");
    assert!(complete.diagnostics.is_empty());
}

#[test]
fn incomplete_placeholder_substitution_does_not_publish_a_partial_label() {
    let raw = "{{Missing}}{{MissingAgain}}";
    let trees = ProgramTrees::single(DynamicTree::from_nodes(vec![DynamicNode {
        text: Some(raw.into()),
        ..node(0, None, "ParameterBlock")
    }]));
    // Walk, raw allowance, first lookup/copy/diagnostic and next lookup.
    // Refuse the second output before publishing the first substituted prefix.
    let through_first = 1 + raw.len() + 1 + "{{Missing}}".len() + 1 + 1;
    let activation = with_remaining(&trees, &ValueMap::default(), through_first);
    assert!(activation.labels.is_empty());
    assert_eq!(activation.diagnostics.len(), 2);
    assert!(!activation.diagnostics[0].diagnostic.may_hide_refs());
    assert!(matches!(
        refusal(&activation).diagnostic,
        Diagnostic::EvaluationWorkBudgetExhausted { node_id: 0, .. }
    ));
    assert_eq!(trees.program.node(0).unwrap().text.as_deref(), Some(raw));
    assert_eq!(substitute_text(raw, None), raw);
}

#[test]
fn activation_budget_marker_is_cached_and_not_duplicated() {
    let mut activation = Activation {
        skipped_refs_reported: MAX_MODULE_ACTIVATIONS,
        ..Activation::default()
    };
    assert!(activation.activation_budget_spent(None, 0, None));
    assert!(activation.activation_budget_already_diagnosed());
    assert!(activation.activation_budget_spent(None, 1, None));
    assert_eq!(activation.diagnostics.len(), 1);
    assert!(matches!(
        activation.diagnostics[0].diagnostic,
        Diagnostic::ModuleExpansionBudgetExhausted {
            budget: MAX_MODULE_ACTIVATIONS,
            ..
        }
    ));
}

#[test]
fn internal_admission_state_does_not_change_public_result_equality() {
    let trees = ProgramTrees::single(DynamicTree::from_nodes(vec![node(0, None, "Assign")]));
    assert_eq!(
        evaluate(&trees, &ValueMap::default()),
        Activation::default()
    );
}

#[test]
fn literal_label_utf8_copies_use_exact_byte_cost_without_clipping() {
    let raw = "🧰é";
    let trees = ProgramTrees::single(DynamicTree::from_nodes(vec![DynamicNode {
        text: Some(raw.into()),
        ..node(7, None, "ParameterBlock")
    }]));
    let exact_cost = 1 + raw.len() * 2;
    let exact = with_remaining(&trees, &ValueMap::default(), exact_cost);
    assert_eq!(exact.labels.len(), 1);
    assert_eq!(exact.labels[0].raw_text, raw);
    assert_eq!(exact.labels[0].text, raw);
    assert!(exact.diagnostics.is_empty());
    assert_eq!(exact.work_units_used, MAX_EVALUATION_WORK);

    let short = with_remaining(&trees, &ValueMap::default(), exact_cost - 1);
    assert!(short.labels.is_empty());
    assert_eq!(refusal(&short).scope, None);
    assert_eq!(trees.program.node(7).unwrap().text.as_deref(), Some(raw));
}
