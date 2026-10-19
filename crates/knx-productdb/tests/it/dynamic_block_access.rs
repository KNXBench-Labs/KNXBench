//! Block `Access` reaches every activated ref and separator below it (ADR-0139).
//!
//! Hand-built trees, no database. Corpus measurement behind the occurrence
//! rule (17 October 2026, local collection, aggregate only): 254,222
//! parameter references sit under a restricting block somewhere, 301 of them
//! also under an unrestricted one; no block re-opens a more restrictive
//! enclosing block.

use std::collections::HashMap;

use knx_productdb::dynamic::{
    evaluate, Activation, ActiveSeparator, ControlKind, DynamicNode, DynamicTree, ProgramTrees,
};

fn nd(node_id: i64, parent_id: Option<i64>, kind: &str) -> DynamicNode {
    DynamicNode {
        node_id,
        parent_id,
        kind: kind.to_string(),
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

fn with_ref(mut n: DynamicNode, id: &str) -> DynamicNode {
    n.ref_id = Some(id.to_string());
    n
}

fn with_text(mut n: DynamicNode, text: &str) -> DynamicNode {
    n.text = Some(text.to_string());
    n
}

fn run(tree: DynamicTree, values: &[(&str, &str)]) -> Activation {
    let trees = ProgramTrees::from_parts(tree, HashMap::new());
    let values: HashMap<String, String> = values
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    evaluate(&trees, &values.into())
}

fn access_of(activation: &Activation, id: &str) -> Option<String> {
    activation
        .parameter_refs
        .iter()
        .find(|r| r.ref_id == id)
        .unwrap_or_else(|| panic!("{id} was not activated"))
        .block_access
        .clone()
}

#[test]
fn a_block_binds_everything_below_it_and_the_tighter_block_wins() {
    let tree = DynamicTree::from_nodes(vec![
        nd(0, None, "Dynamic"),
        nd(1, Some(0), "ParameterBlock"),
        with_ref(nd(2, Some(1), "ParameterRefRef"), "P-OPEN"),
        nd(3, Some(0), "ParameterBlock"),
        with_ref(nd(4, Some(3), "ParameterRefRef"), "P-READ"),
        nd(5, Some(3), "ParameterBlock"),
        with_ref(nd(6, Some(5), "ParameterRefRef"), "P-HIDDEN"),
        // `ReadWrite` inside `Read` does not re-open (and never occurs).
        nd(7, Some(3), "ParameterBlock"),
        with_ref(nd(8, Some(7), "ParameterRefRef"), "P-STILL-READ"),
        nd(9, Some(0), "ParameterBlock"),
        with_ref(nd(10, Some(9), "ComObjectRefRef"), "O-1"),
        with_ref(nd(11, Some(0), "ParameterRefRef"), "P-TOP"),
    ])
    .with_access([
        (1, "ReadWrite"),
        (3, "Read"),
        (5, "None"),
        (7, "ReadWrite"),
        (9, "None"),
    ]);
    let activation = run(tree, &[]);

    assert_eq!(access_of(&activation, "P-OPEN"), None);
    assert_eq!(access_of(&activation, "P-READ").as_deref(), Some("Read"));
    assert_eq!(access_of(&activation, "P-HIDDEN").as_deref(), Some("None"));
    assert_eq!(
        access_of(&activation, "P-STILL-READ").as_deref(),
        Some("Read")
    );
    assert_eq!(access_of(&activation, "P-TOP"), None);
    // Objects record it too; nothing consumes it for them.
    assert_eq!(
        activation.com_object_refs[0].block_access.as_deref(),
        Some("None")
    );
}

#[test]
fn an_unknown_token_restricts_and_stays_verbatim() {
    let tree = DynamicTree::from_nodes(vec![
        nd(0, None, "Dynamic"),
        nd(1, Some(0), "ParameterBlock"),
        with_ref(nd(2, Some(1), "ParameterRefRef"), "P-1"),
        nd(3, Some(1), "ParameterBlock"),
        with_ref(nd(4, Some(3), "ParameterRefRef"), "P-2"),
    ])
    .with_access([(1, "Locked"), (3, "Read")]);
    let activation = run(tree, &[]);
    assert_eq!(access_of(&activation, "P-1").as_deref(), Some("Locked"));
    // Same rank as `Read`: the outer, earlier declaration stays.
    assert_eq!(access_of(&activation, "P-2").as_deref(), Some("Locked"));
}

#[test]
fn a_module_inherits_the_block_it_is_instantiated_in() {
    let program = DynamicTree::from_nodes(vec![
        nd(0, None, "Dynamic"),
        nd(1, Some(0), "ParameterBlock"),
        with_ref(nd(2, Some(1), "Module"), "MD-1"),
    ])
    .with_access([(1, "Read")]);
    let module = DynamicTree::from_nodes(vec![
        nd(10, None, "Dynamic"),
        with_ref(nd(11, Some(10), "ParameterRefRef"), "P-MOD"),
        nd(12, Some(10), "ParameterBlock"),
        with_ref(nd(13, Some(12), "ParameterRefRef"), "P-MOD-HIDDEN"),
    ])
    .with_access([(12, "None")]);
    let trees = ProgramTrees::from_parts(program, HashMap::from([("MD-1".to_string(), module)]));
    let activation = evaluate(&trees, &HashMap::<String, String>::new().into());
    assert_eq!(access_of(&activation, "P-MOD").as_deref(), Some("Read"));
    assert_eq!(
        access_of(&activation, "P-MOD-HIDDEN").as_deref(),
        Some("None")
    );
}

#[test]
fn a_ref_activated_twice_keeps_the_least_restrictive_path() {
    let tree = DynamicTree::from_nodes(vec![
        nd(0, None, "Dynamic"),
        nd(1, Some(0), "ParameterBlock"),
        with_ref(nd(2, Some(1), "ParameterRefRef"), "P-BOTH"),
        with_ref(nd(3, Some(1), "ParameterRefRef"), "P-NONE-THEN-READ"),
        nd(4, Some(0), "ParameterBlock"),
        with_ref(nd(5, Some(4), "ParameterRefRef"), "P-BOTH"),
        nd(6, Some(0), "ParameterBlock"),
        with_ref(nd(7, Some(6), "ParameterRefRef"), "P-NONE-THEN-READ"),
    ])
    .with_access([(1, "None"), (6, "Read")]);
    let activation = run(tree, &[]);
    // Listed once, at its first position, but writable where it is open.
    let ids: Vec<&str> = activation
        .parameter_refs
        .iter()
        .map(|r| r.ref_id.as_str())
        .collect();
    assert_eq!(ids, ["P-BOTH", "P-NONE-THEN-READ"]);
    assert_eq!(access_of(&activation, "P-BOTH"), None);
    assert_eq!(
        access_of(&activation, "P-NONE-THEN-READ").as_deref(),
        Some("Read")
    );
}

#[test]
fn separators_are_listed_where_they_were_activated_rulers_included() {
    let mut choose = with_ref(nd(5, Some(1), "choose"), "P-SEL");
    choose.control_kind = Some(ControlKind::Comparable);
    let mut when_on = nd(6, Some(5), "when");
    when_on.test = Some("1".to_string());
    let mut when_off = nd(8, Some(5), "when");
    when_off.test = Some("0".to_string());
    let tree = DynamicTree::from_nodes(vec![
        nd(0, None, "Dynamic"),
        nd(1, Some(0), "ParameterBlock"),
        with_text(nd(2, Some(1), "ParameterSeparator"), "Heading"),
        with_ref(nd(3, Some(1), "ParameterRefRef"), "P-1"),
        nd(4, Some(1), "ParameterSeparator"),
        choose,
        when_on,
        with_text(nd(7, Some(6), "ParameterSeparator"), "Shown"),
        when_off,
        with_text(nd(9, Some(8), "ParameterSeparator"), "Not shown"),
        with_ref(nd(10, Some(1), "ParameterRefRef"), "P-2"),
    ])
    .with_access([(1, "Read")]);
    let activation = run(tree, &[("P-SEL", "1")]);
    let read = Some("Read".to_string());
    assert_eq!(
        activation.separators,
        [
            ActiveSeparator {
                scope: None,
                node_id: 2,
                block_access: read.clone(),
                parameter_refs_before: 0,
            },
            ActiveSeparator {
                scope: None,
                node_id: 4,
                block_access: read.clone(),
                parameter_refs_before: 1,
            },
            ActiveSeparator {
                scope: None,
                node_id: 7,
                block_access: read,
                parameter_refs_before: 1,
            },
        ]
    );
    // Only the separators with text have a label; the ruler has none.
    let labelled: Vec<i64> = activation
        .labels
        .iter()
        .filter(|l| l.kind == "ParameterSeparator")
        .map(|l| l.node_id)
        .collect();
    assert_eq!(labelled, [2, 7]);
}
