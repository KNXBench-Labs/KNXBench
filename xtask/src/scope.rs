//! Validates the runtime workspace selected for a repository gate.

use std::path::{Path, PathBuf};

/// Resolve a supplied root, or the working directory, without a baked build path.
/// A gate never climbs to a parent workspace or falls back to another checkout.
pub fn workspace_root(explicit: Option<&str>) -> Result<PathBuf, String> {
    let candidate = match explicit {
        Some(path) => PathBuf::from(path),
        None => {
            std::env::current_dir().map_err(|e| format!("cannot read working directory: {e}"))?
        }
    };
    let root = candidate
        .canonicalize()
        .map_err(|e| format!("gate target {} is unavailable: {e}", candidate.display()))?;
    if !root.join("Cargo.toml").is_file() {
        return Err(format!("gate target {} has no Cargo.toml", root.display()));
    }
    let metadata = cargo_metadata::MetadataCommand::new()
        .manifest_path(root.join("Cargo.toml"))
        .current_dir(&root)
        .no_deps()
        .exec()
        .map_err(|e| format!("cannot validate gate target {}: {e}", root.display()))?;
    let resolved = Path::new(metadata.workspace_root.as_str())
        .canonicalize()
        .map_err(|e| format!("cannot resolve workspace root: {e}"))?;
    if root != resolved {
        return Err(format!(
            "gate target {} is not the workspace root {}",
            root.display(),
            resolved.display()
        ));
    }
    for (name, manifest) in [
        ("xtask", "xtask/Cargo.toml"),
        ("knx-core", "crates/knx-core/Cargo.toml"),
    ] {
        let expected = root
            .join(manifest)
            .canonicalize()
            .map_err(|e| format!("gate target {} lacks {manifest}: {e}", root.display()))?;
        if !metadata.packages.iter().any(|package| {
            package.name == name
                && metadata.workspace_members.contains(&package.id)
                && Path::new(package.manifest_path.as_str())
                    .canonicalize()
                    .is_ok_and(|path| path == expected)
        }) {
            return Err(format!(
                "gate target {} lacks workspace member {name} at {manifest}",
                root.display()
            ));
        }
    }
    Ok(root)
}
