//! `knx products import-legacy`: publishes a legacy ETS3 product database (ADR-0094).

use std::path::Path;
use std::process::ExitCode;

use knx_app::legacy::open_legacy_file;
use knx_productdb::legacy::{publish_legacy, LegacyError, LegacyPublishReport};

use crate::legacy_inspect::{parse, read_bounded, read_password};

pub(crate) fn run(args: &[String]) -> ExitCode {
    let (product_db, rest) = match super::split_product_db_flag(args) {
        Ok(split) => split,
        Err(e) => {
            eprintln!("{e}\n{}", super::USAGE);
            return ExitCode::FAILURE;
        }
    };
    let args = match parse(&rest) {
        Ok(args) => args,
        Err(e) => {
            eprintln!("{e}\n{}", super::USAGE);
            return ExitCode::FAILURE;
        }
    };
    let bytes = match read_bounded(&args.file) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    let password = match read_password(&args.password) {
        Ok(password) => password,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    // Decrypt before the product database is even opened: a missing or
    // wrong password must not leave a database file behind.
    let payload = match open_legacy_file(&bytes, password.as_ref()) {
        Ok(payload) => payload,
        Err(error) => {
            eprintln!("import-legacy refused {}: {error}", args.file);
            if matches!(error, LegacyError::PasswordRequired) {
                eprintln!(
                    "rerun with --password-stdin (and type the password) or --password-file <path>"
                );
            }
            return ExitCode::FAILURE;
        }
    };
    let conn = match super::open_products_db(product_db.as_deref()) {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    let name = Path::new(&args.file)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| args.file.clone());
    match publish_legacy(&conn, &name, &bytes, &payload) {
        Ok(report) => {
            print!("{}", format_report(&args.file, &report));
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("import-legacy refused {}: {error}", args.file);
            ExitCode::FAILURE
        }
    }
}

fn format_report(file: &str, report: &LegacyPublishReport) -> String {
    let mut out = String::new();
    if report.skipped {
        out.push_str(&format!(
            "{file}: already imported (same content, namespace {}); recorded this file\n",
            report.namespace
        ));
    } else {
        out.push_str(&format!(
            "imported {file} (namespace {})\n",
            report.namespace
        ));
    }
    let programs = report.programs.len();
    out.push_str(&format!(
        "  {programs} application program{}\n",
        if programs == 1 { "" } else { "s" }
    ));
    for program in &report.programs {
        out.push_str(&format!("    {program}\n"));
    }
    out.push_str(&format!(
        "  {} catalog items, {} parameters, {} parameter refs, {} object refs, {} translations\n",
        report.catalog_items,
        report.parameters,
        report.parameter_refs,
        report.com_object_refs,
        report.translations
    ));
    out.push_str(&format!("  payload sha256 {}\n", report.payload_sha256));
    if report.diagnostics.is_empty() {
        out.push_str("  no mapping diagnostics\n");
    } else {
        out.push_str(&format!(
            "  {} mapping diagnostics:\n",
            report.diagnostics.len()
        ));
        for (kind, detail) in &report.diagnostics {
            out.push_str(&format!("    {kind}: {detail}\n"));
        }
    }
    out.push_str("  downloading these programs is not supported yet (ADR-0094)\n");
    out
}
