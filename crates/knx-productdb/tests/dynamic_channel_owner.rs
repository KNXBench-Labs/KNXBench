//! Checks that every activated reference names the channel it was activated under (ISSUE-08).
//!
//! Hand-built trees, no database. The corpus measurement behind the rule:
//! in the three corpus projects' programs every `ComObjectRefRef` sits
//! under exactly one `Channel` (5,630 / 5,630 / 8), so "the innermost
//! enclosing channel element" is a total, unambiguous owner there.

use std::collections::HashMap;
use std::rc::Rc;

use knx_productdb::dynamic::{
    evaluate, ActiveRef, ChannelOwner, ControlKind, DynamicNode, DynamicTree, ModuleScope,
    ProgramTrees,
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

fn with_id(mut n: DynamicNode, id: &str) -> DynamicNode {
    n.element_id = Some(id.to_string());
    n
}

fn with_ref(mut n: DynamicNode, id: &str) -> DynamicNode {
    n.ref_id = Some(id.to_string());
    n
}

fn owner(scope: Option<Rc<ModuleScope>>, node_id: i64, kind: &str, id: &str) -> ChannelOwner {
    ChannelOwner {
        scope,
        node_id,
        kind: kind.to_string(),
        element_id: Some(id.to_string()),
        name: None,
        number: None,
    }
}

fn channel_of<'a>(refs: &'a [ActiveRef], id: &str) -> Option<&'a ChannelOwner> {
    refs.iter()
        .find(|r| r.ref_id == id)
        .unwrap_or_else(|| panic!("{id} was not activated"))
        .channel
        .as_ref()
}

fn program_only(nodes: Vec<DynamicNode>) -> ProgramTrees {
    ProgramTrees::from_parts(DynamicTree::from_nodes(nodes), HashMap::new())
}

#[test]
fn refs_name_their_channel_and_the_channel_independent_block() {
    let trees = program_only(vec![
        nd(0, None, "Dynamic"),
        with_id(nd(1, Some(0), "ChannelIndependentBlock"), "CIB"),
        with_ref(nd(2, Some(1), "ComObjectRefRef"), "O-GEN"),
        with_id(nd(3, Some(0), "Channel"), "CH-1"),
        nd(4, Some(3), "ParameterBlock"),
        with_ref(nd(5, Some(4), "ComObjectRefRef"), "O-A"),
        with_ref(nd(6, Some(4), "ParameterRefRef"), "P-A"),
    ]);
    let activation = evaluate(&trees, &HashMap::<String, String>::new().into());

    assert_eq!(
        channel_of(&activation.com_object_refs, "O-GEN"),
        Some(&owner(None, 1, "ChannelIndependentBlock", "CIB"))
    );
    assert_eq!(
        channel_of(&activation.com_object_refs, "O-A"),
        Some(&owner(None, 3, "Channel", "CH-1"))
    );
    assert_eq!(
        channel_of(&activation.parameter_refs, "P-A"),
        Some(&owner(None, 3, "Channel", "CH-1")),
        "parameters are owned the same way"
    );
}

#[test]
fn a_ref_outside_every_channel_has_no_owner_and_leaving_a_channel_forgets_it() {
    let trees = program_only(vec![
        nd(0, None, "Dynamic"),
        with_id(nd(1, Some(0), "Channel"), "CH-1"),
        with_ref(nd(2, Some(1), "ComObjectRefRef"), "O-IN"),
        // A sibling after the channel closed: must not inherit CH-1.
        with_ref(nd(3, Some(0), "ComObjectRefRef"), "O-OUT"),
    ]);
    let activation = evaluate(&trees, &HashMap::<String, String>::new().into());

    assert!(channel_of(&activation.com_object_refs, "O-IN").is_some());
    assert_eq!(channel_of(&activation.com_object_refs, "O-OUT"), None);
}

#[test]
fn a_ref_activated_through_a_choose_keeps_the_channel_around_the_choose() {
    let mut choose = with_ref(nd(2, Some(1), "choose"), "P-SEL");
    choose.control_kind = Some(ControlKind::Comparable);
    let mut when = nd(3, Some(2), "when");
    when.test = Some("1".to_string());
    let trees = program_only(vec![
        nd(0, None, "Dynamic"),
        with_id(nd(1, Some(0), "Channel"), "CH-1"),
        choose,
        when,
        with_ref(nd(4, Some(3), "ComObjectRefRef"), "O-WHEN"),
    ]);
    let values = HashMap::from([("P-SEL".to_string(), "1".to_string())]);
    let activation = evaluate(&trees, &values.into());

    assert_eq!(
        channel_of(&activation.com_object_refs, "O-WHEN"),
        Some(&owner(None, 1, "Channel", "CH-1"))
    );
}

#[test]
fn a_module_without_its_own_channel_inherits_the_channel_it_was_instantiated_in() {
    let program = DynamicTree::from_nodes(vec![
        nd(0, None, "Dynamic"),
        with_id(nd(1, Some(0), "Channel"), "CH-OUTER"),
        with_ref(with_id(nd(2, Some(1), "Module"), "M-1"), "MD-1"),
    ]);
    let module = DynamicTree::from_nodes(vec![
        nd(10, None, "Dynamic"),
        with_ref(nd(11, Some(10), "ComObjectRefRef"), "O-MOD"),
    ]);
    let trees = ProgramTrees::from_parts(program, HashMap::from([("MD-1".to_string(), module)]));
    let activation = evaluate(&trees, &HashMap::<String, String>::new().into());

    let r = &activation.com_object_refs[0];
    assert!(
        r.scope.is_some(),
        "the ref itself lives in the module's scope"
    );
    assert_eq!(
        r.channel,
        Some(owner(None, 1, "Channel", "CH-OUTER")),
        "its owner is the program's channel, named in the program's scope"
    );
}

#[test]
fn a_module_with_its_own_channel_names_that_channel_in_the_module_scope() {
    let program = DynamicTree::from_nodes(vec![
        nd(0, None, "Dynamic"),
        with_id(nd(1, Some(0), "Channel"), "CH-OUTER"),
        with_ref(with_id(nd(2, Some(1), "Module"), "M-1"), "MD-1"),
    ]);
    let module = DynamicTree::from_nodes(vec![
        nd(10, None, "Dynamic"),
        with_id(nd(11, Some(10), "Channel"), "MD-1_CH-1"),
        with_ref(nd(12, Some(11), "ComObjectRefRef"), "O-MOD"),
    ]);
    let trees = ProgramTrees::from_parts(program, HashMap::from([("MD-1".to_string(), module)]));
    let activation = evaluate(&trees, &HashMap::<String, String>::new().into());

    let r = &activation.com_object_refs[0];
    let channel = r.channel.as_ref().expect("owned");
    assert_eq!((channel.node_id, channel.kind.as_str()), (11, "Channel"));
    assert_eq!(channel.element_id.as_deref(), Some("MD-1_CH-1"));
    assert_eq!(
        channel.scope, r.scope,
        "the innermost channel wins and is named in the scope it was found in"
    );
}
