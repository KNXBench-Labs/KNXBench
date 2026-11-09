//! Conditional block rename: the evaluator reports at most one replacement per block.
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

fn block(id: i64, parent: Option<i64>, element: &str, text: &str) -> DynamicNode {
    DynamicNode {
        element_id: Some(element.into()),
        text: Some(text.into()),
        ..node(id, parent, "ParameterBlock")
    }
}

fn rename(
    id: i64,
    parent: i64,
    kind: &str,
    element: &str,
    target: &str,
    text: &str,
) -> DynamicNode {
    DynamicNode {
        element_id: Some(element.into()),
        ref_id: Some(target.into()),
        text: Some(text.into()),
        ..node(id, Some(parent), kind)
    }
}

fn run(nodes: Vec<DynamicNode>) -> Activation {
    evaluate(
        &ProgramTrees::single(DynamicTree::from_nodes(nodes)),
        &ValueMap::default(),
    )
}

fn refused(activation: &Activation) -> Vec<i64> {
    activation
        .diagnostics
        .iter()
        .filter_map(|d| match d.diagnostic {
            Diagnostic::UnrecognizedNode { node_id, .. } => Some(node_id),
            _ => None,
        })
        .collect()
}

fn base_labels(activation: &Activation) -> Vec<&str> {
    activation.labels.iter().map(|l| l.text.as_str()).collect()
}

#[test]
fn a_leaf_before_or_after_the_blocks_references_replaces_that_block_only() {
    for leaf_first in [true, false] {
        let leaf = rename(1, 0, "ParameterBlockRename", "PR-1", "PB-1", "New");
        let reference = DynamicNode {
            ref_id: Some("P-1_R-1".into()),
            ..node(2, Some(0), "ParameterRefRef")
        };
        let children = if leaf_first {
            vec![leaf, reference]
        } else {
            vec![reference, leaf]
        };
        let mut nodes = vec![block(0, None, "PB-1", "Base")];
        nodes.extend(children);
        let activation = run(nodes);
        assert!(refused(&activation).is_empty(), "leaf_first={leaf_first}");
        assert_eq!(activation.renames.len(), 1);
        assert_eq!(activation.renames[0].target_node_id, 0);
        assert_eq!(activation.renames[0].rename_id, "PR-1");
        // The base label is never rewritten; the consumer picks.
        assert_eq!(base_labels(&activation), ["Base"]);
        assert_eq!(activation.labels[0].raw_text, "Base");
    }
}

#[test]
fn the_converted_rename_kind_is_the_same_contract() {
    let activation = run(vec![
        block(0, None, "PB-1", "Base"),
        rename(1, 0, "Rename", "PR-1", "PB-1", "New"),
    ]);
    assert_eq!(activation.renames[0].kind, "Rename");
    assert!(refused(&activation).is_empty());
}

#[test]
fn only_the_nearest_enclosing_block_can_be_renamed() {
    // Naming an outer block from inside an inner one is a cross-target rename.
    let activation = run(vec![
        block(0, None, "PB-OUT", "Outer"),
        block(1, Some(0), "PB-IN", "Inner"),
        rename(2, 1, "ParameterBlockRename", "PR-1", "PB-OUT", "Cross"),
        rename(3, 1, "ParameterBlockRename", "PR-2", "PB-IN", "Own"),
    ]);
    assert_eq!(refused(&activation), [2]);
    assert_eq!(activation.renames.len(), 1);
    assert_eq!(activation.renames[0].target_node_id, 1);
}

#[test]
fn refused_shapes_report_unrecognized_and_replace_nothing() {
    let wrong = |mutate: fn(&mut DynamicNode)| {
        let mut leaf = rename(1, 0, "ParameterBlockRename", "PR-1", "PB-1", "New");
        mutate(&mut leaf);
        run(vec![block(0, None, "PB-1", "Base"), leaf])
    };
    let cases: Vec<(&str, Activation)> = vec![
        ("no identity", wrong(|l| l.element_id = None)),
        ("no target", wrong(|l| l.ref_id = None)),
        ("other target", wrong(|l| l.ref_id = Some("PB-9".into()))),
        ("no text", wrong(|l| l.text = None)),
        ("empty text", wrong(|l| l.text = Some(String::new()))),
        ("placeholder", wrong(|l| l.text = Some("New {{0}}".into()))),
    ];
    for (name, activation) in cases {
        assert!(activation.renames.is_empty(), "{name}");
        assert_eq!(refused(&activation), [1], "{name}");
        assert_eq!(base_labels(&activation), ["Base"], "{name}");
    }
}

#[test]
fn a_leaf_with_children_or_unmodelled_attributes_is_refused() {
    let nodes = || {
        vec![
            block(0, None, "PB-1", "Base"),
            rename(1, 0, "ParameterBlockRename", "PR-1", "PB-1", "New"),
        ]
    };
    let mut with_child = nodes();
    with_child.push(node(2, Some(1), "ParameterRefRef"));
    let activation = run(with_child);
    assert!(activation.renames.is_empty());
    assert_eq!(refused(&activation), [1]);

    let opaque = DynamicTree::from_nodes(nodes()).with_opaque([1]);
    let activation = evaluate(&ProgramTrees::single(opaque), &ValueMap::default());
    assert!(activation.renames.is_empty());
    assert_eq!(refused(&activation), [1]);
}

#[test]
fn duplicate_identities_and_competing_leaves_have_no_winner() {
    // Two different leaves for one block, both active.
    let competing = run(vec![
        block(0, None, "PB-1", "Base"),
        rename(1, 0, "ParameterBlockRename", "PR-1", "PB-1", "First"),
        rename(2, 0, "ParameterBlockRename", "PR-2", "PB-1", "Second"),
    ]);
    assert!(competing.renames.is_empty());
    assert_eq!(refused(&competing), [1, 2]);

    // One identity declared twice, anywhere in the tree.
    let repeated_leaf = run(vec![
        block(0, None, "PB-1", "Base"),
        rename(1, 0, "ParameterBlockRename", "PR-1", "PB-1", "First"),
        block(2, None, "PB-2", "Other"),
        rename(3, 2, "ParameterBlockRename", "PR-1", "PB-2", "Second"),
    ]);
    assert!(repeated_leaf.renames.is_empty());
    assert_eq!(refused(&repeated_leaf), [1, 3]);

    // One block identity declared twice: the target does not resolve to one block.
    let repeated_block = run(vec![
        block(0, None, "PB-1", "Base"),
        rename(1, 0, "ParameterBlockRename", "PR-1", "PB-1", "New"),
        block(2, None, "PB-1", "Twin"),
    ]);
    assert!(repeated_block.renames.is_empty());
    assert_eq!(refused(&repeated_block), [1]);
}

#[test]
fn a_block_without_a_base_caption_has_nothing_to_replace() {
    let mut captionless = block(0, None, "PB-1", "");
    captionless.text = None;
    let activation = run(vec![
        captionless,
        rename(1, 0, "ParameterBlockRename", "PR-1", "PB-1", "New"),
    ]);
    assert!(activation.renames.is_empty());
    assert_eq!(refused(&activation), [1]);
}

#[test]
fn a_leaf_outside_every_block_is_refused() {
    let activation = run(vec![
        node(0, None, "Channel"),
        rename(1, 0, "ParameterBlockRename", "PR-1", "PB-1", "New"),
    ]);
    assert_eq!(refused(&activation), [1]);
}

#[test]
fn a_spent_activation_budget_refuses_the_leaf_instead_of_growing_the_pass() {
    let trees = ProgramTrees::single(DynamicTree::from_nodes(vec![
        block(0, None, "PB-1", "Base"),
        rename(1, 0, "ParameterBlockRename", "PR-1", "PB-1", "New"),
    ]));
    let mut activation = Activation {
        skipped_refs_reported: MAX_MODULE_ACTIVATIONS,
        ..Activation::default()
    };
    let (mut params, mut coms, mut expansions) = (Seen::new(), Seen::new(), 0);
    walk(
        &trees,
        &trees.program,
        0,
        &ValueMap::default(),
        &mut activation,
        &mut params,
        &mut coms,
        &mut expansions,
        None,
        PathContext::default(),
    );
    assert!(activation.rename_candidates.is_empty());
    assert!(activation.diagnostics.iter().any(|d| matches!(
        d.diagnostic,
        Diagnostic::ModuleExpansionBudgetExhausted { .. }
    )));
    assert!(activation.diagnostics.iter().any(|d| matches!(
        d.diagnostic,
        Diagnostic::UnrecognizedNode { node_id: 1, .. }
    )));
}

fn choose(id: i64, parent: i64, controller: &str, kind: Option<ControlKind>) -> DynamicNode {
    DynamicNode {
        ref_id: Some(controller.into()),
        control_kind: kind,
        ..node(id, Some(parent), "choose")
    }
}

fn when(id: i64, parent: i64, test: &str) -> DynamicNode {
    DynamicNode {
        test: Some(test.into()),
        ..node(id, Some(parent), "when")
    }
}

fn run_with(nodes: Vec<DynamicNode>, values: &[(&str, &str)]) -> Activation {
    let values: HashMap<String, String> = values
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    evaluate(
        &ProgramTrees::single(DynamicTree::from_nodes(nodes)),
        &ValueMap::from(values),
    )
}

#[test]
fn an_uninterpretable_leaf_on_the_same_target_leaves_no_winner_in_either_order() {
    for valid_first in [true, false] {
        let (valid, other) = if valid_first { (1, 2) } else { (2, 1) };
        let mut nodes = vec![
            block(0, None, "PB-1", "Base"),
            rename(valid, 0, "ParameterBlockRename", "PR-A", "PB-1", "Valid"),
            rename(other, 0, "ParameterBlockRename", "PR-B", "PB-1", "Opaque"),
        ];
        nodes.sort_by_key(|n| n.node_id);
        let tree = DynamicTree::from_nodes(nodes).with_opaque([other]);
        let activation = evaluate(&ProgramTrees::single(tree), &ValueMap::default());
        assert!(activation.renames.is_empty(), "valid_first={valid_first}");
        let mut refused = refused(&activation);
        refused.sort_unstable();
        assert_eq!(refused, [1, 2], "valid_first={valid_first}");
        assert_eq!(base_labels(&activation), ["Base"]);
        assert_eq!(activation.labels[0].raw_text, "Base");
    }
}

#[test]
fn a_competing_leaf_in_a_branch_known_to_be_inactive_does_not_block() {
    let activation = run_with(
        vec![
            block(0, None, "PB-1", "Base"),
            rename(1, 0, "ParameterBlockRename", "PR-A", "PB-1", "Valid"),
            choose(2, 0, "P-1", Some(ControlKind::Comparable)),
            when(3, 2, "1"),
            when(4, 2, "2"),
            rename(5, 4, "ParameterBlockRename", "PR-B", "PB-1", "Other {{0}}"),
        ],
        &[("P-1", "1")],
    );
    assert!(refused(&activation).is_empty());
    assert_eq!(activation.renames.len(), 1);
    assert_eq!(activation.renames[0].rename_id, "PR-A");
}

#[test]
fn a_leaf_on_an_undecided_path_to_the_same_target_leaves_no_winner() {
    let shaped = |kind: Option<ControlKind>, values: &[(&str, &str)]| {
        run_with(
            vec![
                block(0, None, "PB-1", "Base"),
                rename(1, 0, "ParameterBlockRename", "PR-A", "PB-1", "Valid"),
                choose(2, 0, "P-1", kind),
                when(3, 2, "1"),
                rename(4, 3, "ParameterBlockRename", "PR-B", "PB-1", "Hidden"),
            ],
            values,
        )
    };
    let cases = [
        // No stored or default value: the branch is undecided, no walk descends it.
        ("missing value", shaped(Some(ControlKind::Comparable), &[])),
        (
            "non-numeric value",
            shaped(Some(ControlKind::Comparable), &[("P-1", "x")]),
        ),
        // The controller itself is not understood: a structural refusal.
        ("unresolved controller", shaped(None, &[("P-1", "1")])),
    ];
    for (name, activation) in cases {
        assert!(activation.renames.is_empty(), "{name}");
        assert!(refused(&activation).contains(&1), "{name}");
        assert_eq!(base_labels(&activation), ["Base"], "{name}");
    }
    // The decided twin of the same tree: the hidden leaf is active and competes openly.
    let decided = shaped(Some(ControlKind::Comparable), &[("P-1", "1")]);
    assert!(decided.renames.is_empty());
    assert_eq!(refused(&decided), [1, 4]);
}

#[test]
fn an_undecided_path_naming_another_block_does_not_block_this_one() {
    let activation = run_with(
        vec![
            block(0, None, "PB-1", "Base"),
            rename(1, 0, "ParameterBlockRename", "PR-A", "PB-1", "Valid"),
            block(2, None, "PB-2", "Second"),
            choose(3, 2, "P-1", Some(ControlKind::Comparable)),
            when(4, 3, "1"),
            rename(5, 4, "ParameterBlockRename", "PR-B", "PB-2", "Hidden"),
        ],
        &[],
    );
    assert_eq!(activation.renames.len(), 1);
    assert_eq!(activation.renames[0].target_id, "PB-1");
}

#[test]
fn a_cross_target_leaf_keeps_the_named_block_from_a_valid_own_leaf() {
    // The inner leaf names the outer block; the outer block's own leaf must not win it.
    let activation = run(vec![
        block(0, None, "PB-OUT", "Outer"),
        rename(1, 0, "ParameterBlockRename", "PR-A", "PB-OUT", "Valid"),
        block(2, Some(0), "PB-IN", "Inner"),
        rename(3, 2, "ParameterBlockRename", "PR-B", "PB-OUT", "Cross"),
    ]);
    assert!(activation.renames.is_empty());
    let mut refused = refused(&activation);
    refused.sort_unstable();
    assert_eq!(refused, [1, 3]);
    assert_eq!(base_labels(&activation), ["Outer", "Inner"]);
}

#[test]
fn a_leaf_skipped_below_a_recognized_leaf_still_names_its_target() {
    // A separator with a child is walked as a leaf; nothing below it is evaluated.
    let activation = run(vec![
        block(0, None, "PB-1", "Base"),
        rename(1, 0, "ParameterBlockRename", "PR-A", "PB-1", "Valid"),
        node(2, Some(0), "ParameterSeparator"),
        rename(3, 2, "ParameterBlockRename", "PR-B", "PB-1", "Below"),
    ]);
    assert!(activation.renames.is_empty());
    assert_eq!(refused(&activation), [1]);
}

#[test]
fn a_leaf_without_a_target_could_name_any_block_and_blocks_them_all() {
    let mut targetless = rename(3, 2, "ParameterBlockRename", "PR-B", "PB-2", "Lost");
    targetless.ref_id = None;
    let activation = run(vec![
        block(0, None, "PB-1", "Base"),
        rename(1, 0, "ParameterBlockRename", "PR-A", "PB-1", "Valid"),
        block(2, None, "PB-2", "Second"),
        targetless,
    ]);
    assert!(activation.renames.is_empty());
    let mut refused = refused(&activation);
    refused.sort_unstable();
    assert_eq!(refused, [1, 3]);
}

#[test]
fn an_oversized_replacement_is_refused_by_the_evaluation_budget_before_any_copy() {
    let long = "x".repeat(MAX_EVALUATION_WORK + 1);
    let activation = run(vec![
        block(0, None, "PB-1", "Base"),
        rename(1, 0, "ParameterBlockRename", "PR-A", "PB-1", &long),
    ]);
    assert!(activation.renames.is_empty());
    assert!(activation.diagnostics.iter().any(|d| matches!(
        d.diagnostic,
        Diagnostic::EvaluationWorkBudgetExhausted { node_id: 1, .. }
    )));
    assert_eq!(base_labels(&activation), ["Base"]);
    // A refused shape is scanned too: the scan alone is charged before it happens.
    let refused_shape = run(vec![
        block(0, None, "PB-1", "Base"),
        rename(1, 0, "ParameterBlockRename", "PR-A", "PB-OTHER", &long),
    ]);
    assert!(refused_shape.diagnostics.iter().any(|d| matches!(
        d.diagnostic,
        Diagnostic::EvaluationWorkBudgetExhausted { node_id: 1, .. }
    )));
    // The scan fits, the retained copy does not: refused before it is made.
    let copy_too_large = "x".repeat(MAX_EVALUATION_WORK * 3 / 5);
    let copied = run(vec![
        block(0, None, "PB-1", "Base"),
        rename(
            1,
            0,
            "ParameterBlockRename",
            "PR-A",
            "PB-1",
            &copy_too_large,
        ),
    ]);
    assert!(copied.renames.is_empty());
    assert!(copied.diagnostics.iter().any(|d| matches!(
        d.diagnostic,
        Diagnostic::EvaluationWorkBudgetExhausted { node_id: 1, .. }
    )));
    // Control: the same tree with a short replacement is accepted.
    let short = run(vec![
        block(0, None, "PB-1", "Base"),
        rename(1, 0, "ParameterBlockRename", "PR-A", "PB-1", "Short"),
    ]);
    assert_eq!(short.renames.len(), 1);
}

#[test]
fn public_evaluate_settles_renames_on_the_budget_its_walk_left() {
    // Two competing leaves under a block whose caption is sized so that the walk fits the
    // budget and the two refusal diagnostics do not both fit.
    let tree = |caption: &str| {
        vec![
            block(0, None, "PB-1", caption),
            rename(1, 0, "ParameterBlockRename", "PR-A", "PB-1", "First"),
            rename(2, 0, "ParameterBlockRename", "PR-B", "PB-1", "Second"),
        ]
    };
    let walked = |caption: &str| {
        let trees = ProgramTrees::single(DynamicTree::from_nodes(tree(caption)));
        let mut activation = Activation::default();
        let (mut params, mut coms, mut expansions) = (Seen::new(), Seen::new(), 0);
        walk(
            &trees,
            &trees.program,
            0,
            &ValueMap::default(),
            &mut activation,
            &mut params,
            &mut coms,
            &mut expansions,
            None,
            PathContext::default(),
        );
        activation.work_units_used
    };
    // A caption is charged twice per byte (input, then output).
    let fixed = walked("");
    let per_byte = walked("ab") - walked("a");
    assert_eq!(per_byte, 2);
    let size = (MAX_EVALUATION_WORK - 1 - fixed) / per_byte;
    let caption = "c".repeat(size);
    let used = walked(&caption);
    assert!(
        used <= MAX_EVALUATION_WORK && used + 2 > MAX_EVALUATION_WORK,
        "{used}"
    );
    let activation = run(tree(&caption));
    assert!(activation.renames.is_empty());
    assert_eq!(
        activation.diagnostics.last().map(|d| &d.diagnostic),
        Some(&Diagnostic::EvaluationWorkBudgetExhausted {
            node_id: 2,
            budget: MAX_EVALUATION_WORK
        }),
        "settling ran out of the walk's own budget"
    );
    // Control: a shorter caption leaves room for both refusals.
    let roomy = run(tree(&caption[..size - 2]));
    assert_eq!(refused(&roomy), [1, 2]);
}

/// Walks `nodes` with `used_before` work units already spent, then settles renames.
fn settle_with_spare(nodes: Vec<DynamicNode>, used_before: usize) -> Activation {
    let trees = ProgramTrees::single(DynamicTree::from_nodes(nodes));
    let mut activation = Activation {
        work_units_used: used_before,
        ..Activation::default()
    };
    let (mut params, mut coms, mut expansions) = (Seen::new(), Seen::new(), 0);
    walk(
        &trees,
        &trees.program,
        0,
        &ValueMap::default(),
        &mut activation,
        &mut params,
        &mut coms,
        &mut expansions,
        None,
        PathContext::default(),
    );
    resolve_renames(&trees.program, &mut activation);
    activation
}

#[test]
fn settling_renames_spends_the_same_work_budget_and_adds_nothing_past_it() {
    let competing = || {
        vec![
            block(0, None, "PB-1", "Base"),
            rename(1, 0, "ParameterBlockRename", "PR-A", "PB-1", "First"),
            rename(2, 0, "ParameterBlockRename", "PR-B", "PB-1", "Second"),
        ]
    };
    // Calibrate: what the walk alone and the walk plus settling cost on a fresh budget.
    let walked = {
        let trees = ProgramTrees::single(DynamicTree::from_nodes(competing()));
        let mut activation = Activation::default();
        let (mut params, mut coms, mut expansions) = (Seen::new(), Seen::new(), 0);
        walk(
            &trees,
            &trees.program,
            0,
            &ValueMap::default(),
            &mut activation,
            &mut params,
            &mut coms,
            &mut expansions,
            None,
            PathContext::default(),
        );
        activation.work_units_used
    };
    let settled = settle_with_spare(competing(), 0);
    assert_eq!(refused(&settled), [1, 2]);
    let total = settled.work_units_used;
    assert!(
        total > walked,
        "settling must be charged: {walked} vs {total}"
    );

    // Exactly enough: the same two conflict diagnostics, nothing refused for budget.
    let fits = settle_with_spare(competing(), MAX_EVALUATION_WORK - total);
    assert_eq!(refused(&fits), [1, 2]);
    assert!(!fits.work_budget_exhausted);

    // One unit short: the walk still fits, settling hits the budget and stops there.
    let short = settle_with_spare(competing(), MAX_EVALUATION_WORK - total + 1);
    assert!(short.work_budget_exhausted);
    assert!(short.renames.is_empty());
    let budget_at = short
        .diagnostics
        .iter()
        .position(|d| {
            matches!(
                d.diagnostic,
                Diagnostic::EvaluationWorkBudgetExhausted { .. }
            )
        })
        .expect("the refusal is named");
    assert_eq!(
        budget_at,
        short.diagnostics.len() - 1,
        "nothing appended after the refusal"
    );
    assert!(refused(&short).len() < 2);
}

#[test]
fn a_lone_candidate_on_an_exhausted_settling_budget_is_not_applied() {
    let lone = || {
        vec![
            block(0, None, "PB-1", "Base"),
            rename(1, 0, "ParameterBlockRename", "PR-A", "PB-1", "Only"),
            choose(2, 0, "P-1", Some(ControlKind::Comparable)),
        ]
    };
    let settled = settle_with_spare(lone(), 0);
    assert_eq!(settled.renames.len(), 1);
    let total = settled.work_units_used;
    let short = settle_with_spare(lone(), MAX_EVALUATION_WORK - total + 1);
    assert!(short.work_budget_exhausted);
    assert!(short.renames.is_empty());
}
