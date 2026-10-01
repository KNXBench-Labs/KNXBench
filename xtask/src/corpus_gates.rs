//! The `check-corpus-gates` lint: a missing private corpus must never read as a pass.
//!
//! The local `OriginalData/` corpus is gitignored, so every CI machine runs
//! without it. A test that answers a missing corpus with `return` is counted
//! as *passed* there while it exercised nothing; that idiom once covered 90
//! early-return sites (docs/KNOWN_LIMITATIONS.md §131). The honest form is `#[ignore =
//! "..."]` plus an `assert!` on the corpus probe, so a run without
//! `--ignored` reports the test as ignored, and a run with it fails loudly
//! when the corpus is absent.
//!
//! This lint is deliberately textual. It flags two spellings of the idiom:
//!
//! 1. a negated corpus probe (`if !corpus_available()`,
//!    `if !reference_*_path().exists()`, `if !oracle_dump_path().exists()`)
//!    whose block returns, on the same line or within the next few;
//! 2. a `"skip…"` message that names the private data (`OriginalData`,
//!    `corpus`, `project_dump`) followed by a `return` in the same block, which
//!    catches probes this list does not know by name (`if !package.exists()`).
//!
//! It cannot prove a test honest; it keeps the known dishonest shapes from
//! coming back. Network-capability skips (`knx-net`'s "skipping …: no route")
//! are a different question and are not matched.

use std::fs;
use std::path::{Path, PathBuf};

/// Calls that ask "is the private corpus here?". A negation of any of them
/// followed by an early `return` is the idiom this lint rejects.
const PROBES: &[&str] = &[
    "corpus_available()",
    "reference_ets4_path().exists()",
    "reference_ets6_path().exists()",
    "reference_kv_schema21_path().exists()",
    "reference_project_path().exists()",
    "oracle_dump_path().exists()",
];

/// Words that mark a skip message as being about the private data.
const CORPUS_WORDS: &[&str] = &["OriginalData", "corpus", "project_dump"];

/// How many lines after the `if` (or the skip message) may hold the
/// `return`: the idiom is an `eprintln!`, possibly wrapped over a few lines,
/// and then `return`.
const RETURN_WINDOW: usize = 6;

/// Directories never descended into, by name, wherever they appear.
const SKIP_DIRS: &[&str] = &["target", "node_modules", "dist", ".git"];

/// Workspace directories the lint walks.
const SCAN_ROOTS: &[&str] = &["apps", "crates"];

/// 1-based line numbers of every early return on missing private data in
/// `source`: the line of the negated probe, or of the skip message when the
/// probe is not one [`PROBES`] knows.
pub fn violations(source: &str) -> Vec<usize> {
    let lines: Vec<&str> = source.lines().collect();
    let mut found = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        let negated_probe =
            trimmed.starts_with("if !") && PROBES.iter().any(|probe| trimmed.contains(probe));
        if negated_probe && (trimmed.contains("return") || returns_soon(&lines, index)) {
            found.push(index + 1);
            continue;
        }
        let corpus_skip_message =
            trimmed.contains("\"skip") && CORPUS_WORDS.iter().any(|word| trimmed.contains(word));
        let already_reported = found
            .last()
            .is_some_and(|&last| index + 1 - last <= RETURN_WINDOW);
        if corpus_skip_message && !already_reported && returns_soon(&lines, index) {
            found.push(index + 1);
        }
    }
    found
}

/// True when a `return` statement follows line `index` within
/// [`RETURN_WINDOW`] lines, before the enclosing block closes.
fn returns_soon(lines: &[&str], index: usize) -> bool {
    lines
        .iter()
        .skip(index + 1)
        .take(RETURN_WINDOW)
        .map(|l| l.trim_start())
        .take_while(|l| !l.starts_with('}'))
        .any(|l| l.starts_with("return"))
}

/// Actual source coverage and violations, in deterministic path order.
#[derive(Debug, Default)]
pub struct Report {
    pub files_scanned: usize,
    pub violations: Vec<(PathBuf, usize)>,
}

/// Scan each required source directory; an absent or empty one is not green.
pub fn scan(root: &Path) -> Result<Report, String> {
    let mut report = Report::default();
    for top in SCAN_ROOTS {
        let dir = root.join(top);
        let before = report.files_scanned;
        walk(root, &dir, &mut report)?;
        if report.files_scanned == before {
            return Err(format!(
                "empty corpus gate scan: no Rust sources under {}",
                dir.display()
            ));
        }
    }
    Ok(report)
}

fn walk(root: &Path, dir: &Path, report: &mut Report) -> Result<(), String> {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .map_err(|e| format!("cannot read {}: {e}", dir.display()))?
        .map(|entry| entry.map(|e| e.path()))
        .collect::<Result<_, _>>()
        .map_err(|e| format!("cannot read an entry of {}: {e}", dir.display()))?;
    entries.sort();
    for path in entries {
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if !SKIP_DIRS.contains(&name) {
                walk(root, &path, report)?;
            }
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let source = fs::read_to_string(&path)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        report.files_scanned += 1;
        let relative = path.strip_prefix(root).unwrap_or(&path).to_path_buf();
        for line in violations(&source) {
            report.violations.push((relative.clone(), line));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_rejects_empty_source_directories() {
        let root = tempfile::tempdir().unwrap();
        for top in SCAN_ROOTS {
            fs::create_dir(root.path().join(top)).unwrap();
        }
        assert!(
            scan(root.path()).is_err(),
            "empty source coverage must fail"
        );
    }

    #[test]
    fn flags_the_silent_return_idiom() {
        let source = "#[test]\nfn t() {\n    if !reference_ets4_path().exists() {\n        \
                      eprintln!(\"skip: OriginalData/ corpus not present\");\n        return;\n    \
                      }\n}\n";
        assert_eq!(violations(source), vec![3]);
    }

    #[test]
    fn flags_a_wrapped_message_and_a_return_with_a_value() {
        let source = "fn f() -> Option<u8> {\n    if !knx_testsupport::corpus_available() {\n        \
                      eprintln!(\n            \"skip\"\n        );\n        return None;\n    }\n}\n";
        assert_eq!(violations(source), vec![2]);
    }

    #[test]
    fn accepts_the_honest_assert() {
        let source =
            "#[test]\n#[ignore = \"requires the corpus\"]\nfn t() {\n    assert!(\n        \
                      reference_ets4_path().exists(),\n        \"corpus missing\"\n    );\n}\n";
        assert!(violations(source).is_empty());
    }

    #[test]
    fn does_not_look_past_the_end_of_the_block() {
        // The `return` belongs to later code, not to the probe's block.
        let source = "if !corpus_available() {\n    panic!(\"missing\");\n}\nreturn;\n";
        assert!(violations(source).is_empty());
    }

    #[test]
    fn flags_a_one_line_guard_and_the_oracle_probe() {
        let one_line = "if !corpus_available() { return; }\n";
        assert_eq!(violations(one_line), vec![1]);
        let oracle = "if !oracle_dump_path().exists() {\n    eprintln!(\"skip: dump\");\n    \
                      return;\n}\n";
        assert_eq!(violations(oracle), vec![1]);
    }

    #[test]
    fn flags_an_unknown_probe_by_its_skip_message() {
        // `package.exists()` is no probe the lint knows by name; the message
        // saying what is missing gives the idiom away.
        let source = "let package = corpus_root().join(PACKAGE);\nif !package.exists() {\n    \
                      eprintln!(\n        \"skip: {PACKAGE} not present (OriginalData/ is \
                      gitignored)\"\n    );\n    return;\n}\n";
        assert_eq!(violations(source), vec![4]);
    }

    #[test]
    fn reports_a_known_probe_once_not_again_for_its_message() {
        let source = "if !corpus_available() {\n    eprintln!(\"skip: OriginalData/ corpus \
                      missing\");\n    return;\n}\n";
        assert_eq!(violations(source), vec![1]);
    }

    #[test]
    fn ignores_network_capability_skips() {
        let source = "if probe.connect(ADDR).await.is_err() {\n    eprintln!(\"skipping t: no \
                      route in this sandbox\");\n    return;\n}\n";
        assert!(violations(source).is_empty());
    }

    #[test]
    fn ignores_unrelated_negations() {
        let source = "if !path.exists() {\n    return;\n}\n";
        assert!(violations(source).is_empty());
    }
}
