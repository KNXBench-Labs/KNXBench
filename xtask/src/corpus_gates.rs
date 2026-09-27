//! The `check-corpus-gates` lint: a missing private corpus must never read as a pass.
//!
//! The local `OriginalData/` corpus is gitignored, so every CI machine runs
//! without it. A test that answers a missing corpus with `return` is counted
//! as *passed* there while it exercised nothing; that idiom once covered 72
//! tests (docs/KNOWN_LIMITATIONS.md §131). The honest form is `#[ignore =
//! "..."]` plus an `assert!` on the corpus probe, so a run without
//! `--ignored` reports the test as ignored, and a run with it fails loudly
//! when the corpus is absent.
//!
//! This lint is deliberately textual and narrow: it flags a negated corpus
//! probe (`if !corpus_available()`, `if !reference_*_path().exists()`) whose
//! block returns within the next few lines. It cannot prove a test honest,
//! only keep the one known dishonest shape from coming back.

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
];

/// How many lines after the `if` may hold the `return`: the idiom is an
/// `eprintln!` (possibly wrapped over a few lines) and then `return`.
const RETURN_WINDOW: usize = 4;

/// Directories never descended into, by name, wherever they appear.
const SKIP_DIRS: &[&str] = &["target", "node_modules", "dist", ".git"];

/// Workspace directories the lint walks.
const SCAN_ROOTS: &[&str] = &["apps", "crates"];

/// 1-based line numbers of every negated corpus probe in `source` whose
/// block returns early.
pub fn violations(source: &str) -> Vec<usize> {
    let lines: Vec<&str> = source.lines().collect();
    let mut found = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        if !trimmed.starts_with("if !") || !PROBES.iter().any(|probe| trimmed.contains(probe)) {
            continue;
        }
        let returns_early = lines
            .iter()
            .skip(index + 1)
            .take(RETURN_WINDOW)
            .map(|l| l.trim_start())
            .take_while(|l| !l.starts_with('}'))
            .any(|l| l.starts_with("return"));
        if returns_early {
            found.push(index + 1);
        }
    }
    found
}

/// Every violation under `root`, as `(path relative to root, line)`, in
/// sorted path order.
pub fn scan(root: &Path) -> Result<Vec<(PathBuf, usize)>, String> {
    let mut found = Vec::new();
    for top in SCAN_ROOTS {
        let dir = root.join(top);
        if dir.is_dir() {
            walk(root, &dir, &mut found)?;
        }
    }
    Ok(found)
}

fn walk(root: &Path, dir: &Path, found: &mut Vec<(PathBuf, usize)>) -> Result<(), String> {
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
                walk(root, &path, found)?;
            }
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let source = fs::read_to_string(&path)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        let relative = path.strip_prefix(root).unwrap_or(&path).to_path_buf();
        for line in violations(&source) {
            found.push((relative.clone(), line));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn ignores_unrelated_negations() {
        let source = "if !path.exists() {\n    return;\n}\n";
        assert!(violations(source).is_empty());
    }
}
