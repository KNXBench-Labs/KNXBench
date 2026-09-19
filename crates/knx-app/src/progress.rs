//! One progress vocabulary for a whole project load, parser stages included.
//!
//! `knx-etsproj` names its own stages ([`knx_etsproj::ImportStage`]) and
//! this crate adds the ones it runs itself — manufacturer ingest,
//! enrichment, the opaque insert. Neither crate names the other's work,
//! which is why [`LoadStage::Parse`] wraps the parser's enum instead of
//! restating it: a stage added down there appears up here without anyone
//! remembering to copy it.
//!
//! ADR-0023 is the decision this implements, including the rule that keeps
//! it honest: [`LoadObserver::items`] is called only where a real total
//! exists before the loop starts, and there is deliberately no way to
//! report an elapsed time.

use knx_etsproj::{ImportObserver, ImportStage};

/// What a project load is doing right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadStage {
    /// A stage inside `knx-etsproj`'s own pipeline, forwarded unchanged.
    Parse(ImportStage),
    /// Handing each manufacturer file in the archive to the product
    /// database. Counted: the file list is complete before the loop
    /// starts.
    IngestManufacturerData,
    /// Ingesting `knx_master.xml`, the one file that describes the
    /// manufacturers themselves rather than a product.
    IngestMasterData,
    /// Filling this project's absent slots from the product database
    /// (ADR-0012).
    EnrichFromProductDatabase,
    /// Writing the opaque passthrough entries and the manufacturer
    /// manifest into the store.
    PersistOpaque,
}

impl LoadStage {
    /// A stable, lower-camel-case name for a caller that has to put this on
    /// a wire. A forwarded parser stage keeps the parser's own spelling —
    /// one vocabulary, not two that agree by hand.
    pub const fn as_str(self) -> &'static str {
        match self {
            LoadStage::Parse(stage) => stage.as_str(),
            LoadStage::IngestManufacturerData => "ingestManufacturerData",
            LoadStage::IngestMasterData => "ingestMasterData",
            LoadStage::EnrichFromProductDatabase => "enrichFromProductDatabase",
            LoadStage::PersistOpaque => "persistOpaque",
        }
    }
}

/// Told what a load is doing as it happens. Same contract as
/// [`ImportObserver`]: cheap, infallible, and never the reason an import
/// fails.
pub trait LoadObserver {
    fn stage(&self, stage: LoadStage);

    /// `completed` of `total` items within the current stage, both real
    /// counts.
    fn items(&self, completed: u64, total: u64) {
        let _ = (completed, total);
    }
}

/// The observer for a caller that wants none — what
/// [`crate::import_ets_project_with`] passes.
impl LoadObserver for () {
    fn stage(&self, _stage: LoadStage) {}
}

/// Presents a [`LoadObserver`] to `knx-etsproj` as an [`ImportObserver`],
/// wrapping each parser stage in [`LoadStage::Parse`] on the way through.
/// The whole of this crate's progress plumbing is this adapter plus five
/// `stage(..)` calls in `import.rs`.
pub(crate) struct ParseStages<'a>(pub(crate) &'a dyn LoadObserver);

impl ImportObserver for ParseStages<'_> {
    fn stage(&self, stage: ImportStage) {
        self.0.stage(LoadStage::Parse(stage));
    }

    fn items(&self, completed: u64, total: u64) {
        self.0.items(completed, total);
    }
}
