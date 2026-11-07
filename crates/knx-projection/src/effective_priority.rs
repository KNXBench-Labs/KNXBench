//! Read-only effective object priority with its winning source; never a wire priority.

use serde::Serialize;
use ts_rs::TS;

use crate::{
    ComObjectNode, SavedInstancePriorityView, SavedPriorityOmissionView,
    SavedPriorityTokenStatusView,
};

/// Project-effective priority (ADR-0158 §3, contract `effective-object-priority.md`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "status", rename_all = "lowercase")]
#[ts(export)]
pub enum EffectivePriorityView {
    Resolved {
        value: EffectivePriorityToken,
        source: EffectivePrioritySource,
    },
    /// `source`/`raw` are set exactly when a winning literal was selected.
    Unresolved {
        reason: EffectivePriorityUnresolved,
        source: Option<EffectivePrioritySource>,
        raw: Option<String>,
    },
}

impl Default for EffectivePriorityView {
    /// Matches the saved view's default (`NotCaptured`): nothing known, nothing inherited.
    fn default() -> Self {
        unresolved(EffectivePriorityUnresolved::InstanceUnavailable)
    }
}

/// The three `ComObjectPriority_t` tokens (Project Schema23 §1.1.2.4); no bit pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub enum EffectivePriorityToken {
    Low,
    High,
    Alert,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "kind", rename_all = "lowercase")]
#[ts(export)]
pub enum EffectivePrioritySource {
    /// The user's own edit (`ComObjectInstance.priority_edit`); wins over
    /// every captured or product value (contract `object-priority-edit.md`).
    #[serde(rename = "userEdit")]
    UserEdit,
    /// The saved object instance's own declaration.
    Instance,
    /// The product literal, applied because the saved instance omitted Priority.
    Product {
        layer: ProductPriorityLayer,
        omission: SavedPriorityOmissionView,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub enum ProductPriorityLayer {
    Program,
    ProgramRef,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub enum EffectivePriorityUnresolved {
    InstanceUnavailable,
    InstanceEmpty,
    InstanceUnknown,
    ProductNotConsulted,
    NoProgramReference,
    NoProductDatabase,
    ProgramNotInDatabase,
    ProductObjectNotFound,
    ProductLiteralAbsent,
    ProductEmpty,
    ProductUnknown,
}

/// Why product data could not be consulted for an omitted declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProductPriorityUnavailable {
    NoProgramReference,
    NoProductDatabase,
    ProgramNotInDatabase,
    ProductObjectNotFound,
}

fn unresolved(reason: EffectivePriorityUnresolved) -> EffectivePriorityView {
    EffectivePriorityView::Unresolved {
        reason,
        source: None,
        raw: None,
    }
}

/// Exact token match, the same rule as `SavedPriorityTokenStatus::classify`.
fn token(raw: &str) -> Option<EffectivePriorityToken> {
    match raw {
        "Low" => Some(EffectivePriorityToken::Low),
        "High" => Some(EffectivePriorityToken::High),
        "Alert" => Some(EffectivePriorityToken::Alert),
        _ => None,
    }
}

/// The user's edit when present, else the already binding-checked saved view;
/// omissions await product data.
pub(crate) fn resolve(
    edit: Option<knx_core::ObjectPriority>,
    saved: &SavedInstancePriorityView,
) -> EffectivePriorityView {
    match edit {
        Some(edit) => EffectivePriorityView::Resolved {
            value: match edit {
                knx_core::ObjectPriority::Low => EffectivePriorityToken::Low,
                knx_core::ObjectPriority::High => EffectivePriorityToken::High,
                knx_core::ObjectPriority::Alert => EffectivePriorityToken::Alert,
            },
            source: EffectivePrioritySource::UserEdit,
        },
        None => from_saved(saved),
    }
}

/// Effective priority from the already binding-checked saved view; omissions await product data.
fn from_saved(saved: &SavedInstancePriorityView) -> EffectivePriorityView {
    use EffectivePriorityUnresolved as R;
    match saved {
        SavedInstancePriorityView::Unavailable { .. } => unresolved(R::InstanceUnavailable),
        SavedInstancePriorityView::Absent { .. } => unresolved(R::ProductNotConsulted),
        SavedInstancePriorityView::Declared {
            raw, token_status, ..
        } => match (token_status, token(raw)) {
            (SavedPriorityTokenStatusView::Known, Some(value)) => EffectivePriorityView::Resolved {
                value,
                source: EffectivePrioritySource::Instance,
            },
            (status, _) => EffectivePriorityView::Unresolved {
                reason: if *status == SavedPriorityTokenStatusView::Empty {
                    R::InstanceEmpty
                } else {
                    R::InstanceUnknown
                },
                source: Some(EffectivePrioritySource::Instance),
                raw: Some(raw.clone()),
            },
        },
    }
}

fn awaiting_product(com: &ComObjectNode) -> Option<SavedPriorityOmissionView> {
    let waiting = matches!(
        com.effective_priority,
        EffectivePriorityView::Unresolved {
            reason: EffectivePriorityUnresolved::ProductNotConsulted,
            ..
        }
    );
    match &com.saved_instance_priority {
        SavedInstancePriorityView::Absent { omission, .. } if waiting => Some(*omission),
        _ => None,
    }
}

/// Marks every object still awaiting product data with why none could be consulted.
pub fn mark_product_priority_unavailable(
    objects: &mut [ComObjectNode],
    why: ProductPriorityUnavailable,
) {
    use EffectivePriorityUnresolved as R;
    let reason = match why {
        ProductPriorityUnavailable::NoProgramReference => R::NoProgramReference,
        ProductPriorityUnavailable::NoProductDatabase => R::NoProductDatabase,
        ProductPriorityUnavailable::ProgramNotInDatabase => R::ProgramNotInDatabase,
        ProductPriorityUnavailable::ProductObjectNotFound => R::ProductObjectNotFound,
    };
    for com in objects {
        if awaiting_product(com).is_some() {
            com.effective_priority = unresolved(reason);
        }
    }
}

/// Applies the product's selected literal (`None` = neither layer states one) to one object.
///
/// Only an object whose saved instance omitted Priority and that still awaits product
/// data changes; a saved declaration, even an unresolved one, is never replaced.
pub fn apply_product_priority(
    com: &mut ComObjectNode,
    selected: Option<(&str, ProductPriorityLayer)>,
) {
    use EffectivePriorityUnresolved as R;
    let Some(omission) = awaiting_product(com) else {
        return;
    };
    com.effective_priority = match selected {
        None => unresolved(R::ProductLiteralAbsent),
        Some((raw, layer)) => {
            let source = EffectivePrioritySource::Product { layer, omission };
            match token(raw) {
                Some(value) => EffectivePriorityView::Resolved { value, source },
                None => EffectivePriorityView::Unresolved {
                    reason: if raw.is_empty() {
                        R::ProductEmpty
                    } else {
                        R::ProductUnknown
                    },
                    source: Some(source),
                    raw: Some(raw.to_owned()),
                },
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CapturedSavedInstanceSource, SavedPrioritySourceKind, SavedPrioritySourceRef};
    use EffectivePriorityUnresolved as R;

    fn source() -> CapturedSavedInstanceSource {
        let r = SavedPrioritySourceRef {
            path: "P-1/0.xml".into(),
            ets_id: "O-1_R-1".into(),
        };
        CapturedSavedInstanceSource {
            kind: SavedPrioritySourceKind::CapturedSavedInstance,
            source_schema: 23,
            device_source: r.clone(),
            object_source: r,
            owner_xpath: "/x".into(),
            topology_sha256: "a".repeat(64),
        }
    }

    fn declared(raw: &str) -> SavedInstancePriorityView {
        let token_status = match knx_core::SavedPriorityTokenStatus::classify(raw) {
            knx_core::SavedPriorityTokenStatus::Known => SavedPriorityTokenStatusView::Known,
            knx_core::SavedPriorityTokenStatus::Empty => SavedPriorityTokenStatusView::Empty,
            knx_core::SavedPriorityTokenStatus::Unknown => SavedPriorityTokenStatusView::Unknown,
        };
        SavedInstancePriorityView::Declared {
            raw: raw.into(),
            token_status,
            source: source(),
        }
    }

    fn absent(omission: SavedPriorityOmissionView) -> SavedInstancePriorityView {
        SavedInstancePriorityView::Absent {
            omission,
            source: source(),
        }
    }

    fn node(saved: SavedInstancePriorityView) -> ComObjectNode {
        let effective_priority = from_saved(&saved);
        ComObjectNode {
            saved_instance_priority: saved,
            effective_priority,
            id: 1,
            number: 1,
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
            activation: crate::ComObjectActivation::NotEvaluated,
            channel: None,
            program_dpt: None,
            dpt_text: None,
            dpt_text_language: None,
            size: None,
            function_text: None,
            name_fill: None,
            project_folder: None,
        }
    }

    fn rejected(reason: R, raw: &str, source: EffectivePrioritySource) -> EffectivePriorityView {
        EffectivePriorityView::Unresolved {
            reason,
            source: Some(source),
            raw: Some(raw.into()),
        }
    }

    #[test]
    fn instance_declarations_resolve_only_for_exact_tokens() {
        use EffectivePriorityToken as T;
        for (raw, value) in [("Low", T::Low), ("High", T::High), ("Alert", T::Alert)] {
            assert_eq!(
                from_saved(&declared(raw)),
                EffectivePriorityView::Resolved {
                    value,
                    source: EffectivePrioritySource::Instance
                }
            );
        }
        let instance = EffectivePrioritySource::Instance;
        assert_eq!(
            from_saved(&declared("")),
            rejected(R::InstanceEmpty, "", instance)
        );
        for raw in [" ", " Low", "High ", "\tAlert", "low", "ALERT", "Höchste"] {
            assert_eq!(
                from_saved(&declared(raw)),
                rejected(R::InstanceUnknown, raw, instance)
            );
        }
    }

    #[test]
    fn unavailable_saved_facts_never_inherit() {
        for reason in [
            crate::SavedPriorityUnavailableView::NotCaptured,
            crate::SavedPriorityUnavailableView::ModuleScope,
            crate::SavedPriorityUnavailableView::SourceBindingMismatch,
        ] {
            let mut com = node(SavedInstancePriorityView::Unavailable { reason });
            assert_eq!(com.effective_priority, unresolved(R::InstanceUnavailable));
            apply_product_priority(&mut com, Some(("Low", ProductPriorityLayer::Program)));
            mark_product_priority_unavailable(
                std::slice::from_mut(&mut com),
                ProductPriorityUnavailable::NoProductDatabase,
            );
            assert_eq!(com.effective_priority, unresolved(R::InstanceUnavailable));
        }
        assert_eq!(
            EffectivePriorityView::default(),
            unresolved(R::InstanceUnavailable)
        );
    }

    #[test]
    fn a_user_edit_wins_over_every_saved_state_and_every_product_literal() {
        use knx_core::ObjectPriority as P;
        use EffectivePriorityToken as T;
        let saved = [
            declared("High"),
            declared(""),
            declared("bogus"),
            absent(SavedPriorityOmissionView::AttributeOmitted),
            SavedInstancePriorityView::Unavailable {
                reason: crate::SavedPriorityUnavailableView::NotCaptured,
            },
        ];
        for state in saved {
            for (edit, value) in [(P::Low, T::Low), (P::High, T::High), (P::Alert, T::Alert)] {
                let wanted = EffectivePriorityView::Resolved {
                    value,
                    source: EffectivePrioritySource::UserEdit,
                };
                let mut com = node(state.clone());
                com.effective_priority = resolve(Some(edit), &state);
                assert_eq!(com.effective_priority, wanted);
                apply_product_priority(&mut com, Some(("Alert", ProductPriorityLayer::Program)));
                mark_product_priority_unavailable(
                    std::slice::from_mut(&mut com),
                    ProductPriorityUnavailable::NoProductDatabase,
                );
                assert_eq!(com.effective_priority, wanted, "{state:?}");
            }
            assert_eq!(resolve(None, &state), from_saved(&state));
        }
    }

    #[test]
    fn declared_values_survive_any_product_literal() {
        for raw in ["High", "", "bogus"] {
            let mut com = node(declared(raw));
            let before = com.effective_priority.clone();
            apply_product_priority(&mut com, Some(("Alert", ProductPriorityLayer::ProgramRef)));
            apply_product_priority(&mut com, None);
            mark_product_priority_unavailable(
                std::slice::from_mut(&mut com),
                ProductPriorityUnavailable::ProgramNotInDatabase,
            );
            assert_eq!(com.effective_priority, before, "raw {raw:?}");
        }
    }

    #[test]
    fn omissions_take_the_product_literal_with_layer_and_omission() {
        use SavedPriorityOmissionView as O;
        for omission in [O::AttributeOmitted, O::InstanceElementOmitted] {
            for layer in [
                ProductPriorityLayer::Program,
                ProductPriorityLayer::ProgramRef,
            ] {
                let source = EffectivePrioritySource::Product { layer, omission };
                let mut com = node(absent(omission));
                assert_eq!(com.effective_priority, unresolved(R::ProductNotConsulted));
                apply_product_priority(&mut com, Some(("High", layer)));
                assert_eq!(
                    com.effective_priority,
                    EffectivePriorityView::Resolved {
                        value: EffectivePriorityToken::High,
                        source
                    }
                );
                let mut com = node(absent(omission));
                apply_product_priority(&mut com, Some(("", layer)));
                assert_eq!(
                    com.effective_priority,
                    rejected(R::ProductEmpty, "", source)
                );
                // Exact match: padding that trimming would turn into a token stays unknown.
                for raw in [" high", " Low", "High ", "\tAlert", "Alert\n", "LOW"] {
                    let mut com = node(absent(omission));
                    apply_product_priority(&mut com, Some((raw, layer)));
                    assert_eq!(
                        com.effective_priority,
                        rejected(R::ProductUnknown, raw, source)
                    );
                }
            }
            let mut com = node(absent(omission));
            apply_product_priority(&mut com, None);
            assert_eq!(com.effective_priority, unresolved(R::ProductLiteralAbsent));
        }
    }

    #[test]
    fn unavailable_product_data_names_its_reason_once() {
        use ProductPriorityUnavailable as U;
        for (why, reason) in [
            (U::NoProgramReference, R::NoProgramReference),
            (U::NoProductDatabase, R::NoProductDatabase),
            (U::ProgramNotInDatabase, R::ProgramNotInDatabase),
            (U::ProductObjectNotFound, R::ProductObjectNotFound),
        ] {
            let mut objects = vec![
                node(absent(SavedPriorityOmissionView::AttributeOmitted)),
                node(declared("Low")),
            ];
            mark_product_priority_unavailable(&mut objects, why);
            assert_eq!(objects[0].effective_priority, unresolved(reason));
            assert!(matches!(
                objects[1].effective_priority,
                EffectivePriorityView::Resolved { .. }
            ));
            // A later product literal cannot overwrite an already explained gap.
            apply_product_priority(
                &mut objects[0],
                Some(("Low", ProductPriorityLayer::Program)),
            );
            assert_eq!(objects[0].effective_priority, unresolved(reason));
        }
    }
}
