//! Runs a literal group-address naming recipe with preview-bound confirmation.

use knx_app::name_recipe::{
    self as recipe, NameRecipeIntent, NameRecipeNativeTarget, NameRecipeRevision,
};
use knx_core::{CommandStack, GroupAddressId};
use knx_store::project_history::{self as history, NativeSnapshot};
use serde_json::json;
use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "usage: knx name-recipe <project.knxdb> --group-address <id>...\n       [--prefix <literal>] [--suffix <literal>] [--confirm <token>]\nWithout --confirm: read-only exact ordered JSON preview; confirmation writes native working history, not root Save.";
struct Args {
    path: PathBuf,
    intent: NameRecipeIntent,
    confirm: Option<String>,
}
fn parse(args: &[String]) -> Result<Args, String> {
    let path = args.first().filter(|p| !p.starts_with("--")).ok_or(USAGE)?;
    let mut ids = vec![];
    let mut prefix = None;
    let mut suffix = None;
    let mut confirm = None;
    let mut i = 1;
    while i < args.len() {
        let flag = args[i].as_str();
        let value = args
            .get(i + 1)
            .ok_or_else(|| format!("missing value for {flag}"))?
            .clone();
        match flag {
            "--group-address" => {
                if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
                    return Err("group-address ID must be an exact decimal integer".into());
                }
                ids.push(GroupAddressId(
                    value
                        .parse::<u32>()
                        .map_err(|_| "group-address ID exceeds u32")?,
                ));
            }
            "--prefix" if prefix.is_none() => prefix = Some(value),
            "--suffix" if suffix.is_none() => suffix = Some(value),
            "--confirm" if confirm.is_none() => confirm = Some(value),
            _ => return Err("unknown or duplicate naming-recipe argument".into()),
        }
        i += 2;
    }
    if confirm.as_ref().is_some_and(|v| {
        v.len() != 64
            || !v
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    }) {
        return Err("confirmation must be the exact lower-case preview token".into());
    }
    Ok(Args {
        path: path.into(),
        intent: NameRecipeIntent {
            ids,
            prefix: prefix.unwrap_or_default(),
            suffix: suffix.unwrap_or_default(),
        },
        confirm,
    })
}
pub(crate) fn run(args: &[String]) -> ExitCode {
    match execute(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("name recipe refused: {error}");
            ExitCode::FAILURE
        }
    }
}
fn execute(values: &[String]) -> Result<(), String> {
    let args = parse(values)?;
    let canonical = std::fs::canonicalize(&args.path).map_err(|e| e.to_string())?;
    let readonly = knx_store::open_existing_read_only(&canonical).map_err(|e| e.to_string())?;
    if readonly.migrated_from.is_some() {
        return Err("open and save the project with this KNXBench version first; naming recipes never migrate stores".into());
    }
    let editor = history::load_editor(&readonly.conn).map_err(|e| e.to_string())?;
    let (mut snapshot, mut stack, generation) = match editor {
        Some(e) => (
            e.working,
            CommandStack::from_snapshot_states(e.undo, e.redo),
            e.generation,
        ),
        None => (
            NativeSnapshot::read(&readonly.conn).map_err(|e| e.to_string())?,
            CommandStack::new(),
            0,
        ),
    };
    let path_hash = knx_etsproj::opaque::sha256_hex(canonical.as_os_str().as_encoded_bytes());
    let authority = format!("cli-name-v1:{path_hash}:{generation}");
    let revision = NameRecipeRevision {
        project_incarnation: format!("cli-name-v1:{path_hash}"),
        generation: generation
            .try_into()
            .map_err(|_| "invalid native history generation")?,
    };
    let plan = recipe::preview_name_recipe(&snapshot, &revision, args.intent)
        .map_err(|e| e.to_string())?;
    let token = plan
        .confirmation_token(&authority)
        .map_err(|e| e.to_string())?;
    if let Some(confirmation) = args.confirm {
        if confirmation != token {
            return Err(
                "confirmation is stale or belongs to another target/intent; preview again".into(),
            );
        }
        drop(readonly);
        let outcome = recipe::apply_name_recipe(
            &mut snapshot,
            &mut stack,
            &revision,
            &plan,
            Some(NameRecipeNativeTarget {
                path: &canonical,
                history_generation: generation,
            }),
        )
        .map_err(|e| e.to_string())?;
        println!(
            "{}",
            json!({"applied":true,"changed":outcome.changed,"nativeGeneration":outcome.native_generation,"persistence":"native-working-history","preview":plan.preview()})
        );
    } else {
        println!(
            "{}",
            json!({"applied":false,"preview":plan.preview(),"confirmationToken":token,"persistence":"native-working-history"})
        );
    }
    Ok(())
}
