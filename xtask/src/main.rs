//! Repository verification tasks that are too specific for a Cargo lint.
//!
//! These are architectural rules that would otherwise erode silently, so they
//! run in CI rather than living in a document.

mod headers;
mod layering;

use std::path::Path;
use std::process::ExitCode;

const AVAILABLE_TASKS: &str = "check-layering, check-headers, freeze-fixture <path>";

fn main() -> ExitCode {
    let task = std::env::args().nth(1);
    match task.as_deref() {
        Some("check-layering") => check_layering(),
        Some("check-headers") => check_headers(),
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
    // knx-csv (T12) is the group-address CSV reader/writer. It stays a pure
    // text-in/typed-rows-out crate: it must not reach the project store or
    // either of the other import/export crates, so that reading a
    // spreadsheet never drags SQLite or ZIP/XML parsing along for the ride.
    violations.extend(layering::forbidden_reachable(
        &graph,
        "knx-csv",
        &["knx-store", "knx-etsproj", "knx-productdb"],
    ));
    // knx-report (T13) renders the project-documentation HTML export. Like
    // knx-csv it stays a pure text-out crate: no project store, no
    // import/export archive parser, no product database. And like
    // knx-projection, which it depends on to resolve communication-object
    // text, it stays exactly as free of IO and storage as knx-core itself —
    // a document renderer has no business with a database, an archive
    // parser, or an async runtime.
    violations.extend(layering::forbidden_reachable(
        &graph,
        "knx-report",
        &["knx-store", "knx-etsproj", "knx-productdb"],
    ));
    violations.extend(layering::forbidden_reachable(
        &graph,
        "knx-report",
        layering::CORE_FORBIDDEN,
    ));
    // knx-diff (T14) computes the "what changed between two saves" project
    // comparison. Like knx-csv and knx-report it stays a pure crate: no
    // project store, no import/export archive parser, no product database
    // — and, like knx-projection and knx-report, it stays exactly as free of
    // IO and storage as knx-core itself. A comparison engine has no business
    // with a database, an archive parser, or an async runtime.
    violations.extend(layering::forbidden_reachable(
        &graph,
        "knx-diff",
        &["knx-store", "knx-etsproj", "knx-productdb"],
    ));
    violations.extend(layering::forbidden_reachable(
        &graph,
        "knx-diff",
        layering::CORE_FORBIDDEN,
    ));
    // knx-secure (ADR-0008, ARCHITECTURE.md §9) must never reach knx-core, so
    // no type path exists along which key material could reach the project
    // model, and must never reach serde, so no knx-secure type can gain a
    // Serialize/Deserialize impl. This used to hold by construction because
    // the crate had no [dependencies] section at all; it grew one (A6, the
    // .knxproj ZIP-password derivation), so the invariant is checked here
    // from now on instead of assumed from the manifest's shape.
    violations.extend(layering::forbidden_reachable(
        &graph,
        "knx-secure",
        layering::SECURE_FORBIDDEN,
    ));

    if violations.is_empty() {
        println!(
            "layering ok: knx-core reaches none of {:?}; knx-etsproj does not reach knx-store; \
             knx-productdb reaches neither knx-etsproj nor knx-store; knx-projection reaches \
             none of {:?}; knx-csv reaches none of knx-store, knx-etsproj, knx-productdb; \
             knx-report reaches none of knx-store, knx-etsproj, knx-productdb, or {:?}; \
             knx-diff reaches none of knx-store, knx-etsproj, knx-productdb, or {:?}; \
             knx-secure reaches none of {:?}",
            layering::CORE_FORBIDDEN,
            layering::CORE_FORBIDDEN,
            layering::CORE_FORBIDDEN,
            layering::CORE_FORBIDDEN,
            layering::SECURE_FORBIDDEN
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

/// Walks `apps/`, `crates/` and `xtask/` and fails on any file whose
/// first-line header breaks the ADR-0018 grammar, or when the number of
/// files *without* a header rises above `headers::ABSENT_CEILING`. The
/// second check is the ratchet: it is what makes "a file created or
/// edited from now on gets a header" a rule rather than a wish, without
/// forcing a repo-wide sweep — the count may only ever go down.
fn check_headers() -> ExitCode {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask lives one level below the workspace root");
    let report = match headers::scan(root) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };

    for (path, why) in &report.invalid {
        eprintln!("header violation: {}: {why}", path.display());
    }
    let ratchet = headers::ratchet_violation(&report);
    if let Some(why) = &ratchet {
        eprintln!("header violation: {why}");
    }
    if report.invalid.is_empty() && ratchet.is_none() {
        println!(
            "headers ok: {} files with a well-formed header, {} without one (ceiling {}), \
             {} generated files skipped",
            report.ok.len(),
            report.absent.len(),
            headers::ABSENT_CEILING,
            report.generated.len()
        );
        return ExitCode::SUCCESS;
    }
    eprintln!(
        "\nA header is the file's first line and nothing else: `//! One sentence.` in \
         Rust, `/** One sentence. */` in TypeScript, at most {} columns, ending in a \
         single period, with no second sentence (docs/adr/0018). A file without a \
         header is fine, up to the ceiling; a header that does not follow the grammar is \
         not.",
        headers::MAX_WIDTH
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
