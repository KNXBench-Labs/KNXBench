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
#[derive(Debug)]
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

    pub fn insert(&mut self, key: TranslationKey, language: Language, text: String) {
        self.entries.insert((key, language), text);
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
}
