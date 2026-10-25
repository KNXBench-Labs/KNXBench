//! `source_still_plans` admits only a source that plans exactly the bytes and steps that ran.

use knx_app::device_download::source_still_plans;

use super::project_edits::{edit_time, install_rich_project};
use super::*;

/// Whether the live catalogue of `f` still plans `prepared`.
fn still_plans(f: &Fixture, prepared: &PreparedDownload) -> bool {
    f.state
        .product_db
        .as_ref()
        .unwrap()
        .with_read(|conn| {
            Ok::<_, std::convert::Infallible>(source_still_plans(
                conn,
                &prepared.request,
                prepared.partial.as_ref().map(|(parts, _)| *parts),
                &prepared.plan,
            ))
        })
        .expect("owner readable")
        .unwrap()
}

/// A catalogue serving `new` against a download prepared from `old`; `None`
/// when the plans do not differ, which would make the case vacuous.
fn after_restore(old: &Source, new: &Source) -> (Fixture, PreparedDownload) {
    let f = fixture(old, new);
    let prepared = prepare(&f.state);
    restore(&f);
    (f, prepared)
}

#[test]
fn the_same_source_is_admitted_for_the_full_and_the_partial_plan() {
    let (f, prepared) = after_restore(&OLD, &OLD);
    assert!(still_plans(&f, &prepared));
    let partial = prepare(&f.state)
        .into_partial(PartialDownloadParts {
            parameters: true,
            group_addresses: false,
        })
        .unwrap();
    assert!(still_plans(&f, &partial));
}

#[test]
fn a_changed_mask_check_or_task_segment_is_not_the_same_plan() {
    for changed in [
        Source {
            check: "000000000128",
            ..OLD
        },
        Source { task: 17409, ..OLD },
        Source { access: 254, ..OLD },
    ] {
        let (f, prepared) = after_restore(&OLD, &changed);
        let now = prepare(&f.state);
        assert_eq!(now.request, prepared.request, "{changed:?}");
        assert_ne!(now.plan, prepared.plan, "precondition: {changed:?}");
        assert!(!still_plans(&f, &prepared), "{changed:?}");
    }
}

#[test]
fn a_source_that_now_declares_a_download_plugin_is_refused_not_admitted() {
    let plugin = Source {
        extension: r#"<Extension EtsDownloadPlugin="{00000000-0000-4000-8000-00000000F006}" RequiresExternalSoftware="0" />"#,
        ..OLD
    };
    let (f, prepared) = after_restore(&OLD, &plugin);
    assert!(!still_plans(&f, &prepared));
}

#[test]
fn an_absent_source_is_not_admitted() {
    let f = fixture(&OLD, &OLD);
    let prepared = prepare(&f.state);
    std::fs::remove_file(&f.backup).unwrap();
    drop(knx_productdb::open_and_migrate(&f.backup).unwrap());
    restore(&f);
    assert!(!still_plans(&f, &prepared));
}

#[test]
fn the_check_replans_the_request_that_ran_not_the_edited_project() {
    let f = fixture(&OLD, &OLD);
    install_rich_project(&f.state);
    let ran = prepare(&f.state);
    edit_time(&f.state);
    let edited = prepare(&f.state);
    assert_ne!(
        edited.plan, ran.plan,
        "precondition: the edit changes the plan"
    );
    assert!(
        still_plans(&f, &ran),
        "the original request still plans the plan that ran"
    );
    // The edited project's request does not plan what ran.
    let replans = f
        .state
        .product_db
        .as_ref()
        .unwrap()
        .with_read(|conn| {
            Ok::<_, std::convert::Infallible>(source_still_plans(
                conn,
                &edited.request,
                None,
                &ran.plan,
            ))
        })
        .expect("owner readable")
        .unwrap();
    assert!(!replans);
}
