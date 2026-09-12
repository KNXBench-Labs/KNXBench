//! The `check-headers` lint: a file's first line must say what the file is for.
//!
//! ADR-0018 defines the header: one sentence on line 1, as a `//!` module
//! doc in Rust and a single-line `/** ... */` block in TypeScript. This
//! module checks the *shape* of a header wherever one exists. It does not
//! demand that every file has one — the convention spreads as files are
//! created or edited, not by a repo-wide sweep — so a file without a
//! header is counted, never failed. A pre-convention module doc (one whose
//! first paragraph runs past line 1) is, by construction, not a header
//! either; it becomes one the next time someone edits the file.
//!
//! What this lint cannot do, and does not pretend to: judge whether the
//! sentence is *true*, or whether it still describes the file. Review does
//! that. See the ADR for why there is no per-file version to check.

use std::fs;
use std::path::{Path, PathBuf};

/// Widest a header line may be, in characters — rustfmt's default
/// `max_width`, so a header never wraps in an editor the rest of the file
/// fits in.
pub const MAX_WIDTH: usize = 100;

/// Directories never descended into, by name, wherever they appear.
const SKIP_DIRS: &[&str] = &["target", "node_modules", "dist", ".git"];

/// Directories holding generated files, relative to the workspace root.
/// `ts-rs` writes `apps/knx-web/src/bindings/`; its first line is its
/// own "do not edit" banner, which is neither a header nor a mistake.
const GENERATED_DIRS: &[&str] = &["apps/knx-web/src/bindings"];

/// Workspace directories the lint walks. Deliberately not the repository
/// root: `tools/` holds Python, `docs/` holds Markdown, neither has a
/// header grammar here.
const SCAN_ROOTS: &[&str] = &["apps", "crates", "xtask"];

/// Outcome of inspecting one file's first line.
#[derive(Debug, PartialEq, Eq)]
pub enum Header {
    /// Nothing in the convention's shape on line 1: no comment at all, a
    /// pre-convention multi-line module doc, or a docblock pragma.
    Absent,
    /// A header in the convention's shape, with a well-formed sentence.
    Ok,
    /// Something shaped like a header that breaks the grammar. The string
    /// says how, for the person who has to fix it.
    Invalid(String),
}

/// Which grammar applies to a file, by extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Rust,
    TypeScript,
}

impl Language {
    pub fn of(path: &Path) -> Option<Language> {
        match path.extension().and_then(|e| e.to_str()) {
            Some("rs") => Some(Language::Rust),
            Some("ts" | "tsx") => Some(Language::TypeScript),
            _ => None,
        }
    }
}

/// Checks a Rust source file. The header is line 1 as `//! <sentence>`,
/// and line 2 must not continue the paragraph — a `//!` line with text
/// on it means this is a pre-convention module doc, which is `Absent`,
/// not `Invalid`. A blank `//!` line or any non-doc line on line 2 is
/// fine; the rest of the module doc is free-form.
pub fn check_rust(source: &str) -> Header {
    let mut lines = source.lines();
    let Some(first) = lines.next() else {
        return Header::Absent;
    };
    let Some(body) = first.strip_prefix("//!") else {
        return Header::Absent;
    };
    if let Some(second) = lines.next() {
        if second.starts_with("//!") && second.trim_end() != "//!" {
            return Header::Absent;
        }
    }
    let Some(text) = body.strip_prefix(' ') else {
        return Header::Invalid("expected exactly one space after `//!`".to_string());
    };
    check_sentence(first, text)
}

/// Checks a TypeScript source file. The header is line 1 as
/// `/** <sentence> */`, opened and closed on that one line. A block
/// comment that does not close on line 1 is not a header; neither is a
/// single-line docblock holding only a pragma such as
/// `/** @vitest-environment happy-dom */`.
pub fn check_ts(source: &str) -> Header {
    let Some(first) = source.lines().next() else {
        return Header::Absent;
    };
    let Some(body) = first.strip_prefix("/**") else {
        return Header::Absent;
    };
    let Some(body) = body.strip_suffix("*/") else {
        return Header::Absent;
    };
    if body.trim_start().starts_with('@') {
        return Header::Absent;
    }
    let (Some(text), true) = (body.strip_prefix(' '), body.ends_with(' ')) else {
        return Header::Invalid("expected exactly one space inside `/** ... */`".to_string());
    };
    let text = &text[..text.len() - 1];
    check_sentence(first, text)
}

/// The one grammar both languages share, applied to the sentence with the
/// comment framing already stripped: non-empty, no stray whitespace, ends
/// in exactly one period, contains no second sentence, and the whole
/// line fits in [`MAX_WIDTH`] columns.
///
/// "Second sentence" is a heuristic, stated so it can be worked around
/// rather than fought: a `.`, `!` or `?` followed by whitespace and an
/// ASCII capital letter. `e.g. Foo` trips it; `e.g. `foo`` and `v2.1
/// Bar`-style tokens do not. Keep abbreviations out of the header.
fn check_sentence(line: &str, text: &str) -> Header {
    let width = line.chars().count();
    if width > MAX_WIDTH {
        return Header::Invalid(format!(
            "header line is {width} columns wide, the limit is {MAX_WIDTH}"
        ));
    }
    if text.is_empty() {
        return Header::Invalid("header sentence is empty".to_string());
    }
    if text.trim() != text {
        return Header::Invalid("header sentence has leading or trailing whitespace".to_string());
    }
    if !text.ends_with('.') {
        return Header::Invalid("header sentence must end with a period".to_string());
    }
    if text.ends_with("..") {
        return Header::Invalid("header sentence ends with more than one period".to_string());
    }
    let chars: Vec<char> = text.chars().collect();
    for i in 0..chars.len() {
        if !matches!(chars[i], '.' | '!' | '?') {
            continue;
        }
        let mut j = i + 1;
        if j >= chars.len() || !chars[j].is_whitespace() {
            continue;
        }
        while j < chars.len() && chars[j].is_whitespace() {
            j += 1;
        }
        if j < chars.len() && chars[j].is_ascii_uppercase() {
            let first: String = chars[..=i].iter().collect();
            return Header::Invalid(format!(
                "header must be exactly one sentence (a second one seems to start after `{first}`)"
            ));
        }
    }
    Header::Ok
}

/// What a walk of the workspace found.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Report {
    /// Files with a well-formed header.
    pub ok: Vec<PathBuf>,
    /// Files with no header in the convention's shape. Counted, not failed.
    pub absent: Vec<PathBuf>,
    /// Files under a generated directory, never inspected.
    pub generated: Vec<PathBuf>,
    /// Files with a malformed header, and why. Any entry here fails the lint.
    pub invalid: Vec<(PathBuf, String)>,
}

/// Walks the workspace under `root` and checks every Rust and TypeScript
/// source file. Paths in the report are relative to `root`, in
/// deterministic (sorted) order.
pub fn scan(root: &Path) -> Result<Report, String> {
    let mut report = Report::default();
    for top in SCAN_ROOTS {
        let dir = root.join(top);
        if dir.is_dir() {
            walk(root, &dir, &mut report)?;
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
        let relative = path.strip_prefix(root).unwrap_or(&path).to_path_buf();
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if SKIP_DIRS.contains(&name) {
                continue;
            }
            if GENERATED_DIRS.iter().any(|g| relative == Path::new(g)) {
                collect_generated(&path, root, report)?;
                continue;
            }
            walk(root, &path, report)?;
            continue;
        }
        let Some(language) = Language::of(&path) else {
            continue;
        };
        let source = fs::read_to_string(&path)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        let result = match language {
            Language::Rust => check_rust(&source),
            Language::TypeScript => check_ts(&source),
        };
        match result {
            Header::Ok => report.ok.push(relative),
            Header::Absent => report.absent.push(relative),
            Header::Invalid(why) => report.invalid.push((relative, why)),
        }
    }
    Ok(())
}

fn collect_generated(dir: &Path, root: &Path, report: &mut Report) -> Result<(), String> {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .map_err(|e| format!("cannot read {}: {e}", dir.display()))?
        .map(|entry| entry.map(|e| e.path()))
        .collect::<Result<_, _>>()
        .map_err(|e| format!("cannot read an entry of {}: {e}", dir.display()))?;
    entries.sort();
    for path in entries {
        if path.is_dir() {
            collect_generated(&path, root, report)?;
        } else if Language::of(&path).is_some() {
            report
                .generated
                .push(path.strip_prefix(root).unwrap_or(&path).to_path_buf());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn invalid(h: Header) -> String {
        match h {
            Header::Invalid(why) => why,
            other => panic!("expected Invalid, got {other:?}"),
        }
    }

    // ---- Rust grammar ----

    #[test]
    fn rust_one_sentence_followed_by_blank_doc_line_is_ok() {
        let src =
            "//! Parses the thing.\n//!\n//! More detail here. Several sentences.\n\nuse x;\n";
        assert_eq!(check_rust(src), Header::Ok);
    }

    #[test]
    fn rust_one_sentence_followed_by_code_is_ok() {
        assert_eq!(check_rust("//! Parses the thing.\n\nuse x;\n"), Header::Ok);
        assert_eq!(check_rust("//! Parses the thing.\n"), Header::Ok);
        assert_eq!(check_rust("//! Parses the thing."), Header::Ok);
    }

    #[test]
    fn rust_multi_line_first_paragraph_is_pre_convention_not_an_error() {
        let src = "//! Headless entry point. Keeping a real CLI alongside\n//! the desktop app forces the core to stay UI-free.\n";
        assert_eq!(check_rust(src), Header::Absent);
    }

    #[test]
    fn rust_file_without_module_doc_is_absent() {
        assert_eq!(check_rust("use std::fs;\n"), Header::Absent);
        assert_eq!(
            check_rust("#![allow(dead_code)]\n//! Doc.\n"),
            Header::Absent
        );
        assert_eq!(check_rust(""), Header::Absent);
    }

    #[test]
    fn rust_two_sentences_on_line_one_is_invalid() {
        let why = invalid(check_rust("//! Parses the thing. It also frobs.\n//!\n"));
        assert!(why.contains("exactly one sentence"), "{why}");
        assert!(why.contains("Parses the thing."), "{why}");
    }

    #[test]
    fn rust_missing_period_is_invalid() {
        let why = invalid(check_rust("//! Parses the thing\n\nuse x;\n"));
        assert!(why.contains("end with a period"), "{why}");
    }

    #[test]
    fn rust_double_period_is_invalid() {
        let why = invalid(check_rust("//! Parses the thing..\n"));
        assert!(why.contains("more than one period"), "{why}");
    }

    #[test]
    fn rust_empty_or_padded_sentence_is_invalid() {
        assert!(invalid(check_rust("//! \n")).contains("empty"));
        assert!(invalid(check_rust("//!  Parses.\n")).contains("whitespace"));
        assert!(invalid(check_rust("//! Parses. \n")).contains("whitespace"));
        assert!(invalid(check_rust("//!Parses.\n")).contains("exactly one space"));
    }

    #[test]
    fn rust_overlong_line_is_invalid() {
        let long = format!("//! {}.\n", "x".repeat(MAX_WIDTH));
        let why = invalid(check_rust(&long));
        assert!(why.contains("columns wide"), "{why}");
        let exact = format!("//! {}.\n", "x".repeat(MAX_WIDTH - 5));
        assert_eq!(check_rust(&exact), Header::Ok);
    }

    #[test]
    fn rust_periods_inside_identifiers_and_before_lowercase_do_not_split() {
        assert_eq!(
            check_rust("//! Stage 5: maps a [`SourceDocument`] into a `knx_core::Project`.\n//!\n"),
            Header::Ok
        );
        assert_eq!(
            check_rust("//! Reads `.knxproj` files, i.e. zipped XML, into a document.\n"),
            Header::Ok
        );
        assert_eq!(
            check_rust("//! Core v01.06.02 AS §7 structures.\n"),
            Header::Ok
        );
    }

    #[test]
    fn rust_abbreviation_before_a_capital_is_treated_as_a_boundary() {
        // Documented heuristic: keep abbreviations out of the header.
        let why = invalid(check_rust("//! Reads formats, e.g. ETS ones.\n"));
        assert!(why.contains("exactly one sentence"), "{why}");
    }

    #[test]
    fn rust_question_or_exclamation_terminator_is_invalid() {
        assert!(invalid(check_rust("//! Why is this here?\n")).contains("end with a period"));
    }

    // ---- TypeScript grammar ----

    #[test]
    fn ts_single_line_docblock_is_ok() {
        assert_eq!(
            check_ts("/** Fetch-based client for the knx-server HTTP API. */\nimport x;\n"),
            Header::Ok
        );
        assert_eq!(check_ts("/** Fetch-based client. */"), Header::Ok);
    }

    #[test]
    fn ts_header_may_precede_a_vitest_pragma_line() {
        // vitest matches `@vitest-environment` anywhere in the file, so the
        // pragma can live on line 2 and the header keeps line 1.
        let src =
            "/** Tests for the toast queue. */\n// @vitest-environment happy-dom\nimport x;\n";
        assert_eq!(check_ts(src), Header::Ok);
    }

    #[test]
    fn ts_multi_line_block_is_absent() {
        assert_eq!(
            check_ts("/**\n * Something.\n */\nimport x;\n"),
            Header::Absent
        );
        assert_eq!(check_ts("/* Something\n */\n"), Header::Absent);
    }

    #[test]
    fn ts_line_comment_or_import_on_line_one_is_absent() {
        assert_eq!(
            check_ts("// @vitest-environment happy-dom\n"),
            Header::Absent
        );
        assert_eq!(
            check_ts("//! Rust-style doc borrowed by a TS file.\n"),
            Header::Absent
        );
        assert_eq!(check_ts("import x from \"y\";\n"), Header::Absent);
        assert_eq!(
            check_ts("/// <reference types=\"vite/client\" />\n"),
            Header::Absent
        );
        assert_eq!(check_ts(""), Header::Absent);
    }

    #[test]
    fn ts_pragma_only_docblock_is_absent() {
        assert_eq!(
            check_ts("/** @vitest-environment happy-dom */\n"),
            Header::Absent
        );
    }

    #[test]
    fn ts_two_sentences_is_invalid() {
        let why = invalid(check_ts("/** Fetches things. Also caches. */\n"));
        assert!(why.contains("exactly one sentence"), "{why}");
    }

    #[test]
    fn ts_missing_period_or_bad_padding_is_invalid() {
        assert!(invalid(check_ts("/** Fetches things */\n")).contains("end with a period"));
        assert!(invalid(check_ts("/**Fetches things. */\n")).contains("exactly one space"));
        assert!(invalid(check_ts("/** Fetches things.*/\n")).contains("exactly one space"));
        assert!(invalid(check_ts("/**  Fetches things. */\n")).contains("whitespace"));
    }

    #[test]
    fn ts_overlong_line_is_invalid() {
        let long = format!("/** {}. */\n", "x".repeat(MAX_WIDTH));
        assert!(invalid(check_ts(&long)).contains("columns wide"));
    }

    // ---- the walk ----

    fn write(root: &Path, rel: &str, content: &str) {
        let path = root.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    #[test]
    fn scan_classifies_files_and_skips_what_it_should() {
        let root = std::env::temp_dir().join(format!(
            "xtask-headers-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&root);

        write(
            &root,
            "crates/a/src/lib.rs",
            "//! A crate.\n//!\n//! Long story.\n",
        );
        write(
            &root,
            "crates/a/src/old.rs",
            "//! Old style. Two\n//! lines.\n",
        );
        write(&root, "crates/a/src/bad.rs", "//! No period\n");
        write(&root, "crates/a/src/none.rs", "use x;\n");
        write(
            &root,
            "crates/a/target/debug/gen.rs",
            "//! Bad but skipped\n",
        );
        write(&root, "apps/web/src/api.ts", "/** Client. */\nexport {};\n");
        write(&root, "apps/web/src/App.tsx", "import x;\n");
        write(&root, "apps/web/src/bad.tsx", "/** Two. Sentences. */\n");
        write(&root, "apps/web/node_modules/x/index.ts", "/** bad */\n");
        write(&root, "apps/web/dist/app.ts", "/** bad */\n");
        write(&root, "apps/web/src/notes.md", "not source\n");
        write(
            &root,
            "apps/knx-web/src/bindings/Foo.ts",
            "// generated by ts-rs\n",
        );
        write(&root, "tools/script.ts", "/** not walked */\n");
        write(&root, "xtask/src/main.rs", "//! Tasks.\n");

        let report = scan(&root).unwrap();
        fs::remove_dir_all(&root).unwrap();

        let paths = |v: &[PathBuf]| -> Vec<String> {
            v.iter().map(|p| p.to_string_lossy().into_owned()).collect()
        };
        assert_eq!(
            paths(&report.ok),
            vec![
                "apps/web/src/api.ts",
                "crates/a/src/lib.rs",
                "xtask/src/main.rs"
            ]
        );
        assert_eq!(
            paths(&report.absent),
            vec![
                "apps/web/src/App.tsx",
                "crates/a/src/none.rs",
                "crates/a/src/old.rs"
            ]
        );
        assert_eq!(
            paths(&report.generated),
            vec!["apps/knx-web/src/bindings/Foo.ts"]
        );
        let invalid: Vec<String> = report
            .invalid
            .iter()
            .map(|(p, _)| p.to_string_lossy().into_owned())
            .collect();
        assert_eq!(invalid, vec!["apps/web/src/bad.tsx", "crates/a/src/bad.rs"]);
    }

    #[test]
    fn scan_of_a_root_without_source_dirs_is_empty() {
        let root = std::env::temp_dir().join(format!("xtask-headers-empty-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let report = scan(&root).unwrap();
        fs::remove_dir_all(&root).unwrap();
        assert_eq!(report, Report::default());
    }
}
