//! The import pipeline's own progress vocabulary: which stage is running, and how far into it.
//!
//! ADR-0023 decides that a progress indicator may only report work the
//! pipeline actually did, which means the pipeline has to say so itself —
//! the HTTP layer above cannot see into this crate and would have to invent
//! the stages. [`ImportStage`] is that vocabulary, and nothing here knows
//! or cares who is listening: no HTTP, no JSON, no dependency added.
//!
//! Two rules the callers in `lib.rs` follow, and the reason this module has
//! a doc comment at all:
//!
//! 1. A stage is announced *before* the work it names, so the label a user
//!    reads is the thing currently running, not the thing just finished.
//! 2. [`ImportObserver::items`] is only ever called where a real total is
//!    known before the loop starts. There is no variant of it that takes a
//!    guess, an elapsed time or a byte offset — a bar driven by any of
//!    those reports something other than progress.

/// A stage of [`crate::import_knxproj_bytes_observed`], in the order that
/// function runs them. Every variant names a step that exists in the code
/// — this enum is not a curated summary, and adding a step to the pipeline
/// without adding it here leaves a gap nothing but review will notice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImportStage {
    /// Reading the ZIP central directory and, for a protected archive,
    /// decrypting it.
    OpenContainer,
    /// Working out which `.knxproj` schema version this is.
    DetectSchema,
    /// Parsing `<part>/0.xml`, the topology — typically the largest entry
    /// in the archive and the longest stage of the import.
    ParseTopology,
    /// Parsing `<part>/Project.xml` (or `project.xml` at schema ≥ 21).
    ParseProjectInfo,
    /// Checking the parsed document's internal references.
    Validate,
    /// Mapping the source document into `knx_core::Project`.
    Map,
    /// Inferring each group address's datapoint type from what is linked
    /// to it.
    InferDatapointTypes,
    /// Reading back every container entry the exporter cannot regenerate.
    /// The one stage in this crate with a real total: the archive's entry
    /// count is known before the loop starts.
    CollectContainerEntries,
}

impl ImportStage {
    /// A stable, lower-camel-case name, for a caller that has to put this
    /// on a wire. Defined here rather than in the server so the two cannot
    /// drift apart in a way only a running browser would reveal.
    pub const fn as_str(self) -> &'static str {
        match self {
            ImportStage::OpenContainer => "openContainer",
            ImportStage::DetectSchema => "detectSchema",
            ImportStage::ParseTopology => "parseTopology",
            ImportStage::ParseProjectInfo => "parseProjectInfo",
            ImportStage::Validate => "validate",
            ImportStage::Map => "map",
            ImportStage::InferDatapointTypes => "inferDatapointTypes",
            ImportStage::CollectContainerEntries => "collectContainerEntries",
        }
    }
}

/// Told, as it happens, what the import is doing. Implementations must be
/// cheap and must not fail: this is called from inside the pipeline, which
/// has no way to report a broken listener and no business trying.
pub trait ImportObserver {
    /// A new stage has begun. Any item count from the previous stage is
    /// stale from this moment on.
    fn stage(&self, stage: ImportStage);

    /// `completed` of `total` items done *within the current stage*, where
    /// both are real counts. The default does nothing, because most
    /// observers only care about stage labels.
    fn items(&self, completed: u64, total: u64) {
        let _ = (completed, total);
    }
}

/// The observer for a caller that wants none: every `import_knxproj*`
/// function without an observer passes `&()`, so there is exactly one
/// import implementation rather than an observed and an unobserved copy.
impl ImportObserver for () {
    fn stage(&self, _stage: ImportStage) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Records what it is told, in order. `Mutex` rather than `RefCell`
    /// because [`ImportObserver`] takes `&self` and a future caller may
    /// well hand one of these across a thread.
    #[derive(Default)]
    pub(crate) struct Recorder {
        pub(crate) stages: Mutex<Vec<ImportStage>>,
        pub(crate) items: Mutex<Vec<(ImportStage, u64, u64)>>,
    }

    impl ImportObserver for Recorder {
        fn stage(&self, stage: ImportStage) {
            self.stages.lock().unwrap().push(stage);
        }

        fn items(&self, completed: u64, total: u64) {
            let current = *self
                .stages
                .lock()
                .unwrap()
                .last()
                .expect("items() is only ever called inside a stage");
            self.items.lock().unwrap().push((current, completed, total));
        }
    }

    #[test]
    fn a_minimal_import_announces_every_stage_exactly_once_in_pipeline_order() {
        let recorder = Recorder::default();
        crate::import_knxproj_bytes_observed(
            knx_testsupport::minimal_knxproj_bytes(),
            "minimal.knxproj",
            &recorder,
        )
        .unwrap();

        assert_eq!(
            *recorder.stages.lock().unwrap(),
            vec![
                ImportStage::OpenContainer,
                ImportStage::DetectSchema,
                ImportStage::ParseTopology,
                ImportStage::ParseProjectInfo,
                ImportStage::Validate,
                ImportStage::Map,
                ImportStage::InferDatapointTypes,
                ImportStage::CollectContainerEntries,
            ]
        );
    }

    #[test]
    fn the_only_counted_stage_counts_the_entries_it_actually_read() {
        let recorder = Recorder::default();
        let outcome = crate::import_knxproj_bytes_observed(
            knx_testsupport::minimal_knxproj_bytes(),
            "minimal.knxproj",
            &recorder,
        )
        .unwrap();

        let items = recorder.items.lock().unwrap().clone();
        assert!(
            items
                .iter()
                .all(|(stage, ..)| *stage == ImportStage::CollectContainerEntries),
            "only the entry-collection stage has a real total: {items:?}"
        );
        // The fixture has six entries; `0.xml` and `Project.xml` are
        // regenerated on export, so four are read back here — and the
        // count reported is the count that came out, not a prediction.
        // The whole sequence, not just where it ended up: a counter that
        // skipped an entry, reported one twice or announced its total
        // early would land on (6, 6) all the same.
        let counts: Vec<(u64, u64)> = items
            .iter()
            .map(|(_, completed, total)| (*completed, *total))
            .collect();
        assert_eq!(
            counts,
            vec![(1, 6), (2, 6), (3, 6), (4, 6), (5, 6), (6, 6)],
            "every entry in the archive is walked, once, in order"
        );
        let whole_files = outcome.opaque.iter().filter(|e| e.xpath.is_empty()).count();
        assert_eq!(whole_files + outcome.manufacturer.len(), 4);
    }

    /// The sequence above is the same under either convention — reporting
    /// an entry before reading it counts 1..6 exactly like reporting it
    /// after. What separates them is an entry that cannot be read: the
    /// count may only name work that is done, so the entry the collector
    /// dies on must never have been reported as complete (ADR-0023, fix
    /// round 6 finding F-E). Everywhere else on this branch a displayed
    /// number follows its work; this was the one place it led.
    #[test]
    fn a_counted_entry_is_only_reported_once_its_work_is_done() {
        /// Bare counter: this exercises `collect_container_entries_observed`
        /// directly, so there is no enclosing stage for `Recorder` to
        /// attribute the counts to.
        #[derive(Default)]
        struct Counts(Mutex<Vec<(u64, u64)>>);

        impl ImportObserver for Counts {
            fn stage(&self, _stage: ImportStage) {}

            fn items(&self, completed: u64, total: u64) {
                self.0.lock().unwrap().push((completed, total));
            }
        }

        let mut container = crate::Container::open(zip_with_a_broken_last_entry())
            .expect("the inventory is intact; only the third entry's bytes are not");
        assert_eq!(container.entries().len(), 3);
        let counts = Counts::default();
        let error = crate::opaque::collect_container_entries_observed(&mut container, &[], &counts)
            .expect_err("the third entry's bytes do not match its CRC");

        assert!(
            format!("{error}").contains("third.xml"),
            "the read dies on the third entry, not somewhere else: {error}"
        );
        assert_eq!(
            *counts.0.lock().unwrap(),
            vec![(1, 3), (2, 3)],
            "the entry the collector never finished reading is not counted as done"
        );
    }

    /// Three stored entries, with one byte of the *third* entry's body
    /// flipped after the archive was written, so its stored CRC-32 no
    /// longer matches its bytes. Headers and central directory are
    /// untouched: the container opens, `entries()` reports three, and the
    /// archive only falls apart when that entry is actually read — which
    /// is exactly the moment the test above is about.
    fn zip_with_a_broken_last_entry() -> Vec<u8> {
        use std::io::{Cursor, Write};
        const THIRD_BODY: &[u8] = b"<third-entry/>";
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        for (name, body) in [
            ("first.xml", b"<first-entry/>".as_slice()),
            ("second.xml", b"<second-entry/>".as_slice()),
            ("third.xml", THIRD_BODY),
        ] {
            writer.start_file(name, options).expect("zip entry");
            writer.write_all(body).expect("zip entry body");
        }
        let mut bytes = writer.finish().expect("zip central directory").into_inner();
        // Stored, not deflated, so the body sits in the file verbatim and
        // appears exactly once.
        let at = bytes
            .windows(THIRD_BODY.len())
            .position(|w| w == THIRD_BODY)
            .expect("a stored entry's body is its bytes");
        bytes[at] = b'!';
        bytes
    }

    #[test]
    fn a_stage_name_is_stable_lower_camel_case() {
        assert_eq!(ImportStage::ParseTopology.as_str(), "parseTopology");
        assert_eq!(
            ImportStage::CollectContainerEntries.as_str(),
            "collectContainerEntries"
        );
    }
}
