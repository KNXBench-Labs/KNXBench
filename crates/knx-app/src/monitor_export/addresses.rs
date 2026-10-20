//! Group addresses: declaration-aware Core classification, not decoder guesses.
use super::{context::*, validate::Selection};
use knx_core::*;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn project(p: &Project, s: &Selection<'_>, ctx: &mut Context) -> Result<Value, String> {
    let mut linked: BTreeMap<GroupAddressId, Vec<DptRef>> = BTreeMap::new();
    let mut reverse: BTreeMap<GroupAddressId, Vec<String>> = BTreeMap::new();
    for d in s.devices.values() {
        for &id in &d.com_objects {
            let o = p
                .devices
                .com_object(id)
                .ok_or("dangling object reference")?;
            let mut seen = BTreeSet::new();
            for link in &o.links {
                if seen.insert(link.ga) {
                    if let Some(dpt) = o.dpt.value() {
                        ctx.record(&[], 16)?;
                        linked.entry(link.ga).or_default().push(dpt.value);
                    }
                    if let Some(address) = d.address {
                        let key = format!("{address}/O-{}", o.id.0);
                        ctx.record(&[&key], 16)?;
                        reverse.entry(link.ga).or_default().push(key);
                    }
                }
            }
        }
    }
    let mut addresses = map();
    for g in s.addresses.values() {
        let entity = format!("GA-{}", g.id.0);
        let declared = match &g.declared_dpt {
            Override::Value(r) => Override::Value(*r),
            Override::Absent => Override::Absent,
            Override::Empty => Override::Empty,
            Override::Malformed(_) => Override::Malformed(String::new()),
        };
        let linked = group_address_dpt_from(linked.remove(&g.id).unwrap_or_default());
        let candidate_count = match &linked {
            GroupAddressDpt::Conflict(v) => v.len(),
            GroupAddressDpt::Single(_) => 1,
            GroupAddressDpt::None => 0,
        };
        ctx.record(
            &[],
            candidate_count
                .checked_mul(256)
                .ok_or("generated output limit exceeded")?,
        )?;
        let resolved = group_address_type_from(
            &declared,
            linked,
            p.info.unlifted_group_address_dpt_declarations > 0,
        );
        let effective = resolved.effective();
        let projected = match &effective {
            GroupAddressDpt::Single(v) => dpt(*v),
            _ => Value::Null,
        };
        let code = match resolved.outcome {
            GroupAddressTypeOutcome::Inferred | GroupAddressTypeOutcome::DeclarationNotLifted
                if matches!(effective, GroupAddressDpt::Single(_)) =>
            {
                "inferred"
            }
            GroupAddressTypeOutcome::SizeConflict => "dpt_conflict",
            _ => "provenance",
        };
        // Candidates are bounded numeric model data, never raw malformed values.
        // Admit before formatting Debug's potentially long candidate list.
        let candidate_count = match &resolved.linked {
            GroupAddressDpt::Conflict(v) => v.len(),
            GroupAddressDpt::Single(_) => 1,
            GroupAddressDpt::None => 0,
        };
        ctx.record(
            &[],
            candidate_count
                .checked_mul(128)
                .ok_or("generated output limit exceeded")?,
        )?;
        let declaration = g.declared_dpt.value().map(|v| (v.value, v.layer));
        ctx.issue(code,&entity,"dpt",&format!("Core outcome {:?}; declared {:?}; linked {:?}. Only effective Single is emitted; no decode support implied.",resolved.outcome,declaration,resolved.linked))?;
        ctx.issue("unknown_omitted",&entity,"data_secure","No normalized group-security authority; data_secure always omitted, never inferred from missing keys.")?;
        ctx.gap(&entity, "project_uid/description/comment")?;
        let key = ga_key(p, g);
        let ids = reverse.remove(&g.id).unwrap_or_default();
        ctx.record(&[&g.name, &entity, &key, &key], 768)?;
        addresses.insert(key.clone(),json!({"name":g.name,"identifier":entity,"raw_address":g.address.raw(),"address":key,"project_uid":null,"dpt":projected,"communication_object_ids":ids,"description":"","comment":""}));
    }
    Ok(Value::Object(addresses))
}
