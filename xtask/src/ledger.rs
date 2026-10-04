//! The `check-ledger` lint: one well-formed status ledger and no status elsewhere.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

/// One rule violation, located at a `file:line`.
#[derive(Debug, PartialEq, Eq)]
pub struct Problem {
    pub file: PathBuf,
    pub line: usize,
    pub message: String,
}

/// What a check of the ledger and the rest of the documentation found.
#[derive(Debug, Default)]
pub struct Report {
    pub rows: usize,
    pub problems: Vec<Problem>,
}

/// The 2026-10-01 inventory has exactly this many main-table IDs.
pub const SNAPSHOT_ROWS: usize = 180;

/// Where the ledger lives, relative to the repository root.
pub const LEDGER: &str = "docs/status/LEDGER.md";

const LIMITATIONS: &str = "docs/KNOWN_LIMITATIONS.md";
const HEADER: &str =
    "| ID | P | Owner | Route | Status | Owner disposition | Evidence and remaining work |";
const STATUSES: &[&str] = &[
    "TODO",
    "IN_PROGRESS",
    "DONE",
    "BLOCKED_EXTERNAL",
    "WAITING_OWNER",
    "WAITING_DECISION",
    "ACCEPTED_BOUNDARY",
    "LATER",
];
const OWNERS: &[&str] = &["alpha", "commission", "later", "ui"];
const PRIORITIES: &[&str] = &["P0", "P1", "P2", "P3"];
const SECTIONS: &[&str] = &["Snapshot IDs", "Post-snapshot IDs"];
/// Directories whose tables are history by definition, plus build trees.
const SKIP_DIRS: &[&str] = &[
    "archive",
    "history",
    "target",
    "node_modules",
    "dist",
    ".git",
];

/// One parsed ledger row.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct Row {
    id: String,
    priority: String,
    owner: String,
    status: String,
}

fn cells(line: &str) -> Vec<&str> {
    line.trim()
        .trim_matches('|')
        .split('|')
        .map(str::trim)
        .collect()
}

/// A ledger row has seven cells and a backticked ID first.
fn parse_row(line: &str) -> Option<Row> {
    let c = cells(line);
    if c.len() != 7 {
        return None;
    }
    let id = c[0].strip_prefix('`')?.strip_suffix('`')?;
    Some(Row {
        id: id.to_string(),
        priority: c[1].to_string(),
        owner: c[2].to_string(),
        status: c[4].to_string(),
    })
}

/// The count line the ledger must carry for one table: nonzero statuses in
/// vocabulary order, owners and priorities sorted.
fn expected_counts_line(label: &str, rows: &[Row]) -> String {
    let count =
        |pick: &dyn Fn(&Row) -> &str, word: &str| rows.iter().filter(|r| pick(r) == word).count();
    let join = |pick: &dyn Fn(&Row) -> &str, words: &[&str]| {
        words
            .iter()
            .map(|w| (w, count(pick, w)))
            .filter(|(_, n)| *n > 0)
            .map(|(w, n)| format!("{w}={n}"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    format!(
        "- **{label}** ({} rows) — status: {}; owner: {}; priority: {}.\n",
        rows.len(),
        join(&|r| &r.status, STATUSES),
        join(&|r| &r.owner, OWNERS),
        join(&|r| &r.priority, PRIORITIES),
    )
}

/// Checks the ledger file and scans the other documentation for stray
/// per-ID status tables.
pub fn check(root: &Path) -> Result<Report, String> {
    let path = root.join(LEDGER);
    let text =
        fs::read_to_string(&path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let mut problems = Vec::new();
    let mut problem = |file: &str, line: usize, message: String| {
        problems.push(Problem {
            file: PathBuf::from(file),
            line,
            message,
        });
    };

    let tables = section_tables(&text);
    let mut seen: HashMap<String, usize> = HashMap::new();
    for (label, rows) in &tables {
        if rows.is_none() {
            problem(
                LEDGER,
                0,
                format!("section `## {label}` with the ledger table header is missing"),
            );
        }
    }
    let mut all = Vec::new();
    for (label, rows) in &tables {
        let Some(rows) = rows else { continue };
        let mut parsed = Vec::new();
        for (line_no, line) in rows {
            let Some(row) = parse_row(line) else {
                problem(
                    LEDGER,
                    *line_no,
                    format!("expected 7 cells with a backticked ID, found `{line}`"),
                );
                continue;
            };
            if let Some(first) = seen.insert(row.id.clone(), *line_no) {
                problem(
                    LEDGER,
                    *line_no,
                    format!("duplicate ID `{}` (first at line {first})", row.id),
                );
            }
            for (word, allowed, what) in [
                (&row.status, STATUSES, "status"),
                (&row.owner, OWNERS, "owner"),
                (&row.priority, PRIORITIES, "priority"),
            ] {
                if !allowed.contains(&word.as_str()) {
                    problem(
                        LEDGER,
                        *line_no,
                        format!("unknown {what} `{word}` for `{}`", row.id),
                    );
                }
            }
            parsed.push((*line_no, row));
        }
        if *label == SECTIONS[0] && rows.len() != SNAPSHOT_ROWS {
            problem(
                LEDGER,
                0,
                format!("{} snapshot rows, expected {SNAPSHOT_ROWS}", rows.len()),
            );
        }
        let only_rows: Vec<Row> = parsed.iter().map(|(_, r)| r.clone()).collect();
        let expected = expected_counts_line(label, &only_rows);
        if !text.contains(&expected) {
            problem(
                LEDGER,
                0,
                format!(
                    "stale count line for {label}; expected: {}",
                    expected.trim_end()
                ),
            );
        }
        all.extend(parsed);
    }

    let headings = limitation_numbers(root)?;
    for (line_no, row) in &all {
        if let Some(number) = kl_number(&row.id) {
            if !headings.iter().any(|h| h == number) {
                problem(
                    LEDGER,
                    *line_no,
                    format!("`{}` has no heading in {LIMITATIONS}", row.id),
                );
            }
        }
    }

    let ids: HashSet<&str> = all.iter().map(|(_, r)| r.id.as_str()).collect();
    for rel in documentation_files(root)? {
        let source = fs::read_to_string(root.join(&rel))
            .map_err(|e| format!("cannot read {}: {e}", rel.display()))?;
        for (index, line) in source.lines().enumerate() {
            if let Some(id) = stray_status_row(line, &ids) {
                problems.push(Problem {
                    file: rel.clone(),
                    line: index + 1,
                    message: format!("status outside the ledger for `{id}`; edit {LEDGER} instead"),
                });
            }
        }
    }

    Ok(Report {
        rows: all.len(),
        problems,
    })
}

/// Table lines with their 1-based line numbers.
type NumberedLines = Vec<(usize, String)>;

/// For each ledger section: the numbered table lines after the header, or
/// `None` when the section or its header is missing.
fn section_tables(text: &str) -> Vec<(&'static str, Option<NumberedLines>)> {
    let lines: Vec<&str> = text.lines().collect();
    SECTIONS
        .iter()
        .map(|label| {
            let heading = format!("## {label}");
            let Some(start) = lines.iter().position(|l| l.trim_end() == heading) else {
                return (*label, None);
            };
            let mut rows = Vec::new();
            let mut header_seen = false;
            for (i, line) in lines.iter().enumerate().skip(start + 1) {
                if line.starts_with("## ") {
                    break;
                }
                if line.trim_end() == HEADER {
                    header_seen = true;
                    continue;
                }
                if header_seen && line.starts_with('|') && !line.starts_with("| ---") {
                    rows.push((i + 1, line.to_string()));
                }
            }
            (*label, header_seen.then_some(rows))
        })
        .collect()
}

/// `KL-130-GATE` and `KL-130` both need limitation heading 130.
fn kl_number(id: &str) -> Option<&str> {
    let rest = id.strip_prefix("KL-")?;
    let digits = rest.split('-').next()?;
    (!digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())).then_some(digits)
}

/// Numbers of the `## N…` / `## §N…` headings of the limitations file.
fn limitation_numbers(root: &Path) -> Result<Vec<String>, String> {
    let path = root.join(LIMITATIONS);
    let text =
        fs::read_to_string(&path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    Ok(text
        .lines()
        .filter_map(|l| l.strip_prefix("## "))
        .map(|h| h.trim_start_matches('§'))
        .map(|h| {
            h.chars()
                .take_while(char::is_ascii_digit)
                .collect::<String>()
        })
        .filter(|n| !n.is_empty())
        .collect())
}

/// A table row whose first cell is exactly a ledger ID and another cell is
/// exactly a status word: a second status record. Prose that merely
/// mentions a status word is evidence, not a record.
fn stray_status_row<'a>(line: &str, ids: &HashSet<&'a str>) -> Option<&'a str> {
    if !line.starts_with('|') || line.starts_with("| ---") {
        return None;
    }
    let c = cells(line);
    let first = c.first()?.trim_matches('`');
    let id = ids.get(first)?;
    c[1..]
        .iter()
        .any(|cell| STATUSES.contains(&cell.trim_matches(|ch| ch == '`' || ch == '*')))
        .then_some(*id)
}

/// `docs/**/*.md` without archive/history, plus root `*.md`; never the
/// ledger itself.
fn documentation_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    walk(root, &root.join("docs"), &mut out)?;
    for entry in fs::read_dir(root).map_err(|e| format!("cannot read {}: {e}", root.display()))? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("md") {
            out.push(path.strip_prefix(root).unwrap_or(&path).to_path_buf());
        }
    }
    out.retain(|p| p != Path::new(LEDGER));
    out.sort();
    Ok(out)
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(dir).map_err(|e| format!("cannot read {}: {e}", dir.display()))? {
        let path = entry.map_err(|e| e.to_string())?.path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if path.is_dir() {
            if !SKIP_DIRS.contains(&name) {
                walk(root, &path, out)?;
            }
        } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
            out.push(path.strip_prefix(root).unwrap_or(&path).to_path_buf());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEADER: &str = "| ID | P | Owner | Route | Status | Owner disposition | Evidence and remaining work |\n| --- | --- | --- | --- | --- | --- | --- |\n";

    fn row(id: &str, prio: &str, owner: &str, status: &str) -> String {
        format!("| `{id}` | {prio} | {owner} | AR01 | {status} | — | evidence |\n")
    }

    /// A ledger with `n` snapshot rows (`X-1` … `X-n`, all `TODO`, P2,
    /// alpha) and the given post-snapshot rows, plus matching count lines.
    fn ledger(snapshot: &[String], post: &[String]) -> String {
        format!(
            "# Source-ID ledger\n\n## Counts\n\n{}{}\n## Snapshot IDs\n\n{HEADER}{}\n## Post-snapshot IDs\n\n{HEADER}{}\n## Reconciliation record\n",
            counts_line("Snapshot IDs", snapshot),
            counts_line("Post-snapshot IDs", post),
            snapshot.concat(),
            post.concat()
        )
    }

    fn counts_line(label: &str, rows: &[String]) -> String {
        expected_counts_line(
            label,
            &rows
                .iter()
                .map(|r| parse_row(r).unwrap())
                .collect::<Vec<_>>(),
        )
    }

    fn snapshot_rows(n: usize) -> Vec<String> {
        (1..=n)
            .map(|i| row(&format!("X-{i}"), "P2", "alpha", "TODO"))
            .collect()
    }

    fn repo(ledger_text: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "docs/status/LEDGER.md", ledger_text);
        write(
            dir.path(),
            "docs/KNOWN_LIMITATIONS.md",
            "# Known limitations\n\n## 1. One\n\n## §130 Gate\n",
        );
        dir
    }

    fn write(root: &Path, rel: &str, text: &str) {
        let path = root.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    fn messages(root: &Path) -> Vec<String> {
        check(root)
            .unwrap()
            .problems
            .into_iter()
            .map(|p| p.message)
            .collect()
    }

    #[test]
    fn a_consistent_ledger_passes() {
        let dir = repo(&ledger(
            &snapshot_rows(SNAPSHOT_ROWS),
            &[row("KL-1", "P1", "ui", "DONE")],
        ));
        let report = check(dir.path()).unwrap();
        assert_eq!(report.problems, vec![]);
        assert_eq!(report.rows, SNAPSHOT_ROWS + 1);
    }

    #[test]
    fn a_missing_ledger_is_an_error_not_a_pass() {
        let dir = tempfile::tempdir().unwrap();
        assert!(check(dir.path()).is_err());
    }

    #[test]
    fn the_snapshot_must_keep_exactly_180_rows() {
        let dir = repo(&ledger(&snapshot_rows(SNAPSHOT_ROWS - 1), &[]));
        assert!(messages(dir.path())
            .iter()
            .any(|m| m.contains("179 snapshot rows, expected 180")));
    }

    #[test]
    fn a_duplicate_id_is_reported_even_across_tables() {
        let mut snap = snapshot_rows(SNAPSHOT_ROWS);
        snap[0] = row("KL-1", "P2", "alpha", "TODO");
        let dir = repo(&ledger(&snap, &[row("KL-1", "P1", "alpha", "TODO")]));
        assert!(messages(dir.path())
            .iter()
            .any(|m| m.contains("duplicate ID `KL-1`")));
    }

    #[test]
    fn unknown_status_owner_and_priority_words_are_reported() {
        let mut snap = snapshot_rows(SNAPSHOT_ROWS);
        snap[1] = row("X-2", "P2", "alpha", "FINISHED");
        snap[2] = row("X-3", "P2", "nobody", "TODO");
        snap[3] = row("X-4", "P7", "alpha", "TODO");
        let text = ledger(&snapshot_rows(SNAPSHOT_ROWS), &[]);
        // Keep the count lines of a valid ledger so only the words are wrong.
        let text = text.replace(&snapshot_rows(SNAPSHOT_ROWS).concat(), &snap.concat());
        let found = messages(repo(&text).path());
        assert!(
            found
                .iter()
                .any(|m| m.contains("unknown status `FINISHED`")),
            "{found:?}"
        );
        assert!(
            found.iter().any(|m| m.contains("unknown owner `nobody`")),
            "{found:?}"
        );
        assert!(
            found.iter().any(|m| m.contains("unknown priority `P7`")),
            "{found:?}"
        );
    }

    #[test]
    fn a_row_with_the_wrong_number_of_cells_is_reported() {
        let text = ledger(&snapshot_rows(SNAPSHOT_ROWS), &[]).replace(
            "| `X-5` | P2 | alpha | AR01 | TODO | — | evidence |",
            "| `X-5` | P2 | alpha | TODO |",
        );
        assert!(messages(repo(&text).path())
            .iter()
            .any(|m| m.contains("expected 7 cells")));
    }

    #[test]
    fn a_stale_count_line_is_reported_with_the_expected_text() {
        let text = ledger(&snapshot_rows(SNAPSHOT_ROWS), &[]).replace(
            "| `X-6` | P2 | alpha | AR01 | TODO |",
            "| `X-6` | P2 | alpha | AR01 | DONE |",
        );
        let found = messages(repo(&text).path());
        assert!(
            found
                .iter()
                .any(|m| m.contains("stale count line") && m.contains("TODO=179, DONE=1")),
            "{found:?}"
        );
    }

    #[test]
    fn a_kl_id_needs_a_known_limitations_heading() {
        let post = [
            row("KL-1", "P1", "alpha", "TODO"),
            row("KL-130-GATE", "P3", "alpha", "TODO"),
            row("KL-999", "P2", "alpha", "TODO"),
        ];
        let found = messages(repo(&ledger(&snapshot_rows(SNAPSHOT_ROWS), &post)).path());
        assert_eq!(
            found
                .iter()
                .filter(|m| m.contains("no heading in docs/KNOWN_LIMITATIONS.md"))
                .count(),
            1,
            "{found:?}"
        );
        assert!(found.iter().any(|m| m.contains("`KL-999`")));
    }

    #[test]
    fn a_status_table_outside_the_ledger_is_reported() {
        let dir = repo(&ledger(&snapshot_rows(SNAPSHOT_ROWS), &[]));
        write(
            dir.path(),
            "docs/OTHER.md",
            "| ID | Status |\n| --- | --- |\n| `X-7` | DONE |\n",
        );
        write(dir.path(), "goal-x.md", "| X-8 | P2 | `WAITING_OWNER` |\n");
        let report = check(dir.path()).unwrap();
        let stray: Vec<_> = report
            .problems
            .iter()
            .filter(|p| p.message.contains("status outside the ledger"))
            .collect();
        assert_eq!(stray.len(), 2, "{:?}", report.problems);
        assert!(stray
            .iter()
            .any(|p| p.file == Path::new("docs/OTHER.md") && p.line == 3));
        assert!(stray
            .iter()
            .any(|p| p.file == Path::new("goal-x.md") && p.line == 1));
    }

    #[test]
    fn archive_history_and_evidence_tables_are_not_status_tables() {
        let dir = repo(&ledger(&snapshot_rows(SNAPSHOT_ROWS), &[]));
        write(dir.path(), "docs/archive/OLD.md", "| `X-7` | DONE |\n");
        write(dir.path(), "docs/history/OLD.md", "| `X-7` | DONE |\n");
        // An evidence row that mentions a status word inside prose is fine.
        write(
            dir.path(),
            "docs/EVIDENCE.md",
            "| `X-7` | Already DONE in U13 | none |\n",
        );
        // An ID that is not in the ledger is not this check's business.
        write(dir.path(), "docs/OTHER.md", "| `Y-1` | DONE |\n");
        assert_eq!(check(dir.path()).unwrap().problems, vec![]);
    }
}
