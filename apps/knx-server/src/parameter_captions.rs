//! Fills parameter block and separator captions through the existing device text resolver.
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
}

impl CaptionFills {
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
    for (index, label) in evaluation.activation.labels.iter().enumerate() {
        if !matches!(label.kind.as_str(), "ParameterBlock" | "ParameterSeparator") {
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
    Ok(CaptionFills { fills, budget })
}
