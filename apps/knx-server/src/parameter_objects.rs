//! Plans missing unscoped communication objects after a validated offline parameter edit.
use knx_core::{ComObjectInstance, Command, DeviceId, Override, Project, ResolvedFlags, SourceRef};
use std::collections::HashSet;

pub(crate) struct ObjectMetadataInput {
    pub id: u32,
    pub reference: String,
    pub module_based: bool,
    pub dpt_resolved: bool,
}

fn declared_references(
    products: &knx_productdb::Connection,
    program: &str,
) -> Result<HashSet<String>, String> {
    knx_productdb::query::com_object_ref_ids(products, program)
        .map(|ids| ids.into_iter().collect())
        .map_err(|error| error.to_string())
}

// A catalogue-local ID is not automatically a project-local alias. Only the
// actual declarations can disambiguate them; original instance IDs stay intact.
fn catalogue_reference(
    declared: &HashSet<String>,
    program: &str,
    reference: &str,
    module_based: bool,
) -> Result<String, String> {
    let expanded = knx_productdb::com_object_lookup_id(program, reference, module_based);
    if expanded != reference && declared.contains(reference) && declared.contains(&expanded) {
        return Err(format!("ambiguous communication-object reference '{reference}': both its literal identity and project-local expansion '{expanded}' are declared; no mapping was guessed"));
    }
    Ok(if declared.contains(reference) {
        reference.to_owned()
    } else {
        expanded
    })
}

/// Match structural declarations to configured IDs, never by captions or object numbers.
/// Module instances need an exact scope mapping and are not inferred here.
pub(crate) fn unscoped_object_ids(
    products: &knx_productdb::Connection,
    program: &str,
    objects: &[ObjectMetadataInput],
) -> Result<std::collections::HashMap<String, Vec<u32>>, String> {
    let declared = declared_references(products, program)?;
    let mut result = std::collections::HashMap::<String, Vec<u32>>::new();
    for object in objects.iter().filter(|object| !object.module_based) {
        // The existing diagnostics report an ambiguous identity; it offers no control.
        if let Ok(reference) = catalogue_reference(&declared, program, &object.reference, false) {
            result.entry(reference).or_default().push(object.id);
        }
    }
    Ok(result)
}

fn append_dpt_diagnostics(
    issues: Vec<knx_productdb::EnrichmentIssue>,
    diagnostics: &mut Vec<crate::routes::ParameterDiagnosticDto>,
) -> Result<(), String> {
    for issue in issues {
        match issue {
            knx_productdb::EnrichmentIssue::AmbiguousDpt {
                ref_id,
                alternatives,
            } => {
                diagnostics.push(crate::routes::ParameterDiagnosticDto {
                    scope: None,
                    kind: crate::routes::ParameterDiagnosticKindDto::ObjectDptUnresolved,
                    severity: crate::routes::ParameterDiagnosticSeverityDto::Warning,
                    message: "A configured communication object's declared datapoint type is ambiguous or malformed; no type was guessed.".into(),
                    detail: format!("'{ref_id}': declared DPT values {alternatives:?}; object DPT remains unset."),
                });
            }
            other => {
                return Err(format!(
                    "communication-object metadata could not be resolved: {other:?}"
                ))
            }
        }
    }
    Ok(())
}

/// Re-derived on every read: ambiguity stays visible without hiding readable
/// parameter fields, and a follow-up GET cannot erase unresolved DPT warnings.
pub(crate) fn object_metadata_diagnostics(
    products: &knx_productdb::Connection,
    program_id: &str,
    objects: &[ObjectMetadataInput],
) -> Result<Vec<crate::routes::ParameterDiagnosticDto>, String> {
    let mut diagnostics = Vec::new();
    if objects.is_empty() {
        return Ok(diagnostics);
    }
    let declared = declared_references(products, program_id)?;
    for object in objects {
        let lookup = match catalogue_reference(
            &declared,
            program_id,
            &object.reference,
            object.module_based,
        ) {
            Ok(reference) => reference,
            Err(detail) => {
                diagnostics.push(crate::routes::ParameterDiagnosticDto {
                    scope: None,
                    kind: crate::routes::ParameterDiagnosticKindDto::ObjectReferenceAmbiguous,
                    severity: crate::routes::ParameterDiagnosticSeverityDto::Warning,
                    message: "A communication-object reference has more than one catalogue identity; no mapping was guessed.".into(),
                    detail,
                });
                continue;
            }
        };
        if object.dpt_resolved {
            continue;
        }
        if let Some(view) =
            knx_productdb::query::com_object_view(products, program_id, &lookup, None)
                .map_err(|error| error.to_string())?
        {
            let mut issues = Vec::new();
            knx_productdb::enrich::declared_dpt(
                &object.reference,
                view.dpt_list.as_deref(),
                &mut issues,
            );
            append_dpt_diagnostics(issues, &mut diagnostics)?;
        }
    }
    Ok(diagnostics)
}

pub(crate) fn include_missing(
    products: &knx_productdb::Connection,
    before: &Project,
    device: DeviceId,
    parameter: Command,
    mut ids: knx_core::IdAllocators,
    program_id: &str,
) -> Result<(Command, Vec<crate::routes::ParameterDiagnosticDto>), String> {
    let mut preview = before.clone();
    parameter.apply(&mut preview).map_err(|e| e.to_string())?;
    let stored = preview
        .installations
        .iter()
        .flat_map(|i| &i.parameters)
        .filter(|p| p.device == device)
        .map(|p| (p.source.ets_id.clone(), p.raw.clone()))
        .collect();
    let modules = preview
        .devices
        .module_instances()
        .filter(|m| m.device == device)
        .cloned()
        .collect::<Vec<_>>();
    let evaluation =
        knx_productdb::device_evaluation::evaluate_device(products, program_id, stored, &modules)
            .map_err(|e| e.to_string())?;
    let declared = declared_references(products, program_id)?;
    let represented = preview
        .devices
        .com_objects()
        .filter(|object| object.device == device)
        .map(|object| {
            catalogue_reference(
                &declared,
                program_id,
                &object.source.ets_id,
                object.module_instance.is_some(),
            )
            .map(|reference| (object.module_instance, reference))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut known = represented
        .iter()
        .filter(|(instance, _)| instance.is_none())
        .map(|(_, reference)| reference.clone())
        .collect::<HashSet<_>>();
    let mut objects = Vec::new();
    let mut diagnostics = Vec::new();
    for active in &evaluation.activation.com_object_refs {
        // Instance construction for new module scopes needs a separate, exact
        // ModuleInstance authority; do not invent one from a declaration.
        // ADR-0177: the one resolved owner must also carry a well-formed
        // instance counter — a malformed `MI-` suffix is not an owner, so
        // the whole edit refuses here instead of guessing a number.
        // ADR-0177: when several instances share the `RefId`, the
        // `Module/@Id`'s own `MI-<k>` tail picks the owning instance.
        if let Some(scope) = &active.scope {
            let module_id = scope.module_id.as_deref();
            let owners = modules
                .iter()
                .filter(|instance| {
                    module_id.is_some_and(|id| {
                        id.contains(&format!("_{}", instance.source.ets_id))
                    })
                })
                .collect::<Vec<_>>();
            let owner = match owners.as_slice() {
                [] => None,
                [one] if one.instance_counter().is_some() => Some(*one),
                many => module_id
                    .and_then(mi_counter_of)
                    .and_then(|k| {
                        many.iter()
                            .find(|m| m.instance_counter() == Some(k))
                            .copied()
                    }),
            };
            let authoritative = owner.is_some();
            let present = authoritative
                && represented.iter().any(|(instance, reference)| {
                    *instance == Some(owner.unwrap().id) && reference == &active.ref_id
                });
            if !present {
                return Err("The parameter would require new module communication objects whose instance/number mapping is not supported; nothing was changed.".into());
            }
            continue;
        }
        if !known.insert(active.ref_id.clone()) {
            continue;
        }
        let view =
            knx_productdb::query::com_object_view(products, program_id, &active.ref_id, None)
                .map_err(|e| e.to_string())?
                .ok_or("active communication-object declaration is unavailable")?;
        let number = view
            .number
            .and_then(|n| u16::try_from(n).ok())
            .ok_or("active communication-object number is invalid")?;
        let id = ids
            .next_com_object_instance_id()
            .map_err(|e| e.to_string())?;
        preview.devices.insert_com_object(ComObjectInstance {
            priority_edit: None,
            saved_instance_priority: Default::default(),
            id,
            source: SourceRef {
                path: active.ref_id.clone(),
                ets_id: active.ref_id.clone(),
            },
            device,
            number,
            text: Override::Absent,
            description: Override::Absent,
            dpt: Override::Absent,
            flags: ResolvedFlags::none(),
            size: None,
            is_active: true,
            links: vec![],
            module_instance: None,
        });
        // Reuse the catalog's sole field/flag/provenance mapper. Existing
        // objects are never re-enriched or replaced by this operation.
        let mut issues = Vec::new();
        knx_productdb::enrich::apply(&mut preview, id, &active.ref_id, &view, &mut issues);
        append_dpt_diagnostics(issues, &mut diagnostics)?;
        objects.push(
            preview
                .devices
                .com_object(id)
                .expect("just inserted")
                .clone(),
        );
    }
    let mut commands = Vec::new();
    if ids != before.ids {
        commands.push(Command::ReserveIds { through: ids });
    }
    commands.push(parameter);
    if !objects.is_empty() {
        commands.push(Command::AddComObjects {
            device,
            objects,
            program_defaults: vec![],
        });
    }
    let command = if commands.len() == 1 {
        commands.pop().expect("one command")
    } else {
        Command::Batch(commands)
    };
    Ok((command, diagnostics))
}

/// The `MI-<k>` digits a program-side `Module/@Id` carries, when well-formed
/// (ADR-0177): rightmost `_MI-` whose tail is all ASCII digits. This is the
/// same counter definition as `knx_core::ModuleInstance::instance_counter`.
fn mi_counter_of(module_id: &str) -> Option<u32> {
    let marker = module_id.rfind("_MI-")?;
    let digits = &module_id[marker + 4..];
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

