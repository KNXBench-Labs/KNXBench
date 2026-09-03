//! Repository verification tasks that are too specific for a Cargo lint.
//!
//! These are architectural rules that would otherwise erode silently, so they
//! run in CI rather than living in a document.

mod layering;

use std::process::ExitCode;

const AVAILABLE_TASKS: &str = "check-layering, freeze-fixture <path>";

fn main() -> ExitCode {
    let task = std::env::args().nth(1);
    match task.as_deref() {
        Some("check-layering") => check_layering(),
        Some("freeze-fixture") => freeze_fixture(std::env::args().nth(2)),
        Some(other) => {
            eprintln!("unknown task: {other}");
            eprintln!("available tasks: {AVAILABLE_TASKS}");
            ExitCode::FAILURE
        }
        None => {
            eprintln!("usage: cargo run -p xtask -- <task>");
            eprintln!("available tasks: {AVAILABLE_TASKS}");
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

    let mut violations =
        layering::forbidden_reachable(&graph, "knx-core", layering::CORE_FORBIDDEN);
    // knx-app (Task 22) is deliberately the only crate that sees both
    // knx-etsproj and knx-store, so the OpaqueEntry -> StoredOpaqueEntry
    // conversion has exactly one home; knx-etsproj reaching knx-store
    // directly would mean the import/export crate had grown a storage
    // dependency of its own.
    violations.extend(layering::forbidden_reachable(
        &graph,
        "knx-etsproj",
        &["knx-store"],
    ));
    // knx-productdb owns its own parser and its own store (ADR-0011). It
    // must not reach the project-import crate or the project store: a
    // .knxprod ingest added later must not have to travel through the
    // .knxproj importer, and product data must stay separable from project
    // files (ADR-0005).
    violations.extend(layering::forbidden_reachable(
        &graph,
        "knx-productdb",
        &["knx-etsproj", "knx-store"],
    ));
    // knx-projection turns Project into display-shaped structs for the
    // desktop UI (ADR-0009, Session 5). It must stay exactly as free of IO
    // and storage as knx-core itself — a projection layer that reached
    // rusqlite or quick-xml directly would defeat the point of having one.
    violations.extend(layering::forbidden_reachable(
        &graph,
        "knx-projection",
        layering::CORE_FORBIDDEN,
    ));

    if violations.is_empty() {
        println!(
            "layering ok: knx-core reaches none of {:?}; knx-etsproj does not reach knx-store; \
             knx-productdb reaches neither knx-etsproj nor knx-store; knx-projection reaches \
             none of {:?}",
            layering::CORE_FORBIDDEN,
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
         storage or async runtime (architecture spec, section 3.1). \
         knx-etsproj must not depend on knx-store: the conversion between \
         OpaqueEntry and StoredOpaqueEntry belongs in knx-app alone."
    );
    ExitCode::FAILURE
}

/// Creates (or migrates, if it already exists) a SQLite file at `path` and
/// `VACUUM`s it, so a committed migration-test fixture is minimal and
/// reproducible rather than carrying whatever incidental page layout a
/// fresh `Connection::open` happened to leave behind.
fn freeze_fixture(path: Option<String>) -> ExitCode {
    let Some(path) = path else {
        eprintln!("usage: cargo run -p xtask -- freeze-fixture <path>");
        return ExitCode::FAILURE;
    };
    let path = std::path::Path::new(&path);

    let conn = match knx_store::open_and_migrate(path) {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("failed to open/migrate {}: {e}", path.display());
            return ExitCode::FAILURE;
        }
    };
    if let Err(e) = conn.execute_batch("VACUUM;") {
        eprintln!("failed to vacuum {}: {e}", path.display());
        return ExitCode::FAILURE;
    }

    println!("froze fixture at {}", path.display());
    ExitCode::SUCCESS
}
