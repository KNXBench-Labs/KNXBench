//! Headless entry point. Keeping a real CLI alongside the desktop application
//! is what forces the core to stay free of user-interface dependencies and
//! makes import, roundtrip and regression tests runnable in CI without a
//! display.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str =
    "usage: knx import <file.knxproj> [--store <path.knxdb>] [--report-json <path.json>]\n\
     \x20                  [--product-db <path>] [--no-product-db]\n\
     \x20     knx products list [--manufacturer M-xxxx] [--product-db <path>]\n\
     \x20     knx products ingest <file.knxproj> [--product-db <path>]\n\
     \x20     knx products show <program-id> [--product-db <path>]\n\
     \x20     knx products verify [--product-db <path>]\n\
     exit codes: 0 = imported cleanly (warnings allowed), 1 = could not import,\n\
     2 = imported, but the report contains errors";

/// Exit code for "the import produced a project, but the report contains
/// `Severity::Error` entries" — data the mapper could not use, such as a
/// dangling reference or a duplicate id. Distinct from `ExitCode::FAILURE`
/// (nothing was imported at all) so a script can tell "no project" from
/// "a project with known holes in it", and distinct from success so those
/// holes cannot pass unnoticed in CI.
const EXIT_IMPORTED_WITH_ERRORS: u8 = 2;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("import") => run_import(&args[1..]),
        Some("products") => run_products(&args[1..]),
        _ => {
            eprintln!("{USAGE}");
            ExitCode::FAILURE
        }
    }
}

struct ImportArgs {
    file: String,
    store: Option<String>,
    report_json: Option<String>,
    product_db: Option<String>,
    no_product_db: bool,
}

/// Reads the value following a `--flag`. Refuses to treat the *next* flag
/// as this one's value (`--store --report-json out.json f.knxproj` would
/// otherwise silently swallow `--report-json` as `--store`'s path, leaving
/// `--report-json` itself unrecognized) — a missing value is a usage
/// error, not a value that happens to start with `--`.
fn take_value(args: &[String], i: usize, flag: &str) -> Result<String, String> {
    match args.get(i) {
        Some(v) if !v.starts_with("--") => Ok(v.clone()),
        _ => Err(format!("{flag} needs a value")),
    }
}

fn parse_import_args(args: &[String]) -> Result<ImportArgs, String> {
    let mut file = None;
    let mut store = None;
    let mut report_json = None;
    let mut product_db = None;
    let mut no_product_db = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--store" => {
                i += 1;
                store = Some(take_value(args, i, "--store")?);
            }
            "--report-json" => {
                i += 1;
                report_json = Some(take_value(args, i, "--report-json")?);
            }
            "--product-db" => {
                i += 1;
                product_db = Some(take_value(args, i, "--product-db")?);
            }
            "--no-product-db" => {
                no_product_db = true;
            }
            other if other.starts_with("--") => {
                return Err(format!("unknown flag: {other}"));
            }
            other if file.is_none() => file = Some(other.to_string()),
            other => return Err(format!("unexpected extra argument: {other}")),
        }
        i += 1;
    }
    if product_db.is_some() && no_product_db {
        return Err("--product-db and --no-product-db cannot both be given".to_string());
    }
    let file = file.ok_or_else(|| "missing <file.knxproj>".to_string())?;
    Ok(ImportArgs {
        file,
        store,
        report_json,
        product_db,
        no_product_db,
    })
}

/// Resolves the product database path: the explicit `--product-db` value,
/// else `knx_productdb::default_path()`, else a usage error naming the
/// environment variable that would have supplied one.
fn resolve_product_db_path(explicit: Option<&str>) -> Result<PathBuf, String> {
    if let Some(path) = explicit {
        return Ok(PathBuf::from(path));
    }
    knx_productdb::default_path().ok_or_else(|| {
        "no --product-db given and neither XDG_DATA_HOME nor HOME is set to derive a default"
            .to_string()
    })
}

fn run_import(args: &[String]) -> ExitCode {
    let parsed = match parse_import_args(args) {
        Ok(parsed) => parsed,
        Err(e) => {
            eprintln!("{e}\n{USAGE}");
            return ExitCode::FAILURE;
        }
    };

    // `--store` names a persistent database; without it, this run's opaque
    // entries live only in a temp file for the duration of the process —
    // `open_and_migrate` always takes a path, so a plain "just show me the
    // report" invocation still needs one to exist, just not to outlive it.
    let _temp_dir;
    let db_path: PathBuf = match &parsed.store {
        Some(path) => PathBuf::from(path),
        None => {
            _temp_dir = match tempfile::tempdir() {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("failed to create a temporary store: {e}");
                    return ExitCode::FAILURE;
                }
            };
            _temp_dir.path().join("import.knxdb")
        }
    };

    let conn = match knx_store::open_and_migrate(&db_path) {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("failed to open store at {}: {e}", db_path.display());
            return ExitCode::FAILURE;
        }
    };

    // `--no-product-db` runs exactly the Session 3 path: manufacturer bytes
    // go into the project's own opaque store, no ingest, no enrichment —
    // a tested fallback rather than an assertion (spec §8).
    let products_conn = if parsed.no_product_db {
        None
    } else {
        let path = match resolve_product_db_path(parsed.product_db.as_deref()) {
            Ok(path) => path,
            Err(e) => {
                eprintln!("{e}\n{USAGE}");
                return ExitCode::FAILURE;
            }
        };
        match knx_productdb::open_and_migrate(&path) {
            Ok(conn) => Some(conn),
            Err(e) => {
                eprintln!("failed to open product database at {}: {e}", path.display());
                return ExitCode::FAILURE;
            }
        }
    };

    let imported = match knx_app::import_ets_project_with(
        Path::new(&parsed.file),
        &conn,
        knx_app::ImportOptions {
            product_db: products_conn.as_ref(),
        },
    ) {
        Ok(imported) => imported,
        Err(e) => {
            eprintln!("import failed for {}: {e}", parsed.file);
            return ExitCode::FAILURE;
        }
    };

    print_summary(&parsed.file, &imported);

    if let Some(path) = &parsed.report_json {
        if let Err(e) = std::fs::write(path, imported.report.to_json()) {
            eprintln!("failed to write report to {path}: {e}");
            return ExitCode::FAILURE;
        }
    }

    // Warnings never change the exit code: an import that reports plenty of
    // unsupported constructs still succeeded at producing a project. Real
    // errors do — see EXIT_IMPORTED_WITH_ERRORS.
    if error_count(&imported.report) > 0 {
        return ExitCode::from(EXIT_IMPORTED_WITH_ERRORS);
    }
    ExitCode::SUCCESS
}

/// Report entries that are genuine errors, not warnings — `ImportReport`
/// keeps both in one `errors` Vec, told apart by their `Severity`.
fn error_count(report: &knx_etsproj::ImportReport) -> usize {
    report
        .errors
        .iter()
        .filter(|e| e.severity == knx_etsproj::report::Severity::Error)
        .count()
}

fn print_summary(file: &str, imported: &knx_app::ImportedProject) {
    let count = |entity: &str| {
        imported
            .report
            .counts
            .rows
            .iter()
            .find(|c| c.entity == entity)
            .map(|c| c.mapped)
            .unwrap_or(0)
    };

    println!("imported {file}");
    println!("  {} devices", count("DeviceInstance"));
    println!("  {} group addresses", count("GroupAddress"));
    println!("  {} communication objects", count("ComObjectInstanceRef"));
    println!(
        "  {} unsupported feature(s), {} opaque entr{} stored",
        imported.report.unsupported.len(),
        imported.opaque_entries,
        if imported.opaque_entries == 1 {
            "y"
        } else {
            "ies"
        }
    );
    let error_count = error_count(&imported.report);
    let warning_count = imported.report.errors.len() - error_count;
    if imported.report.has_losses() {
        println!(
            "  {error_count} error(s), {} unknown construct(s) — see the report for detail",
            imported.report.unknown.len()
        );
    }
    if warning_count > 0 {
        println!("  {warning_count} warning(s) — see the report for detail");
    }
    if let Some(enrichment) = &imported.enrichment {
        println!(
            "  {} manufacturer file(s) ingested, {} already known",
            imported.manufacturer_ingested, imported.manufacturer_skipped
        );
        println!(
            "  {} communication object(s) enriched from {} application program(s), {} issue(s)",
            enrichment.com_objects_enriched,
            enrichment.devices_resolved,
            enrichment.issues.len()
        );
    }
}

/// `knx products list|ingest|show|verify` — inspection and separate ingest
/// of the shared product database (spec §8).
fn run_products(args: &[String]) -> ExitCode {
    match args.first().map(String::as_str) {
        Some("list") => run_products_list(&args[1..]),
        Some("ingest") => run_products_ingest(&args[1..]),
        Some("show") => run_products_show(&args[1..]),
        Some("verify") => run_products_verify(&args[1..]),
        _ => {
            eprintln!("{USAGE}");
            ExitCode::FAILURE
        }
    }
}

/// Pulls `--product-db <path>` out of an arbitrary flag/positional mix,
/// returning what is left over (in order) as the positional arguments.
fn split_product_db_flag(args: &[String]) -> Result<(Option<String>, Vec<String>), String> {
    let mut product_db = None;
    let mut rest = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--product-db" {
            i += 1;
            product_db = Some(take_value(args, i, "--product-db")?);
        } else {
            rest.push(args[i].clone());
        }
        i += 1;
    }
    Ok((product_db, rest))
}

fn open_products_db(explicit: Option<&str>) -> Result<knx_productdb::Connection, String> {
    let path = resolve_product_db_path(explicit)?;
    knx_productdb::open_and_migrate(&path)
        .map_err(|e| format!("failed to open product database at {}: {e}", path.display()))
}

fn run_products_list(args: &[String]) -> ExitCode {
    let (product_db, rest) = match split_product_db_flag(args) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{e}\n{USAGE}");
            return ExitCode::FAILURE;
        }
    };
    let mut manufacturer_filter = None;
    let mut i = 0;
    while i < rest.len() {
        match rest[i].as_str() {
            "--manufacturer" => {
                i += 1;
                match take_value(&rest, i, "--manufacturer") {
                    Ok(v) => manufacturer_filter = Some(v),
                    Err(e) => {
                        eprintln!("{e}\n{USAGE}");
                        return ExitCode::FAILURE;
                    }
                }
            }
            other => {
                eprintln!("unknown flag: {other}\n{USAGE}");
                return ExitCode::FAILURE;
            }
        }
        i += 1;
    }

    let conn = match open_products_db(product_db.as_deref()) {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };

    let manufacturers = match knx_productdb::query::manufacturers(&conn) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    for (id, name) in &manufacturers {
        if let Some(filter) = &manufacturer_filter {
            if id != filter {
                continue;
            }
        }
        println!("{id}  {}", name.as_deref().unwrap_or("(unnamed)"));
        match knx_productdb::query::programs(&conn, Some(id)) {
            Ok(programs) => {
                for p in programs {
                    println!(
                        "  {}  {}  application {} v{}  mask {}",
                        p.id,
                        p.name.as_deref().unwrap_or(""),
                        p.application_number.as_deref().unwrap_or("?"),
                        p.application_version.as_deref().unwrap_or("?"),
                        p.mask_version.as_deref().unwrap_or("?"),
                    );
                }
            }
            Err(e) => eprintln!("{e}"),
        }
    }
    ExitCode::SUCCESS
}

fn run_products_ingest(args: &[String]) -> ExitCode {
    let (product_db, rest) = match split_product_db_flag(args) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{e}\n{USAGE}");
            return ExitCode::FAILURE;
        }
    };
    let Some(file) = rest.first() else {
        eprintln!("missing <file.knxproj>\n{USAGE}");
        return ExitCode::FAILURE;
    };

    let conn = match open_products_db(product_db.as_deref()) {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };

    let outcome = match knx_etsproj::import_knxproj(Path::new(file)) {
        Ok(outcome) => outcome,
        Err(e) => {
            eprintln!("import failed for {file}: {e}");
            return ExitCode::FAILURE;
        }
    };

    let mut ingested = 0usize;
    let mut skipped = 0usize;
    for m in &outcome.manufacturer {
        match knx_productdb::ingest_file(&conn, &m.source_path, &m.bytes) {
            Ok(knx_productdb::IngestOutcome::Ingested { .. }) => ingested += 1,
            Ok(knx_productdb::IngestOutcome::Skipped { .. }) => skipped += 1,
            Err(e) => {
                eprintln!("failed to ingest {}: {e}", m.source_path);
                return ExitCode::FAILURE;
            }
        }
    }
    if let Some(master) = outcome
        .opaque
        .iter()
        .find(|e| e.kind == knx_etsproj::opaque::OpaqueKind::MasterData)
    {
        if let Err(e) = knx_productdb::ingest_master_data(&conn, &master.bytes) {
            eprintln!("failed to ingest master data: {e}");
            return ExitCode::FAILURE;
        }
    }

    println!("{ingested} manufacturer file(s) ingested, {skipped} already known");
    ExitCode::SUCCESS
}

fn run_products_show(args: &[String]) -> ExitCode {
    let (product_db, rest) = match split_product_db_flag(args) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{e}\n{USAGE}");
            return ExitCode::FAILURE;
        }
    };
    let Some(program_id) = rest.first() else {
        eprintln!("missing <program-id>\n{USAGE}");
        return ExitCode::FAILURE;
    };

    let conn = match open_products_db(product_db.as_deref()) {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };

    let programs = match knx_productdb::query::programs(&conn, None) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    let Some(program) = programs.into_iter().find(|p| &p.id == program_id) else {
        eprintln!("no such program: {program_id}");
        return ExitCode::FAILURE;
    };

    let com_objects: i64 = conn
        .query_row(
            "SELECT count(*) FROM com_object WHERE program_id = ?1",
            [program_id],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let parameters: i64 = conn
        .query_row(
            "SELECT count(*) FROM parameter WHERE program_id = ?1",
            [program_id],
            |r| r.get(0),
        )
        .unwrap_or(0);

    println!("{}  {}", program.id, program.name.as_deref().unwrap_or(""));
    println!(
        "  manufacturer {}  application {} v{}  mask {}",
        program.manufacturer_id,
        program.application_number.as_deref().unwrap_or("?"),
        program.application_version.as_deref().unwrap_or("?"),
        program.mask_version.as_deref().unwrap_or("?"),
    );
    println!("  {com_objects} communication object(s), {parameters} parameter(s)");
    ExitCode::SUCCESS
}

fn run_products_verify(args: &[String]) -> ExitCode {
    let (product_db, _rest) = match split_product_db_flag(args) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{e}\n{USAGE}");
            return ExitCode::FAILURE;
        }
    };
    let conn = match open_products_db(product_db.as_deref()) {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    let mismatches = match knx_productdb::verify(&conn) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    for m in &mismatches {
        println!(
            "{}  stored {}  actual {}",
            m.source_path, m.sha256, m.actual_sha256
        );
    }
    println!("{} mismatch(es)", mismatches.len());
    if mismatches.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
