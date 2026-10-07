//! `/api/achievements` — reading, merging and resetting the achievements record.
//!
//! Three routes over `crate::achievements` (ADR-0089):
//!
//! * `GET /api/achievements` — the record this session shows, plus an
//!   honest account of a file that was refused or moved aside.
//! * `POST /api/achievements/record` — a *delta*, merged into the file: an
//!   unlock keeps its earliest time, a counter its highest value, and
//!   nothing is ever removed. Refused (409, file untouched) while the file
//!   on disk is from a newer build.
//! * `POST /api/achievements/reset` — the one way back: the current file is
//!   moved aside, never deleted, and tracking starts from nothing.
//!
//! All three sit behind the same guard as the rest of `/api/` (ADR-0026),
//! and none accepts a path: there is one record and `AppState::data_dir`
//! says where it is.

use std::collections::BTreeMap;

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Serialize;

use crate::achievements::{
    self, AchievementDelta, AchievementRecord, AchievementsLoad, ACHIEVEMENTS_SCHEMA_VERSION,
};
use crate::errors::ApiError;
use crate::session_log::{self, LogEntry, Severity};
use crate::SharedState;

pub fn achievement_routes() -> Router<SharedState> {
    Router::new()
        .route("/api/achievements", get(read_achievements))
        .route("/api/achievements/record", post(record_achievements))
        .route("/api/achievements/reset", post(reset_achievements))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
enum AchievementsStatus {
    Ok,
    Absent,
    RefusedNewer,
    Quarantined,
}

/// The body every route here answers with.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AchievementsDto {
    /// Always this build's version, including when the file says otherwise.
    schema_version: u32,
    unlocked: BTreeMap<String, String>,
    progress: BTreeMap<String, u64>,
    status: AchievementsStatus,
    /// The version a refused file claims.
    #[serde(skip_serializing_if = "Option::is_none")]
    file_schema_version: Option<u32>,
    /// Where a damaged or reset file went: a name relative to the data
    /// directory, never a host path (`crate::paths`).
    #[serde(skip_serializing_if = "Option::is_none")]
    moved_to: Option<String>,
    /// Plain English for the user, for every status but `ok` and `absent`.
    /// Untranslated, like `settings_routes`'s (KNOWN_LIMITATIONS §122).
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

fn file_name(path: &std::path::Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

impl AchievementsDto {
    fn new(record: AchievementRecord, status: AchievementsStatus) -> Self {
        Self {
            schema_version: ACHIEVEMENTS_SCHEMA_VERSION,
            unlocked: record.unlocked,
            progress: record.progress,
            status,
            file_schema_version: None,
            moved_to: None,
            message: None,
        }
    }

    fn from_load(load: &AchievementsLoad) -> Self {
        match load {
            AchievementsLoad::Absent => Self::new(load.record(), AchievementsStatus::Absent),
            AchievementsLoad::Current(record) => Self::new(record.clone(), AchievementsStatus::Ok),
            AchievementsLoad::RefusedNewer { file_version } => {
                let mut dto = Self::new(load.record(), AchievementsStatus::RefusedNewer);
                dto.file_schema_version = Some(*file_version);
                dto.message = Some(format!(
                    "The achievements file is schema version {file_version}, which this build \
                     (version {ACHIEVEMENTS_SCHEMA_VERSION}) cannot read. It was left untouched; \
                     achievements are not tracked in this session."
                ));
                dto
            }
            AchievementsLoad::Quarantined { moved_to, detail } => {
                let name = file_name(moved_to);
                let mut dto = Self::new(load.record(), AchievementsStatus::Quarantined);
                dto.message = Some(format!(
                    "The achievements file {detail}. It was moved to \"{name}\" in the data \
                     directory and tracking starts over."
                ));
                dto.moved_to = Some(name);
                dto
            }
        }
    }

    /// The same account in the session log, so it shows in the Log panel
    /// and in a debug report without the frontend inventing an entry.
    fn log(&self, state: &SharedState) {
        if let Some(message) = self.message.clone() {
            log(state, Severity::Warning, message);
        }
    }
}

fn log(state: &SharedState, severity: Severity, message: String) {
    state
        .session_log
        .lock()
        .expect("state mutex poisoned")
        .push(LogEntry {
            timestamp: session_log::now(),
            severity,
            source: "achievements".to_string(),
            message,
            location: None,
            detail: None,
            diagnostic: None,
        });
}

fn read(state: &SharedState) -> Result<AchievementsLoad, ApiError> {
    achievements::load(&state.data_dir)
        .map_err(|e| ApiError::internal(format!("achievements file unusable: {e}")))
}

/// Reads the record. Takes the write lock because this read is not one:
/// a damaged file is moved aside on the way.
async fn read_achievements(
    State(state): State<SharedState>,
) -> Result<Json<AchievementsDto>, ApiError> {
    let _guard = state
        .achievements_lock
        .lock()
        .expect("state mutex poisoned");
    let load = read(&state)?;
    let dto = AchievementsDto::from_load(&load);
    dto.log(&state);
    Ok(Json(dto))
}

/// Merges a delta into the file and answers with the merged record.
async fn record_achievements(
    State(state): State<SharedState>,
    Json(delta): Json<AchievementDelta>,
) -> Result<Json<AchievementsDto>, ApiError> {
    achievements::validate(&delta).map_err(ApiError::bad_request)?;

    // One writer at a time, so two windows reporting at once both land.
    let _guard = state
        .achievements_lock
        .lock()
        .expect("state mutex poisoned");
    let load = read(&state)?;
    if load.blocks_writes() {
        let dto = AchievementsDto::from_load(&load);
        dto.log(&state);
        return Err(ApiError::with_status(
            StatusCode::CONFLICT,
            dto.message
                .unwrap_or_else(|| "the achievements file was written by a newer build".into()),
        ));
    }
    if matches!(load, AchievementsLoad::Quarantined { .. }) {
        AchievementsDto::from_load(&load).log(&state);
    }

    let mut record = load.record();
    achievements::merge(&mut record, delta).map_err(ApiError::bad_request)?;
    achievements::store(&state.data_dir, &record)
        .map_err(|e| ApiError::internal(format!("achievements file could not be written: {e}")))?;
    Ok(Json(AchievementsDto::new(record, AchievementsStatus::Ok)))
}

/// Moves the record aside and starts over.
async fn reset_achievements(
    State(state): State<SharedState>,
) -> Result<Json<AchievementsDto>, ApiError> {
    let _guard = state
        .achievements_lock
        .lock()
        .expect("state mutex poisoned");
    let moved = achievements::reset(&state.data_dir).map_err(|e| {
        ApiError::internal(format!("achievements file could not be moved aside: {e}"))
    })?;
    let mut dto = AchievementsDto::new(AchievementRecord::default(), AchievementsStatus::Absent);
    if let Some(moved) = moved {
        let name = file_name(&moved);
        log(
            &state,
            Severity::Info,
            format!(
                "Achievements were reset. The previous record was kept as \"{name}\" in the data \
                 directory."
            ),
        );
        dto.moved_to = Some(name);
    }
    Ok(Json(dto))
}
