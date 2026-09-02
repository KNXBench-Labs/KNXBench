//! Repository verification tasks that are too specific for a Cargo lint.
//!
//! These are architectural rules that would otherwise erode silently, so they
//! run in CI rather than living in a document.

mod layering;

use std::process::ExitCode;

fn main() -> ExitCode {
    let task = std::env::args().nth(1);
    match task.as_deref() {
        Some("check-layering") => check_layering(),
        Some(other) => {
            eprintln!("unknown task: {other}");
            eprintln!("available tasks: check-layering");
            ExitCode::FAILURE
        }
        None => {
            eprintln!("usage: cargo run -p xtask -- <task>");
            eprintln!("available tasks: check-layering");
            ExitCode::FAILURE
        }
    }
}

fn check_layering() -> ExitCode {
    let graph = match layering::workspace_graph() {
        Ok(g) => g,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };

    let violations = layering::forbidden_reachable(&graph, "knx-core", layering::CORE_FORBIDDEN);

    if violations.is_empty() {
        println!(
            "layering ok: knx-core reaches none of {:?}",
            layering::CORE_FORBIDDEN
        );
        return ExitCode::SUCCESS;
    }

    for v in &violations {
        eprintln!(
            "layering violation: {} reaches forbidden package {} via {}",
            v.root,
            v.forbidden,
            v.path.join(" -> ")
        );
    }
    eprintln!(
        "\nknx-core must perform no IO and must not depend on any format, \
         storage or async runtime. See the architecture spec, section 3.1."
    );
    ExitCode::FAILURE
}
