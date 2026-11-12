//! Compares independent written links and flags without whole-image admission or row guessing.

use crate::AppState;
use chrono::SecondsFormat;
use knx_core::commissioning::parameter_baseline::{
    LinkBaseline, ParameterBaseline, WrittenObjectFlags,
};
use knx_core::{DeviceId, Project};
use knx_productdb::enrich::com_object_lookup_id;
use knx_projection::{
    DownloadComparisonState as State, DownloadFlagDifference, DownloadLinkDifference,
    DownloadObjectDifference, DownloadObjectFlags, DownloadSection, ObjectLastDownload,
};
use std::collections::{BTreeMap, BTreeSet};

/// Detached under the project lock together with the detail and baseline records.
pub(crate) struct ComparisonInput {
    project: Project,
    device: DeviceId,
    parameter: Option<ParameterBaseline>,
    links: Option<LinkBaseline>,
}
impl ComparisonInput {
    pub(crate) fn collect(state: &AppState, project: &Project, device: DeviceId) -> Self {
        let baselines = state
            .parameter_baselines
            .lock()
            .expect("state mutex poisoned");
        Self {
            project: project.clone(),
            device,
            parameter: baselines.get(device).cloned(),
            links: baselines.get_links(device).cloned(),
        }
    }
}
fn section(state: State) -> DownloadSection {
    DownloadSection { state, at: None }
}
fn flags(value: WrittenObjectFlags) -> DownloadObjectFlags {
    DownloadObjectFlags {
        read: value.read,
        write: value.write,
        transmit: value.transmit,
        update: value.update,
        communication: value.communication,
        read_on_init: value.read_on_init,
    }
}

pub(crate) fn last_download(state: &AppState, input: &ComparisonInput) -> ObjectLastDownload {
    let unavailable = || compare(None, input);
    let Some(owner) = &state.product_db else {
        return unavailable();
    };
    owner
        .with_read(|conn| Ok::<_, std::convert::Infallible>(compare(Some(conn), input)))
        .ok()
        .and_then(Result::ok)
        .unwrap_or_else(unavailable)
}

fn compare(
    conn: Option<&knx_productdb::Connection>,
    input: &ComparisonInput,
) -> ObjectLastDownload {
    let mut result = ObjectLastDownload {
        links: section(State::Absent),
        flags: section(State::Absent),
        differing: Vec::new(),
        instances: BTreeMap::new(),
        differing_objects: 0,
        no_selected_row: 0,
    };
    let Some(instance) = input.project.devices.get(input.device) else {
        return result;
    };
    let stored_flags = input
        .parameter
        .as_ref()
        .and_then(|baseline| baseline.object_flags.as_ref());
    if input.links.is_none() && stored_flags.is_none() {
        return result;
    }
    let program = conn.and_then(|conn| {
        knx_productdb::query::resolve_program(conn, &instance.program_ref)
            .ok()
            .flatten()
    });
    let usable = |stored_program: &str| match program.as_deref() {
        None => State::ProductUnavailable,
        Some(program) if program != stored_program => State::OtherProgram,
        Some(_) if instance.address.is_none() => State::CurrentUnreadable,
        Some(_) => State::Usable,
    };
    if let Some(baseline) = &input.links {
        result.links = section(usable(&baseline.program_id));
    }
    if stored_flags.is_some() {
        result.flags = section(usable(
            &input
                .parameter
                .as_ref()
                .expect("flags have a header")
                .program_id,
        ));
    }
    let mut differing: BTreeMap<u8, DownloadObjectDifference> = BTreeMap::new();
    let entry = |number| DownloadObjectDifference {
        number,
        instance_id: None,
        links: Vec::new(),
        flags: None,
    };
    if result.links.state == State::Usable {
        // Use only the writer's table mapping, not priority/size or flag admission.
        let current = knx_productdb::image_request::links(&input.project, input.device);
        let module_link = instance
            .com_objects
            .iter()
            .filter_map(|id| input.project.devices.com_object(*id))
            .any(|object| object.module_instance.is_some() && !object.links.is_empty());
        if let (Ok(current), false) = (current, module_link) {
            let baseline = input.links.as_ref().expect("usable links have a header");
            result.links.at = Some(baseline.at.to_rfc3339_opts(SecondsFormat::AutoSi, true));
            let was: BTreeMap<_, _> = baseline
                .links
                .iter()
                .map(|link| ((link.object, link.group_address.raw()), link.sending))
                .collect();
            let now: BTreeMap<_, _> = current
                .iter()
                .map(|link| ((link.object, link.group_address.raw()), link.sending))
                .collect();
            let keys: BTreeSet<_> = was.keys().chain(now.keys()).copied().collect();
            for (number, address) in keys {
                if was.get(&(number, address)) != now.get(&(number, address)) {
                    differing
                        .entry(number)
                        .or_insert_with(|| entry(number))
                        .links
                        .push(DownloadLinkDifference {
                            group_address: address,
                            downloaded: was.get(&(number, address)).copied(),
                            current: now.get(&(number, address)).copied(),
                        });
                }
            }
        } else {
            result.links = section(State::CurrentUnreadable);
        }
    }
    // Resolve exact active reference provenance once, even if flags are unknown: link row
    // attribution may use it, but an error never suppresses an independently usable link verdict.
    let active = conn
        .zip(program.as_deref())
        .ok_or(State::ProductUnavailable)
        .and_then(|(conn, program)| {
            let mut lookup = BTreeSet::new();
            for id in &instance.com_objects {
                let object = input
                    .project
                    .devices
                    .com_object(*id)
                    .ok_or(State::CurrentUnreadable)?;
                if object.module_instance.is_none()
                    && !lookup.insert(com_object_lookup_id(program, &object.source.ets_id, false))
                {
                    return Err(State::CurrentUnreadable);
                }
            }
            let values =
                knx_productdb::image_request::chosen_parameter_values(&input.project, input.device)
                    .map_err(|_| State::CurrentUnreadable)?;
            let overrides =
                knx_productdb::image_request::flag_overrides(&input.project, input.device, program)
                    .map_err(|_| State::CurrentUnreadable)?;
            knx_productdb::image::resolve_effective_object_flags_with_refs(
                conn, program, &values, &overrides,
            )
            .map_err(|error| match error {
                knx_productdb::image::ImageError::Database(_)
                | knx_productdb::image::ImageError::UnknownProgram(_)
                | knx_productdb::image::ImageError::Code(
                    knx_productdb::code::CodeError::Database(_)
                    | knx_productdb::code::CodeError::MissingSource { .. },
                ) => State::ProductUnavailable,
                _ => State::CurrentUnreadable,
            })
        });
    if result.flags.state == State::Usable {
        if let Ok(active) = &active {
            let baseline = input
                .parameter
                .as_ref()
                .expect("usable flags have a header");
            result.flags.at = Some(baseline.at.to_rfc3339_opts(SecondsFormat::AutoSi, true));
            let was = stored_flags.expect("usable flags are known");
            let now: BTreeMap<u8, WrittenObjectFlags> = active
                .iter()
                .map(|(number, active)| (*number, active.flags.into()))
                .collect();
            let numbers: BTreeSet<_> = was.keys().chain(now.keys()).copied().collect();
            for number in numbers {
                if was.get(&number) != now.get(&number) {
                    differing
                        .entry(number)
                        .or_insert_with(|| entry(number))
                        .flags = Some(DownloadFlagDifference {
                        downloaded: was.get(&number).copied().map(flags),
                        current: now.get(&number).copied().map(flags),
                    });
                }
            }
        } else {
            result.flags = section(*active.as_ref().expect_err("no current flags"));
        }
    }
    for (number, difference) in &mut differing {
        let rows: Vec<_> = instance
            .com_objects
            .iter()
            .filter_map(|id| input.project.devices.com_object(*id))
            .filter(|object| {
                object.module_instance.is_none() && object.number == u16::from(*number)
            })
            .collect();
        let linked: Vec<_> = rows
            .iter()
            .filter(|object| !object.links.is_empty())
            .collect();
        let selected = match linked.as_slice() {
            [object] => Some(object.id.0),
            [] => active
                .as_ref()
                .ok()
                .and_then(|active| active.get(number))
                .and_then(|active| {
                    let program = program.as_deref()?;
                    let matching: Vec<_> = rows
                        .iter()
                        .filter(|object| {
                            com_object_lookup_id(program, &object.source.ets_id, false)
                                == active.ref_id
                        })
                        .collect();
                    match matching.as_slice() {
                        [object] => Some(object.id.0),
                        _ => None,
                    }
                }),
            _ => None,
        };
        difference.instance_id = selected;
        if let Some(id) = selected {
            result.instances.insert(id, difference.clone());
        } else {
            result.no_selected_row += 1;
        }
    }
    result.differing = differing.into_values().collect();
    result.differing_objects = result.differing.len();
    result
}
