//! `/api/device-readiness`: every project device, graded as its download would be.
//!
//! The HTTP face of `knx device readiness` ([`project_readiness`]): offline,
//! per device of the open project, the download `/api/device-download/plan`
//! would prepare, graded as that route grades it. No gateway field and no
//! tunnel: this route never reaches the bus, so it takes no tunnel lock
//! either and can run beside a monitor or a download.

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;

use knx_app::download_support::{shipped_evidence, SupportLevel};
use knx_app::project_readiness::{project_readiness, DeviceReadiness, ReadinessSummary};

use crate::errors::ApiError;
use crate::SharedState;

pub fn device_readiness_routes() -> Router<SharedState> {
    Router::new().route("/api/device-readiness", get(readiness))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DeviceDto {
    /// `None` for a device without an individual address.
    address: Option<String>,
    name: String,
    program_ref: String,
    /// `verified`, `untested`, `unsupported`, `excluded` or `no-address`.
    readiness: &'static str,
    /// For `unsupported`: the category's stable name.
    category: Option<&'static str>,
    /// For `unsupported`: the refusal the download would give, in full.
    /// For `verified`: where the hardware run is documented.
    detail: Option<String>,
    /// For a plan (`verified`/`untested`): steps and segment octets.
    steps: Option<usize>,
    octets: Option<usize>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ReadinessResponse {
    devices: Vec<DeviceDto>,
    /// Devices per readiness code.
    counts: std::collections::BTreeMap<&'static str, usize>,
}

async fn readiness(State(state): State<SharedState>) -> Result<Json<ReadinessResponse>, ApiError> {
    let Some(products) = state.product_db.as_ref() else {
        return Err(ApiError::with_status(
            StatusCode::CONFLICT,
            "no product database is configured, so no application program can be read",
        ));
    };
    let evidence = shipped_evidence()
        .map_err(|e| ApiError::with_status(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    // The project lock, then the product lock, as `/api/device-download/plan`
    // takes them; both are released before the response is built.
    let rows = {
        let project_guard = state.project.lock().expect("state mutex poisoned");
        let Some(project) = project_guard.as_ref() else {
            return Err(ApiError::with_status(
                StatusCode::CONFLICT,
                "no project is open",
            ));
        };
        let products = products.lock().expect("state mutex poisoned");
        project_readiness(&products, project, &evidence)
    };
    let counts = ReadinessSummary::of(&rows).by_code;
    let devices = rows
        .into_iter()
        .map(|row| {
            let (category, detail, steps, octets) = match &row.readiness {
                DeviceReadiness::Graded(SupportLevel::Verified {
                    evidence,
                    steps,
                    octets,
                }) => (
                    None,
                    Some(format!(
                        "{}, {}; {}",
                        evidence.device, evidence.date, evidence.reference
                    )),
                    Some(*steps),
                    Some(*octets),
                ),
                DeviceReadiness::Graded(SupportLevel::Untested { steps, octets }) => {
                    (None, None, Some(*steps), Some(*octets))
                }
                DeviceReadiness::Graded(SupportLevel::Unsupported { category, detail }) => {
                    (Some(category.code()), Some(detail.clone()), None, None)
                }
                DeviceReadiness::Excluded | DeviceReadiness::NoAddress => (None, None, None, None),
            };
            DeviceDto {
                address: row.address.map(|address| address.to_string()),
                readiness: row.readiness.code(),
                name: row.name,
                program_ref: row.program_ref,
                category,
                detail,
                steps,
                octets,
            }
        })
        .collect();
    Ok(Json(ReadinessResponse { devices, counts }))
}
