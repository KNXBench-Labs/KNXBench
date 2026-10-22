//! Snapshot-bound, read-only device catalogue metadata; each product is resolved once.

use std::collections::HashMap;
use std::sync::atomic::Ordering;

use axum::extract::{Query, State};
use axum::Json;
use knx_core::system_family::{declared_system, DeclaredSystem, SystemFamily};
use knx_productdb::query::DeviceProgramRelation;
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

/// WP-20: the system family the device's application program declares
/// through its product-database mask version. A mask read from a device is a
/// separate session fact and never reaches this field.
#[derive(Clone, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
enum DeviceSystem {
    /// The product did not resolve; `resolution` says why.
    NoProduct,
    /// The device names no application program (empty program reference).
    NoApplicationProgram,
    /// The device names a program link, but the link or its application
    /// program is not installed in the product database.
    ApplicationNotInDatabase,
    /// The program link belongs to other hardware than the device's
    /// product; its program's mask says nothing about this product.
    ApplicationNotOfProduct,
    NoMaskVersion,
    InvalidMask {
        declared: String,
    },
    UnknownMask {
        mask: u16,
    },
    Family {
        mask: u16,
        family: FamilyToken,
    },
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
enum FamilyToken {
    System1,
    System2,
    System7,
    SystemB,
    System300,
    Coupler,
    KnxnetIpRouter,
    UsbInterface,
    IrDecoder,
    RfBidirectional,
    RfUnidirectional,
}

impl From<SystemFamily> for FamilyToken {
    fn from(family: SystemFamily) -> Self {
        match family {
            SystemFamily::System1 => Self::System1,
            SystemFamily::System2 => Self::System2,
            SystemFamily::System7 => Self::System7,
            SystemFamily::SystemB => Self::SystemB,
            SystemFamily::System300 => Self::System300,
            SystemFamily::Coupler => Self::Coupler,
            SystemFamily::KnxnetIpRouter => Self::KnxnetIpRouter,
            SystemFamily::UsbInterface => Self::UsbInterface,
            SystemFamily::IrDecoder => Self::IrDecoder,
            SystemFamily::RfBidirectional => Self::RfBidirectional,
            SystemFamily::RfUnidirectional => Self::RfUnidirectional,
        }
    }
}

impl DeviceSystem {
    fn of(row: &knx_productdb::query::DeviceProductRow, program_ref: &str) -> Self {
        if program_ref.is_empty() {
            return Self::NoApplicationProgram;
        }
        match row.program_relation {
            DeviceProgramRelation::HardwareMismatch => return Self::ApplicationNotOfProduct,
            DeviceProgramRelation::Missing => return Self::ApplicationNotInDatabase,
            DeviceProgramRelation::Matched if row.application_program_id.is_none() => {
                return Self::ApplicationNotInDatabase
            }
            DeviceProgramRelation::Matched => {}
        }
        match declared_system(row.mask_version.as_deref()) {
            DeclaredSystem::NoMaskVersion => Self::NoMaskVersion,
            DeclaredSystem::InvalidMask(declared) => Self::InvalidMask { declared },
            DeclaredSystem::UnknownMask(mask) => Self::UnknownMask { mask: mask.0 },
            DeclaredSystem::Family { mask, family } => Self::Family {
                mask: mask.0,
                family: family.into(),
            },
        }
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProductMetadata {
    resolution: ProductResolution,
    system: DeviceSystem,
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
            system: DeviceSystem::NoProduct,
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
            .map(|device| {
                let refs = (device.product_ref.clone(), device.program_ref.clone());
                (build_device_node(device), refs)
            })
            .collect();
        (state.project_revision.load(Ordering::Relaxed), inputs)
    };
    inputs.sort_by_key(|(device, _)| device.id);
    let compute = |products: Option<&knx_productdb::Connection>,
                   inputs: Vec<(DeviceNode, (String, String))>|
     -> Result<(Vec<DeviceCatalogRow>, Option<&'static str>), ApiError> {
        let mut metadata = HashMap::new();
        let mut problem = None;
        let mut devices = Vec::with_capacity(inputs.len());
        for (device, refs) in inputs {
            let (product_ref, program_ref) = &refs;
            if !metadata.contains_key(&refs) {
                let value = if product_ref.is_empty() {
                    ProductMetadata::unavailable(ProductResolution::NoReference)
                } else if let Some(products) = &products {
                    // Product identity plus the program's declared mask (WP-20); no
                    // application compatibility or parameter evaluation.
                    match knx_productdb::query::device_product(
                        products,
                        product_ref,
                        program_ref,
                        query.language.as_deref(),
                    ) {
                        Ok(Some(row)) => ProductMetadata {
                            resolution: ProductResolution::Resolved,
                            system: DeviceSystem::of(&row, program_ref),
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
                metadata.insert(refs.clone(), value);
            }
            devices.push(DeviceCatalogRow {
                id: device.id,
                device,
                product: metadata[&refs].clone(),
                product_ref: (!refs.0.is_empty()).then(|| refs.0.clone()),
            });
        }
        Ok((devices, problem))
    };
    let (devices, problem) = match state.product_db.as_ref() {
        Some(owner) => owner
            .with_read(|conn| compute(Some(conn), inputs))
            .map_err(crate::catalogue_owner::owner_error)??,
        None => compute(None, inputs)?,
    };
    Ok(Json(DeviceCatalog {
        schema_version: 1,
        server_incarnation: state.server_incarnation.clone(),
        snapshot_revision: revision,
        devices,
        problem,
    }))
}
