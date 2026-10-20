//! Pure offline monitoring projection of a current normalized project.
use knx_core::{InstallationId, Project};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default)]
pub struct ExportOptions {
    pub installation: Option<InstallationId>,
    pub language: Option<String>,
}

#[derive(Debug)]
pub struct ExportBundle {
    pub bytes: Vec<u8>,
    pub report: ExportReport,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ExportReport {
    pub format_version: u32,
    pub profile: String,
    pub producer: String,
    pub producer_version: String,
    pub installation: u32,
    pub project_sha256: String,
    pub counts: ExportCounts,
    pub issues: Vec<ExportIssue>,
    pub excluded_data: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct ExportCounts {
    pub devices: Count,
    pub communication_objects: Count,
    pub group_addresses: Count,
    pub spaces: Count,
    pub functions: Count,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct Count {
    pub selected: u64,
    pub exported: u64,
    pub omitted: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ExportIssue {
    pub code: String,
    pub entity: String,
    pub field: String,
    pub detail: String,
}

mod addresses;
mod budget;
mod context;
mod objects;
mod package;
mod snapshot;
mod structure;
mod validate;

/// Build one deterministic sparse monitoring ZIP from the borrowed current
/// normalized snapshot. No source archive, persistence or runtime bus access.
/// The optional catalogue is queried for product display metadata only; no
/// enrichment is applied to the caller or to a hidden whole-project clone.
pub fn build(
    project: &Project,
    products: Option<&knx_productdb::Connection>,
    options: &ExportOptions,
) -> Result<ExportBundle, String> {
    use serde_json::{json, Value};
    use sha2::{Digest, Sha256};
    budget::admit(project, options)?;
    let selected = validate::select(project, options.installation)?;
    let language = knx_core::Language(
        options
            .language
            .as_deref()
            .unwrap_or(&project.strings.default_language().0)
            .into(),
    );
    let mut ctx = context::Context::default();
    let mut counts = ExportCounts::default();
    let (devices, communication_objects) = objects::project(
        project,
        &selected,
        products,
        &language,
        &mut ctx,
        &mut counts,
    )?;
    let group_addresses = addresses::project(project, &selected, &mut ctx)?;
    let topology = structure::topology(&selected, &mut ctx)?;
    let locations = structure::locations(&selected, &mut ctx)?;
    let group_ranges = structure::ranges(project, &selected, &mut ctx)?;
    let functions = structure::functions(project, &selected, &mut ctx)?;
    counts.group_addresses = Count::complete(selected.addresses.len());
    counts.spaces = Count::complete(selected.spaces.len());
    counts.functions = Count::complete(selected.functions.len());
    ctx.gap("project", "guid/schema_version/xknxproject_version")?;
    if project.info.last_modified.is_none() {
        ctx.gap("project", "last_modified")?;
    }
    ctx.issue("snapshot_scope","project","input","Current normalized input, one selected installation. Other installations are outside selection scope, not counted as omissions. Retained source/import context is unavailable to this builder, not measured zero.")?;
    ctx.issue("projection_limits","project","metadata","No source XML reparse, manufacturer object translation/name fill, channel/module/DPA projection or program-default enrichment. StringTable requested-language/default fallback is used; authored literals remain unchanged.")?;
    ctx.issue("unprojected","project","normalized_metadata","Completion, device visibility, GA central/unfiltered flags, domain/routing configuration and source identifiers are not represented. Keys use project-local native IDs, not source identity or global UUIDs.")?;
    let style = match project.info.group_address_style {
        knx_core::GroupAddressStyle::ThreeLevel => "ThreeLevel",
        knx_core::GroupAddressStyle::TwoLevel => "TwoLevel",
        knx_core::GroupAddressStyle::Free => "Free",
    };
    ctx.record(
        &[&project.info.name, &project.info.project_id, &language.0],
        1024,
    )?;
    let info = json!({"project_id":project.info.project_id,"name":project.info.name,"last_modified":project.info.last_modified.map(|t|t.to_rfc3339()),"group_address_style":style,"guid":"","created_by":"KNXBench","schema_version":"","tool_version":env!("CARGO_PKG_VERSION"),"xknxproject_version":"","language_code":language.0});
    let mut root = context::map();
    for (name, value) in [
        ("info", info),
        ("devices", devices),
        ("communication_objects", communication_objects),
        ("topology", topology),
        ("locations", locations),
        ("group_addresses", group_addresses),
        ("group_ranges", group_ranges),
        ("functions", functions),
    ] {
        root.insert(name.into(), value);
    }
    let project_json = package::json(&Value::Object(root))?;
    ctx.issues.sort_by(|a, b| {
        (&a.entity, &a.field, &a.code, &a.detail).cmp(&(&b.entity, &b.field, &b.code, &b.detail))
    });
    let report = ExportReport {
        format_version: 1,
        profile: "monitoring-sparse-v1".into(),
        producer: "KNXBench".into(),
        producer_version: env!("CARGO_PKG_VERSION").into(),
        installation: u32::from(selected.installation.id.0),
        project_sha256: format!("{:x}", Sha256::digest(&project_json)),
        counts,
        issues: ctx.issues,
        excluded_data: vec![
            "parameters".into(),
            "notes".into(),
            "instructions".into(),
            "commissioning/recovery".into(),
            "history".into(),
            "keys/passwords".into(),
            "manufacturer sources/baggage/opaque/attachments".into(),
            "Not a backup; no receiver acceptance or verified JSON/ZIP ingress".into(),
        ],
    };
    let report_json = package::json(&report)?;
    let bytes = package::zip(&project_json, &report_json)?;
    Ok(ExportBundle { bytes, report })
}
/// Admit the normalized snapshot before an adapter clones it. Call while
/// holding the same project lock used for the subsequent clone. Checks all
/// normalized text/record/reference vectors (including excluded notes and
/// instructions), plus graph/selection integrity and a conservative 32 MiB
/// complete Debug-walk guard covering private Core side maps. The Debug walk
/// retains no content and can refuse inputs the pure builder would admit. Does not query a
/// catalogue, bound native history loading, or promise output admission.
/// `build` independently repeats these checks on its actual borrowed input.
pub fn validate_request(project: &Project, options: &ExportOptions) -> Result<(), String> {
    budget::admit(project, options)?;
    validate::select(project, options.installation)?;
    snapshot::admit(project)?;
    Ok(())
}

/// Borrowed resource preflight before an adapter clones a normalized snapshot.
/// Call under the same project lock as the clone. Includes excluded heap fields
/// and a conservative streamed 32 MiB complete Debug walk for private Core side
/// maps. No catalogue, source parser, I/O, mutation or clone is performed.
/// Does not choose an installation or promise output admission; use
/// `validate_request` for selection/graph checks as well. Native/history loading
/// before this call remains an adapter concern.
pub fn admit_snapshot(project: &Project) -> Result<(), String> {
    budget::admit(project, &ExportOptions::default())?;
    snapshot::admit(project)
}

impl Count {
    fn complete(n: usize) -> Self {
        Self {
            selected: n as u64,
            exported: n as u64,
            omitted: 0,
        }
    }
}
