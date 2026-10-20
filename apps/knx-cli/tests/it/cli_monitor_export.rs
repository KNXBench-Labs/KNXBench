//! Actual offline CLI against synthetic native files; no imported archive or bus.
use std::{
    io::Read,
    path::Path,
    process::{Command, Output},
};

use knx_core::{
    CompletionStatus, GroupAddress, GroupAddressEntry, Installation, InstallationId, Language,
    Project, SourceRef, Topology,
};
use serde_json::Value;
use sha2::{Digest, Sha256};

mod layout;

fn source(tag: &str) -> SourceRef {
    SourceRef {
        path: tag.into(),
        ets_id: tag.into(),
    }
}

fn project() -> Project {
    let mut project = Project::new(Language("en".into()));
    project.info.name = "Synthetic monitor project".into();
    let id = project.ids.next_group_address_id().unwrap();
    project.installations.push(Installation {
        id: InstallationId(0),
        name: "Installation".into(),
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
        parameters: vec![],
        group_addresses: vec![GroupAddressEntry {
            id,
            source: source("GA-1"),
            name: "Synthetic light".into(),
            address: GroupAddress::parse("1/1/1", project.info.group_address_style).unwrap(),
            central: false,
            unfiltered: false,
            range: None,
            declared_dpt: Default::default(),
        }],
    });
    project
}

fn save(path: &Path, project: &Project) {
    let conn = knx_store::open_and_migrate(path).unwrap();
    knx_store::save_project(&conn, project).unwrap();
}

fn cli(dir: &Path, input: &Path, output: &Path, flags: &[&str]) -> Output {
    let home = dir.join("home");
    std::fs::create_dir_all(&home).unwrap();
    Command::new(env!("CARGO_BIN_EXE_knx"))
        .arg("monitor-export")
        .arg(input)
        .arg(output)
        .args(flags)
        .env("HOME", &home)
        .env("XDG_DATA_HOME", home.join("data"))
        .env("XDG_CONFIG_HOME", home.join("config"))
        .env("XDG_CACHE_HOME", home.join("cache"))
        .current_dir(dir)
        .output()
        .unwrap()
}

fn member(path: &Path, name: &str) -> Vec<u8> {
    let mut archive = zip::ZipArchive::new(std::fs::File::open(path).unwrap()).unwrap();
    let mut bytes = Vec::new();
    archive
        .by_name(name)
        .unwrap()
        .read_to_end(&mut bytes)
        .unwrap();
    bytes
}

#[test]
fn exports_native_project_and_hash_bound_report_without_mutation() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("project.knxdb");
    let output = dir.path().join("monitor.zip");
    save(&input, &project());
    let before = std::fs::read(&input).unwrap();
    let result = cli(dir.path(), &input, &output, &["--no-product-db"]);
    assert!(
        result.status.success(),
        "monitor-export must succeed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let bytes = member(&output, "project.json");
    let document: Value = serde_json::from_slice(&bytes).unwrap();
    let report: Value = serde_json::from_slice(&member(&output, "export-report.json")).unwrap();
    assert_eq!(document["info"]["name"], "Synthetic monitor project");
    assert_eq!(
        document["group_addresses"]["1/1/1"]["name"],
        "Synthetic light"
    );
    assert_eq!(
        report["project_sha256"],
        format!("{:x}", Sha256::digest(&bytes))
    );
    assert_eq!(report["profile"], "monitoring-sparse-v1");
    assert_eq!(report["counts"]["group_addresses"]["exported"], 1);
    assert!(!report["issues"].as_array().unwrap().is_empty());
    let summary = String::from_utf8(result.stdout).unwrap();
    for expected in ["reduced", "not a backup", "issues", "group_addresses"] {
        assert!(summary.contains(expected), "missing {expected}: {summary}");
    }
    assert!(summary.contains(&format!(
        "issues: {}",
        report["issues"].as_array().unwrap().len()
    )));
    assert_eq!(std::fs::read(&input).unwrap(), before);
    assert!(!dir.path().join("home/data/knx/products.sqlite").exists());
    let archive = zip::ZipArchive::new(std::fs::File::open(&output).unwrap()).unwrap();
    assert_eq!(archive.len(), 3);
    assert!(member(&output, "README.txt").is_ascii());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&output).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}

fn value<T>(value: T) -> knx_core::Override<T> {
    knx_core::Override::Value(knx_core::Resolved {
        value,
        layer: knx_core::Layer::UserEdit,
    })
}

fn project_with_object() -> Project {
    use knx_core::{
        ComObjectInstance, CommissioningState, DeviceInstance, Direction, DptRef, GroupLink,
        ResolvedFlags, Text,
    };
    let mut project = project();
    let device = project.ids.next_device_id().unwrap();
    let object = project.ids.next_com_object_instance_id().unwrap();
    let ga = project.installations[0].group_addresses[0].id;
    project.installations[0].group_addresses[0].declared_dpt = value(DptRef {
        main: 1,
        sub: Some(1),
    });
    let mut other_ga = project.installations[0].group_addresses[0].clone();
    other_ga.id = project.ids.next_group_address_id().unwrap();
    other_ga.address = GroupAddress::from_raw(other_ga.address.raw() + 1);
    other_ga.name = "Other address".into();
    project.installations[0].group_addresses.push(other_ga);
    project.installations[0].topology.unassigned.push(device);
    project.devices.insert(DeviceInstance {
        id: device,
        source: source("D-1"),
        name: "Saved root device".into(),
        description: None,
        address: Some("1.1.1".parse().unwrap()),
        product_ref: "P-Synthetic".into(),
        program_ref: "H-Synthetic".into(),
        commissioning: CommissioningState::default(),
        visibility_calculated: false,
        com_objects: vec![object],
        binary_data: vec![],
    });
    project.devices.insert_com_object(ComObjectInstance {
        id: object,
        source: source("O-1"),
        device,
        number: 0,
        text: value(Text::Literal("Saved object".into())),
        description: Default::default(),
        dpt: value(DptRef {
            main: 1,
            sub: Some(1),
        }),
        flags: ResolvedFlags::default(),
        size: None,
        is_active: true,
        links: vec![GroupLink {
            ga,
            direction: Direction::Send,
        }],
        module_instance: None,
    });
    let addressless_id = project.ids.next_device_id().unwrap();
    let mut addressless = project.devices.get(device).unwrap().clone();
    addressless.id = addressless_id;
    addressless.source = source("D-2");
    addressless.name = "Addressless engineering device (omitted)".into();
    addressless.address = None;
    addressless.com_objects.clear();
    project.installations[0]
        .topology
        .unassigned
        .push(addressless_id);
    project.devices.insert(addressless);
    // Same normalized room/Function shape as the store's synthetic fixtures.
    let room = project.ids.next_building_part_id().unwrap();
    project.installations[0]
        .buildings
        .push(knx_core::BuildingPart {
            id: room,
            source: source("S-1"),
            name: "Synthetic room".into(),
            number: None,
            kind: knx_core::BuildingPartType::Room,
            default_line: None,
            completion: CompletionStatus::FinishedDesign,
            children: vec![],
            devices: vec![device, addressless_id],
            parent: None,
        });
    project.functions.push(knx_core::function::Function {
        id: project.ids.next_function_id().unwrap(),
        source: source("F-1"),
        location: room,
        details: knx_core::function::FunctionDetails {
            name: "Room lighting".into(),
            function_type: Some("Synthetic lighting".into()),
            ..Default::default()
        },
        members: vec![knx_core::function::FunctionMember {
            id: project.ids.next_function_member_id().unwrap(),
            source: source("FM-1"),
            group_address: ga,
            name: "Light level".into(),
            role: Some("SwitchOnOff".into()),
        }],
    });
    project
}

fn assert_success(result: &Output) {
    assert!(
        result.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
}

fn assert_refused(result: &Output, reason: &str) {
    assert_eq!(result.status.code(), Some(1));
    let error = String::from_utf8_lossy(&result.stderr);
    assert!(error.contains("monitor-export failed:"), "{error}");
    assert!(
        error
            .to_ascii_lowercase()
            .contains(&reason.to_ascii_lowercase()),
        "expected {reason}: {error}"
    );
}

#[test]
fn recovers_working_rename_dpt_and_links_without_writing_native_or_catalogue() {
    use knx_core::{CommandStack, Direction, DptRef, GroupLink, Text};
    use knx_store::project_history::{self as history, NativeSnapshot};
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("working.knxdb");
    let output = dir.path().join("monitor.zip");
    let catalogue = dir.path().join("products.sqlite");
    drop(knx_productdb::open_and_migrate(&catalogue).unwrap());
    let conn = knx_store::open_and_migrate(&input).unwrap();
    let mut snapshot = NativeSnapshot {
        project: project_with_object(),
        opaque: vec![],
        manufacturer_refs: vec![],
    };
    let generation =
        history::save_editor(&conn, &snapshot, &CommandStack::new(), None, true).unwrap();
    let device_id = snapshot.project.devices.iter().next().unwrap().id;
    let object_id = snapshot.project.devices.com_objects().next().unwrap().id;
    snapshot.project.info.name = "Recovered working project".into();
    snapshot.project.devices.get_mut(device_id).unwrap().name = "Working device".into();
    snapshot.project.installations[0].group_addresses[0].name = "Working address".into();
    snapshot.project.installations[0].group_addresses[0].declared_dpt = value(DptRef {
        main: 5,
        sub: Some(1),
    });
    let second_ga = snapshot.project.installations[0].group_addresses[1].id;
    let object = snapshot.project.devices.com_object_mut(object_id).unwrap();
    object.text = value(Text::Literal("Working {literal}".into()));
    object.dpt = value(DptRef {
        main: 5,
        sub: Some(1),
    });
    object.links = vec![GroupLink {
        ga: second_ga,
        direction: Direction::Receive,
    }];
    history::save_editor(
        &conn,
        &snapshot,
        &CommandStack::new(),
        Some(generation),
        false,
    )
    .unwrap();
    assert_eq!(
        knx_store::load_project(&conn).unwrap().info.name,
        "Synthetic monitor project"
    );
    assert_eq!(
        history::load_editor(&conn).unwrap().unwrap().working,
        snapshot
    );
    drop(conn);
    let native_before = std::fs::read(&input).unwrap();
    let products_before = std::fs::read(&catalogue).unwrap();
    let result = cli(
        dir.path(),
        &input,
        &output,
        &[
            "--product-db",
            catalogue.to_str().unwrap(),
            "--language",
            "de-DE",
        ],
    );
    // Optional private evidence retained by the task runner, including the input
    // when an independently owned builder is still unavailable.
    if let Some(destination) = std::env::var_os("KNX_MONITOR_EXPORT_TEST_EVIDENCE") {
        let destination = Path::new(&destination);
        std::fs::create_dir_all(destination).unwrap();
        std::fs::write(destination.join("stdout.txt"), &result.stdout).unwrap();
        std::fs::write(destination.join("stderr.txt"), &result.stderr).unwrap();
        std::fs::write(
            destination.join("exit.txt"),
            result.status.code().unwrap().to_string(),
        )
        .unwrap();
        for (from, name) in [(&input, "working.knxdb"), (&catalogue, "products.sqlite")] {
            std::fs::copy(from, destination.join(name)).unwrap();
        }
        if output.exists() {
            std::fs::copy(&output, destination.join("monitor.zip")).unwrap();
        }
    }
    assert_success(&result);
    let document: Value = serde_json::from_slice(&member(&output, "project.json")).unwrap();
    assert_eq!(document["info"]["name"], "Recovered working project");
    assert_eq!(document["info"]["language_code"], "de-DE");
    assert_eq!(document["devices"].as_object().unwrap().len(), 1);
    let report: Value = serde_json::from_slice(&member(&output, "export-report.json")).unwrap();
    assert_eq!(
        report["counts"]["devices"],
        serde_json::json!({"selected":2,"exported":1,"omitted":1})
    );
    assert_eq!(report["counts"]["communication_objects"]["exported"], 1);
    assert_eq!(
        report["project_sha256"],
        format!("{:x}", Sha256::digest(member(&output, "project.json")))
    );
    assert_eq!(report["counts"]["spaces"]["exported"], 1);
    assert_eq!(report["counts"]["functions"]["exported"], 1);
    assert_eq!(document["locations"]["Synthetic room"]["identifier"], "S-1");
    assert_eq!(document["functions"]["F-1"]["space_id"], "S-1");
    assert_eq!(document["devices"]["1.1.1"]["name"], "Working device");
    assert_eq!(
        document["group_addresses"]["1/1/1"]["name"],
        "Working address"
    );
    assert_eq!(
        document["group_addresses"]["1/1/1"]["dpt"],
        serde_json::json!({"main":5,"sub":1})
    );
    let object = &document["communication_objects"][format!("1.1.1/O-{}", object_id.0)];
    assert_eq!(object["text"], "Working {literal}");
    assert_eq!(object["dpts"], serde_json::json!([{"main":5,"sub":1}]));
    assert_eq!(object["group_address_links"], serde_json::json!(["1/1/2"]));
    assert_eq!(std::fs::read(&input).unwrap(), native_before);
    assert_eq!(std::fs::read(&catalogue).unwrap(), products_before);
    for path in [&input, &catalogue] {
        for suffix in ["-wal", "-shm", "-journal"] {
            assert!(!Path::new(&format!("{}{suffix}", path.display())).exists());
        }
    }
}

#[test]
fn multiple_installations_require_explicit_selection_and_pass_language() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("project.knxdb");
    let output = dir.path().join("monitor.zip");
    let mut project = project();
    let mut other = project.installations[0].clone();
    other.id = InstallationId(7);
    other.group_addresses[0].id = project.ids.next_group_address_id().unwrap();
    other.group_addresses[0].name = "Selected second installation".into();
    project.installations.push(other);
    save(&input, &project);
    let before = std::fs::read(&input).unwrap();
    assert_refused(
        &cli(dir.path(), &input, &output, &["--no-product-db"]),
        "installation",
    );
    assert!(!output.exists());
    assert_refused(
        &cli(
            dir.path(),
            &input,
            &output,
            &["--no-product-db", "--installation", "99"],
        ),
        "installation",
    );
    assert!(!output.exists());
    assert_success(&cli(
        dir.path(),
        &input,
        &output,
        &["--no-product-db", "--installation", "7", "--language", "fr"],
    ));
    let document: Value = serde_json::from_slice(&member(&output, "project.json")).unwrap();
    let report: Value = serde_json::from_slice(&member(&output, "export-report.json")).unwrap();
    assert_eq!(
        document["group_addresses"]["1/1/1"]["name"],
        "Selected second installation"
    );
    assert_eq!(document["info"]["language_code"], "fr");
    assert_eq!(report["installation"], 7);
    assert_eq!(std::fs::read(&input).unwrap(), before);
}

#[test]
fn validates_flags_before_opening_any_input_or_creating_output() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("missing.knxdb");
    let output = dir.path().join("new.zip");
    for (flags, reason) in [
        (vec!["--unknown"], "unexpected argument"),
        (vec!["extra"], "unexpected argument"),
        (vec!["--installation"], "needs a value"),
        (vec!["--installation", "--no-product-db"], "needs a value"),
        (vec!["--installation", "-1"], "--installation"),
        (vec!["--installation", "+1"], "unsigned decimal"),
        (vec!["--installation", "256"], "0..255"),
        (vec!["--installation", "4294967296"], "0..255"),
        (
            vec!["--installation", "0", "--installation", "1"],
            "duplicate --installation",
        ),
        (vec!["--language"], "needs a value"),
        (vec!["--language", ""], "nonempty"),
        (vec!["--language", " en "], "whitespace"),
        (
            vec!["--language", "en", "--language", "de"],
            "duplicate --language",
        ),
        (vec!["--product-db"], "needs a value"),
        (
            vec!["--product-db", "one", "--product-db", "two"],
            "duplicate --product-db",
        ),
        (
            vec!["--no-product-db", "--no-product-db"],
            "duplicate --no-product-db",
        ),
        (
            vec!["--product-db", "one", "--no-product-db"],
            "mutually exclusive",
        ),
    ] {
        assert_refused(&cli(dir.path(), &input, &output, &flags), reason);
        assert!(!input.exists());
        assert!(!output.exists());
    }
    for args in [
        vec!["monitor-export"],
        vec!["monitor-export", "input.knxdb"],
    ] {
        let result = Command::new(env!("CARGO_BIN_EXE_knx"))
            .args(args)
            .current_dir(dir.path())
            .output()
            .unwrap();
        assert_refused(&result, "knx monitor-export");
    }
}

#[test]
fn missing_catalogue_is_allowed_but_corrupt_or_unreadable_catalogue_is_not_swallowed() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("project.knxdb");
    let output = dir.path().join("monitor.zip");
    let products = dir.path().join("products.sqlite");
    save(&input, &project());
    assert_success(&cli(dir.path(), &input, &output, &[]));
    assert!(!dir.path().join("home/data/knx/products.sqlite").exists());
    std::fs::remove_file(&output).unwrap();
    assert_success(&cli(
        dir.path(),
        &input,
        &output,
        &["--product-db", products.to_str().unwrap()],
    ));
    assert!(!products.exists());
    std::fs::remove_file(&output).unwrap();
    let corrupt = b"not a catalogue";
    std::fs::write(&products, corrupt).unwrap();
    assert_refused(
        &cli(
            dir.path(),
            &input,
            &output,
            &["--product-db", products.to_str().unwrap()],
        ),
        "product database",
    );
    assert_eq!(std::fs::read(&products).unwrap(), corrupt);
    assert!(!output.exists());
    let bad_parent = products.join("unreadable.sqlite");
    assert_refused(
        &cli(
            dir.path(),
            &input,
            &output,
            &["--product-db", bad_parent.to_str().unwrap()],
        ),
        "product database",
    );
    assert!(!output.exists());
    let default_products = dir.path().join("home/data/knx/products.sqlite");
    std::fs::create_dir_all(default_products.parent().unwrap()).unwrap();
    std::fs::write(&default_products, corrupt).unwrap();
    assert_refused(&cli(dir.path(), &input, &output, &[]), "product database");
    assert_success(&cli(dir.path(), &input, &output, &["--no-product-db"]));
    assert_eq!(std::fs::read(&default_products).unwrap(), corrupt);
    assert_eq!(std::fs::read(&products).unwrap(), corrupt);
}

#[test]
fn refuses_existing_destination_and_input_catalogue_aliases_without_changes() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("project.knxdb");
    let output = dir.path().join("existing.zip");
    let products = dir.path().join("products.sqlite");
    save(&input, &project());
    drop(knx_productdb::open_and_migrate(&products).unwrap());
    let native_before = std::fs::read(&input).unwrap();
    let products_before = std::fs::read(&products).unwrap();
    std::fs::write(&output, b"original destination").unwrap();
    assert_refused(
        &cli(dir.path(), &input, &output, &["--no-product-db"]),
        "already exists",
    );
    assert_eq!(std::fs::read(&output).unwrap(), b"original destination");
    assert_refused(
        &cli(dir.path(), &input, &input, &["--no-product-db"]),
        "already exists",
    );
    assert_refused(
        &cli(
            dir.path(),
            &input,
            &products,
            &["--product-db", products.to_str().unwrap()],
        ),
        "already exists",
    );
    let hardlink = dir.path().join("hardlink.zip");
    std::fs::hard_link(&input, &hardlink).unwrap();
    assert_refused(
        &cli(dir.path(), &input, &hardlink, &["--no-product-db"]),
        "already exists",
    );
    assert_eq!(std::fs::read(&hardlink).unwrap(), native_before);
    let missing_products = dir.path().join("missing-products.sqlite");
    assert_refused(
        &cli(
            dir.path(),
            &input,
            &missing_products,
            &["--product-db", missing_products.to_str().unwrap()],
        ),
        "aliases",
    );
    assert!(!missing_products.exists());
    assert_eq!(std::fs::read(&input).unwrap(), native_before);
    assert_eq!(std::fs::read(&products).unwrap(), products_before);
    assert!(!std::fs::read_dir(dir.path()).unwrap().any(|entry| {
        entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".knx-monitor-export-")
    }));
}

#[cfg(unix)]
#[test]
fn refuses_live_and_dangling_output_symlinks_without_following_or_replacing() {
    use std::os::unix::fs::symlink;
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("project.knxdb");
    save(&input, &project());
    let native_before = std::fs::read(&input).unwrap();
    for (name, target) in [
        ("live.zip", input.clone()),
        ("dangling.zip", dir.path().join("missing-target")),
    ] {
        let output = dir.path().join(name);
        symlink(&target, &output).unwrap();
        assert_refused(
            &cli(dir.path(), &input, &output, &["--no-product-db"]),
            "already exists",
        );
        assert_eq!(std::fs::read_link(&output).unwrap(), target);
        if name == "dangling.zip" {
            assert!(!target.exists());
        }
    }
    assert_eq!(std::fs::read(&input).unwrap(), native_before);
}

#[test]
fn native_open_refusals_publish_nothing_and_preserve_inputs() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.knxdb");
    let output = dir.path().join("new.zip");
    assert_refused(
        &cli(dir.path(), &input, &output, &["--no-product-db"]),
        "native project",
    );
    assert!(!input.exists());
    assert!(!output.exists());
    for bytes in [vec![], b"not a database".to_vec()] {
        std::fs::write(&input, &bytes).unwrap();
        assert_refused(
            &cli(dir.path(), &input, &output, &["--no-product-db"]),
            "native project",
        );
        assert_eq!(std::fs::read(&input).unwrap(), bytes);
        assert!(!output.exists());
    }
    std::fs::remove_file(&input).unwrap();
    save(&input, &project());
    let conn = knx_store::Connection::open(&input).unwrap();
    conn.pragma_update(None, "user_version", 9999).unwrap();
    drop(conn);
    let before = std::fs::read(&input).unwrap();
    assert_refused(
        &cli(dir.path(), &input, &output, &["--no-product-db"]),
        "native project",
    );
    assert_eq!(std::fs::read(&input).unwrap(), before);
    assert!(!output.exists());
}

#[cfg(unix)]
fn dangling_catalogue_alias_is_refused(use_default: bool) {
    use std::os::unix::fs::symlink;
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("project.knxdb");
    let output = dir.path().join("new.zip");
    let catalogue = if use_default {
        dir.path().join("home/data/knx/products.sqlite")
    } else {
        dir.path().join("products.sqlite")
    };
    std::fs::create_dir_all(catalogue.parent().unwrap()).unwrap();
    save(&input, &project());
    symlink(&output, &catalogue).unwrap();
    let native_before = std::fs::read(&input).unwrap();
    let flags = if use_default {
        vec![]
    } else {
        vec!["--product-db", catalogue.to_str().unwrap()]
    };
    let result = cli(dir.path(), &input, &output, &flags);
    assert_refused(&result, "product database symlink target");
    assert!(
        !output.exists(),
        "must not publish at dangling catalogue target"
    );
    assert_eq!(std::fs::read_link(&catalogue).unwrap(), output);
    assert!(std::fs::read(&input).unwrap() == native_before);
    for suffix in ["-wal", "-shm", "-journal"] {
        assert!(!Path::new(&format!("{}{suffix}", input.display())).exists());
    }
    assert!(!std::fs::read_dir(dir.path()).unwrap().any(|entry| {
        entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".knx-monitor-export-")
    }));
}

#[cfg(unix)]
#[test]
fn refuses_dangling_explicit_catalogue_symlink_alias_without_changes() {
    dangling_catalogue_alias_is_refused(false);
}

#[cfg(unix)]
#[test]
fn refuses_dangling_default_catalogue_symlink_alias_without_changes() {
    dangling_catalogue_alias_is_refused(true);
}

#[test]
fn corrupt_working_history_is_refused_instead_of_falling_back_to_root() {
    use knx_store::project_history::{self as history, NativeSnapshot};
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("history.knxdb");
    let output = dir.path().join("new.zip");
    let conn = knx_store::open_and_migrate(&input).unwrap();
    let snapshot = NativeSnapshot {
        project: project(),
        opaque: vec![],
        manufacturer_refs: vec![],
    };
    history::save_editor(&conn, &snapshot, &knx_core::CommandStack::new(), None, true).unwrap();
    conn.execute(
        "UPDATE project_history_state SET working_hash = ?1",
        ["0".repeat(64)],
    )
    .unwrap();
    drop(conn);
    let before = std::fs::read(&input).unwrap();
    assert_refused(
        &cli(dir.path(), &input, &output, &["--no-product-db"]),
        "working snapshot",
    );
    assert_eq!(std::fs::read(&input).unwrap(), before);
    assert!(!output.exists());
}
