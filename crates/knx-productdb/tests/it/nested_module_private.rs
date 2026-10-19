//! Opt-in private nested-package storage evidence, with coarse failure output only.

use std::collections::BTreeMap;
use std::io::Read;

use knx_productdb::{install_package, load_source_file, open_and_migrate, sha256_hex};
use quick_xml::events::Event;
use quick_xml::Reader;

type ScopeCounts = BTreeMap<(String, String), i64>;

fn private_result<T, E>(result: Result<T, E>, message: &'static str) -> T {
    result.unwrap_or_else(|_| panic!("{message}"))
}

fn read_sample(path: &std::ffi::OsStr) -> Option<Vec<u8>> {
    const MAX_BYTES: u64 = 256 * 1024 * 1024;
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(
            (rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::NONBLOCK).bits() as i32,
        );
    }
    let file = options.open(std::path::Path::new(path)).ok()?;
    let metadata = file.metadata().ok()?;
    if !metadata.is_file() || metadata.len() > MAX_BYTES {
        return None;
    }
    let mut bytes = Vec::new();
    file.take(MAX_BYTES + 1).read_to_end(&mut bytes).ok()?;
    (bytes.len() as u64 <= MAX_BYTES).then_some(bytes)
}

/// Independent lexical census of Dynamic elements in retained XML, not an
/// evaluator and not a manufacturer grammar or download compatibility claim.
fn source_scope_counts(bytes: &[u8]) -> Result<(ScopeCounts, bool), ()> {
    let mut reader = Reader::from_reader(bytes);
    let mut buffer = Vec::new();
    let mut program = String::new();
    let mut definitions = Vec::<String>::new();
    let mut dynamic_depth = 0usize;
    let mut nested = false;
    let mut counts = BTreeMap::new();
    loop {
        buffer.clear();
        let event = reader.read_event_into(&mut buffer).map_err(|_| ())?;
        let is_start = matches!(&event, Event::Start(_));
        match event {
            Event::Start(element) | Event::Empty(element) => {
                let name = knx_productdb::xml::local_name(&element);
                if name == "ApplicationProgram" {
                    let attributes =
                        knx_productdb::xml::attrs(&element, "private sample").map_err(|_| ())?;
                    program = attributes.get("Id").unwrap_or_default().to_string();
                    definitions.clear();
                }
                if name == "ModuleDef" && is_start {
                    let attributes =
                        knx_productdb::xml::attrs(&element, "private sample").map_err(|_| ())?;
                    nested |= !definitions.is_empty();
                    definitions.push(attributes.get("Id").unwrap_or_default().to_string());
                }
                if name == "Dynamic" || dynamic_depth > 0 {
                    let scope = definitions.last().cloned().unwrap_or_default();
                    *counts.entry((program.clone(), scope)).or_default() += 1;
                    if is_start {
                        dynamic_depth += 1;
                    }
                }
            }
            Event::End(element) => {
                dynamic_depth = dynamic_depth.saturating_sub(1);
                if element.local_name().as_ref() == "ModuleDef" {
                    definitions.pop();
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok((counts, nested))
}

#[test]
fn lexical_census_counts_nested_empty_and_enclosing_dynamic_scopes() {
    let bytes = br#"<ApplicationProgram Id="program"><Dynamic><Channel/></Dynamic>
        <ModuleDefs><ModuleDef Id="outer"><ModuleDefs><ModuleDef Id="inner">
        <Dynamic><Channel><ParameterBlock/></Channel></Dynamic>
        </ModuleDef><ModuleDef Id="empty"/></ModuleDefs><Dynamic/></ModuleDef>
        </ModuleDefs></ApplicationProgram>"#;
    let (counts, nested) = source_scope_counts(bytes).unwrap();
    assert!(nested);
    assert_eq!(
        counts,
        BTreeMap::from([
            (("program".into(), "".into()), 2),
            (("program".into(), "inner".into()), 3),
            (("program".into(), "outer".into()), 1),
        ])
    );
    assert!(source_scope_counts(b"<ModuleDef></Dynamic>").is_err());
}

#[test]
fn private_fixture_reader_is_bounded_and_refuses_nonregular_inputs() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("sample.knxprod");
    std::fs::write(&source, b"synthetic").unwrap();
    assert_eq!(read_sample(source.as_os_str()), Some(b"synthetic".to_vec()));
    assert!(read_sample(directory.path().as_os_str()).is_none());
    assert!(read_sample(directory.path().join("missing").as_os_str()).is_none());
    let oversized = directory.path().join("oversized");
    std::fs::File::create(&oversized)
        .unwrap()
        .set_len(256 * 1024 * 1024 + 1)
        .unwrap();
    assert!(read_sample(oversized.as_os_str()).is_none());
    #[cfg(unix)]
    {
        let link = directory.path().join("link");
        std::os::unix::fs::symlink(&source, &link).unwrap();
        assert!(read_sample(link.as_os_str()).is_none());
    }
}

#[test]
#[ignore = "requires an explicitly configured private nested package; set KNXBENCH_NESTED_MODULE_PACKAGE"]
fn private_nested_package_stores_each_dynamic_scope() {
    let path = std::env::var_os("KNXBENCH_NESTED_MODULE_PACKAGE")
        .expect("private scope configuration required");
    let bytes = read_sample(&path).expect("private sample read failed");
    let directory = tempfile::tempdir().expect("temporary database unavailable");
    let conn = private_result(
        open_and_migrate(&directory.path().join("products.sqlite")),
        "temporary database initialization failed",
    );
    let report = private_result(
        install_package(&conn, "sample.knxprod", &bytes),
        "private nested package admission failed",
    );
    assert!(
        !report.skipped && !report.members.is_empty(),
        "private install witness missing"
    );
    let mut original_zip = private_result(
        zip::ZipArchive::new(std::io::Cursor::new(&bytes)),
        "private archive witness unavailable",
    );
    for member in &report.members {
        let retained = private_result(
            load_source_file(&conn, &member.sha256),
            "retained private member query failed",
        )
        .expect("retained private member unavailable");
        let mut original = Vec::new();
        let zipped = private_result(
            original_zip.by_name(&member.path),
            "private archive member unavailable",
        );
        private_result(
            zipped.take(member.size + 1).read_to_end(&mut original),
            "private archive member read failed",
        );
        assert!(
            original == retained
                && sha256_hex(&retained) == member.sha256
                && retained.len() as u64 == member.size,
            "private member integrity mismatch"
        );
    }
    let mut statement = private_result(conn.prepare(
        "SELECT DISTINCT s.bytes FROM source_file s JOIN application_program a ON a.source_sha256 = s.sha256"
    ), "private source query unavailable");
    let sources: Vec<Vec<u8>> = private_result(
        private_result(
            statement.query_map([], |r| r.get(0)),
            "private source rows unavailable",
        )
        .collect::<Result<_, _>>(),
        "private source decoding failed",
    );
    assert!(!sources.is_empty(), "private program witness missing");
    let mut expected = ScopeCounts::new();
    let mut nested = false;
    for source in sources {
        let (counts, is_nested) = private_result(
            source_scope_counts(&source),
            "private lexical census failed",
        );
        nested |= is_nested;
        for (key, count) in counts {
            assert!(
                expected.insert(key, count).is_none(),
                "private census scope duplicated"
            );
        }
    }
    assert!(
        nested && !expected.is_empty(),
        "configured private sample has no nested scope witness"
    );
    let mut statement = private_result(conn.prepare(
        "SELECT program_id, module_def_id, count(*) FROM dynamic_node GROUP BY program_id, module_def_id"
    ), "private stored scope query unavailable");
    let actual: ScopeCounts = private_result(
        private_result(
            statement.query_map([], |r| Ok(((r.get(0)?, r.get(1)?), r.get(2)?))),
            "private stored scope rows unavailable",
        )
        .collect::<Result<_, _>>(),
        "private stored scope decoding failed",
    );
    assert!(
        actual == expected,
        "private lexical and stored scope counts differ"
    );
    assert!(
        install_package(&conn, "retry.knxprod", &bytes)
            .ok()
            .is_some_and(|r| r.skipped),
        "private retry failed"
    );
    assert!(
        read_sample(&path).is_some_and(|after| after == bytes),
        "private original changed"
    );
}
