//! Composes product-database projections for the pure documentation renderer.

use std::collections::{BTreeSet, HashMap};

use chrono::{DateTime, Utc};
use knx_core::Project;
use knx_report::{ReportDeviceData, ReportField, ReportLanguage, ReportOptions, ReportSection};

fn non_blank(value: Option<String>) -> Option<String> {
    value.filter(|value| !value.trim().is_empty())
}

pub fn report_options(
    project: &Project,
    products: Option<&knx_productdb::Connection>,
    generated_at: DateTime<Utc>,
    language: ReportLanguage,
    sections: BTreeSet<ReportSection>,
) -> ReportOptions {
    let mut options = ReportOptions::new(generated_at);
    options.language = language;
    options.sections = sections;
    let Some(products) = products else {
        return options;
    };

    for device in project.devices.iter() {
        let mut data = ReportDeviceData::default();
        let identity = knx_productdb::query::device_product(
            products,
            &device.product_ref,
            &device.program_ref,
            Some(language.code()),
        );
        let program_id = match identity {
            Ok(Some(row)) => {
                data.manufacturer_reference = Some(row.manufacturer_id.clone());
                data.manufacturer = non_blank(row.manufacturer_name);
                data.product = non_blank(row.catalog_item_name)
                    .or_else(|| non_blank(row.product_text))
                    .or_else(|| non_blank(row.hardware_name));
                data.application_program = non_blank(row.application_name);
                if row.program_relation
                    == knx_productdb::query::DeviceProgramRelation::HardwareMismatch
                {
                    data.problems.push(format!(
                        "product/program references {} and {} do not share hardware; raw references shown",
                        device.product_ref, device.program_ref
                    ));
                }
                row.application_program_id
            }
            Ok(None) | Err(_) => None,
        };

        let parameter_views = program_id
            .as_deref()
            .and_then(|program_id| {
                knx_productdb::query::parameter_views(products, program_id, Some(language.code()))
                    .ok()
            })
            .unwrap_or_default();
        let parameter_views: HashMap<_, _> = parameter_views
            .into_iter()
            .map(|view| (view.id.clone(), view))
            .collect();
        for parameter in project
            .installations
            .iter()
            .flat_map(|installation| installation.parameters.iter())
            .filter(|parameter| parameter.device == device.id)
        {
            let view = parameter_views.get(&parameter.source.ets_id);
            let (display_value, problem) = match view {
                None => (
                    None,
                    Some(
                        "parameter declaration could not be resolved; raw value shown".to_string(),
                    ),
                ),
                Some(view) if view.kind == "Restriction" => match view
                    .enum_options
                    .iter()
                    .find(|(value, _)| value == &parameter.raw)
                {
                    Some((_, text)) => match non_blank(text.clone()) {
                        Some(text) => (Some(text), None),
                        None => (
                            None,
                            Some(format!(
                                "restriction value '{}' has no display text; raw value shown",
                                parameter.raw
                            )),
                        ),
                    },
                    None => (
                        None,
                        Some(format!(
                            "restriction value '{}' is not declared; raw value shown",
                            parameter.raw
                        )),
                    ),
                },
                Some(view) => (
                    None,
                    Some(format!(
                        "parameter kind '{}' has no report formatter; raw value shown",
                        view.kind
                    )),
                ),
            };
            data.parameters.push(ReportField {
                reference: parameter.source.ets_id.clone(),
                name: view.and_then(|view| {
                    non_blank(view.text.clone()).or_else(|| non_blank(view.name.clone()))
                }),
                raw_value: parameter.raw.clone(),
                display_value,
                problem,
            });
        }

        let argument_declarations: HashMap<_, _> = program_id
            .as_deref()
            .and_then(|program_id| {
                knx_productdb::dynamic::load_module_def_arguments(products, program_id).ok()
            })
            .unwrap_or_default()
            .into_iter()
            .map(|argument| (argument.id.clone(), argument))
            .collect();
        for module in project
            .devices
            .module_instances()
            .filter(|module| module.device == device.id)
        {
            for (source, raw) in &module.arguments {
                let declaration = argument_declarations.get(&source.ets_id);
                data.module_arguments.push(ReportField {
                    reference: source.ets_id.clone(),
                    name: declaration.and_then(|argument| non_blank(argument.name.clone())),
                    raw_value: raw.clone(),
                    display_value: None,
                    problem: match declaration {
                        None => Some(
                            "module argument declaration could not be resolved; raw value shown"
                                .to_string(),
                        ),
                        Some(argument)
                            if argument.kind()
                                == knx_productdb::dynamic::ArgumentKind::Unsupported =>
                        {
                            Some(format!(
                                "module argument kind '{}' is unsupported; raw value shown",
                                argument.arg_type.as_deref().unwrap_or("unknown")
                            ))
                        }
                        Some(_) => None,
                    },
                });
            }
        }
        options.device_data.insert(device.id, data);
    }
    options
}
