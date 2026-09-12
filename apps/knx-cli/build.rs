//! Stamps the short git commit into the build so `--version` can name it.
//!
//! Emits `KNX_BUILD_SHA` for `option_env!`. Order of preference: an
//! explicit `KNX_BUILD_SHA` environment variable (a source tarball or a
//! Docker build has no `.git`, so the sha comes in from outside), then
//! `git rev-parse --short HEAD`, then nothing. Nothing is not an error: a
//! build without git still builds, it just cannot say which commit it is.
//! The sha names the commit, not the working tree — uncommitted edits are
//! invisible here, on purpose, because cargo cannot watch "dirty".

use std::path::Path;
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

/// The abbreviated commit hash, if this is a git checkout with a working
/// `git` on `PATH`. Also tells cargo to re-run this script when `HEAD`
/// moves: the `HEAD` file itself and, when it is symbolic, the branch ref
/// it points at (only if that ref is loose; a packed ref has no file of
/// its own to watch, and a nonexistent path would make cargo re-run every
/// build).
fn sha_from_git() -> Option<String> {
    if let Some(head) = git(&["rev-parse", "--git-path", "HEAD"]) {
        println!("cargo:rerun-if-changed={head}");
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

fn git(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_string())
}
