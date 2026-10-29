//! Owned typed result projections shared by retained jobs and diagnostic bundle collection.

use crate::diagnostic_bundle_context::ProjectContext;
use knx_app::diagnostic_bundle::BundleFile;
use std::io::Read;

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RecordedDiagnosticResult {
    report: knx_app::device_diagnostics::Report,
    memory: Option<knx_app::device_diagnostics::MemoryEvidence>,
}

impl From<RecordedDiagnosticResult> for knx_app::device_diagnostics::Export {
    fn from(result: RecordedDiagnosticResult) -> Self {
        Self {
            report: result.report,
            memory: result.memory,
        }
    }
}

pub(crate) struct Evidence {
    pub identity: serde_json::Value,
    pub context: Option<ProjectContext>,
    pub device_id: Option<u32>,
    pub reports: Vec<BundleFile>,
    pub operation_files: Vec<BundleFile>,
}

/// Reuse the existing writer and its independent readback; never duplicate its grammar.
pub(crate) fn diagnostic_files(
    export: &knx_app::device_diagnostics::Export,
) -> Result<Vec<BundleFile>, String> {
    members(
        knx_app::device_diagnostics::bundle_export(export)?,
        &[
            "report.json",
            "properties.json",
            "memory.json",
            "README.txt",
        ],
    )
}

pub(crate) fn members(bytes: Vec<u8>, names: &[&str]) -> Result<Vec<BundleFile>, String> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|_| "the retained report archive is unavailable")?;
    let mut out = Vec::new();
    for name in names {
        match archive.by_name(name) {
            Ok(mut file) => {
                let mut bytes = Vec::new();
                file.read_to_end(&mut bytes)
                    .map_err(|_| "the retained report member cannot be read")?;
                out.push(BundleFile::new(*name, bytes));
            }
            Err(zip::result::ZipError::FileNotFound) => {}
            Err(_) => return Err("the retained report member cannot be read".into()),
        }
    }
    Ok(out)
}
