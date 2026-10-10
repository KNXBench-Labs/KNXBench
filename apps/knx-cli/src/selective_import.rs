//! Previews and confirms selected ETS lines/devices in an existing native project.

use knx_app::selective_import::{self as import, Selection};
use knx_core::CommandStack;
use knx_store::project_history::{self as history, NativeSnapshot};
use serde_json::json;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "usage: knx import-selection inspect <source.knxproj> [--password-stdin]\n       knx import-selection <source.knxproj> --project <target.knxdb>\n         --source-installation <id> --target-installation <id>\n         [--device <id>]... [--line <id>]... [--confirm <token>] [--password-stdin]\nWithout --confirm: read-only JSON preview. Import retains private source evidence.";

struct Args {
    source: PathBuf,
    target: Option<PathBuf>,
    selection: Option<Selection>,
    confirm: Option<String>,
    password_stdin: bool,
}

fn parse(args: &[String]) -> Result<Args, String> {
    let inspect = args.first().is_some_and(|a| a == "inspect");
    let start = usize::from(inspect);
    let source = args
        .get(start)
        .filter(|p| !p.starts_with("--"))
        .ok_or(USAGE)?;
    let mut target = None;
    let mut source_installation = None;
    let mut target_installation = None;
    let mut devices = vec![];
    let mut lines = vec![];
    let mut confirm = None;
    let mut password_stdin = false;
    let mut i = start + 1;
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
            "--source-installation" if source_installation.is_none() => {
                source_installation = Some(
                    value
                        .parse::<u8>()
                        .map_err(|_| "invalid source installation id")?,
                )
            }
            "--target-installation" if target_installation.is_none() => {
                target_installation = Some(
                    value
                        .parse::<u8>()
                        .map_err(|_| "invalid target installation id")?,
                )
            }
            "--device" => devices.push(value.parse::<u32>().map_err(|_| "invalid device id")?),
            "--line" => lines.push(value.parse::<u32>().map_err(|_| "invalid line id")?),
            "--confirm" if confirm.is_none() => confirm = Some(value),
            _ => return Err("unknown or duplicate selective-import argument".into()),
        }
        i += 2;
    }
    if inspect {
        if target.is_some()
            || source_installation.is_some()
            || target_installation.is_some()
            || !devices.is_empty()
            || !lines.is_empty()
            || confirm.is_some()
        {
            return Err("inspect accepts only a source and optional --password-stdin".into());
        }
        return Ok(Args {
            source: source.into(),
            target: None,
            selection: None,
            confirm,
            password_stdin,
        });
    }
    Ok(Args {
        source: source.into(),
        target: Some(target.ok_or("--project is required")?),
        selection: Some(Selection {
            source_installation: source_installation.ok_or("--source-installation is required")?,
            target_installation: target_installation.ok_or("--target-installation is required")?,
            devices,
            lines,
        }),
        confirm,
        password_stdin,
    })
}

pub(crate) fn run(args: &[String]) -> ExitCode {
    match execute(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("selective import refused: {error}");
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
    let source = import::read_source(&args.source, password.as_ref())?;
    let Some(path) = args.target else {
        println!(
            "{}",
            serde_json::to_string(&source.inventory()).map_err(|e| e.to_string())?
        );
        return Ok(());
    };
    // Older stores are inspected/migrated only in memory, never upgraded by this command.
    let readonly = knx_store::open_existing_read_only(&path).map_err(|e| e.to_string())?;
    if readonly.migrated_from.is_some() {
        return Err("open and save the target with this KNXBench version first; selective import does not migrate the target".into());
    }
    let editor = history::load_editor(&readonly.conn).map_err(|e| e.to_string())?;
    let (mut snapshot, mut stack, generation) = match editor {
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
    let before_hash = snapshot.semantic_hash().map_err(|e| e.to_string())?;
    let plan = import::plan(
        &snapshot,
        &source,
        args.selection.ok_or("selection missing")?,
    )?;
    let canonical = std::fs::canonicalize(&path).map_err(|e| e.to_string())?;
    let path_hash = knx_etsproj::opaque::sha256_hex(canonical.as_os_str().as_encoded_bytes());
    let token = plan.confirmation_token(&format!("cli-v1:{path_hash}:{generation}"))?;
    if let Some(confirmation) = args.confirm {
        if confirmation != token {
            return Err("confirmation is missing or stale; preview again".into());
        }
        let preview = import::apply(&mut snapshot, &mut stack, plan)?;
        drop(readonly);
        let conn =
            history::open_editor_store(Path::new(&path), false).map_err(|e| e.to_string())?;
        history::save_editor_if_unchanged(&conn, &snapshot, &stack, generation, &before_hash, true)
            .map_err(|e| e.to_string())?;
        println!("{}", json!({"applied":true,"preview":preview}));
    } else {
        println!(
            "{}",
            json!({"applied":false,"preview":plan.preview,"confirmationToken":token})
        );
    }
    Ok(())
}
