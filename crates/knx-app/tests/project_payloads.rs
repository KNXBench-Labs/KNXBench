//! Retained payloads are inspectable/downloadable bytes, never executable support.

use knx_app::project_payloads;
use knx_store::StoredOpaqueEntry;

fn payload(path: &str, kind: &str, bytes: &[u8]) -> StoredOpaqueEntry {
    StoredOpaqueEntry {
        source_path: path.into(),
        xpath: String::new(),
        kind: kind.into(),
        name: String::new(),
        bytes: bytes.to_vec(),
        sha256: knx_store::project_history::image_hash(bytes),
    }
}
#[test]
fn catalogue_preserves_exact_paths_and_distinguishes_candidate_role_from_content() {
    let mut entries = vec![
        payload(
            "P-FICTION/UserFiles/document.html",
            "ContainerEntry",
            b"<script>foreign()</script>",
        ),
        payload(
            "P-FICTION/AddinData/state.dat",
            "ContainerEntry",
            b"opaque addin",
        ),
        payload(
            "tools/example.etsapp",
            "ContainerEntry",
            b"PK\x03\x04fictional",
        ),
        payload(
            "M-FICTION/Baggages/picture.png",
            "Baggage",
            b"\x89PNG\r\n\x1a\nfictional",
        ),
        payload(
            "knxbench-native/import-report.json",
            "KnxBenchImportEvidence",
            b"{}",
        ),
        payload(
            "P-FICTION/0.xml",
            "ContainerEntry",
            b"<?xml version=\"1.0\"?>",
        ),
    ];
    let mut attribute = payload(
        "P-FICTION/UserFiles/fragment",
        "RetainedAttribute",
        b"fragment",
    );
    attribute.xpath = "/x/@v".into();
    entries.push(attribute);
    let before = entries.clone();
    let catalogue = project_payloads::catalogue(&entries).unwrap();
    assert_eq!(
        catalogue.len(),
        4,
        "payloads remained hidden or unrelated metadata was exposed"
    );
    assert_eq!(catalogue[0].source_path, entries[0].source_path);
    assert_eq!(catalogue[0].role, "user-file-candidate");
    assert_eq!(catalogue[0].content_class, "unknown");
    assert_eq!(catalogue[2].role, "ets-app-named-candidate");
    assert_eq!(catalogue[2].content_class, "zip");
    assert_eq!(catalogue[3].content_class, "png");
    assert_eq!(
        project_payloads::download(&entries, catalogue[0].id).unwrap(),
        entries[0].bytes.as_slice()
    );
    assert_eq!(entries, before);
}
#[test]
fn actual_import_and_native_reopen_keep_catalogue_bytes_model_and_manifest() {
    let xml = br#"<KNX xmlns="http://knx.org/xml/project/23"><Project Id="P-FICTION"><Installations><Installation InstallationId="0"><Topology/><GroupAddresses/></Installation></Installations></Project></KNX>"#;
    let user = b"<script>foreign()</script>";
    let extension = b"PK\x03\x04fictional app";
    let baggage = b"\x89PNG\r\n\x1a\nfictional";
    let archive = knx_testsupport::zip_with_entries(&[
        ("P-FICTION.signature", b""),
        ("P-FICTION/0.xml", xml),
        ("P-FICTION/project.xml", xml),
        ("P-FICTION/UserFiles/document.html", user),
        ("P-FICTION/AddinData/tool.etsapp", extension),
        ("M-FICTION/Baggages/picture.png", baggage),
    ]);
    let conn = knx_store::open_and_migrate_in_memory().unwrap();
    let imported = knx_app::import::import_ets_project_bytes(
        archive,
        "fictional.knxproj",
        &conn,
        Default::default(),
    )
    .unwrap();
    let rows = knx_store::load_opaque(&conn).unwrap();
    let refs = knx_store::load_manufacturer_refs(&conn).unwrap();
    assert_eq!(refs.len(), 1);
    let before = imported.project.clone();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fictional.knxdb");
    let native = knx_store::open_and_migrate(&path).unwrap();
    knx_store::save_project_with_passthrough(&native, &imported.project, &rows, &refs).unwrap();
    drop(native);
    let reopened = knx_store::open_existing_and_migrate(&path).unwrap();
    let retained = knx_store::load_opaque(&reopened).unwrap();
    let catalogue = project_payloads::catalogue(&retained).unwrap();
    assert_eq!(catalogue.len(), 3);
    for (source, expected) in [
        ("P-FICTION/UserFiles/document.html", user.as_slice()),
        ("P-FICTION/AddinData/tool.etsapp", extension.as_slice()),
        ("M-FICTION/Baggages/picture.png", baggage.as_slice()),
    ] {
        let row = catalogue
            .iter()
            .find(|row| row.source_path == source)
            .unwrap();
        assert_eq!(
            project_payloads::download(&retained, row.id).unwrap(),
            expected
        );
    }
    assert_eq!(retained, rows);
    assert_eq!(knx_store::load_manufacturer_refs(&reopened).unwrap(), refs);
    assert_eq!(knx_store::load_project(&reopened).unwrap(), before);
}

#[test]
fn selected_hash_mismatch_and_unlisted_selector_refuse_without_changing_any_data() {
    let mut entries = vec![payload(
        "P-FICTION/UserFiles/private.bin",
        "ContainerEntry",
        b"exact bytes",
    )];
    entries[0].bytes[0] ^= 1;
    let before = entries.clone();
    assert!(project_payloads::download(&entries, 0).is_err());
    assert!(project_payloads::download(&entries, 1).is_err());
    assert_eq!(entries, before);
}
#[test]
fn excessive_catalogue_refuses_completely_instead_of_returning_a_truncated_list() {
    let entries = vec![
        payload(
            "P-FICTION/UserFiles/repeated.bin",
            "ContainerEntry",
            b"fictional"
        );
        project_payloads::MAX_PAYLOADS + 1
    ];
    assert!(project_payloads::catalogue(&entries).is_err());
}

#[test]
fn metadata_budget_refuses_the_whole_catalogue_before_unbounded_cloning() {
    let long_path = format!("P-FICTION/UserFiles/{}", "x".repeat(4 * 1024 * 1024));
    let entries = vec![payload(&long_path, "ContainerEntry", b"small payload")];
    assert!(project_payloads::catalogue(&entries).is_err());
    assert_eq!(
        project_payloads::download(&entries, 0).unwrap(),
        b"small payload"
    );
}

#[test]
fn display_only_source_paths_never_become_download_destinations() {
    let entries = vec![payload(
        "P-FICTION/UserFiles/../../elsewhere.html",
        "ContainerEntry",
        b"exact foreign bytes",
    )];
    assert_eq!(
        project_payloads::catalogue(&entries).unwrap()[0].source_path,
        entries[0].source_path
    );
    assert_eq!(
        project_payloads::download(&entries, 0).unwrap(),
        b"exact foreign bytes"
    );
    assert!(project_payloads::download(&entries, u32::MAX).is_err());
}

#[test]
fn larger_payload_remains_listed_but_explicit_download_budget_refuses() {
    let entries = vec![payload(
        "P-FICTION/UserFiles/large.bin",
        "ContainerEntry",
        &vec![b'x'; project_payloads::MAX_DOWNLOAD_BYTES + 1],
    )];
    let catalogue = project_payloads::catalogue(&entries).unwrap();
    assert_eq!(catalogue.len(), 1);
    assert!(!catalogue[0].downloadable);
    assert!(project_payloads::download(&entries, catalogue[0].id).is_err());
}
