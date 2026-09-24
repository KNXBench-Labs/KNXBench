//! Builds the explicit, machine-readable product-corpus compatibility matrix.

mod corpus_support;

use std::collections::{BTreeMap, HashSet};
use std::ffi::OsString;
use std::fs;
use std::io::{Cursor, Read, Write};
use std::os::fd::OwnedFd;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use corpus_support::{
    bounded_zip_entry_count, configured_corpus, discover_packages, ConfiguredCorpus,
    CorpusPackageSource, CorpusReadBudget, DiscoveredCorpus,
};
use knx_productdb::{Connection, InstallReport, PackageError, ProductDbError};
use quick_xml::events::Event;
use quick_xml::name::ResolveResult;
use rustix::fs::{
    fstat, openat, openat2, renameat, unlinkat, AtFlags, Mode, OFlags, ResolveFlags, CWD,
};
use serde_json::{json, Value};

const MAX_SCHEME_XML_BYTES: u64 = 64 * 1024 * 1024;
const EXPECTED_PACKAGE_INSTANCES: usize = 115;
const EXPECTED_UNIQUE_PACKAGES: usize = 113;
const EXPECTED_ISOLATED_INSTALLS: usize = 104;
const EXPECTED_SHARED_INSTALLS: usize = 102;
const EXPECTED_SHARED_DEDUPLICATIONS: usize = 2;
const EXPECTED_UNSUPPORTED_NAMESPACES: usize = 11;
const EXPECTED_BASELINE_COMMITMENT: &str =
    "fb6a070e2b02d6ab754fbe6023ba17f76e8bcfdb95d7e09179cdad359f4d3825";
static NEXT_OUTPUT_TEMP: AtomicU64 = AtomicU64::new(0);

fn configured_output() -> PathBuf {
    std::env::var_os("KNXBENCH_PRODUCT_MATRIX_OUTPUT")
        .map(PathBuf::from)
        .expect("SKIP: set KNXBENCH_PRODUCT_MATRIX_OUTPUT for the private matrix JSON")
}

fn package_scheme(bytes: &[u8]) -> Option<u32> {
    bounded_zip_entry_count(bytes);
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).ok()?;
    let master = archive.by_name("knx_master.xml").ok()?;
    if master.size() > MAX_SCHEME_XML_BYTES {
        return None;
    }
    let mut xml = Vec::new();
    master
        .take(MAX_SCHEME_XML_BYTES + 1)
        .read_to_end(&mut xml)
        .ok()?;
    if xml.len() as u64 > MAX_SCHEME_XML_BYTES {
        return None;
    }
    let mut reader = quick_xml::NsReader::from_reader(xml.as_slice());
    loop {
        match reader.read_resolved_event().ok()? {
            (ResolveResult::Bound(namespace), Event::Start(root) | Event::Empty(root))
                if root.local_name().as_ref() == "KNX" =>
            {
                let namespace = namespace.as_ref();
                return namespace
                    .strip_prefix("http://knx.org/xml/project/")?
                    .parse()
                    .ok();
            }
            (_, Event::Start(_) | Event::Empty(_)) | (_, Event::Eof) => return None,
            _ => {}
        }
    }
}

fn report_json(report: InstallReport) -> Value {
    json!({
        "status": if report.skipped { "deduplicated" } else { "installed" },
        "report": {
            "member_count": report.members.len(),
            "unknown_count": report.unknown,
            "conflict_count": report.conflicts.len(),
            "dropped_datapoint_type_count": report.dropped_datapoint_types,
            "translation_counts": {
                "program": report.translations.program,
                "catalog": report.translations.catalog,
                "hardware": report.translations.hardware,
                "master": report.translations.master,
            }
        }
    })
}

fn error_category(error: PackageError) -> &'static str {
    match error {
        PackageError::LegacyVd2 { .. } => "legacy_container",
        PackageError::InvalidZip { .. } => "invalid_zip",
        PackageError::Encrypted { .. } => "encrypted_member",
        PackageError::UnsafeMember { .. } => "unsafe_member",
        PackageError::DuplicateMember { .. } => "duplicate_member",
        PackageError::SizeLimit { .. } => "size_limit",
        PackageError::MissingMaster => "missing_master",
        PackageError::UnsupportedNamespace { .. } => "unsupported_namespace",
        PackageError::ProjectArchive => "project_archive",
        PackageError::MissingManufacturerData => "missing_manufacturer_data",
        PackageError::Database(ProductDbError::Xml { .. }) => "xml",
        PackageError::Database(ProductDbError::Sqlite(_)) => "sqlite",
        PackageError::Database(ProductDbError::FutureVersion { .. }) => "future_database",
    }
}

fn install_json(conn: &Connection, ordinal: usize, bytes: &[u8]) -> Value {
    match knx_productdb::install_package(
        conn,
        &format!("corpus-package-{ordinal:04}.knxprod"),
        bytes,
    ) {
        Ok(report) => report_json(report),
        Err(error) => json!({
            "status": "rejected",
            "category": error_category(error),
        }),
    }
}

fn count_outcomes(records: &[Value], field: &str) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for record in records {
        let status = record[field]["status"]
            .as_str()
            .expect("matrix outcome status")
            .to_string();
        *counts.entry(status.clone()).or_default() += 1;
        if status == "rejected" {
            let category = record[field]["category"]
                .as_str()
                .expect("matrix rejection category");
            *counts.entry(format!("rejected:{category}")).or_default() += 1;
        }
    }
    counts
}

fn report_totals(records: &[Value], field: &str, include_deduplicated: bool) -> Value {
    let mut members = 0_u64;
    let mut unknown = 0_u64;
    let mut conflicts = 0_u64;
    let mut dropped_dpts = 0_u64;
    let mut program = 0_u64;
    let mut catalog = 0_u64;
    let mut hardware = 0_u64;
    let mut master = 0_u64;
    let mut attempts = 0_u64;

    for record in records {
        let outcome = &record[field];
        let status = outcome["status"].as_str().expect("matrix outcome status");
        if status != "installed" && !(include_deduplicated && status == "deduplicated") {
            continue;
        }
        let report = &outcome["report"];
        attempts += 1;
        members += report["member_count"].as_u64().expect("member count");
        unknown += report["unknown_count"].as_u64().expect("unknown count");
        conflicts += report["conflict_count"].as_u64().expect("conflict count");
        dropped_dpts += report["dropped_datapoint_type_count"]
            .as_u64()
            .expect("dropped DPT count");
        program += report["translation_counts"]["program"]
            .as_u64()
            .expect("program translation count");
        catalog += report["translation_counts"]["catalog"]
            .as_u64()
            .expect("catalog translation count");
        hardware += report["translation_counts"]["hardware"]
            .as_u64()
            .expect("hardware translation count");
        master += report["translation_counts"]["master"]
            .as_u64()
            .expect("master translation count");
    }

    json!({
        "attempt_count": attempts,
        "member_count": members,
        "unknown_count": unknown,
        "conflict_count": conflicts,
        "dropped_datapoint_type_count": dropped_dpts,
        "translation_counts": {
            "program": program,
            "catalog": catalog,
            "hardware": hardware,
            "master": master,
        }
    })
}

fn database_counts(conn: &Connection) -> BTreeMap<String, u64> {
    let names = conn
        .prepare(
            "SELECT name FROM sqlite_schema
             WHERE type = 'table' AND name NOT LIKE 'sqlite_%'
             ORDER BY name",
        )
        .expect("read product database schema")
        .query_map([], |row| row.get::<_, String>(0))
        .expect("query product database schema")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect product database schema");
    names
        .into_iter()
        .map(|name| {
            let quoted = name.replace('"', "\"\"");
            let count = conn
                .query_row(&format!("SELECT COUNT(*) FROM \"{quoted}\""), [], |row| {
                    row.get::<_, i64>(0)
                })
                .expect("count product database rows");
            (name, count as u64)
        })
        .collect()
}

struct OutputTarget {
    directory: OwnedFd,
    file_name: OsString,
}

struct PendingOutput<'a> {
    directory: &'a OwnedFd,
    name: OsString,
    published: bool,
}

impl Drop for PendingOutput<'_> {
    fn drop(&mut self) {
        if !self.published {
            let _ = unlinkat(self.directory, &self.name, AtFlags::empty());
        }
    }
}

fn validate_output_path(corpus_root: &Path, corpus_device: u64, path: &Path) -> OutputTarget {
    let file_name = path
        .file_name()
        .filter(|name| !name.is_empty())
        .expect("matrix output must name a file");
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let canonical_parent = parent
        .canonicalize()
        .expect("matrix output directory must already exist");
    assert!(
        !canonical_parent.starts_with(corpus_root),
        "matrix output must be outside the configured corpus root"
    );
    let directory = openat2(
        CWD,
        &canonical_parent,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
        ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
    )
    .expect("open matrix output directory without following symlinks");
    let output_device = fstat(&directory)
        .expect("read matrix output directory metadata")
        .st_dev;
    assert_ne!(
        output_device, corpus_device,
        "matrix output directory must be on a different filesystem from the corpus"
    );
    OutputTarget {
        directory,
        file_name: file_name.to_os_string(),
    }
}

fn write_atomic(target: &OutputTarget, value: &Value) {
    let (fd, temp_name) = (0..128)
        .find_map(|_| {
            let sequence = NEXT_OUTPUT_TEMP.fetch_add(1, Ordering::Relaxed);
            let name = OsString::from(format!(
                ".knxbench-matrix-{}-{sequence}.tmp",
                std::process::id()
            ));
            match openat(
                &target.directory,
                &name,
                OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::CLOEXEC,
                Mode::RUSR | Mode::WUSR,
            ) {
                Ok(fd) => Some((fd, name)),
                Err(rustix::io::Errno::EXIST) => None,
                Err(error) => panic!("create matrix output file: {error}"),
            }
        })
        .expect("create a unique matrix output file");
    let mut pending = PendingOutput {
        directory: &target.directory,
        name: temp_name,
        published: false,
    };
    let mut output = fs::File::from(fd);
    serde_json::to_writer_pretty(&mut output, value).expect("serialize compatibility matrix");
    output
        .write_all(b"\n")
        .expect("finish compatibility matrix");
    output.flush().expect("flush compatibility matrix");
    renameat(
        &target.directory,
        &pending.name,
        &target.directory,
        &target.file_name,
    )
    .expect("publish compatibility matrix atomically");
    pending.published = true;
}

fn remove_previous_output(target: &OutputTarget) {
    match unlinkat(&target.directory, &target.file_name, AtFlags::empty()) {
        Ok(()) | Err(rustix::io::Errno::NOENT) => {}
        Err(error) => panic!("remove previous matrix output: {error}"),
    }
}

fn discover_after_output_cleanup(
    corpus: ConfiguredCorpus,
    output: OutputTarget,
) -> (DiscoveredCorpus, OutputTarget) {
    remove_previous_output(&output);
    let discovered = corpus.discover();
    (discovered, output)
}

fn ordered_sources_by_hash(
    sources: Vec<CorpusPackageSource>,
) -> Vec<(String, CorpusPackageSource)> {
    let mut budget = CorpusReadBudget::new();
    let mut ordered = sources
        .into_iter()
        .enumerate()
        .map(|(discovery_index, source)| {
            let sha256 = knx_productdb::sha256_hex(&source.load(&mut budget));
            (sha256, discovery_index, source)
        })
        .collect::<Vec<_>>();
    ordered.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));
    ordered
        .into_iter()
        .map(|(sha256, _, source)| (sha256, source))
        .collect()
}

fn public_outcome(outcome: &Value) -> Value {
    let status = outcome["status"].as_str().expect("matrix outcome status");
    match status {
        "rejected" => json!({
            "status": "rejected",
            "category": outcome["category"],
        }),
        _ => json!({"status": status}),
    }
}

fn committed_outcome(outcome: &Value) -> Value {
    match outcome["status"].as_str().expect("matrix outcome status") {
        "rejected" => public_outcome(outcome),
        _ => outcome.clone(),
    }
}

fn baseline_commitment(private_records: &[Value], mut projection: Value) -> String {
    let per_hash_outcomes = private_records
        .iter()
        .map(|record| {
            json!({
                "sha256": record["sha256"],
                "scheme": record["scheme"],
                "isolation": committed_outcome(&record["isolation"]),
                "shared": committed_outcome(&record["shared"]),
            })
        })
        .collect::<Vec<_>>();
    projection["per_hash_outcomes"] = json!(per_hash_outcomes);
    knx_productdb::sha256_hex(
        &serde_json::to_vec(&projection).expect("serialize private baseline projection"),
    )
}

#[test]
#[ignore = "requires explicit KNXBENCH_PRODUCT_CORPUS, KNXBENCH_PRODUCT_CORPUS_SCOPES and KNXBENCH_PRODUCT_MATRIX_OUTPUT"]
fn product_corpus_is_measured_in_isolation_and_shared_order() {
    let corpus = configured_corpus();
    let output = validate_output_path(
        &corpus.canonical_root,
        corpus.root_device,
        &configured_output(),
    );
    let (discovered, output) = discover_after_output_cleanup(corpus, output);
    assert_eq!(
        discovered.packages.len(),
        EXPECTED_PACKAGE_INSTANCES,
        "product corpus instance count changed before installation"
    );
    let ordered_sources = ordered_sources_by_hash(discovered.packages);
    let shared_dir = tempfile::tempdir().expect("temporary shared database directory");
    let shared = knx_productdb::open_and_migrate(&shared_dir.path().join("products.sqlite"))
        .expect("temporary shared product database");
    let mut private_records = Vec::with_capacity(ordered_sources.len());
    let mut hashes = HashSet::new();
    let mut install_budget = CorpusReadBudget::new();

    for (index, (sha256, source)) in ordered_sources.into_iter().enumerate() {
        let ordinal = index + 1;
        let bytes = source.load(&mut install_budget);
        assert!(
            knx_productdb::sha256_hex(&bytes) == sha256,
            "corpus package changed between ordering and installation"
        );
        hashes.insert(sha256.clone());

        let (isolation, shared_outcome) = std::thread::scope(|scope| {
            let isolated = scope.spawn(|| {
                let isolated_dir =
                    tempfile::tempdir().expect("temporary isolated database directory");
                let isolated =
                    knx_productdb::open_and_migrate(&isolated_dir.path().join("products.sqlite"))
                        .expect("temporary isolated product database");
                install_json(&isolated, ordinal, &bytes)
            });
            let shared_outcome = install_json(&shared, ordinal, &bytes);
            (
                isolated.join().expect("isolated installation worker"),
                shared_outcome,
            )
        });
        private_records.push(json!({
            "ordinal": ordinal,
            "sha256": sha256,
            "scheme": package_scheme(&bytes),
            "isolation": isolation,
            "shared": shared_outcome,
        }));
    }

    let isolation_counts = count_outcomes(&private_records, "isolation");
    let shared_counts = count_outcomes(&private_records, "shared");
    let mut scheme_counts: BTreeMap<u32, usize> = BTreeMap::new();
    for record in &private_records {
        let scheme = record["scheme"].as_u64().expect("package scheme") as u32;
        *scheme_counts.entry(scheme).or_default() += 1;
    }
    let isolation_totals = report_totals(&private_records, "isolation", false);
    let shared_installed_totals = report_totals(&private_records, "shared", false);
    let shared_successful_totals = report_totals(&private_records, "shared", true);
    let final_counts = database_counts(&shared);
    let commitment = baseline_commitment(
        &private_records,
        json!({
            "schemes": scheme_counts,
            "isolation_outcomes": isolation_counts,
            "shared_outcomes": shared_counts,
            "isolation_report_totals": isolation_totals,
            "shared_installed_report_totals": shared_installed_totals,
            "shared_successful_attempt_report_totals": shared_successful_totals,
            "shared_final_database_counts": final_counts,
        }),
    );
    let public_records = private_records
        .iter()
        .map(|record| {
            json!({
                "ordinal": record["ordinal"],
                "scheme": record["scheme"],
                "isolation": public_outcome(&record["isolation"]),
                "shared": public_outcome(&record["shared"]),
            })
        })
        .collect::<Vec<_>>();
    let matrix = json!({
        "format": "knxbench-product-corpus-matrix-v1",
        "package_instances": public_records.len(),
        "unique_package_hashes": hashes.len(),
        "aggregate_identity_and_outcome_commitment": commitment,
        "scheme_counts": scheme_counts,
        "isolation_outcomes": isolation_counts,
        "shared_outcomes": shared_counts,
        "isolation_report_totals": isolation_totals,
        "shared_installed_report_totals": shared_installed_totals,
        "shared_successful_attempt_report_totals": shared_successful_totals,
        "shared_final_database_counts": final_counts,
        "packages": public_records,
    });

    assert_eq!(matrix["package_instances"], EXPECTED_PACKAGE_INSTANCES);
    assert_eq!(matrix["unique_package_hashes"], EXPECTED_UNIQUE_PACKAGES);
    assert_eq!(
        matrix["scheme_counts"],
        json!({"11": 48, "12": 1, "13": 4, "14": 3, "20": 56, "21": 3}),
        "product corpus scheme distribution changed; matrix output was not published"
    );
    assert_eq!(
        matrix["isolation_outcomes"],
        json!({
            "installed": EXPECTED_ISOLATED_INSTALLS,
            "rejected": EXPECTED_UNSUPPORTED_NAMESPACES,
            "rejected:unsupported_namespace": EXPECTED_UNSUPPORTED_NAMESPACES,
        })
    );
    assert_eq!(
        matrix["shared_outcomes"],
        json!({
            "deduplicated": EXPECTED_SHARED_DEDUPLICATIONS,
            "installed": EXPECTED_SHARED_INSTALLS,
            "rejected": EXPECTED_UNSUPPORTED_NAMESPACES,
            "rejected:unsupported_namespace": EXPECTED_UNSUPPORTED_NAMESPACES,
        })
    );
    assert_eq!(
        matrix["isolation_report_totals"],
        json!({
            "attempt_count": 104,
            "member_count": 1539,
            "unknown_count": 22404,
            "conflict_count": 0,
            "dropped_datapoint_type_count": 0,
            "translation_counts": {"program": 2864784, "catalog": 2821, "hardware": 1368, "master": 103312},
        })
    );
    assert_eq!(
        matrix["shared_installed_report_totals"],
        json!({
            "attempt_count": 102,
            "member_count": 1519,
            "unknown_count": 22279,
            "conflict_count": 371,
            "dropped_datapoint_type_count": 35729,
            "translation_counts": {"program": 2740855, "catalog": 2229, "hardware": 1112, "master": 1636},
        })
    );
    assert_eq!(
        matrix["shared_successful_attempt_report_totals"],
        json!({
            "attempt_count": 104,
            "member_count": 1539,
            "unknown_count": 22404,
            "conflict_count": 373,
            "dropped_datapoint_type_count": 36462,
            "translation_counts": {"program": 2751044, "catalog": 2295, "hardware": 1126, "master": 1636},
        })
    );
    assert_eq!(
        matrix["shared_final_database_counts"]["package"], 102,
        "shared package rows changed"
    );
    assert_eq!(
        matrix["shared_final_database_counts"]["source_file"], 1101,
        "shared source-file rows changed"
    );
    assert_eq!(
        matrix["shared_final_database_counts"]["parameter"], 207711,
        "shared parameter rows changed"
    );
    assert_eq!(
        matrix["shared_final_database_counts"]["translation"], 2745832,
        "shared translation rows changed"
    );
    assert_eq!(
        matrix["aggregate_identity_and_outcome_commitment"],
        EXPECTED_BASELINE_COMMITMENT,
        "product corpus identity, outcomes, reports, or final database counts changed; matrix output was not published"
    );
    write_atomic(&output, &matrix);
    eprintln!(
        "wrote compatibility outcomes for {} package instances",
        EXPECTED_PACKAGE_INSTANCES
    );
}

#[test]
fn report_totals_distinguish_installs_from_deduplicated_attempts() {
    let records = vec![
        json!({"shared": {"status": "installed", "report": {
            "member_count": 2, "unknown_count": 3, "conflict_count": 4,
            "dropped_datapoint_type_count": 5,
            "translation_counts": {"program": 6, "catalog": 7, "hardware": 8, "master": 9}
        }}}),
        json!({"shared": {"status": "deduplicated", "report": {
            "member_count": 10, "unknown_count": 11, "conflict_count": 12,
            "dropped_datapoint_type_count": 13,
            "translation_counts": {"program": 14, "catalog": 15, "hardware": 16, "master": 17}
        }}}),
    ];
    assert_eq!(report_totals(&records, "shared", false)["attempt_count"], 1);
    assert_eq!(report_totals(&records, "shared", false)["member_count"], 2);
    assert_eq!(report_totals(&records, "shared", true)["attempt_count"], 2);
    assert_eq!(report_totals(&records, "shared", true)["member_count"], 12);
}

#[test]
fn public_outcomes_do_not_expose_per_package_report_fingerprints() {
    let outcome = json!({
        "status": "installed",
        "report": {
            "member_count": 17,
            "unknown_count": 23,
            "translation_counts": {"program": 42}
        }
    });
    assert_eq!(public_outcome(&outcome), json!({"status": "installed"}));
}

#[test]
fn matrix_output_is_refused_inside_the_corpus_root() {
    let corpus = tempfile::tempdir().expect("corpus directory");
    let result = std::panic::catch_unwind(|| {
        validate_output_path(corpus.path(), u64::MAX, &corpus.path().join("matrix.json"));
    });
    assert!(result.is_err());
}

#[test]
fn zip_entry_budget_is_checked_before_constructing_an_archive() {
    let mut eocd = vec![0_u8; 22];
    eocd[0..4].copy_from_slice(b"PK\x05\x06");
    let oversized = 16_385_u16.to_le_bytes();
    eocd[8..10].copy_from_slice(&oversized);
    eocd[10..12].copy_from_slice(&oversized);
    let result = std::panic::catch_unwind(|| bounded_zip_entry_count(&eocd));
    assert!(result.is_err());
}

#[test]
fn zip_entry_budget_rejects_fallback_eocd_outside_the_comment_window() {
    let mut bytes = vec![0_u8; 70_000];
    bytes[0..4].copy_from_slice(b"PK\x05\x06");
    bytes.extend_from_slice(b"PK\x05\x06");
    bytes.extend_from_slice(&[0_u8; 18]);

    let result = std::panic::catch_unwind(|| bounded_zip_entry_count(&bytes));
    assert!(result.is_err());
}

#[test]
fn discovery_failure_cannot_leave_a_previous_matrix() {
    let corpus = tempfile::tempdir().expect("corpus directory");
    let selected = corpus.path().join("selected");
    fs::create_dir_all(&selected).unwrap();
    fs::write(selected.join("broken.zip"), b"not a ZIP").unwrap();
    let corpus = corpus_support::open_corpus(corpus.path(), std::slice::from_ref(&selected));

    let output_dir = tempfile::tempdir().expect("output directory");
    let output_path = output_dir.path().join("matrix.json");
    fs::write(&output_path, b"stale matrix").unwrap();
    let output = validate_output_path(&corpus.canonical_root, u64::MAX, &output_path);

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        discover_after_output_cleanup(corpus, output)
    }));
    assert!(result.is_err());
    assert!(!output_path.exists());
}

#[test]
fn scope_resolution_failure_cannot_leave_a_previous_matrix() {
    let corpus_root = tempfile::tempdir().expect("corpus directory");
    let missing_scope = corpus_root.path().join("missing");
    let corpus = corpus_support::open_corpus(corpus_root.path(), &[missing_scope]);

    let output_dir = tempfile::tempdir().expect("output directory");
    let output_path = output_dir.path().join("matrix.json");
    fs::write(&output_path, b"stale matrix").unwrap();
    let output = validate_output_path(&corpus.canonical_root, u64::MAX, &output_path);

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        discover_after_output_cleanup(corpus, output)
    }));
    assert!(result.is_err());
    assert!(!output_path.exists());
}

#[test]
fn explicit_scopes_exclude_unselected_siblings_and_reject_symlinks() {
    let corpus = tempfile::tempdir().expect("corpus directory");
    let selected = corpus.path().join("selected");
    let other = corpus.path().join("other");
    fs::create_dir_all(&selected).unwrap();
    fs::create_dir_all(&other).unwrap();
    fs::write(selected.join("one.knxprod"), b"one").unwrap();
    fs::write(other.join("two.knxprod"), b"two").unwrap();
    let packages = discover_packages(corpus.path(), std::slice::from_ref(&selected));
    assert_eq!(packages.len(), 1);

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        symlink(&other, selected.join("escape")).unwrap();
        let result = std::panic::catch_unwind(|| {
            discover_packages(corpus.path(), std::slice::from_ref(&selected));
        });
        assert!(result.is_err());
    }
}
