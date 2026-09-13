//! The parsers, one module per manufacturer file kind. Each takes the
//! file's bytes and its content hash, writes rows, and returns the
//! constructs it did not recognize. None of them fails the ingest because
//! of one unknown element or attribute.

pub mod catalog;
pub mod comobject;
pub mod hardware;
pub mod master;
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

/// `"1"`/`"0"` and `xs:boolean`'s other canonical spelling, `"true"`/`"false"`,
/// as SQLite integers; anything else stays `None` rather than being guessed
/// into a boolean. `"false"` earned its place here because schema 20/21
/// write `Linkable="false"`, and that is not unknown text, it is the
/// datatype's own lexical form — so `"True"`, `"yes"` and `"-1"` still stay
/// `None`, uppercase and synonyms included.
///
/// An attribute that is simply absent is not reported — there is nothing to
/// discard. One present under a spelling `xs:boolean` does not recognize is
/// reported through `collector` at `xpath` instead of vanishing into `None`
/// unremarked, so the next corpus's surprise ends up in an ingest report
/// rather than nowhere.
pub(crate) fn bool_flag(
    collector: &mut UnknownCollector,
    xpath: &str,
    a: &Attrs,
    name: &str,
) -> Option<i64> {
    match a.get(name) {
        Some("1") | Some("true") => Some(1),
        Some("0") | Some("false") => Some(0),
        Some(other) => {
            collector.attribute(xpath, name, other);
            None
        }
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quick_xml::events::Event;
    use quick_xml::Reader;

    fn attrs_from(xml: &str) -> Attrs {
        let mut r = Reader::from_reader(xml.as_bytes());
        let mut buf = Vec::new();
        let start = match r.read_event_into(&mut buf).unwrap() {
            Event::Empty(e) => e,
            other => panic!("expected an empty element, got {other:?}"),
        };
        crate::xml::attrs(&start, "t.xml").unwrap()
    }

    #[test]
    fn all_four_xs_boolean_spellings_are_recognised() {
        let mut unknown = UnknownCollector::default();
        assert_eq!(
            bool_flag(&mut unknown, "/E", &attrs_from(r#"<E F="1"/>"#), "F"),
            Some(1)
        );
        assert_eq!(
            bool_flag(&mut unknown, "/E", &attrs_from(r#"<E F="0"/>"#), "F"),
            Some(0)
        );
        assert_eq!(
            bool_flag(&mut unknown, "/E", &attrs_from(r#"<E F="true"/>"#), "F"),
            Some(1)
        );
        assert_eq!(
            bool_flag(&mut unknown, "/E", &attrs_from(r#"<E F="false"/>"#), "F"),
            Some(0)
        );
        assert!(unknown.into_vec().is_empty());
    }

    #[test]
    fn capitalised_or_absent_spellings_stay_none() {
        let mut unknown = UnknownCollector::default();
        assert_eq!(
            bool_flag(&mut unknown, "/E", &attrs_from(r#"<E F="True"/>"#), "F"),
            None
        );
        assert_eq!(
            bool_flag(&mut unknown, "/E", &attrs_from(r#"<E F=""/>"#), "F"),
            None
        );
        assert_eq!(
            bool_flag(&mut unknown, "/E", &attrs_from(r#"<E/>"#), "F"),
            None
        );
    }

    #[test]
    fn an_unrecognised_spelling_is_recorded_through_the_collector() {
        let mut unknown = UnknownCollector::default();
        let value = bool_flag(&mut unknown, "/E", &attrs_from(r#"<E F="True"/>"#), "F");
        assert_eq!(value, None);
        let recorded = unknown.into_vec();
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0].xpath, "/E");
        assert_eq!(recorded[0].name, "F");
        assert_eq!(recorded[0].sample.as_deref(), Some("True"));
    }

    #[test]
    fn an_absent_attribute_is_not_reported_as_unrecognised() {
        let mut unknown = UnknownCollector::default();
        let value = bool_flag(&mut unknown, "/E", &attrs_from(r#"<E/>"#), "F");
        assert_eq!(value, None);
        assert!(unknown.into_vec().is_empty());
    }
}
