//! Projects parameter captions and literal grid axes with one device text budget.
use std::collections::HashMap;

use knx_productdb::device_evaluation::DeviceEvaluation;
use knx_productdb::dynamic::{TextProjectionBudget, TextProjectionError};
use knx_productdb::text_parameters::{DeviceTexts, TextParameters};
use knx_projection::TextFill;

pub(crate) type CaptionKey = (Option<Vec<i64>>, i64);
/// One request's resolver and DTO-copy admission context. Never reset per label or section.
#[derive(Debug, Default)]
pub(crate) struct CaptionFills {
    fills: HashMap<CaptionKey, TextFill>,
    budget: TextProjectionBudget,
    /// WP-05n: per-definition overlays; one section's axes never read another
    /// tree's caption.
    axes: HashMap<String, knx_productdb::query::GridAxisCaptions>,
    /// ADR-0177: resolved parameter-driven axis captions, keyed by axis `Id`.
    axis_texts: HashMap<String, (String, knx_projection::TextFill)>,
}

impl CaptionFills {
    /// The shared text-projection budget, for callers that fill one more
    /// positional text through [`DeviceTexts`] (WP-05 button text refs).
    pub(crate) fn budget_mut(&mut self) -> &mut TextProjectionBudget {
        &mut self.budget
    }

    /// Charge one already-filled text against the shared budget.
    pub(crate) fn admit_copy(&mut self, text: &str) -> Result<(), TextProjectionError> {
        self.budget.admit_copy(text)
    }

    pub(crate) fn load_grid_axes(
        &mut self,
        products: &knx_productdb::Connection,
        program: &str,
        layout: &knx_productdb::parameter_layout::ParameterLayout,
        definition: &str,
        language: Option<&str>,
    ) -> Result<(), String> {
        let axes = knx_productdb::query::parameter_grid_axis_captions(
            products,
            program,
            layout,
            definition,
            language,
            &mut self.budget,
        )
        .map_err(|e| e.to_string())?;
        self.axes.insert(definition.to_owned(), axes);
        Ok(())
    }

    /// ADR-0177: resolve every declared axis's `TextParameterRefId` in the
    /// program scope through the existing positional resolver and retain the
    /// fill provenance per axis `Id`, charged to the same sticky budget.
    pub(crate) fn load_axis_texts(
        &mut self,
        layout: &knx_productdb::parameter_layout::ParameterLayout,
        program: &str,
        texts: &knx_productdb::text_parameters::DeviceTexts<'_>,
    ) -> Result<(), String> {
        // WP-05n: the program tree's bound axes, as before; module trees are
        // resolved by their own definitions in `load_axis_texts_module`.
        let Some(axes) = layout.literal_grid_axes(program, "") else {
            return Ok(());
        };
        let bound: std::collections::HashSet<&str> = axes.collect();
        for grid in layout.program_grids() {
            for axis in grid.rows.iter().chain(grid.columns.iter()) {
                let Some(reference) = axis.text_parameter_ref_id.as_deref() else {
                    continue;
                };
                // ADR-0177: the reference must sit on an axis this tree
                // actually admitted as a caption host; WP-05n makes that
                // admission scope-local, so the lookup below consults only
                // the program tree's own bound axes (a module-scope axis id
                // must never satisfy the program tree's reference check).
                if !bound.contains(axis.id.as_str()) {
                    continue;
                }
                let value = texts.value(
                    Some(reference),
                    knx_productdb::text_parameters::TextParameterScope::Program,
                );
                // ADR-0177 / support.knx.org 360000160080: a declared
                // `TextParameterRefId` replaces the axis caption with the
                // parameter's value; the census shows axis texts carry no
                // `{{0}}` placeholder, so a placeholder-less literal text is
                // wrapped to make the replacement explicit. A literal text
                // that does carry placeholders keeps template semantics.
                let template = match axis.text.as_deref() {
                    Some(text) if text.contains("{{") => text.to_owned(),
                    _ => "{{0}}".to_owned(),
                };
                let filled =
                    knx_productdb::dynamic::fill_positional(&template, value, &mut self.budget)
                        .map_err(|e| e.to_string())?;
                self.budget
                    .admit_copy(&template)
                    .map_err(|e| e.to_string())?;
                if filled.source != knx_productdb::dynamic::PositionalSource::NoPlaceholder {
                    // Retain the resolved caption text itself as the fill's
                    // payload, plus its provenance template.
                    self.budget
                        .admit_copy(&filled.text)
                        .map_err(|e| e.to_string())?;
                }
                if let Some(fill) =
                    crate::com_object_activation::text_fill(filled.source, &template)
                {
                    self.budget
                        .admit_copy(&fill.template)
                        .map_err(|e| e.to_string())?;
                    self.axis_texts.insert(axis.id.clone(), (filled.text, fill));
                }
            }
        }
        Ok(())
    }

    /// ADR-0177: the resolved axis captions collected by `load_axis_texts`.
    #[cfg(test)]
    #[allow(dead_code)]
    pub(crate) fn axis_texts(&self) -> HashMap<String, (String, TextFill)> {
        self.axis_texts
            .iter()
            .map(|(id, entry)| (id.clone(), entry.clone()))
            .collect()
    }

    pub(crate) fn copy_axis_text(
        &mut self,
        definition: &str,
        axis: &knx_productdb::parameter_layout::GridAxis,
    ) -> Result<(Option<String>, Option<knx_projection::TextFill>), String> {
        // WP-05n: any definition's bound literal axes translate, keyed by
        // `(module_def_id, axis id)`; the ADR-0177 parameter-driven overlay
        // below stays program-tree-only.
        let translated = self
            .axes
            .get(definition)
            .and_then(|axes| axes.texts.get(&(definition.to_owned(), axis.id.clone())))
            .map(String::as_str);
        let base = translated.or(axis.text.as_deref());
        // ADR-0177: a program-owned literal axis whose declared
        // `TextParameterRefId` resolved shows the parameter-driven caption and
        // its fill provenance; unresolved references keep the axis's own text.
        let (text, fill) = if definition.is_empty() {
            if let Some((resolved, fill)) = self.axis_texts.get(&axis.id) {
                let shown = match fill.source {
                    knx_projection::TextFillSource::StoredValue
                    | knx_projection::TextFillSource::ProductDefault => Some(resolved.clone()),
                    knx_projection::TextFillSource::TemplateDefault
                    | knx_projection::TextFillSource::EmptyValue => base.map(str::to_owned),
                    _ => base.map(str::to_owned),
                };
                (shown, Some(fill.clone()))
            } else {
                (base.map(str::to_owned), None)
            }
        } else {
            (base.map(str::to_owned), None)
        };
        let text = text
            .map(|text| self.budget.copy_text(&text))
            .transpose()
            .map_err(|e| e.to_string())?;
        if let Some(fill) = &fill {
            self.budget
                .admit_copy(&fill.template)
                .map_err(|e| e.to_string())?;
        }
        Ok((text, fill))
    }

    /// WP-05n: whether `definition`'s axis `id` has a selected translation that
    /// is unsupported (a placeholder template); any definition's bound axes.
    pub(crate) fn axis_unsupported(&self, definition: &str, id: &str) -> bool {
        self.axes.get(definition).is_some_and(|axes| {
            axes.unsupported
                .contains(&(definition.to_owned(), id.to_owned()))
        })
    }

    /// The requested language's own `Text` translation for a defining tree/node,
    /// if one was admitted; the caller decides the fallback.
    pub(crate) fn translated_text(&self, key: CaptionKey) -> Option<String> {
        self.fills.get(&key).map(|fill| fill.template.clone())
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
    // At most one active replacement per (tree, block) — the evaluator refuses competing
    // ones. The tree key is the rename's own module scope chain, matching the label's scope.
    let tree_key = |scope: &Option<std::rc::Rc<knx_productdb::dynamic::ModuleScope>>| {
        scope
            .as_ref()
            .map_or(String::new(), |s| s.module_def_id.clone())
    };
    let renames: HashMap<(String, i64), &knx_productdb::dynamic::ActiveRename> = evaluation
        .activation
        .renames
        .iter()
        .map(|rename| {
            let tree = rename
                .scope
                .as_ref()
                .and_then(|c| c.last())
                .map_or(String::new(), |s| s.module_def_id.clone());
            ((tree, rename.target_node_id), rename)
        })
        .collect();
    for (index, label) in evaluation.activation.labels.iter().enumerate() {
        if !matches!(
            label.kind.as_str(),
            "ParameterBlock"
                | "ParameterSeparator"
                | "Channel"
                | "ChannelIndependentBlock"
                | "Button"
        ) {
            continue;
        }
        // The replacement is translated by its own identity, never by the host's: the base
        // host translation and its positional fill are not consulted while one is active.
        // A rename may address any labelled host of the same tree — a `ParameterBlock`
        // (enclosing or cross-target), a channel owner (`Channel`/
        // `ChannelIndependentBlock`), or since WP-05k a `ParameterSeparator`
        // (WP-05 cross-target renames).
        if let Some(rename) = renames
            .get(&(tree_key(&label.scope), label.node_id))
            .filter(|_| {
                matches!(
                    label.kind.as_str(),
                    "ParameterBlock" | "Channel" | "ChannelIndependentBlock" | "ParameterSeparator"
                )
            })
        {
            // A selected translation with a placeholder is not filled: positional values
            // belong to the base caption's own text parameters. The source text is shown
            // instead and the refusal is named against the replacement's own node — unless
            // the leaf itself names a text parameter (WP-05 `rename-text-parameter-ref-v1`),
            // whose value fills the replacement's placeholder.
            let rename_tree = rename
                .scope
                .as_ref()
                .and_then(|c| c.last())
                .map_or(String::new(), |s| s.module_def_id.clone());
            let selected = translated.get(&(rename_tree, rename.node_id));
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
            let text = if rename.text.contains("{{") && rename.text_parameter_ref.is_some() {
                let place = match rename.scope.as_ref().and_then(|c| c.last()) {
                    None => knx_productdb::text_parameters::TextParameterScope::Program,
                    Some(s) => knx_productdb::text_parameters::TextParameterScope::Module(
                        s.module_id.as_deref(),
                    ),
                };
                let value = texts
                    .fill_rename_scoped(
                        &text,
                        place,
                        rename.node_id,
                        rename.text_parameter_ref.as_deref(),
                        &mut budget,
                    )
                    .map_err(|e| e.to_string())?;
                if value.source != knx_productdb::dynamic::PositionalSource::NoPlaceholder {
                    budget.admit_copy(&text).map_err(|e| e.to_string())?;
                }
                value.text
            } else {
                text
            };
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
            // A plain (placeholder-free) translation is retained as the label's
            // overlay fill, so other readers (e.g. button labels) can pick it up.
            if let Some(text) = translated_text {
                // NoPlaceholder maps to None in text_fill, so build the fill
                // directly: the template is the translated text itself.
                let fill = knx_projection::TextFill {
                    template: text.clone(),
                    source: knx_projection::TextFillSource::StoredValue,
                };
                filled.push((index, text, Some(fill)));
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
        axis_texts: Default::default(),
    })
}

/// The inside of the first placeholder in `text`, or everything after an unclosed opener.
fn placeholder_name(text: &str) -> Option<&str> {
    let inside = &text[text.find("{{")? + 2..];
    Some(inside.find("}}").map_or(inside, |close| &inside[..close]))
}
