//! Decides which commit a KNXBench binary names, and refuses a release build from a modified tree.
//!
//! Both binaries' `build.rs` call [`emit`]. Two modes (ADR-0018 §2,
//! KNOWN_LIMITATIONS §65):
//!
//! * **Development** (default): the stamp is an explicit `KNX_BUILD_SHA`,
//!   else the workspace's `HEAD`, else nothing. It names a commit, never a
//!   working tree; uncommitted edits are invisible, because cargo re-runs a
//!   build script only for files it watches.
//! * **Release** (`KNX_REQUIRE_CLEAN_TREE=1`): the build script re-runs on
//!   every build, and the build fails unless git confirms this workspace,
//!   resolves `HEAD`, reports no modified or untracked (non-ignored) path,
//!   and any explicit `KNX_BUILD_SHA` names that same commit. A release
//!   artifact therefore never names a commit it was not built from.
//!
//! Zero dependencies: this runs inside other crates' build scripts.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Environment variable that switches [`Mode::Release`] on when it is `1`.
pub const REQUIRE_CLEAN_TREE_VAR: &str = "KNX_REQUIRE_CLEAN_TREE";
/// Environment variable carrying an explicit commit (Docker, tarballs).
pub const BUILD_SHA_VAR: &str = "KNX_BUILD_SHA";
/// How many modified paths a refusal lists before summarising the rest.
const LISTED_PATHS: usize = 10;

/// Whether a build may name a commit it cannot prove it was built from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Development,
    Release,
}

impl Mode {
    /// `Release` exactly when the variable is `1`; anything else, including
    /// a typo, is `Development` — and [`emit`] says which mode it chose.
    pub fn from_env_value(value: Option<&str>) -> Self {
        match value.map(str::trim) {
            Some("1") => Self::Release,
            _ => Self::Development,
        }
    }
}

/// What git reported about this workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitFacts {
    /// `git rev-parse --short HEAD`, when it answered.
    pub head: Option<String>,
    /// Porcelain status lines: modified, staged, deleted or untracked
    /// (non-ignored) paths. Empty means clean.
    pub modified: Vec<String>,
}

/// The stamp for `mode`, or why a release build must not proceed.
///
/// `git` is `None` when git is missing or answers for a different
/// repository than this workspace.
pub fn stamp(explicit: Option<&str>, git: Option<&GitFacts>, mode: Mode) -> Result<String, String> {
    let explicit = explicit.map(str::trim).filter(|s| !s.is_empty());
    match mode {
        Mode::Development => Ok(explicit
            .map(str::to_string)
            .or_else(|| git.and_then(|g| g.head.clone()))
            .unwrap_or_default()),
        Mode::Release => {
            let git = git.ok_or_else(|| {
                format!(
                    "{REQUIRE_CLEAN_TREE_VAR}=1, but git cannot confirm this workspace; \
                     build a release from a git checkout of KNXBench itself"
                )
            })?;
            let head = git
                .head
                .clone()
                .ok_or_else(|| format!("{REQUIRE_CLEAN_TREE_VAR}=1, but git has no HEAD commit"))?;
            if !git.modified.is_empty() {
                let mut listed: Vec<&str> = git
                    .modified
                    .iter()
                    .take(LISTED_PATHS)
                    .map(String::as_str)
                    .collect();
                if git.modified.len() > LISTED_PATHS {
                    listed.push("…");
                }
                return Err(format!(
                    "{REQUIRE_CLEAN_TREE_VAR}=1, but the tree differs from {head} in {} path(s): {}",
                    git.modified.len(),
                    listed.join(", ")
                ));
            }
            if let Some(explicit) = explicit {
                if !(explicit.starts_with(&head) || head.starts_with(explicit)) {
                    return Err(format!(
                        "{REQUIRE_CLEAN_TREE_VAR}=1, but {BUILD_SHA_VAR}={explicit} is not HEAD ({head})"
                    ));
                }
            }
            Ok(head)
        }
    }
}

/// Asks git about `workspace`. `None` when git is unavailable or the
/// repository it finds is not `workspace` itself — a tree unpacked inside an
/// unrelated checkout must not borrow that checkout's commit.
pub fn git_facts(workspace: &Path) -> Option<GitFacts> {
    let toplevel = canonical(&git(workspace, &["rev-parse", "--show-toplevel"])?)?;
    if toplevel != workspace.canonicalize().ok()? {
        return None;
    }
    let status = git(
        workspace,
        &["status", "--porcelain=v1", "--untracked-files=normal"],
    )
    .unwrap_or_default();
    Some(GitFacts {
        head: git(workspace, &["rev-parse", "--short", "HEAD"]),
        modified: status
            .lines()
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty())
            .collect(),
    })
}

/// The build-script entry point: prints the cargo directives and
/// `KNX_BUILD_SHA`, or fails the build in release mode.
///
/// # Panics
/// In [`Mode::Release`], when [`stamp`] refuses — the build must stop.
pub fn emit() {
    println!("cargo:rerun-if-env-changed={BUILD_SHA_VAR}");
    println!("cargo:rerun-if-env-changed={REQUIRE_CLEAN_TREE_VAR}");
    let mode = Mode::from_env_value(std::env::var(REQUIRE_CLEAN_TREE_VAR).ok().as_deref());
    let workspace = std::env::var("CARGO_MANIFEST_DIR")
        .ok()
        .map(|dir| PathBuf::from(dir).join("../.."));
    let facts = workspace.as_deref().and_then(git_facts);
    match mode {
        // A path that never exists makes cargo re-run this script on every
        // build, so the cleanliness check cannot be skipped by a cached
        // result from an earlier, clean build.
        Mode::Release => println!("cargo:rerun-if-changed=.knx-build-stamp-always-rerun"),
        Mode::Development => {
            if let Some(workspace) = &workspace {
                watch_head(workspace);
            }
        }
    }
    let explicit = std::env::var(BUILD_SHA_VAR).ok();
    match stamp(explicit.as_deref(), facts.as_ref(), mode) {
        Ok(sha) => println!("cargo:rustc-env={BUILD_SHA_VAR}={sha}"),
        Err(refusal) => panic!("{refusal}"),
    }
}

/// Development watches: `HEAD`, its reflog (appended on every commit,
/// checkout and reset, packed refs or not) and the loose branch ref.
fn watch_head(workspace: &Path) {
    for watched in ["HEAD", "logs/HEAD"] {
        if let Some(path) = git(workspace, &["rev-parse", "--git-path", watched]) {
            println!(
                "cargo:rerun-if-changed={}",
                absolute(workspace, &path).display()
            );
        }
    }
    if let Some(branch) = git(workspace, &["symbolic-ref", "-q", "HEAD"]) {
        if let Some(path) = git(workspace, &["rev-parse", "--git-path", &branch]) {
            let path = absolute(workspace, &path);
            if path.exists() {
                println!("cargo:rerun-if-changed={}", path.display());
            }
        }
    }
}

fn absolute(workspace: &Path, path: &str) -> PathBuf {
    let path = Path::new(path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        workspace.join(path)
    }
}

fn canonical(path: &str) -> Option<PathBuf> {
    Path::new(path).canonicalize().ok()
}

fn git(dir: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    let text = text.trim_end();
    (!text.is_empty()).then(|| text.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(head: Option<&str>, modified: &[&str]) -> GitFacts {
        GitFacts {
            head: head.map(str::to_string),
            modified: modified.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn only_the_value_one_switches_release_mode_on() {
        assert_eq!(Mode::from_env_value(Some("1")), Mode::Release);
        assert_eq!(Mode::from_env_value(Some(" 1 ")), Mode::Release);
        for other in [None, Some(""), Some("0"), Some("true"), Some("yes")] {
            assert_eq!(Mode::from_env_value(other), Mode::Development, "{other:?}");
        }
    }

    #[test]
    fn development_keeps_todays_order_explicit_then_head_then_nothing() {
        let dirty = facts(Some("abc1234"), &[" M src/main.rs"]);
        assert_eq!(
            stamp(Some("feed123"), Some(&dirty), Mode::Development).unwrap(),
            "feed123"
        );
        assert_eq!(
            stamp(Some("  "), Some(&dirty), Mode::Development).unwrap(),
            "abc1234"
        );
        assert_eq!(stamp(None, None, Mode::Development).unwrap(), "");
    }

    #[test]
    fn release_names_head_only_for_a_clean_confirmed_tree() {
        let clean = facts(Some("abc1234"), &[]);
        assert_eq!(stamp(None, Some(&clean), Mode::Release).unwrap(), "abc1234");
        assert_eq!(
            stamp(Some("abc1234"), Some(&clean), Mode::Release).unwrap(),
            "abc1234"
        );
        assert_eq!(
            stamp(Some("abc1234def"), Some(&clean), Mode::Release).unwrap(),
            "abc1234",
            "a longer form of the same commit is the same commit"
        );
    }

    #[test]
    fn release_refuses_a_modified_tree_and_names_the_paths() {
        let dirty = facts(Some("abc1234"), &[" M src/main.rs", "?? notes.txt"]);
        let refusal = stamp(None, Some(&dirty), Mode::Release).unwrap_err();
        assert!(refusal.contains("2 path(s)"), "{refusal}");
        assert!(
            refusal.contains("src/main.rs") && refusal.contains("notes.txt"),
            "{refusal}"
        );
    }

    #[test]
    fn release_lists_at_most_ten_paths() {
        let many: Vec<String> = (0..12).map(|i| format!(" M f{i}")).collect();
        let many: Vec<&str> = many.iter().map(String::as_str).collect();
        let refusal = stamp(None, Some(&facts(Some("abc1234"), &many)), Mode::Release).unwrap_err();
        assert!(
            refusal.contains("12 path(s)") && refusal.contains("f9"),
            "{refusal}"
        );
        assert!(
            !refusal.contains("f10") && refusal.ends_with('…'),
            "{refusal}"
        );
    }

    #[test]
    fn release_refuses_without_git_without_head_or_with_a_foreign_explicit_sha() {
        assert!(stamp(Some("abc1234"), None, Mode::Release).is_err());
        assert!(stamp(None, Some(&facts(None, &[])), Mode::Release).is_err());
        let refusal = stamp(
            Some("feed123"),
            Some(&facts(Some("abc1234"), &[])),
            Mode::Release,
        )
        .unwrap_err();
        assert!(
            refusal.contains("feed123") && refusal.contains("abc1234"),
            "{refusal}"
        );
    }
}
