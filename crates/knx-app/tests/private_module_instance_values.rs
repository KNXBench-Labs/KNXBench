//! Opt-in, read-only comparison of each imported instance's own stored values.
//!
//! No private identifiers, totals, or source values are printed. This does not
//! prove Repeat activation or whole-project import reconciliation (ADR-0107).

use std::path::PathBuf;

#[test]
#[ignore = "requires KNXBENCH_PRIVATE_SCHEMA23_PROJECT; private read-only evidence"]
fn private_instances_read_their_own_stored_values() {
    let path = std::env::var_os("KNXBENCH_PRIVATE_SCHEMA23_PROJECT")
        .map(PathBuf::from)
        .unwrap_or_else(|| panic!("refusal: private-project opt-in variable is required"));
    assert!(
        path.is_file(),
        "refusal: configured private project is unavailable"
    );
    let evidence =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.ai/logs/private-import-analysis");
    std::fs::create_dir_all(&evidence)
        .unwrap_or_else(|_| panic!("private evidence directory is unavailable"));
    let dir = tempfile::Builder::new()
        .prefix("instance-values-")
        .tempdir_in(evidence)
        .unwrap_or_else(|_| panic!("private evidence workspace is unavailable"));
    let store = knx_store::open_and_migrate(&dir.path().join("project.sqlite"))
        .unwrap_or_else(|_| panic!("temporary project store cannot be initialized"));
    let products = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite"))
        .unwrap_or_else(|_| panic!("temporary product database cannot be initialized"));
    let imported = knx_app::import::import_ets_project_with(
        &path,
        &store,
        knx_app::ImportOptions {
            product_db: Some(&products),
        },
    )
    .unwrap_or_else(|error| match error {
        knx_app::import::AppError::Import(_) => panic!("private project parser refused import"),
        knx_app::import::AppError::ProductDb(_) => {
            panic!("private product ingestion refused import")
        }
        knx_app::import::AppError::Store(_) | knx_app::import::AppError::Sql(_) => {
            panic!("temporary project persistence failed")
        }
    });
    let project = imported.project;
    let mut witnessed = false;
    for device in project.devices.iter() {
        let instances: Vec<_> = project
            .devices
            .module_instances()
            .filter(|instance| instance.device == device.id)
            .cloned()
            .collect();
        if instances.is_empty() {
            continue;
        }
        let Some(program_id) =
            knx_productdb::query::resolve_program(&products, &device.program_ref)
                .unwrap_or_else(|_| panic!("program resolution failed"))
        else {
            panic!("an instance-bearing device has no installed program");
        };
        let stored: Vec<_> = project
            .installations
            .iter()
            .flat_map(|i| i.parameters.iter())
            .filter(|p| p.device == device.id)
            .map(|p| (p.source.ets_id.clone(), p.raw.clone()))
            .collect();
        let evaluation = knx_productdb::device_evaluation::evaluate_device(
            &products,
            &program_id,
            stored.clone(),
            &instances,
        )
        .unwrap_or_else(|_| panic!("private device evaluation failed"));
        // Oracle uses the imported instance identity, not the evaluator's id map.
        for instance in &instances {
            let qualify = |id: &str| {
                if id.starts_with(&format!("{program_id}_")) {
                    id.to_owned()
                } else {
                    format!("{program_id}_{id}")
                }
            };
            let instance_id = qualify(&instance.instance_ets_id);
            let module_id = qualify(&instance.source.ets_id);
            let Some((definition, _)) = module_id.rsplit_once("_M-") else {
                panic!("private instance owner has an unsupported identity form");
            };
            let prefix = format!("{instance_id}_");
            for (ets_id, raw) in &stored {
                let Some(local_ref) = ets_id.strip_prefix(&prefix) else {
                    continue;
                };
                if !local_ref.starts_with("P-") {
                    continue;
                }
                let declared = format!("{definition}_{local_ref}");
                assert!(
                    evaluation.values.get_instance(
                        &module_id,
                        &instance.instance_ets_id,
                        &declared
                    ) == Some(raw.as_str()),
                    "an instance did not read its own stored value"
                );
                witnessed = true;
            }
        }
    }
    assert!(
        witnessed,
        "refusal: no instance-owned stored value was exercised"
    );
}
