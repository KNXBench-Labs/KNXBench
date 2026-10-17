//! Previews and confirms adoption of plug-in bytes into an existing native project.
//!
//! Data only (ADR-0025): bytes are copied verbatim, nothing is decoded or run.

use knx_app::plugin_data::{self as adopt, Request};
use knx_core::CommandStack;
use knx_store::project_history::{self as history, NativeSnapshot};
use serde_json::json;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "usage: knx adopt-plugin-data <source.knxproj> --project <target.knxdb> [--password-stdin]\n\
     \x20                         (lists the device pairs that could take plug-in bytes)\n\
     \x20      knx adopt-plugin-data <source.knxproj> --project <target.knxdb>\n\
     \x20                         --source-device <id> --target-device <id> [--confirm <token>] [--password-stdin]\n\
     \x20                         (without --confirm: preview and token; with it: apply)";

struct Args {
    source: PathBuf,
    target: PathBuf,
    request: Option<Request>,
    confirm: Option<String>,
    password_stdin: bool,
}

fn parse(args: &[String]) -> Result<Args, String> {
    let source = args.first().filter(|p| !p.starts_with("--")).ok_or(USAGE)?;
    let mut target = None;
    let mut source_device = None;
    let mut target_device = None;
    let mut confirm = None;
    let mut password_stdin = false;
    let mut i = 1;
    while i < args.len() {
        let flag = args[i].as_str();
        if flag == "--password-stdin" {
            if password_stdin {
                return Err("duplicate password source".into());
            }
            password_stdin = true;
            i += 1;
            continue;
        }
        if flag == "--password" || flag.starts_with("--password=") {
            return Err("passwords are accepted only with --password-stdin".into());
        }
        let value = crate::take_value(args, i + 1, flag)?;
        match flag {
            "--project" if target.is_none() => target = Some(PathBuf::from(value)),
            "--source-device" if source_device.is_none() => {
                source_device = Some(
                    value
                        .parse::<u32>()
                        .map_err(|_| "invalid source device id")?,
                )
            }
            "--target-device" if target_device.is_none() => {
                target_device = Some(
                    value
                        .parse::<u32>()
                        .map_err(|_| "invalid target device id")?,
                )
            }
            "--confirm" if confirm.is_none() => confirm = Some(value),
            _ => return Err("unknown or duplicate adopt-plugin-data argument".into()),
        }
        i += 2;
    }
    let request = match (source_device, target_device) {
        (Some(source_device), Some(target_device)) => Some(Request {
            source_device,
            target_device,
        }),
        (None, None) => None,
        _ => return Err("--source-device and --target-device go together".into()),
    };
    if request.is_none() && confirm.is_some() {
        return Err("--confirm needs --source-device and --target-device".into());
    }
    Ok(Args {
        source: source.into(),
        target: target.ok_or("--project is required")?,
        request,
        confirm,
        password_stdin,
    })
}

pub(crate) fn run(args: &[String]) -> ExitCode {
    match execute(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("plug-in adoption refused: {error}");
            ExitCode::FAILURE
        }
    }
}

fn execute(args: &[String]) -> Result<(), String> {
    let args = parse(args)?;
    let password = if args.password_stdin {
        Some(crate::read_password_from_stdin()?)
    } else {
        None
    };
    let source = knx_app::selective_import::read_source(&args.source, password.as_ref())?;
    // Older stores are inspected in memory only, never upgraded by this command.
    let readonly = knx_store::open_existing_read_only(&args.target).map_err(|e| e.to_string())?;
    if readonly.migrated_from.is_some() {
        return Err("open and save the target with this KNXBench version first; adoption does not migrate the target".into());
    }
    let editor = history::load_editor(&readonly.conn).map_err(|e| e.to_string())?;
    let (mut snapshot, stack, generation) = match editor {
        Some(editor) => (
            editor.working,
            CommandStack::from_snapshot_states(editor.undo, editor.redo),
            editor.generation,
        ),
        None => (
            NativeSnapshot::read(&readonly.conn).map_err(|e| e.to_string())?,
            CommandStack::new(),
            0,
        ),
    };
    let Some(request) = args.request else {
        println!(
            "{}",
            json!({"sourceHash": source.source_hash, "candidates": adopt::candidates(&snapshot, &source)})
        );
        return Ok(());
    };
    let before_hash = snapshot.semantic_hash().map_err(|e| e.to_string())?;
    let plan = adopt::plan(&snapshot, &source, request).map_err(|e| e.to_string())?;
    let canonical = std::fs::canonicalize(&args.target).map_err(|e| e.to_string())?;
    let path_hash = knx_etsproj::opaque::sha256_hex(canonical.as_os_str().as_encoded_bytes());
    let token = plan.confirmation_token(&format!("cli-adopt-v1:{path_hash}:{generation}"))?;
    let Some(confirmation) = args.confirm else {
        println!(
            "{}",
            json!({"applied": false, "preview": plan.preview, "confirmationToken": token})
        );
        return Ok(());
    };
    if confirmation != token {
        return Err("confirmation is missing or stale; preview again".into());
    }
    let preview = adopt::apply(&mut snapshot, plan).map_err(|e| e.to_string())?;
    drop(readonly);
    let conn =
        history::open_editor_store(Path::new(&args.target), false).map_err(|e| e.to_string())?;
    history::save_editor_if_unchanged(&conn, &snapshot, &stack, generation, &before_hash, true)
        .map_err(|e| e.to_string())?;
    println!("{}", json!({"applied": true, "preview": preview}));
    Ok(())
}
