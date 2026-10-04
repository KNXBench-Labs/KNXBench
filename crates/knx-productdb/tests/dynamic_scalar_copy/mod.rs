//! ADR-0065: public synthetic content-copy admission and scope-local names.
//!
//! The old work counter did not promise this stronger byte-cost policy.
use super::{
    evaluate, nd, Diagnostic, DynamicNode, DynamicTree, ProgramTrees, ValueMap, MAX_EVALUATION_WORK,
};
use knx_productdb::dynamic::{Activation, ModuleDefArgument};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

fn prefix() -> DynamicNode {
    DynamicNode {
        ref_id: Some("PUBLIC-PREFIX".into()),
        ..nd(0, None, "ParameterRefRef")
    }
}

fn suffix() -> DynamicNode {
    DynamicNode {
        ref_id: Some("PUBLIC-SUFFIX".into()),
        ..nd(1, None, "ParameterRefRef")
    }
}

fn declaration(id: String, definition: &str, name: String) -> ModuleDefArgument {
    ModuleDefArgument {
        id,
        module_def_id: definition.into(),
        name: Some(name),
        arg_type: Some("Text".into()),
        allocates: None,
    }
}

/// Count only these owned scalar contents, not capacity/RSS, source buffers,
/// ref IDs or diagnostic strings. Rc-shared ancestor arguments count once.
fn copied_content_bytes(activation: &Activation) -> usize {
    let mut bytes: usize = activation
        .labels
        .iter()
        .map(|label| label.raw_text.len() + label.text.len())
        .sum();
    let scopes = activation
        .parameter_refs
        .iter()
        .chain(&activation.com_object_refs)
        .filter_map(|reference| reference.scope.as_deref())
        .chain(
            activation
                .labels
                .iter()
                .filter_map(|label| label.scope.as_deref()),
        )
        .chain(
            activation
                .diagnostics
                .iter()
                .filter_map(|diagnostic| diagnostic.scope.as_deref()),
        );
    let mut seen = HashSet::new();
    for scope in scopes {
        let mut current = Some(scope);
        while let Some(scope) = current {
            if seen.insert(scope.node_chain()) {
                bytes += scope
                    .arguments
                    .iter()
                    .map(|argument| argument.name.len() + argument.value.len())
                    .sum::<usize>();
            }
            current = scope.parent.as_deref();
        }
    }
    bytes
}

fn assert_refusal(trees: &ProgramTrees, family: &str, chain: Option<&[i64]>) {
    // The fingerprint is over public synthetic trees, never private inputs.
    let before = Sha256::digest(format!("{trees:?}").as_bytes());
    let start = Instant::now();
    let activation = evaluate(trees, &ValueMap::default());
    let elapsed = start.elapsed();
    let copied = copied_content_bytes(&activation);
    eprintln!(
        "public scalar admission: family={family} bytes={copied} ceiling={MAX_EVALUATION_WORK} labels={} refs={} diagnostics={} elapsed_ms={}",
        activation.labels.len(),
        activation.parameter_refs.len(),
        activation.diagnostics.len(),
        elapsed.as_millis()
    );
    assert!(
        elapsed < Duration::from_secs(10),
        "finite synthetic evaluation"
    );
    assert_eq!(Sha256::digest(format!("{trees:?}").as_bytes()), before);
    assert!(
        copied <= MAX_EVALUATION_WORK,
        "admitted scalar contents exceed the proposed shared byte-cost policy"
    );
    let markers: Vec<_> = activation
        .diagnostics
        .iter()
        .filter(|diagnostic| {
            matches!(
                diagnostic.diagnostic,
                Diagnostic::EvaluationWorkBudgetExhausted { .. }
            )
        })
        .collect();
    assert_eq!(markers.len(), 1, "one explicit work-refusal marker");
    let marker = markers[0];
    assert!(marker.diagnostic.may_hide_refs());
    assert!(matches!(
        marker.diagnostic,
        Diagnostic::EvaluationWorkBudgetExhausted {
            budget: MAX_EVALUATION_WORK,
            ..
        }
    ));
    assert_eq!(
        marker.scope.as_ref().map(|scope| scope.node_chain()),
        chain.map(<[i64]>::to_vec)
    );
    assert!(activation.labels.is_empty(), "no partial substituted label");
    assert_eq!(activation.parameter_refs.len(), 1, "admitted prefix only");
    assert_eq!(activation.parameter_refs[0].ref_id, "PUBLIC-PREFIX");
    assert!(activation.parameter_refs[0].scope.is_none());
    assert!(activation.com_object_refs.is_empty());
}

#[test]
fn literal_labels_pay_for_input_and_copied_content() {
    let tree = DynamicTree::from_nodes(vec![
        prefix(),
        DynamicNode {
            text: Some("L".repeat(MAX_EVALUATION_WORK + 1)),
            ..nd(2, None, "Channel")
        },
        DynamicNode {
            node_id: 3,
            ..suffix()
        },
    ]);
    assert_refusal(&ProgramTrees::single(tree), "literal", None);
}

#[test]
fn repeated_named_values_cannot_amplify_one_admitted_label() {
    let program = DynamicTree::from_nodes(vec![
        prefix(),
        DynamicNode {
            element_id: Some("PUBLIC-MODULE".into()),
            ref_id: Some("PUBLIC-DEF".into()),
            ..nd(1, None, "Module")
        },
        DynamicNode {
            ref_id: Some("PUBLIC-ARG".into()),
            value: Some("V".repeat(8192)),
            ..nd(2, Some(1), "TextArg")
        },
    ]);
    let modules = HashMap::from([(
        "PUBLIC-DEF".into(),
        DynamicTree::from_nodes(vec![
            DynamicNode {
                text: Some("{{Payload}}".repeat(1024)),
                ..nd(0, None, "ParameterBlock")
            },
            suffix(),
        ]),
    )]);
    let trees = ProgramTrees::from_parts(program, modules).with_arguments([declaration(
        "PUBLIC-ARG".into(),
        "PUBLIC-DEF",
        "Payload".into(),
    )]);
    assert_refusal(&trees, "replacement", Some(&[1]));
}

#[test]
fn successful_bindings_pay_before_cloning_names_and_values() {
    let binding_count = 512;
    let mut nodes = vec![
        prefix(),
        DynamicNode {
            element_id: Some("PUBLIC-MODULE".into()),
            ref_id: Some("PUBLIC-DEF".into()),
            ..nd(1, None, "Module")
        },
    ];
    nodes.extend((0..binding_count).map(|i| DynamicNode {
        ref_id: Some(format!("PUBLIC-ARG-{i}")),
        value: Some("V".repeat(16384)),
        ..nd(i + 2, Some(1), "TextArg")
    }));
    let modules = HashMap::from([("PUBLIC-DEF".into(), DynamicTree::from_nodes(vec![suffix()]))]);
    let trees = ProgramTrees::from_parts(DynamicTree::from_nodes(nodes), modules).with_arguments(
        (0..binding_count).map(|i| {
            declaration(
                format!("PUBLIC-ARG-{i}"),
                "PUBLIC-DEF",
                format!("Public_{i}"),
            )
        }),
    );
    assert_refusal(&trees, "bindings", None);
}

#[test]
fn parent_argument_names_do_not_become_inherited_bindings() {
    let binding_count = 1024;
    let mut nodes = vec![
        prefix(),
        DynamicNode {
            element_id: Some("PUBLIC-OUTER".into()),
            ref_id: Some("PUBLIC-OUTER-DEF".into()),
            ..nd(1, None, "Module")
        },
    ];
    nodes.extend((0..binding_count).map(|i| DynamicNode {
        ref_id: Some(format!("PUBLIC-ARG-{i}")),
        value: Some("END".into()),
        ..nd(i + 2, Some(1), "TextArg")
    }));
    let modules = HashMap::from([
        (
            "PUBLIC-OUTER-DEF".into(),
            DynamicTree::from_nodes(vec![DynamicNode {
                element_id: Some("PUBLIC-INNER".into()),
                ref_id: Some("PUBLIC-INNER-DEF".into()),
                ..nd(0, None, "Module")
            }]),
        ),
        (
            "PUBLIC-INNER-DEF".into(),
            DynamicTree::from_nodes(vec![
                DynamicNode {
                    text: Some("{{Target}}".repeat(4096)),
                    ..nd(0, None, "ParameterBlock")
                },
                suffix(),
            ]),
        ),
    ]);
    let trees = ProgramTrees::from_parts(DynamicTree::from_nodes(nodes), modules).with_arguments(
        (0..binding_count).map(|i| {
            declaration(
                format!("PUBLIC-ARG-{i}"),
                "PUBLIC-OUTER-DEF",
                if i == binding_count - 1 {
                    "Target".into()
                } else {
                    format!("Public_{i}")
                },
            )
        }),
    );
    let before = Sha256::digest(format!("{trees:?}").as_bytes());
    let activation = evaluate(&trees, &ValueMap::default());
    assert_eq!(Sha256::digest(format!("{trees:?}").as_bytes()), before);
    assert!(copied_content_bytes(&activation) <= MAX_EVALUATION_WORK);
    assert_eq!(activation.labels.len(), 1);
    let label = &activation.labels[0];
    assert_eq!(label.text, label.raw_text);
    assert_eq!(label.text, "{{Target}}".repeat(4096));
    let scope = label.scope.as_ref().unwrap();
    assert_eq!(scope.node_chain(), vec![1, 0]);
    assert!(scope.arguments.is_empty());
    assert!(scope
        .parent
        .as_ref()
        .unwrap()
        .arguments
        .iter()
        .any(|a| a.name == "Target"));
    assert_eq!(activation.diagnostics.len(), 4096);
    assert!(activation.diagnostics.iter().all(|diagnostic| {
        matches!(&diagnostic.diagnostic, Diagnostic::UnresolvedTextPlaceholder { name, .. } if name == "Target")
            && !diagnostic.diagnostic.may_hide_refs()
            && diagnostic.scope.as_ref().unwrap().node_chain() == vec![1, 0]
    }));
    assert_eq!(activation.parameter_refs.len(), 2);
    assert!(activation
        .parameter_refs
        .iter()
        .any(|r| r.ref_id == "PUBLIC-SUFFIX"));
}
