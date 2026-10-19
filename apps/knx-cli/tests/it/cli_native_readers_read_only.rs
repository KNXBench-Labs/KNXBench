//! Reading CLI commands leave older, current and refused native stores byte-identical.
//
// KL-157 / ADR-0090: a command that only reads a `.knxdb` must not migrate,
// rewrite or add a sidecar to the user's file. The older inputs are the
// committed schema-13 fixtures written by the real predecessor writer: one
// with only a saved project, one with a working history (undo/redo) and
// retained opaque entries. Every command here is offline: download runs in
// plan mode, and compare/probe stop at an unreadable product database before
// any connection could exist.

use std::path::{Path, PathBuf};
use std::process::Output;

const SAVED: &[u8] = include_bytes!(
    "../../../../crates/knx-store/tests/fixtures/native13-before-notes-saved.sqlite"
);
const WORKING: &[u8] = include_bytes!(
    "../../../../crates/knx-store/tests/fixtures/native13-before-notes-working.sqlite"
);

fn knx(args: &[&str]) -> Output {
    std::process::Command::new(env!("CARGO_BIN_EXE_knx"))
        .args(args)
        .output()
        .expect("run knx")
}

/// Every file in `dir` with its bytes, sorted by name: a journal or WAL
/// sidecar left behind counts as a change just like rewritten bytes.
fn directory_bytes(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut files: Vec<(String, Vec<u8>)> = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            (
                entry.file_name().to_string_lossy().into_owned(),
                std::fs::read(entry.path()).unwrap(),
            )
        })
        .collect();
    files.sort();
    files
}

/// The SQLite header's `user_version` (big-endian, offset 60), read from the
/// bytes so that checking the version never opens the file.
fn schema_version(path: &Path) -> i64 {
    let bytes = std::fs::read(path).unwrap();
    i64::from(u32::from_be_bytes(bytes[60..64].try_into().unwrap()))
}

/// A project directory holding only `name` with `bytes`, plus a separate
/// directory for everything a command may legitimately create.
struct Case {
    project_dir: tempfile::TempDir,
    other_dir: tempfile::TempDir,
    project: PathBuf,
}

impl Case {
    fn new(bytes: &[u8]) -> Self {
        let project_dir = tempfile::tempdir().unwrap();
        let other_dir = tempfile::tempdir().unwrap();
        let project = project_dir.path().join("project.knxdb");
        std::fs::write(&project, bytes).unwrap();
        Self {
            project_dir,
            other_dir,
            project,
        }
    }

    fn project(&self) -> &str {
        self.project.to_str().unwrap()
    }

    fn other(&self, name: &str) -> String {
        self.other_dir.path().join(name).display().to_string()
    }

    fn not_a_database(&self) -> String {
        let path = self.other("not-a-database.sqlite");
        std::fs::write(&path, b"plain text, not SQLite").unwrap();
        path
    }

    fn assert_untouched_by(&self, label: &str, run: impl FnOnce(&Self) -> Output) {
        let before = directory_bytes(self.project_dir.path());
        let output = run(self);
        assert_eq!(
            directory_bytes(self.project_dir.path()),
            before,
            "{label} changed the project directory; stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// One reading command, named, as it runs offline against a [`Case`].
type ReadingCommand = (&'static str, fn(&Case) -> Output);

/// Every reading command, as it runs offline.
fn reading_commands() -> Vec<ReadingCommand> {
    vec![
        ("ga-export", |c| {
            knx(&["ga-export", c.project(), &c.other("out.csv")])
        }),
        ("doc-export", |c| {
            knx(&["doc-export", c.project(), &c.other("out.html")])
        }),
        ("diff", |c| knx(&["diff", c.project(), c.project()])),
        ("device readiness", |c| {
            knx(&[
                "device",
                "readiness",
                "--project",
                c.project(),
                "--product-db",
                &c.other("products.sqlite"),
            ])
        }),
        ("device download (plan)", |c| {
            knx(&[
                "device",
                "download",
                "1.1.1",
                "--project",
                c.project(),
                "--product-db",
                &c.other("products.sqlite"),
            ])
        }),
        ("device compare", |c| {
            knx(&[
                "device",
                "compare",
                "1.1.1",
                "--project",
                c.project(),
                "--product-db",
                &c.not_a_database(),
                "--gateway",
                "127.0.0.1:9",
            ])
        }),
        ("device byte-order-probe", |c| {
            knx(&[
                "device",
                "byte-order-probe",
                "1.1.1",
                "--project",
                c.project(),
                "--product-db",
                &c.not_a_database(),
                "--gateway",
                "127.0.0.1:9",
            ])
        }),
        ("ga-import --dry-run", |c| {
            let csv = c.other("in.csv");
            std::fs::write(&csv, "Address,Name\n1/1/1,Planned\n").unwrap();
            knx(&["ga-import", c.project(), &csv, "--dry-run"])
        }),
        ("ga-import rejected", |c| {
            let csv = c.other("bad.csv");
            std::fs::write(&csv, "Address,Name\nnot-an-address,Broken\n").unwrap();
            knx(&["ga-import", c.project(), &csv])
        }),
    ]
}

#[test]
fn reading_commands_leave_older_stores_and_their_history_untouched() {
    for (fixture, bytes) in [("saved", SAVED), ("working", WORKING)] {
        for (label, run) in reading_commands() {
            let case = Case::new(bytes);
            case.assert_untouched_by(&format!("{label} on the {fixture} v13 fixture"), run);
            assert_eq!(schema_version(&case.project), 13);
        }
    }
}

#[test]
fn reading_commands_succeed_on_an_older_store() {
    for (fixture, bytes) in [("saved", SAVED), ("working", WORKING)] {
        let case = Case::new(bytes);
        for (label, args) in [
            (
                "ga-export",
                vec!["ga-export", case.project(), &case.other("a.csv")],
            ),
            (
                "doc-export",
                vec!["doc-export", case.project(), &case.other("a.html")],
            ),
            (
                "diff",
                vec!["diff", "--exit-code", case.project(), case.project()],
            ),
        ] {
            let args: Vec<&str> = args.iter().map(|arg| arg.as_ref()).collect();
            let output = knx(&args);
            assert!(
                output.status.success(),
                "{label} on the {fixture} fixture failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
}

#[test]
fn ga_import_refusals_say_nothing_was_written_and_mean_it() {
    let case = Case::new(SAVED);
    let csv = case.other("bad.csv");
    std::fs::write(&csv, "Address,Name\nnot-an-address,Broken\n").unwrap();
    let before = directory_bytes(case.project_dir.path());

    let output = knx(&["ga-import", case.project(), &csv]);

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("store written: no"));
    assert_eq!(directory_bytes(case.project_dir.path()), before);
}

#[test]
fn reading_commands_leave_a_current_store_untouched() {
    let current = {
        let case = Case::new(SAVED);
        let conn = knx_store::open_and_migrate(&case.project).unwrap();
        drop(conn);
        std::fs::read(&case.project).unwrap()
    };
    for (label, run) in reading_commands() {
        let case = Case::new(&current);
        case.assert_untouched_by(&format!("{label} on a current store"), run);
    }
}

#[test]
fn a_newer_store_is_refused_untouched_by_every_reader() {
    let newer = {
        let case = Case::new(SAVED);
        drop(knx_store::open_and_migrate(&case.project).unwrap());
        knx_store::Connection::open(&case.project)
            .unwrap()
            .pragma_update(None, "user_version", knx_store::CURRENT_SCHEMA_VERSION + 1)
            .unwrap();
        std::fs::read(&case.project).unwrap()
    };
    for (label, run) in reading_commands() {
        let case = Case::new(&newer);
        case.assert_untouched_by(&format!("{label} on a newer store"), |c| {
            let output = run(c);
            assert!(!output.status.success(), "{label} accepted a newer store");
            output
        });
    }
}

/// The deliberate writer keeps its documented boundary: a confirmed
/// `ga-import` into an older store saves the change and, with it, upgrades
/// the file (KL-157 remaining boundary, disclosed in the dossier).
#[test]
fn a_real_ga_import_still_writes_and_upgrades_an_older_store() {
    let case = Case::new(SAVED);
    let csv = case.other("in.csv");
    std::fs::write(&csv, "Address,Name\n1/1/1,Written\n").unwrap();

    let output = knx(&["ga-import", case.project(), &csv]);

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("store written: yes"));
    assert_eq!(
        schema_version(&case.project),
        knx_store::CURRENT_SCHEMA_VERSION
    );
    let read = knx_store::open_existing_read_only(&case.project).unwrap();
    let project = knx_store::load_project(&read.conn).unwrap();
    assert!(project
        .installations
        .iter()
        .flat_map(|installation| &installation.group_addresses)
        .any(|entry| entry.name == "Written"));
}
