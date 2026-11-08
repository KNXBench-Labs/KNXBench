//! Wall times and size facts of one journal write, for diagnostics without project content.

use std::time::{Duration, Instant};

/// What one journal write did and how long each phase took (ADR-0165). Holds
/// counts, byte sizes and flags only: no names, paths or project values.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct JournalWriteReport {
    /// Wall time per phase, in the order the phases ran.
    pub phases: Vec<(&'static str, Duration)>,
    pub undo_states: usize,
    pub redo_states: usize,
    /// Stack rows kept as stored (ADR-0163).
    pub kept_rows: usize,
    /// New stack rows stored with the previous working image's bytes.
    pub copied_rows: usize,
    /// New stack rows encoded afresh.
    pub encoded_rows: usize,
    pub saved_versions: usize,
    /// Stored bytes of the working state's retained context.
    pub context_bytes: u64,
    /// Whether the retained context was taken from what the writer knew.
    pub context_reused: bool,
    /// Whether the baseline's semantic hash was taken from what the writer knew.
    pub baseline_hash_reused: bool,
}

impl JournalWriteReport {
    /// Records `phase` as having run from `started` until now.
    pub fn phase(&mut self, phase: &'static str, started: Instant) {
        self.phases.push((phase, started.elapsed()));
    }

    /// The sum of all recorded phases.
    pub fn total(&self) -> Duration {
        self.phases.iter().map(|(_, time)| *time).sum()
    }
}
