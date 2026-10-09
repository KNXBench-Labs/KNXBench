//! Snapshot-bound, read-only device catalogue metadata; each product is resolved once.

use std::collections::HashMap;
use std::sync::atomic::Ordering;

use axum::extract::{Query, State};
use axum::Json;
use knx_projection::{build_device_node, DeviceNode};
use serde::{Deserialize, Serialize};

use crate::{errors::ApiError, SharedState};

#[derive(Deserialize)]
pub(crate) struct LanguageQuery {
    language: Option<String>,
}

// Product identity only: no claim about the device's application compatibility.
#[derive(Clone, Serialize)]
enum ProductResolution {
    Resolved,
    NoReference,
    NoDatabase,
    NotInDatabase,
    Unavailable,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProductMetadata {
    resolution: ProductResolution,
    manufacturer_id: Option<String>,
    manufacturer_name: Option<String>,
    product_text: Option<String>,
    order_number: Option<String>,
    product_text_language: Option<String>,
    product_source_language: Option<String>,
}

impl ProductMetadata {
    fn unavailable(resolution: ProductResolution) -> Self {
        Self {
            resolution,
            manufacturer_id: None,
            manufacturer_name: None,
            product_text: None,
            order_number: None,
            product_text_language: None,
            product_source_language: None,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DeviceCatalogRow {
    id: u32,
    device: DeviceNode,
    product_ref: Option<String>,
    #[serde(flatten)]
    product: ProductMetadata,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DeviceCatalog {
    schema_version: u32,
    server_incarnation: String,
    snapshot_revision: u64,
    devices: Vec<DeviceCatalogRow>,
    problem: Option<&'static str>,
}

pub(crate) async fn get(
    State(state): State<SharedState>,
    Query(query): Query<LanguageQuery>,
) -> Result<Json<DeviceCatalog>, ApiError> {
    // Copy only the identity/reference inputs while holding the project lock.
    // Product lookup never evaluates application parameters or touches the bus.
    let (revision, mut inputs) = {
        let guard = state.project.lock().expect("state mutex poisoned");
        let project = guard
            .as_ref()
            .ok_or_else(|| ApiError::bad_request("no project open"))?;
        let inputs: Vec<_> = project
            .devices
            .iter()
            .map(|device| (build_device_node(device), device.product_ref.clone()))
            .collect();
        (state.project_revision.load(Ordering::Relaxed), inputs)
    };
    inputs.sort_by_key(|(device, _)| device.id);
    let mut metadata = HashMap::new();
    let mut problem = None;
    let products = state
        .product_db
        .as_ref()
        .map(|mutex| mutex.lock().expect("state mutex poisoned"));
    let mut devices = Vec::with_capacity(inputs.len());
    for (device, product_ref) in inputs {
        if !metadata.contains_key(&product_ref) {
            let value = if product_ref.is_empty() {
                ProductMetadata::unavailable(ProductResolution::NoReference)
            } else if let Some(products) = &products {
                // This list resolves product identity only, not application/program
                // compatibility. An empty program ref avoids needless program work.
                match knx_productdb::query::device_product(
                    products,
                    &product_ref,
                    "",
                    query.language.as_deref(),
                ) {
                    Ok(Some(row)) => ProductMetadata {
                        resolution: ProductResolution::Resolved,
                        manufacturer_id: Some(row.manufacturer_id),
                        manufacturer_name: row.manufacturer_name,
                        product_text: row.product_text,
                        order_number: row.order_number,
                        product_text_language: row.product_text_language,
                        product_source_language: row.product_source_language,
                    },
                    Ok(None) => ProductMetadata::unavailable(ProductResolution::NotInDatabase),
                    Err(_) => {
                        problem = Some("lookupFailed");
                        ProductMetadata::unavailable(ProductResolution::Unavailable)
                    }
                }
            } else {
                ProductMetadata::unavailable(ProductResolution::NoDatabase)
            };
            metadata.insert(product_ref.clone(), value);
        }
        devices.push(DeviceCatalogRow {
            id: device.id,
            device,
            product: metadata[&product_ref].clone(),
            product_ref: (!product_ref.is_empty()).then_some(product_ref),
        });
    }
    Ok(Json(DeviceCatalog {
        schema_version: 1,
        server_incarnation: state.server_incarnation.clone(),
        snapshot_revision: revision,
        devices,
        problem,
    }))
}
