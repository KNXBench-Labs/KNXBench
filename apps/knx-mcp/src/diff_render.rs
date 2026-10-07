//! Renders a `knx_diff::ProjectDiff` as one flat list of changes.
//!
//! `knx-diff` renders nothing; each calling surface turns its typed result
//! into its own form (knx-diff design spec §5). This is the MCP surface's
//! form: one entry per added, removed, changed or ambiguous entity, keyed
//! the same human-readable way `knx diff` prints them, so an agent can page
//! through a large diff.

use knx_diff::{EntityTable, FieldChange, FieldDiff, ProjectDiff};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldOut {
    pub field: &'static str,
    pub left: String,
    pub right: String,
}

impl From<FieldChange> for FieldOut {
    fn from(change: FieldChange) -> Self {
        Self {
            field: change.field,
            left: change.left,
            right: change.right,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeOut {
    /// The installation's id; `None` for project information.
    pub installation: Option<u8>,
    /// `project`, `installation`, `area`, `line`, `groupRange`,
    /// `groupAddress`, `building`, `device`, `comObject` or `parameter`.
    pub entity: &'static str,
    pub key: String,
    /// `added`, `removed`, `changed` or `ambiguous`.
    pub change: &'static str,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<FieldOut>,
    /// For `ambiguous`: how many candidates each side had for the key.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidates: Option<[usize; 2]>,
}

fn device_key(key: &knx_diff::DeviceKey) -> String {
    match &key.address {
        Some(address) => address.clone(),
        None => key.ets_id.clone().unwrap_or_else(|| "-".into()),
    }
}

fn table<K, F: FieldDiff>(
    out: &mut Vec<ChangeOut>,
    installation: Option<u8>,
    entity: &'static str,
    table: &EntityTable<K, F>,
    key: impl Fn(&K) -> String,
) {
    let plain = |k: &K, change: &'static str| ChangeOut {
        installation,
        entity,
        key: key(k),
        change,
        fields: Vec::new(),
        candidates: None,
    };
    out.extend(table.added.iter().map(|(k, _)| plain(k, "added")));
    out.extend(table.removed.iter().map(|(k, _)| plain(k, "removed")));
    for change in &table.changed {
        out.push(ChangeOut {
            fields: change
                .field_changes()
                .into_iter()
                .map(FieldOut::from)
                .collect(),
            ..plain(&change.key, "changed")
        });
    }
    for note in &table.ambiguous {
        out.push(ChangeOut {
            candidates: Some([note.left_candidates, note.right_candidates]),
            ..plain(&note.key, "ambiguous")
        });
    }
}

/// Every change in `diff`, in `knx diff`'s order.
pub fn flatten(diff: &ProjectDiff) -> Vec<ChangeOut> {
    let mut out = Vec::new();
    if !diff.info_changes.is_empty() {
        out.push(ChangeOut {
            installation: None,
            entity: "project",
            key: "info".into(),
            change: "changed",
            fields: diff
                .info_changes
                .iter()
                .cloned()
                .map(FieldOut::from)
                .collect(),
            candidates: None,
        });
    }
    for inst in &diff.installations {
        let id = Some(inst.id);
        let status = match inst.status {
            knx_diff::EntityStatus::Added => Some("added"),
            knx_diff::EntityStatus::Removed => Some("removed"),
            knx_diff::EntityStatus::Matched => None,
        };
        if status.is_some() || !inst.field_changes.is_empty() {
            out.push(ChangeOut {
                installation: id,
                entity: "installation",
                key: inst.id.to_string(),
                change: status.unwrap_or("changed"),
                fields: inst
                    .field_changes
                    .iter()
                    .cloned()
                    .map(FieldOut::from)
                    .collect(),
                candidates: None,
            });
        }
        table(&mut out, id, "area", &inst.areas, |k| k.address.to_string());
        table(&mut out, id, "line", &inst.lines, |k| {
            format!("{}.{}", k.area_address, k.line_address)
        });
        table(&mut out, id, "groupRange", &inst.group_ranges, |k| {
            format!("{}-{}", k.start, k.end)
        });
        table(&mut out, id, "groupAddress", &inst.group_addresses, |k| {
            k.address.clone()
        });
        table(&mut out, id, "building", &inst.buildings, |k| {
            k.path.join("/")
        });

        let devices = &inst.devices;
        let plain = |key: &knx_diff::DeviceKey, change: &'static str| ChangeOut {
            installation: id,
            entity: "device",
            key: device_key(key),
            change,
            fields: Vec::new(),
            candidates: None,
        };
        out.extend(devices.added.iter().map(|(k, _)| plain(k, "added")));
        out.extend(devices.removed.iter().map(|(k, _)| plain(k, "removed")));
        for change in &devices.changed {
            // A device whose only change is nested still gets its own entry,
            // as `knx diff` prints "(own fields unchanged)".
            out.push(ChangeOut {
                fields: change
                    .field_changes()
                    .into_iter()
                    .map(FieldOut::from)
                    .collect(),
                ..plain(&change.key, "changed")
            });
            let owner = device_key(&change.key);
            table(&mut out, id, "comObject", &change.com_objects, |k| {
                format!("{owner} object {}", k.number)
            });
            table(&mut out, id, "parameter", &change.parameters, |k| {
                format!("{owner} {}", k.ets_id)
            });
        }
        for note in &devices.ambiguous {
            out.push(ChangeOut {
                candidates: Some([note.left_candidates, note.right_candidates]),
                ..plain(&note.key, "ambiguous")
            });
        }
    }
    out
}
