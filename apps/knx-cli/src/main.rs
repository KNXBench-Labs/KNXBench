//! Headless entry point. Keeping a real CLI alongside the desktop application
//! is what forces the core to stay free of user-interface dependencies and
//! makes import, roundtrip and regression tests runnable in CI without a
//! display.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str =
    "usage: knx import <file.knxproj> [--store <path.knxdb>] [--report-json <path.json>]\n\
     \x20                  [--product-db <path>] [--no-product-db]\n\
     \x20     knx export <store.knxdb> <out.knxproj> [--product-db <path>] [--no-product-db]\n\
     \x20     knx ga-export <store.knxdb> <out.csv>\n\
     \x20     knx ga-import <store.knxdb> <in.csv> [--dry-run]\n\
     \x20     knx products list [--manufacturer M-xxxx] [--product-db <path>]\n\
     \x20     knx products ingest <file.knxproj|file.knxprod|file.vd2> [--product-db <path>]\n\
     \x20     knx products show <program-id> [--product-db <path>]\n\
     \x20     knx products verify [--product-db <path>]\n\
     \x20     knx bus discover\n\
     \x20     knx bus monitor --gateway <host:port> [--project <path.knxdb>]\n\
     \x20     knx bus write --gateway <host:port> <main/middle/sub> <0|1|hex>\n\
     \x20     knx bus route-monitor --source-address <area.line.device> [--project <path.knxdb>]\n\
     \x20     knx bus route-send --source-address <area.line.device> <main/middle/sub> <0|1|hex>\n\
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
        Some("export") => run_export(&args[1..]),
        Some("ga-export") => run_ga_export(&args[1..]),
        Some("ga-import") => run_ga_import(&args[1..]),
        Some("products") => run_products(&args[1..]),
        Some("bus") => run_bus(&args[1..]),
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

    // Save the project to the store so it can be loaded back later (e.g., for export).
    if let Err(e) = knx_store::save_project(&conn, &imported.project) {
        eprintln!("failed to save project to store: {e}");
        return ExitCode::FAILURE;
    }

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

struct ExportArgs {
    store: String,
    output: String,
    product_db: Option<String>,
    no_product_db: bool,
}

fn parse_export_args(args: &[String]) -> Result<ExportArgs, String> {
    let mut store = None;
    let mut output = None;
    let mut product_db = None;
    let mut no_product_db = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
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
            other if store.is_none() => store = Some(other.to_string()),
            other if output.is_none() => output = Some(other.to_string()),
            other => return Err(format!("unexpected extra argument: {other}")),
        }
        i += 1;
    }
    if product_db.is_some() && no_product_db {
        return Err("--product-db and --no-product-db cannot both be given".to_string());
    }
    let store = store.ok_or_else(|| "missing <store.knxdb>".to_string())?;
    let output = output.ok_or_else(|| "missing <out.knxproj>".to_string())?;
    Ok(ExportArgs {
        store,
        output,
        product_db,
        no_product_db,
    })
}

fn run_export(args: &[String]) -> ExitCode {
    let parsed = match parse_export_args(args) {
        Ok(parsed) => parsed,
        Err(e) => {
            eprintln!("{e}\n{USAGE}");
            return ExitCode::FAILURE;
        }
    };

    let conn = match knx_store::open_and_migrate(&PathBuf::from(&parsed.store)) {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("failed to open store at {}: {e}", parsed.store);
            return ExitCode::FAILURE;
        }
    };

    let project = match knx_store::project::load_project(&conn) {
        Ok(project) => project,
        Err(e) => {
            eprintln!("failed to load project from store: {e}");
            return ExitCode::FAILURE;
        }
    };

    // `--no-product-db` runs without external product database, using only
    // what is stored in the project itself (the fallback from Session 3).
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

    let outcome = match knx_app::export_ets_project(&project, &conn, products_conn.as_ref()) {
        Ok(outcome) => outcome,
        Err(e) => {
            eprintln!("export failed: {e}");
            return ExitCode::FAILURE;
        }
    };

    // Print every warning to stderr.
    for warning in &outcome.warnings {
        eprintln!("warning: {:?}", warning);
    }

    // Write the exported bytes to the output file.
    if let Err(e) = std::fs::write(&parsed.output, &outcome.bytes) {
        eprintln!("failed to write export to {}: {e}", parsed.output);
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

struct GaExportArgs {
    store: String,
    output: String,
}

fn parse_ga_export_args(args: &[String]) -> Result<GaExportArgs, String> {
    let mut store = None;
    let mut output = None;
    for arg in args {
        match arg.as_str() {
            other if other.starts_with("--") => {
                return Err(format!("unknown flag: {other}"));
            }
            other if store.is_none() => store = Some(other.to_string()),
            other if output.is_none() => output = Some(other.to_string()),
            other => return Err(format!("unexpected extra argument: {other}")),
        }
    }
    let store = store.ok_or_else(|| "missing <store.knxdb>".to_string())?;
    let output = output.ok_or_else(|| "missing <out.csv>".to_string())?;
    Ok(GaExportArgs { store, output })
}

/// `knx ga-export` — writes every group address in the store's project as
/// "KNXBench group-address CSV v1" text (design §3, §7). This is a project
/// format this application defines and owns; it is not a claim of
/// compatibility with any export ETS produces.
fn run_ga_export(args: &[String]) -> ExitCode {
    let parsed = match parse_ga_export_args(args) {
        Ok(parsed) => parsed,
        Err(e) => {
            eprintln!("{e}\n{USAGE}");
            return ExitCode::FAILURE;
        }
    };

    let conn = match knx_store::open_and_migrate(&PathBuf::from(&parsed.store)) {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("failed to open store at {}: {e}", parsed.store);
            return ExitCode::FAILURE;
        }
    };

    let project = match knx_store::load_project(&conn) {
        Ok(project) => project,
        Err(e) => {
            eprintln!("failed to load project from store: {e}");
            return ExitCode::FAILURE;
        }
    };

    let export = knx_csv::export_group_addresses(&project);

    if let Err(e) = std::fs::write(&parsed.output, export.text.as_bytes()) {
        eprintln!("failed to write {}: {e}", parsed.output);
        return ExitCode::FAILURE;
    }

    print_export_report(&parsed.output, &export);
    ExitCode::SUCCESS
}

/// Prints an export's report as human-readable lines (design §7: "the
/// report prints as human-readable lines, not JSON") — every warning
/// `knx-csv` produced, not just a count, so nothing it reported is
/// silently dropped on the way to the terminal.
fn print_export_report(output: &str, export: &knx_csv::CsvExport) {
    println!("exported group addresses to {output}");
    println!("  {} warning(s)", export.warnings.len());
    for warning in &export.warnings {
        print_csv_problem(warning);
    }
}

fn print_csv_problem(problem: &knx_csv::CsvProblem) {
    let kind = match problem.severity {
        knx_csv::Severity::Error => "error",
        knx_csv::Severity::Warning => "warning",
    };
    match problem.row {
        Some(row) => println!("  {kind}: row {row}: {}", problem.detail),
        None => println!("  {kind}: {}", problem.detail),
    }
}

struct GaImportArgs {
    store: String,
    input: String,
    dry_run: bool,
}

fn parse_ga_import_args(args: &[String]) -> Result<GaImportArgs, String> {
    let mut store = None;
    let mut input = None;
    let mut dry_run = false;
    for arg in args {
        match arg.as_str() {
            "--dry-run" => dry_run = true,
            other if other.starts_with("--") => {
                return Err(format!("unknown flag: {other}"));
            }
            other if store.is_none() => store = Some(other.to_string()),
            other if input.is_none() => input = Some(other.to_string()),
            other => return Err(format!("unexpected extra argument: {other}")),
        }
    }
    let store = store.ok_or_else(|| "missing <store.knxdb>".to_string())?;
    let input = input.ok_or_else(|| "missing <in.csv>".to_string())?;
    Ok(GaImportArgs {
        store,
        input,
        dry_run,
    })
}

/// `knx ga-import` — reads `in.csv` as "KNXBench group-address CSV v1"
/// text, plans the edit against the store's project (design §4), and,
/// unless any row is a row-level error, applies the single resulting
/// `Command::Batch` and saves the store. `--dry-run` runs the identical
/// plan and prints the identical report but returns before the `apply`/
/// `save_project` calls below, so the store is provably untouched — the
/// printed report is built solely from `plan.report`, never from whether
/// the save happened, so it is byte-identical either way.
fn run_ga_import(args: &[String]) -> ExitCode {
    let parsed = match parse_ga_import_args(args) {
        Ok(parsed) => parsed,
        Err(e) => {
            eprintln!("{e}\n{USAGE}");
            return ExitCode::FAILURE;
        }
    };

    let conn = match knx_store::open_and_migrate(&PathBuf::from(&parsed.store)) {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("failed to open store at {}: {e}", parsed.store);
            return ExitCode::FAILURE;
        }
    };

    let mut project = match knx_store::load_project(&conn) {
        Ok(project) => project,
        Err(e) => {
            eprintln!("failed to load project from store: {e}");
            return ExitCode::FAILURE;
        }
    };

    let text = match std::fs::read_to_string(&parsed.input) {
        Ok(text) => text,
        Err(e) => {
            eprintln!("failed to read {}: {e}", parsed.input);
            return ExitCode::FAILURE;
        }
    };

    let parsed_csv = knx_csv::parse_group_addresses(&text, project.info.group_address_style);
    let plan = knx_csv::plan_import(&project, &parsed_csv);

    print_import_report(&parsed.input, &plan.report);

    let has_row_errors = plan
        .report
        .problems
        .iter()
        .any(|p| p.severity == knx_csv::Severity::Error);
    if has_row_errors {
        // design §4's all-or-nothing rule: `plan.command` is already `None`
        // here, so nothing below would have applied anyway — returning
        // early just keeps that guarantee explicit and keeps the store
        // untouched for both a real run and `--dry-run` alike.
        return ExitCode::from(EXIT_IMPORTED_WITH_ERRORS);
    }

    if parsed.dry_run {
        return ExitCode::SUCCESS;
    }

    if let Some(command) = plan.command {
        if let Err(e) = command.apply(&mut project) {
            eprintln!("failed to apply import: {e}");
            return ExitCode::FAILURE;
        }
        if let Err(e) = knx_store::save_project(&conn, &project) {
            eprintln!("failed to save project to store: {e}");
            return ExitCode::FAILURE;
        }
    }

    ExitCode::SUCCESS
}

/// Prints an import report as human-readable lines (design §7). Shared
/// verbatim between a real import and `--dry-run` — see `run_ga_import` —
/// so the two can never drift apart.
fn print_import_report(input: &str, report: &knx_csv::CsvImportReport) {
    println!("imported {input}");
    println!(
        "  {} row(s) read, {} created, {} updated, {} unchanged",
        report.rows_read, report.created, report.updated, report.unchanged
    );
    if report.created == 0 && report.updated == 0 {
        println!("  nothing to do");
    }
    for ignored in &report.ignored_columns {
        let reason = match ignored.reason {
            knx_csv::IgnoredColumnReason::ExportOnly => "export-only column, not applied on import",
            knx_csv::IgnoredColumnReason::Unknown => "unrecognized column",
        };
        println!("  ignored column '{}' ({reason})", ignored.name);
    }
    for problem in &report.problems {
        print_csv_problem(problem);
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
        eprintln!("missing <file.knxproj|file.knxprod|file.vd2>\n{USAGE}");
        return ExitCode::FAILURE;
    };

    let conn = match open_products_db(product_db.as_deref()) {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };

    if matches!(
        Path::new(file)
            .extension()
            .and_then(|extension| extension.to_str()),
        Some("knxprod" | "vd2")
    ) {
        let bytes = match std::fs::read(file) {
            Ok(bytes) => bytes,
            Err(error) => {
                eprintln!("failed to read product package {file}: {error}");
                return ExitCode::FAILURE;
            }
        };
        return match knx_productdb::install_package(&conn, file, &bytes) {
            Ok(report) => {
                println!(
                    "package installed: scheme {}, {} member(s), {} unknown construct(s), {} conflict(s){}",
                    report.scheme,
                    report.members.len(),
                    report.unknown,
                    report.conflicts.len(),
                    if report.skipped { " (already known)" } else { "" },
                );
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("failed to install product package {file}: {error}");
                ExitCode::FAILURE
            }
        };
    }

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

/// `knx bus monitor` — connects to a real KNXnet/IP gateway over tunnelling
/// and prints decoded telegrams as they arrive (spec §9, Task 9).
fn run_bus(args: &[String]) -> ExitCode {
    match args.first().map(String::as_str) {
        Some("discover") => run_bus_discover(&args[1..]),
        Some("monitor") => run_bus_monitor(&args[1..]),
        Some("write") => run_bus_write(&args[1..]),
        Some("route-monitor") => run_bus_route_monitor(&args[1..]),
        Some("route-send") => run_bus_route_send(&args[1..]),
        _ => {
            eprintln!("{USAGE}");
            ExitCode::FAILURE
        }
    }
}

/// `knx bus discover` — multicasts a `SEARCH_REQUEST` and prints every
/// gateway that answers within the spec's 10s window. Takes no arguments;
/// that's the point (no `--gateway` to already know).
fn run_bus_discover(args: &[String]) -> ExitCode {
    if !args.is_empty() {
        eprintln!("knx bus discover takes no arguments\n{USAGE}");
        return ExitCode::FAILURE;
    }
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("could not start async runtime: {e}");
            return ExitCode::FAILURE;
        }
    };
    runtime.block_on(run_bus_discover_async())
}

async fn run_bus_discover_async() -> ExitCode {
    use knx_net::BusConnection;
    let client = knx_net::KnxNetIpClient::new();
    match client.discover().await {
        Ok(gateways) if gateways.is_empty() => {
            println!("no gateways responded");
            ExitCode::SUCCESS
        }
        Ok(gateways) => {
            for g in gateways {
                let tag = if g.supports_tunnelling {
                    " [tunnelling]"
                } else {
                    ""
                };
                println!(
                    "{}  {}  {}{}",
                    g.individual_address, g.friendly_name, g.control_endpoint, tag
                );
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("discovery failed: {e}");
            ExitCode::FAILURE
        }
    }
}

struct BusMonitorArgs {
    gateway: String,
    project: Option<String>,
}

fn parse_bus_monitor_args(args: &[String]) -> Result<BusMonitorArgs, String> {
    let mut gateway = None;
    let mut project = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--gateway" => {
                gateway = Some(take_value(args, i + 1, "--gateway")?);
                i += 2;
            }
            "--project" => {
                project = Some(take_value(args, i + 1, "--project")?);
                i += 2;
            }
            other => return Err(format!("unrecognized argument: {other}")),
        }
    }
    Ok(BusMonitorArgs {
        gateway: gateway.ok_or_else(|| "--gateway is required".to_string())?,
        project,
    })
}

fn run_bus_monitor(args: &[String]) -> ExitCode {
    let parsed = match parse_bus_monitor_args(args) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}\n{USAGE}");
            return ExitCode::FAILURE;
        }
    };
    let gateway: std::net::SocketAddrV4 = match parsed.gateway.parse() {
        Ok(g) => g,
        Err(_) => {
            eprintln!("--gateway must be host:port, e.g. 192.0.2.1:3671");
            return ExitCode::FAILURE;
        }
    };
    let ga_names: std::collections::HashMap<u16, String> = match &parsed.project {
        Some(path) => match load_group_address_names(Path::new(path)) {
            Ok(names) => names,
            Err(e) => {
                eprintln!("could not load project {path}: {e}");
                return ExitCode::FAILURE;
            }
        },
        None => std::collections::HashMap::new(),
    };

    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("could not start async runtime: {e}");
            return ExitCode::FAILURE;
        }
    };

    runtime.block_on(run_bus_monitor_async(gateway, ga_names))
}

async fn run_bus_monitor_async(
    gateway: std::net::SocketAddrV4,
    ga_names: std::collections::HashMap<u16, String>,
) -> ExitCode {
    use knx_net::BusConnection;
    let client = knx_net::KnxNetIpClient::new();
    let tunnel = match client.connect_tunnel(gateway).await {
        Ok(t) => t,
        Err(e) => {
            eprintln!("could not connect to {gateway}: {e}");
            return ExitCode::FAILURE;
        }
    };
    eprintln!(
        "connected to {gateway}, assigned individual address {}. Ctrl-C to stop.",
        tunnel.assigned_address()
    );
    let mut telegrams = tunnel.subscribe();
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                eprintln!("disconnecting...");
                if let Err(e) = tunnel.disconnect().await {
                    eprintln!("disconnect: {e}");
                }
                break;
            }
            received = telegrams.recv() => match received {
                Ok(knx_net::TunnelEvent::Telegram(telegram)) => {
                    println!("{}", format_telegram(&telegram, &ga_names));
                }
                Ok(knx_net::TunnelEvent::Closed) => {
                    eprintln!("gateway closed the tunnel");
                    break;
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    eprintln!("warning: {n} telegram(s) dropped (receiver too slow)");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            },
        }
    }
    ExitCode::SUCCESS
}

struct BusWriteArgs {
    gateway: String,
    group_address: String,
    value: String,
}

fn parse_bus_write_args(args: &[String]) -> Result<BusWriteArgs, String> {
    let mut gateway = None;
    let mut positional = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--gateway" => {
                gateway = Some(take_value(args, i + 1, "--gateway")?);
                i += 2;
            }
            other => {
                positional.push(other.to_string());
                i += 1;
            }
        }
    }
    let [group_address, value] = &positional[..] else {
        return Err("expected exactly one group address and one value".to_string());
    };
    Ok(BusWriteArgs {
        gateway: gateway.ok_or_else(|| "--gateway is required".to_string())?,
        group_address: group_address.clone(),
        value: value.clone(),
    })
}

/// Parses a `GroupValueWrite` payload with no DPT interpretation (same
/// scope cut as the read-only monitor's decode side): `0`/`1` is a 6-bit
/// inline value (e.g. DPT-1), anything else is read as a hex byte string
/// (optionally `0x`-prefixed, e.g. `2a99`).
fn parse_group_value(s: &str) -> Result<knx_net::GroupValue, String> {
    match s {
        "0" => Ok(knx_net::GroupValue::Short(0)),
        "1" => Ok(knx_net::GroupValue::Short(1)),
        hex => {
            let hex = hex.strip_prefix("0x").unwrap_or(hex);
            if hex.is_empty() || hex.len() % 2 != 0 {
                return Err(format!(
                    "value must be 0, 1, or an even-length hex byte string, got {s}"
                ));
            }
            (0..hex.len())
                .step_by(2)
                .map(|i| {
                    u8::from_str_radix(&hex[i..i + 2], 16)
                        .map_err(|_| format!("invalid hex value: {s}"))
                })
                .collect::<Result<Vec<u8>, String>>()
                .map(knx_net::GroupValue::Bytes)
        }
    }
}

fn run_bus_write(args: &[String]) -> ExitCode {
    let parsed = match parse_bus_write_args(args) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}\n{USAGE}");
            return ExitCode::FAILURE;
        }
    };
    let gateway: std::net::SocketAddrV4 = match parsed.gateway.parse() {
        Ok(g) => g,
        Err(_) => {
            eprintln!("--gateway must be host:port, e.g. 192.0.2.1:3671");
            return ExitCode::FAILURE;
        }
    };
    let group_address = match knx_core::GroupAddress::parse(
        &parsed.group_address,
        knx_core::GroupAddressStyle::ThreeLevel,
    ) {
        Ok(ga) => ga,
        Err(e) => {
            eprintln!("invalid group address {}: {e}", parsed.group_address);
            return ExitCode::FAILURE;
        }
    };
    let value = match parse_group_value(&parsed.value) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };

    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("could not start async runtime: {e}");
            return ExitCode::FAILURE;
        }
    };
    runtime.block_on(run_bus_write_async(gateway, group_address, value))
}

async fn run_bus_write_async(
    gateway: std::net::SocketAddrV4,
    group_address: knx_core::GroupAddress,
    value: knx_net::GroupValue,
) -> ExitCode {
    use knx_net::{ApplicationService, BusConnection, Destination};
    let client = knx_net::KnxNetIpClient::new();
    let tunnel = match client.connect_tunnel(gateway).await {
        Ok(t) => t,
        Err(e) => {
            eprintln!("could not connect to {gateway}: {e}");
            return ExitCode::FAILURE;
        }
    };
    let result = tunnel
        .send(
            Destination::Group(group_address),
            ApplicationService::GroupValueWrite(value),
        )
        .await;
    if let Err(e) = tunnel.disconnect().await {
        eprintln!("disconnect: {e}");
    }
    match result {
        Ok(()) => {
            println!(
                "wrote to {}",
                group_address.format(knx_core::GroupAddressStyle::ThreeLevel)
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("write failed: {e}");
            ExitCode::FAILURE
        }
    }
}

struct BusRouteMonitorArgs {
    source_address: String,
    project: Option<String>,
}

fn parse_bus_route_monitor_args(args: &[String]) -> Result<BusRouteMonitorArgs, String> {
    let mut source_address = None;
    let mut project = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--source-address" => {
                source_address = Some(take_value(args, i + 1, "--source-address")?);
                i += 2;
            }
            "--project" => {
                project = Some(take_value(args, i + 1, "--project")?);
                i += 2;
            }
            other => return Err(format!("unrecognized argument: {other}")),
        }
    }
    Ok(BusRouteMonitorArgs {
        source_address: source_address.ok_or_else(|| "--source-address is required".to_string())?,
        project,
    })
}

fn run_bus_route_monitor(args: &[String]) -> ExitCode {
    let parsed = match parse_bus_route_monitor_args(args) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}\n{USAGE}");
            return ExitCode::FAILURE;
        }
    };
    let own_address: knx_core::IndividualAddress = match parsed.source_address.parse() {
        Ok(a) => a,
        Err(_) => {
            eprintln!("--source-address must be area.line.device, e.g. 1.1.1");
            return ExitCode::FAILURE;
        }
    };
    let ga_names: std::collections::HashMap<u16, String> = match &parsed.project {
        Some(path) => match load_group_address_names(Path::new(path)) {
            Ok(names) => names,
            Err(e) => {
                eprintln!("could not load project {path}: {e}");
                return ExitCode::FAILURE;
            }
        },
        None => std::collections::HashMap::new(),
    };

    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("could not start async runtime: {e}");
            return ExitCode::FAILURE;
        }
    };
    runtime.block_on(run_bus_route_monitor_async(own_address, ga_names))
}

async fn run_bus_route_monitor_async(
    own_address: knx_core::IndividualAddress,
    ga_names: std::collections::HashMap<u16, String>,
) -> ExitCode {
    use knx_net::BusConnection;
    let client = knx_net::KnxNetIpClient::new();
    let routing = match client.connect_routing(own_address).await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("could not join the routing multicast group: {e}");
            return ExitCode::FAILURE;
        }
    };
    eprintln!("joined routing multicast as {own_address}. Ctrl-C to stop.");
    let mut telegrams = routing.subscribe();
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => break,
            received = telegrams.recv() => match received {
                Ok(telegram) => println!("{}", format_telegram(&telegram, &ga_names)),
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    eprintln!("warning: {n} telegram(s) dropped (receiver too slow)");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            },
        }
    }
    ExitCode::SUCCESS
}

struct BusRouteSendArgs {
    source_address: String,
    group_address: String,
    value: String,
}

fn parse_bus_route_send_args(args: &[String]) -> Result<BusRouteSendArgs, String> {
    let mut source_address = None;
    let mut positional = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--source-address" => {
                source_address = Some(take_value(args, i + 1, "--source-address")?);
                i += 2;
            }
            other => {
                positional.push(other.to_string());
                i += 1;
            }
        }
    }
    // Check source_address first to provide clear error messaging
    let source_address =
        source_address.ok_or_else(|| "--source-address is required".to_string())?;
    let [group_address, value] = &positional[..] else {
        return Err("expected exactly one group address and one value".to_string());
    };
    Ok(BusRouteSendArgs {
        source_address,
        group_address: group_address.clone(),
        value: value.clone(),
    })
}

fn run_bus_route_send(args: &[String]) -> ExitCode {
    let parsed = match parse_bus_route_send_args(args) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}\n{USAGE}");
            return ExitCode::FAILURE;
        }
    };
    let own_address: knx_core::IndividualAddress = match parsed.source_address.parse() {
        Ok(a) => a,
        Err(_) => {
            eprintln!("--source-address must be area.line.device, e.g. 1.1.1");
            return ExitCode::FAILURE;
        }
    };
    let group_address = match knx_core::GroupAddress::parse(
        &parsed.group_address,
        knx_core::GroupAddressStyle::ThreeLevel,
    ) {
        Ok(ga) => ga,
        Err(e) => {
            eprintln!("invalid group address {}: {e}", parsed.group_address);
            return ExitCode::FAILURE;
        }
    };
    let value = match parse_group_value(&parsed.value) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };

    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("could not start async runtime: {e}");
            return ExitCode::FAILURE;
        }
    };
    runtime.block_on(run_bus_route_send_async(own_address, group_address, value))
}

async fn run_bus_route_send_async(
    own_address: knx_core::IndividualAddress,
    group_address: knx_core::GroupAddress,
    value: knx_net::GroupValue,
) -> ExitCode {
    use knx_net::{ApplicationService, BusConnection, Destination};
    let client = knx_net::KnxNetIpClient::new();
    let routing = match client.connect_routing(own_address).await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("could not join the routing multicast group: {e}");
            return ExitCode::FAILURE;
        }
    };
    // Routing is unconfirmed (Routing v01.05.02 AS §5.1) — "sent", not
    // "wrote", since there's no ACK to confirm delivery, unlike tunnelling.
    match routing
        .send(
            Destination::Group(group_address),
            ApplicationService::GroupValueWrite(value),
        )
        .await
    {
        Ok(()) => {
            println!(
                "sent to {}",
                group_address.format(knx_core::GroupAddressStyle::ThreeLevel)
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("send failed: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Loads `group_address -> name` for every installation in a stored
/// project, so `format_telegram` can annotate a raw group address with the
/// name the user gave it in ETS. Silently returns an empty map only when
/// the caller passed no `--project` at all (see `run_bus_monitor`) — any
/// failure to open or read a *given* path is reported, never swallowed.
fn load_group_address_names(path: &Path) -> Result<std::collections::HashMap<u16, String>, String> {
    let conn = knx_store::migration::open_and_migrate(path).map_err(|e| e.to_string())?;
    let project = knx_store::project::load_project(&conn).map_err(|e| e.to_string())?;
    let mut names = std::collections::HashMap::new();
    for installation in &project.installations {
        for entry in &installation.group_addresses {
            names.insert(entry.address.raw(), entry.name.clone());
        }
    }
    Ok(names)
}

fn format_telegram(
    telegram: &knx_net::LDataFrame,
    ga_names: &std::collections::HashMap<u16, String>,
) -> String {
    use knx_net::Destination;
    let dest = match telegram.destination {
        Destination::Group(ga) => {
            let formatted = ga.format(knx_core::GroupAddressStyle::ThreeLevel);
            match ga_names.get(&ga.raw()) {
                Some(name) => format!("{formatted} ({name})"),
                None => formatted,
            }
        }
        Destination::Individual(ia) => ia.to_string(),
    };
    format!(
        "{} -> {dest}: {}",
        telegram.source,
        format_service(&telegram.service)
    )
}

fn format_service(service: &knx_net::ApplicationService) -> String {
    use knx_net::{ApplicationService, GroupValue};
    let format_value = |v: &GroupValue| match v {
        GroupValue::Short(bits) => format!("{bits:#04x} (6-bit)"),
        GroupValue::Bytes(bytes) => format!("{bytes:02x?}"),
    };
    match service {
        ApplicationService::GroupValueRead => "GroupValueRead".to_string(),
        ApplicationService::GroupValueResponse(v) => {
            format!("GroupValueResponse {}", format_value(v))
        }
        ApplicationService::GroupValueWrite(v) => format!("GroupValueWrite {}", format_value(v)),
        ApplicationService::Other { apci, data } => {
            format!("APCI {apci:#06x} data {data:02x?}")
        }
    }
}
