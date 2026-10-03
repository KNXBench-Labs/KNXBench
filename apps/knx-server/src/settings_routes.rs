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
use crate::session_log::{
    self, LogEntry, SettingsDiagnostic, SettingsQuarantineReasonDto, Severity,
};
use crate::settings::{
    self, Preferences, SettingsDocument, SettingsLoad, SettingsQuarantineReason,
    BROWSER_ERA_SCHEMA_VERSION, CURRENT_SCHEMA_VERSION,
};
use crate::SharedState;

impl From<SettingsQuarantineReason> for SettingsQuarantineReasonDto {
    fn from(value: SettingsQuarantineReason) -> Self {
        match value {
            SettingsQuarantineReason::Unreadable => Self::Unreadable,
            SettingsQuarantineReason::InvalidJson => Self::InvalidJson,
            SettingsQuarantineReason::NotObject => Self::NotObject,
            SettingsQuarantineReason::MissingSchemaVersion => Self::MissingSchemaVersion,
            SettingsQuarantineReason::SettingsNotObject => Self::SettingsNotObject,
        }
    }
}

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
    /// Wire capability, independent of the unchanged opaque file schema.
    /// New clients must not assume old servers enforce expectedSettings.
    conditional_patch_version: u8,
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
    /// `absent`. Untranslated: the string is built here, and the frontend
    /// shows it as it stands — `console.warn` in `settingsStore.ts`, and
    /// the Log panel renders `message` verbatim. A German user therefore
    /// reads an English sentence. Recorded as a gap in
    /// `docs/KNOWN_LIMITATIONS.md` §122; the settings surface (T10) owns
    /// the fix, because translating this needs a machine-readable code
    /// plus catalogue keys, and that shape belongs with the panel that
    /// will display it rather than beside it.
    #[serde(skip_serializing_if = "Option::is_none")]
    diagnostic: Option<SettingsDiagnostic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

impl SettingsDto {
    fn from_load(load: &SettingsLoad) -> Self {
        let document = load.document();
        let mut dto = Self {
            conditional_patch_version: 1,
            schema_version: document.version,
            settings: document.preferences,
            status: SettingsStatus::Ok,
            file_schema_version: None,
            moved_to: None,
            diagnostic: None,
            message: None,
        };
        match load {
            SettingsLoad::Current(_) => {}
            SettingsLoad::Absent => dto.status = SettingsStatus::Absent,
            SettingsLoad::Migrated { from, .. } => {
                dto.status = SettingsStatus::Migrated;
                dto.file_schema_version = Some(*from);
                dto.diagnostic = Some(SettingsDiagnostic::Migrated {
                    from_version: *from,
                    to_version: CURRENT_SCHEMA_VERSION,
                });
                dto.message = Some(format!(
                    "Settings were migrated from schema version {from} to {CURRENT_SCHEMA_VERSION}."
                ));
            }
            SettingsLoad::RefusedNewer { file_version } => {
                dto.status = SettingsStatus::RefusedNewer;
                dto.file_schema_version = Some(*file_version);
                dto.diagnostic = Some(SettingsDiagnostic::RefusedNewer {
                    file_version: *file_version,
                    current_version: CURRENT_SCHEMA_VERSION,
                });
                dto.message = Some(format!(
                    "The settings file is schema version {file_version}, which this build \
                     (version {CURRENT_SCHEMA_VERSION}) cannot read. It was left untouched and \
                     this session is running on defaults."
                ));
            }
            SettingsLoad::Quarantined {
                moved_to,
                reason,
                detail,
            } => {
                let name = moved_to
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                dto.status = SettingsStatus::Quarantined;
                dto.diagnostic = Some(SettingsDiagnostic::Quarantined {
                    reason: (*reason).into(),
                    moved_to: name.clone(),
                });
                dto.message = Some(format!(
                    "The settings file {detail}. It was moved to \"{name}\" in the data \
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
        let Some(message) = self.message.clone() else {
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
                message,
                location: None,
                detail: None,
                diagnostic: self.diagnostic.clone(),
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
///
/// Takes the same lock as the two writing routes, because this read is not
/// one: [`settings::load`] writes back a migrated document and renames a
/// damaged one aside. Without the lock, a GET that migrates could land its
/// pre-patch document *after* a concurrent PUT and silently undo it.
async fn read_settings(State(state): State<SharedState>) -> Result<Json<SettingsDto>, ApiError> {
    let _guard = state.settings_lock.lock().expect("state mutex poisoned");

    let load = read(&state)?;
    let dto = SettingsDto::from_load(&load);
    dto.log(&state, "settings");
    Ok(Json(dto))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PatchRequest {
    settings: Preferences,
    /// Optional key-scoped compare-and-patch for acknowledged preference edits.
    /// Null expects absence, not a stored null. Older callers omit this field.
    expected_settings: Option<Preferences>,
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
    if load.blocks_writes()
        || (request.expected_settings.is_some() && matches!(load, SettingsLoad::Quarantined { .. }))
    {
        let dto = SettingsDto::from_load(&load);
        dto.log(&state, "settings");
        return Err(ApiError::with_status(
            StatusCode::CONFLICT,
            dto.message
                .unwrap_or_else(|| "the settings file was written by a newer build".to_string()),
        ));
    }

    let mut document = load.document();
    if let Some(expected) = &request.expected_settings {
        let changed = expected.iter().any(|(key, value)| {
            if value.is_null() {
                document.preferences.contains_key(key)
            } else {
                document.preferences.get(key) != Some(value)
            }
        });
        if changed {
            return Err(ApiError::with_status(
                StatusCode::CONFLICT,
                "settings changed; read the current record before retrying",
            ));
        }
    }
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
    match load {
        SettingsLoad::Absent => {}
        // `read` has just moved the damaged file aside, so saying "a file
        // already exists" would be false by the time it was read. The
        // retry is genuinely all that is needed: the next read is
        // `Absent`.
        SettingsLoad::Quarantined { .. } => {
            let dto = SettingsDto::from_load(&load);
            dto.log(&state, "settings");
            return Err(ApiError::with_status(
                StatusCode::CONFLICT,
                dto.message.unwrap_or_else(|| {
                    "the settings file could not be read and was moved aside".to_string()
                }),
            ));
        }
        _ => {
            return Err(ApiError::with_status(
                StatusCode::CONFLICT,
                "a settings file already exists; there is nothing to adopt into",
            ));
        }
    }

    let document = settings::migrated_document(request.settings, request.schema_version);
    write(&state, &document)?;

    let migrated_from_browser = request.schema_version == BROWSER_ERA_SCHEMA_VERSION;
    let mut dto = SettingsDto::from_load(&SettingsLoad::Current(document));
    if migrated_from_browser {
        dto.status = SettingsStatus::Migrated;
        dto.file_schema_version = Some(BROWSER_ERA_SCHEMA_VERSION);
        dto.diagnostic = Some(SettingsDiagnostic::Adopted {
            from_version: BROWSER_ERA_SCHEMA_VERSION,
            to_version: CURRENT_SCHEMA_VERSION,
        });
        dto.message = Some(
            "Preferences that were stored in this browser have been adopted into the \
             application settings file."
                .to_string(),
        );
    }
    dto.log(&state, "settings");
    Ok(Json(dto))
}
