//! `knx devconf export-stored` through the real binary against projects saved by the store API.

use knx_core::devconf::DeviceConfigProfile;
use knx_core::ids::SourceRef;
use knx_core::{
    Area, AreaId, Command, DeviceId, DeviceInstance, Installation, InstallationId, Language, Line,
    LineId, Project,
};
use std::path::{Path, PathBuf};
use std::process::{Command as Process, Output};

/// Reusable fixture API (fictional data only): a saved project holding device 1, optionally with
/// one configuration profile `(module id, version, sha256, document bytes)` written through
/// `Command::SetDeviceConfigProfile` and `knx_store::save_project`.
pub fn save_project_with_profile(path: &Path, profile: Option<(&str, &str, &str, &[u8])>) {
    let mut project = Project::new(Language("en".into()));
    let source = SourceRef {
        path: "synthetic".into(),
        ets_id: "D-1".into(),
    };
    project.installations.push(Installation {
        id: InstallationId(1),
        name: "Installation".into(),
        default_line: None,
        multicast_address: None,
        completion: knx_core::CompletionStatus::Undefined,
        topology: knx_core::Topology {
            areas: vec![Area {
                id: AreaId(1),
                source: source.clone(),
                name: "Area".into(),
                address: 1,
                completion: knx_core::CompletionStatus::Undefined,
                lines: vec![LineId(1)],
            }],
            lines: vec![Line {
                id: LineId(1),
                source: source.clone(),
                name: "Line".into(),
                address: 1,
                medium_ref: "MT-1".into(),
                domain_address: None,
                domain_address_is_checked: None,
                ip_routing_multicast_address: None,
                multicast_ttl: None,
                completion: knx_core::CompletionStatus::Undefined,
                devices: vec![DeviceId(1)],
            }],
            unassigned: vec![],
        },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses: vec![],
        parameters: vec![],
    });
    project.devices.insert(DeviceInstance {
        id: DeviceId(1),
        source,
        name: "Device 1".into(),
        description: None,
        address: None,
        product_ref: "P-1".into(),
        program_ref: "H-1".into(),
        commissioning: Default::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
    project.ids.raise_to(&knx_core::IdAllocators::from_counts(
        2, 2, 2, 1, 1, 1, 1, 1, 1,
    ));
    if let Some((id, version, sha, document)) = profile {
        let profile =
            DeviceConfigProfile::new(DeviceId(1), id, version, sha, document.to_vec()).unwrap();
        Command::SetDeviceConfigProfile { profile }
            .apply(&mut project)
            .unwrap();
    }
    let conn = knx_store::open_and_migrate(path).unwrap();
    knx_store::save_project(&conn, &project).unwrap();
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/knx-devconf/tests/fixtures")
        .join(name)
}

struct Case {
    dir: tempfile::TempDir,
    module: PathBuf,
    module_bytes: Vec<u8>,
    id: String,
    version: String,
    sha: String,
    valid: Vec<u8>,
    sentinel: PathBuf,
}

fn case() -> Case {
    let dir = tempfile::tempdir().unwrap();
    let module_bytes = std::fs::read(fixture("demo-panel.devconf.json")).unwrap();
    let module = dir.path().join("module.devconf.json");
    std::fs::write(&module, &module_bytes).unwrap();
    let parsed = knx_devconf::Module::load(&module_bytes).unwrap();
    let sentinel = dir.path().join("out.bin");
    std::fs::write(&sentinel, b"sentinel").unwrap();
    Case {
        id: parsed.id().to_owned(),
        version: parsed.version().to_owned(),
        sha: parsed.sha256().to_owned(),
        valid: std::fs::read(fixture("demo-panel.valid.json")).unwrap(),
        module,
        module_bytes,
        sentinel,
        dir,
    }
}

impl Case {
    fn project(&self, name: &str, id: &str, version: &str, sha: &str, doc: &[u8]) -> PathBuf {
        let path = self.dir.path().join(name);
        save_project_with_profile(&path, Some((id, version, sha, doc)));
        path
    }

    fn good_project(&self) -> PathBuf {
        self.project("p.knxdb", &self.id, &self.version, &self.sha, &self.valid)
    }

    fn run(&self, project: &Path, device: &str, output: &Path) -> Output {
        knx(&[
            "export-stored",
            project.to_str().unwrap(),
            "--device",
            device,
            "--module",
            self.module.to_str().unwrap(),
            "--export",
            "image",
            "--output",
            output.to_str().unwrap(),
        ])
    }

    fn expected(&self, doc: &[u8]) -> Vec<u8> {
        knx_devconf::Module::load(&self.module_bytes)
            .unwrap()
            .export(doc, "image")
            .unwrap()
    }

    fn refused(&self, project: &Path, device: &str, code: i32, why: &str) {
        let before = std::fs::read(project).unwrap();
        let out = self.run(project, device, &self.sentinel);
        assert_eq!(
            out.status.code(),
            Some(code),
            "{why}: {}",
            text(&out.stderr)
        );
        assert_eq!(std::fs::read(&self.sentinel).unwrap(), b"sentinel", "{why}");
        assert_eq!(std::fs::read(project).unwrap(), before, "{why}");
        assert_eq!(
            std::fs::read_dir(self.dir.path())
                .unwrap()
                .filter(|e| e
                    .as_ref()
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .ends_with(".tmp"))
                .count(),
            0,
            "{why}: no temporary file"
        );
    }
}

fn knx(args: &[&str]) -> Output {
    let home = tempfile::tempdir().unwrap();
    Process::new(env!("CARGO_BIN_EXE_knx"))
        .arg("devconf")
        .args(args)
        .env("XDG_DATA_HOME", home.path())
        .env("XDG_CONFIG_HOME", home.path())
        .env("HOME", home.path())
        .output()
        .expect("run the real CLI")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn stored_document_exports_exact_bytes_and_leaves_the_project_untouched() {
    let c = case();
    let project = c.good_project();
    let before = std::fs::read(&project).unwrap();
    let output = c.dir.path().join("fresh.bin");
    let out = c.run(&project, "1", &output);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert_eq!(std::fs::read(&output).unwrap(), c.expected(&c.valid));
    assert_eq!(std::fs::read(&project).unwrap(), before);
    // The stored document is itself exactly the bytes that were saved.
    let conn = knx_store::open_existing_read_only(&project).unwrap().conn;
    let loaded = knx_store::load_project(&conn).unwrap();
    assert_eq!(
        loaded.devices.config_profile(DeviceId(1)).unwrap().document,
        c.valid
    );
}

#[test]
fn a_replaced_output_is_the_export_and_whitespace_or_order_is_not_corruption() {
    let c = case();
    let mut doc: serde_json::Value = serde_json::from_slice(&c.valid).unwrap();
    doc["name"] = "Hall".into();
    // Same value, different field order and layout than a canonical dump.
    let mut padded = b"  \n".to_vec();
    padded.extend_from_slice(&serde_json::to_vec_pretty(&doc).unwrap());
    padded.extend_from_slice(b"\n\n");
    let project = c.project("p.knxdb", &c.id, &c.version, &c.sha, &padded);
    let out = c.run(&project, "1", &c.sentinel);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert_eq!(std::fs::read(&c.sentinel).unwrap(), c.expected(&padded));
}

#[test]
fn module_identity_mismatches_are_refused_one_field_at_a_time() {
    let c = case();
    let other_sha = "b".repeat(64);
    let cases = [
        (
            "id",
            "example.other-panel",
            c.version.as_str(),
            c.sha.as_str(),
        ),
        ("version", c.id.as_str(), "9.9.9", c.sha.as_str()),
        (
            "sha256",
            c.id.as_str(),
            c.version.as_str(),
            other_sha.as_str(),
        ),
    ];
    for (which, id, version, sha) in cases {
        let project = c.project(&format!("{which}.knxdb"), id, version, sha, &c.valid);
        c.refused(&project, "1", 1, which);
        let stderr = text(&c.run(&project, "1", &c.sentinel).stderr);
        assert!(stderr.contains("nothing written"), "{which}: {stderr}");
    }
}

#[test]
fn missing_device_or_profile_is_unusable_input() {
    let c = case();
    let project = c.good_project();
    c.refused(&project, "2", 2, "no such device");
    let bare = c.dir.path().join("bare.knxdb");
    save_project_with_profile(&bare, None);
    c.refused(&bare, "1", 2, "device without profile");
    // Positive control: the same project with a profile exports.
    let out = c.run(&project, "1", &c.dir.path().join("ok.bin"));
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
}

#[test]
fn invalid_stored_documents_are_refused_before_any_write() {
    let c = case();
    let mut bad: serde_json::Value = serde_json::from_slice(&c.valid).unwrap();
    bad["buttons"][2]["partner"] = 9.into();
    let rule_invalid = serde_json::to_vec(&bad).unwrap();
    for (name, doc) in [
        ("malformed", b"{ not json".to_vec()),
        ("rule", rule_invalid),
        ("duplicate-member", br#"{"version":1,"version":1}"#.to_vec()),
    ] {
        let project = c.project(&format!("{name}.knxdb"), &c.id, &c.version, &c.sha, &doc);
        c.refused(&project, "1", 1, name);
    }
}

#[test]
fn corrupt_or_foreign_projects_are_unusable_input() {
    let c = case();
    let good = std::fs::read(c.good_project()).unwrap();
    let truncated = c.dir.path().join("truncated.knxdb");
    std::fs::write(&truncated, &good[..good.len() / 3]).unwrap();
    let garbage = c.dir.path().join("garbage.knxdb");
    std::fs::write(&garbage, b"this is not a database at all, just text").unwrap();
    let empty = c.dir.path().join("empty.knxdb");
    std::fs::write(&empty, b"").unwrap();
    for (name, path) in [
        ("truncated", truncated),
        ("garbage", garbage),
        ("empty", empty),
    ] {
        c.refused(&path, "1", 2, name);
    }
    let missing = c.dir.path().join("missing.knxdb");
    let out = c.run(&missing, "1", &c.sentinel);
    assert_eq!(out.status.code(), Some(2));
    assert!(!missing.exists(), "a missing project is not created");
}

#[test]
fn malformed_command_lines_are_refused_without_writing() {
    let c = case();
    let project = c.good_project();
    let p = project.to_str().unwrap();
    let m = c.module.to_str().unwrap();
    let o = c.sentinel.to_str().unwrap();
    let base = [
        "export-stored",
        p,
        "--device",
        "1",
        "--module",
        m,
        "--export",
        "image",
        "--output",
        o,
    ];
    for (why, extra) in [
        ("duplicate --device", vec!["--device", "1"]),
        ("duplicate --module", vec!["--module", m]),
        ("duplicate --export", vec!["--export", "image"]),
        ("duplicate --output", vec!["--output", o]),
        ("unknown flag", vec!["--force"]),
        ("second positional", vec![m]),
    ] {
        let args: Vec<&str> = base.iter().copied().chain(extra).collect();
        assert_eq!(knx(&args).status.code(), Some(2), "{why}");
    }
    for drop in [1, 2, 4, 6, 8] {
        // Drop a positional or one flag with its value.
        let mut args = base.to_vec();
        let n = if drop == 1 { 1 } else { 2 };
        args.drain(drop..drop + n);
        assert_eq!(knx(&args).status.code(), Some(2), "missing at {drop}");
    }
    for device in ["0", "x", "-1", "4294967296", ""] {
        let mut args = base.to_vec();
        args[3] = device;
        assert_eq!(knx(&args).status.code(), Some(2), "device {device:?}");
    }
    let mut args = base.to_vec();
    args[3] = "--module";
    assert_eq!(knx(&args).status.code(), Some(2), "flag as value");
    assert_eq!(std::fs::read(&c.sentinel).unwrap(), b"sentinel");
    // Positive control: the unmodified base line works.
    assert_eq!(knx(&base).status.code(), Some(0));
}

#[test]
fn unknown_export_is_unusable_input_and_writes_nothing() {
    let c = case();
    let project = c.good_project();
    let out = knx(&[
        "export-stored",
        project.to_str().unwrap(),
        "--device",
        "1",
        "--module",
        c.module.to_str().unwrap(),
        "--export",
        "nope",
        "--output",
        c.sentinel.to_str().unwrap(),
    ]);
    assert_eq!(out.status.code(), Some(2), "{}", text(&out.stderr));
    assert_eq!(std::fs::read(&c.sentinel).unwrap(), b"sentinel");
}

#[test]
fn outputs_aliasing_the_project_or_module_are_refused_before_any_write() {
    use std::os::unix::fs::symlink;
    let c = case();
    let project = c.good_project();
    let project_bytes = std::fs::read(&project).unwrap();
    let dir = c.dir.path();
    std::fs::create_dir(dir.join("sub")).unwrap();
    let soft = dir.join("soft.bin");
    symlink(&project, &soft).unwrap();
    let hard = dir.join("hard.bin");
    std::fs::hard_link(&c.module, &hard).unwrap();
    let module_soft = dir.join("module-soft.bin");
    symlink(&c.module, &module_soft).unwrap();
    for (name, output) in [
        ("project itself", project.clone()),
        ("module itself", c.module.clone()),
        ("symlink to project", soft),
        ("hard link to module", hard),
        ("symlink to module", module_soft),
        (
            "project through a parent-directory path",
            dir.join("sub/../p.knxdb"),
        ),
    ] {
        let out = c.run(&project, "1", &output);
        assert_eq!(out.status.code(), Some(2), "{name}: {}", text(&out.stderr));
        assert_eq!(std::fs::read(&project).unwrap(), project_bytes, "{name}");
        assert_eq!(std::fs::read(&c.module).unwrap(), c.module_bytes, "{name}");
    }
    assert!(std::fs::symlink_metadata(dir.join("soft.bin"))
        .unwrap()
        .file_type()
        .is_symlink());
    // Positive control: a distinct output with the same inputs succeeds.
    let out = c.run(&project, "1", &dir.join("distinct.bin"));
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
}
