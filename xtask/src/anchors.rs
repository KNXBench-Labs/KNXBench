//! The `check-anchors` lint: every in-repo markdown anchor link must resolve.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Directories never descended into, wherever they appear (mirrors
/// `headers.rs`; `docs/` carries none of these today, but a stray
/// `node_modules` from a future doc-tooling experiment should not make
/// the walk choke on it).
const SKIP_DIRS: &[&str] = &["target", "node_modules", "dist", ".git"];

/// One dead link: a `file:line` whose anchor does not resolve.
#[derive(Debug, PartialEq, Eq)]
pub struct DeadAnchor {
    pub file: PathBuf,
    pub line: usize,
    /// The link's target, exactly as written (`path#anchor` or `#anchor`).
    pub raw_link: String,
    pub kind: DeadKind,
}

/// Why a link is dead: either it points at a file this repo does not have,
/// or the file exists (and, if markdown under our scan roots, was read)
/// but carries no heading slug and no `<a id="…">` matching the anchor.
#[derive(Debug, PartialEq, Eq)]
pub enum DeadKind {
    MissingFile,
    MissingAnchor { suggestion: Option<String> },
}

/// What a sweep of the repository's markdown found.
#[derive(Debug, Default)]
pub struct Report {
    pub files_scanned: usize,
    pub links_checked: usize,
    pub dead: Vec<DeadAnchor>,
}

/// Walks `docs/**/*.md` plus the markdown files directly in the repo root
/// (not `.ai/`, not any other root subdirectory — those are not this
/// project's documentation set, and pulling them in would make the link
/// count depend on what a handover log happened to link to). Every
/// resolvable in-repo anchor link is checked against the target file's
/// heading slugs and explicit `<a id="…">` aliases.
pub fn scan(root: &Path) -> Result<Report, String> {
    let files = discover(root)?;

    let mut sources: HashMap<PathBuf, String> = HashMap::new();
    let mut anchors_by_file: HashMap<PathBuf, HashSet<String>> = HashMap::new();
    for rel in &files {
        let abs = root.join(rel);
        let source =
            fs::read_to_string(&abs).map_err(|e| format!("cannot read {}: {e}", abs.display()))?;
        anchors_by_file.insert(rel.clone(), anchors_in_file(&source));
        sources.insert(rel.clone(), source);
    }

    let mut dead = Vec::new();
    let mut links_checked = 0usize;
    for rel in &files {
        let source = &sources[rel];
        let dir = rel.parent().unwrap_or_else(|| Path::new(""));
        for link in links_in_file(source) {
            links_checked += 1;
            let target_rel = if link.target_path.is_empty() {
                rel.clone()
            } else {
                normalize_path(&dir.join(&link.target_path))
            };

            let Some(live) = anchors_by_file.get(&target_rel) else {
                // Either the file does not exist, or it exists but is
                // outside our scan roots (e.g. a source file, or a
                // markdown file under `apps/`, `crates/` or `.ai/`) —
                // in the second case there is nothing for this lint to
                // check, so it is silent rather than wrong.
                if !root.join(&target_rel).exists() {
                    dead.push(DeadAnchor {
                        file: rel.clone(),
                        line: link.line,
                        raw_link: link.raw,
                        kind: DeadKind::MissingFile,
                    });
                }
                continue;
            };

            if !live.contains(&link.anchor) {
                dead.push(DeadAnchor {
                    file: rel.clone(),
                    line: link.line,
                    raw_link: link.raw.clone(),
                    kind: DeadKind::MissingAnchor {
                        suggestion: nearest(&link.anchor, live),
                    },
                });
            }
        }
    }

    Ok(Report {
        files_scanned: files.len(),
        links_checked,
        dead,
    })
}

/// Collects every `.md` file under `docs/` (recursively) and directly in
/// `root` (not recursively — a root-level subdirectory is not "the
/// repo-root markdown files").
fn discover(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();

    let docs = root.join("docs");
    if docs.is_dir() {
        walk_markdown(root, &docs, &mut files)?;
    }

    let mut root_entries: Vec<PathBuf> = fs::read_dir(root)
        .map_err(|e| format!("cannot read {}: {e}", root.display()))?
        .map(|entry| entry.map(|e| e.path()))
        .collect::<Result<_, _>>()
        .map_err(|e| format!("cannot read an entry of {}: {e}", root.display()))?;
    root_entries.sort();
    for path in root_entries {
        if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("md") {
            files.push(path.strip_prefix(root).unwrap_or(&path).to_path_buf());
        }
    }

    files.sort();
    Ok(files)
}

fn walk_markdown(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .map_err(|e| format!("cannot read {}: {e}", dir.display()))?
        .map(|entry| entry.map(|e| e.path()))
        .collect::<Result<_, _>>()
        .map_err(|e| format!("cannot read an entry of {}: {e}", dir.display()))?;
    entries.sort();

    for path in entries {
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if SKIP_DIRS.contains(&name) {
                continue;
            }
            walk_markdown(root, &path, out)?;
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) == Some("md") {
            out.push(path.strip_prefix(root).unwrap_or(&path).to_path_buf());
        }
    }
    Ok(())
}

/// Resolves `.`/`..` components lexically, without touching the
/// filesystem — the target may not exist, which is itself something this
/// lint reports, so resolution cannot depend on `fs::canonicalize`
/// succeeding.
fn normalize_path(path: &Path) -> PathBuf {
    let mut out: Vec<std::ffi::OsString> = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                out.pop();
            }
            std::path::Component::CurDir => {}
            other => out.push(other.as_os_str().to_owned()),
        }
    }
    out.into_iter().collect()
}

/// A line's fence markers (` ``` ` or `~~~`) toggle code-block state. This
/// does not track fence length or character-matching per CommonMark — no
/// file in this repo nests a shorter fence inside a longer one — so a
/// mismatched fence pair would under- or over-skip. Good enough for prose
/// documentation; a future doc that needs the exact rule can add it here.
fn is_fence(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("```") || t.starts_with("~~~")
}

/// Extracts every heading's github-slugger slug and every explicit
/// `<a id="…">` back-compat anchor, in document order, skipping fenced
/// code blocks.
fn anchors_in_file(source: &str) -> HashSet<String> {
    let mut live = HashSet::new();
    let mut seen_slug_counts: HashMap<String, usize> = HashMap::new();
    let mut in_code = false;

    for line in source.lines() {
        if is_fence(line) {
            in_code = !in_code;
            continue;
        }
        if in_code {
            continue;
        }
        if let Some(text) = heading_text(line) {
            let base = slugify_heading(text);
            let count = seen_slug_counts.entry(base.clone()).or_insert(0);
            let slug = if *count == 0 {
                base
            } else {
                format!("{base}-{count}")
            };
            *count += 1;
            live.insert(slug);
        }
        for id in explicit_anchor_ids(line) {
            live.insert(id);
        }
    }

    live
}

/// An ATX heading's text (`## Some Heading` -> `Some Heading`), with any
/// closing `###`-style hashes trimmed. `None` for anything else, including
/// a `#` with no following space (not a heading — could be a `#5` issue
/// reference) and anything past level 6.
fn heading_text(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    let hashes = trimmed.chars().take_while(|&c| c == '#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    let rest = &trimmed[hashes..];
    if rest.is_empty() {
        return None;
    }
    let text = rest.strip_prefix(' ')?.trim();
    Some(text.trim_end_matches('#').trim_end())
}

/// Every `id` in a line's `<a id="…">` tags, in order.
fn explicit_anchor_ids(line: &str) -> Vec<String> {
    let mut ids = Vec::new();
    let mut rest = line;
    while let Some(pos) = rest.find("<a id=\"") {
        let after = &rest[pos + "<a id=\"".len()..];
        let Some(end) = after.find('"') else { break };
        ids.push(after[..end].to_string());
        rest = &after[end + 1..];
    }
    ids
}

/// One in-repo `[text](path#anchor)` or `[text](#anchor)` link found in a
/// body line.
struct LinkRef {
    line: usize,
    /// The part before `#`; empty for a same-file link.
    target_path: String,
    anchor: String,
    /// The full `(...)` contents, for the error message.
    raw: String,
}

/// Every anchor-bearing markdown link on non-fenced lines. External links
/// (`http(s)://`, `mailto:`) and links with no `#` at all (nothing for
/// this lint to check) are skipped here rather than by the caller, so a
/// caller can trust that every `LinkRef` it sees needs resolving.
fn links_in_file(source: &str) -> Vec<LinkRef> {
    let mut out = Vec::new();
    let mut in_code = false;

    for (idx, line) in source.lines().enumerate() {
        if is_fence(line) {
            in_code = !in_code;
            continue;
        }
        if in_code {
            continue;
        }
        for target in line_link_targets(line) {
            let target = target.split_whitespace().next().unwrap_or("");
            if target.is_empty()
                || target.starts_with("http://")
                || target.starts_with("https://")
                || target.starts_with("mailto:")
            {
                continue;
            }
            let Some(hash) = target.find('#') else {
                continue;
            };
            let (path_part, anchor_part) = target.split_at(hash);
            let anchor = &anchor_part[1..];
            if anchor.is_empty() {
                continue; // a link to a file, not a place in it
            }
            out.push(LinkRef {
                line: idx + 1,
                target_path: path_part.to_string(),
                anchor: anchor.to_string(),
                raw: target.to_string(),
            });
        }
    }

    out
}

/// Every `(...)` target of a `[...]  (...)` inline link on one line.
/// Bracket and paren nesting are both tracked one level deep, which is
/// enough for the link text and URLs this repository actually writes;
/// reference-style `[text][ref]` links are not used anywhere in `docs/`
/// (verified by sweep, not assumed) so they are not handled here.
fn line_link_targets(line: &str) -> Vec<String> {
    let chars: Vec<char> = line.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '[' {
            if let Some(close) = matching_delim(&chars, i, '[', ']') {
                if chars.get(close + 1) == Some(&'(') {
                    if let Some(paren_close) = matching_delim(&chars, close + 1, '(', ')') {
                        let target: String = chars[close + 2..paren_close].iter().collect();
                        out.push(target);
                        i = paren_close + 1;
                        continue;
                    }
                }
            }
        }
        i += 1;
    }
    out
}

/// Index of the delimiter that closes the one opened at `open` (which
/// must hold `open_ch`), tracking nesting depth.
fn matching_delim(chars: &[char], open: usize, open_ch: char, close_ch: char) -> Option<usize> {
    let mut depth = 0;
    for (idx, &c) in chars.iter().enumerate().skip(open) {
        if c == open_ch {
            depth += 1;
        } else if c == close_ch {
            depth -= 1;
            if depth == 0 {
                return Some(idx);
            }
        }
    }
    None
}

/// Renders heading markdown to the plain text github-slugger slugifies:
/// HTML tags are dropped, `[text](url)` becomes `text`, and backtick and
/// `*`-emphasis markers are removed. Underscores are left alone —
/// intraword (`PID_PROGRAM_VERSION`) is the only case this repo's
/// headings use, and treating a lone `_word_` as italic emphasis is not a
/// rule anything here needs.
fn render_heading(raw: &str) -> String {
    let chars: Vec<char> = raw.chars().collect();
    let mut out = String::with_capacity(raw.len());
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '<' => {
                if let Some(end) = chars[i..].iter().position(|&c| c == '>') {
                    i += end + 1;
                } else {
                    out.push(chars[i]);
                    i += 1;
                }
            }
            '`' | '*' => {
                i += 1;
            }
            '[' => {
                if let Some(close) = matching_delim(&chars, i, '[', ']') {
                    if chars.get(close + 1) == Some(&'(') {
                        if let Some(paren_close) = matching_delim(&chars, close + 1, '(', ')') {
                            out.extend(&chars[i + 1..close]);
                            i = paren_close + 1;
                            continue;
                        }
                    }
                }
                out.push(chars[i]);
                i += 1;
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    out
}

/// The github-slugger rule: lowercase the rendered heading, drop every
/// character that is not a word character, a Unicode letter or digit, a
/// hyphen or a space, then turn each remaining space into a hyphen.
/// `char::is_alphanumeric` covers "Unicode letter/digit"; it does not
/// cover a bare combining mark (Unicode category Mn/Mc) on its own, which
/// no heading in this repo's `docs/` tree contains today.
fn slugify_heading(text: &str) -> String {
    let rendered = render_heading(text);
    let lower = rendered.to_lowercase();
    let kept: String = lower
        .chars()
        .filter(|&c| c.is_alphanumeric() || c == '_' || c == '-' || c == ' ')
        .collect();
    kept.chars()
        .map(|c| if c == ' ' { '-' } else { c })
        .collect()
}

/// The closest live anchor to a dead one, by Levenshtein distance, if any
/// candidate is close enough to be worth printing — a suggestion that is
/// almost as long as the anchor itself is noise, not help.
fn nearest(anchor: &str, candidates: &HashSet<String>) -> Option<String> {
    candidates
        .iter()
        .map(|c| (levenshtein(anchor, c), c))
        .min_by_key(|(d, _)| *d)
        .filter(|(d, c)| *d <= (anchor.len().max(c.len()) / 3).max(3))
        .map(|(_, c)| c.clone())
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, &ca) in a.iter().enumerate() {
        let mut cur = vec![i + 1; b.len() + 1];
        for (j, &cb) in b.iter().enumerate() {
            let cost = if ca == cb { 0 } else { 1 };
            cur[j + 1] = (prev[j + 1] + 1).min(cur[j] + 1).min(prev[j] + cost);
        }
        prev = cur;
    }
    prev[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugifies_plain_text() {
        assert_eq!(slugify_heading("Hello World"), "hello-world");
    }

    #[test]
    fn slug_drops_punctuation_with_no_replacement() {
        assert_eq!(slugify_heading("apps/knx-cli"), "appsknx-cli");
        assert_eq!(slugify_heading("(ETS4/ETS5)"), "ets4ets5");
    }

    #[test]
    fn slug_strips_backticks_and_emphasis_but_keeps_intraword_underscore() {
        assert_eq!(
            slugify_heading("`--version` names a commit, never a working tree"),
            "--version-names-a-commit-never-a-working-tree"
        );
        assert_eq!(
            slugify_heading("still writes `PID_PROGRAM_VERSION` unconditionally"),
            "still-writes-pid_program_version-unconditionally"
        );
        assert_eq!(slugify_heading("*bold* word"), "bold-word");
    }

    #[test]
    fn spaced_em_dash_leaves_a_double_hyphen() {
        assert_eq!(
            slugify_heading("Project licence — resolved 2026-09-16"),
            "project-licence--resolved-2026-09-16"
        );
    }

    #[test]
    fn slug_renders_a_markdown_link_as_its_text() {
        assert_eq!(
            slugify_heading("See [the spec](../spec.md) for details"),
            "see-the-spec-for-details"
        );
    }

    #[test]
    fn duplicate_headings_in_one_file_collide_with_dash_suffixes() {
        let src = "# Notes\n\n## Setup\n\nsome text\n\n## Setup\n\nmore text\n\n## Setup\n";
        let live = anchors_in_file(src);
        assert!(live.contains("setup"));
        assert!(live.contains("setup-1"));
        assert!(live.contains("setup-2"));
    }

    #[test]
    fn explicit_anchor_tags_are_collected() {
        let src = "Some text.\n\n<a id=\"legacy-slug\"></a>\n\n## Current Heading\n";
        let live = anchors_in_file(src);
        assert!(live.contains("legacy-slug"));
        assert!(live.contains("current-heading"));
    }

    #[test]
    fn fenced_code_blocks_are_not_scanned_for_headings_or_links() {
        let src = "# Real Heading\n\n```\n## Not A Heading\n[fake](#nope)\n```\n\n[real](#real-heading)\n";
        let live = anchors_in_file(src);
        assert!(live.contains("real-heading"));
        assert!(!live.contains("not-a-heading"));

        let links = links_in_file(src);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].anchor, "real-heading");
    }

    #[test]
    fn heading_text_requires_a_space_after_the_hashes() {
        assert_eq!(heading_text("## Real heading"), Some("Real heading"));
        assert_eq!(heading_text("##Not a heading"), None);
        assert_eq!(heading_text("#######Also not (level 7)"), None);
        assert_eq!(heading_text("## Closed heading ##"), Some("Closed heading"));
    }

    fn write(root: &Path, rel: &str, content: &str) {
        let path = root.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    fn scratch_root(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "xtask-anchors-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&root);
        root
    }

    #[test]
    fn a_dead_anchor_fixture_fails_the_check() {
        let root = scratch_root("dead");
        write(
            &root,
            "docs/A.md",
            "# Alpha\n\nLinks to [B's real heading](B.md#beta-heading) and to a \
             [renamed one](B.md#beta-headings).\n",
        );
        write(&root, "docs/B.md", "# Beta Heading\n");

        let report = scan(&root).unwrap();
        fs::remove_dir_all(&root).unwrap();

        assert_eq!(report.files_scanned, 2);
        assert_eq!(report.links_checked, 2);
        assert_eq!(report.dead.len(), 1);
        let dead = &report.dead[0];
        assert_eq!(dead.file, PathBuf::from("docs/A.md"));
        assert_eq!(dead.line, 3);
        match &dead.kind {
            DeadKind::MissingAnchor { suggestion } => {
                assert_eq!(suggestion.as_deref(), Some("beta-heading"));
            }
            other => panic!("expected MissingAnchor, got {other:?}"),
        }
    }

    #[test]
    fn a_link_to_a_missing_file_is_reported_distinctly() {
        let root = scratch_root("missing-file");
        write(&root, "docs/A.md", "[gone](nowhere.md#somewhere)\n");

        let report = scan(&root).unwrap();
        fs::remove_dir_all(&root).unwrap();

        assert_eq!(report.dead.len(), 1);
        assert_eq!(report.dead[0].kind, DeadKind::MissingFile);
    }

    #[test]
    fn same_file_links_and_relative_paths_resolve() {
        let root = scratch_root("resolve");
        write(
            &root,
            "docs/sub/C.md",
            "# Sub Heading\n\n[up](../A.md#alpha) and [self](#sub-heading)\n",
        );
        write(&root, "docs/A.md", "# Alpha\n");

        let report = scan(&root).unwrap();
        fs::remove_dir_all(&root).unwrap();

        assert!(report.dead.is_empty(), "{:?}", report.dead);
    }

    #[test]
    fn root_level_markdown_is_scanned_but_root_subdirectories_are_not() {
        let root = scratch_root("root-scope");
        write(&root, "README.md", "[to docs](docs/A.md#alpha)\n");
        write(&root, "docs/A.md", "# Alpha\n");
        write(&root, "other/NOTES.md", "# Not Scanned\n");

        let report = scan(&root).unwrap();
        fs::remove_dir_all(&root).unwrap();

        assert_eq!(report.files_scanned, 2);
        assert!(report.dead.is_empty(), "{:?}", report.dead);
    }

    #[test]
    fn external_links_are_never_checked() {
        let root = scratch_root("external");
        write(
            &root,
            "docs/A.md",
            "[gpl](https://www.gnu.org/licenses/gpl-3.0.html#section13)\n",
        );

        let report = scan(&root).unwrap();
        fs::remove_dir_all(&root).unwrap();

        assert_eq!(report.links_checked, 0);
        assert!(report.dead.is_empty());
    }
}
