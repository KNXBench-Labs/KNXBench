//! CLI CSV group-address exchange targets the installation named by `--installation`.
//!
//! MODEL-01: `knx ga-export`/`ga-import --installation <id>` target any
//! installation; without the flag the first one stays the target.
//!
//! Mirrors `apps/knx-server/tests/csv_installation_scope.rs` for the CLI:
//! Home (installation 0) and Garage (installation 1) both use `1/1/1`.

use std::path::Path;
use std::process::{Command, Output};

use knx_core::{
    CompletionStatus, GroupAddress, GroupAddressEntry, GroupAddressId, GroupAddressStyle,
    IdAllocators, Installation, InstallationId, Language, Project, SourceRef, Topology,
};

fn installation(id: u8, name: &str, ga_id: u32, ga_name: &str) -> Installation {
    Installation {
        id: InstallationId(id),
        name: name.into(),
        default_line: None,
        multicast_address: None,
        completion: CompletionStatus::FinishedDesign,
        topology: Topology {
            areas: vec![],
            lines: vec![],
            unassigned: vec![],
        },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses: vec![GroupAddressEntry {
            id: GroupAddressId(ga_id),
            source: SourceRef {
                path: format!("ga{ga_id}"),
                ets_id: format!("ga{ga_id}"),
            },
            name: ga_name.into(),
            address: GroupAddress::parse("1/1/1", GroupAddressStyle::ThreeLevel).unwrap(),
            central: false,
            unfiltered: false,
            range: None,
        }],
        parameters: vec![],
    }
}

fn write_store(path: &Path) {
    let mut project = Project::new(Language("en".into()));
    project
        .installations
        .push(installation(0, "Home", 1, "Home light"));
    project
        .installations
        .push(installation(1, "Garage", 2, "Garage light"));
    project
        .ids
        .raise_to(&IdAllocators::from_counts(0, 0, 0, 0, 0, 2, 0, 0, 0));
    let conn = knx_store::open_and_migrate(path).unwrap();
    knx_store::save_project(&conn, &project).unwrap();
}

/// `(installation, address, name)` for every group address in the store.
fn store_addresses(path: &Path) -> Vec<(u8, String, String)> {
    let conn = knx_store::open_and_migrate(path).unwrap();
    let project = knx_store::load_project(&conn).unwrap();
    project
        .installations
        .iter()
        .flat_map(|installation| {
            installation.group_addresses.iter().map(|ga| {
                (
                    installation.id.0,
                    ga.address.format(GroupAddressStyle::ThreeLevel),
                    ga.name.clone(),
                )
            })
        })
        .collect()
}

fn run_cli(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_knx"))
        .args(args)
        .output()
        .expect("failed to run the knx binary")
}

#[test]
fn ga_export_and_import_target_the_named_installation() {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("p.knxdb");
    write_store(&store);
    let store_arg = store.to_str().unwrap();

    let out = dir.path().join("garage.csv");
    let export = run_cli(&[
        "ga-export",
        store_arg,
        out.to_str().unwrap(),
        "--installation",
        "1",
    ]);
    assert!(export.status.success(), "{export:?}");
    let text = std::fs::read_to_string(&out).unwrap();
    assert!(
        text.contains("Garage light") && !text.contains("Home light"),
        "{text}"
    );

    let csv = dir.path().join("in.csv");
    std::fs::write(
        &csv,
        "Address,Name\n1/1/1,Garage renamed\n1/2/9,Garage new\n",
    )
    .unwrap();
    let import = run_cli(&[
        "ga-import",
        store_arg,
        csv.to_str().unwrap(),
        "--installation",
        "1",
    ]);
    assert!(import.status.success(), "{import:?}");
    assert_eq!(
        store_addresses(&store),
        [
            (0, "1/1/1".into(), "Home light".into()),
            (1, "1/1/1".into(), "Garage renamed".into()),
            (1, "1/2/9".into(), "Garage new".into()),
        ]
    );
}

#[test]
fn without_the_flag_the_first_installation_stays_the_target() {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("p.knxdb");
    write_store(&store);
    let csv = dir.path().join("in.csv");
    std::fs::write(&csv, "Address,Name\n1/1/1,Home renamed\n").unwrap();
    let import = run_cli(&["ga-import", store.to_str().unwrap(), csv.to_str().unwrap()]);
    assert!(import.status.success(), "{import:?}");
    assert_eq!(
        store_addresses(&store),
        [
            (0, "1/1/1".into(), "Home renamed".into()),
            (1, "1/1/1".into(), "Garage light".into()),
        ]
    );
}

#[test]
fn an_unknown_installation_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("p.knxdb");
    write_store(&store);
    let before = store_addresses(&store);
    let out = dir.path().join("x.csv");
    let export = run_cli(&[
        "ga-export",
        store.to_str().unwrap(),
        out.to_str().unwrap(),
        "--installation",
        "7",
    ]);
    assert!(!export.status.success(), "{export:?}");
    assert!(!out.exists());

    let csv = dir.path().join("in.csv");
    std::fs::write(&csv, "Address,Name\n1/1/1,X\n").unwrap();
    let import = run_cli(&[
        "ga-import",
        store.to_str().unwrap(),
        csv.to_str().unwrap(),
        "--installation",
        "7",
    ]);
    assert!(!import.status.success(), "{import:?}");
    let stdout = String::from_utf8_lossy(&import.stdout);
    assert!(stdout.contains("installation 7 does not exist"), "{stdout}");
    assert_eq!(store_addresses(&store), before);

    let bad = run_cli(&[
        "ga-import",
        store.to_str().unwrap(),
        csv.to_str().unwrap(),
        "--installation",
        "garage",
    ]);
    assert!(!bad.status.success(), "{bad:?}");
}

/// A delete previewed for Home yields a token that does not confirm the
/// same file against the Garage.
#[test]
fn a_preview_token_does_not_cross_installations() {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("p.knxdb");
    write_store(&store);
    let store_arg = store.to_str().unwrap();
    let csv = dir.path().join("delete.csv");
    std::fs::write(&csv, "Address,Action,Name\n1/1/1,delete,x\n").unwrap();
    let csv_arg = csv.to_str().unwrap();

    let preview = run_cli(&["ga-import", store_arg, csv_arg, "--installation", "0"]);
    let stdout = String::from_utf8_lossy(&preview.stdout).into_owned();
    let token = stdout
        .lines()
        .find_map(|line| line.strip_prefix("confirmation token: "))
        .unwrap_or_else(|| panic!("no token in {stdout}"))
        .to_string();

    let crossed = run_cli(&[
        "ga-import",
        store_arg,
        csv_arg,
        "--installation",
        "1",
        "--confirm",
        &token,
    ]);
    assert!(!crossed.status.success(), "{crossed:?}");
    assert_eq!(store_addresses(&store).len(), 2, "nothing deleted");

    let confirmed = run_cli(&[
        "ga-import",
        store_arg,
        csv_arg,
        "--installation",
        "0",
        "--confirm",
        &token,
    ]);
    assert!(confirmed.status.success(), "{confirmed:?}");
    assert_eq!(
        store_addresses(&store),
        [(1, "1/1/1".into(), "Garage light".into())]
    );
}
