//! `/api/settings` — reading, patching and one-time adoption of the application settings file.
//!
//! Three routes over `crate::settings`:
//!
//! * `GET /api/settings` — what this session should run on, plus an honest
//!   account of how it got there (migrated, refused, quarantined).
//! * `PUT /api/settings` — a *patch*, not a replacement: the keys it names
//!   are written, a `null` removes one, and every key it does not mention
//!   — including one this build has never heard of — is left alone.
//! * `POST /api/settings/adopt` — the one-time handover of preferences
//!   that still live in a browser's `localStorage`, accepted only while
//!   no settings file exists, so it can never overwrite a real record.
//!
//! All three sit behind the same guard as the rest of `/api/` (ADR-0026);
//! none of them accepts a path from the client, because there is exactly
//! one settings file and `AppState::data_dir` says where it is.

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::errors::ApiError;
use crate::session_log::{self, LogEntry, Severity};
use crate::settings::{
    self, Preferences, SettingsDocument, SettingsLoad, BROWSER_ERA_SCHEMA_VERSION,
    CURRENT_SCHEMA_VERSION,
};
use crate::SharedState;

pub fn settings_routes() -> Router<SharedState> {
    Router::new()
        .route("/api/settings", get(read_settings).put(patch_settings))
        .route("/api/settings/adopt", axum::routing::post(adopt_settings))
}

/// How the settings this session runs on were arrived at. `ok` and
/// `absent` are the two quiet ones; the other three each mean a file on
/// disk did not match this build, and each carries the detail the
/// frontend needs to say so out loud.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
enum SettingsStatus {
    Ok,
    Absent,
    Migrated,
    RefusedNewer,
    Quarantined,
}

/// The body every route here answers with, so a client has one shape to
/// parse whether it just read, wrote or adopted.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SettingsDto {
    /// The version of the document in `settings` — always this build's
    /// current version, including when the file on disk says otherwise.
    schema_version: u32,
    settings: Preferences,
    status: SettingsStatus,
    /// The version the *file* claims, when that differs from
    /// `schemaVersion`: the version migrated away from, or the future one
    /// that was refused.
    #[serde(skip_serializing_if = "Option::is_none")]
    file_schema_version: Option<u32>,
    /// The name a quarantined file was moved to, relative to the data
    /// directory. A name, never a host path — the browser has no business
    /// learning the server's filesystem layout (`crate::paths`).
    #[serde(skip_serializing_if = "Option::is_none")]
    moved_to: Option<String>,
    /// Plain English for the user, set for every status but `ok` and
    /// `absent`. English on purpose: it quotes a server-side event, and
    /// the frontend's catalogues translate the sentence around it.
    #[serde(skip_serializing_if = "Option::is_none")]
    notice: Option<String>,
}

impl SettingsDto {
    fn from_load(load: &SettingsLoad) -> Self {
        let document = load.document();
        let mut dto = Self {
            schema_version: document.version,
            settings: document.preferences,
            status: SettingsStatus::Ok,
            file_schema_version: None,
            moved_to: None,
            notice: None,
        };
        match load {
            SettingsLoad::Current(_) => {}
            SettingsLoad::Absent => dto.status = SettingsStatus::Absent,
            SettingsLoad::Migrated { from, .. } => {
                dto.status = SettingsStatus::Migrated;
                dto.file_schema_version = Some(*from);
                dto.notice = Some(format!(
                    "Settings were migrated from schema version {from} to {CURRENT_SCHEMA_VERSION}."
                ));
            }
            SettingsLoad::RefusedNewer { file_version } => {
                dto.status = SettingsStatus::RefusedNewer;
                dto.file_schema_version = Some(*file_version);
                dto.notice = Some(format!(
                    "The settings file is schema version {file_version}, which this build \
                     (version {CURRENT_SCHEMA_VERSION}) cannot read. It was left untouched and \
                     this session is running on defaults."
                ));
            }
            SettingsLoad::Quarantined { moved_to, reason } => {
                let name = moved_to
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                dto.status = SettingsStatus::Quarantined;
                dto.notice = Some(format!(
                    "The settings file {reason}. It was moved to \"{name}\" in the data \
                     directory and this session is running on defaults."
                ));
                dto.moved_to = Some(name);
            }
        }
        dto
    }

    /// The same account, in the session log — so the user sees it in the
    /// log panel without the frontend having to invent an entry, and so a
    /// developer reading a debug report sees it too.
    fn log(&self, state: &SharedState, source: &str) {
        let Some(notice) = self.notice.clone() else {
            return;
        };
        let severity = match self.status {
            SettingsStatus::Migrated => Severity::Info,
            _ => Severity::Warning,
        };
        state
            .session_log
            .lock()
            .expect("state mutex poisoned")
            .push(LogEntry {
                timestamp: session_log::now(),
                severity,
                source: source.to_string(),
                message: notice,
                location: None,
                detail: None,
            });
    }
}

fn read(state: &SharedState) -> Result<SettingsLoad, ApiError> {
    settings::load(&state.data_dir)
        .map_err(|e| ApiError::internal(format!("settings file unusable: {e}")))
}

fn write(state: &SharedState, document: &SettingsDocument) -> Result<(), ApiError> {
    settings::store(&state.data_dir, document)
        .map_err(|e| ApiError::internal(format!("settings file could not be written: {e}")))
}

/// Reads the settings file, repairing it where [`settings::load`] can, and
/// reports what happened. Answers with defaults rather than an error for
/// every case a user can find themselves in — no file yet, a file from the
/// future, a file a disk ate — because a preferences read that 500s would
/// take the whole first paint down with it.
async fn read_settings(State(state): State<SharedState>) -> Result<Json<SettingsDto>, ApiError> {
    let load = read(&state)?;
    let dto = SettingsDto::from_load(&load);
    dto.log(&state, "settings");
    Ok(Json(dto))
}

#[derive(Debug, Deserialize)]
struct PatchRequest {
    settings: Preferences,
}

/// Merges a patch into the file. Refuses — 409, file untouched — when the
/// file on disk is newer than this build: the alternative is overwriting
/// preferences this build cannot even read, which is the one outcome the
/// version check exists to prevent.
async fn patch_settings(
    State(state): State<SharedState>,
    Json(request): Json<PatchRequest>,
) -> Result<Json<SettingsDto>, ApiError> {
    // One writer at a time: two windows changing two different preferences
    // at the same moment would otherwise each write back the document they
    // read, and the slower one would quietly undo the faster one.
    let _guard = state.settings_lock.lock().expect("state mutex poisoned");

    let load = read(&state)?;
    if load.blocks_writes() {
        let dto = SettingsDto::from_load(&load);
        dto.log(&state, "settings");
        return Err(ApiError::with_status(
            StatusCode::CONFLICT,
            dto.notice
                .unwrap_or_else(|| "the settings file was written by a newer build".to_string()),
        ));
    }

    let mut document = load.document();
    settings::apply_patch(&mut document.preferences, request.settings);
    write(&state, &document)?;

    let mut dto = SettingsDto::from_load(&load);
    dto.settings = document.preferences;
    dto.log(&state, "settings");
    Ok(Json(dto))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AdoptRequest {
    /// The version the payload is written against — normally
    /// [`BROWSER_ERA_SCHEMA_VERSION`], since what is being adopted is a
    /// browser's `localStorage`. Explicit rather than assumed, so the
    /// migration chain has something real to start from.
    schema_version: u32,
    settings: Preferences,
}

/// Takes over preferences that still live in a browser, exactly once.
///
/// Conditional on the settings file not existing: a second browser, or a
/// second tab that lost the race, gets a 409 and keeps whatever the file
/// already says. Without that condition, every fresh browser profile would
/// stamp its own defaults over a record the user had been curating on the
/// server for months.
async fn adopt_settings(
    State(state): State<SharedState>,
    Json(request): Json<AdoptRequest>,
) -> Result<Json<SettingsDto>, ApiError> {
    if request.schema_version > CURRENT_SCHEMA_VERSION {
        return Err(ApiError::bad_request(format!(
            "cannot adopt settings at schema version {} — this build writes {CURRENT_SCHEMA_VERSION}",
            request.schema_version
        )));
    }

    let _guard = state.settings_lock.lock().expect("state mutex poisoned");
    let load = read(&state)?;
    if !matches!(load, SettingsLoad::Absent) {
        return Err(ApiError::with_status(
            StatusCode::CONFLICT,
            "a settings file already exists; there is nothing to adopt into",
        ));
    }

    let document = settings::migrated_document(request.settings, request.schema_version);
    write(&state, &document)?;

    let migrated_from_browser = request.schema_version == BROWSER_ERA_SCHEMA_VERSION;
    let mut dto = SettingsDto::from_load(&SettingsLoad::Current(document));
    if migrated_from_browser {
        dto.status = SettingsStatus::Migrated;
        dto.file_schema_version = Some(BROWSER_ERA_SCHEMA_VERSION);
        dto.notice = Some(
            "Preferences that were stored in this browser have been adopted into the \
             application settings file."
                .to_string(),
        );
    }
    dto.log(&state, "settings");
    Ok(Json(dto))
}
