//! Re-measures KNOWN_LIMITATIONS.md §12's manufacturer-data gaps against the installed corpus.

use std::collections::HashSet;

use knx_core::Override;

fn reference_project_path() -> std::path::PathBuf {
    knx_testsupport::reference_ets4_path()
}

/// One shared import, reused by every assertion below — each test that
/// needs the corpus repeats this ~7s import on its own, but splitting the
/// numbers into separate `#[test]` functions keeps a failure's cause
/// legible instead of one giant assertion block naming which of a dozen
/// counts broke.
fn import() -> Option<(knx_app::ImportedProject, knx_productdb::Connection)> {
    if !reference_project_path().exists() {
        eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
        return None;
    }
    let dir = tempfile::tempdir().unwrap();
    let store = knx_store::open_and_migrate(&dir.path().join("p.knxdb")).unwrap();
    let products = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    let imported = knx_app::import_ets_project_with(
        &reference_project_path(),
        &store,
        knx_app::ImportOptions {
            product_db: Some(&products),
        },
    )
    .unwrap();
    Some((imported, products))
}

/// Gap 2's headline claim (KNOWN_LIMITATIONS §12, ADR-0012): 497 of the
/// reference project's 907 `ComObjectInstanceRef` elements carry
/// `DatapointType=""` — `Override::Empty`, not `Absent`, after import and
/// enrichment. Re-measured, not assumed: the figure is exact, and so is
/// the device spread (23 of 36 devices carry at least one), and the split
/// between an `Empty` slot with a genuine single-valued program default
/// sitting behind it (122) versus one where the program itself has
/// nothing to offer either (375) — only the first group is what a
/// `Override<T>` layer-stack fix can actually surface.
#[test]
fn gap2_the_497_of_907_empty_dpt_figure_and_its_liftable_share() {
    let Some((imported, products)) = import() else {
        return;
    };
    let project = &imported.project;

    let mut dpt_absent = 0usize;
    let mut dpt_empty = 0usize;
    let mut dpt_value = 0usize;
    let mut dpt_malformed = 0usize;
    let mut total = 0usize;
    let mut devices_with_empty_dpt = HashSet::new();

    for device in project.devices.iter() {
        for &com_id in &device.com_objects {
            let Some(com) = project.devices.com_object(com_id) else {
                continue;
            };
            total += 1;
            match &com.dpt {
                Override::Absent => dpt_absent += 1,
                Override::Empty => {
                    dpt_empty += 1;
                    devices_with_empty_dpt.insert(device.id);
                }
                Override::Value(_) => dpt_value += 1,
                Override::Malformed(_) => dpt_malformed += 1,
            }
        }
    }

    assert_eq!(total, 907);
    assert_eq!(dpt_empty, 497, "the figure §12 and ADR-0012 both cite");
    assert_eq!(dpt_absent, 120);
    assert_eq!(dpt_value, 290);
    assert_eq!(dpt_malformed, 0);
    assert_eq!(devices_with_empty_dpt.len(), 23, "of 36 devices total");
    assert_eq!(project.devices.iter().count(), 36);

    let mut liftable = 0usize;
    let mut not_liftable = 0usize;
    for device in project.devices.iter() {
        let Some(program_id) =
            knx_productdb::query::resolve_program(&products, &device.program_ref).unwrap()
        else {
            continue;
        };
        for &com_id in &device.com_objects {
            let Some(com) = project.devices.com_object(com_id) else {
                continue;
            };
            if !matches!(com.dpt, Override::Empty) {
                continue;
            }
            let lookup_id = knx_productdb::com_object_lookup_id(
                &program_id,
                &com.source.ets_id,
                com.module_instance.is_some(),
            );
            let view =
                knx_productdb::query::com_object_view(&products, &program_id, &lookup_id, None)
                    .unwrap();
            match view.and_then(|v| v.dpt_list) {
                Some(_) => liftable += 1,
                None => not_liftable += 1,
            }
        }
    }
    assert_eq!(
        liftable, 122,
        "Empty dpt slots with a program value behind them"
    );
    assert_eq!(
        not_liftable, 375,
        "Empty dpt slots the program never stated either"
    );
}

/// The same gap, for `description`: 82 of 907 are `Empty`, `text` never
/// is (a program always states a communication object's own text), and
/// every one of those 82 has a program-layer value behind it — unlike
/// `dpt`, description's program value is never itself ambiguous, so this
/// side of gap 2 has no unresolved remainder to report.
#[test]
fn gap2_description_and_text_empty_counts() {
    let Some((imported, products)) = import() else {
        return;
    };
    let project = &imported.project;

    let (mut text_empty, mut desc_empty) = (0usize, 0usize);
    for device in project.devices.iter() {
        for &com_id in &device.com_objects {
            let Some(com) = project.devices.com_object(com_id) else {
                continue;
            };
            if matches!(com.text, Override::Empty) {
                text_empty += 1;
            }
            if matches!(com.description, Override::Empty) {
                desc_empty += 1;
            }
        }
    }
    assert_eq!(text_empty, 0);
    assert_eq!(desc_empty, 82);

    let mut liftable = 0usize;
    for device in project.devices.iter() {
        let Some(program_id) =
            knx_productdb::query::resolve_program(&products, &device.program_ref).unwrap()
        else {
            continue;
        };
        for &com_id in &device.com_objects {
            let Some(com) = project.devices.com_object(com_id) else {
                continue;
            };
            if !matches!(com.description, Override::Empty) {
                continue;
            }
            let lookup_id = knx_productdb::com_object_lookup_id(
                &program_id,
                &com.source.ets_id,
                com.module_instance.is_some(),
            );
            let view =
                knx_productdb::query::com_object_view(&products, &program_id, &lookup_id, None)
                    .unwrap();
            if view.and_then(|v| v.visible_description).is_some() {
                liftable += 1;
            }
        }
    }
    assert_eq!(
        liftable, 82,
        "every Empty description has a program value behind it"
    );
}

/// Gap 3's real footprint, re-measured rather than assumed. `enrich()`
/// currently raises `EnrichmentIssue::AmbiguousDpt` whenever a
/// `ComObjectRef`'s own `DatapointType` list is ambiguous, regardless of
/// what the instance slot already held — so of 107 raised issues, 0 land
/// on an `Empty` slot, 85 land on a slot the instance already stated its
/// own value for (informational noise: nothing was ever going to be
/// filled there), and only 22 land on a genuinely `Absent` slot that
/// stays unfilled specifically *because* of the ambiguity. Those 22 span
/// 11 devices and 51 distinct `ComObjectRef`s carry an ambiguous list in
/// total.
#[test]
fn gap3_ambiguous_dpt_the_107_issues_and_the_22_that_actually_block_a_fill() {
    let Some((imported, products)) = import() else {
        return;
    };
    let report = imported.enrichment.as_ref().unwrap();
    let ambiguous_issues = report
        .issues
        .iter()
        .filter(|i| matches!(i, knx_productdb::EnrichmentIssue::AmbiguousDpt { .. }))
        .count();
    assert_eq!(report.issues.len(), 107);
    assert_eq!(ambiguous_issues, 107, "every current issue is AmbiguousDpt");

    let project = &imported.project;
    let (mut and_empty, mut and_absent, mut and_value) = (0usize, 0usize, 0usize);
    let mut devices = HashSet::new();
    let mut refs = HashSet::new();

    for device in project.devices.iter() {
        let Some(program_id) =
            knx_productdb::query::resolve_program(&products, &device.program_ref).unwrap()
        else {
            continue;
        };
        for &com_id in &device.com_objects {
            let Some(com) = project.devices.com_object(com_id) else {
                continue;
            };
            let lookup_id = knx_productdb::com_object_lookup_id(
                &program_id,
                &com.source.ets_id,
                com.module_instance.is_some(),
            );
            let Some(view) =
                knx_productdb::query::com_object_view(&products, &program_id, &lookup_id, None)
                    .unwrap()
            else {
                continue;
            };
            let Some(list) = view.dpt_list else {
                continue;
            };
            let alts: Vec<&str> = list.split_whitespace().collect();
            let is_ambiguous =
                alts.len() > 1 || (alts.len() == 1 && knx_core::DptRef::parse(alts[0]).is_err());
            if !is_ambiguous {
                continue;
            }
            devices.insert(device.id);
            refs.insert(lookup_id);
            match &com.dpt {
                Override::Empty => and_empty += 1,
                Override::Absent => and_absent += 1,
                Override::Value(_) => and_value += 1,
                Override::Malformed(_) => {}
            }
        }
    }

    assert_eq!(and_empty, 0);
    assert_eq!(and_absent, 22, "genuinely blocked by the ambiguity");
    assert_eq!(
        and_value, 85,
        "already stated at instance level; informational only"
    );
    assert_eq!(devices.len(), 11);
    assert_eq!(refs.len(), 51);
}
