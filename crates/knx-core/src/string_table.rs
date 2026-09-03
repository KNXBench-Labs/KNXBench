//! Localized strings as handles into a project-wide table, not bare
//! `String`s (DATA_MODEL §8). Import populates the table from the
//! `TranslationUnit` trees — one application program alone carries 5919
//! translation elements — which is why the indirection is in the model from
//! day one rather than retrofitted later.

use std::collections::HashMap;
use std::fmt;

/// A language tag, e.g. `"de-DE"`. Not validated against BCP-47 here; that
/// belongs to the importer, which only ever sees tags ETS itself produced.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Language(pub String);

/// The source id a `TranslationUnit` entry is keyed by.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TranslationKey(pub String);

/// A handle into a `StringTable`. Never a bare `String` — resolving it
/// requires the table and the active language.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LocalizedString(pub TranslationKey);

/// Maps `(TranslationKey, Language)` to display text. Display resolves
/// against the active language and falls back to `default_language`.
#[derive(Debug, PartialEq)]
pub struct StringTable {
    default_language: Language,
    entries: HashMap<(TranslationKey, Language), String>,
}

impl StringTable {
    pub fn new(default_language: Language) -> Self {
        Self {
            default_language,
            entries: HashMap::new(),
        }
    }

    /// The language display falls back to when no entry exists for the
    /// language asked for. A consumer that has no particular language in
    /// mind (export, comparison) uses this rather than naming one itself.
    pub fn default_language(&self) -> &Language {
        &self.default_language
    }

    pub fn insert(&mut self, key: TranslationKey, language: Language, text: String) {
        self.entries.insert((key, language), text);
    }

    /// Every entry as `(key, language, value)`. Iteration order is
    /// unspecified (backed by a `HashMap`) — persistence writes all of it
    /// regardless of order, since string-table entries are looked up by
    /// key, never enumerated positionally (DATA_MODEL §8).
    pub fn iter(&self) -> impl Iterator<Item = (&TranslationKey, &Language, &str)> {
        self.entries.iter().map(|((k, l), v)| (k, l, v.as_str()))
    }

    /// Resolves `handle` for `language`, falling back to the table's
    /// default language if `language` has no entry for that key. Returns
    /// `None` only when the key has no entry in either language.
    pub fn resolve(&self, handle: &LocalizedString, language: &Language) -> Option<&str> {
        let key = &handle.0;
        self.entries
            .get(&(key.clone(), language.clone()))
            .or_else(|| {
                self.entries
                    .get(&(key.clone(), self.default_language.clone()))
            })
            .map(String::as_str)
    }
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A display string that is either literal or a handle into the string table.
///
/// Instance-level overrides in `0.xml` are literal — ETS writes the text into
/// the project file and keeps no translation for it. Program-level defaults
/// are localized, resolved through `TranslationUnit` trees in the application
/// program. One field has to hold both.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Text {
    Literal(String),
    Localized(LocalizedString),
}

impl StringTable {
    /// Resolves either form of text for `language`. A literal is returned as
    /// it stands; a handle is resolved through the table.
    pub fn text<'a>(&'a self, text: &'a Text, language: &Language) -> Option<&'a str> {
        match text {
            Text::Literal(s) => Some(s.as_str()),
            Text::Localized(handle) => self.resolve(handle, language),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_exact_language_match() {
        let mut t = StringTable::new(Language("en".into()));
        let key = TranslationKey("k1".into());
        t.insert(key.clone(), Language("de".into()), "Licht".into());
        t.insert(key.clone(), Language("en".into()), "Light".into());
        let h = LocalizedString(key);
        assert_eq!(t.resolve(&h, &Language("de".into())), Some("Licht"));
    }

    #[test]
    fn falls_back_to_default_language_when_missing() {
        let mut t = StringTable::new(Language("en".into()));
        let key = TranslationKey("k1".into());
        t.insert(key.clone(), Language("en".into()), "Light".into());
        let h = LocalizedString(key);
        assert_eq!(t.resolve(&h, &Language("fr".into())), Some("Light"));
    }

    #[test]
    fn returns_none_when_key_entirely_absent() {
        let t = StringTable::new(Language("en".into()));
        let h = LocalizedString(TranslationKey("missing".into()));
        assert_eq!(t.resolve(&h, &Language("en".into())), None);
    }

    #[test]
    fn literal_text_resolves_without_the_table_localized_text_does_not() {
        let table = StringTable::new(Language("en".into()));
        let literal = Text::Literal("LICHT_AN_AUS_EG_GARDEROBE".into());
        assert_eq!(
            table.text(&literal, &Language("de".into())),
            Some("LICHT_AN_AUS_EG_GARDEROBE")
        );
        let localized = Text::Localized(LocalizedString(TranslationKey("k1".into())));
        assert_eq!(table.text(&localized, &Language("de".into())), None);
    }

    #[test]
    fn iter_yields_every_entry_regardless_of_order() {
        let mut t = StringTable::new(Language("en".into()));
        t.insert(
            TranslationKey("k1".into()),
            Language("de".into()),
            "Licht".into(),
        );
        t.insert(
            TranslationKey("k2".into()),
            Language("en".into()),
            "Heat".into(),
        );
        let mut seen: Vec<(String, String, String)> = t
            .iter()
            .map(|(k, l, v)| (k.0.clone(), l.0.clone(), v.to_string()))
            .collect();
        seen.sort();
        assert_eq!(
            seen,
            vec![
                ("k1".to_string(), "de".to_string(), "Licht".to_string()),
                ("k2".to_string(), "en".to_string(), "Heat".to_string()),
            ]
        );
    }
}
