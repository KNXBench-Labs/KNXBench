//! Plans an additive source-scoped merge while preserving every selected dependency.

use std::collections::{BTreeMap, BTreeSet};

use knx_core::*;
use knx_store::project_history::NativeSnapshot;

use super::{scoped, structure, Counts, IdMapping, Plan, Preview, Selection, Source};

pub(super) fn plan(
    before: &NativeSnapshot,
    source: &Source,
    selection: Selection,
) -> Result<Plan, String> {
    // This closure enumerates model-v11 references. A model bump needs a new audit,
    // not accidental acceptance of newly cloned but unremapped entity fields.
    if before.project.schema_version != super::SELECTIVE_MODEL_VERSION
        || source.project.schema_version != super::SELECTIVE_MODEL_VERSION
    {
        return Err("selective import supports normalized model v11 only; a new model requires an audited dependency mapping".into());
    }
    before.encode().map_err(|e| e.to_string())?;
    if source.report.error_count() != 0 {
        return Err("source import has mapping/validation errors; repair the source before selective import".into());
    }
    let source_installation = unique(
        source
            .project
            .installations
            .iter()
            .filter(|i| i.id.0 == selection.source_installation),
        "source installation",
    )?;
    let target_index = unique(
        before
            .project
            .installations
            .iter()
            .enumerate()
            .filter(|(_, i)| i.id.0 == selection.target_installation)
            .map(|(index, _)| index),
        "target installation",
    )?;
    let explicit_devices = selected(&selection.devices, "device")?;
    let explicit_lines = selected(&selection.lines, "line")?;
    if explicit_devices.is_empty() && explicit_lines.is_empty() {
        return Err("empty import selection".into());
    }
    let mut devices: BTreeSet<_> = explicit_devices.iter().copied().map(DeviceId).collect();
    let mut lines: BTreeSet<_> = explicit_lines.iter().copied().map(LineId).collect();
    for id in &lines {
        let line = unique(
            source_installation
                .topology
                .lines
                .iter()
                .filter(|l| l.id == *id),
            "selected line",
        )?;
        devices.extend(line.devices.iter().copied());
    }
    for id in &devices {
        let _ = source
            .project
            .devices
            .get(*id)
            .ok_or_else(|| format!("selected device {id} not found"))?;
        let owners: Vec<_> = source
            .project
            .installations
            .iter()
            .flat_map(|i| {
                i.topology
                    .lines
                    .iter()
                    .flat_map(move |l| {
                        l.devices
                            .iter()
                            .filter(move |d| **d == *id)
                            .map(move |_| (i.id, Some(l.id)))
                    })
                    .chain(
                        i.topology
                            .unassigned
                            .iter()
                            .filter(move |d| **d == *id)
                            .map(move |_| (i.id, None)),
                    )
            })
            .collect();
        let (owner, line) = unique(owners.into_iter(), "selected device placement")?;
        if owner != source_installation.id {
            return Err("selected device is outside the source installation".into());
        }
        if let Some(line) = line {
            lines.insert(line);
        }
    }
    let mut buildings = BTreeSet::new();
    for i in &source.project.installations {
        for b in &i.buildings {
            if b.devices.iter().any(|id| devices.contains(id)) {
                if i.id != source_installation.id {
                    return Err("selected building reference crosses installations".into());
                }
                structure::ancestors(b.id, &mut buildings, |id| {
                    unique(
                        source_installation.buildings.iter().filter(|b| b.id == id),
                        "building part",
                    )
                    .map(|b| b.parent)
                })?;
            }
        }
    }
    for b in source_installation
        .buildings
        .iter()
        .filter(|b| buildings.contains(&b.id))
    {
        if let Some(line) = b.default_line {
            lines.insert(line);
        }
    }
    let mut after = before.clone();
    let mut mappings = vec![];
    let mut notes=vec!["Original archive and opaque evidence are retained, including unselected source data. Undo removes imported model data, not retained source evidence.".into()];
    let mut counts = Counts::default();
    let mut device_map = BTreeMap::new();
    for old in &devices {
        let device = source
            .project
            .devices
            .get(*old)
            .ok_or("selected device not found")?;
        if let Some(address) = device.address {
            if before.project.installations[target_index]
                .topology
                .lines
                .iter()
                .flat_map(|l| &l.devices)
                .chain(
                    &before.project.installations[target_index]
                        .topology
                        .unassigned,
                )
                .filter_map(|id| before.project.devices.get(*id))
                .any(|d| d.address == Some(address))
            {
                return Err(format!(
                    "individual address {address} already exists in the target installation"
                ));
            }
            if devices
                .iter()
                .filter_map(|id| source.project.devices.get(*id))
                .filter(|d| d.address == Some(address))
                .count()
                != 1
            {
                return Err(format!("duplicate selected individual address {address}"));
            }
        }
        let new = after
            .project
            .ids
            .next_device_id()
            .map_err(|e| e.to_string())?;
        device_map.insert(*old, new);
        mappings.push(mapping("device", old.0, new.0, false));
    }
    let (line_map, building_map) = structure::topology_and_buildings(
        &mut after.project,
        structure::PlacementRequest {
            source: source_installation,
            target_index,
            lines: &lines,
            buildings: &buildings,
            devices: &device_map,
            prefix: &source.prefix,
        },
        &mut mappings,
        &mut notes,
    )?;
    let mut group_ids = BTreeSet::new();
    let mut com_ids = BTreeSet::new();
    for id in &devices {
        let device = source
            .project
            .devices
            .get(*id)
            .ok_or("selected device not found")?;
        let owned: BTreeSet<_> = source
            .project
            .devices
            .com_objects()
            .filter(|c| c.device == *id)
            .map(|c| c.id)
            .collect();
        let listed: BTreeSet<_> = device.com_objects.iter().copied().collect();
        if owned != listed || listed.len() != device.com_objects.len() {
            return Err("selected device communication-object ownership is ambiguous".into());
        }
        com_ids.extend(listed);
    }
    for id in &com_ids {
        let com = source
            .project
            .devices
            .com_object(*id)
            .ok_or("selected communication object not found")?;
        for link in &com.links {
            group_ids.insert(link.ga);
        }
    }
    let group_map = structure::groups(
        &mut after.project,
        source_installation,
        target_index,
        &group_ids,
        &source.prefix,
        &mut mappings,
    )?;
    let mut module_map = BTreeMap::new();
    for module in source
        .project
        .devices
        .module_instances()
        .filter(|m| devices.contains(&m.device))
    {
        let new = after
            .project
            .ids
            .next_module_instance_id()
            .map_err(|e| e.to_string())?;
        module_map.insert(module.id, new);
        let mut copied = module.clone();
        copied.id = new;
        copied.device = device_map[&module.device];
        copied.source = scoped(&module.source, &source.prefix);
        for (reference, _) in &mut copied.arguments {
            *reference = scoped(reference, &source.prefix);
        }
        after.project.devices.insert_module_instance(copied);
        mappings.push(mapping("module", module.id.0, new.0, false));
        counts.modules += 1;
    }
    let mut com_map = BTreeMap::new();
    let mut text_keys = BTreeSet::new();
    for id in &com_ids {
        let com = source
            .project
            .devices
            .com_object(*id)
            .ok_or("selected communication object not found")?;
        let new = after
            .project
            .ids
            .next_com_object_instance_id()
            .map_err(|e| e.to_string())?;
        let mut copied = com.clone();
        copied.id = new;
        copied.device = device_map[&com.device];
        copied.source = scoped(&com.source, &source.prefix);
        for link in &mut copied.links {
            link.ga = *group_map
                .get(&link.ga)
                .ok_or("selected group address is missing")?;
        }
        copied.module_instance = com
            .module_instance
            .map(|id| {
                module_map
                    .get(&id)
                    .copied()
                    .ok_or("selected module reference is missing or belongs to another device")
            })
            .transpose()?;
        if let Some(mid) = com.module_instance {
            if source
                .project
                .devices
                .module_instance(mid)
                .is_none_or(|m| m.device != com.device)
            {
                return Err("selected module reference has inconsistent ownership".into());
            }
        }
        collect_text(&copied.text, &mut text_keys);
        collect_text(&copied.description, &mut text_keys);
        after.project.devices.insert_com_object(copied);
        if let Some(defaults) = source.project.devices.program_defaults(*id) {
            if let Some(t) = &defaults.text {
                collect_value(&t.value, &mut text_keys);
            }
            if let Some(t) = &defaults.description {
                collect_value(&t.value, &mut text_keys);
            }
            after
                .project
                .devices
                .set_program_defaults(new, defaults.clone());
        }
        com_map.insert(*id, new);
        mappings.push(mapping("communication_object", id.0, new.0, false));
        counts.communication_objects += 1;
    }
    for old in &devices {
        let device = source
            .project
            .devices
            .get(*old)
            .ok_or("selected device not found")?;
        let mut copied = device.clone();
        copied.id = device_map[old];
        copied.source = scoped(&device.source, &source.prefix);
        copied.com_objects = device.com_objects.iter().map(|id| com_map[id]).collect();
        after.project.devices.insert(copied);
        let source_line = source_installation
            .topology
            .lines
            .iter()
            .find(|l| l.devices.contains(old));
        if let Some(line) = source_line {
            if let Some(address) = device.address {
                let area = unique(
                    source_installation
                        .topology
                        .areas
                        .iter()
                        .filter(|a| a.lines.contains(&line.id)),
                    "source line owner",
                )?;
                if address.area() != area.address || address.line() != line.address {
                    return Err("selected device address does not match its source line".into());
                }
            }
        }
        counts.devices += 1;
    }
    // Preserve source vector order; numeric IDs are identities, never positions.
    for line in &source_installation.topology.lines {
        if let Some(mapped) = line_map.get(&line.id) {
            let destination = after.project.installations[target_index]
                .topology
                .lines
                .iter_mut()
                .find(|l| l.id == *mapped)
                .ok_or("target line missing")?;
            destination.devices.extend(
                line.devices
                    .iter()
                    .filter_map(|id| device_map.get(id).copied()),
            );
        }
    }
    after.project.installations[target_index]
        .topology
        .unassigned
        .extend(
            source_installation
                .topology
                .unassigned
                .iter()
                .filter_map(|id| device_map.get(id).copied()),
        );
    for i in &source.project.installations {
        for parameter in i.parameters.iter().filter(|p| devices.contains(&p.device)) {
            if i.id != source_installation.id {
                return Err("selected parameter reference crosses installations".into());
            }
            let mut copied = parameter.clone();
            copied.id = after
                .project
                .ids
                .next_parameter_instance_id()
                .map_err(|e| e.to_string())?;
            copied.device = device_map[&parameter.device];
            copied.source = scoped(&parameter.source, &source.prefix);
            mappings.push(mapping("parameter", parameter.id.0, copied.id.0, false));
            after.project.installations[target_index]
                .parameters
                .push(copied);
            counts.parameters += 1;
        }
    }
    if !text_keys.is_empty()
        && source.project.strings.default_language() != after.project.strings.default_language()
    {
        return Err("localized source text has a different fallback language; align languages before selective import".into());
    }
    for (key, language, value) in source
        .project
        .strings
        .iter()
        .filter(|(k, _, _)| text_keys.contains(&k.0))
    {
        if let Some((_, _, current)) = after
            .project
            .strings
            .iter()
            .find(|(k, l, _)| *k == key && *l == language)
        {
            if current != value {
                return Err("conflicting localized source text".into());
            }
        }
        after
            .project
            .strings
            .insert(key.clone(), language.clone(), value.into());
    }
    counts.lines = line_map.len();
    counts.building_parts = building_map.len();
    counts.group_addresses = group_map.len();
    for entry in &source.opaque {
        if !after.opaque.contains(entry) {
            after.opaque.push(entry.clone());
        }
    }
    for reference in &source.manufacturer_refs {
        if !after.manufacturer_refs.contains(reference) {
            after.manufacturer_refs.push(reference.clone());
        }
    }
    // Exact reopen admission catches unsupported shapes and allocator/reference loss.
    after.encode().map_err(|e| e.to_string())?;
    Ok(Plan {
        before: before.clone(),
        after,
        preview: Preview {
            source_hash: source.source_hash.clone(),
            selection,
            counts,
            mappings,
            source_report: source.report.clone(),
            retained_source_may_contain_unselected_data: true,
            retained_source_entries: source.opaque.len(),
            notes,
        },
    })
}

pub(super) fn unique<T>(mut values: impl Iterator<Item = T>, kind: &str) -> Result<T, String> {
    let value = values.next().ok_or_else(|| format!("{kind} not found"))?;
    if values.next().is_some() {
        return Err(format!("ambiguous {kind}"));
    }
    Ok(value)
}

fn selected(ids: &[u32], kind: &str) -> Result<BTreeSet<u32>, String> {
    let unique: BTreeSet<_> = ids.iter().copied().collect();
    if unique.len() != ids.len() {
        return Err(format!("duplicate selected {kind}"));
    }
    Ok(unique)
}

pub(super) fn mapping(kind: &'static str, source: u32, target: u32, reused: bool) -> IdMapping {
    IdMapping {
        kind,
        source,
        target,
        reused,
    }
}
fn collect_text(text: &Override<Text>, keys: &mut BTreeSet<String>) {
    if let Some(value) = text.value() {
        collect_value(&value.value, keys);
    }
}
fn collect_value(text: &Text, keys: &mut BTreeSet<String>) {
    if let Text::Localized(handle) = text {
        keys.insert(handle.0 .0.clone());
    }
}
