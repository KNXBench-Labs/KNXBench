//! What a journal writer already verified, so its next write re-derives none of it.

use std::collections::BTreeMap;

use super::{HistoryError, NativeSnapshot, StackRows};
use crate::{ManufacturerRef, StoredOpaqueEntry};
use knx_core::Project;

/// The retained context a writer stored or admitted: its value, its semantic
/// identity and the SHA-256 of its stored bytes.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct KnownContext {
    pub(super) opaque: Vec<StoredOpaqueEntry>,
    pub(super) manufacturer_refs: Vec<ManufacturerRef>,
    pub(super) content_hash: String,
    pub(super) image_hash: String,
}

impl KnownContext {
    /// Whether `snapshot` carries exactly this context value.
    pub(super) fn holds(&self, snapshot: &NativeSnapshot) -> bool {
        self.opaque == snapshot.opaque && self.manufacturer_refs == snapshot.manufacturer_refs
    }
}

/// A saved baseline and its semantic hash.
#[derive(Debug, Clone, PartialEq)]
struct KnownBaseline {
    project: Project,
    /// The baseline's retained context, kept only while it differs from the
    /// known context (unsaved plugin data, for example); `None` is that one.
    retained: Option<(Vec<StoredOpaqueEntry>, Vec<ManufacturerRef>)>,
    hash: String,
}

/// What a writer knows of the journal it last wrote or opened (ADR-0165).
///
/// `rows` are the stored stack row image hashes (ADR-0163). The context,
/// baseline and working state are remembered with full values: a stored row
/// whose bytes still have the remembered SHA-256 holds that value, and a saved
/// baseline equal to the remembered one has its semantic hash, because the hash
/// is a function of the value alone. Every claim is checked against the file; a
/// stale one only costs the full decode or encode it would have saved.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct JournalTrust {
    pub rows: StackRows,
    context: Option<KnownContext>,
    baseline: Option<KnownBaseline>,
    /// The stored working model and the SHA-256 of its stored image.
    working: Option<(Project, String)>,
    /// Other retained contexts this writer decoded and checked, such as those
    /// of saved versions: semantic identity to the SHA-256 of the stored bytes.
    checked_contexts: BTreeMap<String, String>,
}

impl JournalTrust {
    /// Knows nothing: every check decodes and every write encodes.
    pub(super) const EMPTY: Self = Self {
        rows: StackRows {
            undo: Vec::new(),
            redo: Vec::new(),
        },
        context: None,
        baseline: None,
        working: None,
        checked_contexts: BTreeMap::new(),
    };

    /// Only stored row hashes; no context or baseline is known.
    pub fn from_rows(rows: StackRows) -> Self {
        Self {
            rows,
            ..Self::default()
        }
    }

    /// The same knowledge with other stack rows, e.g. a proven shared prefix.
    /// Consumes the trust, so the large retained context is moved, not copied.
    pub fn with_rows(self, rows: StackRows) -> Self {
        Self { rows, ..self }
    }

    /// The same knowledge, with `checked` as the other checked contexts.
    pub(super) fn with_checked_contexts(self, checked: BTreeMap<String, String>) -> Self {
        Self {
            checked_contexts: checked,
            ..self
        }
    }

    /// Whether this writer checked the context `content_hash` stored with the
    /// SHA-256 `image_hash`.
    pub(super) fn knows_context(&self, content_hash: &str, image_hash: &str) -> bool {
        self.context.as_ref().is_some_and(|known| {
            known.content_hash == content_hash && known.image_hash == image_hash
        }) || self
            .checked_contexts
            .get(content_hash)
            .is_some_and(|known| known == image_hash)
    }

    pub(super) fn context(&self) -> Option<&KnownContext> {
        self.context.as_ref()
    }

    /// Remembers `context`, the stored `working` model with its image hash and
    /// the `baseline` with its semantic hash `baseline_hash`.
    pub(super) fn knowing(
        rows: StackRows,
        context: KnownContext,
        baseline: &NativeSnapshot,
        baseline_hash: &str,
        working: (Project, String),
    ) -> Self {
        let retained = (!context.holds(baseline))
            .then(|| (baseline.opaque.clone(), baseline.manufacturer_refs.clone()));
        Self {
            rows,
            context: Some(context),
            baseline: Some(KnownBaseline {
                project: baseline.project.clone(),
                retained,
                hash: baseline_hash.to_owned(),
            }),
            working: Some(working),
            checked_contexts: BTreeMap::new(),
        }
    }

    pub(super) fn working(&self) -> Option<&(Project, String)> {
        self.working.as_ref()
    }

    /// The semantic hash of `snapshot`: the remembered one when it equals the
    /// remembered baseline, otherwise computed.
    pub(super) fn semantic_hash(&self, snapshot: &NativeSnapshot) -> Result<String, HistoryError> {
        match self.known_semantic_hash(snapshot) {
            Some(hash) => Ok(hash),
            None => snapshot.semantic_hash(),
        }
    }

    /// The remembered semantic hash of `snapshot` when it equals the
    /// remembered baseline, value and retained context alike.
    pub(super) fn known_semantic_hash(&self, snapshot: &NativeSnapshot) -> Option<String> {
        let baseline = self.baseline.as_ref()?;
        let same_context = match &baseline.retained {
            Some((opaque, manufacturer_refs)) => {
                snapshot.opaque == *opaque && snapshot.manufacturer_refs == *manufacturer_refs
            }
            None => self
                .context
                .as_ref()
                .is_some_and(|known| known.holds(snapshot)),
        };
        (same_context && snapshot.project == baseline.project).then(|| baseline.hash.clone())
    }
}
