//! Typed, payload-free facts of a failure inside the final System-2 Disconnect step.
//!
//! Built from the typed classification Net attaches where the failure arises, by an explicit
//! match. Never derived from Display/Debug text, a completed-step label or a guessed cause.
use crate::property_validation::{FailureCode, FailureDiagnosticRun};
use crate::property_validation_bundle::invalid;
use knx_net::commissioning::{
    final_step::{FinalStepFailure, FinalStepKind, FinalStepStage as NetStage},
    property_download::PropertySimulationError,
};
use serde::{Deserialize, Deserializer, Serialize};
use std::io;

/// What kind of operation of the final step failed (Net decides this where it fails).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FinalStepCategory {
    /// A comparison the final step performs disagreed with the expected value.
    VerificationMismatch,
    /// A read the final step needs did not produce a usable answer.
    ReadFailed,
    /// The final write (restoring Verify Mode) did not complete.
    WriteFailed,
}
/// Which operation of the final step was in flight.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FinalStepStage {
    Limit,
    Allocation,
    LoadedState,
    Pointer,
    ProgramVersion,
    VerifyModeRead,
    VerifyModeWrite,
    VerifyModeReadback,
}
/// Private journal form, stored inside `Failed`/`RestoreFailed`; the run is the variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FinalStepRecord {
    pub category: FinalStepCategory,
    pub stage: FinalStepStage,
}
impl FinalStepRecord {
    /// The classification Net attached to the stop. `None` when Net classified nothing, and
    /// when the combination contradicts the failure code (nothing is invented).
    pub fn from_error(error: &PropertySimulationError, code: FailureCode) -> Option<Self> {
        let FinalStepFailure { kind, stage } = error.final_step?;
        let record = Self {
            category: match kind {
                FinalStepKind::VerificationMismatch => FinalStepCategory::VerificationMismatch,
                FinalStepKind::ReadFailed => FinalStepCategory::ReadFailed,
                FinalStepKind::WriteFailed => FinalStepCategory::WriteFailed,
            },
            stage: match stage {
                NetStage::Limit => FinalStepStage::Limit,
                NetStage::Allocation => FinalStepStage::Allocation,
                NetStage::LoadedState => FinalStepStage::LoadedState,
                NetStage::Pointer => FinalStepStage::Pointer,
                NetStage::ProgramVersion => FinalStepStage::ProgramVersion,
                NetStage::VerifyModeRead => FinalStepStage::VerifyModeRead,
                NetStage::VerifyModeWrite => FinalStepStage::VerifyModeWrite,
                NetStage::VerifyModeReadback => FinalStepStage::VerifyModeReadback,
            },
        };
        record.validate(code, false).ok().map(|()| record)
    }
    /// Category, stage and failure code must agree as Net produces them, and a PID 7 record
    /// may sit beside a final-step record only for the final pointer read.
    pub(crate) fn validate(&self, code: FailureCode, with_pointer_record: bool) -> io::Result<()> {
        use FinalStepCategory as C;
        use FinalStepStage as S;
        let stage_fits = match self.category {
            C::VerificationMismatch => matches!(
                self.stage,
                S::Allocation | S::LoadedState | S::ProgramVersion | S::VerifyModeReadback
            ),
            C::ReadFailed => !matches!(self.stage, S::VerifyModeWrite),
            C::WriteFailed => self.stage == S::VerifyModeWrite,
        };
        let code_fits = match self.category {
            C::VerificationMismatch => code == FailureCode::IdentityOrRecovery,
            C::WriteFailed => code == FailureCode::Executor,
            C::ReadFailed => matches!(
                code,
                FailureCode::Executor | FailureCode::IdentityOrRecovery
            ),
        };
        let pointer_fits = !with_pointer_record
            || (self.category == C::ReadFailed
                && self.stage == S::Pointer
                && code == FailureCode::IdentityOrRecovery);
        if !(stage_fits && code_fits && pointer_fits) {
            return Err(invalid("inconsistent typed final-step failure record"));
        }
        Ok(())
    }
    pub(crate) fn diagnostic(&self, run: FailureDiagnosticRun) -> FinalStepDiagnostic {
        FinalStepDiagnostic {
            run,
            category: self.category,
            stage: self.stage,
        }
    }
}
/// Journal key reader: an absent key is `None` (via `default`), a present key must hold a
/// record. Explicit `null` is refused, so no checkpoint carries an empty key.
pub(crate) fn present_record<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<FinalStepRecord>, D::Error> {
    FinalStepRecord::deserialize(deserializer).map(Some)
}
/// Shareable projection: the run it belongs to, its category and its stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FinalStepDiagnostic {
    pub run: FailureDiagnosticRun,
    pub category: FinalStepCategory,
    pub stage: FinalStepStage,
}
