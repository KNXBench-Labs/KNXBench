//! `knx products legacy-password set|forget|status`: the one remembered legacy password.

use std::process::ExitCode;

use knx_app::legacy::{default_remembered_password_path, RememberedPassword};

use crate::legacy_inspect::{parse_password_args, read_password};

pub(crate) fn store() -> Result<RememberedPassword, String> {
    default_remembered_password_path()
        .map(RememberedPassword::at)
        .ok_or_else(|| {
            "neither XDG_CONFIG_HOME nor HOME is set, so there is no place to remember a password"
                .to_string()
        })
}

pub(crate) fn run(args: &[String]) -> ExitCode {
    match run_inner(args) {
        Ok(message) => {
            println!("{message}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

fn run_inner(args: &[String]) -> Result<String, String> {
    let (command, rest) = args
        .split_first()
        .ok_or_else(|| format!("missing set, forget or status\n{}", super::USAGE))?;
    let store = store()?;
    let path = store.path().display().to_string();
    match command.as_str() {
        "set" => {
            let (extra, source) = parse_password_args(rest)?;
            if let Some(extra) = extra {
                return Err(format!("unexpected extra argument: {extra}"));
            }
            let password = read_password(&source)?
                .ok_or("give the password with --password-stdin or --password-file <path>")?;
            store.store(&password).map_err(|e| e.to_string())?;
            Ok(format!(
                "remembered the legacy password in {path} (mode 0600); \
                 `knx products legacy-password forget` removes it"
            ))
        }
        "forget" if rest.is_empty() => Ok(if store.forget().map_err(|e| e.to_string())? {
            format!("forgot the remembered legacy password ({path})")
        } else {
            format!("no legacy password was remembered ({path})")
        }),
        "status" if rest.is_empty() => Ok(match store.load().map_err(|e| e.to_string())? {
            Some(_) => format!("a legacy password is remembered ({path})"),
            None => format!("no legacy password is remembered ({path})"),
        }),
        "forget" | "status" => Err(format!("{command} takes no arguments")),
        other => Err(format!("unknown legacy-password command: {other}")),
    }
}
