//! Bounded read-only views over metadata the independent domain already models.

use knx_core::Project;
use serde::Serialize;
pub const MAX_TEXT_BYTES: usize = 64 * 1024;
pub(crate) fn completion_name(
    completion: knx_core::commissioning::CompletionStatus,
) -> &'static str {
    match completion {
        knx_core::commissioning::CompletionStatus::Undefined => "Undefined",
        knx_core::commissioning::CompletionStatus::Editing => "Editing",
        knx_core::commissioning::CompletionStatus::FinishedDesign => "FinishedDesign",
        knx_core::commissioning::CompletionStatus::FinishedCommissioning => "FinishedCommissioning",
        knx_core::commissioning::CompletionStatus::Tested => "Tested",
        knx_core::commissioning::CompletionStatus::Locked => "Locked",
        knx_core::commissioning::CompletionStatus::Accepted => "Accepted",
    }
}

#[derive(Debug, Serialize)]
pub struct ProjectMetadata {
    pub view_version: u32,
    pub scope: &'static str,
    pub model_schema_version: u32,
    pub project_id: String,
    pub name: String,
    pub project_number: Option<String>,
    pub group_address_style: &'static str,
    pub completion: &'static str,
    pub last_modified: Option<String>,
    pub project_start: Option<String>,
    pub ets_schema_version: u32,
    pub unlifted_group_address_dpt_declarations: u32,
}
pub fn project(project: &Project) -> Result<ProjectMetadata, String> {
    let info = &project.info;
    // Admit all modeled text before cloning; do not truncate source values.
    [
        info.project_id.len(),
        info.name.len(),
        info.project_number.as_ref().map_or(0, String::len),
    ]
    .into_iter()
    .try_fold(0usize, |total, length| total.checked_add(length))
    .filter(|total| *total <= MAX_TEXT_BYTES)
    .ok_or("modeled project metadata exceeds 64 KiB of text; project remains unchanged")?;
    let group_address_style = match info.group_address_style {
        knx_core::address::GroupAddressStyle::Free => "Free",
        knx_core::address::GroupAddressStyle::TwoLevel => "TwoLevel",
        knx_core::address::GroupAddressStyle::ThreeLevel => "ThreeLevel",
    };
    let completion = completion_name(info.completion);
    Ok(ProjectMetadata {
        view_version: 1,
        scope: "normalizedProject",
        model_schema_version: project.schema_version,
        project_id: info.project_id.clone(),
        name: info.name.clone(),
        project_number: info.project_number.clone(),
        group_address_style,
        completion,
        last_modified: info.last_modified.as_ref().map(|date| date.to_rfc3339()),
        project_start: info.project_start.as_ref().map(|date| date.to_rfc3339()),
        ets_schema_version: info.ets_schema_version,
        unlifted_group_address_dpt_declarations: info.unlifted_group_address_dpt_declarations,
    })
}
