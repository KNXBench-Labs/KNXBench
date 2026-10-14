//! Fills positional `{{0}}` placeholders in product-supplied object names (ADR-0126).
//!
//! The same rule as [`crate::com_object_language`], shared by the device-detail
//! route and the documentation report: only a name whose stored override sits
//! at the `Program` or `ProgramRef` layer came from the product database and
//! may be filled; a name the project owns (`Instance`, `Inferred`, `UserEdit`)
//! is left exactly as stored, braces included. Filling runs on the name as
//! shown, so after the translation overlay. Nothing here writes to the project.
//!
//! Split like `com_object_language`: [`NameFillInput::collect`] reads the
//! project, the `fill_*` functions read the product database.

use std::collections::HashMap;

use knx_core::{ComObjectInstanceId, DeviceId, Layer, ModuleInstanceId, Project};
use knx_productdb::dynamic::{PositionalSource, TextProjectionBudget};
use knx_productdb::text_parameters::{DeviceTexts, TextParameters};
use knx_productdb::Connection;

/// One object name after filling: what to show, what it was, and where the
/// inserted text came from. Only produced for a name that had a `{{0}}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilledName {
    pub text: String,
    pub template: String,
    pub source: PositionalSource,
}

#[derive(Debug, Clone)]
struct NameSource {
    ets_id: String,
    instance: Option<ModuleInstanceId>,
    from_product: bool,
}

/// What filling one device's object names needs from the project.
#[derive(Debug, Clone)]
pub struct NameFillInput {
    program_ref: String,
    stored: Vec<(String, String)>,
    module_instances: Vec<knx_core::ModuleInstance>,
    objects: HashMap<ComObjectInstanceId, NameSource>,
}

impl NameFillInput {
    /// `None` when the device does not exist or states no program.
    pub fn collect(project: &Project, device: DeviceId) -> Option<Self> {
        let dev = project.devices.get(device)?;
        if dev.program_ref.is_empty() {
            return None;
        }
        let stored = project
            .installations
            .iter()
            .flat_map(|installation| &installation.parameters)
            .filter(|p| p.device == device)
            .map(|p| (p.source.ets_id.clone(), p.raw.clone()))
            .collect();
        let module_instances = project
            .devices
            .module_instances()
            .filter(|m| m.device == device)
            .cloned()
            .collect();
        let objects = dev
            .com_objects
            .iter()
            .filter_map(|id| project.devices.com_object(*id))
            .map(|com| {
                let layer = com.text.layer();
                (
                    com.id,
                    NameSource {
                        ets_id: com.source.ets_id.clone(),
                        instance: com.module_instance,
                        from_product: matches!(layer, Some(Layer::Program | Layer::ProgramRef)),
                    },
                )
            })
            .collect();
        Some(Self {
            program_ref: dev.program_ref.clone(),
            stored,
            module_instances,
            objects,
        })
    }

    pub fn module_instances(&self) -> &[knx_core::ModuleInstance] {
        &self.module_instances
    }
}

/// Fills `names` (each object's name as shown) with an evaluation the caller
/// already holds. Returns a [`FilledName`] per object whose product name had a
/// `{{0}}`. A text that would not fit refuses the whole overlay, as the
/// channel-text overlay does.
pub fn fill_object_names_with(
    texts: &DeviceTexts<'_>,
    program_id: &str,
    input: &NameFillInput,
    names: &HashMap<ComObjectInstanceId, String>,
) -> Result<HashMap<ComObjectInstanceId, FilledName>, String> {
    let mut budget = TextProjectionBudget::default();
    let mut filled = HashMap::new();
    let mut ids: Vec<&ComObjectInstanceId> = names.keys().collect();
    ids.sort_unstable();
    for id in ids {
        let name = &names[id];
        let Some(source) = input.objects.get(id).filter(|s| s.from_product) else {
            continue;
        };
        if !name.contains("{{") {
            continue;
        }
        let lookup_id = knx_productdb::com_object_lookup_id(
            program_id,
            &source.ets_id,
            source.instance.is_some(),
        );
        let instance = source
            .instance
            .and_then(|i| input.module_instances.iter().find(|m| m.id == i));
        let result = texts
            .fill_object(name, &lookup_id, instance, &mut budget)
            .map_err(|e| e.to_string())?;
        if result.source != PositionalSource::NoPlaceholder {
            filled.insert(
                *id,
                FilledName {
                    text: result.text,
                    template: name.clone(),
                    source: result.source,
                },
            );
        }
    }
    Ok(filled)
}

/// [`fill_object_names_with`] for a caller without an evaluation (the
/// documentation report): evaluates the device the way the parameter panel
/// does. Empty when the program is not installed.
pub fn fill_object_names(
    products: &Connection,
    input: &NameFillInput,
    names: &HashMap<ComObjectInstanceId, String>,
) -> Result<HashMap<ComObjectInstanceId, FilledName>, String> {
    if !names.values().any(|n| n.contains("{{")) {
        return Ok(HashMap::new());
    }
    let Some(program_id) = knx_productdb::query::resolve_program(products, &input.program_ref)
        .map_err(|e| e.to_string())?
    else {
        return Ok(HashMap::new());
    };
    let evaluation = knx_productdb::device_evaluation::evaluate_device(
        products,
        &program_id,
        input.stored.clone(),
        &input.module_instances,
    )
    .map_err(|e| e.to_string())?;
    let parameters = TextParameters::load(products, &program_id).map_err(|e| e.to_string())?;
    let texts = DeviceTexts {
        parameters: &parameters,
        evaluation: &evaluation,
        module_instances: &input.module_instances,
    };
    fill_object_names_with(&texts, &program_id, input, names)
}
