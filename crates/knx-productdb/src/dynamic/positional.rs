//! Positional `{{0}}` placeholders filled from a host's text parameter (ADR-0126).
//!
//! The Manufacturer Tool documentation defines the placeholder: ETS fills
//! `{{0}}` or `{{0:def}}` with the value of the text parameter the host's
//! `TextParameterRefId` names, and shows `def` when that value is empty. This
//! module is that rule and nothing more: index `0` only, the value as given, and
//! every other brace left exactly where it was. Module arguments are substituted
//! before this runs (`substitute_text_checked`), including inside a default.

use super::evaluate::{TextProjectionBudget, TextProjectionError};

const OPEN: &str = "{{";
const CLOSE: &str = "}}";

/// The text parameter's value on one device, or why there is none to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PositionalValue<'a> {
    /// The device stores this value (possibly empty, which counts as empty).
    Stored(&'a str),
    /// The device stores none; this is the product's default value.
    ProductDefault(&'a str),
    /// No value can be used; every placeholder stays verbatim.
    Unavailable(UnavailableReason),
}

/// Why a host's placeholders were left as written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnavailableReason {
    /// The host carries no `TextParameterRefId`.
    NoReference,
    /// The reference names no `ParameterRef` of the program.
    UnknownParameter,
    /// The referenced parameter is not a text parameter.
    NotTextParameter,
    /// The value cannot be decided (e.g. module instances that cannot be told apart).
    Undecided,
}

/// Where the text inserted for `{{0}}` came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PositionalSource {
    /// The template has no `{{0}}`; it is returned unchanged.
    NoPlaceholder,
    /// The device's stored, non-empty value.
    StoredValue,
    /// The product default, non-empty, because the device stores none.
    ProductDefault,
    /// The value is empty; each placeholder's own default text was shown.
    TemplateDefault,
    /// The value is empty and no placeholder has a default: nothing inserted.
    EmptyValue,
    /// Nothing inserted; the placeholders are shown as written.
    Unresolved(UnavailableReason),
}

/// A template with its `{{0}}` placeholders filled, and where the filling came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PositionalText {
    pub text: String,
    pub source: PositionalSource,
}

/// Fill every `{{0}}`/`{{0:def}}` of `template` from `value`. Other indices,
/// empty or unusual heads, an unclosed `{{` and a stray `}}` are copied as they
/// are. Admission runs against `budget` before any copy, as for the named pass.
pub fn fill_positional(
    template: &str,
    value: PositionalValue<'_>,
    budget: &mut TextProjectionBudget,
) -> Result<PositionalText, TextProjectionError> {
    // Input reservation precedes allocation, as in the named pass.
    budget.admit_copy(template)?;
    let fill = match value {
        PositionalValue::Stored(v) | PositionalValue::ProductDefault(v) => Some(v),
        PositionalValue::Unavailable(_) => None,
    };
    let mut out = String::with_capacity(template.len());
    let mut push = |part: &str, out: &mut String| -> Result<(), TextProjectionError> {
        budget.admit_copy(part)?;
        out.push_str(part);
        Ok(())
    };
    let mut found_zero = false;
    let mut default_shown = false;
    let mut rest = template;
    while let Some(open) = rest.find(OPEN) {
        let after = &rest[open + OPEN.len()..];
        // An unclosed `{{` is not a placeholder; nothing after it is read.
        let Some(len) = placeholder_len(after) else {
            break;
        };
        let inner = &after[..len];
        push(&rest[..open], &mut out)?;
        let (head, default) = match inner.split_once(':') {
            Some((head, default)) => (head, Some(default)),
            None => (inner, None),
        };
        match (head == "0", fill) {
            (true, Some(v)) if !v.is_empty() => {
                found_zero = true;
                push(v, &mut out)?;
            }
            (true, Some(_)) => {
                found_zero = true;
                if let Some(default) = default {
                    default_shown = true;
                    push(default, &mut out)?;
                }
            }
            (true, None) | (false, _) => {
                found_zero |= head == "0";
                push(&rest[open..open + OPEN.len() + len + CLOSE.len()], &mut out)?;
            }
        }
        rest = &after[len + CLOSE.len()..];
    }
    push(rest, &mut out)?;
    let source = match (found_zero, value) {
        (false, _) => PositionalSource::NoPlaceholder,
        (true, PositionalValue::Unavailable(reason)) => PositionalSource::Unresolved(reason),
        (true, PositionalValue::Stored(v)) if !v.is_empty() => PositionalSource::StoredValue,
        (true, PositionalValue::ProductDefault(v)) if !v.is_empty() => {
            PositionalSource::ProductDefault
        }
        (true, _) if default_shown => PositionalSource::TemplateDefault,
        (true, _) => PositionalSource::EmptyValue,
    };
    Ok(PositionalText { text: out, source })
}

/// Length of a placeholder's inside, given the text after its `{{`: up to the
/// `}}` that closes it, stepping over one nested `{{…}}` (a module argument left
/// in a default). `None` when it never closes.
pub(crate) fn placeholder_len(after_open: &str) -> Option<usize> {
    let mut pos = 0;
    loop {
        let close = after_open[pos..].find(CLOSE)?;
        match after_open[pos..].find(OPEN) {
            Some(open) if open < close => {
                let inner_start = pos + open + OPEN.len();
                let inner_close = after_open[inner_start..].find(CLOSE)?;
                pos = inner_start + inner_close + CLOSE.len();
            }
            _ => return Some(pos + close),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fill(template: &str, value: PositionalValue<'_>) -> PositionalText {
        fill_positional(template, value, &mut TextProjectionBudget::default()).unwrap()
    }

    fn text(s: &str, source: PositionalSource) -> PositionalText {
        PositionalText {
            text: s.to_string(),
            source,
        }
    }

    #[test]
    fn a_stored_value_replaces_the_placeholder_and_its_default() {
        assert_eq!(
            fill("{{0:Page 1}}", PositionalValue::Stored("Kitchen")),
            text("Kitchen", PositionalSource::StoredValue)
        );
        assert_eq!(
            fill("Page 1 - {{0:Icon 1}}", PositionalValue::Stored("Lights")),
            text("Page 1 - Lights", PositionalSource::StoredValue)
        );
        assert_eq!(
            fill("{{0}}", PositionalValue::Stored("Hall")),
            text("Hall", PositionalSource::StoredValue)
        );
    }

    #[test]
    fn a_non_empty_product_default_is_the_value_too() {
        assert_eq!(
            fill("Channel {{0:A}}", PositionalValue::ProductDefault("Blinds")),
            text("Channel Blinds", PositionalSource::ProductDefault)
        );
    }

    #[test]
    fn an_empty_value_shows_each_placeholders_own_default() {
        assert_eq!(
            fill("{{0:Page 1}}", PositionalValue::Stored("")),
            text("Page 1", PositionalSource::TemplateDefault)
        );
        assert_eq!(
            fill("{{0:Page 1}}", PositionalValue::ProductDefault("")),
            text("Page 1", PositionalSource::TemplateDefault)
        );
        // Two placeholders, two defaults; the value would have filled both.
        assert_eq!(
            fill("{{0:A}} / {{0:B}}", PositionalValue::Stored("")),
            text("A / B", PositionalSource::TemplateDefault)
        );
        assert_eq!(
            fill("{{0:A}} / {{0:B}}", PositionalValue::Stored("X")),
            text("X / X", PositionalSource::StoredValue)
        );
    }

    #[test]
    fn an_empty_value_without_a_default_inserts_nothing() {
        assert_eq!(
            fill("Output {{0}}", PositionalValue::Stored("")),
            text("Output ", PositionalSource::EmptyValue)
        );
        // An empty default after the colon is a default that is empty.
        assert_eq!(
            fill("[{{0:}}]", PositionalValue::Stored("")),
            text("[]", PositionalSource::TemplateDefault)
        );
    }

    #[test]
    fn an_unavailable_value_leaves_every_placeholder_as_written() {
        for reason in [
            UnavailableReason::NoReference,
            UnavailableReason::UnknownParameter,
            UnavailableReason::NotTextParameter,
            UnavailableReason::Undecided,
        ] {
            assert_eq!(
                fill("{{0:Page 1}} - {{0}}", PositionalValue::Unavailable(reason)),
                text("{{0:Page 1}} - {{0}}", PositionalSource::Unresolved(reason))
            );
        }
    }

    #[test]
    fn a_template_without_index_zero_is_returned_unchanged() {
        for template in [
            "Plain",
            "",
            "{{1}}",
            "{{2001:Value}}",
            "{{ChNo}}",
            "{{ 0 }}",
        ] {
            assert_eq!(
                fill(template, PositionalValue::Stored("X")),
                text(template, PositionalSource::NoPlaceholder),
                "{template:?}"
            );
            // Even with nothing to fill, a missing value changes nothing.
            assert_eq!(
                fill(
                    template,
                    PositionalValue::Unavailable(UnavailableReason::NoReference)
                ),
                text(template, PositionalSource::NoPlaceholder),
                "{template:?}"
            );
        }
    }

    #[test]
    fn other_indices_and_odd_braces_stay_put_beside_a_filled_zero() {
        assert_eq!(
            fill(
                "{{1:One}} {{0:Zero}} {{2001}}",
                PositionalValue::Stored("V")
            ),
            text("{{1:One}} V {{2001}}", PositionalSource::StoredValue)
        );
        assert_eq!(
            fill("}} {{0}} {{", PositionalValue::Stored("V")),
            text("}} V {{", PositionalSource::StoredValue)
        );
        // An unclosed placeholder is not a placeholder; nothing after it is read.
        assert_eq!(
            fill("{{0:never closed", PositionalValue::Stored("V")),
            text("{{0:never closed", PositionalSource::NoPlaceholder)
        );
        // `00` is not index 0, and neither is `0x`.
        assert_eq!(
            fill("{{00}} {{0x}}", PositionalValue::Stored("V")),
            text("{{00}} {{0x}}", PositionalSource::NoPlaceholder)
        );
    }

    #[test]
    fn colons_unicode_and_braces_inside_a_default_survive() {
        assert_eq!(
            fill("{{0:Zeit: 12:00}}", PositionalValue::Stored("")),
            text("Zeit: 12:00", PositionalSource::TemplateDefault)
        );
        assert_eq!(
            fill("{{0:Küche – 温度}}", PositionalValue::Stored("")),
            text("Küche – 温度", PositionalSource::TemplateDefault)
        );
        // An argument the named pass could not bind stays visible in the default.
        assert_eq!(
            fill("{{0:Button {{ChNo}}}}", PositionalValue::Stored("")),
            text("Button {{ChNo}}", PositionalSource::TemplateDefault)
        );
        assert_eq!(
            fill("{{0:Button {{ChNo}}}} on", PositionalValue::Stored("Up")),
            text("Up on", PositionalSource::StoredValue)
        );
    }

    #[test]
    fn the_inserted_value_is_never_scanned_again() {
        assert_eq!(
            fill("{{0}}", PositionalValue::Stored("{{0:loop}}")),
            text("{{0:loop}}", PositionalSource::StoredValue)
        );
    }

    fn scope(arguments: &[(&str, &str)]) -> super::super::ModuleScope {
        super::super::ModuleScope {
            module_node: 1,
            module_id: Some("M-1".to_string()),
            module_def_id: "MD-1".to_string(),
            arguments: arguments
                .iter()
                .map(|(name, value)| super::super::BoundArgument {
                    name: name.to_string(),
                    value: value.to_string(),
                })
                .collect(),
            parent: None,
        }
    }

    fn named(raw: &str, scope: &super::super::ModuleScope) -> String {
        super::super::substitute_text_checked(
            raw,
            Some(scope),
            &mut TextProjectionBudget::default(),
        )
        .unwrap()
    }

    #[test]
    fn the_named_pass_reaches_an_argument_inside_a_default() {
        let s = scope(&[("ChNo", "3")]);
        assert_eq!(named("{{0:Button {{ChNo}}}}", &s), "{{0:Button 3}}");
        assert_eq!(
            named("PB{{ChNo}}: {{0:Buttons {{ChNo}}}}", &s),
            "PB3: {{0:Buttons 3}}"
        );
        // Then the positional pass sees an ordinary default.
        assert_eq!(
            fill(
                &named("{{0:Button {{ChNo}}}}", &s),
                PositionalValue::Stored("")
            ),
            text("Button 3", PositionalSource::TemplateDefault)
        );
        // Unbound stays visible, a numeric head stays numeric, nothing else moves.
        assert_eq!(
            named("{{0:Button {{Other}}}}", &s),
            "{{0:Button {{Other}}}}"
        );
        assert_eq!(named("{{0:Plain}} {{ChNo}}", &s), "{{0:Plain}} 3");
        assert_eq!(named("{{0:open {{ChNo}}", &s), "{{0:open 3");
    }

    #[test]
    fn a_result_that_does_not_fit_is_refused_not_cut() {
        let mut budget = TextProjectionBudget::default();
        let huge = "x".repeat(super::super::evaluate::MAX_TEXT_PROJECTION_WORK);
        assert!(fill_positional("{{0}}", PositionalValue::Stored(&huge), &mut budget).is_err());
        // The budget stays exhausted for the rest of the overlay.
        assert!(fill_positional("a", PositionalValue::Stored("b"), &mut budget).is_err());
    }
}
