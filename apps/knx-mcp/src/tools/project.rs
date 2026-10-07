//! Project-wide tools: summary, search, issues, diff and CSV validation.

use std::collections::{BTreeMap, HashMap};

use knx_core::{DeviceId, InstallationId, Project};
use serde::Serialize;
use serde_json::{json, Value};

use super::{envelope, Page, ToolResult};
use crate::issues::{self, Severity};
use crate::workspace::{Snapshot, Workspace};

/// Longest accepted search query, in characters.
pub const MAX_QUERY_CHARS: usize = 200;
/// Largest accepted CSV text, in bytes.
pub const MAX_CSV_BYTES: usize = 1 << 20;

fn summary(snapshot: &Snapshot) -> Value {
    let project = &snapshot.project;
    let installations: Vec<Value> = project
        .installations
        .iter()
        .map(|i| {
            let devices = i
                .topology
                .lines
                .iter()
                .map(|l| l.devices.len())
                .sum::<usize>()
                + i.topology.unassigned.len();
            json!({
                "id": i.id.0,
                "name": i.name,
                "areas": i.topology.areas.len(),
                "lines": i.topology.lines.len(),
                "devices": devices,
                "groupAddresses": i.group_addresses.len(),
                "groupRanges": i.group_ranges.len(),
                "buildingParts": i.buildings.len(),
            })
        })
        .collect();
    let com_objects = project.devices.com_objects().count();
    let links: usize = project.devices.com_objects().map(|c| c.links.len()).sum();
    let parameters: usize = project
        .installations
        .iter()
        .map(|i| i.parameters.len())
        .sum();
    json!({
        "name": project.info.name,
        "projectNumber": project.info.project_number,
        "groupAddressStyle": snapshot.tree.group_address_style,
        "etsSchemaVersion": project.info.ets_schema_version,
        "installations": installations,
        "totals": {
            "devices": project.devices.iter().count(),
            "comObjects": com_objects,
            "groupLinks": links,
            "storedParameterValues": parameters,
        },
    })
}

/// `project_summary`: one project, or every configured one when `alias` is
/// `None` — which is also how an agent learns the aliases.
pub fn project_summary(ws: &Workspace, alias: Option<&str>) -> ToolResult {
    match alias {
        Some(alias) => {
            let snapshot = ws.snapshot(alias)?;
            let mut result = summary(&snapshot);
            result["productDatabase"] = ws.products_json();
            Ok(envelope(snapshot.source_json(), result))
        }
        None => {
            let projects: Vec<Value> = ws
                .aliases()
                .into_iter()
                .map(|alias| match ws.snapshot(alias) {
                    Ok(snapshot) => json!({
                        "source": snapshot.source_json(),
                        "summary": summary(&snapshot),
                    }),
                    Err(error) => json!({ "project": alias, "error": error }),
                })
                .collect();
            Ok(envelope(
                json!({ "projects": ws.aliases() }),
                json!({ "projects": projects, "productDatabase": ws.products_json() }),
            ))
        }
    }
}

/// The kinds `search` knows, in result order.
pub const SEARCH_KINDS: [&str; 5] = [
    "device",
    "groupAddress",
    "groupRange",
    "building",
    "comObject",
];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Hit {
    kind: &'static str,
    id: u32,
    installation: Option<String>,
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    address: Option<String>,
    /// For a communication object: its device.
    #[serde(skip_serializing_if = "Option::is_none")]
    device: Option<Value>,
    matched_in: Vec<&'static str>,
}

/// The fields of `fields` that contain a term, when every term occurs in
/// at least one of them; `None` otherwise.
fn matched_fields(
    terms: &[String],
    fields: &[(&'static str, Option<&str>)],
) -> Option<Vec<&'static str>> {
    let lowered: Vec<(&'static str, String)> = fields
        .iter()
        .filter_map(|(name, value)| value.map(|v| (*name, v.to_lowercase())))
        .collect();
    let all_found = terms.iter().all(|term| {
        lowered
            .iter()
            .any(|(_, value)| value.contains(term.as_str()))
    });
    if !all_found {
        return None;
    }
    Some(
        lowered
            .iter()
            .filter(|(_, value)| terms.iter().any(|term| value.contains(term.as_str())))
            .map(|(name, _)| *name)
            .collect(),
    )
}

/// Device id -> installation name, from each installation's topology.
fn device_installations(project: &Project) -> HashMap<DeviceId, String> {
    let mut map = HashMap::new();
    for installation in &project.installations {
        let lines = installation.topology.lines.iter().flat_map(|l| &l.devices);
        for id in lines.chain(&installation.topology.unassigned) {
            map.insert(*id, installation.name.clone());
        }
    }
    map
}

/// `search`: every entity of the requested kinds whose text holds all
/// whitespace-separated terms of `query`, case-insensitively.
pub fn search(
    ws: &Workspace,
    alias: &str,
    query: &str,
    kinds: Option<&[String]>,
    page: Page,
) -> ToolResult {
    if query.chars().count() > MAX_QUERY_CHARS {
        return Err(format!("query is longer than {MAX_QUERY_CHARS} characters"));
    }
    let terms: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    if terms.is_empty() {
        return Err("query is empty".into());
    }
    let wanted: Vec<&str> = match kinds {
        None => SEARCH_KINDS.to_vec(),
        Some(list) => {
            for kind in list {
                if !SEARCH_KINDS.contains(&kind.as_str()) {
                    return Err(format!(
                        "unknown kind {kind:?}; expected one of {}",
                        SEARCH_KINDS.join(", ")
                    ));
                }
            }
            SEARCH_KINDS
                .iter()
                .copied()
                .filter(|k| list.iter().any(|w| w == k))
                .collect()
        }
    };
    let snapshot = ws.snapshot(alias)?;
    let project = &snapshot.project;
    let owners = device_installations(project);
    let mut hits: Vec<Hit> = Vec::new();

    if wanted.contains(&"device") {
        for device in project.devices.iter() {
            let address = device.address.map(|a| a.to_string());
            if let Some(matched_in) = matched_fields(
                &terms,
                &[
                    ("name", Some(device.name.as_str())),
                    ("description", device.description.as_deref()),
                    ("address", address.as_deref()),
                ],
            ) {
                hits.push(Hit {
                    kind: "device",
                    id: device.id.0,
                    installation: owners.get(&device.id).cloned(),
                    name: Some(device.name.clone()),
                    address,
                    device: None,
                    matched_in,
                });
            }
        }
    }
    for (installation, node) in project
        .installations
        .iter()
        .zip(&snapshot.tree.installations)
    {
        if wanted.contains(&"groupAddress") {
            for ga in &node.group_addresses {
                if let Some(matched_in) = matched_fields(
                    &terms,
                    &[
                        ("name", Some(ga.name.as_str())),
                        ("address", Some(ga.address.as_str())),
                    ],
                ) {
                    hits.push(Hit {
                        kind: "groupAddress",
                        id: ga.id,
                        installation: Some(installation.name.clone()),
                        name: Some(ga.name.clone()),
                        address: Some(ga.address.clone()),
                        device: None,
                        matched_in,
                    });
                }
            }
        }
        if wanted.contains(&"groupRange") {
            for range in &node.group_ranges {
                let span = format!("{}-{}", range.start, range.end);
                if let Some(matched_in) = matched_fields(
                    &terms,
                    &[
                        ("name", Some(range.name.as_str())),
                        ("address", Some(span.as_str())),
                    ],
                ) {
                    hits.push(Hit {
                        kind: "groupRange",
                        id: range.id,
                        installation: Some(installation.name.clone()),
                        name: Some(range.name.clone()),
                        address: Some(span),
                        device: None,
                        matched_in,
                    });
                }
            }
        }
        if wanted.contains(&"building") {
            for part in &installation.buildings {
                let kind = format!("{:?}", part.kind);
                if let Some(matched_in) = matched_fields(
                    &terms,
                    &[
                        ("name", Some(part.name.as_str())),
                        ("number", part.number.as_deref()),
                        ("kind", Some(kind.as_str())),
                    ],
                ) {
                    hits.push(Hit {
                        kind: "building",
                        id: part.id.0,
                        installation: Some(installation.name.clone()),
                        name: Some(part.name.clone()),
                        address: None,
                        device: None,
                        matched_in,
                    });
                }
            }
        }
    }
    if wanted.contains(&"comObject") {
        for device in project.devices.iter() {
            let Some(detail) = knx_projection::build_device_detail(project, device.id) else {
                continue;
            };
            for com in &detail.com_objects {
                let number = format!("object {}", com.number);
                if let Some(matched_in) = matched_fields(
                    &terms,
                    &[
                        ("name", com.name.as_deref()),
                        ("description", com.description.as_deref()),
                        ("number", Some(number.as_str())),
                        ("device", Some(detail.name.as_str())),
                    ],
                ) {
                    hits.push(Hit {
                        kind: "comObject",
                        id: com.id,
                        installation: owners.get(&device.id).cloned(),
                        name: com.name.clone(),
                        address: None,
                        device: Some(json!({
                            "id": detail.id,
                            "name": detail.name,
                            "address": detail.address,
                            "objectNumber": com.number,
                        })),
                        matched_in,
                    });
                }
            }
        }
    }
    Ok(envelope(
        snapshot.source_json(),
        json!({ "query": query, "kinds": wanted, "hits": page.apply(&hits) }),
    ))
}

/// `find_issues`: [`issues::find_issues`], filtered to `min_severity` and
/// paged, with complete counts.
pub fn find_issues(
    ws: &Workspace,
    alias: &str,
    min_severity: Option<&str>,
    page: Page,
) -> ToolResult {
    let min = match min_severity {
        Some(text) => Severity::parse(text)?,
        None => Severity::Info,
    };
    let snapshot = ws.snapshot(alias)?;
    let all = issues::find_issues(&snapshot.project, &snapshot.tree);
    let mut by_code: BTreeMap<&str, usize> = BTreeMap::new();
    let mut by_severity: BTreeMap<String, usize> = BTreeMap::new();
    for issue in &all {
        *by_code.entry(issue.code).or_default() += 1;
        let severity = serde_json::to_value(issue.severity).unwrap_or_default();
        *by_severity
            .entry(severity.as_str().unwrap_or_default().to_string())
            .or_default() += 1;
    }
    let shown: Vec<_> = all.into_iter().filter(|i| i.severity <= min).collect();
    Ok(envelope(
        snapshot.source_json(),
        json!({
            "counts": { "bySeverity": by_severity, "byCode": by_code },
            "issues": page.apply(&shown),
            "scope": "Structural checks of the saved project only; no bus contact and no device reads.",
        }),
    ))
}

/// `diff_projects`: what changed from `left` to `right`, flattened.
pub fn diff_projects(ws: &Workspace, left: &str, right: &str, page: Page) -> ToolResult {
    let l = ws.snapshot(left)?;
    let r = ws.snapshot(right)?;
    let diff = knx_diff::diff_projects(&l.project, &r.project);
    let changes = crate::diff_render::flatten(&diff);
    let mut summary: BTreeMap<String, usize> = BTreeMap::new();
    for change in &changes {
        *summary
            .entry(format!("{} {}", change.entity, change.change))
            .or_default() += 1;
    }
    Ok(envelope(
        json!({ "left": l.source_json(), "right": r.source_json() }),
        json!({
            "identical": diff.is_empty(),
            "summary": summary,
            "changes": page.apply(&changes),
            "note": "A KNXBench project diff of two saved files, not an ETS comparison.",
        }),
    ))
}

fn severity_text(severity: knx_csv::Severity) -> &'static str {
    match severity {
        knx_csv::Severity::Error => "error",
        knx_csv::Severity::Warning => "warning",
    }
}

/// `validate_ga_csv`: plans a "KNXBench group-address CSV v1" text against
/// the project exactly as `knx ga-import --dry-run` would, and applies
/// nothing. The plan's command is discarded unexamined.
pub fn validate_ga_csv(
    ws: &Workspace,
    alias: &str,
    csv: &str,
    installation: Option<u8>,
) -> ToolResult {
    if csv.len() > MAX_CSV_BYTES {
        return Err(format!("csv is larger than {MAX_CSV_BYTES} bytes"));
    }
    let snapshot = ws.snapshot(alias)?;
    let project = &snapshot.project;
    let style = project.info.group_address_style;
    let parsed = knx_csv::parse_group_addresses(csv, style);
    let plan = knx_csv::plan_import_into(project, &parsed, installation.map(InstallationId));
    let report = &plan.report;
    let errors = report
        .problems
        .iter()
        .filter(|p| p.severity == knx_csv::Severity::Error)
        .count();
    let problems: Vec<Value> = report
        .problems
        .iter()
        .map(|p| json!({ "row": p.row, "severity": severity_text(p.severity), "detail": p.detail }))
        .collect();
    let destructive: Vec<Value> = report
        .destructive_changes
        .iter()
        .map(|c| {
            json!({
                "row": c.row,
                "action": match c.action {
                    knx_csv::CsvDestructiveAction::Readdress => "readdress",
                    knx_csv::CsvDestructiveAction::Delete => "delete",
                },
                "address": c.source_address.format(style),
                "newAddress": c.target_address.map(|a| a.format(style)),
                "affectedLinks": c.affected_links.len(),
            })
        })
        .collect();
    let ignored: Vec<Value> = report
        .ignored_columns
        .iter()
        .map(|c| json!({ "name": c.name, "reason": format!("{:?}", c.reason) }))
        .collect();
    Ok(envelope(
        snapshot.source_json(),
        json!({
            "valid": errors == 0,
            "wouldChangeProject": plan.command.is_some(),
            "separator": report.separator.to_string(),
            "counts": {
                "rowsRead": report.rows_read,
                "created": report.created,
                "updated": report.updated,
                "readdressed": report.readdressed,
                "deleted": report.deleted,
                "unchanged": report.unchanged,
            },
            "destructiveChanges": destructive,
            "ignoredColumns": ignored,
            "problems": problems,
            "applied": false,
            "howToApply": "Nothing was changed. A person applies the CSV with \
        `knx ga-import <project.knxdb> <file.csv> --dry-run`, reads the result, then runs it again \
        without --dry-run (deletions and re-addressing additionally need the --confirm token the dry \
        run prints), or imports it in the KNXBench UI. Close the project in KNXBench first.",
        }),
    ))
}
