//! `git_facts` against real temporary repositories: clean, modified, untracked, ignored, foreign.

use std::path::Path;
use std::process::Command;

use knx_build_stamp::{git_facts, stamp, Mode};

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args([
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
        ])
        .args(args)
        .status()
        .expect("git must be installed to run this test");
    assert!(status.success(), "git {args:?} failed");
}

fn committed_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    std::fs::write(dir.path().join(".gitignore"), "target/\n").unwrap();
    std::fs::write(dir.path().join("main.rs"), "fn main() {}\n").unwrap();
    git(dir.path(), &["add", "."]);
    git(dir.path(), &["commit", "-q", "-m", "first"]);
    dir
}

#[test]
fn a_clean_checkout_is_stamped_in_release_mode() {
    let repo = committed_repo();
    let facts = git_facts(repo.path()).expect("the repository is the workspace");
    assert!(facts.modified.is_empty(), "{facts:?}");
    let head = facts.head.clone().unwrap();
    assert_eq!(stamp(None, Some(&facts), Mode::Release).unwrap(), head);
}

#[test]
fn modified_and_untracked_paths_block_a_release_but_ignored_ones_do_not() {
    let repo = committed_repo();
    std::fs::create_dir(repo.path().join("target")).unwrap();
    std::fs::write(repo.path().join("target/out.bin"), "build output").unwrap();
    assert!(git_facts(repo.path()).unwrap().modified.is_empty());

    std::fs::write(repo.path().join("main.rs"), "fn main() { edited(); }\n").unwrap();
    std::fs::write(repo.path().join("notes.txt"), "scratch").unwrap();
    let facts = git_facts(repo.path()).unwrap();
    let refusal = stamp(None, Some(&facts), Mode::Release).unwrap_err();
    assert!(
        refusal.contains("main.rs") && refusal.contains("notes.txt"),
        "{refusal}"
    );
    // Development builds keep naming HEAD, as before.
    assert_eq!(
        stamp(None, Some(&facts), Mode::Development).unwrap(),
        facts.head.unwrap()
    );
}

#[test]
fn a_tree_inside_a_foreign_repository_borrows_nothing() {
    let outer = committed_repo();
    let inner = outer.path().join("unpacked-tarball");
    std::fs::create_dir(&inner).unwrap();
    assert_eq!(git_facts(&inner), None);
    assert!(stamp(None, None, Mode::Release).is_err());
}
