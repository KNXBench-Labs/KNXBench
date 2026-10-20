//! WP-16 selected-member restoration is deliberately not whole-archive atomic.

use super::project_stream_fixture as stream;
use knx_app::manufacturer_files::restore_from_knxproj;
use knx_store::ManufacturerRef;

fn reference(path: &str, bytes: &[u8]) -> ManufacturerRef {
    ManufacturerRef {
        source_path: path.into(),
        sha256: knx_store::manufacturer_file_hash(bytes),
        len: bytes.len() as i64,
        kind: "ManufacturerData".into(),
    }
}

#[test]
fn wp16_restore_continues_after_selected_stream_failure_and_keeps_earlier_new_install() {
    let dir = tempfile::tempdir().unwrap();
    let product_path = dir.path().join("products.sqlite");
    let products = knx_productdb::open_and_migrate(&product_path).unwrap();
    let first = stream::manufacturer("restore-first-new");
    let damaged = stream::manufacturer("restore-damaged-new");
    let last = stream::manufacturer("restore-last-new");
    let present = stream::manufacturer("restore-already-present");
    let refs = vec![
        reference(stream::EARLY, &first),
        reference(stream::LATE_MANUFACTURER, &damaged),
        reference("M-0001/after.xml", &last),
        reference("M-0001/present.xml", &present),
    ];
    knx_productdb::ingest_file(&products, &refs[3].source_path, &present).unwrap();
    for r in &refs[..3] {
        assert!(!knx_productdb::has_source_file(&products, &r.sha256).unwrap());
    }
    let source = stream::archive(&[
        stream::stored(stream::EARLY, &first),
        stream::deflated(stream::LATE_MANUFACTURER, &damaged, true),
        stream::stored("M-0001/after.xml", &last),
        // This damaged member must not be read at all: its hash is present.
        stream::deflated("M-0001/present.xml", &present, true),
    ]);
    let source_path = dir.path().join("fixed-restore.knxproj");
    std::fs::write(&source_path, &source).unwrap();
    let report =
        restore_from_knxproj(&products, &refs, std::fs::read(&source_path).unwrap()).unwrap();
    assert_eq!(report.failed.len(), 1, "late selected stream must be a per-member failure, not installed or whole-archive rollback");
    assert_eq!(report.failed[0].0, stream::LATE_MANUFACTURER);
    assert!(report.failed[0].1.contains(stream::LATE_MANUFACTURER));
    assert_eq!(report.installed, [stream::EARLY, "M-0001/after.xml"]);
    assert_eq!(report.already_installed, 1);
    assert!(report.mismatched.is_empty() && report.not_in_archive.is_empty());
    assert!(!knx_productdb::has_source_file(&products, &refs[1].sha256).unwrap());
    for (r, expected) in [(&refs[0], &first), (&refs[2], &last), (&refs[3], &present)] {
        assert_eq!(
            knx_productdb::load_source_file(&products, &r.sha256)
                .unwrap()
                .unwrap(),
            *expected
        );
    }
    drop(products);
    let reopened = knx_productdb::open_and_migrate(&product_path).unwrap();
    assert!(!knx_productdb::has_source_file(&reopened, &refs[1].sha256).unwrap());
    for (r, expected) in [(&refs[0], &first), (&refs[2], &last), (&refs[3], &present)] {
        assert_eq!(
            knx_productdb::load_source_file(&reopened, &r.sha256)
                .unwrap()
                .unwrap(),
            *expected
        );
    }
    assert_eq!(std::fs::read(source_path).unwrap(), source);
}

#[test]
fn wp16_restore_does_not_reread_already_present_or_unselected_damaged_streams() {
    let dir = tempfile::tempdir().unwrap();
    let products = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    let present = stream::manufacturer("restore-already-present");
    let fresh = stream::manufacturer("restore-selected-new");
    let refs = [
        reference("M-0001/present.xml", &present),
        reference(stream::EARLY, &fresh),
    ];
    knx_productdb::ingest_file(&products, &refs[0].source_path, &present).unwrap();
    let source = stream::archive(&[
        stream::deflated("M-0001/present.xml", &present, true),
        stream::deflated(
            "M-0001/unselected.xml",
            &stream::manufacturer("restore-unselected"),
            true,
        ),
        stream::deflated(stream::EARLY, &fresh, false),
    ]);
    let report = restore_from_knxproj(&products, &refs, source).unwrap();
    assert_eq!(report.installed, [stream::EARLY]);
    assert_eq!(report.already_installed, 1);
    assert!(
        report.failed.is_empty()
            && report.mismatched.is_empty()
            && report.not_in_archive.is_empty()
    );
    assert_eq!(
        knx_productdb::load_source_file(&products, &refs[1].sha256)
            .unwrap()
            .unwrap(),
        fresh
    );
}
