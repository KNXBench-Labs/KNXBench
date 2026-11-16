//! `knx devconf`: checks device configuration modules, validates documents and writes exports.
//!
//! Files only (ADR-0175): nothing here opens a project, a bus connection or a device.

use knx_devconf::{ExportError, Finding, FindingKind, Module, Validation};
use serde_json::{json, Value};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "usage: knx devconf check <module.devconf.json> [--json]\n\
     \x20      knx devconf validate <module.devconf.json> <document.json> [--json]\n\
     \x20      knx devconf export <module.devconf.json> <document.json> --export <id> --output <file>\n\
     exit status: 0 success, 1 invalid document or refused export, 2 unusable input";

fn unusable(message: impl std::fmt::Display) -> ExitCode {
    eprintln!("{message}");
    ExitCode::from(2)
}

fn read(path: &str) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| format!("cannot read {path}: {e}"))
}

fn load(path: &str) -> Result<Module, String> {
    Module::load(&read(path)?).map_err(|e| format!("{path}: unusable module: {e}"))
}

/// Runs `knx devconf <subcommand> ...`.
pub fn run(args: &[String]) -> ExitCode {
    let result = match args.first().map(String::as_str) {
        Some("check") => check(&args[1..]),
        Some("validate") => validate(&args[1..]),
        Some("export") => export(&args[1..]),
        _ => Err(USAGE.to_owned()),
    };
    result.unwrap_or_else(unusable)
}

/// Splits positional arguments from a trailing `--json` flag.
fn positional(args: &[String], count: usize) -> Result<(Vec<&str>, bool), String> {
    let json = args.iter().any(|a| a == "--json");
    let rest: Vec<&str> = args
        .iter()
        .map(String::as_str)
        .filter(|a| *a != "--json")
        .collect();
    if rest.len() != count
        || rest.iter().any(|a| a.starts_with("--"))
        || args.iter().filter(|a| *a == "--json").count() > 1
    {
        return Err(USAGE.to_owned());
    }
    Ok((rest, json))
}

fn identity(module: &Module) -> Value {
    json!({ "id": module.id(), "version": module.version(), "sha256": module.sha256() })
}

fn check(args: &[String]) -> Result<ExitCode, String> {
    let (paths, as_json) = positional(args, 1)?;
    let module = load(paths[0])?;
    let outline = module.outline();
    if as_json {
        let tables: Vec<Value> = outline
            .tables
            .iter()
            .map(|t| json!({ "name": t.name, "min_rows": t.min_rows, "max_rows": t.max_rows, "key": t.key, "columns": t.columns.len() }))
            .collect();
        let exports: Vec<Value> = module
            .exports()
            .iter()
            .map(|e| json!({ "id": e.id, "title": e.title, "extension": e.extension, "max_bytes": e.max_bytes }))
            .collect();
        let report = json!({ "module": identity(&module), "title": module.title(), "tables": tables, "rules": outline.rules, "exports": exports });
        println!("{report:#}");
    } else {
        println!(
            "module {} {} ({})",
            module.id(),
            module.version(),
            module.title()
        );
        println!("sha256 {}", module.sha256());
        for t in &outline.tables {
            let key = t
                .key
                .as_deref()
                .map(|k| format!(", key {k}"))
                .unwrap_or_default();
            println!(
                "table {}: {}..{} rows, {} members{key}",
                t.name,
                t.min_rows,
                t.max_rows,
                t.columns.len()
            );
        }
        println!("rules {}", outline.rules);
        for e in module.exports() {
            println!(
                "export {}: {} (.{}, at most {} bytes)",
                e.id, e.title, e.extension, e.max_bytes
            );
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn finding_json(f: &Finding) -> Value {
    let kind = match f.kind {
        FindingKind::Structure => "structure",
        FindingKind::Rule => "rule",
    };
    json!({ "kind": kind, "path": f.path, "rule": f.rule, "message": f.message })
}

fn report(module: &Module, validation: &Validation, as_json: bool) {
    if as_json {
        let findings: Vec<Value> = validation.findings.iter().map(finding_json).collect();
        let report = json!({ "module": identity(module), "valid": validation.is_valid(), "findings": findings, "omitted": validation.omitted });
        println!("{report:#}");
        return;
    }
    if validation.is_valid() {
        println!("valid for {} {}", module.id(), module.version());
        return;
    }
    for finding in &validation.findings {
        println!("{finding}");
    }
    if validation.omitted > 0 {
        println!("... {} more findings", validation.omitted);
    }
}

fn validate(args: &[String]) -> Result<ExitCode, String> {
    let (paths, as_json) = positional(args, 2)?;
    let module = load(paths[0])?;
    let document = read(paths[1])?;
    let validation = module
        .validate(&document)
        .map_err(|e| format!("{}: {e}", paths[0]))?;
    report(&module, &validation, as_json);
    Ok(if validation.is_valid() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

fn export(args: &[String]) -> Result<ExitCode, String> {
    let mut paths = Vec::new();
    let mut export_id = None;
    let mut output = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--export" if export_id.is_none() => {
                export_id = Some(crate::take_value(args, i + 1, "--export")?)
            }
            "--output" if output.is_none() => {
                output = Some(PathBuf::from(crate::take_value(args, i + 1, "--output")?))
            }
            flag if flag.starts_with("--") => return Err(USAGE.to_owned()),
            path => {
                paths.push(path);
                i += 1;
                continue;
            }
        }
        i += 2;
    }
    let (Some(export_id), Some(output), [module_path, document_path]) =
        (export_id, output, paths.as_slice())
    else {
        return Err(USAGE.to_owned());
    };
    let module = load(module_path)?;
    let document = read(document_path)?;
    let bytes = match module.export(&document, &export_id) {
        Ok(bytes) => bytes,
        Err(ExportError::Invalid(validation)) => {
            report(&module, &validation, false);
            eprintln!("nothing written: the document is invalid");
            return Ok(ExitCode::FAILURE);
        }
        Err(e @ ExportError::Refused { .. }) => {
            eprintln!("nothing written: {e}");
            return Ok(ExitCode::FAILURE);
        }
        Err(e) => return Err(format!("{module_path}: {e}")),
    };
    write_atomically(&output, &bytes)
        .map_err(|e| format!("cannot write {}: {e}", output.display()))?;
    println!("wrote {} bytes to {}", bytes.len(), output.display());
    Ok(ExitCode::SUCCESS)
}

/// Writes through a temporary file in the target directory so a reader never sees half a file.
///
/// The temporary file is created like any new file (mode from the umask), renamed over the
/// target and the directory is synced afterwards, so the rename survives a crash.
fn write_atomically(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let dir = match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p,
        _ => Path::new("."),
    };
    let name = path.file_name().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "the output needs a file name",
        )
    })?;
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos());
    let mut temp_name = std::ffi::OsString::from(".");
    temp_name.push(name);
    temp_name.push(format!(".{}-{nanos}.tmp", std::process::id()));
    let temp = dir.join(temp_name);
    let result = (|| {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        std::fs::rename(&temp, path)?;
        std::fs::File::open(dir)?.sync_all()
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result
}
