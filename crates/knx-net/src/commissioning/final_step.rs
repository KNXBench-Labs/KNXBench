//! Closed, payload-free classification of a failure inside the final System-2 Disconnect step.

use super::final_step_evidence::AllocationMismatchEvidence;
use super::property_download::PropertySimulationFailure;
use super::property_recovery::invalid;

/// What kind of operation of the final step failed. Decided where the failure arises, from the
/// operation in flight, never from an error message or the label of a completed step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinalStepKind {
    /// A comparison the final step performs disagreed with the expected value.
    VerificationMismatch,
    /// A read the final step needs did not produce a usable answer.
    ReadFailed,
    /// The final write (restoring Verify Mode) did not complete.
    WriteFailed,
}

/// The operation of the final step that was in flight. Order follows the executor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinalStepStage {
    /// Reading the device's maximum APDU length before the memory comparison.
    Limit,
    /// Reading back and comparing the full memory allocations.
    Allocation,
    /// Reading and comparing each resource's final Loaded state.
    LoadedState,
    /// Reading each resource's final table reference.
    Pointer,
    /// Reading and comparing the final program version.
    ProgramVersion,
    /// Reading Device Control before restoring Verify Mode.
    VerifyModeRead,
    /// Writing Device Control to restore Verify Mode.
    VerifyModeWrite,
    /// Reading Device Control back after the restore write.
    VerifyModeReadback,
}

/// A final-step failure: what kind of operation, and which one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FinalStepFailure {
    pub kind: FinalStepKind,
    pub stage: FinalStepStage,
}

/// A final-step stop that keeps the actual failure beside its classification, and for an
/// allocation mismatch the compared bytes (kept apart so the classification stays `Copy`).
pub(crate) struct FinalStepError {
    pub(crate) failure: PropertySimulationFailure,
    pub(crate) classified: FinalStepFailure,
    pub(crate) evidence: Option<Box<AllocationMismatchEvidence>>,
}
impl FinalStepError {
    fn new(failure: PropertySimulationFailure, kind: FinalStepKind, stage: FinalStepStage) -> Self {
        Self {
            failure,
            classified: FinalStepFailure { kind, stage },
            evidence: None,
        }
    }
    /// An operation that reads: any error it returns, including a short or refused answer.
    pub(crate) fn read<E: Into<PropertySimulationFailure>>(
        stage: FinalStepStage,
    ) -> impl FnOnce(E) -> Self {
        move |error| Self::new(error.into(), FinalStepKind::ReadFailed, stage)
    }
    /// The one operation that writes.
    pub(crate) fn write<E: Into<PropertySimulationFailure>>(
        stage: FinalStepStage,
    ) -> impl FnOnce(E) -> Self {
        move |error| Self::new(error.into(), FinalStepKind::WriteFailed, stage)
    }
    /// A completed read whose value disagreed with the expectation.
    pub(crate) fn mismatch(stage: FinalStepStage, reason: &str) -> Self {
        Self::new(invalid(reason), FinalStepKind::VerificationMismatch, stage)
    }
    /// The same mismatch, carrying the bytes that decided it.
    pub(crate) fn with_evidence(mut self, evidence: AllocationMismatchEvidence) -> Self {
        self.evidence = Some(Box::new(evidence));
        self
    }
}
