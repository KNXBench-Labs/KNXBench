//! Typed download-result facts derived only from the validated journal state.
use serde::Serialize;

/// What a System-2 download run provably achieved; never a loaded or clean claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DownloadResult {
    /// The run has not ended.
    InProgress,
    /// The run ended and no write could have been sent.
    NothingWritten,
    /// A write may have been sent and some planned transfer steps never completed.
    TransferIncomplete,
    /// Every planned transfer step completed; the final verification step never started.
    TransferCompleteVerificationNotStarted,
    /// Every planned transfer step completed; the final verification step started but
    /// did not finish. The journal does not say whether it compared, timed out or refused.
    TransferCompleteVerificationUnfinished,
    /// Every step including the final verification finished.
    Verified,
    /// The journal cannot prove any of the above.
    Unknown,
}

use super::{FailureDiagnostic, LocalOutcome, OperationProgress};

/// The download run's own ended facts, frozen when it ends so a later recovery cannot rewrite them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct DownloadFacts {
    pub result: DownloadResult,
    pub outcome: LocalOutcome,
    pub completed_steps: usize,
    pub total_steps: Option<usize>,
}

impl DownloadResult {
    /// The result safe to publish for a progress snapshot: never `Verified` unless the
    /// download itself finished, and never `NothingWritten` when a write may have been sent.
    pub fn settled(outcome: LocalOutcome, mutation_possible: bool, claimed: Self) -> Self {
        let ended_fallback = if mutation_possible {
            Self::Unknown
        } else {
            Self::NothingWritten
        };
        match (outcome, claimed) {
            (LocalOutcome::Running, _) => Self::InProgress,
            (LocalOutcome::Finished, Self::Verified) => Self::Verified,
            (LocalOutcome::Finished, _) => Self::Unknown,
            (LocalOutcome::Failed | LocalOutcome::Cancelled, Self::Verified) => Self::Unknown,
            (_, Self::InProgress) => ended_fallback,
            (_, Self::NothingWritten) if mutation_possible => Self::Unknown,
            (_, other) => other,
        }
    }
}

/// Which build executed a run. Old operations carry no record: explicitly unknown, never
/// the exporting build.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum ExecutionBuild {
    Recorded { version: String },
    Unknown,
}

impl ExecutionBuild {
    /// Only a real build version is provenance: `MAJOR.MINOR.PATCH[-prerelease][+g<hex sha>]`,
    /// exactly what `GET /api/version` serves. Anything else (a path, free text, a truncated
    /// or padded value) is unknown, never sanitized into an invented record.
    pub fn recorded(version: &str) -> Self {
        if is_build_version(version) {
            Self::Recorded {
                version: version.to_owned(),
            }
        } else {
            Self::Unknown
        }
    }
}

/// The closed build-version vocabulary shared by the sidecar, the exporter and the client.
pub fn is_build_version(text: &str) -> bool {
    let (core, sha) = match text.split_once("+g") {
        Some((core, sha)) => (core, Some(sha)),
        None => (text, None),
    };
    let (numbers, pre) = match core.split_once('-') {
        Some((numbers, pre)) => (numbers, Some(pre)),
        None => (core, None),
    };
    let numeric = |part: &str| {
        !part.is_empty() && part.len() <= 9 && part.bytes().all(|b| b.is_ascii_digit())
    };
    text.len() <= 64
        && numbers.split('.').count() == 3
        && numbers.split('.').all(numeric)
        && pre.is_none_or(|pre| {
            !pre.is_empty()
                && pre
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-')
        })
        && sha.is_none_or(|sha| {
            (1..=40).contains(&sha.len())
                && sha.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
        })
}

fn outcome_text(outcome: LocalOutcome) -> &'static str {
    match outcome {
        LocalOutcome::Running => "running",
        LocalOutcome::Restoring => "restoring",
        LocalOutcome::Interrupted => "interrupted",
        LocalOutcome::Finished => "finished",
        LocalOutcome::Failed => "failed",
        LocalOutcome::Cancelled => "cancelled",
        LocalOutcome::Restored => "restored",
        LocalOutcome::RestoreFailed => "restoreFailed",
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DownloadSection {
    outcome: &'static str,
    result: DownloadResult,
    completed_steps: usize,
    total_steps: Option<usize>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RecoverySection {
    outcome: &'static str,
    completed_steps: usize,
    total_steps: Option<usize>,
}

/// Shareable summary of an ended System-2 run. Facts come from the validated journal only:
/// no address, name, parameter, backup byte, path, phrase or raw text. The download keeps
/// its own frozen outcome and counters; a recovery has its own section.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct System2Report {
    format: u16,
    kind: &'static str,
    support: &'static str,
    /// What the facts rest on; the report is not a device read.
    evidence: &'static str,
    download: DownloadSection,
    recovery: Option<RecoverySection>,
    mutation_possible: bool,
    prestate_differs: bool,
    failure_diagnostics: Vec<FailureDiagnostic>,
    /// Build that executed the run, when it was recorded; unknown for older operations.
    execution_build: ExecutionBuild,
    /// Build that wrote this report; never a claim about the execution.
    exporter_build: String,
}

impl System2Report {
    /// `None` while the run or its recovery is still running: nothing is final yet.
    pub fn from_progress(
        progress: &OperationProgress,
        execution_build: ExecutionBuild,
        exporter_build: &str,
    ) -> Option<Self> {
        if matches!(
            progress.outcome,
            LocalOutcome::Running | LocalOutcome::Restoring
        ) || progress.download.outcome == LocalOutcome::Running
        {
            return None;
        }
        let download = &progress.download;
        let recovery = progress.restore_attempted.then(|| RecoverySection {
            outcome: outcome_text(progress.outcome),
            completed_steps: progress.completed_steps,
            total_steps: progress.total_steps,
        });
        Some(Self {
            format: 2,
            kind: "knxbench-system2-download-summary",
            support: "EXPERIMENTAL BEST-EFFORT; HARDWARE COMPATIBILITY AND RECOVERY UNVERIFIED",
            evidence: "local-journal-only",
            download: DownloadSection {
                outcome: outcome_text(download.outcome),
                result: DownloadResult::settled(
                    download.outcome,
                    progress.mutation_possible,
                    download.result,
                ),
                completed_steps: download.completed_steps,
                total_steps: download.total_steps,
            },
            recovery,
            mutation_possible: progress.mutation_possible,
            prestate_differs: progress.prestate_differs,
            failure_diagnostics: progress.failure_diagnostics.clone(),
            execution_build,
            exporter_build: exporter_build
                .chars()
                .filter(char::is_ascii_graphic)
                .take(64)
                .collect(),
        })
    }
}
