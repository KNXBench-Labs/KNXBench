//! Fills parameter block and separator captions through the existing device text resolver.
use std::collections::HashMap;

use knx_productdb::device_evaluation::DeviceEvaluation;
use knx_productdb::dynamic::TextProjectionBudget;
use knx_productdb::text_parameters::{DeviceTexts, TextParameters};
use knx_projection::TextFill;

pub(crate) type CaptionKey = (Option<Vec<i64>>, i64);
pub(crate) type CaptionFills = HashMap<CaptionKey, TextFill>;

/// Overlay this request's evaluated labels only; source metadata and project values stay untouched.
pub(crate) fn apply(
    products: &knx_productdb::Connection,
    program: &str,
    evaluation: &mut DeviceEvaluation,
    module_instances: &[knx_core::ModuleInstance],
) -> Result<CaptionFills, String> {
    let parameters = TextParameters::load(products, program).map_err(|e| e.to_string())?;
    let texts = DeviceTexts {
        parameters: &parameters,
        evaluation,
        module_instances,
    };
    let mut budget = TextProjectionBudget::default();
    let mut filled = Vec::new();
    for (index, label) in evaluation.activation.labels.iter().enumerate() {
        if !matches!(label.kind.as_str(), "ParameterBlock" | "ParameterSeparator")
            || !label.text.contains("{{")
        {
            continue;
        }
        // fill_channel indexes all dynamic hosts by exact (definition, node),
        // not by caption or by a suffix. Module arguments have already been filled.
        let value = texts
            .fill_channel(
                &label.text,
                label.scope.as_deref(),
                label.node_id,
                &mut budget,
            )
            .map_err(|e| e.to_string())?;
        if let Some(fill) = crate::com_object_activation::text_fill(value.source, &label.text) {
            filled.push((index, value.text, fill));
        }
    }
    let mut fills = HashMap::new();
    for (index, text, fill) in filled {
        let label = &mut evaluation.activation.labels[index];
        fills.insert(
            (label.scope.as_ref().map(|s| s.node_chain()), label.node_id),
            fill,
        );
        label.text = text;
    }
    Ok(fills)
}
