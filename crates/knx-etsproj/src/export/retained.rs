//! Holds the attributes an import preserved but the domain model does not carry, hands each one back to the element it came from, and turns whatever could not be written into an export warning instead of a silence.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

use super::ExportWarning;
use crate::opaque::{OpaqueEntry, OpaqueKind};

type Key = (String, String);

/// What the store knows about one `(xpath, name)` key.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Value {
    /// Exactly one source element produced this key.
    Unique(String),
    /// Several did — their identities collapsed into the same key, so which
    /// element any of the values belongs to is no longer recoverable. Never
    /// written back: `KNOWN_LIMITATIONS.md` §34's ruling is that corruption
    /// is worse than loss, and writing one of them would be a coin flip.
    Ambiguous { instances: usize },
}

/// What happened to a key while the document was being written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Outcome {
    /// Written back onto its own element. The happy path.
    Restored,
    /// The model already carried this attribute and wrote the same text, so
    /// nothing was lost — the retained copy was simply redundant.
    ModelAgrees,
    /// The model wrote a *different* value. Not a bug on its own: a project
    /// edited after import is supposed to win over the imported source. It
    /// is still reported, because the other reading — the model parsed the
    /// source value wrongly and is now overwriting it — looks identical
    /// from here, and staying quiet would hide it.
    ModelDisagrees,
}

/// Every retained attribute in an opaque set, keyed by the element instance
/// it belongs to ([`crate::xpath`]), with a note of what each export made of
/// it.
///
/// Interior mutability on purpose: the writers take `&RetainedAttrs` and are
/// threaded through a dozen call sites that have no business holding a
/// mutable borrow of a read-only lookup table, and the two documents an
/// export produces (`0.xml` and `Project.xml`) share one store so that a key
/// consumed by either counts as consumed.
pub(crate) struct RetainedAttrs {
    values: BTreeMap<Key, Value>,
    outcomes: RefCell<BTreeMap<Key, Outcome>>,
}

impl RetainedAttrs {
    pub(crate) fn from_opaque(opaque: &[OpaqueEntry]) -> Self {
        let mut values: BTreeMap<Key, Value> = BTreeMap::new();
        for entry in opaque
            .iter()
            .filter(|e| e.kind == OpaqueKind::RetainedAttribute)
        {
            let key = (entry.xpath.clone(), entry.name.clone());
            let text = String::from_utf8_lossy(&entry.bytes).into_owned();
            values
                .entry(key)
                .and_modify(|existing| {
                    let instances = match existing {
                        Value::Unique(prior) if *prior == text => return,
                        Value::Unique(_) => 2,
                        Value::Ambiguous { instances } => *instances + 1,
                    };
                    *existing = Value::Ambiguous { instances };
                })
                .or_insert(Value::Unique(text));
        }
        Self {
            values,
            outcomes: RefCell::new(BTreeMap::new()),
        }
    }

    /// The unambiguous value for one key, marked as restored. Callers that
    /// want an attribute in a particular position in the element rather than
    /// appended last use this; everything else goes through
    /// [`super::schema11::Attrs::fill_retained`].
    pub(crate) fn take(&self, xpath: &str, name: &str) -> Option<String> {
        let key = (xpath.to_string(), name.to_string());
        match self.values.get(&key) {
            Some(Value::Unique(v)) => {
                self.outcomes.borrow_mut().insert(key, Outcome::Restored);
                Some(v.clone())
            }
            _ => None,
        }
    }

    /// Every unambiguous `(name, value)` retained for one element instance.
    fn unique_at(&self, xpath: &str) -> Vec<(String, String)> {
        self.values
            .range((xpath.to_string(), String::new())..)
            .take_while(|((x, _), _)| x == xpath)
            .filter_map(|((_, name), value)| match value {
                Value::Unique(v) => Some((name.clone(), v.clone())),
                Value::Ambiguous { .. } => None,
            })
            .collect()
    }

    /// Every unambiguous retained value, without its key. Tests use this to
    /// pick a value out of the corpus rather than writing a real device's
    /// name or address into the repository.
    #[cfg(test)]
    pub(crate) fn unique_values(&self) -> impl Iterator<Item = String> + '_ {
        self.values.values().filter_map(|v| match v {
            Value::Unique(s) => Some(s.clone()),
            Value::Ambiguous { .. } => None,
        })
    }

    fn record(&self, xpath: &str, name: &str, outcome: Outcome) {
        self.outcomes
            .borrow_mut()
            .insert((xpath.to_string(), name.to_string()), outcome);
    }

    /// One warning per `(element, attribute)` class that did not make it
    /// back into the document intact, naming what was dropped and why.
    /// Grouped rather than per instance: 514 group addresses that all lost
    /// their `Puid` is one fact, not 514.
    pub(crate) fn residue(&self) -> Vec<ExportWarning> {
        let outcomes = self.outcomes.borrow();
        let mut classes: BTreeMap<(String, String, Reason), usize> = BTreeMap::new();
        for (key, value) in &self.values {
            let reason = match (outcomes.get(key), value) {
                (_, Value::Ambiguous { .. }) => Reason::Ambiguous,
                (None, _) => Reason::NotReconstructed,
                (Some(Outcome::ModelDisagrees), _) => Reason::Overwritten,
                (Some(Outcome::Restored | Outcome::ModelAgrees), _) => continue,
            };
            let instances = match value {
                Value::Ambiguous { instances } => *instances,
                Value::Unique(_) => 1,
            };
            *classes
                .entry((element_of(&key.0), key.1.clone(), reason))
                .or_default() += instances;
        }
        classes
            .into_iter()
            .map(|((element, attribute, reason), instances)| {
                let detail = reason.detail(&element, &attribute, instances);
                ExportWarning::RetainedAttributeNotExported {
                    element,
                    attribute,
                    instances,
                    detail,
                }
            })
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Reason {
    Ambiguous,
    NotReconstructed,
    Overwritten,
}

impl Reason {
    fn detail(self, element: &str, attribute: &str, instances: usize) -> String {
        match self {
            Reason::Ambiguous => format!(
                "{instances} source elements shared one `{element}` identity, so \
                 `@{attribute}` was left out rather than written onto whichever of \
                 them happened to come first; the value is still preserved verbatim \
                 in the project's opaque store"
            ),
            Reason::NotReconstructed => format!(
                "`{element}/@{attribute}` was preserved on import ({instances} \
                 element(s)) but this writer has nowhere to put it back; the value \
                 is still preserved verbatim in the project's opaque store"
            ),
            Reason::Overwritten => format!(
                "`{element}/@{attribute}` ({instances} element(s)) was written from \
                 the project, which disagrees with the value the import preserved; \
                 the imported value is still in the project's opaque store"
            ),
        }
    }
}

/// `"DeviceInstance"` from
/// `"/KNX/…/Segment/DeviceInstance[@Id='P-0001-0_DI-1']"`. The id itself is
/// deliberately not carried into the warning: a corpus `SerialNumber` sits
/// on a real device, and a warning is a user-facing string that ends up in
/// logs and reports.
fn element_of(xpath: &str) -> String {
    xpath
        .rsplit('/')
        .next()
        .unwrap_or(xpath)
        .split('[')
        .next()
        .unwrap_or(xpath)
        .to_string()
}

impl super::schema11::Attrs {
    /// Fills every attribute this element instance has in the retained
    /// store. A name the model already wrote wins — the project is the
    /// authority on anything it models — but the disagreement is recorded
    /// either way, so [`RetainedAttrs::residue`] can report it.
    pub(crate) fn fill_retained(&mut self, retained: &RetainedAttrs, xpath: &str) -> &mut Self {
        for (name, value) in retained.unique_at(xpath) {
            match self.value_of(&name) {
                None => {
                    self.push(&name, value);
                    retained.record(xpath, &name, Outcome::Restored);
                }
                Some(written) if written == value => {
                    retained.record(xpath, &name, Outcome::ModelAgrees)
                }
                Some(_) => retained.record(xpath, &name, Outcome::ModelDisagrees),
            }
        }
        self
    }
}

/// The raw-XML side of the same idea: whole elements an import preserved
/// verbatim ([`crate::source::RetainedElement`]) because the domain model
/// has no home for them, keyed by the element instance they came from, with
/// a note of which ones a writer actually put back.
pub(crate) struct RetainedElements {
    by_xpath: BTreeMap<String, Vec<u8>>,
    /// Entry counts per xpath: six devices' `BinaryData` blobs collapse into
    /// one key, and a warning that says "6" is worth more than one that
    /// says "1".
    instances: BTreeMap<String, usize>,
    used: RefCell<BTreeSet<String>>,
}

impl RetainedElements {
    pub(crate) fn from_opaque(opaque: &[OpaqueEntry]) -> Self {
        let mut by_xpath = BTreeMap::new();
        let mut instances: BTreeMap<String, usize> = BTreeMap::new();
        for entry in opaque
            .iter()
            .filter(|e| e.kind == OpaqueKind::RetainedElement)
        {
            by_xpath.insert(entry.xpath.clone(), entry.bytes.clone());
            *instances.entry(entry.xpath.clone()).or_default() += 1;
        }
        Self {
            by_xpath,
            instances,
            used: RefCell::new(BTreeSet::new()),
        }
    }

    /// The raw bytes for one element instance, marked as written back.
    pub(crate) fn take(&self, xpath: &str) -> Option<&[u8]> {
        let raw = self.by_xpath.get(xpath)?;
        self.used.borrow_mut().insert(xpath.to_string());
        Some(raw)
    }

    /// Every retained blob, for tests that want to prove one came back out
    /// verbatim without writing its contents into the repository.
    #[cfg(test)]
    pub(crate) fn raw_values(&self) -> impl Iterator<Item = &[u8]> + '_ {
        self.by_xpath.values().map(|v| v.as_slice())
    }

    /// One warning per element name that no writer claimed. An element whose
    /// key holds more than one instance was never restorable in the first
    /// place — the instances are indistinguishable — so the count is
    /// reported and the bytes stay where they are safe, in the opaque store.
    pub(crate) fn residue(&self) -> Vec<ExportWarning> {
        let used = self.used.borrow();
        let mut classes: BTreeMap<String, usize> = BTreeMap::new();
        for (xpath, count) in &self.instances {
            if used.contains(xpath) {
                continue;
            }
            *classes.entry(element_of(xpath)).or_default() += count;
        }
        classes
            .into_iter()
            .map(|(element, instances)| ExportWarning::RetainedElementNotExported {
                detail: format!(
                    "`<{element}>` ({instances} element(s)) was preserved verbatim on \
                     import but this writer has nowhere to put it back; the XML is \
                     still preserved byte-exact in the project's opaque store"
                ),
                element,
                instances,
            })
            .collect()
    }
}
