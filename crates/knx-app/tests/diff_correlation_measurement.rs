//! Measures project-diff correlation gaps against the optional private corpus without exposing it.

use std::collections::BTreeMap;

use knx_core::{BuildingPartId, Project};

#[derive(Debug, Default, PartialEq, Eq)]
struct ProjectMetrics {
    devices_without_address_or_ets_id: usize,
    duplicate_sibling_name_groups: usize,
}

#[derive(Debug, Default, PartialEq, Eq)]
struct ReexportMetrics {
    device_address_keys_with_changed_ets_id: usize,
    building_paths_with_changed_ets_id: usize,
    group_addresses_with_changed_ets_id: usize,
}

fn project_metrics(project: &Project) -> ProjectMetrics {
    let devices_without_address_or_ets_id = project
        .devices
        .iter()
        .filter(|device| device.address.is_none() && device.source.ets_id.is_empty())
        .count();

    let duplicate_sibling_name_groups = project
        .installations
        .iter()
        .flat_map(|installation| {
            let mut groups: BTreeMap<(Option<BuildingPartId>, &str), usize> = BTreeMap::new();
            for part in &installation.buildings {
                *groups.entry((part.parent, part.name.as_str())).or_default() += 1;
            }
            groups.into_values().filter(|count| *count > 1)
        })
        .count();

    ProjectMetrics {
        devices_without_address_or_ets_id,
        duplicate_sibling_name_groups,
    }
}

fn unique_id_by_key<K: Ord>(pairs: impl IntoIterator<Item = (K, String)>) -> BTreeMap<K, String> {
    let mut grouped: BTreeMap<K, Vec<String>> = BTreeMap::new();
    for (key, id) in pairs {
        grouped.entry(key).or_default().push(id);
    }
    grouped
        .into_iter()
        .filter_map(|(key, ids)| (ids.len() == 1).then(|| (key, ids.into_iter().next().unwrap())))
        .collect()
}

fn reexport_metrics(left: &Project, right: &Project) -> ReexportMetrics {
    let left_devices = unique_id_by_key(left.devices.iter().filter_map(|device| {
        device
            .address
            .map(|address| (address, device.source.ets_id.clone()))
    }));
    let right_devices = unique_id_by_key(right.devices.iter().filter_map(|device| {
        device
            .address
            .map(|address| (address, device.source.ets_id.clone()))
    }));
    let device_address_keys_with_changed_ets_id = left_devices
        .iter()
        .filter(|(key, id)| {
            right_devices
                .get(key)
                .is_some_and(|right_id| right_id != *id)
        })
        .count();

    let building_pairs = |project: &Project| {
        project
            .installations
            .iter()
            .flat_map(|installation| {
                let by_id: BTreeMap<BuildingPartId, &knx_core::BuildingPart> = installation
                    .buildings
                    .iter()
                    .map(|part| (part.id, part))
                    .collect();
                installation
                    .buildings
                    .iter()
                    .map(move |part| {
                        (
                            knx_diff::building_part_key(part, &by_id).path,
                            part.source.ets_id.clone(),
                        )
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    };
    let left_buildings = unique_id_by_key(building_pairs(left));
    let right_buildings = unique_id_by_key(building_pairs(right));
    let building_paths_with_changed_ets_id = left_buildings
        .iter()
        .filter(|(key, id)| {
            right_buildings
                .get(*key)
                .is_some_and(|right_id| right_id != *id)
        })
        .count();

    let group_address_pairs = |project: &Project| {
        project
            .installations
            .iter()
            .flat_map(|installation| {
                installation
                    .group_addresses
                    .iter()
                    .map(|entry| (entry.address.raw(), entry.source.ets_id.clone()))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    };
    let left_group_addresses = unique_id_by_key(group_address_pairs(left));
    let right_group_addresses = unique_id_by_key(group_address_pairs(right));
    let group_addresses_with_changed_ets_id = left_group_addresses
        .iter()
        .filter(|(key, id)| {
            right_group_addresses
                .get(key)
                .is_some_and(|right_id| right_id != *id)
        })
        .count();

    ReexportMetrics {
        device_address_keys_with_changed_ets_id,
        building_paths_with_changed_ets_id,
        group_addresses_with_changed_ets_id,
    }
}

#[test]
fn measures_the_three_documented_correlation_gaps_without_guessing() {
    if !knx_testsupport::corpus_available() {
        eprintln!("skip: OriginalData corpus not present (gitignored, local-only)");
        return;
    }

    let ets4 = knx_etsproj::import_knxproj(&knx_testsupport::reference_ets4_path())
        .expect("reference ETS4 project imports")
        .project;
    let ets6 = knx_etsproj::import_knxproj(&knx_testsupport::reference_ets6_path())
        .expect("reference ETS6 project imports")
        .project;
    let schema21 = knx_etsproj::import_knxproj(&knx_testsupport::reference_kv_schema21_path())
        .expect("reference schema-21 project imports")
        .project;

    let metrics = [
        project_metrics(&ets4),
        project_metrics(&ets6),
        project_metrics(&schema21),
    ];
    let reexport = reexport_metrics(&ets4, &ets6);

    eprintln!("correlation gap metrics: projects={metrics:?}; ets4_vs_ets6={reexport:?}");

    assert_eq!(
        metrics,
        [
            ProjectMetrics::default(),
            ProjectMetrics::default(),
            ProjectMetrics::default()
        ]
    );
    assert_eq!(reexport, ReexportMetrics::default());
}
