//! `knx devconf` through the real binary: exit codes, reports and atomic export files.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/knx-devconf/tests/fixtures")
        .join(name)
}

fn knx(args: &[&str], home: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_knx"))
        .arg("devconf")
        .args(args)
        .env("XDG_DATA_HOME", home)
        .env("XDG_CONFIG_HOME", home)
        .env("HOME", home)
        .output()
        .expect("run the real CLI")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

struct Case {
    dir: tempfile::TempDir,
    module: String,
    valid: String,
}

fn case() -> Case {
    let dir = tempfile::tempdir().unwrap();
    Case {
        module: fixture("demo-panel.devconf.json")
            .to_string_lossy()
            .into_owned(),
        valid: fixture("demo-panel.valid.json")
            .to_string_lossy()
            .into_owned(),
        dir,
    }
}

impl Case {
    fn write(&self, name: &str, bytes: &[u8]) -> String {
        let path = self.dir.path().join(name);
        std::fs::write(&path, bytes).unwrap();
        path.to_string_lossy().into_owned()
    }

    fn edited(&self, name: &str, edit: impl FnOnce(&mut serde_json::Value)) -> String {
        let mut doc: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&self.valid).unwrap()).unwrap();
        edit(&mut doc);
        self.write(name, &serde_json::to_vec(&doc).unwrap())
    }
}

#[test]
fn check_reports_identity_and_shape() {
    let c = case();
    let out = knx(&["check", &c.module], c.dir.path());
    assert!(out.status.success(), "{}", text(&out.stderr));
    let stdout = text(&out.stdout);
    assert!(
        stdout.contains("module example.demo-panel 1.0.0"),
        "{stdout}"
    );
    assert!(
        stdout.contains("table buttons: 4..4 rows, 2 members"),
        "{stdout}"
    );
    assert!(
        stdout.contains("export image: Panel image (.bin, at most 512 bytes)"),
        "{stdout}"
    );
    let out = knx(&["check", &c.module, "--json"], c.dir.path());
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["module"]["sha256"].as_str().unwrap().len(), 64);
    assert_eq!(report["rules"], 6);
}

#[test]
fn validate_exit_codes_and_findings() {
    let c = case();
    let out = knx(&["validate", &c.module, &c.valid], c.dir.path());
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert!(text(&out.stdout).contains("valid for example.demo-panel 1.0.0"));
    let bad = c.edited("bad.json", |d| d["buttons"][2]["partner"] = 9.into());
    let out = knx(&["validate", &c.module, &bad], c.dir.path());
    assert_eq!(out.status.code(), Some(1));
    assert!(
        text(&out.stdout).contains(
            "/buttons/2/partner: Partners are reciprocal neighbours (0/1, 2/3). [partner-pair]"
        ),
        "{}",
        text(&out.stdout)
    );
    let out = knx(&["validate", &c.module, &bad, "--json"], c.dir.path());
    assert_eq!(out.status.code(), Some(1));
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["valid"], false);
    assert_eq!(report["findings"][0]["kind"], "rule");
    assert_eq!(report["findings"][0]["path"], "/buttons/2/partner");
}

#[test]
fn export_writes_exactly_the_library_bytes() {
    let c = case();
    let output = c.dir.path().join("panel.bin");
    let out = knx(
        &[
            "export",
            &c.module,
            &c.valid,
            "--export",
            "image",
            "--output",
            output.to_str().unwrap(),
        ],
        c.dir.path(),
    );
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    let module = knx_devconf::Module::load(&std::fs::read(&c.module).unwrap()).unwrap();
    let expected = module
        .export(&std::fs::read(&c.valid).unwrap(), "image")
        .unwrap();
    assert_eq!(std::fs::read(&output).unwrap(), expected);
    assert_eq!(
        std::fs::read_dir(c.dir.path()).unwrap().count(),
        1,
        "no temporary file left"
    );
    // The output gets the mode of any new file under the inherited umask, not a temp-file 0600.
    let reference = c.write("reference", b"");
    use std::os::unix::fs::PermissionsExt;
    let mode = |p: &Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode(&output), mode(Path::new(&reference)));
}

#[test]
fn refused_exports_write_nothing_and_keep_an_existing_file() {
    let c = case();
    let output = c.write("panel.bin", b"previous");
    for (name, edit) in [
        (
            "long.json",
            Box::new(|d: &mut serde_json::Value| d["name"] = "Entrance hall".into())
                as Box<dyn Fn(&mut serde_json::Value)>,
        ),
        (
            "invalid.json",
            Box::new(|d: &mut serde_json::Value| d["offset"] = 500.into()),
        ),
    ] {
        let doc = c.edited(name, edit);
        let out = knx(
            &[
                "export", &c.module, &doc, "--export", "image", "--output", &output,
            ],
            c.dir.path(),
        );
        assert_eq!(out.status.code(), Some(1), "{name}: {}", text(&out.stderr));
        assert!(text(&out.stderr).contains("nothing written"), "{name}");
        assert_eq!(std::fs::read(&output).unwrap(), b"previous", "{name}");
    }
}

#[test]
fn unusable_input_exits_with_two() {
    let c = case();
    let broken = c.write(
        "broken.devconf.json",
        br#"{"format":"knxbench-device-configuration-module","format_version":2}"#,
    );
    let missing = c
        .dir
        .path()
        .join("missing.json")
        .to_string_lossy()
        .into_owned();
    let out_path = c.dir.path().join("x.bin").to_string_lossy().into_owned();
    for args in [
        vec!["check", missing.as_str()],
        vec!["check", broken.as_str()],
        vec!["validate", c.module.as_str(), missing.as_str()],
        vec![
            "export",
            c.module.as_str(),
            c.valid.as_str(),
            "--export",
            "nope",
            "--output",
            out_path.as_str(),
        ],
        vec![
            "export",
            c.module.as_str(),
            c.valid.as_str(),
            "--export",
            "image",
        ],
        vec!["frobnicate"],
    ] {
        let out = knx(&args, c.dir.path());
        assert_eq!(
            out.status.code(),
            Some(2),
            "{args:?}: {}",
            text(&out.stderr)
        );
    }
    assert!(!Path::new(&out_path).exists());
}

#[test]
fn long_output_names_and_unwritable_targets() {
    let c = case();
    let long = c.dir.path().join(format!("{}.bin", "p".repeat(240)));
    let out = knx(
        &[
            "export",
            &c.module,
            &c.valid,
            "--export",
            "image",
            "--output",
            long.to_str().unwrap(),
        ],
        c.dir.path(),
    );
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert_eq!(std::fs::read(&long).unwrap().len(), 65);
    // A directory as target makes the rename fail: the temporary file is removed and the
    // directory is untouched.
    let target = c.dir.path().join("target-dir");
    std::fs::create_dir(&target).unwrap();
    std::fs::write(target.join("keep"), b"x").unwrap();
    let out = knx(
        &[
            "export",
            &c.module,
            &c.valid,
            "--export",
            "image",
            "--output",
            target.to_str().unwrap(),
        ],
        c.dir.path(),
    );
    assert_eq!(out.status.code(), Some(2), "{}", text(&out.stdout));
    assert_eq!(std::fs::read(target.join("keep")).unwrap(), b"x");
    let mut names: Vec<String> = std::fs::read_dir(c.dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(
        names,
        [
            long.file_name().unwrap().to_string_lossy().into_owned(),
            "target-dir".to_owned()
        ]
    );
}
