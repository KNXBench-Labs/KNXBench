//! `knx products update|restore`: previewed, token-confirmed catalogue updates.
//!
//! ADR-0117. The package is a manufacturer file the user supplies; nothing is
//! downloaded. Without `--confirm` the command only previews (JSON on stdout,
//! the product database untouched). With the preview's token it writes a
//! backup next to the database and then installs. `restore` copies such a
//! backup back.

use std::path::Path;
use std::process::ExitCode;

use serde_json::json;

const UPDATE_USAGE: &str = "usage: knx products update <file.knxprod> [--product-db <path>] [--allow-large-package] [--confirm <token>]";
const RESTORE_USAGE: &str = "usage: knx products restore <backup.sqlite> [--product-db <path>]";

struct UpdateArgs {
    file: String,
    product_db: Option<String>,
    allow_large: bool,
    confirm: Option<String>,
}

fn parse_update(args: &[String]) -> Result<UpdateArgs, String> {
    let mut file = None;
    let mut product_db = None;
    let mut allow_large = false;
    let mut confirm = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--product-db" if product_db.is_none() => {
                product_db = Some(args.get(i + 1).ok_or("--product-db needs a path")?.clone());
                i += 2;
            }
            "--confirm" if confirm.is_none() => {
                confirm = Some(
                    args.get(i + 1)
                        .ok_or("--confirm needs the preview token")?
                        .clone(),
                );
                i += 2;
            }
            "--allow-large-package" if !allow_large => {
                allow_large = true;
                i += 1;
            }
            other if !other.starts_with("--") && file.is_none() => {
                file = Some(other.to_string());
                i += 1;
            }
            other => return Err(format!("unexpected or repeated argument: {other}")),
        }
    }
    if let Some(token) = &confirm {
        let well_formed = token.len() == 64
            && token
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
        if !well_formed {
            return Err("--confirm needs the exact lower-case token the preview printed".into());
        }
    }
    Ok(UpdateArgs {
        file: file.ok_or("missing <file.knxprod>")?,
        product_db,
        allow_large,
        confirm,
    })
}

/// `knx products update`.
pub(crate) fn run_update(args: &[String]) -> ExitCode {
    match update(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("catalogue update refused: {error}");
            ExitCode::FAILURE
        }
    }
}

fn update(args: &[String]) -> Result<(), String> {
    let args = parse_update(args).map_err(|e| format!("{e}\n{UPDATE_USAGE}"))?;
    let db = crate::resolve_product_db_path(args.product_db.as_deref())?;
    let bytes = match crate::read_bounded_package(&args.file) {
        Ok(Ok(bytes)) => bytes,
        Ok(Err(refusal)) => return Err(refusal.to_string()),
        Err(error) => return Err(format!("cannot read {}: {error}", args.file)),
    };
    let source_name = Path::new(&args.file)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| args.file.clone());
    let limits = if args.allow_large {
        knx_productdb::PackageLimits::LARGE
    } else {
        knx_productdb::PackageLimits::STANDARD
    };
    match args.confirm {
        None => {
            let preview = knx_productdb::preview_package_update(&db, &source_name, &bytes, limits)
                .map_err(|e| e.to_string())?;
            println!(
                "{}",
                json!({
                    "applied": false,
                    "productDb": db,
                    "preview": preview,
                    "bindings": "existing projects keep their exact application programs; a new version is added beside the old one",
                    "next": "rerun with --confirm <preview.token> to back up the product database and install",
                })
            );
        }
        Some(token) => {
            let applied =
                knx_productdb::apply_package_update(&db, &source_name, &bytes, limits, &token)
                    .map_err(|e| e.to_string())?;
            println!(
                "{}",
                json!({
                    "applied": true,
                    "productDb": db,
                    "backup": applied.backup,
                    "preview": applied.preview,
                    "restore": format!("knx products restore {} --product-db {}", applied.backup.display(), db.display()),
                })
            );
        }
    }
    Ok(())
}

/// `knx products restore`.
pub(crate) fn run_restore(args: &[String]) -> ExitCode {
    match restore(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("restore refused: {error}");
            ExitCode::FAILURE
        }
    }
}

fn restore(args: &[String]) -> Result<(), String> {
    let (product_db, rest) =
        crate::split_product_db_flag(args).map_err(|e| format!("{e}\n{RESTORE_USAGE}"))?;
    let [backup] = rest.as_slice() else {
        return Err(RESTORE_USAGE.into());
    };
    let db = crate::resolve_product_db_path(product_db.as_deref())?;
    knx_productdb::restore_product_db_backup(&db, Path::new(backup)).map_err(|e| e.to_string())?;
    println!("restored {} from {backup}", db.display());
    Ok(())
}
