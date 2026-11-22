//! `knx devconf`: checks device configuration modules, validates documents and writes exports.
//!
//! Module and document commands are files only (ADR-0175). `export-stored` additionally reads one
//! saved project read-only (no migration in place, no write); nothing here opens a bus connection
//! or a device.

use knx_devconf::{ExportError, Finding, FindingKind, Module, Validation};
use serde_json::{json, Value};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "usage: knx devconf check <module.devconf.json> [--json]\n\
     \x20      knx devconf validate <module.devconf.json> <document.json> [--json]\n\
     \x20      knx devconf export <module.devconf.json> <document.json> --export <id> --output <file>\n\
     \x20      knx devconf export-stored <project.knxdb> --device <id> --module <module.devconf.json> --export <id> --output <file>\n\
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
        Some("export-stored") => export_stored(&args[1..]),
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
    emit(&module, module_path, &document, &export_id, &output)
}

/// Exports from the module and document, writing nothing unless the export succeeds.
fn emit(
    module: &Module,
    module_path: &str,
    document: &[u8],
    export_id: &str,
    output: &Path,
) -> Result<ExitCode, String> {
    let bytes = match module.export(document, export_id) {
        Ok(bytes) => bytes,
        Err(ExportError::Invalid(validation)) => {
            report(module, &validation, false);
            eprintln!("nothing written: the document is invalid");
            return Ok(ExitCode::FAILURE);
        }
        Err(e @ ExportError::Refused { .. }) => {
            eprintln!("nothing written: {e}");
            return Ok(ExitCode::FAILURE);
        }
        Err(e) => return Err(format!("{module_path}: {e}")),
    };
    write_atomically(output, &bytes)
        .map_err(|e| format!("cannot write {}: {e}", output.display()))?;
    println!("wrote {} bytes to {}", bytes.len(), output.display());
    Ok(ExitCode::SUCCESS)
}

/// Exports the configuration document stored in a saved project for one device.
///
/// The project is only read (never migrated in place). The stored profile must name exactly the
/// explicitly given module (id, version and SHA-256); the stored bytes reach the module's own
/// validation unchanged.
fn export_stored(args: &[String]) -> Result<ExitCode, String> {
    let mut project_path = None;
    let (mut device, mut module_path, mut export_id, mut output) = (None, None, None, None);
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--device" if device.is_none() => {
                device = Some(crate::take_value(args, i + 1, "--device")?)
            }
            "--module" if module_path.is_none() => {
                module_path = Some(crate::take_value(args, i + 1, "--module")?)
            }
            "--export" if export_id.is_none() => {
                export_id = Some(crate::take_value(args, i + 1, "--export")?)
            }
            "--output" if output.is_none() => {
                output = Some(PathBuf::from(crate::take_value(args, i + 1, "--output")?))
            }
            flag if flag.starts_with("--") => return Err(USAGE.to_owned()),
            path if project_path.is_none() => {
                project_path = Some(path.to_owned());
                i += 1;
                continue;
            }
            _ => return Err(USAGE.to_owned()),
        }
        i += 2;
    }
    let (Some(project_path), Some(device), Some(module_path), Some(export_id), Some(output)) =
        (project_path, device, module_path, export_id, output)
    else {
        return Err(USAGE.to_owned());
    };
    let device = device
        .parse::<u32>()
        .ok()
        .filter(|id| *id != 0)
        .map(knx_core::DeviceId)
        .ok_or_else(|| format!("--device {device}: not a device id (a positive number)"))?;
    ensure_output_distinct(
        &output,
        &[("project", &project_path), ("module", &module_path)],
    )?;
    let module = load(&module_path)?;
    let conn = knx_store::open_existing_read_only(Path::new(&project_path))
        .map(|store| store.conn)
        .map_err(|e| format!("cannot open project {project_path}: {e}"))?;
    let project = knx_store::load_project(&conn)
        .map_err(|e| format!("cannot load project {project_path}: {e}"))?;
    if project.devices.get(device).is_none() {
        return Err(format!("{project_path}: no device {}", device.0));
    }
    let profile = project.devices.config_profile(device).ok_or_else(|| {
        format!(
            "{project_path}: device {} has no stored configuration profile",
            device.0
        )
    })?;
    let stored = (
        profile.module_id(),
        profile.module_version(),
        profile.module_sha256(),
    );
    if stored != (module.id(), module.version(), module.sha256()) {
        eprintln!(
            "nothing written: the stored profile of device {} is for module {} {} (sha256 {}), not {} {} (sha256 {})",
            device.0,
            stored.0,
            stored.1,
            stored.2,
            module.id(),
            module.version(),
            module.sha256()
        );
        return Ok(ExitCode::FAILURE);
    }
    emit(
        &module,
        &module_path,
        &profile.document,
        &export_id,
        &output,
    )
}

/// Refuses an output that is, or already links to, one of the inputs (symlink or hard link).
fn ensure_output_distinct(output: &Path, inputs: &[(&str, &str)]) -> Result<(), String> {
    use std::os::unix::fs::MetadataExt;
    let Ok(target) = std::fs::metadata(output) else {
        return Ok(());
    };
    for (what, path) in inputs {
        if let Ok(input) = std::fs::metadata(path) {
            if (input.dev(), input.ino()) == (target.dev(), target.ino()) {
                return Err(format!(
                    "--output {} is the same file as the {what} {path}",
                    output.display()
                ));
            }
        }
    }
    Ok(())
}

/// Writes through a temporary file in the target directory so a reader never sees half a file.
///
/// The temporary file is created like any new file (mode from the umask) under a short name of
/// its own, renamed over the target and the directory is synced afterwards. Only a temporary
/// file this call created is ever removed. A failed directory sync after the rename is reported
/// as such: the output is then already replaced, only its durability is unconfirmed.
fn write_atomically(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos());
    let temp_name = format!(".knx-devconf-{}-{nanos}.tmp", std::process::id());
    write_atomically_via(path, bytes, &temp_name)
}

fn write_atomically_via(path: &Path, bytes: &[u8], temp_name: &str) -> std::io::Result<()> {
    let dir = match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p,
        _ => Path::new("."),
    };
    let temp = dir.join(temp_name);
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)?;
    let written = file
        .write_all(bytes)
        .and_then(|()| file.sync_all())
        .and_then(|()| std::fs::rename(&temp, path));
    if let Err(error) = written {
        let _ = std::fs::remove_file(&temp);
        return Err(error);
    }
    std::fs::File::open(dir)?.sync_all().map_err(|e| {
        std::io::Error::new(
            e.kind(),
            format!("the output was replaced, but syncing its directory failed: {e}"),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::write_atomically_via;

    #[test]
    fn a_colliding_temporary_name_never_touches_the_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let foreign = dir.path().join(".taken.tmp");
        std::fs::write(&foreign, b"someone else's").unwrap();
        let target = dir.path().join("out.bin");
        let error = write_atomically_via(&target, b"new", ".taken.tmp").unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
        assert_eq!(std::fs::read(&foreign).unwrap(), b"someone else's");
        assert!(!target.exists());
        write_atomically_via(&target, b"new", ".free.tmp").unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"new");
        assert!(!dir.path().join(".free.tmp").exists());
    }
}
