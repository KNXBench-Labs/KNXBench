//! Measures aggregate master-language shapes in an explicitly scoped private corpus.
//!
//! This research probe emits aggregate XML paths
//! and attribute names only: no source identities, paths, values or hashes.
//! Corpus discovery uses the existing descriptor-confined bounded helper;
//! originals are never extracted, modified or copied into the repository.
mod corpus_support;

use std::collections::{BTreeMap, HashSet};
use std::io::{Cursor, Read};

use quick_xml::events::Event;
use quick_xml::name::ResolveResult;
use quick_xml::NsReader;

#[test]
#[ignore = "private corpus research: explicit root, scopes and output required"]
fn master_languages_shape_census() {
    let output = std::env::var_os("KNXBENCH_AR05_SHAPE_OUTPUT").expect("aggregate output required");
    run_census(
        corpus_support::configured_corpus(),
        std::path::Path::new(&output),
    );
}

fn run_census(corpus: corpus_support::ConfiguredCorpus, output: &std::path::Path) {
    let output = corpus_support::output::validate_output_path(
        &corpus.canonical_root,
        corpus.root_device,
        output,
    );
    corpus_support::output::remove_previous_output(&output);
    let corpus = corpus.discover();
    let mut budget = corpus_support::CorpusReadBudget::new();
    let mut packages = HashSet::new();
    let mut masters = HashSet::new();
    let mut shapes = BTreeMap::<String, BTreeMap<String, u64>>::new();
    let mut instance_count = 0usize;
    let mut expanded = 0u64;
    for source in corpus.packages {
        instance_count += 1;
        let bytes = source.load(&mut budget);
        if !packages.insert(knx_productdb::sha256_hex(&bytes)) {
            continue;
        }
        assert!(
            corpus_support::bounded_zip_entry_count(&bytes) <= 16_384,
            "archive entry budget exceeded"
        );
        let Ok(mut archive) = zip::ZipArchive::new(Cursor::new(bytes)) else {
            panic!("corpus package unreadable");
        };
        assert!(archive.len() <= 16_384, "archive entry budget exceeded");
        let Ok(mut member) = archive.by_name("knx_master.xml") else {
            panic!("corpus package has no root master");
        };
        assert!(
            member.size() <= 64 * 1024 * 1024,
            "master byte budget exceeded"
        );
        let mut data = Vec::new();
        assert!(member
            .by_ref()
            .take(64 * 1024 * 1024 + 1)
            .read_to_end(&mut data)
            .is_ok());
        assert!(
            data.len() <= 64 * 1024 * 1024,
            "decoded master budget exceeded"
        );
        expanded += data.len() as u64;
        assert!(
            expanded <= 1024 * 1024 * 1024,
            "cumulative master budget exceeded"
        );
        if !masters.insert(knx_productdb::sha256_hex(&data)) {
            continue;
        }
        let mut reader = NsReader::from_reader(data.as_slice());
        let mut buffer = Vec::new();
        let mut path = Vec::<(String, String)>::new();
        let mut events = 0usize;
        loop {
            buffer.clear();
            let Ok((namespace, event)) = reader.read_resolved_event_into(&mut buffer) else {
                panic!("master XML unreadable");
            };
            events += 1;
            assert!(
                events <= 1_000_000 && path.len() <= 1024,
                "master event/depth budget exceeded"
            );
            match event {
                Event::Start(ref element) | Event::Empty(ref element) => {
                    let local = element.local_name().as_ref().to_string();
                    let uri = match namespace {
                        ResolveResult::Bound(namespace) => namespace.as_ref().to_string(),
                        ResolveResult::Unbound => String::new(),
                        ResolveResult::Unknown(_) => panic!("master namespace unresolved"),
                    };
                    let language_branch = path.len() >= 2
                        && path[0].0 == "KNX"
                        && path[1].0 == "MasterData"
                        && ((path.len() == 2 && local == "Languages")
                            || (path.len() >= 3 && path[2].0 == "Languages"));
                    if language_branch {
                        let mut full_path = path
                            .iter()
                            .map(|(name, namespace)| format!("{{{namespace}}}{name}"))
                            .collect::<Vec<_>>();
                        full_path.push(format!("{{{uri}}}{local}"));
                        let counts = shapes.entry(full_path.join("/")).or_default();
                        *counts.entry("#elements".into()).or_default() += 1;
                        for attribute in element.attributes() {
                            let Ok(attribute) = attribute else {
                                panic!("master attribute unreadable");
                            };
                            if attribute.key.as_ref() == "xmlns"
                                || attribute.key.as_ref().starts_with("xmlns:")
                            {
                                continue;
                            }
                            let (namespace, name) =
                                reader.resolver().resolve_attribute(attribute.key);
                            let attr_uri = match namespace {
                                ResolveResult::Bound(namespace) => namespace.as_ref().to_string(),
                                ResolveResult::Unbound => String::new(),
                                ResolveResult::Unknown(_) => {
                                    panic!("attribute namespace unresolved")
                                }
                            };
                            *counts
                                .entry(format!("{{{attr_uri}}}{}", name.as_ref()))
                                .or_default() += 1;
                        }
                    }
                    if matches!(event, Event::Start(_)) {
                        path.push((local, uri));
                    }
                }
                Event::End(_) => {
                    path.pop();
                }
                Event::Eof => break,
                _ => {}
            }
        }
        assert!(path.is_empty(), "master incomplete");
    }
    let json = serde_json::json!({"instances":instance_count,"unique_packages":packages.len(),"unique_masters":masters.len(),"shapes":shapes});
    corpus_support::output::write_atomic(&output, &json);
    println!("AR05 aggregate shape census complete");
}

fn synthetic_corpus(directory: &std::path::Path) -> std::path::PathBuf {
    use std::io::Write;
    let root = directory.join("corpus");
    std::fs::create_dir(&root).unwrap();
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    writer
        .start_file("knx_master.xml", zip::write::SimpleFileOptions::default())
        .unwrap();
    writer.write_all(br#"<KNX xmlns="http://knx.org/xml/project/11"><MasterData><Languages/></MasterData></KNX>"#).unwrap();
    std::fs::write(
        root.join("synthetic.knxprod"),
        writer.finish().unwrap().into_inner(),
    )
    .unwrap();
    root
}

#[test]
fn census_refuses_to_overwrite_a_corpus_file() {
    let directory = tempfile::tempdir().unwrap();
    let root = synthetic_corpus(directory.path());
    let output = root.join("source.bin");
    std::fs::write(&output, b"retained synthetic source").unwrap();
    let corpus = corpus_support::open_corpus(&root, std::slice::from_ref(&root));
    let result =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run_census(corpus, &output)));
    assert_eq!(
        std::fs::read(&output).unwrap(),
        b"retained synthetic source"
    );
    assert!(result.is_err(), "corpus output destination was admitted");
}

#[test]
fn census_refuses_an_output_on_the_corpus_filesystem() {
    let directory = tempfile::tempdir().unwrap();
    let root = synthetic_corpus(directory.path());
    let output = directory.path().join("result.json");
    let corpus = corpus_support::open_corpus(&root, std::slice::from_ref(&root));
    let result =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run_census(corpus, &output)));
    assert!(result.is_err(), "same-filesystem output was admitted");
    assert!(!output.exists());
}

#[test]
fn census_invalidates_old_output_before_discovery_failure() {
    let directory = tempfile::tempdir().unwrap();
    let root = synthetic_corpus(directory.path());
    let output = directory.path().join("old-result.json");
    std::fs::write(&output, b"old synthetic result").unwrap();
    let mut corpus = corpus_support::open_corpus(&root, &[root.join("missing-scope")]);
    // Simulate a distinct input device for this failure-ordering unit test.
    // The real opt-in gate uses two actual separate filesystems.
    corpus.root_device = corpus.root_device.wrapping_add(1);
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run_census(corpus, &output)))
            .is_err()
    );
    assert!(
        !output.exists(),
        "failed discovery left stale census evidence"
    );
}

#[test]
fn census_checks_declared_entry_budget_before_opening_an_archive() {
    let directory = tempfile::tempdir().unwrap();
    let root = synthetic_corpus(directory.path());
    let archive = root.join("synthetic.knxprod");
    let mut bytes = std::fs::read(&archive).unwrap();
    let eocd = bytes
        .windows(4)
        .rposition(|bytes| bytes == b"PK\x05\x06")
        .unwrap();
    let excessive_count = (16_384u16 + 1).to_le_bytes();
    bytes[eocd + 8..eocd + 10].copy_from_slice(&excessive_count);
    bytes[eocd + 10..eocd + 12].copy_from_slice(&excessive_count);
    std::fs::write(&archive, bytes).unwrap();
    let mut corpus = corpus_support::open_corpus(&root, std::slice::from_ref(&root));
    // Isolate archive ordering from the separately tested filesystem refusal.
    corpus.root_device = corpus.root_device.wrapping_add(1);
    let output = directory.path().join("result.json");
    let failure =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run_census(corpus, &output)))
            .expect_err("excessive declared entries admitted");
    let message = failure
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| failure.downcast_ref::<&str>().copied())
        .unwrap_or("");
    assert!(
        message.contains("corpus archive contains too many members"),
        "archive was opened before checking its declared entry budget"
    );
    assert!(!output.exists());
}
