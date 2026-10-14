//! What a device's positional `{{0}}` texts read, by host (ADR-0126).
//!
//! [`TextParameters`] holds one program's references and the product facts
//! behind them; [`DeviceTexts`] adds one device's evaluation and fills a
//! channel heading or an object name through [`fill_positional`]. Pure over
//! the shared [`DeviceEvaluation`]: the value is the device's stored one,
//! otherwise the product default, and a module instance that cannot be told
//! apart decides nothing.

use std::collections::HashMap;

use rusqlite::Connection;

use crate::device_evaluation::{resolve_mi_authority, DeviceEvaluation, MiAuthority};
use crate::dynamic::{
    fill_positional, ModuleScope, PositionalText, PositionalValue, TextProjectionBudget,
    TextProjectionError, UnavailableReason,
};
use crate::query::{
    text_parameter_facts, text_parameter_refs, TextParameterFact, TextParameterRefs,
};
use crate::ProductDbError;

/// One program's references and the facts of the parameters they name.
#[derive(Debug, Clone, Default)]
pub struct TextParameters {
    refs: TextParameterRefs,
    facts: HashMap<String, TextParameterFact>,
}

impl TextParameters {
    pub fn load(conn: &Connection, program_id: &str) -> Result<Self, ProductDbError> {
        let refs = text_parameter_refs(conn, program_id)?;
        let mut ids: Vec<&str> = refs
            .dynamic
            .values()
            .chain(refs.com_object_refs.values())
            .map(String::as_str)
            .collect();
        ids.sort_unstable();
        ids.dedup();
        let facts = text_parameter_facts(conn, program_id, &ids)?;
        Ok(Self { refs, facts })
    }

    /// Built from parts, for callers and tests that already hold them.
    pub fn from_parts(refs: TextParameterRefs, facts: HashMap<String, TextParameterFact>) -> Self {
        Self { refs, facts }
    }
}

/// Where a host sits, for finding its value on one device.
#[derive(Debug, Clone, Copy)]
pub enum TextParameterScope<'a> {
    /// The program's own tree, or an object outside any module.
    Program,
    /// A channel in an evaluated `Module` (`ModuleScope::module_id`, which a
    /// `Module` without `@Id` lacks).
    Module(Option<&'a str>),
    /// A communication object of one imported module instance.
    Instance(&'a knx_core::ModuleInstance),
}

/// One device's evaluation next to its program's text parameters.
pub struct DeviceTexts<'a> {
    pub parameters: &'a TextParameters,
    pub evaluation: &'a DeviceEvaluation,
    pub module_instances: &'a [knx_core::ModuleInstance],
}

impl<'a> DeviceTexts<'a> {
    /// Fill a channel heading (`text`: translated or evaluated, module arguments
    /// already substituted) of the node `node_id` in `scope`'s tree.
    pub fn fill_channel(
        &self,
        text: &str,
        scope: Option<&ModuleScope>,
        node_id: i64,
        budget: &mut TextProjectionBudget,
    ) -> Result<PositionalText, TextProjectionError> {
        let tree = scope.map_or("", |s| s.module_def_id.as_str());
        let reference = self
            .parameters
            .refs
            .dynamic
            .get(&(tree.to_string(), node_id))
            .map(String::as_str);
        let place = match scope {
            None => TextParameterScope::Program,
            Some(s) => TextParameterScope::Module(s.module_id.as_deref()),
        };
        fill_positional(text, self.value(reference, place), budget)
    }

    /// Fill an object name (a product text) of the object whose `ComObjectRef`
    /// lookup id is `lookup_id`, in its own module `instance` if it has one.
    pub fn fill_object(
        &self,
        text: &str,
        lookup_id: &str,
        instance: Option<&knx_core::ModuleInstance>,
        budget: &mut TextProjectionBudget,
    ) -> Result<PositionalText, TextProjectionError> {
        let reference = self
            .parameters
            .refs
            .com_object_refs
            .get(lookup_id)
            .map(String::as_str);
        let place = instance.map_or(TextParameterScope::Program, TextParameterScope::Instance);
        fill_positional(text, self.value(reference, place), budget)
    }

    /// The value `reference` has on this device, or why none can be used.
    pub fn value(
        &self,
        reference: Option<&str>,
        scope: TextParameterScope<'_>,
    ) -> PositionalValue<'a> {
        let Some(reference) = reference else {
            return PositionalValue::Unavailable(UnavailableReason::NoReference);
        };
        let Some(fact) = self.parameters.facts.get(reference) else {
            return PositionalValue::Unavailable(UnavailableReason::UnknownParameter);
        };
        if !fact.is_text {
            return PositionalValue::Unavailable(UnavailableReason::NotTextParameter);
        }
        let default = PositionalValue::ProductDefault(fact.default.as_deref().unwrap_or(""));
        let evaluation = self.evaluation;
        match scope {
            TextParameterScope::Program => evaluation
                .supplied
                .get(reference)
                .map_or(default, |v| PositionalValue::Stored(v)),
            TextParameterScope::Module(None) => {
                PositionalValue::Unavailable(UnavailableReason::Undecided)
            }
            TextParameterScope::Module(Some(module_id)) => {
                match resolve_mi_authority(self.module_instances, module_id) {
                    MiAuthority::Ambiguous { .. } | MiAuthority::Malformed { .. } => {
                        PositionalValue::Unavailable(UnavailableReason::Undecided)
                    }
                    MiAuthority::Found(_) | MiAuthority::NoMatch => evaluation
                        .validated_scoped
                        .get(&(module_id.to_string(), reference.to_string()))
                        .map_or(default, |v| PositionalValue::Stored(v)),
                }
            }
            TextParameterScope::Instance(instance) => evaluation
                .instance_scoped_ids
                .values()
                .find(|(_, id, declared)| id == &instance.instance_ets_id && declared == reference)
                .and_then(|(module_id, id, declared)| {
                    evaluation.values.get_instance(module_id, id, declared)
                })
                .map_or(default, PositionalValue::Stored),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dynamic::{Activation, PositionalSource, ValueMap};

    fn fact(is_text: bool, default: &str) -> TextParameterFact {
        TextParameterFact {
            is_text,
            default: Some(default.to_string()),
        }
    }

    fn parameters() -> TextParameters {
        let mut refs = TextParameterRefs::default();
        refs.dynamic.insert((String::new(), 3), "R-name".into());
        refs.dynamic
            .insert(("MD-1".into(), 2), "MD-1_R-name".into());
        refs.com_object_refs
            .insert("O-1_R-1".into(), "R-name".into());
        refs.com_object_refs
            .insert("MD-1_O-1_R-1".into(), "MD-1_R-name".into());
        refs.com_object_refs
            .insert("O-2_R-1".into(), "R-count".into());
        let facts = HashMap::from([
            ("R-name".to_string(), fact(true, "")),
            ("MD-1_R-name".to_string(), fact(true, "Button")),
            ("R-count".to_string(), fact(false, "1")),
        ]);
        TextParameters::from_parts(refs, facts)
    }

    fn evaluation() -> DeviceEvaluation {
        let mut values = ValueMap::default();
        values.insert_instance(
            "A_MD-1_M-1".into(),
            "MD-1_M-1_MI-2".into(),
            "MD-1_R-name".into(),
            "Up".into(),
        );
        DeviceEvaluation {
            ref_ids: Default::default(),
            supplied: HashMap::from([("R-name".to_string(), "Kitchen".to_string())]),
            validated_scoped: HashMap::from([(
                ("A_MD-1_M-1".to_string(), "MD-1_R-name".to_string()),
                "Down".to_string(),
            )]),
            scoped_ids: Default::default(),
            instance_scoped_ids: HashMap::from([(
                "A_MD-1_M-1_MI-2_R-name".to_string(),
                (
                    "A_MD-1_M-1".into(),
                    "MD-1_M-1_MI-2".into(),
                    "MD-1_R-name".into(),
                ),
            )]),
            stale: vec![],
            findings: vec![],
            values,
            activation: Activation::default(),
        }
    }

    fn instance(ref_id: &str, instance_id: &str) -> knx_core::ModuleInstance {
        knx_core::ModuleInstance {
            id: knx_core::ModuleInstanceId(1),
            device: knx_core::DeviceId(1),
            source: knx_core::SourceRef {
                path: String::new(),
                ets_id: ref_id.to_string(),
            },
            repeat_index: String::new(),
            instance_ets_id: instance_id.to_string(),
            arguments: vec![],
        }
    }

    fn scope(module_id: Option<&str>) -> ModuleScope {
        ModuleScope {
            module_node: 7,
            module_id: module_id.map(str::to_string),
            module_def_id: "MD-1".into(),
            arguments: vec![],
            parent: None,
        }
    }

    fn filled(text: &str, source: PositionalSource) -> PositionalText {
        PositionalText {
            text: text.to_string(),
            source,
        }
    }

    #[test]
    fn a_program_host_reads_the_stored_value_then_the_product_default() {
        let parameters = parameters();
        let stored = evaluation();
        let instances = [];
        let texts = DeviceTexts {
            parameters: &parameters,
            evaluation: &stored,
            module_instances: &instances,
        };
        let mut budget = TextProjectionBudget::default();
        assert_eq!(
            texts
                .fill_channel("{{0:Channel A}}", None, 3, &mut budget)
                .unwrap(),
            filled("Kitchen", PositionalSource::StoredValue)
        );
        assert_eq!(
            texts
                .fill_object("Switch {{0:Out}}", "O-1_R-1", None, &mut budget)
                .unwrap(),
            filled("Switch Kitchen", PositionalSource::StoredValue)
        );
        // Nothing stored: the (empty) product default, so the template's own text.
        let empty = DeviceEvaluation {
            supplied: HashMap::new(),
            ..evaluation()
        };
        let texts = DeviceTexts {
            evaluation: &empty,
            ..texts
        };
        assert_eq!(
            texts
                .fill_channel("{{0:Channel A}}", None, 3, &mut budget)
                .unwrap(),
            filled("Channel A", PositionalSource::TemplateDefault)
        );
    }

    #[test]
    fn a_missing_reference_or_a_non_text_parameter_decides_nothing() {
        let parameters = parameters();
        let stored = evaluation();
        let texts = DeviceTexts {
            parameters: &parameters,
            evaluation: &stored,
            module_instances: &[],
        };
        let mut budget = TextProjectionBudget::default();
        let unresolved = |reason| PositionalSource::Unresolved(reason);
        assert_eq!(
            texts
                .fill_channel("{{0:A}}", None, 99, &mut budget)
                .unwrap(),
            filled("{{0:A}}", unresolved(UnavailableReason::NoReference))
        );
        assert_eq!(
            texts
                .fill_object("{{0:N}}", "O-2_R-1", None, &mut budget)
                .unwrap(),
            filled("{{0:N}}", unresolved(UnavailableReason::NotTextParameter))
        );
        assert_eq!(
            texts.value(Some("R-missing"), TextParameterScope::Program),
            PositionalValue::Unavailable(UnavailableReason::UnknownParameter)
        );
    }

    #[test]
    fn a_module_channel_reads_its_own_module_and_refuses_an_ambiguous_one() {
        let parameters = parameters();
        let stored = evaluation();
        let one = [instance("MD-1_M-1", "MD-1_M-1_MI-2")];
        let texts = DeviceTexts {
            parameters: &parameters,
            evaluation: &stored,
            module_instances: &one,
        };
        let mut budget = TextProjectionBudget::default();
        assert_eq!(
            texts
                .fill_channel(
                    "{{0:Button}}",
                    Some(&scope(Some("A_MD-1_M-1"))),
                    2,
                    &mut budget
                )
                .unwrap(),
            filled("Down", PositionalSource::StoredValue)
        );
        // Another instantiation of the module stores nothing: its product default.
        assert_eq!(
            texts
                .fill_channel("{{0:B}}", Some(&scope(Some("A_MD-1_M-2"))), 2, &mut budget)
                .unwrap(),
            filled("Button", PositionalSource::ProductDefault)
        );
        assert_eq!(
            texts
                .fill_channel("{{0:B}}", Some(&scope(None)), 2, &mut budget)
                .unwrap(),
            filled(
                "{{0:B}}",
                PositionalSource::Unresolved(UnavailableReason::Undecided)
            )
        );
        let two = [
            instance("MD-1_M-1", "MD-1_M-1_MI-1"),
            instance("MD-1_M-1", "MD-1_M-1_MI-2"),
        ];
        let texts = DeviceTexts {
            module_instances: &two,
            ..texts
        };
        assert_eq!(
            texts
                .fill_channel("{{0:B}}", Some(&scope(Some("A_MD-1_M-1"))), 2, &mut budget)
                .unwrap(),
            filled(
                "{{0:B}}",
                PositionalSource::Unresolved(UnavailableReason::Undecided)
            )
        );
    }

    #[test]
    fn a_module_object_reads_its_own_instance_never_a_sibling() {
        let parameters = parameters();
        let stored = evaluation();
        let texts = DeviceTexts {
            parameters: &parameters,
            evaluation: &stored,
            module_instances: &[],
        };
        let mut budget = TextProjectionBudget::default();
        let mine = instance("MD-1_M-1", "MD-1_M-1_MI-2");
        let sibling = instance("MD-1_M-1", "MD-1_M-1_MI-1");
        assert_eq!(
            texts
                .fill_object("{{0:B}}", "MD-1_O-1_R-1", Some(&mine), &mut budget)
                .unwrap(),
            filled("Up", PositionalSource::StoredValue)
        );
        assert_eq!(
            texts
                .fill_object("{{0:B}}", "MD-1_O-1_R-1", Some(&sibling), &mut budget)
                .unwrap(),
            filled("Button", PositionalSource::ProductDefault)
        );
    }
}
