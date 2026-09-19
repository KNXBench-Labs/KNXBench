//! `POST /api/debug-report` — builds the bundle and, if asked, writes the zip.
//!
//! T29. A sibling module next to `bus_routes.rs`/`fs_routes.rs` rather than
//! another section of `routes.rs`, for the same reason `bus_routes.rs`
//! gives in its own header: `routes.rs` is already 2000+ lines of
//! project/device/catalog handlers and shares nothing with this but the
//! `#[serde(rename_all = "camelCase")]` DTO and `Result<Json<T>, ApiError>`
//! idiom, which this module follows exactly.
//!
//! One route, two modes, decided by `path`:
//!
//! * `path: null` — build the bundle, return it, write nothing. This is what
//!   the "open a GitHub issue" button uses: it needs `report.md`'s text for
//!   the prefilled body and has no business creating a file nobody asked
//!   for. `written: false` in the response says so out loud.
//! * `path: "<somewhere>"` — the same bundle, additionally written there as
//!   a zip.
//!
//! Neither mode sends anything anywhere. The route returns the bundle to the
//! caller that asked for it and nothing else happens; there is no upload
//! endpoint in this application and no telemetry to carry it.
//!
//! The handler is `async` because `AppState.bus_session` is a
//! `tokio::sync::Mutex` (see `domain.rs`) — reading the monitor buffer at
//! all means awaiting that lock.

use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::debug_report::{self, BundleInput, Redactor};
use crate::errors::ApiError;
use crate::SharedState;

pub fn debug_report_routes() -> Router<SharedState> {
    Router::new().route("/api/debug-report", post(create_debug_report))
}

/// Facts only the browser knows: which build of the frontend is loaded,
/// whether it runs in Tauri or a plain browser tab, and the two settings a
/// user is most likely to have changed before hitting a bug. All optional —
/// a `curl` against this route is a legitimate caller and gets a bundle that
/// says `unknown` rather than a `400`.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClientFacts {
    app_version: Option<String>,
    shell: Option<String>,
    ui_language: Option<String>,
    theme: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DebugReportRequest {
    /// Where to write the zip. `None` builds the bundle without writing
    /// anything — see this module's header.
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    description: String,
    /// The session log is on by default: it is redacted, it is the single
    /// most useful artifact for a bug report, and it holds nothing the user
    /// has not already seen in the log panel.
    #[serde(default = "default_true")]
    include_log: bool,
    /// Off by default, like every artifact that says anything about the
    /// user's own installation.
    #[serde(default)]
    include_project_summary: bool,
    #[serde(default)]
    include_bus_telegrams: bool,
    #[serde(default)]
    client: ClientFacts,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BundleFileDto {
    name: String,
    bytes: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DebugReportResponse {
    /// Whether a zip was actually written. Never inferred by the client
    /// from "no error" — a preview succeeds too, and a UI that said "saved"
    /// after one would be lying.
    written: bool,
    path: Option<String>,
    /// Exactly the files the bundle holds, in order, with their sizes. The
    /// dialog shows this after the fact; `debug_report::build_bundle`
    /// derives it from what it built, not from what was requested.
    files: Vec<BundleFileDto>,
    /// `report.md`'s text, already redacted — the prefilled GitHub issue
    /// body is this string and nothing else.
    report_markdown: String,
}

async fn create_debug_report(
    State(state): State<SharedState>,
    Json(request): Json<DebugReportRequest>,
) -> Result<Json<DebugReportResponse>, ApiError> {
    let log = request.include_log.then(|| {
        let guard = state.session_log.lock().expect("state mutex poisoned");
        json!(guard.entries())
    });

    let (project_open, project_summary) = {
        let guard = state.project.lock().expect("project mutex poisoned");
        let open = guard.is_some();
        let summary = match (request.include_project_summary, guard.as_ref()) {
            (true, Some(project)) => {
                let counts = &*state.import_counts.lock().expect("state mutex poisoned");
                let saved = state
                    .store_path
                    .lock()
                    .expect("state mutex poisoned")
                    .is_some();
                Some(project_summary(project, *counts, saved))
            }
            // Asked for, but there is no project. An explicit marker beats
            // an absent file the user was told to expect.
            (true, None) => Some(json!({ "projectOpen": false })),
            (false, _) => None,
        };
        (open, summary)
    };

    let bus_telegrams = if request.include_bus_telegrams {
        let guard = state.bus_session.lock().await;
        Some(match guard.as_ref() {
            Some(session) => {
                let buffer = session.buffer();
                let buffer = buffer.lock().expect("bus session buffer poisoned");
                json!(buffer
                    .telegrams_since(0)
                    .iter()
                    .map(telegram_json)
                    .collect::<Vec<_>>())
            }
            None => json!([]),
        })
    } else {
        None
    };

    let input = BundleInput {
        description: request.description,
        app_version: request.client.app_version,
        server_version: crate::version_line(),
        shell: request.client.shell,
        ui_language: request.client.ui_language,
        theme: request.client.theme,
        project_open,
        log,
        project_summary,
        bus_telegrams,
    };

    let bundle = debug_report::build_bundle(&input, &Redactor::from_environment());

    let written_path = match request.path.as_deref() {
        Some(path) => {
            let resolved = crate::paths::resolve_new_project_path(&state.data_dir, path)?;
            debug_report::write_zip(&resolved, &bundle.files).map_err(ApiError::internal)?;
            Some(resolved.display().to_string())
        }
        None => None,
    };

    Ok(Json(DebugReportResponse {
        written: written_path.is_some(),
        path: written_path,
        files: bundle
            .files
            .iter()
            .map(|f| BundleFileDto {
                name: f.name.to_string(),
                bytes: f.bytes.len(),
            })
            .collect(),
        report_markdown: bundle.report_markdown,
    }))
}

/// Counts and structural facts only — no name, no address, no free text
/// from the project. That is what lets the dialog promise this file says
/// how big the project is and nothing about what is in it.
///
/// There is deliberately no "where did this project come from" field:
/// nothing in `AppState` records whether the open project arrived by ETS
/// import, by `.knxdb` load or as a new project, and inventing a field for
/// it would be a change to the application's state model dressed up as a
/// debug report. `etsSchemaVersion` is the honest stand-in — it says which
/// ETS schema the project was mapped from, which is the part a maintainer
/// actually needs.
fn project_summary(
    project: &knx_core::Project,
    import_counts: (usize, usize),
    saved_to_store: bool,
) -> Value {
    let installations = &project.installations;
    let sum =
        |f: fn(&knx_core::Installation) -> usize| -> usize { installations.iter().map(f).sum() };
    json!({
        "projectOpen": true,
        "schemaVersion": project.schema_version,
        "etsSchemaVersion": project.info.ets_schema_version,
        "groupAddressStyle": format!("{:?}", project.info.group_address_style),
        "savedToStore": saved_to_store,
        "importErrors": import_counts.0,
        "importWarnings": import_counts.1,
        "installations": installations.len(),
        "areas": sum(|i| i.topology.areas.len()),
        "lines": sum(|i| i.topology.lines.len()),
        "unassignedDevices": sum(|i| i.topology.unassigned.len()),
        "buildingParts": sum(|i| i.buildings.len()),
        "groupRanges": sum(|i| i.group_ranges.len()),
        "groupAddresses": sum(|i| i.group_addresses.len()),
        "parameters": sum(|i| i.parameters.len()),
        "devices": project.devices.iter().count(),
        "comObjects": project.devices.com_objects().count(),
    })
}

/// A telegram as JSON. Deliberately not `bus_routes.rs`'s `TelegramRowDto`
/// — that type is private to its module, and reaching into it would couple
/// the bus HTTP layer's wire shape to the bundle's file format, so that
/// changing one silently changed the other. The overlap is four lines of
/// field names; the coupling would be permanent.
fn telegram_json(row: &crate::bus::TelegramRow) -> Value {
    json!({
        "seq": row.seq,
        "timestamp": row.timestamp,
        "source": row.source,
        "destination": row.destination,
        "destinationName": row.destination_name,
        "service": row.service,
        "rawPayload": row.raw_payload,
        "decoded": row.decoded.as_ref().map(|d| match d {
            crate::bus::DecodedValue::Value { dpt, text } => {
                json!({ "kind": "value", "dpt": dpt, "text": text })
            }
            crate::bus::DecodedValue::Unresolved { text } => {
                json!({ "kind": "unresolved", "text": text })
            }
            crate::bus::DecodedValue::Conflict { text } => {
                json!({ "kind": "conflict", "text": text })
            }
            crate::bus::DecodedValue::Error { text, error } => {
                json!({ "kind": "error", "text": text, "error": error })
            }
        }),
    })
}
