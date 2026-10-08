//! `knx products import-legacy` publishes a legacy EX-IM database into the product database.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const PASSWORD: &str = "marvin-synthetic";

fn fixture(name: &str) -> PathBuf {
    knx_testsupport::workspace_root()
        .join("crates/knx-productdb/fixtures/legacy")
        .join(name)
}

fn run(args: &[&str], stdin: Option<&str>, home: &Path) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_knx"));
    command
        .args(args)
        .env("HOME", home)
        .env("XDG_DATA_HOME", home.join("data"))
        .env("XDG_CONFIG_HOME", home.join("config"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().expect("failed to run the knx binary");
    if let Some(input) = stdin {
        child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(input.as_bytes())
            .unwrap();
    }
    drop(child.stdin.take());
    child.wait_with_output().unwrap()
}

fn text(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

fn import(home: &Path, db: &Path, file: &str, password: Option<&str>) -> Output {
    let path = fixture(file);
    let mut args = vec![
        "products",
        "import-legacy",
        path.to_str().unwrap(),
        "--product-db",
        db.to_str().unwrap(),
    ];
    if password.is_some() {
        args.push("--password-stdin");
    }
    let stdin = password.map(|p| format!("{p}\n"));
    run(&args, stdin.as_deref(), home)
}

fn legacy_programs(db: &Path) -> i64 {
    let conn = knx_productdb::open_and_migrate(db).unwrap();
    conn.query_row("SELECT count(*) FROM legacy_program", [], |r| r.get(0))
        .unwrap()
}

#[test]
fn an_encrypted_database_is_imported_with_the_password() {
    let home = tempfile::tempdir().unwrap();
    let db = home.path().join("products.sqlite");
    let out = import(home.path(), &db, "marvin-program.vd4", Some(PASSWORD));
    let shown = text(&out);
    assert_eq!(out.status.code(), Some(0), "{shown}");
    assert!(shown.contains("imported"), "{shown}");
    assert!(shown.contains("M-1092_A-LX"), "{shown}");
    assert!(shown.contains("1 application program"), "{shown}");
    assert!(shown.contains("unmapped-table"), "{shown}");
    assert!(!shown.contains(PASSWORD), "{shown}");
    assert_eq!(legacy_programs(&db), 1);
}

#[test]
fn importing_the_same_content_again_is_reported_as_already_imported() {
    let home = tempfile::tempdir().unwrap();
    let db = home.path().join("products.sqlite");
    assert_eq!(
        import(home.path(), &db, "marvin-program.vd4", Some(PASSWORD))
            .status
            .code(),
        Some(0)
    );
    let out = import(home.path(), &db, "marvin-program-plain.vd4", None);
    let shown = text(&out);
    assert_eq!(out.status.code(), Some(0), "{shown}");
    assert!(shown.contains("already imported"), "{shown}");
    assert_eq!(legacy_programs(&db), 1);
}

#[test]
fn a_missing_password_is_asked_for_and_nothing_is_written() {
    let home = tempfile::tempdir().unwrap();
    let db = home.path().join("products.sqlite");
    let out = import(home.path(), &db, "marvin-program.vd4", None);
    let shown = text(&out);
    assert_ne!(out.status.code(), Some(0), "{shown}");
    assert!(shown.contains("--password-stdin"), "{shown}");
    // Decryption comes first: not even an empty product database is created.
    assert!(!db.exists(), "{shown}");
}

#[test]
fn a_wrong_password_writes_nothing() {
    let home = tempfile::tempdir().unwrap();
    let db = home.path().join("products.sqlite");
    let out = import(home.path(), &db, "marvin-program.vd4", Some("canary-wrong"));
    let shown = text(&out);
    assert_ne!(out.status.code(), Some(0), "{shown}");
    assert!(shown.contains("wrong password"), "{shown}");
    assert!(!shown.contains("canary-wrong"), "{shown}");
    assert!(!db.exists(), "{shown}");
}

#[test]
fn a_password_on_the_command_line_is_refused_without_echo() {
    let home = tempfile::tempdir().unwrap();
    let path = fixture("marvin-program.vd4");
    let out = run(
        &[
            "products",
            "import-legacy",
            path.to_str().unwrap(),
            "--password=canary-argv",
        ],
        None,
        home.path(),
    );
    let shown = text(&out);
    assert_ne!(out.status.code(), Some(0));
    assert!(
        shown.contains("never accepted on the command line"),
        "{shown}"
    );
    assert!(!shown.contains("canary-argv"), "{shown}");
}
