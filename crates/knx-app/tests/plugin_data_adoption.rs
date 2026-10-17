//! Data-only plug-in adoption: exact replacement, honest refusal, nothing lost.
//!
//! Archives are synthetic ETS-shaped ZIPs built here. The plug-in "bytes" are
//! arbitrary strings; nothing in KNXBench (or here) gives them a meaning.

use knx_app::plugin_data::{self as adopt, Channel, Refusal, Request};
use knx_app::selective_import as import;
use knx_store::project_history::NativeSnapshot;
use knx_testsupport::PluginFixture as Variant;

const BLOB_PATH: &str = "P-0001/BinaryData/b113ce24-349e-45db-8ad5-c2a7003d0edf.dat";
const RBG_PATH: &str = "P-0001/ExtraData/393256.rbg";
const OTHER_RBG_PATH: &str = knx_testsupport::PLUGIN_FIXTURE_OTHER_RBG_PATH;

fn source(v: &Variant) -> import::Source {
    import::source_from_bytes(v.knxproj_bytes(), None).expect("synthetic archive imports")
}

fn target(v: &Variant) -> NativeSnapshot {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.knxproj");
    std::fs::write(&path, v.knxproj_bytes()).unwrap();
    let store = dir.path().join("t.knxdb");
    let conn = knx_store::open_and_migrate(&store).unwrap();
    let imported = knx_app::import_ets_project(&path, &conn).unwrap();
    assert_eq!(imported.report.error_count(), 0);
    knx_store::save_project(&conn, &imported.project).unwrap();
    NativeSnapshot::read(&conn).unwrap()
}

fn request(source: &import::Source, target: &NativeSnapshot) -> Request {
    Request {
        source_device: source.project.devices.iter().next().unwrap().id.0,
        target_device: target.project.devices.iter().next().unwrap().id.0,
    }
}

fn bytes<'a>(snapshot: &'a NativeSnapshot, path: &str) -> &'a [u8] {
    let mut hits = snapshot
        .opaque
        .iter()
        .filter(|e| e.xpath.is_empty() && e.source_path == path);
    let entry = hits.next().unwrap_or_else(|| panic!("{path} missing"));
    assert!(hits.next().is_none(), "{path} duplicated");
    assert_eq!(entry.sha256, knx_etsproj::opaque::sha256_hex(&entry.bytes));
    &entry.bytes
}

fn refusal(result: Result<adopt::Plan, Refusal>) -> Refusal {
    result.expect_err("must refuse")
}

#[test]
fn edited_plugin_bytes_replace_the_old_ones_verbatim_and_nothing_else_moves() {
    let src = source(&Variant::edited());
    let mut dst = target(&Variant::default());
    let before = dst.clone();
    let plan = adopt::plan(&dst, &src, request(&src, &dst)).unwrap();

    let channels: Vec<_> = plan
        .preview
        .replacements
        .iter()
        .map(|r| (r.channel, r.path.as_str()))
        .collect();
    assert_eq!(
        channels,
        vec![
            (Channel::BinaryData, BLOB_PATH),
            (Channel::ExtraData, RBG_PATH)
        ]
    );
    // The other device's table is neither named nor touched.
    assert!(!plan
        .preview
        .unchanged_paths
        .iter()
        .any(|p| p == OTHER_RBG_PATH));

    let preview = adopt::apply(&mut dst, plan).unwrap();
    assert_eq!(preview.replacements.len(), 2);
    assert_eq!(bytes(&dst, BLOB_PATH), Variant::edited().blob);
    assert_eq!(bytes(&dst, RBG_PATH), Variant::edited().rbg);
    assert_eq!(bytes(&dst, OTHER_RBG_PATH), Variant::default().other_rbg);
    assert_eq!(dst.project, before.project, "the model is not touched");
    assert_eq!(dst.manufacturer_refs, before.manufacturer_refs);

    // Every pre-existing entry other than the two replaced ones is identical.
    for old in &before.opaque {
        if old.source_path == BLOB_PATH || old.source_path == RBG_PATH {
            continue;
        }
        assert!(dst.opaque.contains(old), "{} lost", old.source_path);
    }
}

#[test]
fn superseded_bytes_are_retained_and_the_adoption_is_recorded() {
    let src = source(&Variant::edited());
    let mut dst = target(&Variant::default());
    let plan = adopt::plan(&dst, &src, request(&src, &dst)).unwrap();
    adopt::apply(&mut dst, plan).unwrap();

    let superseded: Vec<_> = dst
        .opaque
        .iter()
        .filter(|e| e.kind == adopt::SUPERSEDED_KIND)
        .collect();
    assert_eq!(superseded.len(), 2);
    let blob_old = superseded.iter().find(|e| e.name == BLOB_PATH).unwrap();
    assert_eq!(blob_old.bytes, Variant::default().blob);
    assert!(blob_old
        .source_path
        .starts_with("knxbench-native/plugin-data/superseded/"));
    let rbg_old = superseded.iter().find(|e| e.name == RBG_PATH).unwrap();
    assert_eq!(rbg_old.bytes, Variant::default().rbg);

    let records: Vec<_> = dst
        .opaque
        .iter()
        .filter(|e| e.kind == adopt::ADOPTION_KIND)
        .collect();
    assert_eq!(records.len(), 1);
    let record: serde_json::Value = serde_json::from_slice(&records[0].bytes).unwrap();
    assert_eq!(record["format"], "knxbench-plugin-data-adoption");
    assert_eq!(record["sourceArchiveSha256"], src.source_hash.as_str());
    assert_eq!(record["replacements"].as_array().unwrap().len(), 2);
    assert!(
        !String::from_utf8_lossy(&records[0].bytes).contains("blob v2"),
        "the record names bytes by digest, it does not copy them"
    );
}

#[test]
fn the_adopted_project_saves_and_reopens_exactly() {
    let src = source(&Variant::edited());
    let mut dst = target(&Variant::default());
    let plan = adopt::plan(&dst, &src, request(&src, &dst)).unwrap();
    adopt::apply(&mut dst, plan).unwrap();

    let dir = tempfile::tempdir().unwrap();
    let conn = knx_store::open_and_migrate(&dir.path().join("a.knxdb")).unwrap();
    knx_store::save_project_with_passthrough(
        &conn,
        &dst.project,
        &dst.opaque,
        &dst.manufacturer_refs,
    )
    .unwrap();
    assert_eq!(NativeSnapshot::read(&conn).unwrap(), dst);
}

#[test]
fn adopting_the_same_bytes_twice_is_refused_as_nothing_to_do() {
    let src = source(&Variant::edited());
    let mut dst = target(&Variant::default());
    let plan = adopt::plan(&dst, &src, request(&src, &dst)).unwrap();
    adopt::apply(&mut dst, plan).unwrap();
    assert_eq!(
        refusal(adopt::plan(&dst, &src, request(&src, &dst))),
        Refusal::AlreadyIdentical
    );
}

#[test]
fn identical_projects_have_nothing_to_adopt() {
    let src = source(&Variant::default());
    let dst = target(&Variant::default());
    assert_eq!(
        refusal(adopt::plan(&dst, &src, request(&src, &dst))),
        Refusal::AlreadyIdentical
    );
}

#[test]
fn only_the_changed_channel_is_replaced() {
    let edited = Variant {
        rbg: b"only the table changed",
        ..Variant::default()
    };
    let src = source(&edited);
    let dst = target(&Variant::default());
    let plan = adopt::plan(&dst, &src, request(&src, &dst)).unwrap();
    assert_eq!(plan.preview.replacements.len(), 1);
    assert_eq!(plan.preview.replacements[0].channel, Channel::ExtraData);
    assert_eq!(plan.preview.unchanged_paths, vec![BLOB_PATH.to_string()]);
}

#[test]
fn another_product_or_application_version_is_refused() {
    let dst = target(&Variant::default());
    for (variant, what) in [
        (
            Variant {
                program: "M-0001_H-1_HP-2",
                ..Variant::edited()
            },
            "application program reference",
        ),
        (
            Variant {
                product: "M-0001_H-1_P-2",
                ..Variant::edited()
            },
            "product reference",
        ),
    ] {
        let src = source(&variant);
        match refusal(adopt::plan(&dst, &src, request(&src, &dst))) {
            Refusal::ProductDiffers { what: got, .. } => assert_eq!(got, what),
            other => panic!("{other:?}"),
        }
    }
}

#[test]
fn different_blob_or_table_references_are_not_remapped() {
    let dst = target(&Variant::default());
    let src = source(&Variant {
        blob_id: "2868e24a-9fc3-4099-82f0-3f535bde8fc1",
        ..Variant::edited()
    });
    assert_eq!(
        refusal(adopt::plan(&dst, &src, request(&src, &dst))),
        Refusal::BinaryDataReferencesDiffer
    );
    let src = source(&Variant {
        extra_value: "393257",
        ..Variant::edited()
    });
    assert_eq!(
        refusal(adopt::plan(&dst, &src, request(&src, &dst))),
        Refusal::ExtraDataAssociationDiffers
    );
}

#[test]
fn changed_group_address_links_are_refused() {
    let dst = target(&Variant::default());
    let src = source(&Variant {
        group_address: 2,
        ..Variant::edited()
    });
    assert_eq!(
        refusal(adopt::plan(&dst, &src, request(&src, &dst))),
        Refusal::GroupLinksDiffer
    );
}

#[test]
fn a_different_individual_address_is_reported_not_refused() {
    let src = source(&Variant {
        address: "2",
        ..Variant::edited()
    });
    let dst = target(&Variant::default());
    let plan = adopt::plan(&dst, &src, request(&src, &dst)).unwrap();
    assert!(plan
        .preview
        .notes
        .iter()
        .any(|n| n.contains("individual addresses")));
}

#[test]
fn missing_and_ambiguous_entries_are_named() {
    let src = source(&Variant::edited());
    let dst = target(&Variant::default());

    let mut missing = dst.clone();
    missing.opaque.retain(|e| e.source_path != BLOB_PATH);
    match refusal(adopt::plan(&missing, &src, request(&src, &missing))) {
        Refusal::EntryMissing { side, path } => {
            assert_eq!(side, "destination");
            assert_eq!(path, BLOB_PATH);
        }
        other => panic!("{other:?}"),
    }

    let mut ambiguous = dst.clone();
    let copy = ambiguous
        .opaque
        .iter()
        .find(|e| e.source_path == BLOB_PATH)
        .unwrap()
        .clone();
    ambiguous.opaque.push(copy);
    assert!(matches!(
        refusal(adopt::plan(&ambiguous, &src, request(&src, &ambiguous))),
        Refusal::EntryAmbiguous {
            side: "destination",
            ..
        }
    ));
}

#[test]
fn unknown_devices_and_stale_destinations_are_refused() {
    let src = source(&Variant::edited());
    let mut dst = target(&Variant::default());
    let good = request(&src, &dst);
    assert_eq!(
        refusal(adopt::plan(
            &dst,
            &src,
            Request {
                source_device: 9999,
                ..good
            }
        )),
        Refusal::UnknownDevice {
            side: "source",
            id: 9999
        }
    );
    assert_eq!(
        refusal(adopt::plan(
            &dst,
            &src,
            Request {
                target_device: 9999,
                ..good
            }
        )),
        Refusal::UnknownDevice {
            side: "destination",
            id: 9999
        }
    );

    let plan = adopt::plan(&dst, &src, good).unwrap();
    dst.project.info.name = "edited after the preview".into();
    let untouched = dst.clone();
    assert_eq!(adopt::apply(&mut dst, plan).unwrap_err(), Refusal::Stale);
    assert_eq!(dst, untouched, "a stale apply changes nothing");
}

#[test]
fn a_device_without_any_plugin_data_has_nothing_to_adopt() {
    // "none" is not a file number: the device has no ExtraData association.
    let plain = Variant {
        extra_value: "none",
        ..Variant::default()
    };
    let src = source(&plain);
    let dst = target(&plain);
    // Both still carry a BinaryData reference, so remove it from the model.
    let mut src_view_project = src.project.clone();
    let mut dst = dst;
    for id in src_view_project
        .devices
        .iter()
        .map(|d| d.id)
        .collect::<Vec<_>>()
    {
        src_view_project
            .devices
            .get_mut(id)
            .unwrap()
            .binary_data
            .clear();
    }
    for id in dst.project.devices.iter().map(|d| d.id).collect::<Vec<_>>() {
        dst.project.devices.get_mut(id).unwrap().binary_data.clear();
    }
    let view = adopt::SourceView {
        project: &src_view_project,
        opaque: &[],
        entry_prefix: String::new(),
        source_hash: "synthetic",
    };
    let request = request_for(&src_view_project, &dst);
    assert_eq!(
        refusal(adopt::plan_from(&dst, &view, request)),
        Refusal::NoPluginData
    );
}

fn request_for(project: &knx_core::Project, target: &NativeSnapshot) -> Request {
    Request {
        source_device: project.devices.iter().next().unwrap().id.0,
        target_device: target.project.devices.iter().next().unwrap().id.0,
    }
}

#[test]
fn the_confirmation_token_binds_source_request_and_scope() {
    let src = source(&Variant::edited());
    let dst = target(&Variant::default());
    let plan = adopt::plan(&dst, &src, request(&src, &dst)).unwrap();
    let again = adopt::plan(&dst, &src, request(&src, &dst)).unwrap();
    let token = plan.confirmation_token("cli-v1:/a:0").unwrap();
    assert_eq!(token, again.confirmation_token("cli-v1:/a:0").unwrap());
    assert_ne!(token, plan.confirmation_token("cli-v1:/b:0").unwrap());
    assert_ne!(token, plan.confirmation_token("cli-v1:/a:1").unwrap());

    let other = source(&Variant {
        blob: b"a different edit",
        ..Variant::edited()
    });
    let other_plan = adopt::plan(&dst, &other, request(&other, &dst)).unwrap();
    assert_ne!(token, other_plan.confirmation_token("cli-v1:/a:0").unwrap());
}

#[test]
fn candidates_pair_devices_by_ets_id_and_say_why_not() {
    let dst = target(&Variant::default());

    let good = source(&Variant::edited());
    let found = adopt::candidates(&dst, &good);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].refusal, None);
    assert_eq!(found[0].replacements, 2);

    let bad = source(&Variant {
        group_address: 2,
        ..Variant::edited()
    });
    let found = adopt::candidates(&dst, &bad);
    assert_eq!(found.len(), 1);
    assert!(found[0]
        .refusal
        .as_deref()
        .unwrap()
        .contains("group addresses"));
}

#[test]
fn a_source_archive_is_read_only_evidence_never_executed() {
    // The module has no way to run anything: its inputs are bytes and its
    // outputs are bytes. This pins the one observable consequence — planning
    // does not write the destination, so a dropped plan leaves it untouched.
    let src = source(&Variant::edited());
    let dst = target(&Variant::default());
    let snapshot = dst.clone();
    drop(adopt::plan(&dst, &src, request(&src, &dst)).unwrap());
    assert_eq!(dst, snapshot);
}

#[test]
fn a_device_that_arrived_through_selective_import_is_found_under_its_import_prefix() {
    // Selective import keeps a device's source path and its archive entries
    // under `imports/<hash>/`; adoption must look where the device says it is.
    let src = source(&Variant::edited());
    let mut dst = target(&Variant::default());
    let prefix = "imports/0123abcd/";
    for id in dst.project.devices.iter().map(|d| d.id).collect::<Vec<_>>() {
        let device = dst.project.devices.get_mut(id).unwrap();
        device.source.path = format!("{prefix}{}", device.source.path);
    }
    for entry in &mut dst.opaque {
        if entry.source_path.contains("/BinaryData/") || entry.source_path.contains("/ExtraData/") {
            entry.source_path = format!("{prefix}{}", entry.source_path);
        }
    }
    let plan = adopt::plan(&dst, &src, request(&src, &dst)).unwrap();
    let paths: Vec<_> = plan
        .preview
        .replacements
        .iter()
        .map(|r| r.path.clone())
        .collect();
    assert_eq!(
        paths,
        vec![
            format!("{prefix}{BLOB_PATH}"),
            format!("{prefix}{RBG_PATH}")
        ]
    );
    adopt::apply(&mut dst, plan).unwrap();
    assert_eq!(
        bytes(&dst, &format!("{prefix}{BLOB_PATH}")),
        Variant::edited().blob
    );
}
