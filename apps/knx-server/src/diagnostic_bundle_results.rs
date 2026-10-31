//! Owned typed result projections shared by retained jobs and diagnostic bundle collection.

use crate::diagnostic_bundle_context::ProjectContext;
use knx_app::diagnostic_bundle::BundleFile;

pub(crate) struct Evidence {
    pub identity: serde_json::Value,
    pub context: Option<ProjectContext>,
    pub device_id: Option<u32>,
    pub reports: Vec<BundleFile>,
    pub operation_files: Vec<BundleFile>,
}

pub(crate) enum EvidenceLookup {
    Missing,
    Expired,
    Unfinished,
    NonExportable,
    Unavailable,
}

pub(crate) fn retained_identity_file(name: &str, identity: &serde_json::Value) -> BundleFile {
    BundleFile::json(name, identity)
}
