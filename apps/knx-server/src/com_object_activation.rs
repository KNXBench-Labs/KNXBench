//! Maps one device's evaluated `Dynamic` tree onto its communication objects (ISSUE-08).
//!
//! Pure: no locks, no queries. `domain::device_detail` gathers the inputs
//! (the evaluation it shares with the parameter panel, each object's lookup
//! id and module instance, translated channel texts) and this module decides
//! [`ComObjectActivation`] and [`ComObjectChannel`] per object.
//!
//! The rules, each one refusing to guess:
//!
//! - An unscoped object is `Active` when an unscoped `ComObjectRefRef`
//!   names its lookup id.
//! - A module-based object is `Active` when a scoped `ComObjectRefRef`
//!   names its lookup id and that scope's `Module/@Id` belongs to exactly
//!   this object's `ModuleInstance` (the parameter panel's D39 rule:
//!   `module_id` ends with `_<ModuleInstance RefId>`). Two instances with
//!   the same `RefId` cannot be told apart (D40), so their objects are
//!   `Undetermined`, never `Active` or `Inactive` by chance.
//! - Not activated is `Inactive` only when every stored value reached the
//!   evaluation (`values_complete`: none was set aside as stale), nothing in
//!   the evaluation could have hidden the object
//!   ([`Diagnostic::may_hide_refs`]), and no scoped activation of the same
//!   ref is left without an owner; otherwise `Undetermined`.
//! - `Active` is not downgraded by stale rows. It is exactly what the
//!   parameter panel shows from the same evaluation, and a stale row mostly
//!   names no parameter of this program or repeats one already fed, neither
//!   of which can switch a branch. The one exception, a module row whose
//!   `_MI-` digits disagree with the imported instance, leaves that
//!   parameter on its default: a limitation shared with the panel
//!   (KNOWN_LIMITATIONS, ISSUE-08 entry), not hidden here.
//!
//! [`Diagnostic::may_hide_refs`]: knx_productdb::dynamic::Diagnostic::may_hide_refs

use std::collections::HashMap;
use std::rc::Rc;

use knx_productdb::dynamic::{Activation, ActiveRef, ChannelOwner, ModuleScope};
use knx_projection::{ComObjectActivation, ComObjectChannel, ComObjectNode};

/// What `apply` needs to know about one communication object.
pub(crate) struct ComObjectKey<'a> {
    /// `knx_productdb::com_object_lookup_id` for this object.
    pub(crate) lookup_id: String,
    /// The object's own `ModuleInstance`, `None` for an unscoped object.
    pub(crate) module_instance: Option<&'a knx_core::ModuleInstance>,
}

/// Sets `activation` and `channel` on every object in `com_objects` that
/// `keys` names (by `ComObjectNode::id`). Objects `keys` does not name keep
/// `NotEvaluated`.
///
/// `channel_texts` maps a channel element's `@Id` to its `@Text` in the
/// requested language, placeholders not yet substituted; empty when no
/// language was requested or none is translated. `values_complete` is
/// `false` when any stored value could not be fed to the evaluation.
pub(crate) fn apply(
    com_objects: &mut [ComObjectNode],
    keys: &HashMap<u32, ComObjectKey<'_>>,
    device_instances: &[knx_core::ModuleInstance],
    activation: &Activation,
    channel_texts: &HashMap<String, String>,
    values_complete: bool,
) {
    let uncertain = !values_complete
        || activation
            .diagnostics
            .iter()
            .any(|d| d.diagnostic.may_hide_refs());
    let mut channels = ChannelTable::in_tree_order(activation);

    for com in com_objects.iter_mut() {
        let Some(key) = keys.get(&com.id) else {
            continue;
        };
        let (state, hit) = decide(key, device_instances, activation, uncertain);
        com.activation = state;
        com.channel = hit
            .and_then(|hit| hit.channel.as_ref())
            .map(|owner| channels.node(owner, activation, channel_texts));
        // A module-based object's `FunctionText` names the module's own
        // arguments (`{{argChannel}}`); only the activating expansion knows
        // their values. Without one the placeholder stays visible.
        if let Some(scope) = hit.and_then(|hit| hit.scope.as_deref()) {
            com.function_text = com
                .function_text
                .as_deref()
                .map(|text| knx_productdb::dynamic::substitute_text(text, Some(scope)));
        }
    }
}

fn decide<'a>(
    key: &ComObjectKey<'_>,
    device_instances: &[knx_core::ModuleInstance],
    activation: &'a Activation,
    uncertain: bool,
) -> (ComObjectActivation, Option<&'a ActiveRef>) {
    let same_ref = activation
        .com_object_refs
        .iter()
        .filter(|r| r.ref_id == key.lookup_id);

    match key.module_instance {
        None => match same_ref.clone().find(|r| r.scope.is_none()) {
            Some(hit) => (ComObjectActivation::Active, Some(hit)),
            None if uncertain => (ComObjectActivation::Undetermined, None),
            None => (ComObjectActivation::Inactive, None),
        },
        Some(own) => {
            if instances_sharing_ref(device_instances, own) > 1 {
                return (ComObjectActivation::Undetermined, None);
            }
            let mut orphaned = false;
            for hit in same_ref.filter(|r| r.scope.is_some()) {
                match owning_instance(hit, device_instances) {
                    Some(instance) if instance.id == own.id => {
                        return (ComObjectActivation::Active, Some(hit));
                    }
                    Some(_) => {}
                    None => orphaned = true,
                }
            }
            if uncertain || orphaned {
                (ComObjectActivation::Undetermined, None)
            } else {
                (ComObjectActivation::Inactive, None)
            }
        }
    }
}

/// How many of the device's `ModuleInstance`s share `own`'s `RefId`.
fn instances_sharing_ref(
    device_instances: &[knx_core::ModuleInstance],
    own: &knx_core::ModuleInstance,
) -> usize {
    device_instances
        .iter()
        .filter(|m| m.source.ets_id == own.source.ets_id)
        .count()
}

/// The one device `ModuleInstance` a scoped activation belongs to, by the
/// innermost scope's `Module/@Id` (D39: it ends with `_<RefId>`). `None`
/// when zero or several instances match.
fn owning_instance<'a>(
    hit: &ActiveRef,
    device_instances: &'a [knx_core::ModuleInstance],
) -> Option<&'a knx_core::ModuleInstance> {
    let module_id = hit.scope.as_ref()?.module_id.as_deref()?;
    let mut matches = device_instances
        .iter()
        .filter(|m| module_id.ends_with(&format!("_{}", m.source.ets_id)));
    let first = matches.next()?;
    matches.next().is_none().then_some(first)
}

/// Channel nodes built so far, so every object of one channel shares one
/// node, plus each channel's position in the evaluated tree.
struct ChannelTable {
    order: HashMap<String, u32>,
    known: HashMap<String, ComObjectChannel>,
}

impl ChannelTable {
    /// `com_object_refs` is pushed in walk order, which is document order
    /// with each module expansion in place, so the first ref of each
    /// channel fixes that channel's position.
    fn in_tree_order(activation: &Activation) -> ChannelTable {
        let mut order = HashMap::new();
        for owner in activation
            .com_object_refs
            .iter()
            .filter_map(|r| r.channel.as_ref())
        {
            let next = u32::try_from(order.len()).unwrap_or(u32::MAX);
            order.entry(channel_key(owner)).or_insert(next);
        }
        ChannelTable {
            order,
            known: HashMap::new(),
        }
    }

    fn node(
        &mut self,
        owner: &ChannelOwner,
        activation: &Activation,
        channel_texts: &HashMap<String, String>,
    ) -> ComObjectChannel {
        let key = channel_key(owner);
        let order = self.order.get(&key).copied().unwrap_or(u32::MAX);
        self.known
            .entry(key.clone())
            .or_insert_with(|| ComObjectChannel {
                key,
                kind: owner.kind.clone(),
                text: channel_text(owner, activation, channel_texts),
                name: non_empty(owner.name.as_deref()),
                number: non_empty(owner.number.as_deref()),
                order,
            })
            .clone()
    }
}

/// An attribute the file states, or `None` when it is absent or empty: an
/// empty `@Name` names nothing (4 corpus channels have one).
fn non_empty(value: Option<&str>) -> Option<String> {
    value.filter(|v| !v.is_empty()).map(str::to_string)
}

/// `<module node chain>/<node id>`: node ids restart per `Dynamic` tree, so
/// the chain of `Module` nodes that led there is part of the identity, the
/// same key the evaluator itself uses for its activation sets.
fn channel_key(owner: &ChannelOwner) -> String {
    let chain = owner
        .scope
        .as_ref()
        .map(|s| s.node_chain())
        .unwrap_or_default()
        .iter()
        .map(i64::to_string)
        .collect::<Vec<_>>()
        .join(".");
    format!("{chain}/{}", owner.node_id)
}

/// The translated `@Text` when there is one, else the label the evaluator
/// recorded; module arguments substituted either way.
fn channel_text(
    owner: &ChannelOwner,
    activation: &Activation,
    channel_texts: &HashMap<String, String>,
) -> Option<String> {
    let scope: Option<&ModuleScope> = owner.scope.as_deref();
    if let Some(translated) = owner
        .element_id
        .as_ref()
        .and_then(|id| channel_texts.get(id))
        .filter(|t| !t.is_empty())
    {
        return Some(knx_productdb::dynamic::substitute_text(translated, scope));
    }
    activation
        .labels
        .iter()
        .find(|l| {
            l.node_id == owner.node_id
                && l.kind == owner.kind
                && same_scope(l.scope.as_ref(), owner.scope.as_ref())
        })
        .map(|l| l.text.clone())
}

fn same_scope(a: Option<&Rc<ModuleScope>>, b: Option<&Rc<ModuleScope>>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => a.node_chain() == b.node_chain(),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use knx_productdb::dynamic::{
        evaluate, ControlKind, DynamicNode, DynamicTree, ProgramTrees, ValueMap,
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

    fn channel(node_id: i64, parent: i64, id: &str, text: &str) -> DynamicNode {
        let mut n = nd(node_id, Some(parent), "Channel");
        n.element_id = Some(id.to_string());
        n.text = Some(text.to_string());
        n
    }

    fn com_ref(node_id: i64, parent: i64, ref_id: &str) -> DynamicNode {
        let mut n = nd(node_id, Some(parent), "ComObjectRefRef");
        n.ref_id = Some(ref_id.to_string());
        n
    }

    fn com_node(id: u32) -> ComObjectNode {
        ComObjectNode {
            id,
            number: 0,
            name: None,
            dpt: None,
            dpt_layer: None,
            description: None,
            description_layer: None,
            is_active: true,
            read: false,
            write: false,
            transmit: false,
            update: false,
            communication: false,
            read_on_init: false,
            links: vec![],
            activation: ComObjectActivation::NotEvaluated,
            channel: None,
            program_dpt: None,
            dpt_text: None,
            function_text: None,
        }
    }

    fn unscoped(lookup: &str) -> ComObjectKey<'static> {
        ComObjectKey {
            lookup_id: lookup.to_string(),
            module_instance: None,
        }
    }

    /// Dynamic > Channel "A" > O-1, Channel "B" > O-2.
    fn two_channels() -> ProgramTrees {
        ProgramTrees::single(DynamicTree::from_nodes(vec![
            nd(1, None, "Dynamic"),
            channel(2, 1, "CH-1", "Kanal A"),
            com_ref(3, 2, "O-1"),
            channel(4, 1, "CH-2", "Kanal B"),
            com_ref(5, 4, "O-2"),
        ]))
    }

    fn run(
        trees: &ProgramTrees,
        keys: &HashMap<u32, ComObjectKey<'_>>,
        instances: &[knx_core::ModuleInstance],
        texts: &HashMap<String, String>,
        ids: &[u32],
    ) -> Vec<ComObjectNode> {
        let activation = evaluate(trees, &ValueMap::default());
        let mut nodes: Vec<ComObjectNode> = ids.iter().map(|&id| com_node(id)).collect();
        apply(&mut nodes, keys, instances, &activation, texts, true);
        nodes
    }

    #[test]
    fn activated_objects_carry_their_channel_and_its_text() {
        let keys = HashMap::from([(1, unscoped("O-1")), (2, unscoped("O-2"))]);
        let nodes = run(&two_channels(), &keys, &[], &HashMap::new(), &[1, 2]);
        assert_eq!(nodes[0].activation, ComObjectActivation::Active);
        let a = nodes[0].channel.clone().unwrap();
        let b = nodes[1].channel.clone().unwrap();
        assert_eq!(a.text.as_deref(), Some("Kanal A"));
        assert_eq!(b.text.as_deref(), Some("Kanal B"));
        assert_ne!(a.key, b.key);
        assert_eq!((a.order, b.order), (0, 1));
        assert_eq!(a.kind, "Channel");
    }

    #[test]
    fn channel_order_follows_the_tree_not_the_object_list() {
        let keys = HashMap::from([(1, unscoped("O-1")), (2, unscoped("O-2"))]);
        // Object 2 (channel B) is listed first; B is still second in the tree.
        let nodes = run(&two_channels(), &keys, &[], &HashMap::new(), &[2, 1]);
        assert_eq!(nodes[0].channel.as_ref().unwrap().order, 1);
        assert_eq!(nodes[1].channel.as_ref().unwrap().order, 0);
    }

    #[test]
    fn the_channel_carries_its_name_and_number_as_written() {
        // ADR-0052: shown verbatim, never composed into `text`, and an
        // empty attribute names nothing.
        let mut named = channel(2, 1, "CH-1", "");
        named.name = Some("Light A".to_string());
        named.number = Some("1".to_string());
        let mut unnamed = channel(4, 1, "CH-2", "Kanal B");
        unnamed.name = Some(String::new());
        unnamed.number = Some("B".to_string());
        let trees = ProgramTrees::single(DynamicTree::from_nodes(vec![
            nd(1, None, "Dynamic"),
            named,
            com_ref(3, 2, "O-1"),
            unnamed,
            com_ref(5, 4, "O-2"),
        ]));
        let keys = HashMap::from([(1, unscoped("O-1")), (2, unscoped("O-2"))]);
        let nodes = run(&trees, &keys, &[], &HashMap::new(), &[1, 2]);
        let a = nodes[0].channel.clone().unwrap();
        let b = nodes[1].channel.clone().unwrap();
        assert_eq!(
            (a.name.as_deref(), a.number.as_deref()),
            (Some("Light A"), Some("1"))
        );
        assert_eq!(a.text, None, "an empty @Text stays empty");
        assert_eq!((b.name.as_deref(), b.number.as_deref()), (None, Some("B")));
        assert_eq!(b.text.as_deref(), Some("Kanal B"));
    }

    #[test]
    fn a_translated_channel_text_wins_over_the_stored_one() {
        let keys = HashMap::from([(1, unscoped("O-1"))]);
        let texts = HashMap::from([("CH-1".to_string(), "Channel A".to_string())]);
        let nodes = run(&two_channels(), &keys, &[], &texts, &[1]);
        assert_eq!(
            nodes[0].channel.as_ref().unwrap().text.as_deref(),
            Some("Channel A")
        );
    }

    #[test]
    fn an_object_no_ref_names_is_inactive_on_a_clean_evaluation() {
        let keys = HashMap::from([(9, unscoped("O-9"))]);
        let nodes = run(&two_channels(), &keys, &[], &HashMap::new(), &[9]);
        assert_eq!(nodes[0].activation, ComObjectActivation::Inactive);
        assert!(nodes[0].channel.is_none());
    }

    #[test]
    fn a_stale_stored_value_makes_inactive_undetermined() {
        let keys = HashMap::from([(9, unscoped("O-9"))]);
        let activation = evaluate(&two_channels(), &ValueMap::default());
        let mut nodes = vec![com_node(9)];
        apply(&mut nodes, &keys, &[], &activation, &HashMap::new(), false);
        assert_eq!(nodes[0].activation, ComObjectActivation::Undetermined);
    }

    #[test]
    fn an_object_not_in_keys_stays_not_evaluated() {
        let nodes = run(&two_channels(), &HashMap::new(), &[], &HashMap::new(), &[1]);
        assert_eq!(nodes[0].activation, ComObjectActivation::NotEvaluated);
    }

    /// A `choose` whose parameter has no value is `MissingValue`: its
    /// branches were never walked, so an object not activated may still be.
    #[test]
    fn a_diagnostic_that_may_hide_refs_makes_inactive_undetermined() {
        let mut choose = nd(3, Some(2), "choose");
        choose.ref_id = Some("P-UNKNOWN".to_string());
        choose.control_kind = Some(ControlKind::Comparable);
        let mut when = nd(4, Some(3), "when");
        when.test = Some("1".to_string());
        let trees = ProgramTrees::single(DynamicTree::from_nodes(vec![
            nd(1, None, "Dynamic"),
            channel(2, 1, "CH-1", "A"),
            choose,
            when,
            com_ref(5, 4, "O-1"),
        ]));
        let keys = HashMap::from([(1, unscoped("O-1"))]);
        let nodes = run(&trees, &keys, &[], &HashMap::new(), &[1]);
        assert_eq!(nodes[0].activation, ComObjectActivation::Undetermined);
    }

    fn instance(id: u32, ref_id: &str) -> knx_core::ModuleInstance {
        knx_core::ModuleInstance {
            id: knx_core::ModuleInstanceId(id),
            device: knx_core::DeviceId(1),
            source: knx_core::SourceRef {
                path: String::new(),
                ets_id: ref_id.to_string(),
            },
            repeat_index: String::new(),
            instance_ets_id: format!("{ref_id}_MI-1"),
            arguments: vec![],
        }
    }

    /// Dynamic > Channel > Module(M-1), Channel > Module(M-2), both of MD-1
    /// whose tree is Channel "{{Name}}" > ComObjectRefRef MD-1_O-1.
    fn module_program() -> ProgramTrees {
        let mut m1 = nd(3, Some(2), "Module");
        m1.element_id = Some("A_MD-1_M-1".to_string());
        m1.ref_id = Some("A_MD-1".to_string());
        let mut m2 = nd(5, Some(4), "Module");
        m2.element_id = Some("A_MD-1_M-2".to_string());
        m2.ref_id = Some("A_MD-1".to_string());
        let program = DynamicTree::from_nodes(vec![
            nd(1, None, "Dynamic"),
            nd(2, Some(1), "ChannelIndependentBlock"),
            m1,
            nd(4, Some(1), "ChannelIndependentBlock"),
            m2,
        ]);
        let module = DynamicTree::from_nodes(vec![
            nd(1, None, "Dynamic"),
            channel(2, 1, "A_MD-1_CH-1", "Kanal"),
            com_ref(3, 2, "A_MD-1_O-1"),
        ]);
        ProgramTrees::from_parts(program, HashMap::from([("A_MD-1".to_string(), module)]))
    }

    #[test]
    fn module_objects_are_active_under_their_own_instances_channel() {
        let instances = vec![instance(10, "MD-1_M-1"), instance(11, "MD-1_M-2")];
        let keys = HashMap::from([
            (
                1,
                ComObjectKey {
                    lookup_id: "A_MD-1_O-1".to_string(),
                    module_instance: Some(&instances[0]),
                },
            ),
            (
                2,
                ComObjectKey {
                    lookup_id: "A_MD-1_O-1".to_string(),
                    module_instance: Some(&instances[1]),
                },
            ),
        ]);
        let nodes = run(
            &module_program(),
            &keys,
            &instances,
            &HashMap::new(),
            &[1, 2],
        );
        assert_eq!(nodes[0].activation, ComObjectActivation::Active);
        assert_eq!(nodes[1].activation, ComObjectActivation::Active);
        let (a, b) = (
            nodes[0].channel.clone().unwrap(),
            nodes[1].channel.clone().unwrap(),
        );
        // Same channel element in the same ModuleDef, two expansions: two
        // channels, not one.
        assert_ne!(a.key, b.key);
        assert_eq!(a.text.as_deref(), Some("Kanal"));
    }

    #[test]
    fn a_module_objects_function_text_gets_its_own_instances_arguments() {
        let mut m1 = nd(3, Some(2), "Module");
        m1.element_id = Some("A_MD-1_M-1".to_string());
        m1.ref_id = Some("A_MD-1".to_string());
        let mut arg1 = nd(4, Some(3), "TextArg");
        arg1.ref_id = Some("A_MD-1_A-1".to_string());
        arg1.value = Some("7".to_string());
        let mut m2 = nd(6, Some(5), "Module");
        m2.element_id = Some("A_MD-1_M-2".to_string());
        m2.ref_id = Some("A_MD-1".to_string());
        let mut arg2 = nd(7, Some(6), "TextArg");
        arg2.ref_id = Some("A_MD-1_A-1".to_string());
        arg2.value = Some("8".to_string());
        let program = DynamicTree::from_nodes(vec![
            nd(1, None, "Dynamic"),
            nd(2, Some(1), "ChannelIndependentBlock"),
            m1,
            arg1,
            nd(5, Some(1), "ChannelIndependentBlock"),
            m2,
            arg2,
        ]);
        let module = DynamicTree::from_nodes(vec![
            nd(1, None, "Dynamic"),
            channel(2, 1, "A_MD-1_CH-1", "Kanal {{No}}"),
            com_ref(3, 2, "A_MD-1_O-1"),
        ]);
        let trees =
            ProgramTrees::from_parts(program, HashMap::from([("A_MD-1".to_string(), module)]))
                .with_arguments([knx_productdb::dynamic::ModuleDefArgument {
                    id: "A_MD-1_A-1".to_string(),
                    module_def_id: "A_MD-1".to_string(),
                    name: Some("No".to_string()),
                    arg_type: Some("Text".to_string()),
                    allocates: None,
                }]);
        let instances = vec![instance(10, "MD-1_M-1"), instance(11, "MD-1_M-2")];
        let keys = HashMap::from([
            (
                1,
                ComObjectKey {
                    lookup_id: "A_MD-1_O-1".to_string(),
                    module_instance: Some(&instances[0]),
                },
            ),
            (
                2,
                ComObjectKey {
                    lookup_id: "A_MD-1_O-1".to_string(),
                    module_instance: Some(&instances[1]),
                },
            ),
            (3, unscoped("A_O-9")),
        ]);
        let activation = evaluate(&trees, &ValueMap::default());
        let mut nodes: Vec<ComObjectNode> = [1, 2, 3].iter().map(|&id| com_node(id)).collect();
        for node in &mut nodes {
            node.function_text = Some("Schalten {{No}}".to_string());
        }
        apply(
            &mut nodes,
            &keys,
            &instances,
            &activation,
            &HashMap::new(),
            true,
        );
        assert_eq!(nodes[0].function_text.as_deref(), Some("Schalten 7"));
        assert_eq!(nodes[1].function_text.as_deref(), Some("Schalten 8"));
        assert_eq!(
            nodes[0].channel.as_ref().unwrap().text.as_deref(),
            Some("Kanal 7")
        );
        // No activating expansion, no scope: the placeholder stays visible
        // rather than being filled from some other instance.
        assert_eq!(nodes[2].function_text.as_deref(), Some("Schalten {{No}}"));
    }

    #[test]
    fn two_instances_sharing_one_ref_id_are_undetermined() {
        let instances = vec![instance(10, "MD-1_M-1"), instance(11, "MD-1_M-1")];
        let keys = HashMap::from([(
            1,
            ComObjectKey {
                lookup_id: "A_MD-1_O-1".to_string(),
                module_instance: Some(&instances[0]),
            },
        )]);
        let nodes = run(&module_program(), &keys, &instances, &HashMap::new(), &[1]);
        assert_eq!(nodes[0].activation, ComObjectActivation::Undetermined);
        assert!(nodes[0].channel.is_none());
    }

    /// D40: two instances with one `RefId` cannot be fed their own values,
    /// so even a ref no expansion activates is not known to be inactive.
    #[test]
    fn an_unactivated_ref_of_a_repeated_instance_is_undetermined() {
        let instances = vec![instance(10, "MD-1_M-1"), instance(11, "MD-1_M-1")];
        let keys = HashMap::from([(
            1,
            ComObjectKey {
                lookup_id: "A_MD-1_O-NOT-IN-TREE".to_string(),
                module_instance: Some(&instances[0]),
            },
        )]);
        let nodes = run(&module_program(), &keys, &instances, &HashMap::new(), &[1]);
        assert_eq!(nodes[0].activation, ComObjectActivation::Undetermined);
    }

    #[test]
    fn a_scoped_activation_no_instance_owns_makes_a_miss_undetermined() {
        // Only M-2 is imported; M-1's activation has no owner on this
        // device, so M-2's object is Active and nothing is Inactive by
        // default -- and an M-3 object that no activation names is
        // Undetermined, since M-1's orphaned hit could have been its own.
        let instances = vec![instance(11, "MD-1_M-2"), instance(12, "MD-1_M-3")];
        let keys = HashMap::from([
            (
                2,
                ComObjectKey {
                    lookup_id: "A_MD-1_O-1".to_string(),
                    module_instance: Some(&instances[0]),
                },
            ),
            (
                3,
                ComObjectKey {
                    lookup_id: "A_MD-1_O-1".to_string(),
                    module_instance: Some(&instances[1]),
                },
            ),
        ]);
        let nodes = run(
            &module_program(),
            &keys,
            &instances,
            &HashMap::new(),
            &[2, 3],
        );
        assert_eq!(nodes[0].activation, ComObjectActivation::Active);
        assert_eq!(nodes[1].activation, ComObjectActivation::Undetermined);
    }
}
