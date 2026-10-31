//! Closed, payload-free metadata projection of a private System-2 journal; never raw bytes.
//!
//! One strict `OperationStore::open` yields the original minimized progress and these facts
//! together. Recorded comparison bytes are withheld: nothing here proves them credential-free.
//! Only [`PrivateProjection`] serializes; the outer [`PrivateSnapshot`] deliberately cannot.
use super::{Binding, FailureDiagnosticRun, OperationProgress, OperationStore};
use serde::Serialize;
use std::path::Path;

/// Format of the serialized projection; bumps with any schema change.
pub const PROJECTION_FORMAT: u16 = 1;
/// Fixed `kind` of the serialized projection.
pub const PROJECTION_KIND: &str = "knxbench-system2-private-projection";

/// Why recorded comparison bytes are not exported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum WithheldReason {
    /// Replay validates the recorded comparison, not that arbitrary memory is credential-free.
    UnclassifiedMemory,
}
/// What the export loses by withholding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ExportedLoss {
    /// The journal holds the compared bytes; this export carries none of them.
    RecordedBytesNotExported,
}
/// What is known of one run's final allocation mismatch; no variant carries payload facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "classification", rename_all = "camelCase")]
pub enum AllocationMismatchClass {
    /// A typed mismatch whose comparison record exists in the private journal.
    RecordedBytesWithheld {
        reason: WithheldReason,
        loss: ExportedLoss,
    },
    /// A typed mismatch whose checkpoint carries no comparison record. Not writer-age
    /// inference and not evidence that a read never happened.
    NotRecorded,
    /// Neither a typed mismatch nor a typed witness of absence. Not proven absence.
    Unclassified,
}
/// One run's metadata; runs appear download first, restore only after a recorded restore start.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunProjection {
    pub run: FailureDiagnosticRun,
    pub allocation_mismatch: AllocationMismatchClass,
}
/// The only serializable part: closed enums and fixed constants, deterministic, no identities.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivateProjection {
    pub format: u16,
    pub kind: &'static str,
    pub runs: Vec<RunProjection>,
}

/// Generic refusal: carries no path, error text, source identity or progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unavailable;

/// Progress and metadata of one successful open. Deliberately not `Serialize`: the progress
/// part is private status, only the projection part may leave the process.
#[derive(Debug)]
pub struct AvailableSnapshot {
    progress: OperationProgress,
    projection: PrivateProjection,
}
impl AvailableSnapshot {
    /// The original minimized progress of the same store the projection came from.
    pub fn progress(&self) -> &OperationProgress {
        &self.progress
    }
    /// The closed metadata projection.
    pub fn projection(&self) -> &PrivateProjection {
        &self.projection
    }
    /// Owned separate parts.
    pub fn into_parts(self) -> (OperationProgress, PrivateProjection) {
        (self.progress, self.projection)
    }
}
/// Outcome of one strict read; `Unavailable` is the same for every refusal cause.
#[derive(Debug)]
pub enum PrivateSnapshot {
    Available(Box<AvailableSnapshot>),
    Unavailable(Unavailable),
}
impl PrivateSnapshot {
    pub fn progress(&self) -> Option<&OperationProgress> {
        match self {
            Self::Available(snapshot) => Some(snapshot.progress()),
            Self::Unavailable(_) => None,
        }
    }
    pub fn projection(&self) -> Option<&PrivateProjection> {
        match self {
            Self::Available(snapshot) => Some(snapshot.projection()),
            Self::Unavailable(_) => None,
        }
    }
}

/// Owned projection bytes admitted for diagnostic-bundle consumers.
///
/// The boundary deliberately implements neither `Serialize` nor `Debug`:
///
/// ```compile_fail
/// fn require_serialize<T: serde::Serialize>() {}
/// require_serialize::<
///     knx_app::property_validation::private_projection::AdmittedPrivateProjection,
/// >();
/// ```
///
/// ```compile_fail
/// fn require_debug<T: std::fmt::Debug>() {}
/// require_debug::<
///     knx_app::property_validation::private_projection::AdmittedPrivateProjection,
/// >();
/// ```
pub struct AdmittedPrivateProjection {
    bytes: Vec<u8>,
}
impl AdmittedPrivateProjection {
    /// The exact owned bytes admitted from the consumed snapshot.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Consumes one available snapshot at the private diagnostic-bundle boundary.
pub fn admit_snapshot(
    snapshot: AvailableSnapshot,
) -> Result<(OperationProgress, AdmittedPrivateProjection), Unavailable> {
    let (progress, projection) = snapshot.into_parts();
    if projection.format != PROJECTION_FORMAT || projection.kind != PROJECTION_KIND {
        return Err(Unavailable);
    }
    let ordered = match projection.runs.as_slice() {
        [download] => download.run == FailureDiagnosticRun::Download && !progress.restore_attempted,
        [download, restore] => {
            download.run == FailureDiagnosticRun::Download
                && restore.run == FailureDiagnosticRun::Restore
                && progress.restore_attempted
        }
        _ => false,
    };
    if !ordered
        || !projection.runs.iter().all(|run| {
            matches!(
                run.allocation_mismatch,
                AllocationMismatchClass::RecordedBytesWithheld {
                    reason: WithheldReason::UnclassifiedMemory,
                    loss: ExportedLoss::RecordedBytesNotExported,
                } | AllocationMismatchClass::NotRecorded
                    | AllocationMismatchClass::Unclassified
            )
        })
    {
        return Err(Unavailable);
    }
    let bytes = serde_json::to_vec(&projection).map_err(|_| Unavailable)?;
    if bytes.len() > 4096 {
        return Err(Unavailable);
    }
    Ok((progress, AdmittedPrivateProjection { bytes }))
}

/// Strictly reopen `directory` against `expected` and project it.
///
/// Admission is exactly `OperationStore::open`: no preflight, so no source-size budget is
/// added here and none is claimed.
pub fn read_snapshot(directory: &Path, expected: &Binding) -> PrivateSnapshot {
    let store = match OperationStore::open(directory, expected) {
        Ok(store) => store,
        Err(_) => return PrivateSnapshot::Unavailable(Unavailable),
    };

    let run_projection = |run| RunProjection {
        run,
        allocation_mismatch: match store
            .state
            .allocation_mismatches
            .iter()
            .find(|(recorded_run, _)| *recorded_run == run)
        {
            Some((_, Some(_))) => AllocationMismatchClass::RecordedBytesWithheld {
                reason: WithheldReason::UnclassifiedMemory,
                loss: ExportedLoss::RecordedBytesNotExported,
            },
            Some((_, None)) => AllocationMismatchClass::NotRecorded,
            None => AllocationMismatchClass::Unclassified,
        },
    };

    let mut runs = vec![run_projection(FailureDiagnosticRun::Download)];
    if store.state.restore_attempted {
        runs.push(run_projection(FailureDiagnosticRun::Restore));
    }

    let progress = store.progress();
    let projection = PrivateProjection {
        format: PROJECTION_FORMAT,
        kind: PROJECTION_KIND,
        runs,
    };
    PrivateSnapshot::Available(Box::new(AvailableSnapshot {
        progress,
        projection,
    }))
}

#[cfg(test)]
#[path = "property_validation_private_admission_tests.rs"]
mod admission_tests;
#[cfg(test)]
#[path = "property_validation_private_projection_tests.rs"]
mod tests;
