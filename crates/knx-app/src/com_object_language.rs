//! Translates product-supplied communication-object texts into a display language.
//!
//! One rule, shared by the device-detail route and the documentation report
//! (KNOWN_LIMITATIONS §37): only a `text`/`description` whose stored override
//! sits at the `Program` or `ProgramRef` layer came from the product database
//! and may be replaced, and only when a translation actually answered. A
//! language miss leaves the project's own resolved text standing, and
//! project-authored layers (`Instance`, `Inferred`, `UserEdit`) are never
//! translated. Nothing here writes to the project.
//!
//! The work is split in two so a caller holding the project and the product
//! database behind separate locks never needs both at once: [`ComObjectTextInput::collect`]
//! reads the project, [`translate_com_object_texts`] reads the database.

use std::collections::HashMap;

use knx_core::{ComObjectInstanceId, DeviceId, Layer, Project};
use knx_productdb::{Connection, ProductDbError};

/// What one communication object contributes to the translation lookup.
#[derive(Debug, Clone)]
struct ComObjectTextSource {
    ref_id: String,
    module_based: bool,
    text_layer: Option<Layer>,
    description_layer: Option<Layer>,
}

/// Everything [`translate_com_object_texts`] needs from the project for one
/// device, gathered without touching the product database.
#[derive(Debug, Clone)]
pub struct ComObjectTextInput {
    program_ref: String,
    com_objects: HashMap<ComObjectInstanceId, ComObjectTextSource>,
}

impl ComObjectTextInput {
    /// `None` when the device does not exist.
    pub fn collect(project: &Project, device: DeviceId) -> Option<Self> {
        let dev = project.devices.get(device)?;
        let com_objects = dev
            .com_objects
            .iter()
            .filter_map(|id| project.devices.com_object(*id))
            .map(|com| {
                (
                    com.id,
                    ComObjectTextSource {
                        ref_id: com.source.ets_id.clone(),
                        module_based: com.module_instance.is_some(),
                        text_layer: com.text.layer(),
                        description_layer: com.description.layer(),
                    },
                )
            })
            .collect();
        Some(Self {
            program_ref: dev.program_ref.clone(),
            com_objects,
        })
    }
}

/// The translated replacement for one communication object's displayed
/// texts; `None` keeps the project's own resolved value.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TranslatedComObjectText {
    pub name: Option<String>,
    pub description: Option<String>,
}

fn from_product(layer: Option<Layer>) -> bool {
    matches!(layer, Some(Layer::Program) | Some(Layer::ProgramRef))
}

/// Translations that answered for `language`, keyed by communication-object
/// instance. Objects with nothing to replace are absent. A device whose
/// program is not installed yields an empty map: ordinary project state,
/// not an error.
pub fn translate_com_object_texts(
    products: &Connection,
    input: &ComObjectTextInput,
    language: &str,
) -> Result<HashMap<ComObjectInstanceId, TranslatedComObjectText>, ProductDbError> {
    let Some(program_id) = knx_productdb::query::resolve_program(products, &input.program_ref)?
    else {
        return Ok(HashMap::new());
    };
    // Loaded once for the whole device: a device can own hundreds of
    // communication objects, and the overlay is per program.
    let lookup_ids: HashMap<ComObjectInstanceId, String> = input
        .com_objects
        .iter()
        .map(|(id, source)| {
            (
                *id,
                knx_productdb::com_object_lookup_id(
                    &program_id,
                    &source.ref_id,
                    source.module_based,
                ),
            )
        })
        .collect();
    let refs: Vec<&str> = lookup_ids.values().map(String::as_str).collect();
    let views =
        knx_productdb::query::com_object_views(products, &program_id, &refs, Some(language))?;

    let mut translated = HashMap::new();
    for (id, source) in &input.com_objects {
        let Some(view) = views.get(&lookup_ids[id]) else {
            continue;
        };
        // The layer is the one stored on the project's own instance, never
        // the one the view reports: a `ComObjectRef` translation without a
        // structural override must not translate project-authored text.
        // `*_translated` is true only when the chosen value came out of the
        // overlay (finding M6), so a miss replaces nothing.
        let name = (from_product(source.text_layer) && view.text_translated)
            .then(|| view.text.clone())
            .flatten();
        let description = (from_product(source.description_layer)
            && view.visible_description_translated)
            .then(|| view.visible_description.clone())
            .flatten();
        if name.is_some() || description.is_some() {
            translated.insert(*id, TranslatedComObjectText { name, description });
        }
    }
    Ok(translated)
}
