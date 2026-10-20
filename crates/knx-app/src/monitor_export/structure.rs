//! Hierarchies and references, rendered only after complete graph admission.
use super::{context::*, validate::Selection};
use knx_core::*;
use serde_json::{json, Value};
use std::collections::BTreeMap;

fn device_refs(
    s: &Selection<'_>,
    ids: &[DeviceId],
    ctx: &mut Context,
) -> Result<Vec<String>, String> {
    strings(
        ctx,
        ids.iter().filter_map(|id| {
            s.devices
                .get(id)
                .and_then(|d| d.address)
                .map(|a| a.to_string())
        }),
    )
}
pub(super) fn topology(s: &Selection<'_>, ctx: &mut Context) -> Result<Value, String> {
    let mut areas = map();
    for a in &s.installation.topology.areas {
        let mut lines = map();
        for id in &a.lines {
            let l = s
                .installation
                .topology
                .line(*id)
                .ok_or("dangling line reference")?;
            let medium = match l.medium_ref.as_str() {
                "MT-0" => "Twisted Pair (TP)",
                "MT-1" => "Powerline (PL)",
                "MT-2" => "KNX RF (RF)",
                "MT-5" => "KNXnet/IP (IP)",
                _ => {
                    ctx.issue("unknown",&format!("L-{}",l.id.0),"medium_type","Unrecognized current medium reference; emitted Unknown, no ordinal mapping guessed.")?;
                    "Unknown"
                }
            };
            ctx.record(&[&l.name, medium], 384)?;
            ctx.gap(&format!("L-{}", l.id.0), "description")?;
            lines.insert(l.address.to_string(),json!({"name":l.name,"medium_type":medium,"description":null,"devices":device_refs(s,&l.devices,ctx)?}));
        }
        ctx.record(&[&a.name], 256)?;
        ctx.gap(&format!("A-{}", a.id.0), "description")?;
        areas.insert(
            a.address.to_string(),
            with(
                json!({"name":a.name,"description":null}),
                "lines",
                Value::Object(lines),
            ),
        );
    }
    Ok(Value::Object(areas))
}
pub(super) fn locations(s: &Selection<'_>, ctx: &mut Context) -> Result<Value, String> {
    let mut functions: BTreeMap<BuildingPartId, Vec<String>> = BTreeMap::new();
    for f in &s.functions {
        let key = format!("F-{}", f.id.0);
        ctx.record(&[&key], 16)?;
        functions.entry(f.location).or_default().push(key);
    }
    let mut result = map();
    for b in s.spaces.values().filter(|b| b.parent.is_none()) {
        let value = space(s, b, &mut functions, ctx)?;
        ctx.record(&[&b.name], 16)?;
        result.insert(b.name.clone(), value);
    }
    Ok(Value::Object(result))
}
fn space(
    s: &Selection<'_>,
    b: &BuildingPart,
    functions: &mut BTreeMap<BuildingPartId, Vec<String>>,
    ctx: &mut Context,
) -> Result<Value, String> {
    let mut children = map();
    for id in &b.children {
        let child = s.spaces.get(id).ok_or("dangling space reference")?;
        let value = space(s, child, functions, ctx)?;
        ctx.record(&[&child.name], 16)?;
        children.insert(child.name.clone(), value);
    }
    let kind = match b.kind {
        BuildingPartType::Building => "Building",
        BuildingPartType::Floor => "Floor",
        BuildingPartType::Room => "Room",
        BuildingPartType::Corridor => "Corridor",
        BuildingPartType::DistributionBoard => "DistributionBoard",
        BuildingPartType::BuildingPart => "BuildingPart",
        BuildingPartType::Stairway => "Stairway",
        BuildingPartType::Area => "Area",
        BuildingPartType::Ground => "Ground",
        BuildingPartType::Segment => "Segment",
        BuildingPartType::RoomPart => return Err("unsupported space kind RoomPart".into()),
    };
    let entity = format!("S-{}", b.id.0);
    ctx.gap(&entity, "usage_id/usage_text/description/project_uid")?;
    if b.number.is_none() {
        ctx.gap(&entity, "number")?;
    }
    ctx.record(
        &[&entity, &b.name, b.number.as_deref().unwrap_or(""), kind],
        768,
    )?;
    Ok(with(
        json!({"type":kind,"identifier":entity,"name":b.name,"usage_id":null,"usage_text":"","number":b.number.as_deref().unwrap_or(""),"description":"","project_uid":null,"devices":device_refs(s,&b.devices,ctx)?,"functions":functions.remove(&b.id).unwrap_or_default()}),
        "spaces",
        Value::Object(children),
    ))
}
pub(super) fn functions(
    p: &Project,
    s: &Selection<'_>,
    ctx: &mut Context,
) -> Result<Value, String> {
    let mut functions = map();
    for f in &s.functions {
        let key = format!("F-{}", f.id.0);
        let space_id = format!("S-{}", f.location.0);
        let mut members = map();
        for m in &f.members {
            let ga = s
                .addresses
                .get(&m.group_address)
                .ok_or("dangling function GA reference")?;
            let address = ga_key(p, ga);
            if m.role.is_none() {
                ctx.gap(&format!("FM-{}", m.id.0), "role")?;
            }
            ctx.gap(&format!("FM-{}", m.id.0), "project_uid")?;
            ctx.record(
                &[&address, &address, &m.name, m.role.as_deref().unwrap_or("")],
                256,
            )?;
            members.insert(address.clone(),json!({"address":address,"name":m.name,"project_uid":null,"role":m.role.as_deref().unwrap_or("")}));
        }
        if f.details.function_type.is_none() {
            ctx.gap(&key, "function_type")?;
        }
        ctx.gap(&key, "project_uid/usage_text")?;
        ctx.issue("unprojected",&key,"number/comment/description/completion","Normalized Function number, comment, description and completion are not represented in this monitoring profile.")?;
        ctx.record(
            &[
                &key,
                &key,
                &space_id,
                &f.details.name,
                f.details.function_type.as_deref().unwrap_or(""),
            ],
            768,
        )?;
        functions.insert(key.clone(),with(json!({"function_type":f.details.function_type.as_deref().unwrap_or(""),"identifier":key,"name":f.details.name,"project_uid":null,"space_id":space_id,"usage_text":""}),"group_addresses",Value::Object(members)));
    }
    Ok(Value::Object(functions))
}
pub(super) fn ranges(p: &Project, s: &Selection<'_>, ctx: &mut Context) -> Result<Value, String> {
    let mut direct: BTreeMap<GroupRangeId, Vec<String>> = BTreeMap::new();
    for ga in s.addresses.values() {
        if let Some(id) = ga.range {
            let key = ga_key(p, ga);
            ctx.record(&[&key], 16)?;
            direct.entry(id).or_default().push(key);
        }
    }
    let mut ranges = map();
    for r in s.ranges.values().filter(|r| r.parent.is_none()) {
        let value = range(s, r, &mut direct, ctx)?;
        ranges.insert(format!("R-{}", r.id.0), value);
    }
    Ok(Value::Object(ranges))
}
fn range(
    s: &Selection<'_>,
    r: &GroupRange,
    direct: &mut BTreeMap<GroupRangeId, Vec<String>>,
    ctx: &mut Context,
) -> Result<Value, String> {
    let mut children = map();
    for id in &r.children {
        let child = s.ranges.get(id).ok_or("dangling range reference")?;
        let value = range(s, child, direct, ctx)?;
        children.insert(format!("R-{}", id.0), value);
    }
    ctx.record(&[&r.name], 384)?;
    ctx.gap(&format!("R-{}", r.id.0), "comment")?;
    Ok(with(
        json!({"name":r.name,"address_start":r.start.raw(),"address_end":r.end.raw(),"comment":"","group_addresses":direct.remove(&r.id).unwrap_or_default()}),
        "group_ranges",
        Value::Object(children),
    ))
}
