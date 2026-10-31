//! Project provenance and reviewed-content wire types for diagnostic bundle collection.

use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProjectContext {
    pub server_incarnation: String,
    pub project_incarnation: u64,
    pub revision: u64,
}

impl ProjectContext {
    /// Sample while holding project: replacement publishes counters before unlocking it.
    pub(crate) fn locked(state: &crate::AppState) -> Self {
        Self {
            server_incarnation: state.server_incarnation.clone(),
            project_incarnation: state.project_incarnation.load(Ordering::Relaxed),
            revision: state.project_revision.load(Ordering::Relaxed),
        }
    }

    pub(crate) fn capture(state: &crate::AppState) -> Self {
        let _project = state.project.lock().expect("state mutex poisoned");
        Self::locked(state)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Review {
    pub fingerprint: String,
    pub context: ProjectContext,
}
