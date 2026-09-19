//! Validates the AppImage package contract before release publication.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use cargo_metadata::MetadataCommand;
use serde_json::Value;
use yaml_rust2::{Yaml, YamlLoader};

#[derive(Debug, PartialEq, Eq)]
pub struct VerifiedAppImage {
    pub version: String,
    pub path: PathBuf,
}

pub fn verify(
    root: &Path,
    artifact_dir: &Path,
    tag: Option<&str>,
) -> Result<VerifiedAppImage, String> {
    let version = workspace_desktop_version(root)?;
    verify_config(root, &version)?;
    verify_workflow(root)?;
    let artifact = verify_artifacts(artifact_dir, &version)?;
    verify_tag(tag, &version)?;
    Ok(artifact)
}

fn workspace_desktop_version(root: &Path) -> Result<String, String> {
    let metadata = MetadataCommand::new()
        .manifest_path(root.join("Cargo.toml"))
        .exec()
        .map_err(|error| format!("cargo metadata failed: {error}"))?;
    metadata
        .packages
        .iter()
        .find(|package| package.name == "knx-desktop")
        .map(|package| package.version.to_string())
        .ok_or_else(|| "workspace does not contain knx-desktop".to_string())
}

fn verify_config(root: &Path, version: &str) -> Result<(), String> {
    let tauri_config = read_json(&root.join("apps/knx-desktop/src-tauri/tauri.conf.json"))?;
    let bundle = tauri_config
        .get("bundle")
        .and_then(Value::as_object)
        .ok_or_else(|| "tauri.conf.json has no bundle object".to_string())?;

    if bundle.get("active").and_then(Value::as_bool) != Some(true) {
        return Err("AppImage bundling must be active".to_string());
    }
    if bundle.get("targets") != Some(&Value::Array(vec![Value::String("appimage".to_string())])) {
        return Err("AppImage bundle targets must be exactly [\"appimage\"]".to_string());
    }

    let resources = bundle
        .get("resources")
        .and_then(Value::as_object)
        .ok_or_else(|| "AppImage bundle resources must be an object".to_string())?;
    if resources.get("../../knx-web/dist").and_then(Value::as_str) != Some("frontend") {
        return Err("AppImage bundle must include ../../knx-web/dist as frontend".to_string());
    }
    if resources.get("../../../LICENSE").and_then(Value::as_str) != Some("LICENSE") {
        return Err("AppImage bundle must include ../../../LICENSE as LICENSE".to_string());
    }
    if !root.join("apps/knx-web/dist").is_dir() {
        return Err("frontend distribution directory is missing".to_string());
    }
    if !root.join("LICENSE").is_file() {
        return Err("LICENSE file is missing".to_string());
    }

    let npm_package = read_json(&root.join("apps/knx-web/package.json"))?;
    let npm_version = npm_package
        .get("version")
        .and_then(Value::as_str)
        .ok_or_else(|| "apps/knx-web/package.json has no version string".to_string())?;
    if npm_version != version {
        return Err(format!(
            "knx-desktop Cargo version {version} does not match knx-web npm version {npm_version}"
        ));
    }

    Ok(())
}

fn read_json(path: &Path) -> Result<Value, String> {
    let source = fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    serde_json::from_str(&source)
        .map_err(|error| format!("cannot parse {}: {error}", path.display()))
}

fn verify_workflow(root: &Path) -> Result<(), String> {
    let path = root.join(".github/workflows/linux-appimage.yml");
    let source = fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let documents = YamlLoader::load_from_str(&source)
        .map_err(|error| format!("cannot parse {}: {error}", path.display()))?;
    let [workflow] = documents.as_slice() else {
        return Err(format!(
            "{} must contain exactly one YAML document",
            path.display()
        ));
    };
    let Yaml::Array(steps) = &workflow["jobs"]["build"]["steps"] else {
        return Err("Linux AppImage workflow must set NO_STRIP to 1".to_string());
    };
    if !steps.iter().any(|step| {
        step["name"].as_str() == Some("Build AppImage")
            && step["env"]["NO_STRIP"].as_str() == Some("1")
    }) {
        return Err("Linux AppImage workflow must set NO_STRIP to 1".to_string());
    }

    let smoke_contract = concat!(
        "status=$?\n",
        "set -e\n",
        "test \"$status\" -eq 124 || { cat appimage-startup.log; exit 1; }"
    );
    if !steps.iter().any(|step| {
        step["name"].as_str() == Some("Smoke test AppImage startup")
            && step["run"]
                .as_str()
                .is_some_and(|run| run.trim_end().ends_with(smoke_contract))
    }) {
        return Err("Linux AppImage smoke test must accept only timeout status 124".to_string());
    }

    let repository_context_error =
        "Linux AppImage release step must set GH_REPO to ${{ github.repository }}";
    let Yaml::Array(release_steps) = &workflow["jobs"]["release"]["steps"] else {
        return Err(repository_context_error.to_string());
    };
    if !release_steps.iter().any(|step| {
        step["name"].as_str() == Some("Create or update release")
            && step["env"]["GH_REPO"].as_str() == Some("${{ github.repository }}")
    }) {
        return Err(repository_context_error.to_string());
    }

    Ok(())
}

fn verify_artifacts(dir: &Path, version: &str) -> Result<VerifiedAppImage, String> {
    let entries =
        fs::read_dir(dir).map_err(|error| format!("cannot read {}: {error}", dir.display()))?;
    let artifacts: Vec<PathBuf> = entries
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("cannot read {}: {error}", dir.display()))?
        .into_iter()
        .filter(|path| {
            path.extension().and_then(|extension| extension.to_str()) == Some("AppImage")
        })
        .collect();

    let [path] = artifacts.as_slice() else {
        return Err(format!(
            "expected exactly one AppImage artifact in {}, found {}",
            dir.display(),
            artifacts.len()
        ));
    };
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if !name.ends_with(&format!("_{version}_amd64.AppImage")) {
        return Err(format!(
            "AppImage artifact {name} must use version {version} and x86_64 architecture"
        ));
    }
    let metadata = fs::metadata(path)
        .map_err(|error| format!("cannot inspect {}: {error}", path.display()))?;
    if !metadata.is_file() {
        return Err(format!(
            "AppImage artifact {} is not a regular file",
            path.display()
        ));
    }
    if metadata.len() == 0 {
        return Err(format!("AppImage artifact {} is empty", path.display()));
    }
    if metadata.permissions().mode() & 0o111 == 0 {
        return Err(format!(
            "AppImage artifact {} is not executable",
            path.display()
        ));
    }

    Ok(VerifiedAppImage {
        version: version.to_string(),
        path: path.clone(),
    })
}

fn verify_tag(tag: Option<&str>, version: &str) -> Result<(), String> {
    let Some(tag) = tag else {
        return Ok(());
    };
    if tag.strip_prefix('v') == Some(version) {
        return Ok(());
    }
    Err(format!("tag {tag} must equal v{version}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;

    fn write_executable(path: &Path, contents: &[u8]) {
        fs::write(path, contents).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }

    #[test]
    fn one_executable_versioned_appimage_is_accepted() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("KNXBench_0.1.0-alpha.1_amd64.AppImage");
        write_executable(&path, b"appimage");
        assert_eq!(
            verify_artifacts(dir.path(), "0.1.0-alpha.1").unwrap().path,
            path
        );
    }

    #[test]
    fn missing_appimage_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        assert!(verify_artifacts(dir.path(), "0.1.0-alpha.1").is_err());
    }

    #[test]
    fn multiple_appimages_are_rejected() {
        let dir = tempfile::tempdir().unwrap();
        write_executable(
            &dir.path().join("KNXBench_0.1.0-alpha.1_amd64.AppImage"),
            b"one",
        );
        write_executable(
            &dir.path().join("KNXBench_0.1.0-alpha.1_x86_64.AppImage"),
            b"two",
        );
        assert!(verify_artifacts(dir.path(), "0.1.0-alpha.1").is_err());
    }

    #[test]
    fn empty_appimage_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("KNXBench_0.1.0-alpha.1_amd64.AppImage");
        write_executable(&path, b"");
        assert!(verify_artifacts(dir.path(), "0.1.0-alpha.1").is_err());
    }

    #[test]
    fn non_executable_appimage_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("KNXBench_0.1.0-alpha.1_amd64.AppImage");
        fs::write(path, b"appimage").unwrap();
        assert!(verify_artifacts(dir.path(), "0.1.0-alpha.1").is_err());
    }

    #[test]
    fn directory_named_as_appimage_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("KNXBench_0.1.0-alpha.1_amd64.AppImage");
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();

        assert_eq!(
            verify_artifacts(dir.path(), "0.1.0-alpha.1").unwrap_err(),
            format!("AppImage artifact {} is not a regular file", path.display())
        );
    }

    #[test]
    fn wrong_version_appimage_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        write_executable(
            &dir.path().join("KNXBench_0.1.0-alpha.2_amd64.AppImage"),
            b"appimage",
        );
        assert!(verify_artifacts(dir.path(), "0.1.0-alpha.1").is_err());
    }

    #[test]
    fn non_x86_64_appimage_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        write_executable(
            &dir.path().join("KNXBench_0.1.0-alpha.1_aarch64.AppImage"),
            b"appimage",
        );
        assert!(verify_artifacts(dir.path(), "0.1.0-alpha.1").is_err());
    }

    #[test]
    fn tag_must_equal_the_version_after_its_v_prefix() {
        assert!(verify_tag(Some("v0.1.0-alpha.1"), "0.1.0-alpha.1").is_ok());
        assert!(verify_tag(Some("v0.1.0-alpha.2"), "0.1.0-alpha.1").is_err());
        assert!(verify_tag(Some("0.1.0-alpha.1"), "0.1.0-alpha.1").is_err());
    }

    #[test]
    fn release_workflow_disables_linuxdeploy_stripping() {
        let dir = tempfile::tempdir().unwrap();
        let workflow_dir = dir.path().join(".github/workflows");
        fs::create_dir_all(&workflow_dir).unwrap();
        let workflow = workflow_dir.join("linux-appimage.yml");

        fs::write(
            &workflow,
            "jobs:\n  build:\n    steps:\n      - name: Build AppImage\n        run: cargo tauri build\n",
        )
        .unwrap();
        assert_eq!(
            verify_workflow(dir.path()).unwrap_err(),
            "Linux AppImage workflow must set NO_STRIP to 1"
        );

        fs::write(
            &workflow,
            "jobs:\n  build:\n    steps:\n      - name: Build AppImage\n        run: cargo tauri build\n      - name: Other\n        env:\n          NO_STRIP: \"1\"\n",
        )
        .unwrap();
        assert_eq!(
            verify_workflow(dir.path()).unwrap_err(),
            "Linux AppImage workflow must set NO_STRIP to 1"
        );

        fs::write(
            &workflow,
            "jobs:\n  build:\n    steps:\n      - name: Build AppImage\n        env:\n          NO_STRIP: 1\n        run: cargo tauri build\n",
        )
        .unwrap();
        assert_eq!(
            verify_workflow(dir.path()).unwrap_err(),
            "Linux AppImage workflow must set NO_STRIP to 1"
        );

        fs::write(
            workflow,
            "jobs:\n  build:\n    steps:\n      - name: Build AppImage\n        env:\n          NO_STRIP: \"1\"\n        run: cargo tauri build\n      - name: Smoke test AppImage startup\n        run: |\n          status=$?\n          set -e\n          test \"$status\" -eq 124 || { cat appimage-startup.log; exit 1; }\n  release:\n    steps:\n      - name: Create or update release\n        env:\n          GH_REPO: ${{ github.repository }}\n",
        )
        .unwrap();
        assert!(verify_workflow(dir.path()).is_ok());
    }

    #[test]
    fn release_workflow_rejects_nostrip_in_shell_text() {
        let dir = tempfile::tempdir().unwrap();
        let workflow_dir = dir.path().join(".github/workflows");
        fs::create_dir_all(&workflow_dir).unwrap();
        fs::write(
            workflow_dir.join("linux-appimage.yml"),
            "jobs:\n  build:\n    steps:\n      - name: Build AppImage\n        run: |\n          NO_STRIP: \"1\"\n          cargo tauri build\n",
        )
        .unwrap();

        assert_eq!(
            verify_workflow(dir.path()).unwrap_err(),
            "Linux AppImage workflow must set NO_STRIP to 1"
        );
    }

    #[test]
    fn release_workflow_rejects_nostrip_in_later_unnamed_step() {
        let dir = tempfile::tempdir().unwrap();
        let workflow_dir = dir.path().join(".github/workflows");
        fs::create_dir_all(&workflow_dir).unwrap();
        fs::write(
            workflow_dir.join("linux-appimage.yml"),
            "jobs:\n  build:\n    steps:\n      - name: Build AppImage\n        run: cargo tauri build\n      - env:\n          NO_STRIP: \"1\"\n        run: echo later\n",
        )
        .unwrap();

        assert_eq!(
            verify_workflow(dir.path()).unwrap_err(),
            "Linux AppImage workflow must set NO_STRIP to 1"
        );
    }

    #[test]
    fn release_workflow_requires_exact_repository_context() {
        let dir = tempfile::tempdir().unwrap();
        let workflow_dir = dir.path().join(".github/workflows");
        fs::create_dir_all(&workflow_dir).unwrap();
        fs::write(
            workflow_dir.join("linux-appimage.yml"),
            "jobs:\n  build:\n    steps:\n      - name: Build AppImage\n        env:\n          NO_STRIP: \"1\"\n      - name: Smoke test AppImage startup\n        run: |\n          status=$?\n          set -e\n          test \"$status\" -eq 124 || { cat appimage-startup.log; exit 1; }\n  release:\n    steps:\n      - name: Create or update release\n        env:\n          GH_TOKEN: ${{ github.token }}\n        run: gh release view \"$GITHUB_REF_NAME\"\n",
        )
        .unwrap();

        assert_eq!(
            verify_workflow(dir.path()).unwrap_err(),
            "Linux AppImage release step must set GH_REPO to ${{ github.repository }}"
        );
    }

    #[test]
    fn release_workflow_rejects_smoke_test_that_can_exit_successfully() {
        let dir = tempfile::tempdir().unwrap();
        let workflow_dir = dir.path().join(".github/workflows");
        fs::create_dir_all(&workflow_dir).unwrap();
        fs::write(
            workflow_dir.join("linux-appimage.yml"),
            "jobs:\n  build:\n    steps:\n      - name: Build AppImage\n        env:\n          NO_STRIP: \"1\"\n      - name: Smoke test AppImage startup\n        run: |\n          status=$?\n          set -e\n          test \"$status\" -eq 124 || { cat appimage-startup.log; exit \"$status\"; }\n  release:\n    steps:\n      - name: Create or update release\n        env:\n          GH_REPO: ${{ github.repository }}\n",
        )
        .unwrap();

        assert_eq!(
            verify_workflow(dir.path()).unwrap_err(),
            "Linux AppImage smoke test must accept only timeout status 124"
        );
    }

    fn fixture_root() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("apps/knx-desktop/src-tauri")).unwrap();
        fs::create_dir_all(dir.path().join("apps/knx-web/dist")).unwrap();
        fs::write(dir.path().join("LICENSE"), "AGPL").unwrap();
        fs::write(
            dir.path().join("apps/knx-desktop/src-tauri/tauri.conf.json"),
            r#"{"bundle":{"active":true,"targets":["appimage"],"resources":{"../../knx-web/dist":"frontend","../../../LICENSE":"LICENSE"}}}"#,
        )
        .unwrap();
        fs::write(
            dir.path().join("apps/knx-desktop/src-tauri/Cargo.toml"),
            "[package]\nname = \"knx-desktop\"\nversion = \"0.1.0-alpha.1\"\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("apps/knx-web/package.json"),
            r#"{"version":"0.1.0-alpha.1"}"#,
        )
        .unwrap();
        dir
    }

    #[test]
    fn inactive_bundling_is_rejected() {
        let dir = fixture_root();
        fs::write(dir.path().join("apps/knx-desktop/src-tauri/tauri.conf.json"), r#"{"bundle":{"active":false,"targets":["appimage"],"resources":{"../../knx-web/dist":"frontend","../../../LICENSE":"LICENSE"}}}"#).unwrap();
        assert!(verify_config(dir.path(), "0.1.0-alpha.1").is_err());
    }

    #[test]
    fn non_appimage_target_is_rejected() {
        let dir = fixture_root();
        fs::write(dir.path().join("apps/knx-desktop/src-tauri/tauri.conf.json"), r#"{"bundle":{"active":true,"targets":["deb"],"resources":{"../../knx-web/dist":"frontend","../../../LICENSE":"LICENSE"}}}"#).unwrap();
        assert!(verify_config(dir.path(), "0.1.0-alpha.1").is_err());
    }

    #[test]
    fn absent_frontend_resource_is_rejected() {
        let dir = fixture_root();
        fs::write(dir.path().join("apps/knx-desktop/src-tauri/tauri.conf.json"), r#"{"bundle":{"active":true,"targets":["appimage"],"resources":{"../../../LICENSE":"LICENSE"}}}"#).unwrap();
        assert!(verify_config(dir.path(), "0.1.0-alpha.1").is_err());
    }

    #[test]
    fn absent_licence_resource_is_rejected() {
        let dir = fixture_root();
        fs::write(dir.path().join("apps/knx-desktop/src-tauri/tauri.conf.json"), r#"{"bundle":{"active":true,"targets":["appimage"],"resources":{"../../knx-web/dist":"frontend"}}}"#).unwrap();
        assert!(verify_config(dir.path(), "0.1.0-alpha.1").is_err());
    }

    #[test]
    fn absent_licence_file_is_rejected() {
        let dir = fixture_root();
        fs::remove_file(dir.path().join("LICENSE")).unwrap();
        assert!(verify_config(dir.path(), "0.1.0-alpha.1").is_err());
    }

    #[test]
    fn cargo_and_npm_versions_must_match() {
        let dir = fixture_root();
        fs::write(
            dir.path().join("apps/knx-web/package.json"),
            r#"{"version":"0.1.0-alpha.2"}"#,
        )
        .unwrap();
        assert!(verify_config(dir.path(), "0.1.0-alpha.1").is_err());
    }
}
