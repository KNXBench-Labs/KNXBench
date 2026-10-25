//! `knx device download --changes`: the flag advice, printed and bound, never written by itself.
//!
//! Synthetic product data only (`knx_testsupport::vendor_plugin` without its
//! plug-in declaration); every run here is a plan and opens no connection.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use knx_core::commissioning::LoadPart;
use knx_core::{
    CommissioningState, CompletionStatus, DeviceId, DeviceInstance, Installation, InstallationId,
    Language, Project, SourceRef, Topology,
};
use knx_testsupport::vendor_plugin::{
    program_xml, HARDWARE2PROGRAM_REF, HARDWARE_XML, PRODUCT_REF, TARGET,
};

/// The five flags as bits 0..5 of `bits`, in schema order.
fn flags(bits: u8) -> CommissioningState {
    let mut state = CommissioningState::default();
    for (index, part) in LoadPart::ALL.into_iter().enumerate() {
        state.set_intends_loaded(part, bits & (1 << index) != 0);
    }
    state
}

/// `(project.knxdb, products.sqlite)` with one device at `1.1.7`.
fn inputs(dir: &Path, bits: u8) -> (PathBuf, PathBuf) {
    let products = dir.join("products.sqlite");
    if !products.exists() {
        let db = knx_productdb::open_and_migrate(&products).unwrap();
        knx_productdb::ingest_file(&db, "M-FFF6/A.xml", program_xml("").as_bytes()).unwrap();
        knx_productdb::ingest_file(&db, "M-FFF6/Hardware.xml", HARDWARE_XML.as_bytes()).unwrap();
    }
    let mut project = Project::new(Language("en".into()));
    project.installations.push(Installation {
        id: InstallationId(0),
        name: "I".into(),
        default_line: None,
        multicast_address: None,
        completion: CompletionStatus::FinishedDesign,
        topology: Topology {
            areas: vec![],
            lines: vec![],
            unassigned: vec![DeviceId(1)],
        },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses: vec![],
        parameters: vec![],
    });
    project.devices.insert(DeviceInstance {
        id: DeviceId(1),
        source: SourceRef {
            path: "synthetic".into(),
            ets_id: "DEV-1".into(),
        },
        name: "Synthetic actuator".into(),
        description: None,
        address: Some(TARGET.parse().unwrap()),
        product_ref: PRODUCT_REF.into(),
        program_ref: HARDWARE2PROGRAM_REF.into(),
        commissioning: flags(bits),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
    let path = dir.join(format!("project-{bits:05b}.knxdb"));
    let conn = knx_store::open_and_migrate(&path).unwrap();
    knx_store::save_project(&conn, &project).unwrap();
    (path, products)
}

fn knx(args: &[&str], project: &Path, products: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_knx"))
        .args(args)
        .arg("--project")
        .arg(project)
        .arg("--product-db")
        .arg(products)
        .stdin(Stdio::null())
        .output()
        .unwrap()
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn changes_prints_and_plans_the_advised_scope_for_every_flag_combination() {
    let dir = tempfile::tempdir().unwrap();
    for bits in 0u8..32 {
        let (project, products) = inputs(dir.path(), bits);
        let output = knx(
            &["device", "download", TARGET, "--changes"],
            &project,
            &products,
        );
        let shown = text(&output);
        let case = format!("{bits:05b}:\n{shown}");
        assert!(output.status.success(), "{case}");
        assert!(
            shown.contains("programming flags (the project's belief, not read from the device)"),
            "{case}"
        );
        let (application, parameters, group_addresses) =
            (bits & 0b10 != 0, bits & 0b100 != 0, bits & 0b1000 != 0);
        let (advised, partial_flag) = match (application, parameters, group_addresses) {
            (false, _, _) => ("complete download", None),
            (true, false, false) => ("parameters and group addresses", Some("both")),
            (true, false, true) => ("parameters only", Some("parameters")),
            (true, true, false) => ("group addresses only", Some("group-addresses")),
            (true, true, true) => ("nothing", None),
        };
        assert!(
            shown.contains(&format!("  advised download: {advised}")),
            "{case}"
        );
        assert!(
            shown.contains(if bits & 1 == 0 {
                "  individual address: pending"
            } else {
                "  individual address: programmed"
            }),
            "{case}"
        );
        assert!(
            shown.contains(if bits & 0b10000 == 0 {
                "  medium configuration: pending"
            } else {
                "  medium configuration: programmed"
            }),
            "{case}"
        );
        assert!(shown.contains("written to the device: no"), "{case}");
        if advised == "nothing" {
            assert!(shown.contains("nothing to download"), "{case}");
            assert!(!shown.contains("steps:"), "no plan may be built: {case}");
            assert_eq!(
                shown.contains("the project marks every part programmed"),
                bits == 0b11111,
                "{case}"
            );
            continue;
        }
        assert!(shown.contains("steps:"), "{case}");
        let rerun = match partial_flag {
            Some(flag) => format!("--partial {flag}"),
            None => "the complete download (no --partial)".to_string(),
        };
        assert!(
            shown.contains(&format!("--changes resolved to {rerun}")),
            "{case}"
        );
        assert_eq!(
            shown.contains("partial download (CP §3.9.2.4)"),
            partial_flag.is_some(),
            "{case}"
        );
    }
}

#[test]
fn changes_never_writes_and_never_combines_with_an_explicit_scope() {
    let dir = tempfile::tempdir().unwrap();
    let (project, products) = inputs(dir.path(), 0b11011);
    let phrase = format!("I confirm download to {TARGET}");
    let output = knx(
        &[
            "device",
            "download",
            TARGET,
            "--changes",
            "--gateway",
            "192.0.2.10:3671",
            "--confirm",
            &phrase,
            "--activity-history",
            dir.path().join("history.sqlite").to_str().unwrap(),
        ],
        &project,
        &products,
    );
    let shown = text(&output);
    assert!(!output.status.success(), "{shown}");
    assert!(
        shown.contains("--changes only plans; write with the concrete scope it prints"),
        "{shown}"
    );
    assert!(!shown.contains("steps:"), "{shown}");

    let output = knx(
        &[
            "device",
            "download",
            TARGET,
            "--changes",
            "--partial",
            "both",
        ],
        &project,
        &products,
    );
    let shown = text(&output);
    assert!(!output.status.success(), "{shown}");
    assert!(
        shown.contains("--changes and --partial exclude each other"),
        "{shown}"
    );
}

#[test]
fn an_explicit_plan_is_unchanged_and_shows_the_advice_beside_it() {
    let dir = tempfile::tempdir().unwrap();
    // Everything marked programmed; the explicit complete plan is still built.
    let (project, products) = inputs(dir.path(), 0b11111);
    let output = knx(&["device", "download", TARGET], &project, &products);
    let shown = text(&output);
    assert!(output.status.success(), "{shown}");
    assert!(shown.contains("steps:"), "{shown}");
    assert!(!shown.contains("partial download (CP §3.9.2.4)"), "{shown}");
    assert!(shown.contains("  advised download: nothing"), "{shown}");
    assert!(!shown.contains("--changes resolved"), "{shown}");
    assert!(
        shown.contains("written to the device: no (plan only; add --gateway and --confirm"),
        "{shown}"
    );
}

/// POSIX single quotes: the one form in which nothing but `'` is special.
fn quoted(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

/// The exact command that writes the plan `--changes` resolved to, with the
/// placeholders the run printing it left to fill.
fn rerun_line(project: &Path, products: &Path, partial: Option<&str>) -> String {
    let partial = partial
        .map(|p| format!(" --partial {p}"))
        .unwrap_or_default();
    format!(
        "  knx device download '{TARGET}' --project {} --product-db {}{partial} \
         --activity-history '<activity history file>' --gateway '<gateway host:port>' \
         --confirm 'I confirm download to {TARGET}' \
         --accept-untested 'I accept an untested download to {TARGET}'\n",
        quoted(project.to_str().unwrap()),
        quoted(products.to_str().unwrap()),
    )
}

/// The command lines between the hint and the plan-only tail.
fn printed_command(shown: &str) -> &str {
    let start = shown.find("run:\n").expect("the hint") + "run:\n".len();
    let end = shown
        .rfind("\nwritten to the device: no (plan only)\n")
        .expect("the tail");
    &shown[start..end]
}

/// The arguments a POSIX shell parses from `command`, by a `knx` shell
/// function that only prints them: nothing is downloaded or connected.
fn shell_arguments(command: &str) -> Vec<String> {
    let script = format!("knx() {{ printf '%s\\0' \"$@\"; }}\n{command}\n");
    let output = Command::new("sh")
        .arg("-c")
        .arg(script)
        .env_clear()
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    let mut arguments: Vec<String> = String::from_utf8(output.stdout)
        .unwrap()
        .split('\0')
        .map(str::to_string)
        .collect();
    assert_eq!(arguments.pop().as_deref(), Some(""));
    arguments
}

#[test]
fn changes_prints_the_explicit_command_to_write_it_never_confirm_on_changes() {
    let dir = tempfile::tempdir().unwrap();
    for (bits, partial) in [
        (0b11011, Some("parameters")),
        (0b11101, None),
        (0b10011, Some("both")),
    ] {
        let (project, products) = inputs(dir.path(), bits);
        let output = knx(
            &["device", "download", TARGET, "--changes"],
            &project,
            &products,
        );
        let shown = text(&output);
        assert!(output.status.success(), "{shown}");
        assert!(
            shown.contains(
                "to write exactly this plan, fill in each '<...>' placeholder and run:\n"
            ),
            "{bits:05b}:\n{shown}"
        );
        assert!(
            shown.contains(&rerun_line(&project, &products, partial)),
            "{bits:05b}:\n{shown}"
        );
        // `--confirm` is refused beside `--changes`: never suggest adding it.
        assert!(!shown.contains("add --gateway and --confirm"), "{shown}");
        assert!(!shown.contains("run again with it"), "{shown}");
        assert!(
            shown.ends_with("written to the device: no (plan only)\n"),
            "{bits:05b}:\n{shown}"
        );
    }
    // Given history, backup directory and gateway are kept as given.
    let (project, products) = inputs(dir.path(), 0b11011);
    let history = dir.path().join("history.sqlite");
    let backups = dir.path().join("backups");
    let output = knx(
        &[
            "device",
            "download",
            TARGET,
            "--changes",
            "--activity-history",
            history.to_str().unwrap(),
            "--backup-dir",
            backups.to_str().unwrap(),
            "--gateway",
            "192.0.2.10:3671",
        ],
        &project,
        &products,
    );
    let shown = text(&output);
    assert!(output.status.success(), "{shown}");
    let expected = format!(
        "to write exactly this plan, run:\n  knx device download '{TARGET}' --project {} \
         --product-db {} --partial parameters --activity-history {} --backup-dir {} \
         --gateway '192.0.2.10:3671' --confirm 'I confirm download to {TARGET}' \
         --accept-untested 'I accept an untested download to {TARGET}'\n\
         written to the device: no (plan only)\n",
        quoted(project.to_str().unwrap()),
        quoted(products.to_str().unwrap()),
        quoted(history.to_str().unwrap()),
        quoted(backups.to_str().unwrap()),
    );
    assert!(shown.ends_with(&expected), "{shown}");
    // A plan never opens or creates the history it names.
    assert!(!history.exists());
    assert!(!backups.exists());
}

#[test]
fn the_printed_command_survives_a_posix_shell_unchanged() {
    let root = tempfile::tempdir().unwrap();
    // Every character a shell treats specially inside or outside quotes.
    let dir = root.path().join("it's $HOME `id` \\ \"q\" *\nline");
    std::fs::create_dir(&dir).unwrap();
    let (project, products) = inputs(&dir, 0b11011);
    let history = dir.join("hist'ory $x.sqlite");
    let backups = dir.join("back`ups`\\");
    let output = knx(
        &[
            "device",
            "download",
            TARGET,
            "--changes",
            "--activity-history",
            history.to_str().unwrap(),
            "--backup-dir",
            backups.to_str().unwrap(),
        ],
        &project,
        &products,
    );
    let shown = text(&output);
    assert!(output.status.success(), "{shown}");
    let arguments = shell_arguments(printed_command(&shown));
    let expected: Vec<String> = [
        "device",
        "download",
        TARGET,
        "--project",
        project.to_str().unwrap(),
        "--product-db",
        products.to_str().unwrap(),
        "--partial",
        "parameters",
        "--activity-history",
        history.to_str().unwrap(),
        "--backup-dir",
        backups.to_str().unwrap(),
        "--gateway",
        "<gateway host:port>",
        "--confirm",
        &format!("I confirm download to {TARGET}"),
        "--accept-untested",
        &format!("I accept an untested download to {TARGET}"),
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    assert_eq!(arguments, expected, "{shown}");
    assert!(!history.exists());
    assert!(!backups.exists());
}

#[test]
fn nothing_pending_needs_no_product_data_and_nothing_else_skips_it() {
    let dir = tempfile::tempdir().unwrap();
    // A product database that cannot be opened at all.
    let broken = dir.path().join("broken.sqlite");
    std::fs::write(&broken, b"not a database").unwrap();
    let (project, _) = inputs(dir.path(), 0b01110);
    let output = knx(
        &["device", "download", TARGET, "--changes"],
        &project,
        &broken,
    );
    let shown = text(&output);
    assert!(output.status.success(), "{shown}");
    assert!(shown.contains("  advised download: nothing"), "{shown}");
    assert!(shown.contains("nothing to download"), "{shown}");
    assert!(shown.contains("written to the device: no"), "{shown}");
    assert!(!shown.contains("product database"), "{shown}");
    // Something pending, or an explicit scope, still needs it and says so.
    for (bits, args) in [
        (0b11011u8, vec!["device", "download", TARGET, "--changes"]),
        (0b11111, vec!["device", "download", TARGET]),
    ] {
        let (project, _) = inputs(dir.path(), bits);
        let output = knx(&args, &project, &broken);
        let shown = text(&output);
        assert!(!output.status.success(), "{bits:05b}: {shown}");
        assert!(shown.contains("failed to open product database"), "{shown}");
        assert!(!shown.contains("steps:"), "{shown}");
    }
}

#[test]
fn a_bad_target_is_named_before_the_product_database_is_opened() {
    // Documented precedence: the project resolves the device first. Both
    // the missing and the ambiguous target refuse; nothing is planned,
    // and the named history file is never created.
    let dir = tempfile::tempdir().unwrap();
    let broken = dir.path().join("broken.sqlite");
    std::fs::write(&broken, b"not a database").unwrap();
    let (single, _) = inputs(dir.path(), 0b11011);
    let twin_dir = dir.path().join("twin");
    std::fs::create_dir(&twin_dir).unwrap();
    let (ambiguous, _) = inputs(&twin_dir, 0b11011);
    {
        let conn = knx_store::open_and_migrate(&ambiguous).unwrap();
        let mut project = knx_store::load_project(&conn).unwrap();
        let mut twin = project.devices.get(DeviceId(1)).unwrap().clone();
        twin.id = DeviceId(2);
        project.devices.insert(twin);
        project.installations[0]
            .topology
            .unassigned
            .push(DeviceId(2));
        knx_store::save_project(&conn, &project).unwrap();
    }
    let history = dir.path().join("history.sqlite");
    for (project, address) in [(&single, "1.1.99"), (&ambiguous, TARGET)] {
        for scope in [None, Some("--changes")] {
            let mut args = vec![
                "device",
                "download",
                address,
                "--activity-history",
                history.to_str().unwrap(),
            ];
            args.extend(scope);
            let output = knx(&args, project, &broken);
            let shown = text(&output);
            assert!(!output.status.success(), "{shown}");
            assert!(
                shown.contains(&format!("no download to device {address} prepared")),
                "{shown}"
            );
            assert!(!shown.contains("product database"), "{shown}");
            assert!(!shown.contains("steps:"), "{shown}");
            assert!(shown.contains("written to the device: no"), "{shown}");
            assert!(!history.exists(), "{shown}");
        }
    }
}
