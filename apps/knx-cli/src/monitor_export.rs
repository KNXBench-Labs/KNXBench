//! Offline native-working-snapshot adapter with projection owned exclusively by App.
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};

use knx_app::monitor_export::{self, ExportOptions};
use knx_core::InstallationId;

const USAGE: &str = "knx monitor-export <project.knxdb> <new.zip> [--installation <id>] [--language <code>] [--product-db <path>] [--no-product-db]";

struct Args {
    input: PathBuf,
    output: PathBuf,
    options: ExportOptions,
    product_db: Option<String>,
    no_product_db: bool,
}

fn parse(args: &[String]) -> Result<Args, String> {
    if args.len() < 2
        || args[..2]
            .iter()
            .any(|arg| arg.is_empty() || arg.starts_with('-'))
    {
        return Err(USAGE.into());
    }
    let mut parsed = Args {
        input: PathBuf::from(&args[0]),
        output: PathBuf::from(&args[1]),
        options: ExportOptions::default(),
        product_db: None,
        no_product_db: false,
    };
    let mut index = 2;
    while index < args.len() {
        let flag = args[index].as_str();
        if flag == "--no-product-db" {
            if parsed.no_product_db {
                return Err("duplicate --no-product-db".into());
            }
            parsed.no_product_db = true;
            index += 1;
            continue;
        }
        if !matches!(flag, "--installation" | "--language" | "--product-db") {
            return Err(format!("unexpected argument {flag:?}; {USAGE}"));
        }
        let value = crate::take_value(args, index + 1, flag)?;
        if value.is_empty() || value.starts_with('-') {
            return Err(format!("{flag} needs a nonempty value"));
        }
        match flag {
            "--installation" => {
                if parsed.options.installation.is_some() {
                    return Err("duplicate --installation".into());
                }
                if !value.bytes().all(|byte| byte.is_ascii_digit()) {
                    return Err("--installation needs an unsigned decimal installation id".into());
                }
                parsed.options.installation =
                    Some(InstallationId(value.parse().map_err(|_| {
                        "--installation is outside the supported id range (0..255)"
                    })?));
            }
            "--language" => {
                if parsed.options.language.is_some() {
                    return Err("duplicate --language".into());
                }
                if value.chars().any(char::is_whitespace) {
                    return Err("--language needs a language code without whitespace".into());
                }
                parsed.options.language = Some(value);
            }
            "--product-db" => {
                if parsed.product_db.replace(value).is_some() {
                    return Err("duplicate --product-db".into());
                }
            }
            _ => unreachable!(),
        }
        index += 2;
    }
    if parsed.no_product_db && parsed.product_db.is_some() {
        return Err("--product-db and --no-product-db are mutually exclusive".into());
    }
    Ok(parsed)
}

pub fn run(args: &[String]) -> ExitCode {
    match execute(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("monitor-export failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn parent(path: &Path) -> &Path {
    path.parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
}

/// Resolve the publication name without following a final-component symlink.
fn publication_path(path: &Path) -> Result<PathBuf, String> {
    let name = path.file_name().ok_or("output needs a new filename")?;
    Ok(parent(path)
        .canonicalize()
        .map_err(|error| format!("could not resolve output parent: {error}"))?
        .join(name))
}

/// Admit quiescent standalone rollback-layout inputs before either SQLite opener.
/// SQLite's documented file write/read versions at offsets 18/19 are 1/1 for
/// rollback journalling and 2/2 for WAL (sqlite.org/fileformat.html, section 1.3).
/// Read-only WAL opens can still create sidecars; immutable would ignore live WAL.
fn admit_input_layout(path: &Path, kind: &str, optional: bool) -> Result<(), String> {
    let failed = |error| format!("could not inspect {kind}: {error}");
    let resolved = match path.canonicalize() {
        Ok(path) => Some(path),
        Err(error) if optional && error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(failed(error)),
    };
    // Inspect both names: SQLite resolves live input symlinks to their targets.
    // Refuse every sidecar entry, including cold journals and dangling links.
    for name in std::iter::once(path).chain(resolved.as_deref()) {
        for suffix in ["-wal", "-shm", "-journal"] {
            let mut sidecar = name.as_os_str().to_os_string();
            sidecar.push(suffix);
            match fs::symlink_metadata(Path::new(&sidecar)) {
                Ok(_) => {
                    return Err(format!(
                        "unsupported input layout for {kind}: existing SQLite {suffix} sidecar"
                    ));
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(failed(error)),
            }
        }
    }
    let Some(resolved) = resolved else {
        return Ok(()); // Truly absent optional catalogue; no file is created.
    };
    if !fs::metadata(&resolved).map_err(failed)?.is_file() {
        return Err(format!(
            "unsupported input layout for {kind}: expected a regular SQLite file"
        ));
    }
    let mut header = [0; 20];
    File::open(&resolved)
        .and_then(|mut file| file.read_exact(&mut header))
        .map_err(failed)?;
    if &header[..16] != b"SQLite format 3\0" {
        return Err(format!("could not read {kind}: invalid SQLite header"));
    }
    if header[18..20] != [1, 1] {
        return Err(format!(
            "unsupported input layout for {kind}: SQLite file write/read versions {}/{} (WAL or unknown); only standalone rollback-journal files (1/1) without sidecars are supported",
            header[18], header[19]
        ));
    }
    Ok(())
}

fn execute(raw: &[String]) -> Result<(), String> {
    let args = parse(raw)?;
    let output = publication_path(&args.output)?;
    // symlink_metadata also sees dangling links: no destination is ever replaced.
    match fs::symlink_metadata(&output) {
        Ok(_) => {
            return Err(
                "output already exists; choose a new ZIP filename (never overwritten)".into(),
            );
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(format!("could not inspect output: {error}")),
    }
    let product_path = if args.no_product_db {
        None
    } else {
        Some(crate::resolve_product_db_path(args.product_db.as_deref())?)
    };
    // Also protect a *missing* catalogue path; absence must not become a ZIP at
    // the normal catalogue location. Existing aliases were refused above.
    if let Some(path) = &product_path {
        // An existing dangling input link is not an absent optional catalogue:
        // publication at its target would poison the configured catalogue path.
        match fs::symlink_metadata(path) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                fs::metadata(path).map_err(|error| {
                    format!("could not read product database symlink target: {error}")
                })?;
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(format!("could not inspect product database: {error}")),
        }
        if publication_path(path).ok().as_ref() == Some(&output) {
            return Err("output aliases the product database path".into());
        }
    }
    // Admit both inputs before SQLite can create/recover any on-disk state.
    admit_input_layout(&args.input, "native project", false)?;
    if let Some(path) = &product_path {
        admit_input_layout(path, "product database", true)?;
    }
    let admitted = knx_store::open_existing_read_only(&args.input)
        .map_err(|error| format!("could not read native project: {error}"))?;
    let read = admitted
        .conn
        .unchecked_transaction()
        .map_err(|error| error.to_string())?;
    let snapshot = match knx_store::project_history::load_editor(&read)
        .map_err(|error| format!("could not recover native working snapshot: {error}"))?
    {
        Some(history) => history.working,
        None => knx_store::project_history::NativeSnapshot::read(&read)
            .map_err(|error| format!("could not read native snapshot: {error}"))?,
    };
    read.commit()
        .map_err(|error| format!("native read transaction: {error}"))?;
    let products = if args.no_product_db {
        None
    } else {
        Some(crate::open_products_db_for_reading(
            args.product_db.as_deref(),
        )?)
    };
    let bundle = monitor_export::build(&snapshot.project, products.as_ref(), &args.options)?;

    publish(&bundle.bytes, &output, File::sync_all)?;

    println!("Monitoring ZIP written: {}", args.output.display());
    println!(
        "{}; installation {}; reduced projection, not a backup; no receiver acceptance claimed",
        bundle.report.profile, bundle.report.installation
    );
    let counts = &bundle.report.counts;
    for (name, count) in [
        ("devices", &counts.devices),
        ("communication_objects", &counts.communication_objects),
        ("group_addresses", &counts.group_addresses),
        ("spaces", &counts.spaces),
        ("functions", &counts.functions),
    ] {
        println!(
            "{name}: selected {}, exported {}, omitted {}",
            count.selected, count.exported, count.omitted
        );
    }
    println!(
        "issues: {} (full report in export-report.json)",
        bundle.report.issues.len()
    );
    for issue in &bundle.report.issues {
        println!(
            "  {}: {}.{}: {}",
            issue.code, issue.entity, issue.field, issue.detail
        );
    }
    Ok(())
}

/// One publication boundary; the injected directory-sync operation lets tests
/// distinguish a prepublication refusal from a complete but not durable ZIP.
fn publish(
    bytes: &[u8],
    output: &Path,
    sync_directory: impl FnOnce(&File) -> std::io::Result<()>,
) -> Result<(), String> {
    let directory = File::open(parent(output))
        .map_err(|error| format!("could not open output directory: {error}"))?;
    let mut temporary = tempfile::Builder::new()
        .prefix(".knx-monitor-export-")
        .tempfile_in(parent(output))
        .map_err(|error| format!("could not create private ZIP temporary: {error}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        temporary
            .as_file()
            .set_permissions(fs::Permissions::from_mode(0o600))
            .map_err(|error| format!("could not restrict ZIP permissions: {error}"))?;
    }
    temporary
        .write_all(bytes)
        .map_err(|error| format!("could not write ZIP: {error}"))?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|error| format!("could not sync ZIP: {error}"))?;
    temporary
        .persist_noclobber(output)
        .map_err(|error| format!("could not publish ZIP without overwriting: {error}"))?;
    sync_directory(&directory).map_err(|error| {
        format!(
            "complete ZIP published; durability unconfirmed: output directory sync failed: {error}"
        )
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directory_sync_failure_retains_complete_published_bytes_and_reports_uncertainty() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("complete.zip");
        let complete = b"complete container bytes";
        let result = publish(complete, &output, |_| {
            assert_eq!(fs::read(&output).unwrap(), complete);
            Err(std::io::Error::other("injected directory sync failure"))
        });
        let error = result.unwrap_err();
        assert!(
            error.contains("complete ZIP published; durability unconfirmed"),
            "{error}"
        );
        assert_eq!(fs::read(&output).unwrap(), complete);
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn no_clobber_at_publication_preserves_a_late_competitor() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("competitor.zip");
        fs::write(&output, b"competitor").unwrap();
        let error = publish(b"our new bytes", &output, |_| {
            panic!("must not sync a refused publication")
        })
        .unwrap_err();
        assert!(error.contains("without overwriting"));
        assert_eq!(fs::read(&output).unwrap(), b"competitor");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }
}
