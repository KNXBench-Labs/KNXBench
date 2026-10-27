//! Drives the built `knx` binary to test `doc-export` (T13 —
//! `docs/superpowers/specs/2026-09-10-project-documentation-export-design.md`).
//! Builds a tiny `.knxdb` directly through `knx-core`/`knx-store`, same
//! pattern `cli_group_address_csv.rs` uses. This subcommand exports
//! "project documentation", nothing more, nothing less — it makes no claim
//! of ETS-report parity, since no ETS-produced report sample exists
//! anywhere in this repository to be compatible with.

use std::path::Path;
use std::process::{Command, Output};

use knx_core::{
    CommissioningState, CompletionStatus, DeviceId, DeviceInstance, GroupAddress,
    GroupAddressEntry, GroupRange, GroupRangeId, Installation, InstallationId, Language, Project,
    SourceRef, Topology,
};

fn source(tag: &str) -> SourceRef {
    SourceRef {
        path: tag.into(),
        ets_id: tag.into(),
    }
}

/// A project named "Test Villa" with one installation, one group range
/// spanning the whole address space (so the address never also triggers
/// the unrelated "not inside any group range" warning), one group address
/// inside it (`1/1/1`/"Living Room Light"), and no devices at all — the
/// baseline "nothing structurally odd" fixture (same shape as
/// `apps/knx-server/tests/http_documentation_export.rs`'s
/// `state_with_one_group_address`).
fn tiny_project() -> Project {
    let mut project = Project::new(Language("en".into()));
    project.info.name = "Test Villa".into();
    project.installations.push(Installation {
        id: InstallationId(0),
        name: "I".into(),
        default_line: None,
        multicast_address: None,
        completion: CompletionStatus::FinishedDesign,
        topology: Topology {
            areas: vec![],
            lines: vec![],
            unassigned: vec![],
        },
        buildings: vec![],
        group_ranges: vec![GroupRange {
            id: GroupRangeId(1),
            source: source("gr1"),
            name: "Everything".into(),
            start: GroupAddress::from_raw(0),
            end: GroupAddress::from_raw(u16::MAX),
            parent: None,
            children: vec![],
        }],
        group_addresses: vec![],
        parameters: vec![],
    });

    let id = project.ids.next_group_address_id().unwrap();
    project.installations[0]
        .group_addresses
        .push(GroupAddressEntry {
            id,
            source: source("t1"),
            name: "Living Room Light".into(),
            address: GroupAddress::parse("1/1/1", project.info.group_address_style).unwrap(),
            central: false,
            unfiltered: false,
            range: Some(GroupRangeId(1)),
            declared_dpt: Default::default(),
        });

    project
}

/// Same as [`tiny_project`], plus one device (`DeviceId(1)`) assigned to
/// no line (`Topology::unassigned`) — the structural oddity the
/// "every warning individually" test exercises.
fn project_with_a_device_in_no_line() -> Project {
    let mut project = tiny_project();
    project.installations[0].topology.unassigned = vec![DeviceId(1)];
    project.devices.insert(DeviceInstance {
        id: DeviceId(1),
        source: source("d1"),
        name: "Stray Device".into(),
        description: None,
        address: None,
        product_ref: "P".into(),
        program_ref: "H".into(),
        commissioning: CommissioningState::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
    project
}

/// Writes a fresh `.knxdb` at `path` holding `project`.
fn write_store(path: &Path, project: &Project) {
    let conn = knx_store::open_and_migrate(path).expect("open/migrate store");
    knx_store::save_project(&conn, project).expect("save project");
}

/// Runs `knx` without the developer's own product database: `XDG_DATA_HOME`
/// names a directory no test creates, so `default_path()` finds nothing.
fn run_cli(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_knx"))
        .args(args)
        .env(
            "XDG_DATA_HOME",
            Path::new(env!("CARGO_TARGET_TMPDIR")).join("knx-cli-no-user-data"),
        )
        .output()
        .expect("failed to run the knx binary")
}

#[test]
fn doc_export_writes_a_self_contained_html_document_and_names_output_and_warning_count() {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("project.knxdb");
    let html_path = dir.path().join("documentation.html");
    write_store(&store, &tiny_project());

    let out = run_cli(&[
        "doc-export",
        store.to_str().unwrap(),
        html_path.to_str().unwrap(),
    ]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "doc-export failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let text = std::fs::read_to_string(&html_path).unwrap();
    assert!(text.starts_with("<!DOCTYPE html>"), "{text}");
    assert!(text.contains("Test Villa"), "{text}");
    assert!(text.contains("1/1/1"), "{text}");

    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        stdout.contains(html_path.to_str().unwrap()),
        "stdout must name the output path: {stdout}"
    );
    assert!(
        stdout.contains("0 warning(s)"),
        "stdout must name the warning count: {stdout}"
    );
}

#[test]
fn doc_export_prints_every_warning_individually_not_just_a_count() {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("project.knxdb");
    let html_path = dir.path().join("documentation.html");
    write_store(&store, &project_with_a_device_in_no_line());

    let out = run_cli(&[
        "doc-export",
        store.to_str().unwrap(),
        html_path.to_str().unwrap(),
    ]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "doc-export failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("2 warning(s)"), "{stdout}");
    assert!(
        stdout.contains("device 1"),
        "each warning's location must be printed individually: {stdout}"
    );
    assert!(stdout.contains("product database"), "{stdout}");
}

#[test]
fn doc_export_of_a_missing_store_exits_1_and_writes_no_file() {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("does-not-exist.knxdb");
    let html_path = dir.path().join("documentation.html");

    let out = run_cli(&[
        "doc-export",
        store.to_str().unwrap(),
        html_path.to_str().unwrap(),
    ]);
    assert_eq!(
        out.status.code(),
        Some(1),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        !html_path.exists(),
        "a failed doc-export must not produce an output file"
    );
}

#[test]
fn doc_export_reports_the_actual_default_product_database_open_error() {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("project.knxdb");
    let html_path = dir.path().join("documentation.html");
    let data_home = dir.path().join("data");
    let products_path = data_home.join("knx/products.sqlite");
    write_store(&store, &tiny_project());
    let products = knx_productdb::open_and_migrate(&products_path).unwrap();
    products
        .pragma_update(
            None,
            "user_version",
            knx_productdb::CURRENT_PRODUCTDB_VERSION + 1,
        )
        .unwrap();
    drop(products);

    let out = Command::new(env!("CARGO_BIN_EXE_knx"))
        .args([
            "doc-export",
            store.to_str().unwrap(),
            html_path.to_str().unwrap(),
        ])
        .env("XDG_DATA_HOME", &data_home)
        .env_remove("HOME")
        .output()
        .expect("failed to run the knx binary");

    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(
        html_path.exists(),
        "the report still degrades to raw references"
    );
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.contains("failed to open product database"),
        "{stderr}"
    );
    // Derived from the build's own schema version: hard-coded numbers would
    // pass for the wrong reason after every ProductDB migration.
    let unsupported = knx_productdb::CURRENT_PRODUCTDB_VERSION + 1;
    let supported = knx_productdb::CURRENT_PRODUCTDB_VERSION;
    assert!(
        stderr.contains(&format!("product database is version {unsupported}")),
        "{stderr}"
    );
    assert!(
        stderr.contains(&format!("this build supports up to {supported}")),
        "{stderr}"
    );
}

/// ADR-0090: `doc-export` only reads the user's product database. A newer
/// build migrating it in place locks every older KNXBench out of its own
/// catalogue (it then runs without one), so an older file — here an empty,
/// version-0 one — must keep its bytes and gain no sidecar.
#[test]
fn doc_export_reads_an_older_default_product_database_without_writing_it() {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("project.knxdb");
    let html_path = dir.path().join("documentation.html");
    let data_home = dir.path().join("data");
    let products_dir = data_home.join("knx");
    std::fs::create_dir_all(&products_dir).unwrap();
    std::fs::write(products_dir.join("products.sqlite"), b"").unwrap();
    write_store(&store, &tiny_project());

    let out = Command::new(env!("CARGO_BIN_EXE_knx"))
        .args([
            "doc-export",
            store.to_str().unwrap(),
            html_path.to_str().unwrap(),
        ])
        .env("XDG_DATA_HOME", &data_home)
        .env_remove("HOME")
        .output()
        .expect("failed to run the knx binary");

    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(!stderr.contains("product database"), "{stderr}");
    let after = std::fs::read(products_dir.join("products.sqlite")).unwrap();
    assert!(
        after.is_empty(),
        "a reader migrated the product database: it grew to {} bytes",
        after.len()
    );
    let names: Vec<String> = std::fs::read_dir(&products_dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, vec!["products.sqlite".to_string()], "no sidecar");
}

#[test]
fn doc_export_with_wrong_argument_count_prints_usage_and_exits_1() {
    let out = run_cli(&["doc-export", "only-one-argument"]);
    assert_eq!(
        out.status.code(),
        Some(1),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("usage:"), "{stderr}");
}

/// Replaces the one clock-derived cell ("Generated at" / "Erzeugt am") so two
/// separate process runs can be compared without pretending that two
/// wall-clock reads agree. Nothing else in the document is touched.
fn without_generated_time(html: &str) -> String {
    let mut out = String::new();
    let mut rest = html;
    while let Some(at) = ["Generated at</th><td>", "Erzeugt am</th><td>"]
        .iter()
        .filter_map(|label| rest.find(label).map(|i| (i, label.len())))
        .min()
    {
        let (start, len) = at;
        out.push_str(&rest[..start + len]);
        rest = &rest[start + len..];
        let end = rest.find('<').expect("timestamp cell closes");
        out.push_str("<generated-time>");
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

/// Exports a fresh store with arguments from `build` and asserts its bytes stay unchanged.
/// Returns the process output and generated HTML text.
fn export_with(dir: &Path, build: impl Fn(&str, &str) -> Vec<String>) -> (Output, String) {
    let store = dir.join("project.knxdb");
    let html = dir.join("documentation.html");
    write_store(&store, &tiny_project());
    let before = std::fs::read(&store).unwrap();
    let args = build(store.to_str().unwrap(), html.to_str().unwrap());
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let out = run_cli(&refs);
    assert_eq!(
        std::fs::read(&store).unwrap(),
        before,
        "store bytes changed"
    );
    let text = std::fs::read_to_string(&html).unwrap_or_default();
    (out, text)
}

#[test]
fn doc_export_language_de_and_de_de_render_german_in_any_flag_position() {
    for value in ["de", "de-DE"] {
        for position in 0..3 {
            let dir = tempfile::tempdir().unwrap();
            let (out, text) = export_with(dir.path(), |store, html| {
                let mut args: Vec<String> = vec!["doc-export".into()];
                let mut rest = vec![store.to_string(), html.to_string()];
                rest.insert(position, "--language".into());
                rest.insert(position + 1, value.into());
                args.extend(rest);
                args
            });
            assert_eq!(
                out.status.code(),
                Some(0),
                "{value}@{position}: {}",
                String::from_utf8_lossy(&out.stderr)
            );
            assert!(text.contains("<html lang=\"de\">"), "{value}@{position}");
            assert!(text.contains("Erzeugt am"), "{value}@{position}");
            assert!(!text.contains(">Contents<"), "{value}@{position}");
        }
    }
}

#[test]
fn doc_export_language_en_en_us_and_no_flag_render_the_same_english() {
    let mut documents = Vec::new();
    for flags in [
        &[][..],
        &["--language", "en"][..],
        &["--language", "en-US"][..],
    ] {
        let dir = tempfile::tempdir().unwrap();
        let (out, text) = export_with(dir.path(), |store, html| {
            let mut args = vec!["doc-export".to_string(), store.into(), html.into()];
            args.extend(flags.iter().map(|f| f.to_string()));
            args
        });
        assert_eq!(out.status.code(), Some(0), "{flags:?}");
        assert!(text.contains("<html lang=\"en\">"), "{flags:?}");
        assert!(text.contains("Generated at"), "{flags:?}");
        documents.push(without_generated_time(&text));
    }
    assert_eq!(
        documents[0], documents[1],
        "explicit en differs from default"
    );
    assert_eq!(documents[0], documents[2], "en-US differs from default");
    let german = {
        let dir = tempfile::tempdir().unwrap();
        export_with(dir.path(), |s, h| {
            ["doc-export", s, h, "--language", "de"]
                .map(String::from)
                .to_vec()
        })
        .1
    };
    assert_ne!(documents[0], without_generated_time(&german));
}

#[test]
fn doc_export_refuses_bad_language_options_before_touching_store_or_output() {
    let cases: &[&[&str]] = &[
        &["--language", "fr"],
        &["--language", "DE"],
        &["--language", "en-us"],
        &["--language", "de_DE"],
        &["--language", ""],
        &["--language"],
        &["--language", "--language", "de"],
        &["--language", "en", "--language", "de"],
        &["--language", "de", "--language", "de"],
        &["--lang", "de"],
    ];
    for flags in cases {
        for sentinel in [true, false] {
            let dir = tempfile::tempdir().unwrap();
            let store = dir.path().join("project.knxdb");
            let html = dir.path().join("documentation.html");
            write_store(&store, &tiny_project());
            let store_before = std::fs::read(&store).unwrap();
            if sentinel {
                std::fs::write(&html, b"SENTINEL \xff bytes").unwrap();
            }
            let mut args = vec![
                "doc-export",
                store.to_str().unwrap(),
                html.to_str().unwrap(),
            ];
            args.extend_from_slice(flags);
            let out = run_cli(&args);
            assert_eq!(out.status.code(), Some(1), "{flags:?}");
            assert!(out.stdout.is_empty(), "{flags:?}");
            assert!(
                String::from_utf8_lossy(&out.stderr).contains("usage:"),
                "{flags:?}"
            );
            assert_eq!(std::fs::read(&store).unwrap(), store_before, "{flags:?}");
            if sentinel {
                assert_eq!(
                    std::fs::read(&html).unwrap(),
                    b"SENTINEL \xff bytes",
                    "{flags:?}"
                );
            } else {
                assert!(!html.exists(), "{flags:?} created output");
            }
        }
    }
}

#[test]
fn doc_export_language_error_comes_before_a_missing_store_is_opened() {
    let dir = tempfile::tempdir().unwrap();
    let html = dir.path().join("documentation.html");
    let missing = dir.path().join("nope.knxdb");
    let out = run_cli(&[
        "doc-export",
        missing.to_str().unwrap(),
        html.to_str().unwrap(),
        "--language",
        "fr",
    ]);
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("unsupported report language: fr"),
        "{stderr}"
    );
    assert!(!stderr.contains("failed to open store"), "{stderr}");
    assert!(!missing.exists() && !html.exists());
}
