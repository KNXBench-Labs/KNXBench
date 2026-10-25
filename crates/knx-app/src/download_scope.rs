//! A download scope request, explicit or "what is pending", resolved before any plan is shown.
//!
//! [`ScopeRequest::Changes`] asks for the scope the device's programming
//! flags advise ([`ScopeAdvice`]). It is resolved here, once, to a concrete
//! complete or partial download, and the plan shown is the plan of that
//! concrete scope: approval and execution see a fixed scope, never "changes".
//! Nothing pending prepares nothing: no image, no plan, no complete
//! download in its place. A partial scope whose plan cannot be derived is
//! refused as it is, never widened to the complete download.
//!
//! The flags are the project's belief. The partial plan's own preconditions
//! check the device before the first write
//! ([`knx_core::commissioning::partial_memory_download`]).

use std::fmt;

use knx_core::commissioning::partial_memory_download::{PartialDownloadParts, PartialPlanError};
use knx_core::commissioning::scope_advice::ScopeAdvice;
use knx_core::{IndividualAddress, Project};
use knx_productdb::Connection;

use crate::device_download::{device_at, prepare_device_download, PrepareError, PreparedDownload};

/// What the operator asked to write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeRequest {
    /// The complete download, whatever the flags say.
    Complete,
    /// Exactly this partial download, whatever the flags say.
    Partial(PartialDownloadParts),
    /// Whatever the device's programming flags advise.
    Changes,
}

impl ScopeRequest {
    /// The concrete download this request asks for under `advice`: `None`
    /// for none, `Some(None)` for the complete one, `Some(Some(parts))` for
    /// a partial one. Only [`ScopeRequest::Changes`] reads the advice.
    pub fn resolve(self, advice: &ScopeAdvice) -> Option<Option<PartialDownloadParts>> {
        match self {
            ScopeRequest::Complete => Some(None),
            ScopeRequest::Partial(parts) => Some(Some(parts)),
            ScopeRequest::Changes => advice.download(),
        }
    }
}

/// A request resolved from the project alone, before any product data is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedScope {
    /// What the device's flags advise.
    pub advice: ScopeAdvice,
    /// The concrete scope, as [`ScopeRequest::resolve`] gives it.
    pub resolved: Option<Option<PartialDownloadParts>>,
}

/// A request resolved and, unless nothing was to be written, prepared.
#[derive(Debug)]
pub struct ScopedDownload {
    /// What the device's flags advise; reported for every request.
    pub advice: ScopeAdvice,
    /// The concrete scope, as [`ScopeRequest::resolve`] gives it.
    pub resolved: Option<Option<PartialDownloadParts>>,
    /// The download of `resolved`; `None` exactly when `resolved` is.
    pub prepared: Option<PreparedDownload>,
}

/// Why a scoped download was not prepared.
#[derive(Debug)]
pub enum ScopedPrepareError {
    /// No download to the device could be prepared.
    Prepare(PrepareError),
    /// The partial scope could not be derived from the complete plan.
    Partial(PartialPlanError),
}

impl std::error::Error for ScopedPrepareError {}

impl fmt::Display for ScopedPrepareError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Prepare(error) => write!(f, "{error}"),
            Self::Partial(error) => write!(f, "{error}"),
        }
    }
}

/// The advice for the one project device at `target`.
pub fn advise_device_download(
    project: &Project,
    target: IndividualAddress,
) -> Result<ScopeAdvice, PrepareError> {
    device_at(project, target).map(ScopeAdvice::for_device)
}

/// Resolves `request` for the device at `target` from `project` alone: a
/// caller can answer "nothing pending" without any product data.
pub fn resolve_scope(
    project: &Project,
    target: IndividualAddress,
    request: ScopeRequest,
) -> Result<ResolvedScope, PrepareError> {
    let advice = advise_device_download(project, target)?;
    Ok(ResolvedScope {
        advice,
        resolved: request.resolve(&advice),
    })
}

/// Resolves `request` for the device at `target` and prepares the download
/// it resolves to; nothing at all when it resolves to none. Advice and plan
/// come from the one `project` borrowed here: a caller that holds it under
/// one lock gets both from one snapshot.
pub fn prepare_scoped_download(
    conn: &Connection,
    project: &Project,
    target: IndividualAddress,
    request: ScopeRequest,
) -> Result<ScopedDownload, ScopedPrepareError> {
    let ResolvedScope { advice, resolved } =
        resolve_scope(project, target, request).map_err(ScopedPrepareError::Prepare)?;
    let prepared = match resolved {
        None => None,
        Some(scope) => {
            let complete = prepare_device_download(conn, project, target)
                .map_err(ScopedPrepareError::Prepare)?;
            Some(match scope {
                None => complete,
                Some(parts) => complete
                    .into_partial(parts)
                    .map_err(ScopedPrepareError::Partial)?,
            })
        }
    };
    Ok(ScopedDownload {
        advice,
        resolved,
        prepared,
    })
}
