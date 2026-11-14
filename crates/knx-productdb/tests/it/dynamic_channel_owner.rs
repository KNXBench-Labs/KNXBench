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

#[test]
fn textless_separator_owner_metadata_is_shared_across_fanout() {
    // Metadata length and child count vary independently; no RSS/OOM oracle.
    for (id_len, name_len, number_len) in [(8, 8, 8), (8192, 8, 8), (8, 8192, 8), (8, 8, 8192)] {
        for count in [2, 64] {
            let mut channel = nd(1, Some(0), "Channel");
            channel.element_id = Some("i".repeat(id_len));
            channel.name = Some("n".repeat(name_len));
            channel.number = Some("7".repeat(number_len));
            channel.text = Some("Fable channel".into());
            let mut nodes = vec![nd(0, None, "Dynamic"), channel];
            nodes.extend((0..count).map(|index| nd(index + 2, Some(1), "ParameterSeparator")));
            let activation = evaluate(
                &program_only(nodes),
                &HashMap::<String, String>::new().into(),
            );
            assert_eq!(activation.separators.len(), count as usize);
            assert!(activation.parameter_refs.is_empty());
            assert!(activation.diagnostics.is_empty());
            assert_eq!(activation.labels.len(), 1, "textless rulers have no labels");
            assert_eq!(activation.labels[0].text, "Fable channel");
            let first = activation.separators[0].channel.as_ref().unwrap();
            assert_eq!(first.node_id, 1);
            assert_eq!(first.kind, "Channel");
            assert!(first.scope.is_none());
            assert_eq!(
                first.element_id.as_deref(),
                Some("i".repeat(id_len).as_str())
            );
            assert_eq!(first.name.as_deref(), Some("n".repeat(name_len).as_str()));
            assert_eq!(
                first.number.as_deref(),
                Some("7".repeat(number_len).as_str())
            );
            for (index, separator) in activation.separators.iter().enumerate() {
                assert_eq!(separator.node_id, index as i64 + 2);
                assert_eq!(separator.parameter_refs_before, 0);
                assert!(separator.block_access.is_none());
                let owner = separator.channel.as_ref().unwrap();
                assert_eq!(owner, first);
                for (actual, shared) in [
                    (&owner.element_id, &first.element_id),
                    (&owner.name, &first.name),
                    (&owner.number, &first.number),
                ] {
                    assert_eq!(
                        actual.as_ref().unwrap().as_ptr(),
                        shared.as_ref().unwrap().as_ptr(),
                        "separator fanout must not copy owner payload: lengths={id_len}/{name_len}/{number_len}, count={count}"
                    );
                }
            }
        }
    }
}

#[test]
fn shared_passive_owners_preserve_inherited_and_instance_scopes() {
    let mut outer = with_id(nd(1, Some(0), "Channel"), "FABLE-CH");
    outer.text = Some("Same caption".into());
    let mut inner = with_id(nd(13, Some(10), "Channel"), "FABLE-CH");
    inner.text = Some("Same caption".into());
    let program = DynamicTree::from_nodes(vec![
        nd(0, None, "Dynamic"),
        outer,
        with_ref(with_id(nd(2, Some(1), "Module"), "FABLE-A"), "MD-FABLE"),
        with_ref(with_id(nd(3, Some(1), "Module"), "FABLE-B"), "MD-FABLE"),
        nd(4, Some(0), "ParameterSeparator"),
        nd(5, Some(0), "Button"),
    ]);
    let module = DynamicTree::from_nodes(vec![
        nd(10, None, "Dynamic"),
        nd(11, Some(10), "ParameterSeparator"),
        nd(12, Some(10), "Button"),
        inner,
        nd(14, Some(13), "ParameterBlock"),
        nd(15, Some(14), "ParameterSeparator"),
        nd(16, Some(14), "Button"),
    ])
    .with_access([(14, "Read")]);
    let trees = ProgramTrees::from_parts(program, HashMap::from([("MD-FABLE".into(), module)]));
    let activation = evaluate(&trees, &HashMap::<String, String>::new().into());
    assert_eq!(activation.separators.len(), 5);
    assert_eq!(activation.buttons.len(), 5);
    for (index, chain) in [(0, vec![2]), (2, vec![3])] {
        let separator = &activation.separators[index];
        let button = &activation.buttons[index];
        assert_eq!(separator.scope.as_ref().unwrap().node_chain(), chain);
        assert_eq!(separator.scope, button.scope);
        let inherited = separator.channel.as_ref().unwrap();
        assert!(inherited.scope.is_none());
        assert_eq!(inherited.node_id, 1);
        assert_eq!(button.channel.as_ref(), Some(inherited));
        let local = &activation.separators[index + 1];
        let local_owner = local.channel.as_ref().unwrap();
        assert_eq!(local_owner.node_id, 13);
        assert_eq!(local_owner.scope, local.scope);
        assert_eq!(local_owner.scope.as_ref().unwrap().node_chain(), chain);
        assert_eq!(local.block_access.as_deref(), Some("Read"));
        assert_eq!(
            activation.buttons[index + 1].block_access.as_deref(),
            Some("Read")
        );
        assert_eq!(
            activation.buttons[index + 1].channel.as_ref(),
            Some(local_owner)
        );
    }
    assert!(activation.separators[4].channel.is_none());
    assert!(activation.buttons[4].channel.is_none());
    let captions: Vec<_> = activation
        .labels
        .iter()
        .map(|label| {
            (
                label.scope.as_ref().map(|scope| scope.node_chain()),
                label.node_id,
                label.text.as_str(),
            )
        })
        .collect();
    assert_eq!(
        captions,
        [
            (None, 1, "Same caption"),
            (Some(vec![2]), 13, "Same caption"),
            (Some(vec![3]), 13, "Same caption")
        ]
    );
}
