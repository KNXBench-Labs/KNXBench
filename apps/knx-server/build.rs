//! Stamps the short git commit into the build so `--version` can name it.
//!
//! Emits `KNX_BUILD_SHA` for `option_env!`. Order of preference: an
//! explicit `KNX_BUILD_SHA` environment variable (a source tarball or a
//! Docker build has no `.git`, so the sha comes in from outside), then
//! `git rev-parse --short HEAD` — but only when the repository git finds
//! is *this* workspace, not some unrelated checkout the tree happens to
//! be unpacked inside — then nothing. Nothing is not an error: a build
//! without git still builds, it just cannot say which commit it is. The
//! sha names the commit, not the working tree — uncommitted edits are
//! invisible here, on purpose, because cargo cannot watch "dirty".

use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=KNX_BUILD_SHA");

    let sha = std::env::var("KNX_BUILD_SHA")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(sha_from_git)
        .unwrap_or_default();
    println!("cargo:rustc-env=KNX_BUILD_SHA={sha}");
}

/// The abbreviated commit hash, if this workspace is a git checkout with
/// a working `git` on `PATH`.
///
/// Two guards. First, the repository git answers for must be this
/// workspace: a tarball unpacked inside someone else's checkout would
/// otherwise be stamped with that checkout's commit, and a false version
/// is worse than none. Second, cargo is told what to watch so the sha
/// cannot go stale: `HEAD` (branch switches, detached checkouts), the
/// `HEAD` reflog (appended on every commit, checkout and reset, whether
/// or not the branch ref is packed — per worktree, present by default in
/// any non-bare repository, and if it is missing cargo simply re-runs
/// this script every build, which is the fresh direction to fail in),
/// and the loose branch ref when there is one. The reflog is the
/// load-bearing watch: after `git pack-refs` (which `git gc --auto` runs
/// routinely) the branch file disappears and commits no longer touch
/// `HEAD` itself, so without it every later commit would be invisible to
/// `--version` until `HEAD` moved.
fn sha_from_git() -> Option<String> {
    let toplevel = canonical(&git(&["rev-parse", "--show-toplevel"])?)?;
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").ok()?;
    let workspace = canonical(&format!("{manifest_dir}/../.."))?;
    if toplevel != workspace {
        return None;
    }

    for watched in ["HEAD", "logs/HEAD"] {
        if let Some(path) = git(&["rev-parse", "--git-path", watched]) {
            println!("cargo:rerun-if-changed={path}");
        }
    }
    if let Some(branch) = git(&["symbolic-ref", "-q", "HEAD"]) {
        if let Some(path) = git(&["rev-parse", "--git-path", &branch]) {
            if Path::new(&path).exists() {
                println!("cargo:rerun-if-changed={path}");
            }
        }
    }
    git(&["rev-parse", "--short", "HEAD"])
}

fn canonical(path: &str) -> Option<PathBuf> {
    Path::new(path).canonicalize().ok()
}

fn git(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_string())
}
