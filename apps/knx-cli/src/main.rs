//! Headless entry point. Keeping a real CLI alongside the desktop application
//! is what forces the core to stay free of user-interface dependencies and
//! makes import, roundtrip and regression tests runnable in CI without a
//! display.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str =
    "usage: knx import <file.knxproj> [--store <path.knxdb>] [--report-json <path.json>]\n\
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
            other if other.starts_with("--") => {
                return Err(format!("unknown flag: {other}"));
            }
            other if file.is_none() => file = Some(other.to_string()),
            other => return Err(format!("unexpected extra argument: {other}")),
        }
        i += 1;
    }
    let file = file.ok_or_else(|| "missing <file.knxproj>".to_string())?;
    Ok(ImportArgs {
        file,
        store,
        report_json,
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

    let imported = match knx_app::import_ets_project(Path::new(&parsed.file), &conn) {
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
}
