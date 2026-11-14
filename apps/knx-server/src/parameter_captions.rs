//! Projects parameter captions and literal grid axes with one device text budget.
use std::collections::HashMap;

use knx_productdb::device_evaluation::DeviceEvaluation;
use knx_productdb::dynamic::TextProjectionBudget;
use knx_productdb::text_parameters::{DeviceTexts, TextParameters};
use knx_projection::TextFill;

pub(crate) type CaptionKey = (Option<Vec<i64>>, i64);
/// One request's resolver and DTO-copy admission context. Never reset per label or section.
#[derive(Default)]
pub(crate) struct CaptionFills {
    fills: HashMap<CaptionKey, TextFill>,
    budget: TextProjectionBudget,
    axes: knx_productdb::query::GridAxisCaptions,
}

impl CaptionFills {
    pub(crate) fn load_grid_axes(
        &mut self,
        products: &knx_productdb::Connection,
        program: &str,
        layout: &knx_productdb::parameter_layout::ParameterLayout,
        language: Option<&str>,
    ) -> Result<(), String> {
        self.axes = knx_productdb::query::parameter_grid_axis_captions(
            products,
            program,
            layout,
            language,
            &mut self.budget,
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub(crate) fn copy_axis_text(
        &mut self,
        definition: &str,
        axis: &knx_productdb::parameter_layout::GridAxis,
    ) -> Result<Option<String>, String> {
        let text = definition
            .is_empty()
            .then(|| self.axes.texts.get(&axis.id))
            .flatten()
            .map(String::as_str)
            .or(axis.text.as_deref());
        text.map(|text| self.budget.copy_text(text))
            .transpose()
            .map_err(|e| e.to_string())
    }

    pub(crate) fn axis_unsupported(&self, definition: &str, id: &str) -> bool {
        definition.is_empty() && self.axes.unsupported.contains(id)
    }

    pub(crate) fn copy_text(&mut self, text: Option<&str>) -> Result<Option<String>, String> {
        text.map(|text| self.budget.copy_text(text))
            .transpose()
            .map_err(|e| e.to_string())
    }

    /// Admit both the rendered text and retained template before allocating a DTO copy.
    pub(crate) fn copy(
        &mut self,
        key: CaptionKey,
        text: Option<&str>,
    ) -> Result<(Option<String>, Option<TextFill>), String> {
        let fill = self.fills.get(&key);
        if let Some(text) = text {
            self.budget.admit_copy(text).map_err(|e| e.to_string())?;
        }
        if let Some(fill) = fill {
            self.budget
                .admit_copy(&fill.template)
                .map_err(|e| e.to_string())?;
        }
        Ok((text.map(str::to_owned), fill.cloned()))
    }
}

/// Overlay this request's evaluated labels only; source metadata and project values stay untouched.
pub(crate) fn apply(
    products: &knx_productdb::Connection,
    program: &str,
    evaluation: &mut DeviceEvaluation,
    module_instances: &[knx_core::ModuleInstance],
    language: Option<&str>,
) -> Result<CaptionFills, String> {
    let mut budget = TextProjectionBudget::default();
    let translated = language
        .map(|language| {
            knx_productdb::query::parameter_caption_templates(
                products,
                program,
                language,
                &mut budget,
            )
        })
        .transpose()
        .map_err(|e| e.to_string())?
        .unwrap_or_default();
    let parameters = TextParameters::load(products, program).map_err(|e| e.to_string())?;
    let texts = DeviceTexts {
        parameters: &parameters,
        evaluation,
        module_instances,
    };
    let mut filled = Vec::new();
    let mut refused = Vec::new();
    // At most one active replacement per block (the evaluator refuses competing ones).
    let renames: HashMap<i64, &knx_productdb::dynamic::ActiveRename> = evaluation
        .activation
        .renames
        .iter()
        .map(|rename| (rename.target_node_id, rename))
        .collect();
    for (index, label) in evaluation.activation.labels.iter().enumerate() {
        if !matches!(label.kind.as_str(), "ParameterBlock" | "ParameterSeparator") {
            continue;
        }
        // The replacement is translated by its own identity, never by the block's: the base
        // block translation and its positional fill are not consulted while one is active.
        if let Some(rename) = renames
            .get(&label.node_id)
            .filter(|_| label.kind == "ParameterBlock" && label.scope.is_none())
        {
            // A selected translation with a placeholder is not filled: positional values
            // belong to the base caption's own text parameters. The source text is shown
            // instead and the refusal is named against the replacement's own node.
            let selected = translated.get(&(String::new(), rename.node_id));
            let unsupported = selected.and_then(|text| placeholder_name(text));
            let text = match (selected, unsupported) {
                (Some(text), None) => budget.copy_text(text),
                _ => budget.copy_text(&rename.text),
            }
            .map_err(|e| e.to_string())?;
            if let Some(name) = unsupported {
                refused.push((
                    rename.node_id,
                    budget.copy_text(name).map_err(|e| e.to_string())?,
                ));
            }
            filled.push((index, text, None));
            continue;
        }
        let tree = label
            .scope
            .as_deref()
            .map_or("", |scope| scope.module_def_id.as_str());
        let template = translated.get(&(tree.to_owned(), label.node_id));
        let translated_text = template
            .map(|text| {
                knx_productdb::dynamic::substitute_text_checked(
                    text,
                    label.scope.as_deref(),
                    &mut budget,
                )
            })
            .transpose()
            .map_err(|e| e.to_string())?;
        let text = translated_text.as_deref().unwrap_or(&label.text);
        if !text.contains("{{") {
            if let Some(text) = translated_text {
                filled.push((index, text, None));
            }
            continue;
        }
        // fill_channel indexes all dynamic hosts by exact (definition, node),
        // not by caption or by a suffix. Module arguments have already been filled.
        let value = texts
            .fill_channel(text, label.scope.as_deref(), label.node_id, &mut budget)
            .map_err(|e| e.to_string())?;
        if value.source != knx_productdb::dynamic::PositionalSource::NoPlaceholder {
            // The provenance template is a newly retained output, too.
            budget.admit_copy(text).map_err(|e| e.to_string())?;
        }
        let fill = crate::com_object_activation::text_fill(value.source, text);
        if fill.is_some() || translated_text.is_some() {
            filled.push((index, value.text, fill));
        }
    }
    let mut fills = HashMap::new();
    for (index, text, fill) in filled {
        let label = &mut evaluation.activation.labels[index];
        if let Some(fill) = fill {
            fills.insert(
                (label.scope.as_ref().map(|s| s.node_chain()), label.node_id),
                fill,
            );
        }
        label.text = text;
    }
    // Panel diagnostics list every activation diagnostic; this one hides no reference.
    for (node_id, name) in refused {
        evaluation
            .activation
            .diagnostics
            .push(knx_productdb::dynamic::ScopedDiagnostic {
                scope: None,
                diagnostic: knx_productdb::dynamic::Diagnostic::UnresolvedTextPlaceholder {
                    node_id,
                    name,
                },
            });
    }
    Ok(CaptionFills {
        fills,
        budget,
        axes: Default::default(),
    })
}

/// The inside of the first placeholder in `text`, or everything after an unclosed opener.
fn placeholder_name(text: &str) -> Option<&str> {
    let inside = &text[text.find("{{")? + 2..];
    Some(inside.find("}}").map_or(inside, |close| &inside[..close]))
}
