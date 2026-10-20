//! Project current device/object values with metadata-only, read-only catalogue access.
use super::{context::*, validate::Selection, ExportCounts};
use knx_core::*;
use serde_json::{json, Value};

#[cfg(test)]
mod tests;

pub(super) fn project(
    p: &Project,
    s: &Selection<'_>,
    products: Option<&knx_productdb::Connection>,
    language: &Language,
    ctx: &mut Context,
    counts: &mut ExportCounts,
) -> Result<(Value, Value), String> {
    let mut devices = map();
    let mut objects = map();
    for d in s.devices.values() {
        let entity = format!("D-{}", d.id.0);
        counts.devices.selected += 1;
        counts.communication_objects.selected += d.com_objects.len() as u64;
        let Some(address) = d.address else {
            counts.devices.omitted += 1;
            counts.communication_objects.omitted += d.com_objects.len() as u64;
            ctx.issue("omitted",&entity,"individual_address","No current individual address; whole device, its objects and placement references omitted.")?;
            for id in &d.com_objects {
                ctx.issue(
                    "omitted",
                    &format!("O-{}", id.0),
                    "device_address",
                    "Owner has no current individual address; whole object omitted.",
                )?;
            }
            continue;
        };
        let row = if let Some(conn) = products {
            knx_productdb::query::device_product(
                conn,
                &d.product_ref,
                &d.program_ref,
                Some(&language.0),
            )
            .map_err(|_| format!("product metadata query failed for {entity}"))?
        } else {
            None
        };
        let display = row
            .as_ref()
            .and_then(|r| {
                r.catalog_item_name
                    .as_deref()
                    .filter(|s| !s.trim().is_empty())
                    .or(r.product_text.as_deref().filter(|s| !s.trim().is_empty()))
                    .or(r.hardware_name.as_deref())
            })
            .unwrap_or("");
        let order = row
            .as_ref()
            .and_then(|r| r.order_number.as_deref())
            .unwrap_or("");
        let manufacturer = row
            .as_ref()
            .and_then(|r| r.manufacturer_name.as_deref())
            .unwrap_or("");
        let application = row
            .as_ref()
            .and_then(|r| r.application_program_id.as_deref());
        if row.is_none() {
            ctx.gap(&entity, "product_metadata")?;
        } else {
            ctx.issue("catalogue_metadata",&entity,"product_metadata","Catalogue display/product metadata only; current project edits retain precedence. No object/default/flag enrichment performed.")?;
        }
        for (field, missing) in [
            ("hardware_name", display.is_empty()),
            ("order_number", order.is_empty()),
            ("manufacturer_name", manufacturer.is_empty()),
            ("application", application.is_none()),
            ("description", d.description.is_none()),
        ] {
            if missing {
                ctx.gap(&entity, field)?;
            }
        }
        ctx.issue(
            "unprojected",
            &entity,
            "channels",
            "Channel tree and functional-block metadata are not projected; channels is empty.",
        )?;
        ctx.gap(&entity, "project_uid")?;
        let mut ids = Vec::new();
        for &id in &d.com_objects {
            let o = p
                .devices
                .com_object(id)
                .ok_or("dangling object reference after admission")?;
            let key = format!("{address}/O-{}", o.id.0);
            ctx.record(&[&key], 16)?;
            ids.push(key.clone());
            let obj = object(p, s, d, o, application, language, ctx)?;
            if objects.insert(key, obj).is_some() {
                return Err("duplicate object dictionary key".into());
            }
            counts.communication_objects.exported += 1;
        }
        ctx.record(
            &[
                &d.name,
                display,
                order,
                manufacturer,
                d.description.as_deref().unwrap_or(""),
                application.unwrap_or(""),
            ],
            768,
        )?;
        devices.insert(address.to_string(),json!({"name":d.name,"hardware_name":display,"order_number":order,"description":d.description.as_deref().unwrap_or(""),"manufacturer_name":manufacturer,"individual_address":address.to_string(),"application":application,"project_uid":null,"communication_object_ids":ids,"channels":{}}));
        counts.devices.exported += 1;
    }
    Ok((Value::Object(devices), Value::Object(objects)))
}
fn object(
    p: &Project,
    s: &Selection<'_>,
    d: &DeviceInstance,
    o: &ComObjectInstance,
    application: Option<&str>,
    language: &Language,
    ctx: &mut Context,
) -> Result<Value, String> {
    let entity = format!("O-{}", o.id.0);
    let description = text(ctx, p, language, &entity, "description", &o.description)?;
    let text = text(ctx, p, language, &entity, "text", &o.text)?;
    let mut flags = map();
    for (name, flag) in [
        ("read", &o.flags.read),
        ("write", &o.flags.write),
        ("communication", &o.flags.communication),
        ("transmit", &o.flags.transmit),
        ("update", &o.flags.update),
        ("read_on_init", &o.flags.read_on_init),
    ] {
        if let Override::Value(r) = flag {
            flags.insert(name.into(), json!(r.value));
        } else {
            ctx.issue(
                "unknown_omitted",
                &entity,
                &format!("flags.{name}"),
                "Current flag is absent, cleared or malformed; omitted, never replaced by false.",
            )?;
        }
    }
    let dpts = if let Some(v) = o.dpt.value() {
        ctx.issue("provenance",&entity,"dpts",&format!("Current scalar DPT layer {:?}; no program-default resurrection and no decode support implied.",v.layer))?;
        vec![dpt(v.value)]
    } else {
        ctx.issue("unavailable",&entity,"dpts","Current DPT absent, explicitly cleared or malformed; empty list, no program default restored.")?;
        vec![]
    };
    let size = match o.size.as_ref().map(|r| r.value) {
        Some(ObjectSize::Bit(n)) if n > 0 => format!("{n} Bit"),
        Some(ObjectSize::Byte(1)) => "1 Byte".into(),
        Some(ObjectSize::Byte(n)) if n > 0 => format!("{n} Bytes"),
        _ => {
            ctx.gap(&entity, "object_size")?;
            String::new()
        }
    };
    let mut links = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for direction in [Direction::Send, Direction::Receive] {
        for link in o.links.iter().filter(|l| l.direction == direction) {
            if seen.insert(link.ga) {
                let ga = s
                    .addresses
                    .get(&link.ga)
                    .ok_or("foreign group address reference")?;
                let key = ga_key(p, ga);
                ctx.record(&[&key], 16)?;
                links.push(key);
            } else {
                ctx.issue(
                    "direction_collapsed",
                    &entity,
                    "group_address_links",
                    "Same GA used in both directions appears once; direction is not represented.",
                )?;
            }
        }
    }
    if !o.links.is_empty() {
        ctx.issue(
            "direction_collapsed",
            &entity,
            "group_address_links",
            "Ordered send links precede ordered receive links; wire links do not retain direction.",
        )?;
    }
    ctx.issue("unprojected",&entity,"module_def/channel/dpas","Module definition/root offset, channel membership and DPA semantics unavailable in projection; null, no invented module offset.")?;
    ctx.gap(&entity, "function_text")?;
    if !o.is_active {
        ctx.issue(
            "activity_loss",
            &entity,
            "is_active",
            "Inactive object included; activity is not represented in the monitoring document.",
        )?;
    }
    if o.links.is_empty() {
        ctx.issue(
            "unlinked_included",
            &entity,
            "group_address_links",
            "Unlinked object included; consumer presentation may omit it.",
        )?;
    }
    let address = d
        .address
        .ok_or("addressless object owner after admission")?
        .to_string();
    ctx.record(
        &[
            text,
            text,
            description,
            &size,
            &address,
            application.unwrap_or(""),
        ],
        1024,
    )?;
    Ok(
        json!({"name":text,"number":o.number,"text":text,"function_text":"","description":description,"device_address":address,"device_application":application,"module_def":null,"channel":null,"dpts":dpts,"object_size":size,"group_address_links":links,"flags":flags,"dpas":null}),
    )
}
