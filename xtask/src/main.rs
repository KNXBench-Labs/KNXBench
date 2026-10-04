//! Repository verification tasks that are too specific for a Cargo lint.
//!
//! These are architectural rules that would otherwise erode silently, so they
//! run in CI rather than living in a document.

mod anchors;
mod appimage;
mod corpus_gates;
mod headers;
mod layering;
mod ledger;
mod scope;

use std::path::Path;
use std::process::ExitCode;

const AVAILABLE_TASKS: &str =
    "check-layering, check-headers, check-anchors, check-ledger, check-corpus-gates, \
     check-appimage, freeze-fixture <path>";

fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let explicit_root = if args.first().is_some_and(|arg| arg == "--root") {
        if args.len() < 3 {
            eprintln!("usage: xtask [--root PATH] <task>");
            return ExitCode::FAILURE;
        }
        let root = args.remove(1);
        args.remove(0);
        Some(root)
    } else {
        None
    };
    let task = args.first().map(String::as_str);
    if task.is_some_and(|task| {
        matches!(
            task,
            "check-layering"
                | "check-headers"
                | "check-anchors"
                | "check-ledger"
                | "check-corpus-gates"
        )
    }) && args.len() != 1
    {
        eprintln!("usage: xtask [--root PATH] <task>; this gate accepts no task options");
        return ExitCode::FAILURE;
    }
    let root = if task.is_some_and(|task| {
        matches!(
            task,
            "check-layering"
                | "check-headers"
                | "check-anchors"
                | "check-ledger"
                | "check-corpus-gates"
                | "check-appimage"
        )
    }) {
        match scope::workspace_root(explicit_root.as_deref()) {
            Ok(root) => {
                println!("gate target: {}", root.display());
                Some(root)
            }
            Err(error) => {
                eprintln!("{error}");
                return ExitCode::FAILURE;
            }
        }
    } else {
        None
    };
    let gate_root = root.as_deref().unwrap_or_else(|| Path::new(""));
    match task {
        Some("check-layering") => check_layering(gate_root),
        Some("check-headers") => check_headers(gate_root),
        Some("check-anchors") => check_anchors(gate_root),
        Some("check-ledger") => check_ledger(gate_root),
        Some("check-corpus-gates") => check_corpus_gates(gate_root),
        Some("check-appimage") => check_appimage(gate_root, &args[1..]),
        Some("freeze-fixture") => freeze_fixture(args.get(1).cloned()),
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

fn check_appimage(root: &Path, options: &[String]) -> ExitCode {
    const USAGE: &str =
        "usage: cargo run -p xtask -- check-appimage [--artifact-dir PATH] [--tag vVERSION]";

    let default_artifact_dir = root.join("target/release/bundle/appimage");
    let mut artifact_dir = default_artifact_dir.clone();
    let mut artifact_dir_set = false;
    let mut tag = None;
    let mut args = options.iter().cloned();

    while let Some(option) = args.next() {
        let value = match option.as_str() {
            "--artifact-dir" | "--tag" => args.next(),
            _ => None,
        };
        let Some(value) = value else {
            eprintln!("{USAGE}");
            return ExitCode::FAILURE;
        };
        match option.as_str() {
            "--artifact-dir" if !artifact_dir_set => {
                artifact_dir = value.into();
                artifact_dir_set = true;
            }
            "--tag" if tag.is_none() => tag = Some(value),
            _ => {
                eprintln!("{USAGE}");
                return ExitCode::FAILURE;
            }
        }
    }

    match appimage::verify(root, &artifact_dir, tag.as_deref()) {
        Ok(artifact) => {
            println!(
                "AppImage ok: version {}, artifact {}",
                artifact.version,
                artifact.path.display()
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn check_layering(root: &Path) -> ExitCode {
    let graph = match layering::workspace_graph(root) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };

    println!("layering scope: {} resolved packages", graph.edges.len());

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
    // knx-etsproj's import validation (stage 4, T06) must stay a pure
    // function of one parsed document. Reaching knx-productdb would let a
    // cross-database check sneak into what is supposed to be a
    // single-document validation pass.
    violations.extend(layering::forbidden_reachable(
        &graph,
        "knx-etsproj",
        &["knx-productdb"],
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
            "layering ok: knx-core reaches none of {:?}; knx-etsproj reaches neither knx-store \
             nor knx-productdb; knx-productdb reaches neither knx-etsproj nor knx-store; \
             knx-projection reaches \
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

/// Fails on any test that answers a missing private corpus with an early
/// `return`, which counts as a pass on every machine without the corpus
/// (docs/KNOWN_LIMITATIONS.md §131). See `corpus_gates.rs` for the rule.
fn check_corpus_gates(root: &Path) -> ExitCode {
    let report = match corpus_gates::scan(root) {
        Ok(report) => report,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    if report.violations.is_empty() {
        println!(
            "corpus gates ok: {} Rust files scanned, no silent early return on a missing corpus",
            report.files_scanned
        );
        return ExitCode::SUCCESS;
    }
    for (path, line) in &report.violations {
        eprintln!(
            "corpus gate violation: {}:{line}: a missing corpus returns early (a silent pass)",
            path.display()
        );
    }
    eprintln!(
        "use #[ignore = \"requires the gitignored OriginalData/ corpus; run with --ignored\"] and \
         assert! the corpus probe instead of returning (docs/KNOWN_LIMITATIONS.md §131)"
    );
    ExitCode::FAILURE
}

/// Walks `apps/`, `crates/` and `xtask/` and fails on any file whose
/// first-line header breaks the ADR-0018 grammar, or when the number of
/// files *without* a header rises above `headers::ABSENT_CEILING`. The
/// second check is the ratchet: it is what makes "a file created or
/// edited from now on gets a header" a rule rather than a wish, without
/// forcing a repo-wide sweep — the count may only ever go down.
fn check_headers(root: &Path) -> ExitCode {
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

/// Walks `docs/**/*.md` plus the repo-root markdown files and fails on any
/// in-repo `[text](path#anchor)` or `[text](#anchor)` link whose anchor does
/// not resolve — either the target file does not exist, or it exists but
/// carries no heading slug and no `<a id="…">` alias matching the anchor.
/// A dead anchor used to survive an entire doc-cleanup branch unnoticed
/// (see the fix-round history around ADR-0018 and `KNOWN_LIMITATIONS.md`);
/// this is the check that makes that a compile-time, not a review-time,
/// discovery from now on.
fn check_ledger(root: &Path) -> ExitCode {
    let report = match ledger::check(root) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    if report.problems.is_empty() {
        println!("ledger ok: {} rows in {}", report.rows, ledger::LEDGER);
        return ExitCode::SUCCESS;
    }
    for p in &report.problems {
        eprintln!("ledger: {}:{}: {}", p.file.display(), p.line, p.message);
    }
    eprintln!(
        "\n{} ledger problem(s); see ADR-0076.",
        report.problems.len()
    );
    ExitCode::FAILURE
}

fn check_anchors(root: &Path) -> ExitCode {
    let report = match anchors::scan(root) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };

    if report.dead.is_empty() {
        println!(
            "anchors ok: {} links checked across {} markdown files, none dead",
            report.links_checked, report.files_scanned
        );
        return ExitCode::SUCCESS;
    }

    for dead in &report.dead {
        match &dead.kind {
            anchors::DeadKind::MissingFile => {
                eprintln!(
                    "dead anchor: {}:{}: `{}` — target file does not exist",
                    dead.file.display(),
                    dead.line,
                    dead.raw_link
                );
            }
            anchors::DeadKind::MissingAnchor { suggestion } => {
                let hint = match suggestion {
                    Some(s) => format!(" (nearest live anchor: `{s}`)"),
                    None => String::new(),
                };
                eprintln!(
                    "dead anchor: {}:{}: `{}` — no matching heading or `<a id>`{hint}",
                    dead.file.display(),
                    dead.line,
                    dead.raw_link
                );
            }
        }
    }
    eprintln!(
        "\n{} of {} links across {} markdown files are dead. A heading rename needs a \
         back-compat `<a id=\"…\">` alias for its old slug (see `docs/KNOWN_LIMITATIONS.md`'s \
         own convention), and a moved file needs every link that points at it updated.",
        report.dead.len(),
        report.links_checked,
        report.files_scanned
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
