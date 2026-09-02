//! Headless entry point. Keeping a real CLI alongside the desktop application
//! is what forces the core to stay free of user-interface dependencies and
//! makes import, roundtrip and regression tests runnable in CI without a
//! display.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str =
    "usage: knx import <file.knxproj> [--store <path.knxdb>] [--report-json <path.json>]";

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

fn parse_import_args(args: &[String]) -> Option<ImportArgs> {
    let mut file = None;
    let mut store = None;
    let mut report_json = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--store" => {
                i += 1;
                store = args.get(i).cloned();
            }
            "--report-json" => {
                i += 1;
                report_json = args.get(i).cloned();
            }
            other if file.is_none() => file = Some(other.to_string()),
            _ => {}
        }
        i += 1;
    }
    Some(ImportArgs {
        file: file?,
        store,
        report_json,
    })
}

fn run_import(args: &[String]) -> ExitCode {
    let Some(parsed) = parse_import_args(args) else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
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
    // unsupported constructs still succeeded at producing a project.
    ExitCode::SUCCESS
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
    if imported.report.has_losses() {
        println!(
            "  {} error(s), {} unknown construct(s) — see the report for detail",
            imported.report.errors.len(),
            imported.report.unknown.len()
        );
    }
}
