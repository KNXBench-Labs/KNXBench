//! `list_functions`: the project's KNX Functions, read-only (ADR-0124).

use serde_json::json;

use super::{envelope, Page, ToolResult};
use crate::workspace::Workspace;

/// Every Function of the saved project, in model order, with its room and
/// members (`knx_projection::build_functions_view`, the read shape the web
/// panels use). `installation` keeps only that installation's Functions.
/// Edits stay with the web workbench; this tool changes nothing.
pub fn list_functions(
    ws: &Workspace,
    alias: &str,
    installation: Option<u8>,
    page: Page,
) -> ToolResult {
    let snapshot = ws.snapshot(alias)?;
    let project = &snapshot.project;
    if let Some(id) = installation {
        if !project.installations.iter().any(|i| i.id.0 == id) {
            return Err(format!("no installation {id} in project {alias:?}"));
        }
    }
    let view = knx_projection::build_functions_view(project);
    let functions: Vec<_> = view
        .functions
        .into_iter()
        .filter(|f| installation.is_none_or(|id| f.installation_id == id))
        .collect();
    Ok(envelope(
        snapshot.source_json(),
        json!({
            "installation": installation,
            "functions": page.apply(&functions),
        }),
    ))
}
