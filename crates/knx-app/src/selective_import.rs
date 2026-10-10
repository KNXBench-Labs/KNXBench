//! Reads bounded ETS sources for read-only selection and atomic merge planning.

mod merge;
mod structure;

use std::io::Read;
use std::path::Path;

use knx_core::{Project, SourceRef};
use knx_etsproj::{ImportReport, ProjectPassword};
use knx_store::project_history::NativeSnapshot;
use knx_store::{ManufacturerRef, StoredOpaqueEntry};
use serde::{Deserialize, Serialize};

/// A separate bounded offline selection path, never an increased importer limit.
pub const MAX_SOURCE_BYTES: usize = 64 * 1024 * 1024;
/// Explicitly frozen reference closure; audit new fields before widening.
pub const SELECTIVE_MODEL_VERSION: u32 = 11;
/// Source attributes remain evidence, not unscoped destination settings.
pub const SELECTIVE_ATTRIBUTE_KIND: &str = "SelectiveImportRetainedAttribute";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Selection {
    pub source_installation: u8,
    pub target_installation: u8,
    #[serde(default)]
    pub devices: Vec<u32>,
    #[serde(default)]
    pub lines: Vec<u32>,
}

pub struct Source {
    pub project: Project,
    pub report: ImportReport,
    pub source_hash: String,
    pub(crate) prefix: String,
    pub(crate) opaque: Vec<StoredOpaqueEntry>,
    pub(crate) manufacturer_refs: Vec<ManufacturerRef>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IdMapping {
    pub kind: &'static str,
    pub source: u32,
    pub target: u32,
    pub reused: bool,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Counts {
    pub devices: usize,
    pub lines: usize,
    pub communication_objects: usize,
    pub parameters: usize,
    pub modules: usize,
    pub group_addresses: usize,
    pub building_parts: usize,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub source_hash: String,
    pub selection: Selection,
    pub counts: Counts,
    pub mappings: Vec<IdMapping>,
    pub source_report: ImportReport,
    pub retained_source_may_contain_unselected_data: bool,
    pub retained_source_entries: usize,
    pub notes: Vec<String>,
}

impl std::fmt::Debug for Preview {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Preview")
            .field("counts", &self.counts)
            .finish_non_exhaustive()
    }
}

impl std::fmt::Debug for Plan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Plan")
            .field("source_hash", &self.preview.source_hash)
            .finish_non_exhaustive()
    }
}

pub struct Plan {
    pub preview: Preview,
    pub(crate) before: NativeSnapshot,
    pub(crate) after: NativeSnapshot,
}

/// Reads through a cap even if a file grows after opening. No persistent writes.
pub fn read_source(path: &Path, password: Option<&ProjectPassword>) -> Result<Source, String> {
    knx_etsproj::check_project_filename(path).map_err(|e| e.to_string())?;
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    if file.metadata().map_err(|e| e.to_string())?.len() > MAX_SOURCE_BYTES as u64 {
        return Err("selective-import source exceeds the 64 MiB input limit".into());
    }
    let mut bytes = Vec::new();
    file.take(MAX_SOURCE_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    source_from_bytes(bytes, password)
}

pub fn source_from_bytes(
    bytes: Vec<u8>,
    password: Option<&ProjectPassword>,
) -> Result<Source, String> {
    if bytes.len() > MAX_SOURCE_BYTES {
        return Err("selective-import source exceeds the 64 MiB input limit".into());
    }
    let hash = knx_etsproj::opaque::sha256_hex(&bytes);
    let prefix = format!("imports/{hash}");
    let outcome =
        knx_etsproj::import_knxproj_bytes_with(bytes.clone(), "source.knxproj", password, &())
            .map_err(|e| e.to_string())?;
    let mut opaque: Vec<_> = outcome
        .opaque
        .into_iter()
        .map(|e| StoredOpaqueEntry {
            source_path: format!("{prefix}/{}", e.source_path),
            xpath: e.xpath,
            kind: if e.kind == knx_etsproj::opaque::OpaqueKind::RetainedAttribute {
                SELECTIVE_ATTRIBUTE_KIND.into()
            } else {
                format!("{:?}", e.kind)
            },
            name: e.name,
            bytes: e.bytes,
            sha256: e.sha256,
        })
        .collect();
    let manufacturer_refs = outcome
        .manufacturer
        .iter()
        .map(|m| ManufacturerRef {
            source_path: format!("{prefix}/{}", m.source_path),
            sha256: m.sha256.clone(),
            len: m.bytes.len() as i64,
            kind: format!("{:?}", m.kind),
        })
        .collect();
    opaque.extend(outcome.manufacturer.into_iter().map(|m| StoredOpaqueEntry {
        source_path: format!("{prefix}/{}", m.source_path),
        xpath: String::new(),
        kind: format!("{:?}", m.kind),
        name: String::new(),
        bytes: m.bytes,
        sha256: m.sha256,
    }));
    opaque.push(StoredOpaqueEntry {
        source_path: format!("{prefix}/source.knxproj"),
        xpath: String::new(),
        kind: "SelectiveImportArchive".into(),
        name: String::new(),
        bytes,
        sha256: hash.clone(),
    });
    let report_bytes = outcome.report.to_json().into_bytes();
    opaque.push(StoredOpaqueEntry {
        source_path: format!("{prefix}/import-report.json"),
        xpath: String::new(),
        kind: "SelectiveImportReport".into(),
        name: String::new(),
        sha256: knx_etsproj::opaque::sha256_hex(&report_bytes),
        bytes: report_bytes,
    });
    Ok(Source {
        project: outcome.project,
        report: outcome.report,
        source_hash: hash,
        prefix,
        opaque,
        manufacturer_refs,
    })
}

/// Constructs an admitted candidate, without changing any caller state.
pub fn plan(
    target: &NativeSnapshot,
    source: &Source,
    selection: Selection,
) -> Result<Plan, String> {
    merge::plan(target, source, selection)
}

/// One undo step through the existing snapshot history, without resetting old edits.
pub fn apply(
    target: &mut NativeSnapshot,
    stack: &mut knx_core::CommandStack,
    plan: Plan,
) -> Result<Preview, String> {
    if *target != plan.before {
        return Err("selective-import preview is stale; preview again".into());
    }
    let (mut undo, _) = stack
        .snapshot_states(&target.project)
        .map_err(|e| e.to_string())?;
    undo.push(target.project.clone());
    let next_stack = knx_core::CommandStack::from_snapshot_states(undo, vec![]);
    *target = plan.after;
    *stack = next_stack;
    Ok(plan.preview)
}

pub(crate) fn scoped(source: &SourceRef, prefix: &str) -> SourceRef {
    SourceRef {
        path: format!("{prefix}/{}", source.path),
        ets_id: source.ets_id.clone(),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryDevice {
    pub id: u32,
    pub name: String,
    pub address: Option<String>,
    pub line: Option<u32>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryLine {
    pub id: u32,
    pub name: String,
    pub address: String,
    pub device_count: usize,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryInstallation {
    pub id: u8,
    pub name: String,
    pub devices: Vec<InventoryDevice>,
    pub lines: Vec<InventoryLine>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Inventory {
    pub source_hash: String,
    pub installations: Vec<InventoryInstallation>,
    pub source_report: ImportReport,
}

impl Source {
    pub fn inventory(&self) -> Inventory {
        let installations = self
            .project
            .installations
            .iter()
            .map(|i| {
                let devices = i
                    .topology
                    .lines
                    .iter()
                    .flat_map(|l| l.devices.iter().map(move |id| (*id, Some(l.id))))
                    .chain(i.topology.unassigned.iter().map(|id| (*id, None)))
                    .filter_map(|(id, line)| {
                        self.project.devices.get(id).map(|d| InventoryDevice {
                            id: id.0,
                            name: d.name.clone(),
                            address: d.address.map(|a| a.to_string()),
                            line: line.map(|l| l.0),
                        })
                    })
                    .collect();
                let lines = i
                    .topology
                    .lines
                    .iter()
                    .map(|l| InventoryLine {
                        id: l.id.0,
                        name: l.name.clone(),
                        address: i
                            .topology
                            .area_of(l.id)
                            .map(|a| format!("{}.{}", a.address, l.address))
                            .unwrap_or_else(|| "?".into()),
                        device_count: l.devices.len(),
                    })
                    .collect();
                InventoryInstallation {
                    id: i.id.0,
                    name: i.name.clone(),
                    devices,
                    lines,
                }
            })
            .collect();
        Inventory {
            source_hash: self.source_hash.clone(),
            installations,
            source_report: self.report.clone(),
        }
    }
}

impl Plan {
    /// Binds every model/context effect, source, selection and caller scope.
    pub fn confirmation_token(&self, binding: &str) -> Result<String, String> {
        let payload = serde_json::json!({"version":1,"binding":binding,"source":self.preview.source_hash,
            "selection":self.preview.selection,"before":self.before.semantic_hash().map_err(|e|e.to_string())?,
            "after":self.after.semantic_hash().map_err(|e|e.to_string())?});
        Ok(knx_etsproj::opaque::sha256_hex(
            &serde_json::to_vec(&payload).map_err(|e| e.to_string())?,
        ))
    }
}
