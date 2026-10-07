//! Single-entity tools: a device, one of its parameters, a group address.

use std::collections::HashMap;

use knx_core::{DeviceId, GroupAddress, IndividualAddress, ParameterInstance, Project};
use knx_productdb::query::ParameterView;
use serde_json::{json, Value};

use super::{envelope, ToolResult};
use crate::workspace::{Snapshot, Workspace};

/// How many stored parameter values `get_device` lists inline.
pub const MAX_DEVICE_PARAMETERS: usize = 200;

/// Resolves `#12`/`12` (a device id) or `1.1.5` (an individual address).
/// Several devices sharing an address is an error that names them all.
pub fn resolve_device(project: &Project, reference: &str) -> Result<DeviceId, String> {
    let reference = reference.trim();
    let id_text = reference.strip_prefix('#').unwrap_or(reference);
    if let Ok(id) = id_text.parse::<u32>() {
        return project
            .devices
            .get(DeviceId(id))
            .map(|d| d.id)
            .ok_or_else(|| format!("no device with id {id}"));
    }
    let address: IndividualAddress = reference.parse().map_err(|_| {
        format!("device {reference:?}: expected an id such as #12 or an individual address such as 1.1.5")
    })?;
    let found: Vec<_> = project
        .devices
        .iter()
        .filter(|d| d.address == Some(address))
        .collect();
    match found.as_slice() {
        [] => Err(format!("no device has individual address {address}")),
        [one] => Ok(one.id),
        several => Err(format!(
            "{} devices share individual address {address}; name one by id: {}",
            several.len(),
            several
                .iter()
                .map(|d| format!("#{} {:?}", d.id, d.name))
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

fn stored_parameters(project: &Project, device: DeviceId) -> Vec<&ParameterInstance> {
    project
        .installations
        .iter()
        .flat_map(|i| &i.parameters)
        .filter(|p| p.device == device)
        .collect()
}

/// Where a device's program stands in the product database, plus its
/// declared parameters when the program is installed.
struct ProgramLookup {
    state: &'static str,
    program_id: Option<String>,
    views: HashMap<String, ParameterView>,
}

fn lookup_program(ws: &Workspace, program_ref: &str) -> Result<ProgramLookup, String> {
    let none = |state| ProgramLookup {
        state,
        program_id: None,
        views: HashMap::new(),
    };
    if program_ref.is_empty() {
        return Ok(none("noApplicationProgram"));
    }
    let Some(db) = ws.products() else {
        return Ok(none("noProductDatabase"));
    };
    let conn = db.conn.lock().expect("product database mutex poisoned");
    let Some(program_id) =
        knx_productdb::query::resolve_program(&conn, program_ref).map_err(|e| e.to_string())?
    else {
        return Ok(none("programNotInstalled"));
    };
    let views = knx_productdb::query::parameter_views(&conn, &program_id, None)
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|view| (view.id.clone(), view))
        .collect();
    Ok(ProgramLookup {
        state: "resolved",
        program_id: Some(program_id),
        views,
    })
}

/// The option text a `Restriction` value selects, when the program has one.
fn value_text(view: &ParameterView, raw: &str) -> Option<String> {
    view.enum_options
        .iter()
        .find(|(value, _)| value == raw)
        .and_then(|(_, text)| text.clone())
}

fn device_location(snapshot: &Snapshot, id: u32) -> Value {
    for installation in &snapshot.tree.installations {
        for area in &installation.topology {
            for line in &area.lines {
                if line.devices.iter().any(|d| d.id == id) {
                    return json!({
                        "installation": installation.name,
                        "area": { "address": area.address, "name": area.name },
                        "line": { "address": line.address, "name": line.name },
                        "buildingPaths": building_paths(&installation.buildings, id),
                    });
                }
            }
        }
        if installation.unassigned.iter().any(|d| d.id == id) {
            return json!({
                "installation": installation.name,
                "line": null,
                "buildingPaths": building_paths(&installation.buildings, id),
            });
        }
    }
    Value::Null
}

/// Every root-first `kind name` path to a building part holding `id`.
fn building_paths(roots: &[knx_projection::BuildingNode], id: u32) -> Vec<Vec<String>> {
    fn walk(
        node: &knx_projection::BuildingNode,
        id: u32,
        path: &mut Vec<String>,
        out: &mut Vec<Vec<String>>,
    ) {
        path.push(format!("{} {}", node.kind, node.name));
        if node.devices.iter().any(|d| d.id == id) {
            out.push(path.clone());
        }
        for child in &node.children {
            walk(child, id, path, out);
        }
        path.pop();
    }
    let mut out = Vec::new();
    for root in roots {
        walk(root, id, &mut Vec::new(), &mut out);
    }
    out
}

/// `get_device`: the device as the UI's detail panel shows it, its place in
/// topology and buildings, and its stored parameter values.
pub fn get_device(ws: &Workspace, alias: &str, reference: &str) -> ToolResult {
    let snapshot = ws.snapshot(alias)?;
    let project = &snapshot.project;
    let id = resolve_device(project, reference)?;
    let device = project.devices.get(id).ok_or("device vanished")?;
    let detail = knx_projection::build_device_detail(project, id).ok_or("device vanished")?;
    let program = lookup_program(ws, &device.program_ref)?;
    let stored = stored_parameters(project, id);
    let parameters: Vec<Value> = stored
        .iter()
        .take(MAX_DEVICE_PARAMETERS)
        .map(|p| {
            let view = program.views.get(&p.source.ets_id);
            json!({
                "refId": p.source.ets_id,
                "raw": p.raw,
                "text": view.and_then(|v| v.text.clone()),
                "kind": view.map(|v| v.kind.clone()),
                "valueText": view.and_then(|v| value_text(v, &p.raw)),
            })
        })
        .collect();
    let detail = serde_json::to_value(&detail).map_err(|e| e.to_string())?;
    Ok(envelope(
        snapshot.source_json(),
        json!({
            "device": detail,
            "location": device_location(&snapshot, id.0),
            "productDatabase": { "state": program.state, "programId": program.program_id },
            "parameters": {
                "stored": stored.len(),
                "truncated": stored.len() > MAX_DEVICE_PARAMETERS,
                "items": parameters,
                "visibility": "notEvaluated",
            },
        }),
    ))
}

/// `explain_parameter`: one parameter's declaration and stored value.
/// `parameter` is a `ParameterRef` id as `get_device` lists it; when it
/// matches none, text matches are offered instead of a guess.
pub fn explain_parameter(
    ws: &Workspace,
    alias: &str,
    reference: &str,
    parameter: &str,
) -> ToolResult {
    let snapshot = ws.snapshot(alias)?;
    let project = &snapshot.project;
    let id = resolve_device(project, reference)?;
    let device = project.devices.get(id).ok_or("device vanished")?;
    let program = lookup_program(ws, &device.program_ref)?;
    let parameter = parameter.trim();
    let stored = stored_parameters(project, id)
        .into_iter()
        .find(|p| p.source.ets_id == parameter);
    let view = program.views.get(parameter);

    if stored.is_none() && view.is_none() {
        let needle = parameter.to_lowercase();
        let mut candidates: Vec<(&String, &ParameterView)> = program
            .views
            .iter()
            .filter(|(_, v)| {
                v.text
                    .as_deref()
                    .is_some_and(|t| t.to_lowercase().contains(&needle))
                    || v.name
                        .as_deref()
                        .is_some_and(|n| n.to_lowercase().contains(&needle))
            })
            .collect();
        candidates.sort_by(|a, b| a.0.cmp(b.0));
        let listed: Vec<String> = candidates
            .iter()
            .take(20)
            .map(|(id, v)| format!("{id} ({})", v.text.as_deref().unwrap_or("-")))
            .collect();
        return Err(if listed.is_empty() {
            format!(
                "device #{id} has no parameter {parameter:?} (product database: {}); use a refId from get_device",
                program.state
            )
        } else {
            format!(
                "no parameter refId {parameter:?}; text matches: {}",
                listed.join("; ")
            )
        });
    }

    let mut notes: Vec<&str> = vec![
        "Whether this parameter is currently visible or active is not evaluated (the program's Dynamic tree is not run).",
    ];
    match program.state {
        "noProductDatabase" => notes.push("No product database: the value's meaning is unknown."),
        "programNotInstalled" => notes.push(
            "The device's application program is not installed: the value's meaning is unknown.",
        ),
        "noApplicationProgram" => notes.push("The device names no application program."),
        _ => {}
    }
    if stored.is_some() && view.is_none() && program.state == "resolved" {
        notes.push("The program declares no parameter with this refId; it may belong to a module instance.");
    }
    if stored.is_none() {
        notes.push("The project stores no value for this parameter; the program default applies and is not shown here.");
    }
    let definition = view.map(|v| {
        let options: Vec<Value> = v
            .enum_options
            .iter()
            .map(|(value, text)| {
                json!({
                    "value": value,
                    "text": text,
                    "selected": stored.is_some_and(|s| &s.raw == value),
                })
            })
            .collect();
        json!({
            "name": v.name,
            "text": v.text,
            "kind": v.kind,
            "access": v.access,
            "refAccess": v.ref_access,
            "minInclusive": v.min_inclusive,
            "maxInclusive": v.max_inclusive,
            "sizeInBit": v.size_in_bit,
            "options": options,
        })
    });
    Ok(envelope(
        snapshot.source_json(),
        json!({
            "device": {
                "id": id.0,
                "name": device.name,
                "address": device.address.map(|a| a.to_string()),
            },
            "refId": parameter,
            "storedValue": stored.map(|s| s.raw.clone()),
            "valueText": match (view, stored) {
                (Some(v), Some(s)) => value_text(v, &s.raw),
                _ => None,
            },
            "definition": definition,
            "productDatabase": { "state": program.state, "programId": program.program_id },
            "visibility": "notEvaluated",
            "notes": notes,
        }),
    ))
}

/// The root-first names of the ranges containing `range`.
fn range_path(
    installation: &knx_core::Installation,
    mut range: Option<knx_core::GroupRangeId>,
) -> Vec<String> {
    let mut path = Vec::new();
    // Bounded by the range count, so a malformed parent cycle cannot hang.
    for _ in 0..=installation.group_ranges.len() {
        let Some(id) = range else { break };
        let Some(node) = installation.group_ranges.iter().find(|r| r.id == id) else {
            break;
        };
        path.push(node.name.clone());
        range = node.parent;
    }
    path.reverse();
    path
}

/// `get_group_address`: `#12` names an id; anything else is an address in
/// the project's own notation (a bare number is an address in free style,
/// never an id). Every installation holding it is listed, with links,
/// datapoint types and the range path.
pub fn get_group_address(ws: &Workspace, alias: &str, reference: &str) -> ToolResult {
    let snapshot = ws.snapshot(alias)?;
    let project = &snapshot.project;
    let reference = reference.trim();
    let style = project.info.group_address_style;
    let wanted: Box<dyn Fn(&knx_core::GroupAddressEntry) -> bool> =
        match reference.strip_prefix('#') {
            Some(id) => {
                let id: u32 = id
                    .parse()
                    .map_err(|_| format!("{reference:?} is not a group address id"))?;
                Box::new(move |entry| entry.id.0 == id)
            }
            None => {
                let address = GroupAddress::parse(reference, style).map_err(|_| {
                    format!(
                        "{reference:?} is not a group address in this project's notation ({}); \
                         use e.g. {} or #<id>",
                        snapshot.tree.group_address_style,
                        GroupAddress::from_raw(0x0801).format(style)
                    )
                })?;
                Box::new(move |entry| entry.address == address)
            }
        };
    let mut matches = Vec::new();
    for (installation, node) in project
        .installations
        .iter()
        .zip(&snapshot.tree.installations)
    {
        for entry in installation.group_addresses.iter().filter(|e| wanted(e)) {
            let Some(view) = node.group_addresses.iter().find(|g| g.id == entry.id.0) else {
                continue;
            };
            matches.push(json!({
                "installation": installation.name,
                "groupAddress": view,
                "central": entry.central,
                "unfiltered": entry.unfiltered,
                "rangePath": range_path(installation, entry.range),
            }));
        }
    }
    if matches.is_empty() {
        return Err(format!("no group address {reference} in project {alias:?}"));
    }
    Ok(envelope(
        snapshot.source_json(),
        json!({ "matches": matches }),
    ))
}
