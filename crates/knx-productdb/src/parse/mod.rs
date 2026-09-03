//! The parsers, one module per manufacturer file kind. Each takes the
//! file's bytes and its content hash, writes rows, and returns the
//! constructs it did not recognize. None of them fails the ingest because
//! of one unknown element or attribute.

pub mod catalog;
pub mod comobject;
pub mod hardware;
pub mod program;
pub mod translation;

use crate::report::UnknownCollector;
use crate::xml::Attrs;

/// Reports every attribute on `a` that is not in `known`, so an unmodelled
/// manufacturer attribute is visible rather than lost.
pub(crate) fn report_unknown_attrs(
    collector: &mut UnknownCollector,
    xpath: &str,
    a: &Attrs,
    known: &[&str],
) {
    for name in a.names() {
        if !known.contains(&name) {
            let sample = a.get(name).unwrap_or_default();
            collector.attribute(xpath, name, sample);
        }
    }
}

/// `"1"`/`"0"` as SQLite integers; anything else stays `None` rather than
/// being guessed into a boolean.
pub(crate) fn bool_flag(a: &Attrs, name: &str) -> Option<i64> {
    match a.get(name) {
        Some("1") => Some(1),
        Some("0") => Some(0),
        _ => None,
    }
}
