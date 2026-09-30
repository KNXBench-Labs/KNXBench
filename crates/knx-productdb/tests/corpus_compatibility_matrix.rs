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
const EXPECTED_ISOLATED_INSTALLS: usize = 115;
const EXPECTED_SHARED_INSTALLS: usize = 113;
const EXPECTED_SHARED_DEDUPLICATIONS: usize = 2;
/// Re-pinned for PDB-8 (schema v14): a main-vs-branch run of this gate
/// differed in exactly two of 31 final table counts, both pinned below —
/// `package_install_count` 3,277 -> 3,390 (one `master_subtree` row per
/// installed package) and `package_install_diagnostic` 645 -> 880 (the new
/// subtree diagnostics). Every install outcome and report total was equal.
///
/// Re-pinned for PDB-9 (schema v15): a main-vs-branch run differed in
/// exactly two of 31 final table counts — `ingest_unknown` 23,040 -> 23,051
/// and `package_install_unknown` 9,245 -> 9,251 — because the 75 v14
/// `Element TypeColor`/`TypeTime` rows became 86 unread-attribute rows
/// (per package, distinct: 26 -> 32). An independent Python recount of the
/// same 115 package instances predicts both deltas exactly. No product
/// table changed; the report totals are pinned above.
///
/// Re-pinned for PDB-10 (schema v16): a main-vs-branch run added the three
/// baggage inventory tables (113 / 789 / 776 rows, pinned below) and
/// changed exactly one existing count, `package_install_diagnostic`
/// 880 -> 859: the 34 `unsupported-baggage-index` rows (one per non-empty
/// index; 3 of the 37 indexes declare nothing) gave way to 13
/// `undeclared-baggage-payload` rows and no unresolved declaration. Every
/// outcome and report total was equal. An independent Python recount of the
/// same 115 instances / 113 distinct packages predicts every number.
///
/// Re-pinned for PDB-11 (schema v17): the four identity tables join the
/// final counts and the identity section joins the projection. The v16
/// projection, recomputed without either, must still equal the PDB-10 pin
/// (`EXPECTED_V16_PROJECTION_COMMITMENT`): no install outcome, report total
/// or pre-existing table count moved. The measured run did so; the new
/// tables hold 115 / 1,972 / 528 / 629 rows (pinned below), and per kind the
/// ids recorded in several blobs equal an independent Python recount of the
/// same packages exactly.
/// Re-pinned for ADR-0052 (schema v18): `Channel/@Number` is now a typed
/// attribute. A main-vs-branch matrix comparison found identical public
/// outcomes and every other table/report total unchanged; isolated unknown
/// count 22,769 -> 22,488, shared installed 22,653 -> 22,373,
/// `ingest_unknown` 23,051 -> 22,771, `package_install_unknown` 9,251 ->
/// 9,156. An independent 115-instance / 113-unique-package XML recount
/// predicts the 281 / 280 / 280 / 95 deltas: 274 distinct (blob, xpath)
/// keys plus six duplicate rows from shared blobs. The v16-shaped projection
/// pin below is the *current v18* pin, not the historical PDB-10 pin.
const EXPECTED_BASELINE_COMMITMENT: &str =
    "8bcacd20400d7b874206cafac848f0d16640880682f27694a473f3781a4c46b1";
/// Current v18 outcomes/counts projected without the four v17 tables and
/// PDB-11 identity; historical v16 pin: c204acc8… (see Git history).
const EXPECTED_V16_PROJECTION_COMMITMENT: &str =
    "a5e4e14711098e04ff9712576f7f6b68a9244d2121a3dca0ac47137e9a196b3a";
/// Tables schema v17 added (ADR-0043), left out of the v16 projection.
const V17_TABLES: [&str; 4] = [
    "package_source_name",
    "source_identity",
    "source_identity_scan",
    "source_producer",
];
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

fn assert_new_scheme_persistence(conn: &Connection, outcome: &Value, scheme: u32) {
    assert_eq!(outcome["status"], "installed");
    let member_count = i64::try_from(
        outcome["report"]["member_count"]
            .as_u64()
            .expect("new-scheme member count"),
    )
    .expect("new-scheme member count fits SQLite INTEGER");
    assert_eq!(
        conn.query_row("SELECT count(*) FROM package", [], |row| {
            row.get::<_, i64>(0)
        })
        .expect("count isolated new-scheme package rows"),
        1
    );
    assert_eq!(
        conn.query_row("SELECT count(*) FROM package_member", [], |row| {
            row.get::<_, i64>(0)
        })
        .expect("count isolated new-scheme package members"),
        member_count
    );
    let mut required_tables = vec![
        "source_file",
        "manufacturer",
        "product",
        "application_program",
        "parameter",
        "dynamic_node",
        "package_install_report",
        "package_install_count",
        "package_install_unknown",
    ];
    if scheme == 13 {
        required_tables.extend(["com_object", "datapoint_type"]);
    }
    for table in required_tables {
        let quoted = table.replace('"', "\"\"");
        let rows = conn
            .query_row(&format!("SELECT count(*) FROM \"{quoted}\""), [], |row| {
                row.get::<_, i64>(0)
            })
            .expect("count isolated new-scheme persistence evidence");
        assert!(
            rows > 0,
            "scheme-{scheme} install did not persist required {table} evidence"
        );
    }
}

fn pdb5_feature_occurrences(conn: &Connection) -> Value {
    let count = |kind: &str, name: &str, branch: &str, path_suffix: &str| {
        let branch_path = format!(
            "/KNX/ManufacturerData/Manufacturer/ApplicationPrograms/ApplicationProgram/{branch}"
        );
        let direct_path = format!("{branch_path}{path_suffix}");
        let nested_pattern = format!("{branch_path}/*{path_suffix}");
        let value = conn
            .query_row(
                "SELECT coalesce(sum(occurrences), 0)
                 FROM package_install_unknown
                 WHERE kind = ?1 AND name = ?2
                   AND xpath NOT GLOB '*:*'
                   AND (xpath = ?3 OR xpath GLOB ?4)",
                [kind, name, direct_path.as_str(), nested_pattern.as_str()],
                |row| row.get::<_, i64>(0),
            )
            .expect("count isolated PDB-5 evidence");
        u64::try_from(value).expect("PDB-5 evidence count is non-negative")
    };
    let separator_count = |name: &str| count("Attribute", name, "Dynamic", "/ParameterSeparator");
    json!({
        "applies_to": count("Attribute", "AppliesTo", "Static", "/LdCtrlWriteProp"),
        "occurrence": count("Attribute", "Occurrence", "Static", "/Property"),
        "horizontal_ruler": separator_count("HorizontalRuler"),
        "separator_access": separator_count("Access"),
        "separator_ui_hint": separator_count("UIHint"),
        "ld_ctrl_declare_prop_desc": count("Element", "LdCtrlDeclarePropDesc", "Static", ""),
    })
}

fn pdb6_feature_occurrences(conn: &Connection) -> Value {
    let count = |kind: &str, name: &str, xpath: &str| {
        let value: i64 = conn
            .query_row(
                "SELECT coalesce(sum(occurrences), 0) FROM package_install_unknown
             WHERE kind = ?1 AND name = ?2 AND xpath = ?3",
                [kind, name, xpath],
                |row| row.get(0),
            )
            .expect("count isolated PDB-6 evidence");
        u64::try_from(value).expect("PDB-6 evidence count is non-negative")
    };
    let master = "/KNX/MasterData";
    let program = "/KNX/ManufacturerData/Manufacturer/ApplicationPrograms/ApplicationProgram";
    let hardware =
        "/KNX/ManufacturerData/Manufacturer/Hardware/Hardware/Hardware2Programs/Hardware2Program";
    let datatype = format!("{master}/DatapointTypes/DatapointType");
    let string = format!("{datatype}/DatapointSubtypes/DatapointSubtype/Format/String");
    let resource =
        format!("{master}/MaskVersions/MaskVersion/HawkConfigurationData/Resources/Resource");
    let property = format!("{master}/InterfaceObjectProperties/InterfaceObjectProperty");
    json!({
        "hardware_type": count("Attribute", "HardwareType", program),
        "datapoint_variable_length": count("Attribute", "VariableLength", &datatype),
        "string_null_terminated": count("Attribute", "NullTerminated", &string),
        "string_variable_length": count("Attribute", "VariableLength", &string),
        "optional_resource": count("Attribute", "Optional", &resource),
        "access_policy": count("Attribute", "AccessPolicy", &property),
        "coupler_capabilities": count("Attribute", "CouplerCapabilities", hardware),
        "rf_rx_capabilities": count("Attribute", "RFRxCapabilities", hardware),
        "rf_tx_capabilities": count("Attribute", "RFTxCapabilities", hardware),
        "property_description": count("Element", "LdCtrlDeclarePropDesc", &format!("{program}/Static/LoadProcedures/LoadProcedure")),
    })
}

/// Count winning catalogue rows, never private values or source identity.
fn pdb7_catalogue_presence(conn: &Connection) -> BTreeMap<&'static str, u64> {
    let mut counts = BTreeMap::new();
    for column in [
        "is_secure_enabled",
        "max_security_group_key_table_entries",
        "max_security_individual_address_entries",
        "max_security_p2p_key_table_entries",
        "max_tunneling_user_entries",
        "max_user_entries",
        "min_ets_version",
        "replaces_versions",
    ] {
        let count: i64 = conn
            .query_row(
                &format!("SELECT count(*) FROM application_program WHERE {column} IS NOT NULL"),
                [],
                |row| row.get(0),
            )
            .expect("count persisted catalogue metadata");
        counts.insert(
            column,
            u64::try_from(count).expect("nonnegative catalogue count"),
        );
    }
    counts
}

#[test]
fn pdb5_feature_counts_accept_direct_and_nested_canonical_paths_only() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "CREATE TABLE package_install_unknown (
            xpath TEXT NOT NULL,
            kind TEXT NOT NULL,
            name TEXT NOT NULL,
            occurrences INTEGER NOT NULL
        );
        INSERT INTO package_install_unknown VALUES
          ('/KNX/ManufacturerData/Manufacturer/ApplicationPrograms/ApplicationProgram/Dynamic/ParameterSeparator', 'Attribute', 'Access', 2),
          ('/KNX/ManufacturerData/Manufacturer/ApplicationPrograms/ApplicationProgram/Dynamic/ParameterBlock/ParameterSeparator', 'Attribute', 'Access', 3),
          ('/KNX/ManufacturerData/Manufacturer/ApplicationPrograms/ApplicationProgram/Static/Property', 'Attribute', 'Occurrence', 5),
          ('/KNX/ManufacturerData/Manufacturer/ApplicationPrograms/ApplicationProgram/Static/Block/Property', 'Attribute', 'Occurrence', 7),
          ('/Other/Dynamic/ParameterSeparator', 'Attribute', 'Access', 100),
          ('/KNX/ManufacturerData/Manufacturer/ApplicationPrograms/ApplicationProgram/Dynamic/e:ParameterSeparator', 'Attribute', 'Access', 100),
          ('/KNX/ManufacturerData/Manufacturer/ApplicationPrograms/ApplicationProgram/Dynamic/e:ParameterBlock/ParameterSeparator', 'Attribute', 'Access', 100);",
    )
    .unwrap();

    let counts = pdb5_feature_occurrences(&conn);
    assert_eq!(counts["separator_access"], 5);
    assert_eq!(counts["occurrence"], 12);
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

/// PDB-11 aggregates over the shared database: per identity kind the
/// candidate rows, distinct ids, ids recorded in more than one blob, and ids
/// whose recorded elements do not all share one digest; plus the scan
/// status counts. Counts only, never an id or a hash.
fn pdb11_identity_aggregates(conn: &Connection) -> Value {
    let mut kinds = serde_json::Map::new();
    for kind in knx_productdb::IdentityKind::ALL {
        let (rows, ids, multi_blob, differing): (i64, i64, i64, i64) = conn
            .query_row(
                "SELECT COALESCE(sum(n), 0), count(*), COALESCE(sum(blobs > 1), 0), COALESCE(sum(digests > 1), 0)
                 FROM (SELECT count(*) AS n, count(DISTINCT source_sha256) AS blobs,
                              count(DISTINCT digest) AS digests
                       FROM source_identity WHERE table_name = ?1 GROUP BY logical_id)",
                [kind.as_table()],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .expect("aggregate identity rows");
        kinds.insert(
            kind.as_table().to_string(),
            json!({
                "candidate_rows": rows,
                "distinct_ids": ids,
                "ids_in_multiple_blobs": multi_blob,
                "ids_with_differing_digests": differing,
            }),
        );
    }
    let divergences = knx_productdb::identity_divergences(conn)
        .expect("identity divergence query over the shared database");
    let mut scans = serde_json::Map::new();
    for status in ["measured", "unavailable"] {
        let count: i64 = conn
            .query_row(
                "SELECT count(*) FROM source_identity_scan WHERE status = ?1",
                [status],
                |r| r.get(0),
            )
            .expect("count identity scans");
        scans.insert(status.to_string(), json!(count));
    }
    json!({
        "kinds": kinds,
        "divergent_ids": divergences.len(),
        "scan_status": scans,
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
        let scheme = package_scheme(&bytes);
        assert!(
            knx_productdb::sha256_hex(&bytes) == sha256,
            "corpus package changed between ordering and installation"
        );
        hashes.insert(sha256.clone());

        let (isolation, pdb5_evidence, pdb6_evidence, pdb7_presence, shared_outcome) =
            std::thread::scope(|scope| {
                let isolated = scope.spawn(|| {
                    let isolated_dir =
                        tempfile::tempdir().expect("temporary isolated database directory");
                    let isolated = knx_productdb::open_and_migrate(
                        &isolated_dir.path().join("products.sqlite"),
                    )
                    .expect("temporary isolated product database");
                    let outcome = install_json(&isolated, ordinal, &bytes);
                    if matches!(scheme, Some(12..=14)) {
                        assert_new_scheme_persistence(
                            &isolated,
                            &outcome,
                            scheme.expect("checked new scheme"),
                        );
                    }
                    let evidence = if matches!(scheme, Some(12 | 14)) {
                        pdb5_feature_occurrences(&isolated)
                    } else {
                        Value::Null
                    };
                    let scheme21_evidence = if scheme == Some(21) {
                        assert_eq!(outcome["status"], "installed");
                        pdb6_feature_occurrences(&isolated)
                    } else {
                        Value::Null
                    };
                    let catalogue = pdb7_catalogue_presence(&isolated);
                    (outcome, evidence, scheme21_evidence, catalogue)
                });
                let shared_outcome = install_json(&shared, ordinal, &bytes);
                let (isolation, pdb5_evidence, pdb6_evidence, pdb7_presence) =
                    isolated.join().expect("isolated installation worker");
                (
                    isolation,
                    pdb5_evidence,
                    pdb6_evidence,
                    pdb7_presence,
                    shared_outcome,
                )
            });
        private_records.push(json!({
            "ordinal": ordinal,
            "sha256": sha256,
            "scheme": scheme,
            "isolation": isolation,
            "pdb5_evidence": pdb5_evidence,
            "pdb6_evidence": pdb6_evidence,
            "pdb7_presence": pdb7_presence,
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
    let shared_pdb7_presence = pdb7_catalogue_presence(&shared);
    let mut pdb5_evidence = BTreeMap::new();
    for name in [
        "applies_to",
        "occurrence",
        "horizontal_ruler",
        "separator_access",
        "separator_ui_hint",
        "ld_ctrl_declare_prop_desc",
    ] {
        let occurrences = private_records
            .iter()
            .filter_map(|record| record["pdb5_evidence"][name].as_u64())
            .sum::<u64>();
        pdb5_evidence.insert(name, occurrences);
    }
    let mut pdb6_evidence = BTreeMap::new();
    for name in [
        "hardware_type",
        "datapoint_variable_length",
        "string_null_terminated",
        "string_variable_length",
        "optional_resource",
        "access_policy",
        "coupler_capabilities",
        "rf_rx_capabilities",
        "rf_tx_capabilities",
        "property_description",
    ] {
        let occurrences = private_records
            .iter()
            .filter_map(|record| record["pdb6_evidence"][name].as_u64())
            .sum::<u64>();
        pdb6_evidence.insert(name, occurrences);
    }
    let mut isolated_pdb7_presence = BTreeMap::new();
    for column in shared_pdb7_presence.keys() {
        let occurrences = private_records
            .iter()
            .filter_map(|record| record["pdb7_presence"][column].as_u64())
            .sum::<u64>();
        isolated_pdb7_presence.insert(*column, occurrences);
    }
    let pdb11_identity = pdb11_identity_aggregates(&shared);
    let mut projection = json!({
        "schemes": scheme_counts,
        "isolation_outcomes": isolation_counts,
        "shared_outcomes": shared_counts,
        "isolation_report_totals": isolation_totals,
        "shared_installed_report_totals": shared_installed_totals,
        "shared_successful_attempt_report_totals": shared_successful_totals,
        "shared_final_database_counts": final_counts,
        "scheme_12_14_feature_occurrences": pdb5_evidence,
        "scheme_21_feature_occurrences": pdb6_evidence,
        "isolated_catalogue_metadata_presence": isolated_pdb7_presence,
        "shared_catalogue_metadata_presence": shared_pdb7_presence,
    });
    let v16_projection = {
        let mut v16 = projection.clone();
        let counts = v16["shared_final_database_counts"]
            .as_object_mut()
            .expect("final counts object");
        for table in V17_TABLES {
            assert!(
                counts.remove(table).is_some(),
                "v17 table {table} not counted"
            );
        }
        v16
    };
    let v16_commitment = baseline_commitment(&private_records, v16_projection);
    projection["pdb11_identity"] = pdb11_identity.clone();
    let commitment = baseline_commitment(&private_records, projection);
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
        "scheme_12_14_feature_occurrences": pdb5_evidence,
        "scheme_21_feature_occurrences": pdb6_evidence,
        "isolated_catalogue_metadata_presence": isolated_pdb7_presence,
        "shared_catalogue_metadata_presence": shared_pdb7_presence,
        "pdb11_identity": pdb11_identity,
        "packages": public_records,
    });
    // Aggregates only; printed so a failed pin can be re-measured.
    eprintln!(
        "pdb11 aggregates: {}",
        json!({
            "v16_projection_unchanged": v16_commitment == EXPECTED_V16_PROJECTION_COMMITMENT,
            "identity": matrix["pdb11_identity"],
            "v17_tables": V17_TABLES.map(|t| (t, matrix["shared_final_database_counts"][t].clone())),
        })
    );
    assert_eq!(
        v16_commitment, EXPECTED_V16_PROJECTION_COMMITMENT,
        "PDB-11 changed a pre-existing outcome, report total or table count; matrix output was not published"
    );

    let scheme_13 = private_records
        .iter()
        .filter(|record| record["scheme"] == 13)
        .collect::<Vec<_>>();
    assert_eq!(
        scheme_13.len(),
        4,
        "measured scheme-13 package count changed"
    );
    assert!(
        scheme_13.iter().all(|record| {
            record["isolation"]["status"] == "installed"
                && matches!(
                    record["shared"]["status"].as_str(),
                    Some("installed" | "deduplicated")
                )
        }),
        "every measured scheme-13 package must install in isolation and succeed in shared order"
    );
    for (scheme, expected) in [(12, 1), (14, 3), (21, 3)] {
        let records = private_records
            .iter()
            .filter(|record| record["scheme"] == scheme)
            .collect::<Vec<_>>();
        assert_eq!(
            records.len(),
            expected,
            "measured scheme-{scheme} package count changed"
        );
        assert!(
            records.iter().all(|record| {
                record["isolation"]["status"] == "installed"
                    && matches!(
                        record["shared"]["status"].as_str(),
                        Some("installed" | "deduplicated")
                    )
            }),
            "every measured scheme-{scheme} package must install in isolation and succeed in shared order"
        );
    }
    assert_eq!(
        matrix["scheme_12_14_feature_occurrences"],
        json!({
            "applies_to": 22,
            "occurrence": 5,
            "horizontal_ruler": 5,
            "separator_access": 2,
            "separator_ui_hint": 418,
            "ld_ctrl_declare_prop_desc": 14,
        }),
        "scheme-12/14 semantic evidence changed; matrix output was not published"
    );
    assert_eq!(
        matrix["scheme_21_feature_occurrences"],
        json!({
            "hardware_type": 3,
            "datapoint_variable_length": 6,
            "string_null_terminated": 6,
            "string_variable_length": 6,
            "optional_resource": 48,
            "access_policy": 444,
            "coupler_capabilities": 3,
            "rf_rx_capabilities": 2,
            "rf_tx_capabilities": 2,
            "property_description": 14,
        }),
        "scheme-21 retained evidence differs from read-only XML shape inventory; output was not published"
    );
    assert_eq!(
        matrix["isolated_catalogue_metadata_presence"],
        json!({
            "is_secure_enabled": 34,
            "max_security_group_key_table_entries": 34,
            "max_security_individual_address_entries": 32,
            "max_security_p2p_key_table_entries": 27,
            "max_tunneling_user_entries": 6,
            "max_user_entries": 6,
            "min_ets_version": 310,
            "replaces_versions": 140,
        }),
        "PDB-7 isolated winning-row coverage differs from measured XML inventory"
    );
    assert_eq!(
        matrix["shared_catalogue_metadata_presence"],
        json!({
            "is_secure_enabled": 33,
            "max_security_group_key_table_entries": 33,
            "max_security_individual_address_entries": 31,
            "max_security_p2p_key_table_entries": 27,
            "max_tunneling_user_entries": 5,
            "max_user_entries": 5,
            "min_ets_version": 273,
            "replaces_versions": 130,
        }),
        "PDB-7 shared first-winner catalogue coverage changed"
    );

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
        })
    );
    assert_eq!(
        matrix["shared_outcomes"],
        json!({
            "deduplicated": EXPECTED_SHARED_DEDUPLICATIONS,
            "installed": EXPECTED_SHARED_INSTALLS,
        })
    );
    // PDB-9 (schema v15): +11 on all three `unknown_count` totals. A v14
    // ingest reported each distinct `TypeColor`/`TypeTime` child as one
    // unknown *element* per program member; v15 types both kinds and reports
    // their unmodelled *attributes* instead. An independent Python recount
    // over the same 115 discovered packages (bundles included) predicted
    // 75 element rows -> 86 attribute rows, +11, identically for the 113
    // unique packages; every other total is unchanged.
    assert_eq!(
        matrix["isolation_report_totals"],
        json!({
            "attempt_count": 115,
            "member_count": 1606,
            "unknown_count": 22488,
            "conflict_count": 0,
            "dropped_datapoint_type_count": 0,
            "translation_counts": {"program": 2903208, "catalog": 2991, "hardware": 1424, "master": 112774},
        })
    );
    assert_eq!(
        matrix["shared_installed_report_totals"],
        json!({
            "attempt_count": 113,
            "member_count": 1586,
            "unknown_count": 22373,
            "conflict_count": 398,
            "dropped_datapoint_type_count": 39499,
            "translation_counts": {"program": 2779279, "catalog": 2353, "hardware": 1148, "master": 1640},
        })
    );
    assert_eq!(
        matrix["shared_successful_attempt_report_totals"],
        json!({
            "attempt_count": 115,
            "member_count": 1606,
            "unknown_count": 22488,
            "conflict_count": 400,
            "dropped_datapoint_type_count": 40232,
            "translation_counts": {"program": 2789468, "catalog": 2419, "hardware": 1162, "master": 1640},
        })
    );
    assert_eq!(
        matrix["shared_final_database_counts"]["package"], 113,
        "shared package rows changed"
    );
    assert_eq!(
        matrix["shared_final_database_counts"]["source_file"], 1163,
        "shared source-file rows changed"
    );
    assert_eq!(
        matrix["shared_final_database_counts"]["parameter"], 213930,
        "shared parameter rows changed"
    );
    assert_eq!(
        matrix["shared_final_database_counts"]["translation"], 2784420,
        "shared translation rows changed"
    );
    assert_eq!(
        matrix["shared_final_database_counts"]["package_install_count"], 3390,
        "shared install-count rows changed"
    );
    assert_eq!(
        matrix["shared_final_database_counts"]["package_install_diagnostic"], 859,
        "shared install-diagnostic rows changed"
    );
    // ADR-0052: two newly modelled Channel paths retire exactly these
    // unqualified `@Number` rows, not any other unknown evidence.
    assert_eq!(
        matrix["shared_final_database_counts"]["ingest_unknown"], 22771,
        "shared per-blob unknown evidence changed"
    );
    assert_eq!(
        matrix["shared_final_database_counts"]["package_install_unknown"], 9156,
        "shared per-package unknown evidence changed"
    );
    for (table, rows) in [
        ("package_baggage_inventory", 113),
        ("package_baggage_payload", 789),
        ("package_baggage_declaration", 776),
    ] {
        assert_eq!(
            matrix["shared_final_database_counts"][table], rows,
            "shared {table} rows changed"
        );
    }
    for (table, rows) in [
        ("package_source_name", 115),
        ("source_identity", 1972),
        ("source_identity_scan", 528),
        ("source_producer", 629),
    ] {
        assert_eq!(
            matrix["shared_final_database_counts"][table], rows,
            "shared {table} rows changed"
        );
    }
    // Per kind: candidate rows, distinct ids, ids in several blobs, ids whose
    // recorded elements differ. Every parsed member was measurable.
    let kind = |rows: u64, ids: u64, multi: u64, differing: u64| {
        json!({
            "candidate_rows": rows,
            "distinct_ids": ids,
            "ids_in_multiple_blobs": multi,
            "ids_with_differing_digests": differing,
        })
    };
    assert_eq!(
        matrix["pdb11_identity"],
        json!({
            "kinds": {
                "application_program": kind(302, 273, 29, 25),
                "catalog_item": kind(362, 345, 17, 0),
                "catalog_section": kind(251, 103, 53, 49),
                "hardware": kind(334, 255, 68, 50),
                "hardware2program": kind(337, 298, 33, 0),
                "product": kind(386, 306, 69, 46),
            },
            "divergent_ids": 170,
            "scan_status": {"measured": 528, "unavailable": 0},
        }),
        "PDB-11 identity aggregates changed"
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
