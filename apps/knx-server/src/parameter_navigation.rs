//! Projects evaluated display ownership independently of parameter areas and module scopes.

use std::collections::HashMap;
use std::rc::Rc;

use knx_productdb::dynamic::{Activation, ChannelOwner, ModuleScope};
use knx_productdb::parameter_layout::ParameterLayout;

use crate::parameter_captions::{CaptionFills, CaptionKey};
use crate::routes::{ModuleScopeDto, ParameterContainerDto};

/// A leaf earns a parent only when every reached item agrees on its evaluated owner.
/// Missing/ambiguous layout metadata stays a root area, never a guessed hierarchy.
/// Index once per request, not once per module expansion or DTO copy.
pub(crate) struct AreaOwners<'a> {
    owners: HashMap<CaptionKey, Option<&'a ChannelOwner>>,
    labels: HashMap<CaptionKey, &'a str>,
}

impl<'a> AreaOwners<'a> {
    pub(crate) fn new(activation: &'a Activation, layout: &ParameterLayout) -> Self {
        let mut owners = HashMap::new();
        let mut admit = |scope: &Option<Rc<ModuleScope>>, node, owner| {
            let key = (scope.as_ref().map(|s| s.node_chain()), node);
            owners
                .entry(key)
                .and_modify(|previous| {
                    if *previous != owner {
                        *previous = None;
                    }
                })
                .or_insert(owner);
        };
        for active in &activation.parameter_refs {
            let definition = active
                .scope
                .as_ref()
                .map_or("", |s| s.module_def_id.as_str());
            if let Some(group) = layout.field(definition, &active.ref_id).group {
                admit(&active.scope, group.node_id, active.channel.as_ref());
            }
        }
        for active in &activation.separators {
            let definition = active
                .scope
                .as_ref()
                .map_or("", |s| s.module_def_id.as_str());
            if let Some(group) = layout
                .separator_declaration(definition, active.node_id)
                .and_then(|item| item.group.as_ref())
            {
                admit(&active.scope, group.node_id, active.channel.as_deref());
            }
        }
        for active in &activation.buttons {
            let definition = active
                .scope
                .as_ref()
                .map_or("", |s| s.module_def_id.as_str());
            if let Some(literal) = layout.button(definition, active.node_id) {
                admit(
                    &active.scope,
                    literal.group.node_id,
                    active.channel.as_deref(),
                );
            }
        }
        Self {
            owners,
            labels: activation
                .labels
                .iter()
                .filter(|label| {
                    matches!(label.kind.as_str(), "Channel" | "ChannelIndependentBlock")
                })
                .map(|label| {
                    (
                        (label.scope.as_ref().map(|s| s.node_chain()), label.node_id),
                        label.text.as_str(),
                    )
                })
                .collect(),
        }
    }

    pub(crate) fn container(
        &self,
        scope: &Option<Rc<ModuleScope>>,
        block: i64,
        captions: &mut CaptionFills,
    ) -> Result<Option<ParameterContainerDto>, String> {
        let key = (scope.as_ref().map(|s| s.node_chain()), block);
        let Some(owner) = self.owners.get(&key).copied().flatten() else {
            return Ok(None);
        };
        let key = (
            owner.scope.as_ref().map(|scope| scope.node_chain()),
            owner.node_id,
        );
        let (label, label_fill) = captions.copy(key.clone(), self.labels.get(&key).copied())?;
        let scope = owner
            .scope
            .as_ref()
            .map(|scope| {
                Ok::<_, String>(ModuleScopeDto {
                    node_chain: scope.node_chain(),
                    module_node: scope.module_node,
                    module_id: captions.copy_text(scope.module_id.as_deref())?,
                    module_def_id: captions
                        .copy_text(Some(&scope.module_def_id))?
                        .expect("present definition"),
                })
            })
            .transpose()?;
        Ok(Some(ParameterContainerDto {
            scope,
            node_id: owner.node_id,
            element_id: captions.copy_text(owner.element_id.as_deref())?,
            kind: captions
                .copy_text(Some(&owner.kind))?
                .expect("present kind"),
            label,
            label_fill,
        }))
    }
}
