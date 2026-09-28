//! The headless `knx` command-line entry point.
//!
//! Keeping a real CLI alongside the desktop application is what forces the
//! core to stay free of user-interface dependencies and makes import and
//! regression tests runnable in CI without a display.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

mod device_download;
mod scan;

const USAGE: &str =
    "usage: knx import <file.knxproj> [--store <path.knxdb>] [--report-json <path.json>]\n\
     \x20                  [--product-db <path>] [--no-product-db]\n\
     \x20     knx ga-export <store.knxdb> <out.csv>\n\
     \x20     knx ga-import <store.knxdb> <in.csv> [--dry-run] [--confirm <token>]\n\
     \x20     knx doc-export <store.knxdb> <out.html>\n\
     \x20     knx diff [--exit-code] <a.knxdb|a.knxproj> <b.knxdb|b.knxproj>\n\
     \x20     knx products list [--manufacturer M-xxxx] [--product-db <path>]\n\
     \x20     knx products ingest <file.knxproj|file.knxprod|file.vd2> [--product-db <path>]\n\
     \x20     knx products show <program-id> [--product-db <path>]\n\
     \x20     knx products verify [--product-db <path>]\n\
     \x20     knx products identity <table> <id> [--product-db <path>]\n\
     \x20     knx products family <program-id> [--product-db <path>]\n\
     \x20     knx products order-number <manufacturer-id> <order-number> [--product-db <path>]\n\
     \x20     knx bus discover\n\
     \x20     knx bus monitor --gateway <host:port> [--project <path.knxdb>]\n\
     \x20         (with --project, decodes against each address's resolved DPT)\n\
     \x20     knx bus write --gateway <host:port> [--project <path.knxdb>] [--dpt <DPST-m-s>]\n\
     \x20                  [--input-format <canonical|decimal|hexadecimal|binary|text>]\n\
     \x20                  [--dry-run] <main/middle/sub> <value>\n\
     \x20         (--dpt encodes <value> as that type; --project resolves it from the\n\
     \x20         linked communication objects; neither given falls back to raw\n\
     \x20         0|1|hex; --dry-run encodes and prints without opening a connection)\n\
     \x20     knx bus route-monitor --source-address <area.line.device> [--project <path.knxdb>]\n\
     \x20                  [--multicast-group <addr>]\n\
     \x20     knx bus route-send --source-address <area.line.device> [--multicast-group <addr>]\n\
     \x20                  <main/middle/sub> <0|1|hex>\n\
     \x20         (--multicast-group joins that IPv4 multicast address instead of the\n\
     \x20         standard 224.0.23.12, port fixed at 3671 either way — Routing v01.05.02 AS\n\
     \x20         §2.3.1; omitted, both route-monitor and route-send join the standard group)\n\
     \x20     knx bus scan --gateway <host:port> --line <area.line>\n\
     \x20                  [--range <first>-<last>] [--exclude <addr>[,<addr>...]]...\n\
     \x20                  [--timeout-ms <n>] [--pause-ms <n>] [--project <path.knxdb>] [--dry-run]\n\
     \x20         (--range takes two full area.line.device addresses on the --line given, and\n\
     \x20         its first device may not be 0, the line coupler's own address; --exclude may\n\
     \x20         be given more than once and every occurrence accumulates, never probes the\n\
     \x20         listed addresses, and a malformed one aborts before any frame is sent;\n\
     \x20         --project prints a comparison, never writes it back; --dry-run prints the\n\
     \x20         candidate count, first/last candidate and excluded list, then exits without\n\
     \x20         opening a connection)\n\
     \x20     knx device download <area.line.device> --project <path.knxdb> [--product-db <path>]\n\
     \x20                  [--gateway <host:port> --confirm \"I confirm download to <address>\"]\n\
     \x20         (a download TO the device over the bus; without --confirm it prints the\n\
     \x20         plan (segments, octets, steps) and opens no connection. The phrase must\n\
     \x20         name this device; excluded addresses are refused before anything opens)\n\
     \x20     knx --version\n\
     exit codes: 0 = success (for import/ga-import, warnings are still success),\n\
     1 = failure (bad arguments, I/O, a transport problem, or no usable data);\n\
     2 = import/ga-import only: a project was produced but the report contains\n\
     errors. For `diff --exit-code`: 0 = equal, 1 = different (including\n\
     ambiguity), 2 = arguments, input, import, or store failure.";

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
        Some("ga-export") => run_ga_export(&args[1..]),
        Some("ga-import") => run_ga_import(&args[1..]),
        Some("doc-export") => run_doc_export(&args[1..]),
        Some("diff") => run_diff(&args[1..]),
        Some("products") => run_products(&args[1..]),
        Some("bus") => run_bus(&args[1..]),
        Some("device") => run_device(&args[1..]),
        Some("--version" | "-V") => {
            println!("{}", version_line());
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("{USAGE}");
            ExitCode::FAILURE
        }
    }
}

/// `knx <version>[+g<sha>]`: the manifest version, plus SemVer 2.0.0 build
/// metadata naming the commit when the build knew it (`build.rs` asks git;
/// a tarball or Docker build may not have one). The manifest version is
/// what the program *is*; the sha is which sources it was built from —
/// every file's state at once, which is why no file carries a version of
/// its own (ADR-0018).
fn version_line() -> String {
    format_version_line(
        env!("CARGO_BIN_NAME"),
        env!("CARGO_PKG_VERSION"),
        option_env!("KNX_BUILD_SHA").filter(|sha| !sha.is_empty()),
    )
}

fn format_version_line(name: &str, version: &str, sha: Option<&str>) -> String {
    match sha {
        Some(sha) => format!("{name} {version}+g{sha}"),
        None => format!("{name} {version}"),
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
pub(crate) fn take_value(args: &[String], i: usize, flag: &str) -> Result<String, String> {
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

    // Save the project to the store so it can be loaded back later.
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
    report.error_count()
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
        // Said out loud, because the line above cannot: with an empty
        // product database it reads "0 enriched from 0 programs, 0
        // issues", three zeroes that look like a clean run rather than a
        // project whose devices are all unknown. Since T27 withdrew
        // `.knxproj` export there is no `MissingManufacturerData` export
        // warning left to say it either.
        if enrichment.devices_unresolved > 0 {
            let cause = if enrichment.available {
                "no application program in the product database matches them"
            } else {
                "the product database holds no application program at all"
            };
            println!(
                "  of which {} device(s) have no resolvable product: {cause} — \
                 their communication objects keep only what the project file itself stated",
                enrichment.devices_unresolved
            );
        }
    }
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

struct DocExportArgs {
    store: String,
    output: String,
}

fn parse_doc_export_args(args: &[String]) -> Result<DocExportArgs, String> {
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
    let output = output.ok_or_else(|| "missing <out.html>".to_string())?;
    Ok(DocExportArgs { store, output })
}

/// `knx doc-export` — writes the store's project as one self-contained
/// "project documentation" HTML file (`knx_report::render_html`). This is
/// a document this application defines and owns; it is not a claim of
/// compatibility with any report ETS produces, and is never described as
/// one.
fn run_doc_export(args: &[String]) -> ExitCode {
    let parsed = match parse_doc_export_args(args) {
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

    // The clock read lives here, never inside `knx-report` — that crate
    // reads only `ReportOptions`, which is what keeps its own tests
    // deterministic (see `knx-report`'s doc comment; `knx-server` does the
    // same thing on the HTTP side).
    let products = knx_productdb::default_path()
        .filter(|path| path.exists())
        .and_then(|path| match knx_productdb::open_and_migrate(&path) {
            Ok(products) => Some(products),
            Err(error) => {
                eprintln!(
                    "warning: failed to open product database at {}: {error}",
                    path.display()
                );
                None
            }
        });
    let options = knx_app::documentation::report_options(
        &project,
        products.as_ref(),
        chrono::Utc::now(),
        knx_report::ReportLanguage::English,
        knx_report::ReportSection::ALL.into_iter().collect(),
    );
    let report = knx_report::render_html(&project, &options);

    if let Err(e) = std::fs::write(&parsed.output, report.html.as_bytes()) {
        eprintln!("failed to write {}: {e}", parsed.output);
        return ExitCode::FAILURE;
    }

    print_doc_export_report(&parsed.output, &report);
    // No third exit code here: unlike `import`'s `EXIT_IMPORTED_WITH_ERRORS`,
    // `report.warnings` describes the project the render walked, not a
    // failed export — `render_html` has no `Result` because it cannot
    // fail this way. A document that carries warnings is still a
    // complete, correct report.
    ExitCode::SUCCESS
}

/// Prints a documentation export's report as human-readable lines, mirroring
/// `print_export_report` — every warning `knx-report` found, not just a
/// count, so nothing it reported is silently dropped on the way to the
/// terminal.
fn print_doc_export_report(output: &str, report: &knx_report::HtmlReport) {
    println!("exported documentation to {output}");
    println!("  {} warning(s)", report.warnings.len());
    for warning in &report.warnings {
        println!("  warning: {}: {}", warning.location, warning.detail);
    }
}

struct DiffArgs {
    a: String,
    b: String,
    exit_code: bool,
}

fn parse_diff_args(args: &[String]) -> Result<DiffArgs, String> {
    let mut a = None;
    let mut b = None;
    let mut exit_code = false;
    for arg in args {
        match arg.as_str() {
            "--exit-code" => exit_code = true,
            other if other.starts_with("--") => {
                return Err(format!("unknown flag: {other}"));
            }
            other if a.is_none() => a = Some(other.to_string()),
            other if b.is_none() => b = Some(other.to_string()),
            other => return Err(format!("unexpected extra argument: {other}")),
        }
    }
    let a = a.ok_or_else(|| "missing <a.knxdb|a.knxproj>".to_string())?;
    let b = b.ok_or_else(|| "missing <b.knxdb|b.knxproj>".to_string())?;
    Ok(DiffArgs { a, b, exit_code })
}

/// `knx diff` — computes and prints "what changed" between two native stores,
/// two raw ETS archives, or one of each. Unlike the server's
/// `POST /api/project/diff`, both sides here are files and are normalized by
/// `knx_app::comparison` before the pure diff receives them.
///
/// Both paths are checked with `Path::exists` *before* either store is
/// opened — this command's own new call site for the gotcha design spec §7
/// names by name: `open_and_migrate` "opens, creating if absent", so a
/// typo'd path would otherwise silently become an empty, freshly created
/// `.knxdb`, and the diff would then dutifully report every entity on the
/// other side as removed instead of failing with a clear "not found".
fn run_diff(args: &[String]) -> ExitCode {
    let parsed = match parse_diff_args(args) {
        Ok(parsed) => parsed,
        Err(e) => {
            eprintln!("{e}\n{USAGE}");
            return if args.iter().any(|arg| arg == "--exit-code") {
                ExitCode::from(2)
            } else {
                ExitCode::FAILURE
            };
        }
    };
    let input_error = || {
        if parsed.exit_code {
            ExitCode::from(2)
        } else {
            ExitCode::FAILURE
        }
    };

    if !Path::new(&parsed.a).exists() {
        eprintln!("store not found: {}", parsed.a);
        return input_error();
    }
    if !Path::new(&parsed.b).exists() {
        eprintln!("store not found: {}", parsed.b);
        return input_error();
    }

    let input_a = match knx_app::comparison::load_comparison_input(Path::new(&parsed.a)) {
        Ok(input) => input,
        Err(error) => {
            eprintln!("{error}");
            return input_error();
        }
    };
    let input_b = match knx_app::comparison::load_comparison_input(Path::new(&parsed.b)) {
        Ok(input) => input,
        Err(error) => {
            eprintln!("{error}");
            return input_error();
        }
    };

    if let Some(report) = &input_a.import_report {
        eprintln!(
            "comparison import report for {}: {}",
            parsed.a,
            report.to_json()
        );
    }
    if let Some(report) = &input_b.import_report {
        eprintln!(
            "comparison import report for {}: {}",
            parsed.b,
            report.to_json()
        );
    }

    let import_error_count = input_a
        .import_report
        .iter()
        .chain(input_b.import_report.iter())
        .map(error_count)
        .sum::<usize>();
    if import_error_count > 0 {
        eprintln!(
            "comparison aborted: {import_error_count} error diagnostics across the normalized ETS input(s)"
        );
        return input_error();
    }

    let diff = knx_diff::diff_projects(&input_a.project, &input_b.project);
    print_diff_report(&diff);
    if parsed.exit_code && !diff.is_empty() {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

// ---------------------------------------------------------------------
// `knx diff` rendering — plain text, `+`/`-`/`~`/`?` prefixed lines (design
// spec §5). Each `*Key` prints its own natural, human-readable form rather
// than a `Debug` dump: an `AreaKey` prints its address, a `LineKey` prints
// "area.line", a `BuildingPartKey` prints its path joined with `/`, a
// `GroupRangeKey` prints "start-end", a `GroupAddressKey` prints the
// address exactly as `knx-diff` already formatted it per the project's own
// `GroupAddressStyle`, and a `DeviceKey` prints its individual address, or
// its ETS id when it has none (design spec §6's `DeviceKey` shape).
// ---------------------------------------------------------------------

fn area_key_display(key: &knx_diff::AreaKey) -> String {
    key.address.to_string()
}

fn line_key_display(key: &knx_diff::LineKey) -> String {
    format!("{}.{}", key.area_address, key.line_address)
}

fn group_range_key_display(key: &knx_diff::GroupRangeKey) -> String {
    format!("{}-{}", key.start, key.end)
}

fn group_address_key_display(key: &knx_diff::GroupAddressKey) -> String {
    key.address.clone()
}

fn building_part_key_display(key: &knx_diff::BuildingPartKey) -> String {
    key.path.join("/")
}

fn device_key_display(key: &knx_diff::DeviceKey) -> String {
    match &key.address {
        Some(address) => address.clone(),
        None => key.ets_id.clone().unwrap_or_else(|| "-".to_string()),
    }
}

fn com_object_key_display(key: &knx_diff::ComObjectKey) -> String {
    key.number.to_string()
}

fn parameter_key_display(key: &knx_diff::ParameterKey) -> String {
    key.ets_id.clone()
}

/// Prints one `+`/`-`/`~`/`?` line per `added`/`removed`/`changed`/
/// `ambiguous` entry of a generic `EntityTable<K, F>`, in that order — the
/// "four-line shape" every entity kind but devices shares. `indent` is `""`
/// at the top level, `"  "` for a device's nested `com_objects`/
/// `parameters` tables.
fn print_entity_table<K, F: knx_diff::FieldDiff>(
    table: &knx_diff::EntityTable<K, F>,
    label: &str,
    indent: &str,
    key_display: impl Fn(&K) -> String,
) {
    for (key, _) in &table.added {
        println!("{indent}+ {label} {}", key_display(key));
    }
    for (key, _) in &table.removed {
        println!("{indent}- {label} {}", key_display(key));
    }
    for change in &table.changed {
        for field in change.field_changes() {
            println!(
                "{indent}~ {label} {}: {}: {} -> {}",
                key_display(&change.key),
                field.field,
                field.left,
                field.right
            );
        }
    }
    for note in &table.ambiguous {
        println!(
            "{indent}? {label} {}: {} left candidate(s), {} right candidate(s)",
            key_display(&note.key),
            note.left_candidates,
            note.right_candidates
        );
    }
}

/// Devices' own printer: same four-line shape as [`print_entity_table`],
/// except a `~ device` line additionally prints, indented two spaces, one
/// line per changed communication object and parameter — including when
/// `changed_fields` is itself empty (`"(own fields unchanged)"`), so a
/// device whose only change is a nested communication object or parameter
/// is never silently dropped (orchestrator ruling 1).
fn print_device_table(table: &knx_diff::DeviceTable) {
    for (key, _) in &table.added {
        println!("+ device {}", device_key_display(key));
    }
    for (key, _) in &table.removed {
        println!("- device {}", device_key_display(key));
    }
    for change in &table.changed {
        let own_field_changes = change.field_changes();
        if own_field_changes.is_empty() {
            println!(
                "~ device {}: (own fields unchanged)",
                device_key_display(&change.key)
            );
        } else {
            for field in own_field_changes {
                println!(
                    "~ device {}: {}: {} -> {}",
                    device_key_display(&change.key),
                    field.field,
                    field.left,
                    field.right
                );
            }
        }
        print_entity_table(
            &change.com_objects,
            "communication object",
            "  ",
            com_object_key_display,
        );
        print_entity_table(&change.parameters, "parameter", "  ", parameter_key_display);
    }
    for note in &table.ambiguous {
        println!(
            "? device {}: {} left candidate(s), {} right candidate(s)",
            device_key_display(&note.key),
            note.left_candidates,
            note.right_candidates
        );
    }
}

fn print_field_change(label: &str, change: &knx_diff::FieldChange) {
    println!(
        "~ {label}: {}: {} -> {}",
        change.field, change.left, change.right
    );
}

fn print_installation_diff(installation: &knx_diff::InstallationDiff) {
    match installation.status {
        knx_diff::EntityStatus::Added => println!("+ installation {}", installation.id),
        knx_diff::EntityStatus::Removed => println!("- installation {}", installation.id),
        knx_diff::EntityStatus::Matched => {}
    }
    for change in &installation.field_changes {
        print_field_change(&format!("installation {}", installation.id), change);
    }

    print_entity_table(&installation.areas, "area", "", area_key_display);
    print_entity_table(&installation.lines, "line", "", line_key_display);
    print_entity_table(
        &installation.group_ranges,
        "group range",
        "",
        group_range_key_display,
    );
    print_entity_table(
        &installation.group_addresses,
        "group address",
        "",
        group_address_key_display,
    );
    print_entity_table(
        &installation.buildings,
        "building",
        "",
        building_part_key_display,
    );
    print_device_table(&installation.devices);
}

/// Prints a `ProjectDiff` as plain text (design spec §5): `"no differences
/// found"` when nothing differs anywhere, otherwise `info_changes`, then
/// each installation's own status/field changes and entity tables in turn.
fn print_diff_report(diff: &knx_diff::ProjectDiff) {
    if diff.is_empty() {
        println!("no differences found");
        return;
    }

    for change in &diff.info_changes {
        print_field_change("project info", change);
    }

    for installation in &diff.installations {
        print_installation_diff(installation);
    }
}

struct GaImportArgs {
    store: String,
    input: String,
    dry_run: bool,
    confirmation_token: Option<String>,
}

fn parse_ga_import_args(args: &[String]) -> Result<GaImportArgs, String> {
    let mut store = None;
    let mut input = None;
    let mut dry_run = false;
    let mut confirmation_token = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--dry-run" => dry_run = true,
            "--confirm" => {
                i += 1;
                confirmation_token = Some(take_value(args, i, "--confirm")?);
            }
            other if other.starts_with("--") => {
                return Err(format!("unknown flag: {other}"));
            }
            other if store.is_none() => store = Some(other.to_string()),
            other if input.is_none() => input = Some(other.to_string()),
            other => return Err(format!("unexpected extra argument: {other}")),
        }
        i += 1;
    }
    let store = store.ok_or_else(|| "missing <store.knxdb>".to_string())?;
    let input = input.ok_or_else(|| "missing <in.csv>".to_string())?;
    Ok(GaImportArgs {
        store,
        input,
        dry_run,
        confirmation_token,
    })
}

/// `knx ga-import` — reads `in.csv` as "KNXBench group-address CSV v1"
/// text, plans the edit against the store's project (design §4), and,
/// unless any row is a row-level error, applies the single resulting
/// `Command::Batch` and saves the store. `--dry-run` runs the identical
/// plan and prints the identical report but returns before the `apply`/
/// `save_project` calls below, so the store is provably untouched — the
/// printed report is built solely from `plan.report`, never from whether
/// the save happened, so it is byte-identical either way. A single
/// trailing `store written: yes|no (reason)` line is appended *after*
/// that report body on every terminal path, so it can never make the
/// report itself diverge between a dry run and a real one, and a reader
/// never has to infer from the exit code or counts alone whether a save
/// actually happened.
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

    let mut project = match knx_store::load_project_reporting(&conn) {
        Ok((project, repair)) => {
            // ADR-0039 D7: a repair is never silent. `ga-import` is the one
            // CLI command that writes the file back, so the user learns why
            // the saved counters differ from what was on disk.
            if let Some(repair) = repair {
                eprintln!(
                    "warning: raised a stored id counter that was below an id already in use \
                     (stored {:?}, repaired {:?}); no project content was changed",
                    repair.stored, repair.repaired
                );
            }
            project
        }
        Err(e) => {
            eprintln!("failed to load project from store: {e}");
            return ExitCode::FAILURE;
        }
    };

    // A semantic snapshot catches changes committed through SQLite WAL as
    // well as main-file changes, and ignores SQLite bookkeeping bytes that
    // do not alter the project the user previewed.
    let expected_project = project.clone();
    let project_snapshot = format!("{project:?}");

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
        println!("store written: no (rejected)");
        return ExitCode::from(EXIT_IMPORTED_WITH_ERRORS);
    }

    if !plan.report.destructive_changes.is_empty() {
        let token = ga_import_confirmation_token(project_snapshot.as_bytes(), text.as_bytes());
        println!("confirmation token: {token}");
        if parsed.dry_run || parsed.confirmation_token.is_none() {
            println!("store written: no (destructive confirmation required)");
            return ExitCode::SUCCESS;
        }
        if parsed.confirmation_token.as_deref() != Some(token.as_str()) {
            eprintln!("confirmation token is stale or does not match this project and CSV");
            println!("store written: no (stale confirmation)");
            return ExitCode::FAILURE;
        }
    }

    if parsed.dry_run {
        println!("store written: no (dry run)");
        return ExitCode::SUCCESS;
    }

    match plan.command {
        Some(command) => {
            if let Err(e) = command.apply(&mut project) {
                eprintln!("failed to apply import: {e}");
                println!("store written: no (error)");
                return ExitCode::FAILURE;
            }
            if let Err(e) = knx_store::save_project_if_unchanged(&conn, &expected_project, &project)
            {
                eprintln!("failed to save project to store: {e}");
                let status = if matches!(e, knx_store::StoreError::ConcurrentModification) {
                    "stale confirmation"
                } else {
                    "error"
                };
                println!("store written: no ({status})");
                return ExitCode::FAILURE;
            }
            println!("store written: yes");
        }
        None => {
            // No row created or updated anything (the report's own "nothing
            // to do" line already said so) — there is nothing to apply, so
            // `save_project` is never called.
            println!("store written: no (nothing to do)");
        }
    }

    ExitCode::SUCCESS
}

fn ga_import_confirmation_token(project: &[u8], csv: &[u8]) -> String {
    use sha2::{Digest as _, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(b"knxbench-ga-import-v1\0");
    hasher.update((project.len() as u64).to_le_bytes());
    hasher.update(project);
    hasher.update(csv);
    format!("{:x}", hasher.finalize())
}

/// Prints an import report as human-readable lines (design §7). Shared
/// verbatim between a real import and `--dry-run` — see `run_ga_import` —
/// so the two can never drift apart.
fn print_import_report(input: &str, report: &knx_csv::CsvImportReport) {
    println!("imported {input}");
    println!(
        "  {} row(s) read, {} created, {} updated, {} readdressed, {} deleted, {} unchanged",
        report.rows_read,
        report.created,
        report.updated,
        report.readdressed,
        report.deleted,
        report.unchanged
    );
    if report.created == 0 && report.updated == 0 && report.readdressed == 0 && report.deleted == 0
    {
        println!("  nothing to do");
    }
    for change in &report.destructive_changes {
        let target = change
            .target_address
            .map(|address| format!(" -> {}", address.raw()))
            .unwrap_or_default();
        println!(
            "  {:?} row {}: {}{} (stable id {}, {} affected link(s))",
            change.action,
            change.row,
            change.source_address.raw(),
            target,
            change.id.0,
            change.affected_links.len()
        );
    }
    for ignored in &report.ignored_columns {
        let reason = match ignored.reason {
            knx_csv::IgnoredColumnReason::ReadOnly => {
                "read-only validation column, never applied on import"
            }
            knx_csv::IgnoredColumnReason::Unknown => "unrecognized column",
        };
        println!("  ignored column '{}' ({reason})", ignored.name);
    }
    for problem in &report.problems {
        print_csv_problem(problem);
    }
}

/// `knx products list|ingest|show|verify|identity|family|order-number` —
/// inspection and separate ingest of the shared product database (spec §8,
/// ADR-0043).
fn run_products(args: &[String]) -> ExitCode {
    match args.first().map(String::as_str) {
        Some("list") => run_products_list(&args[1..]),
        Some("ingest") => run_products_ingest(&args[1..]),
        Some("show") => run_products_show(&args[1..]),
        Some("verify") => run_products_verify(&args[1..]),
        Some("identity") => run_products_identity(&args[1..]),
        Some("family") => run_products_family(&args[1..]),
        Some("order-number") => run_products_order_number(&args[1..]),
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

fn install_facts_json(facts: Option<&knx_productdb::InstallFacts>) -> serde_json::Value {
    let Some(facts) = facts else {
        return serde_json::json!({ "status": "unavailable", "facts": null });
    };
    serde_json::json!({
        "status": "measured",
        "facts": {
            "counts": facts.counts.iter().map(|row| serde_json::json!({
                "category": row.category.as_str(),
                "disposition": row.disposition.as_str(),
                "count": row.count,
            })).collect::<Vec<_>>(),
            "unknownConstructs": facts.unknown_constructs.iter().map(|unknown| serde_json::json!({
                "xpath": unknown.xpath,
                "kind": unknown.kind.as_str(),
                "name": unknown.name,
                "occurrences": unknown.occurrences,
                "sample": unknown.sample,
            })).collect::<Vec<_>>(),
            "unknownOccurrences": facts.unknown_occurrences,
            "diagnostics": facts.diagnostics.iter().map(|diagnostic| serde_json::json!({
                "kind": diagnostic.kind().as_str(),
                "archivePath": diagnostic.archive_path(),
                "xmlPath": diagnostic.xml_path(),
                "occurrences": diagnostic.occurrences(),
                "detail": diagnostic.detail(),
            })).collect::<Vec<_>>(),
        },
    })
}

fn print_install_facts(facts: Option<&knx_productdb::InstallFacts>) {
    if facts.is_some() {
        println!("facts: measured");
    } else {
        println!("facts: unavailable (historical install; measured encounter data was not stored)");
    }
    println!(
        "install_facts_json {}",
        serde_json::to_string(&install_facts_json(facts))
            .expect("install facts JSON is serializable")
    );
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
                    "package installed: scheme {}, {} member(s), {} unknown construct(s), {} conflict(s), \
                     {} translation(s) captured (program {}, catalog {}, hardware {}, master {}), \
                     {} datapoint type(s) dropped as duplicates{}",
                    report.scheme,
                    report.members.len(),
                    report.unknown,
                    report.conflicts.len(),
                    report.translations.total(),
                    report.translations.program,
                    report.translations.catalog,
                    report.translations.hardware,
                    report.translations.master,
                    report.dropped_datapoint_types,
                    if report.skipped {
                        format!(
                            " (already known; {} source name(s))",
                            report.source_names.len()
                        )
                    } else {
                        String::new()
                    },
                );
                print_install_facts(report.facts.as_ref());
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
    println!("facts: not applicable (single-file project ingest; package facts are only measured for standalone package installs)");
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
    let metadata = [
        ("IsSecureEnabled", program.is_secure_enabled.as_deref()),
        (
            "MaxSecurityGroupKeyTableEntries",
            program.max_security_group_key_table_entries.as_deref(),
        ),
        (
            "MaxSecurityIndividualAddressEntries",
            program.max_security_individual_address_entries.as_deref(),
        ),
        (
            "MaxSecurityP2PKeyTableEntries",
            program.max_security_p2p_key_table_entries.as_deref(),
        ),
        (
            "MaxTunnelingUserEntries",
            program.max_tunneling_user_entries.as_deref(),
        ),
        ("MaxUserEntries", program.max_user_entries.as_deref()),
        ("MinEtsVersion", program.min_ets_version.as_deref()),
        ("ReplacesVersions", program.replaces_versions.as_deref()),
    ];
    if metadata.iter().any(|(_, value)| value.is_some()) {
        println!("  source catalogue metadata (uninterpreted; not runtime security support):");
        for (name, value) in metadata {
            if let Some(value) = value {
                println!("    {name}: {value}");
            }
        }
    }
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

/// Opens the product database and splits exactly `count` positionals, or
/// prints the usage error. Shared by the PDB-11 inspection commands.
fn products_positionals(
    args: &[String],
    count: usize,
    missing: &str,
) -> Result<(knx_productdb::Connection, Vec<String>), ExitCode> {
    let (product_db, rest) = split_product_db_flag(args).map_err(|e| {
        eprintln!("{e}\n{USAGE}");
        ExitCode::FAILURE
    })?;
    if rest.len() != count {
        eprintln!("{missing}\n{USAGE}");
        return Err(ExitCode::FAILURE);
    }
    let conn = open_products_db(product_db.as_deref()).map_err(|e| {
        eprintln!("{e}");
        ExitCode::FAILURE
    })?;
    Ok((conn, rest))
}

/// `knx products identity <table> <id>`: the winner and every recorded
/// candidate of one package-content id (ADR-0043).
fn run_products_identity(args: &[String]) -> ExitCode {
    let (conn, rest) = match products_positionals(args, 2, "expected <table> <id>") {
        Ok(v) => v,
        Err(code) => return code,
    };
    let Some(kind) = knx_productdb::IdentityKind::parse(&rest[0]) else {
        let tables: Vec<_> = knx_productdb::IdentityKind::ALL
            .iter()
            .map(|kind| kind.as_table())
            .collect();
        eprintln!(
            "unknown identity table {:?}; expected one of {}",
            rest[0],
            tables.join(", ")
        );
        return ExitCode::FAILURE;
    };
    let report = match knx_productdb::identity_candidates(&conn, kind, &rest[1]) {
        Ok(report) => report,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    println!("{} {}", kind.as_table(), report.logical_id);
    println!(
        "  winner: {}",
        report.winner.as_deref().unwrap_or("(no stored row)")
    );
    println!("  {} candidate(s)", report.candidates.len());
    for candidate in &report.candidates {
        println!(
            "    blob {}  occurrence {}  digest {}  {}  {} package(s)  {}",
            candidate.source_sha256,
            candidate.occurrence,
            &candidate.digest[..16],
            match candidate.same_as_winner {
                Some(true) => "same",
                Some(false) => "differs",
                None => "unknown",
            },
            candidate.packages.len(),
            candidate.source_path,
        );
    }
    for unmeasured in &report.unmeasured {
        println!(
            "  unmeasured blob {}: {}",
            unmeasured.source_sha256, unmeasured.reason
        );
    }
    ExitCode::SUCCESS
}

/// `knx products family <program-id>`: programs sharing its manufacturer
/// and parsed `ApplicationNumber`, with `ReplacesVersions` resolved.
fn run_products_family(args: &[String]) -> ExitCode {
    let (conn, rest) = match products_positionals(args, 1, "expected <program-id>") {
        Ok(v) => v,
        Err(code) => return code,
    };
    let family = match knx_productdb::program_family(&conn, &rest[0]) {
        Ok(Some(family)) => family,
        Ok(None) => {
            eprintln!("no such program: {}", rest[0]);
            return ExitCode::FAILURE;
        }
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    match &family.key {
        knx_productdb::FamilyKey::Number(number) => println!(
            "{}  family {} ApplicationNumber {number}: {} member(s)",
            family.program_id,
            family.manufacturer_id,
            family.members.len()
        ),
        knx_productdb::FamilyKey::Unparsed { raw, reason } => println!(
            "{}  no family: ApplicationNumber {} ({reason})",
            family.program_id,
            raw.as_deref()
                .map_or("absent".to_string(), |raw| format!("{raw:?}")),
        ),
    }
    for member in &family.members {
        let version = match (member.parsed_version, member.application_version.as_deref()) {
            (Some(version), _) => version.to_string(),
            (None, Some(raw)) => format!("{raw:?} (unparsed)"),
            (None, None) => "absent".to_string(),
        };
        println!("  {}  version {version}", member.program_id);
        match &member.replaces {
            knx_productdb::ReplacesVersions::Absent => {}
            knx_productdb::ReplacesVersions::Unparsed { raw, reason } => {
                println!("    ReplacesVersions {raw:?}: not linked ({reason})")
            }
            knx_productdb::ReplacesVersions::Parsed { raw, entries } => {
                println!("    ReplacesVersions {raw:?}");
                for (version, programs) in entries {
                    if programs.is_empty() {
                        println!("      {version}: not installed");
                    } else {
                        println!("      {version}: {}", programs.join(", "));
                    }
                }
            }
        }
    }
    ExitCode::SUCCESS
}

/// `knx products order-number <manufacturer-id> <order-number>`: every
/// winning product with exactly that order number. Never merges them.
fn run_products_order_number(args: &[String]) -> ExitCode {
    let (conn, rest) =
        match products_positionals(args, 2, "expected <manufacturer-id> <order-number>") {
            Ok(v) => v,
            Err(code) => return code,
        };
    let products = match knx_productdb::products_by_order_number(&conn, &rest[0], &rest[1]) {
        Ok(products) => products,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    println!(
        "{} product(s) with order number {:?}",
        products.len(),
        rest[1]
    );
    for product in &products {
        println!(
            "  {}  {}  hardware {}",
            product.product_id,
            product.text.as_deref().unwrap_or(""),
            product.hardware_id
        );
        println!(
            "    programs: {}",
            if product.programs.is_empty() {
                "(none)".to_string()
            } else {
                product.programs.join(", ")
            }
        );
        let schemes: Vec<_> = product.schemes.iter().map(u32::to_string).collect();
        println!(
            "    source blob {}  package scheme(s): {}",
            product.source_sha256,
            if schemes.is_empty() {
                "(no package)".to_string()
            } else {
                schemes.join(", ")
            }
        );
    }
    ExitCode::SUCCESS
}

/// `knx bus monitor` — connects to a real KNXnet/IP gateway over tunnelling
/// and prints decoded telegrams as they arrive (spec §9, Task 9).
fn run_device(args: &[String]) -> ExitCode {
    match args.first().map(String::as_str) {
        Some("download") => run_device_download(&args[1..]),
        _ => {
            eprintln!("{USAGE}");
            ExitCode::FAILURE
        }
    }
}

/// `knx device download`. Order matters: address, exclusion list and
/// phrase are checked first; then the project and product database are
/// read and the plan is built; only then, and only with the phrase, does a
/// socket open.
fn run_device_download(args: &[String]) -> ExitCode {
    let parsed = match device_download::parse_download_args(args) {
        Ok(parsed) => parsed,
        Err(e) => {
            eprintln!("{e}\n{USAGE}");
            return ExitCode::FAILURE;
        }
    };
    let (target, mode) = match device_download::check_target(&parsed) {
        Ok(checked) => checked,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    // `open_and_migrate` creates a missing file; a typo must not become an
    // empty project.
    if !Path::new(&parsed.project).exists() {
        eprintln!("project not found: {}", parsed.project);
        return ExitCode::FAILURE;
    }
    let project = match knx_store::open_and_migrate(Path::new(&parsed.project))
        .map_err(|e| e.to_string())
        .and_then(|conn| knx_store::load_project(&conn).map_err(|e| e.to_string()))
    {
        Ok(project) => project,
        Err(e) => {
            eprintln!("could not read project {}: {e}", parsed.project);
            return ExitCode::FAILURE;
        }
    };
    let products = match open_products_db(parsed.product_db.as_deref()) {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    let prepared = match knx_app::device_download::prepare_device_download(
        &products,
        &project,
        target.address(),
    ) {
        Ok(prepared) => prepared,
        Err(e) => {
            eprintln!("no download to device {} prepared: {e}", target.address());
            return ExitCode::FAILURE;
        }
    };
    print!("{}", device_download::format_plan(&prepared));

    let (gateway, confirmation) = match mode {
        device_download::Mode::Plan => {
            println!(
                "written to the device: no (plan only; add --gateway and --confirm {:?} to write)",
                knx_core::commissioning::mutation::required_confirmation_phrase(
                    target.address(),
                    knx_core::WriteScope::Download
                )
            );
            return ExitCode::SUCCESS;
        }
        device_download::Mode::Write {
            gateway,
            confirmation,
        } => (gateway, confirmation),
    };
    let authorisation = match knx_core::WriteAuthorisation::for_hardware(
        target.address(),
        knx_core::WriteScope::Download,
        &confirmation,
    ) {
        Ok(authorisation) => authorisation,
        Err(e) => {
            eprintln!("{e}");
            println!("written to the device: no");
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
    runtime.block_on(async {
        use knx_net::BusConnection;
        let tunnel = match knx_net::KnxNetIpClient::new().connect_tunnel(gateway).await {
            Ok(tunnel) => tunnel,
            Err(e) => {
                eprintln!("could not connect to {gateway}: {e}");
                println!("written to the device: no");
                return ExitCode::FAILURE;
            }
        };
        let written = device_download::execute(
            &tunnel,
            authorisation,
            knx_net::SessionTiming::default(),
            &prepared,
            &mut std::io::stdout(),
        )
        .await;
        if let Err(e) = tunnel.disconnect().await {
            eprintln!("tunnel disconnect: {e}");
        }
        match written {
            device_download::Written::Yes => ExitCode::SUCCESS,
            device_download::Written::No | device_download::Written::Partially => ExitCode::FAILURE,
        }
    })
}

fn run_bus(args: &[String]) -> ExitCode {
    match args.first().map(String::as_str) {
        Some("discover") => run_bus_discover(&args[1..]),
        Some("monitor") => run_bus_monitor(&args[1..]),
        Some("write") => run_bus_write(&args[1..]),
        Some("route-monitor") => run_bus_route_monitor(&args[1..]),
        Some("route-send") => run_bus_route_send(&args[1..]),
        Some("scan") => run_bus_scan(&args[1..]),
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

/// Printed on stderr whenever `discover()` comes back empty. An empty
/// result and a broken deployment look identical otherwise (GAP_ANALYSIS_ETS.md
/// row E5): discovery only works if `SEARCH_REQUEST`'s multicast datagram
/// actually leaves the machine, which it does not on Docker's default
/// bridge network (see the Dockerfile and README for `--network host`).
/// Written unconditionally, not gated on a "looks like a container" check
/// — there is no such check that would not also misfire on an ordinary
/// host with a firewalled or wrong-interface multicast route, and an empty
/// result there deserves the same hint.
const DISCOVER_EMPTY_HINT: &str = "hint: KNXnet/IP discovery depends on IP multicast \
reaching this network segment; an empty result can mean no gateway answered, or that \
the SEARCH_REQUEST never got out — a common cause is running inside a container \
(e.g. Docker's default bridge network) without host networking";

async fn run_bus_discover_async() -> ExitCode {
    use knx_net::BusConnection;
    let client = knx_net::KnxNetIpClient::new();
    match client.discover().await {
        Ok(gateways) if gateways.is_empty() => {
            println!("no gateways responded");
            eprintln!("{DISCOVER_EMPTY_HINT}");
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
    // `None` (not `Some(empty map)`) means "no --project", so `format_telegram`
    // can tell "nothing resolved" from "resolution was never attempted" and
    // stay byte-identical to today's output when the caller passed no
    // `--project` at all (spec E4-D8).
    let ga_dpts: Option<std::collections::HashMap<u16, knx_core::GroupAddressDpt>> =
        match &parsed.project {
            Some(path) => match load_group_address_dpts(Path::new(path)) {
                Ok(dpts) => Some(dpts),
                Err(e) => {
                    eprintln!("could not load project {path}: {e}");
                    return ExitCode::FAILURE;
                }
            },
            None => None,
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

    runtime.block_on(run_bus_monitor_async(gateway, ga_names, ga_dpts))
}

async fn run_bus_monitor_async(
    gateway: std::net::SocketAddrV4,
    ga_names: std::collections::HashMap<u16, String>,
    ga_dpts: Option<std::collections::HashMap<u16, knx_core::GroupAddressDpt>>,
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
                    println!("{}", format_telegram(&telegram, &ga_names, ga_dpts.as_ref()));
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
    project: Option<String>,
    dpt: Option<String>,
    input_format: Option<knx_core::DptInputFormat>,
    dry_run: bool,
    group_address: String,
    value: String,
}

fn parse_bus_write_args(args: &[String]) -> Result<BusWriteArgs, String> {
    let mut gateway = None;
    let mut project = None;
    let mut dpt = None;
    let mut input_format = None;
    let mut dry_run = false;
    let mut positional = Vec::new();
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
            "--dpt" => {
                dpt = Some(take_value(args, i + 1, "--dpt")?);
                i += 2;
            }
            "--input-format" => {
                let value = take_value(args, i + 1, "--input-format")?;
                input_format = Some(knx_core::DptInputFormat::parse_name(&value).ok_or_else(
                    || {
                        format!(
                            "unknown input format {value:?}; expected canonical, decimal, hexadecimal, binary, or text"
                        )
                    },
                )?);
                i += 2;
            }
            "--dry-run" => {
                dry_run = true;
                i += 1;
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
        project,
        dpt,
        input_format,
        dry_run,
        group_address: group_address.clone(),
        value: value.clone(),
    })
}

/// Parses a `GroupValueWrite` payload as raw wire bytes, with no DPT
/// interpretation: `0`/`1` is a 6-bit inline value (e.g. DPT-1), anything
/// else is read as a hex byte string (optionally `0x`-prefixed, e.g.
/// `2a99`). This used to be `bus write`'s only value grammar; spec E4-D7
/// keeps it as the explicit fallback for when the caller gives neither
/// `--dpt` nor `--project` (see `resolve_write_value`) — not because raw
/// values stopped mattering, but because a bus without a loaded project or
/// a known DPT has nothing more informative to parse against.
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

/// Picks the encode path per spec E4-D7 and returns the label the
/// `--dry-run` line and any error should show alongside the payload: the
/// `DptRef` actually used, or `"raw"` for the unchanged fallback. Never
/// falls back silently — a `None`/`Conflict` resolution is an error that
/// names what was found and tells the user to pass `--dpt`, because
/// reinterpreting the user's value as raw would be exactly the kind of
/// guess CLAUDE.md and spec E4-D5 forbid.
fn resolve_write_value(
    parsed: &BusWriteArgs,
    ga: knx_core::GroupAddress,
) -> Result<(String, knx_net::GroupValue), String> {
    if let Some(dpt_str) = &parsed.dpt {
        let dpt = knx_core::DptRef::parse(dpt_str).map_err(|e| e.to_string())?;
        let value = match parsed.input_format {
            Some(format) => knx_core::encode(dpt, &parsed.value, format),
            None => knx_core::encode_inferred_format(dpt, &parsed.value),
        }
        .map_err(|e| e.to_string())?;
        return Ok((dpt.to_string(), value));
    }
    if let Some(project_path) = &parsed.project {
        let dpts = load_group_address_dpts(Path::new(project_path))
            .map_err(|e| format!("could not load project {project_path}: {e}"))?;
        let formatted = ga.format(knx_core::GroupAddressStyle::ThreeLevel);
        return match dpts.get(&ga.raw()) {
            None => Err(format!(
                "no datapoint type resolved for group address {formatted}; pass --dpt"
            )),
            Some(knx_core::GroupAddressDpt::Conflict(dpts)) => {
                let names = dpts
                    .iter()
                    .map(|d| d.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                Err(format!(
                    "group address {formatted} has conflicting datapoint types ({names}); pass --dpt"
                ))
            }
            Some(knx_core::GroupAddressDpt::Single(dpt)) => {
                let value = match parsed.input_format {
                    Some(format) => knx_core::encode(*dpt, &parsed.value, format),
                    None => knx_core::encode_inferred_format(*dpt, &parsed.value),
                }
                .map_err(|e| e.to_string())?;
                Ok((dpt.to_string(), value))
            }
            // `load_group_address_dpts` never stores `None` — a missing key
            // means the same thing, and is handled above.
            Some(knx_core::GroupAddressDpt::None) => Err(format!(
                "no datapoint type resolved for group address {formatted}; pass --dpt"
            )),
        };
    }
    if parsed.input_format.is_some() {
        return Err("--input-format requires --dpt or --project".to_string());
    }
    let value = parse_group_value(&parsed.value)?;
    Ok(("raw".to_string(), value))
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
    // Everything — address, project, DPT, value — is parsed and validated
    // here, before a socket is ever opened in any path.
    let (dpt_label, value) = match resolve_write_value(&parsed, group_address) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };

    if parsed.dry_run {
        println!(
            "{} {dpt_label} {} -> {}",
            group_address.format(knx_core::GroupAddressStyle::ThreeLevel),
            parsed.value,
            format_group_value_payload(&value),
        );
        return ExitCode::SUCCESS;
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
    multicast_group: Option<String>,
}

fn parse_bus_route_monitor_args(args: &[String]) -> Result<BusRouteMonitorArgs, String> {
    let mut source_address = None;
    let mut project = None;
    let mut multicast_group = None;
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
            "--multicast-group" => {
                multicast_group = Some(take_value(args, i + 1, "--multicast-group")?);
                i += 2;
            }
            other => return Err(format!("unrecognized argument: {other}")),
        }
    }
    Ok(BusRouteMonitorArgs {
        source_address: source_address.ok_or_else(|| "--source-address is required".to_string())?,
        project,
        multicast_group,
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
    let multicast_group: Option<std::net::Ipv4Addr> = match &parsed.multicast_group {
        Some(addr) => match addr.parse() {
            Ok(a) => Some(a),
            Err(_) => {
                eprintln!("--multicast-group must be an IPv4 address, e.g. 239.0.2.1");
                return ExitCode::FAILURE;
            }
        },
        None => None,
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
    runtime.block_on(run_bus_route_monitor_async(
        own_address,
        ga_names,
        multicast_group,
    ))
}

async fn run_bus_route_monitor_async(
    own_address: knx_core::IndividualAddress,
    ga_names: std::collections::HashMap<u16, String>,
    multicast_group: Option<std::net::Ipv4Addr>,
) -> ExitCode {
    use knx_net::BusConnection;
    let client = knx_net::KnxNetIpClient::new();
    let routing = match multicast_group {
        Some(group) => client.connect_routing_to_group(own_address, group).await,
        None => client.connect_routing(own_address).await,
    };
    let routing = match routing {
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
                // `route-monitor` does not resolve DPTs (out of scope for
                // T29, spec E4-D7/D8 name only `monitor`/`write`) — `None`
                // keeps its output exactly what it was before this task.
                Ok(telegram) => println!("{}", format_telegram(&telegram, &ga_names, None)),
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
    multicast_group: Option<String>,
}

fn parse_bus_route_send_args(args: &[String]) -> Result<BusRouteSendArgs, String> {
    let mut source_address = None;
    let mut multicast_group = None;
    let mut positional = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--source-address" => {
                source_address = Some(take_value(args, i + 1, "--source-address")?);
                i += 2;
            }
            "--multicast-group" => {
                multicast_group = Some(take_value(args, i + 1, "--multicast-group")?);
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
        multicast_group,
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
    let multicast_group: Option<std::net::Ipv4Addr> = match &parsed.multicast_group {
        Some(addr) => match addr.parse() {
            Ok(a) => Some(a),
            Err(_) => {
                eprintln!("--multicast-group must be an IPv4 address, e.g. 239.0.2.1");
                return ExitCode::FAILURE;
            }
        },
        None => None,
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
    runtime.block_on(run_bus_route_send_async(
        own_address,
        group_address,
        value,
        multicast_group,
    ))
}

async fn run_bus_route_send_async(
    own_address: knx_core::IndividualAddress,
    group_address: knx_core::GroupAddress,
    value: knx_net::GroupValue,
    multicast_group: Option<std::net::Ipv4Addr>,
) -> ExitCode {
    use knx_net::{ApplicationService, BusConnection, Destination};
    let client = knx_net::KnxNetIpClient::new();
    let routing = match multicast_group {
        Some(group) => client.connect_routing_to_group(own_address, group).await,
        None => client.connect_routing(own_address).await,
    };
    let routing = match routing {
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

fn run_bus_scan(args: &[String]) -> ExitCode {
    let parsed = match scan::parse_scan_args(args) {
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
    // Plan and policy are fully built — including the malformed-`--exclude`
    // and cross-line `--range` checks — before a socket is ever opened.
    let (plan, range, excluded) =
        match scan::build_scan_plan(&parsed.line, parsed.range.as_deref(), &parsed.exclude) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("{e}\n{USAGE}");
                return ExitCode::FAILURE;
            }
        };
    // `--dry-run` prints the plan's shape and stops here, before a probe
    // policy, a project file or a tokio runtime — let alone a socket —
    // ever exist. Modelled on `knx bus write --dry-run`.
    if parsed.dry_run {
        println!("{}", scan::format_dry_run(&plan, &excluded));
        return ExitCode::SUCCESS;
    }
    let policy =
        match scan::build_probe_policy(parsed.timeout_ms.as_deref(), parsed.pause_ms.as_deref()) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("{e}\n{USAGE}");
                return ExitCode::FAILURE;
            }
        };
    let project_addresses = match &parsed.project {
        Some(path) => match load_project_individual_addresses(Path::new(path)) {
            Ok(addrs) => Some(addrs),
            Err(e) => {
                eprintln!("could not load project {path}: {e}");
                return ExitCode::FAILURE;
            }
        },
        None => None,
    };
    let excluded_count = excluded.iter().filter(|a| range.contains(**a)).count();

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
    runtime.block_on(run_bus_scan_async(
        gateway,
        plan,
        policy,
        range,
        excluded,
        excluded_count,
        project_addresses,
    ))
}

/// Opens a tunnel, runs [`knx_net::scan_line`], and prints results as they
/// arrive, then a summary, then (with `--project`) a comparison. Per-address
/// round trip is [`scan::round_trip`] applied to the wall-clock gap between
/// successive `progress` calls — see that function's doc comment for
/// exactly what it derives and the one precondition it assumes.
#[allow(clippy::too_many_arguments)]
async fn run_bus_scan_async(
    gateway: std::net::SocketAddrV4,
    plan: knx_core::scan::ScanPlan,
    policy: knx_net::ProbePolicy,
    range: scan::ScannedRange,
    excluded: std::collections::HashSet<knx_core::IndividualAddress>,
    excluded_count: usize,
    project_addresses: Option<Vec<knx_core::IndividualAddress>>,
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
        "connected to {gateway}, assigned individual address {}. scanning {} candidate address(es)...",
        tunnel.assigned_address(),
        plan.addresses().len()
    );

    let start = std::time::Instant::now();
    let mut last_instant = start;
    let mut last_outcome: Option<knx_net::ProbeOutcome> = None;
    let pause = policy.inter_probe_pause();
    let scan_result = knx_net::scan_line(&tunnel, &plan, &policy, |addr, outcome| {
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(last_instant);
        let round_trip = scan::round_trip(elapsed, pause, last_outcome);
        println!("{}", scan::format_probe_line(addr, outcome, round_trip));
        last_instant = now;
        last_outcome = Some(outcome);
    })
    .await;
    let elapsed_total = start.elapsed();

    let (results, scan_ok) = match scan_result {
        Ok(results) => (results, true),
        Err(knx_net::ScanError::Transport { source, completed }) => {
            eprintln!(
                "scan transport failed after {} address(es): {source}",
                completed.len()
            );
            (completed, false)
        }
        Err(knx_net::ScanError::Plan(e)) => {
            eprintln!("scan plan is not safe to run: {e}");
            if let Err(e) = tunnel.disconnect().await {
                eprintln!("disconnect: {e}");
            }
            return ExitCode::FAILURE;
        }
    };

    if let Err(e) = tunnel.disconnect().await {
        eprintln!("disconnect: {e}");
    }

    let summary = scan::summarize(&results, excluded_count);
    println!("{}", scan::format_summary(&summary, elapsed_total, &policy));

    if let Some(project_addresses) = project_addresses {
        let comparison =
            scan::compare_with_project(&range, &results, &excluded, &project_addresses);
        println!("{}", scan::format_project_comparison(&comparison));
    }

    if scan_ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Loads every device's individual address from a stored project, for
/// `bus scan --project`'s comparison only. Read-only: the comparison is
/// printed, never written back into the project (this task discovers, it
/// does not reconcile).
///
/// Checks the path exists before calling `open_and_migrate`, which does
/// not: given a path that does not exist, it happily creates a fresh,
/// empty, freshly-migrated database and hands back a connection to it —
/// exactly the right behaviour for `import`/`ga-import`'s "just show me
/// the store" case, and exactly the wrong one here, where a typo'd
/// `--project` path would otherwise silently compare the scan against an
/// empty project instead of failing loudly.
fn load_project_individual_addresses(
    path: &Path,
) -> Result<Vec<knx_core::IndividualAddress>, String> {
    if !path.exists() {
        return Err(format!(
            "{} does not exist (refusing to create an empty project to compare against)",
            path.display()
        ));
    }
    let conn = knx_store::migration::open_and_migrate(path).map_err(|e| e.to_string())?;
    let project = knx_store::project::load_project(&conn).map_err(|e| e.to_string())?;
    Ok(project.devices.iter().filter_map(|d| d.address).collect())
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

/// Loads the group-address (raw 16-bit) -> resolved-DPT map for a stored
/// project, once, so `format_telegram` never touches the project again per
/// telegram (spec E4-D8). See `knx_core::resolve_project_group_address_dpts`
/// for what "resolved" means; a missing key is the common case, not this
/// function's problem to flag.
fn load_group_address_dpts(
    path: &Path,
) -> Result<std::collections::HashMap<u16, knx_core::GroupAddressDpt>, String> {
    let conn = knx_store::migration::open_and_migrate(path).map_err(|e| e.to_string())?;
    let project = knx_store::project::load_project(&conn).map_err(|e| e.to_string())?;
    Ok(knx_core::resolve_project_group_address_dpts(&project))
}

/// Renders a raw `GroupValue` the way it always has been rendered — a
/// six-bit inline value visibly distinct from one or more octets — so a
/// payload the codec could not (or was never asked to) turn into an
/// engineering value still prints something a human can read off the bus.
/// Shared between the monitor's undecoded fallback and `bus write
/// --dry-run`'s preview line.
fn format_group_value_payload(v: &knx_net::GroupValue) -> String {
    match v {
        knx_net::GroupValue::Short(bits) => format!("{bits:#04x} (6-bit)"),
        knx_net::GroupValue::Bytes(bytes) => format!("{bytes:02x?}"),
    }
}

/// What `format_telegram` knows about a telegram's destination DPT, folded
/// down from the `Option<&HashMap<..>>` / `Option<&GroupAddressDpt>` double
/// lookup into one match. `NotApplicable` covers both "no `--project` was
/// given at all" (the monitor must stay byte-identical to today, spec
/// E4-D8) and "this destination is not a group address" — a DPT is a group
/// address concept, so an individually-addressed telegram is never
/// annotated even if a project happens to be loaded.
enum DptAnnotation<'a> {
    NotApplicable,
    None,
    Single(&'a knx_core::DptRef),
    Conflict(&'a [knx_core::DptRef]),
}

fn format_telegram(
    telegram: &knx_net::LDataFrame,
    ga_names: &std::collections::HashMap<u16, String>,
    ga_dpts: Option<&std::collections::HashMap<u16, knx_core::GroupAddressDpt>>,
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
    let dpt = match (ga_dpts, telegram.destination) {
        (Some(map), Destination::Group(ga)) => match map.get(&ga.raw()) {
            None => DptAnnotation::None,
            Some(knx_core::GroupAddressDpt::None) => DptAnnotation::None,
            Some(knx_core::GroupAddressDpt::Single(dpt)) => DptAnnotation::Single(dpt),
            Some(knx_core::GroupAddressDpt::Conflict(dpts)) => DptAnnotation::Conflict(dpts),
        },
        _ => DptAnnotation::NotApplicable,
    };
    format!(
        "{} -> {dest}: {}",
        telegram.source,
        format_service(&telegram.service, &telegram.transport, dpt)
    )
}

fn format_service(
    service: &knx_net::ApplicationService,
    transport: &knx_net::Tpci,
    dpt: DptAnnotation,
) -> String {
    use knx_net::ApplicationService;
    match service {
        ApplicationService::GroupValueRead => "GroupValueRead".to_string(),
        ApplicationService::GroupValueResponse(v) => {
            format!("GroupValueResponse {}", format_decoded_value(v, dpt))
        }
        ApplicationService::GroupValueWrite(v) => {
            format!("GroupValueWrite {}", format_decoded_value(v, dpt))
        }
        ApplicationService::DeviceDescriptorRead { descriptor_type } => {
            format!("DeviceDescriptorRead type={descriptor_type}")
        }
        ApplicationService::DeviceDescriptorResponse {
            descriptor_type,
            data,
        } => {
            format!("DeviceDescriptorResponse type={descriptor_type} data={data:02x?}")
        }
        // The four control PDUs all carry `NoApplicationPdu` — rendering
        // the `Tpci` instead of the fixed string is the only way to tell
        // a `T_Connect` apart from a `T_Disconnect`/`T_ACK`/`T_NAK` in the
        // monitor (task-2 review finding 6).
        ApplicationService::NoApplicationPdu => format_tpci(transport),
        ApplicationService::Other { apci, data } => {
            format!("APCI {apci:#06x} data {data:02x?}")
        }
        // T30's management services (commissioning spec §6.6). Rendered
        // from the service's own `variant_name`/`payload_summary` rather
        // than fifteen near-identical `format!`s — but still listed one by
        // one instead of a catch-all, so a sixteenth service is a compile
        // error here and not a silently unlabelled monitor row.
        service @ (ApplicationService::IndividualAddressWrite { .. }
        | ApplicationService::IndividualAddressRead
        | ApplicationService::IndividualAddressResponse
        | ApplicationService::MemoryRead { .. }
        | ApplicationService::MemoryResponse { .. }
        | ApplicationService::MemoryWrite { .. }
        | ApplicationService::UserMemoryRead { .. }
        | ApplicationService::UserMemoryResponse { .. }
        | ApplicationService::UserMemoryWrite { .. }
        | ApplicationService::Restart { .. }
        | ApplicationService::AuthorizeRequest { .. }
        | ApplicationService::AuthorizeResponse { .. }
        | ApplicationService::PropertyValueRead { .. }
        | ApplicationService::PropertyValueResponse { .. }
        | ApplicationService::PropertyValueWrite { .. }) => match service.payload_summary() {
            Some(payload) => format!("{} {payload}", service.variant_name()),
            None => service.variant_name().to_string(),
        },
    }
}

/// Renders a `Tpci` for the monitor — only reached for `NoApplicationPdu`
/// rows today, but total over the enum so a future data-shaped caller
/// gets a sensible string too, not a compile error waiting to happen.
fn format_tpci(transport: &knx_net::Tpci) -> String {
    use knx_net::Tpci;
    match transport {
        Tpci::UnnumberedData => "UnnumberedData".to_string(),
        Tpci::NumberedData { seq } => format!("NumberedData seq={seq}"),
        Tpci::Connect => "T_Connect".to_string(),
        Tpci::Disconnect => "T_Disconnect".to_string(),
        Tpci::Ack { seq } => format!("T_ACK seq={seq}"),
        Tpci::Nak { seq } => format!("T_NAK seq={seq}"),
        Tpci::Unknown(octet) => format!("Unknown TPCI {octet:#04x}"),
    }
}

/// Renders one `GroupValueWrite`/`GroupValueResponse` payload: decoded
/// (`<DPST-m-s> <value>`) where `dpt` names exactly one type and decoding
/// succeeds, the raw payload plus a stated reason otherwise (spec E4-D8) —
/// never a silent fallback, because a monitor that goes quiet on what it
/// cannot decode hides more than it shows (RESEARCH §6.1: 38% of group
/// addresses resolve to no DPT at all).
fn format_decoded_value(v: &knx_net::GroupValue, dpt: DptAnnotation) -> String {
    let raw = format_group_value_payload(v);
    match dpt {
        DptAnnotation::NotApplicable => raw,
        DptAnnotation::None => format!("{raw} (no DPT resolved)"),
        DptAnnotation::Single(dpt_ref) => match knx_core::decode(*dpt_ref, v) {
            Ok(value) => format!("{dpt_ref} {}", value.format(*dpt_ref)),
            Err(e) => format!("{raw} ({e})"),
        },
        DptAnnotation::Conflict(dpts) => {
            let names = dpts
                .iter()
                .map(|d| d.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            format!("{raw} (conflicting DPTs: {names})")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        format_version_line, load_project_individual_addresses, parse_bus_route_monitor_args,
        parse_bus_route_send_args, DISCOVER_EMPTY_HINT,
    };
    use std::path::Path;

    /// No socket involved — this only checks the static hint text, so it
    /// runs the same in a sandbox as on a real machine (unlike `discover()`
    /// itself, which needs a multicast-capable network).
    #[test]
    fn discover_empty_hint_names_multicast_and_container_networking() {
        assert!(DISCOVER_EMPTY_HINT.contains("multicast"));
        assert!(DISCOVER_EMPTY_HINT.contains("container"));
        assert!(DISCOVER_EMPTY_HINT.contains("host networking"));
    }

    #[test]
    fn version_line_carries_the_commit_as_build_metadata_when_known() {
        assert_eq!(
            format_version_line("knx", "0.1.0-alpha.1", Some("4cde085")),
            "knx 0.1.0-alpha.1+g4cde085"
        );
    }

    #[test]
    fn version_line_degrades_to_the_manifest_version_without_git() {
        assert_eq!(
            format_version_line("knx", "0.1.0-alpha.1", None),
            "knx 0.1.0-alpha.1"
        );
    }

    /// A project with two devices — one at `1.1.5`, one with no
    /// individual address at all — saved and reloaded through the real
    /// `knx-store` schema, not a hand-built `Vec`. Backs both
    /// `load_project_individual_addresses` tests below.
    fn save_fixture_project(path: &Path) {
        use knx_core::{
            string_table::Language, CommissioningState, CompletionStatus, DeviceId, DeviceInstance,
            IndividualAddress, Installation, InstallationId, Project, SourceRef, Topology,
        };
        let mut project = Project::new(Language("en".into()));
        project.installations.push(Installation {
            id: InstallationId(0),
            name: "I".into(),
            default_line: None,
            multicast_address: None,
            completion: CompletionStatus::FinishedDesign,
            topology: Topology {
                areas: vec![],
                lines: vec![],
                unassigned: vec![DeviceId(1), DeviceId(2)],
            },
            buildings: vec![],
            group_ranges: vec![],
            group_addresses: vec![],
            parameters: vec![],
        });
        project.devices.insert(DeviceInstance {
            id: DeviceId(1),
            source: SourceRef {
                path: "d1".into(),
                ets_id: "d1".into(),
            },
            name: "Addressed Device".into(),
            description: None,
            address: Some(IndividualAddress::new(1, 1, 5).unwrap()),
            product_ref: "P".into(),
            program_ref: "H".into(),
            commissioning: CommissioningState::default(),
            visibility_calculated: true,
            com_objects: vec![],
            binary_data: vec![],
        });
        project.devices.insert(DeviceInstance {
            id: DeviceId(2),
            source: SourceRef {
                path: "d2".into(),
                ets_id: "d2".into(),
            },
            name: "Unaddressed Device".into(),
            description: None,
            address: None,
            product_ref: "P".into(),
            program_ref: "H".into(),
            commissioning: CommissioningState::default(),
            visibility_calculated: true,
            com_objects: vec![],
            binary_data: vec![],
        });
        let conn = knx_store::open_and_migrate(path).expect("open/migrate fixture store");
        knx_store::save_project(&conn, &project).expect("save fixture project");
    }

    #[test]
    fn load_project_individual_addresses_round_trips_through_a_saved_project() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fixture.knxdb");
        save_fixture_project(&path);

        let addresses = load_project_individual_addresses(&path).unwrap();
        assert_eq!(
            addresses,
            vec![knx_core::IndividualAddress::new(1, 1, 5).unwrap()]
        );
    }

    #[test]
    fn load_project_individual_addresses_refuses_a_path_that_does_not_exist() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("does-not-exist.knxdb");

        let err = load_project_individual_addresses(&path).unwrap_err();
        assert!(err.contains("does not exist"));
        assert!(
            !path.exists(),
            "must refuse before open_and_migrate can create an empty database"
        );
    }

    /// E6: omitting `--multicast-group` from `route-monitor` must parse to
    /// `None` — the caller's cue to keep joining the standard group, today's
    /// behaviour byte for byte.
    #[test]
    fn route_monitor_args_without_multicast_group_parses_to_none() {
        let args = ["--source-address".to_string(), "1.1.1".to_string()];
        let parsed = parse_bus_route_monitor_args(&args).unwrap();
        assert_eq!(parsed.multicast_group, None);
    }

    #[test]
    fn route_monitor_args_parses_a_given_multicast_group() {
        let args = [
            "--source-address".to_string(),
            "1.1.1".to_string(),
            "--multicast-group".to_string(),
            "239.0.2.1".to_string(),
        ];
        let parsed = parse_bus_route_monitor_args(&args).unwrap();
        assert_eq!(parsed.multicast_group, Some("239.0.2.1".to_string()));
    }

    /// Same cue, same default, for `route-send`.
    #[test]
    fn route_send_args_without_multicast_group_parses_to_none() {
        let args = [
            "--source-address".to_string(),
            "1.1.1".to_string(),
            "1/2/3".to_string(),
            "1".to_string(),
        ];
        let parsed = parse_bus_route_send_args(&args).unwrap();
        assert_eq!(parsed.multicast_group, None);
    }

    #[test]
    fn route_send_args_parses_a_given_multicast_group() {
        let args = [
            "--source-address".to_string(),
            "1.1.1".to_string(),
            "--multicast-group".to_string(),
            "239.0.2.1".to_string(),
            "1/2/3".to_string(),
            "1".to_string(),
        ];
        let parsed = parse_bus_route_send_args(&args).unwrap();
        assert_eq!(parsed.multicast_group, Some("239.0.2.1".to_string()));
        assert_eq!(parsed.group_address, "1/2/3");
        assert_eq!(parsed.value, "1");
    }
}
