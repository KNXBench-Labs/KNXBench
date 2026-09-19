# Linux AppImage Packaging Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Produce and verify one downloadable `x86_64` KNXBench AppImage locally and through GitHub Actions.

**Architecture:** Tauri remains the only native packaging owner and bundles the existing Rust desktop shell plus compiled React frontend. A new `xtask check-appimage` command validates repository configuration and the concrete artifact before CI uploads it; a separate tag-only job owns release publication.

**Tech Stack:** Tauri 2, Rust 1.98, Cargo xtask, React/Vite, Node.js 22, GitHub Actions, AppImage.

**Spec:** `docs/superpowers/specs/2026-09-17-linux-appimage-design.md`

## Global Constraints

- Build only AppImage for `x86_64-unknown-linux-gnu`.
- Preserve the existing `knx-desktop -> knx-server -> domain/infrastructure` runtime path.
- Never bundle project files, manufacturer databases, passwords, gateway addresses, or fixtures.
- Keep `KNXBench`, `KNX-compatible`, and version `0.1.0-alpha.1` consistent.
- Manual workflow runs upload an Actions artifact; only pushed `v*` tags may publish a release.
- Do not add updates, signing, ARM64, musl, `.deb`, RPM, Flatpak, Snap, or store publication.
- Mark status complete only after a real AppImage build, inspection, and bounded startup pass.

---

### Task 1: Add a testable AppImage contract gate

**Files:**
- Create: `xtask/src/appimage.rs`
- Modify: `xtask/src/main.rs`
- Modify: `xtask/Cargo.toml`

**Interfaces:**
- Produces `appimage::verify(root, artifact_dir, tag) -> Result<VerifiedAppImage, String>`.
- Produces `cargo run -p xtask -- check-appimage [--artifact-dir PATH] [--tag vVERSION]`.

- [ ] **Step 1: Write failing tests**

Add `serde_json.workspace = true` under dependencies and `tempfile.workspace = true` under dev-dependencies. Create tests in `xtask/src/appimage.rs` for one valid executable artifact and for missing, multiple, empty, non-executable, and wrong-version artifacts:

```rust
#[test]
fn one_executable_versioned_appimage_is_accepted() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("KNXBench_0.1.0-alpha.1_amd64.AppImage");
    write_executable(&path, b"appimage");
    assert_eq!(verify_artifacts(dir.path(), "0.1.0-alpha.1").unwrap().path, path);
}

#[test]
fn tag_must_equal_the_version_after_its_v_prefix() {
    assert!(verify_tag(Some("v0.1.0-alpha.1"), "0.1.0-alpha.1").is_ok());
    assert!(verify_tag(Some("v0.1.0-alpha.2"), "0.1.0-alpha.1").is_err());
    assert!(verify_tag(Some("0.1.0-alpha.1"), "0.1.0-alpha.1").is_err());
}
```

Add config-fixture tests rejecting inactive bundling, a non-AppImage target, absent frontend or licence resource, absent licence file, and Cargo/npm version mismatch.

- [ ] **Step 2: Verify RED**

Run `cargo test -p xtask appimage -- --nocapture`.
Expected: compile failure because validation functions do not exist.

- [ ] **Step 3: Implement the validator**

Implement:

```rust
pub struct VerifiedAppImage { pub version: String, pub path: PathBuf }
pub fn verify(root: &Path, artifact_dir: &Path, tag: Option<&str>)
    -> Result<VerifiedAppImage, String>;
fn workspace_desktop_version(root: &Path) -> Result<String, String>;
fn verify_config(root: &Path, version: &str) -> Result<(), String>;
fn verify_artifacts(dir: &Path, version: &str) -> Result<VerifiedAppImage, String>;
fn verify_tag(tag: Option<&str>, version: &str) -> Result<(), String>;
```

Use `cargo_metadata::MetadataCommand` for `knx-desktop`, `serde_json::Value` for both JSON files, and `PermissionsExt::mode() & 0o111` for execute bits. Require exact targets `['appimage']` and resources `../../knx-web/dist -> frontend`, `../../../LICENSE -> LICENSE`.

Wire `check-appimage` into `xtask/src/main.rs`. Accept only the documented options, default the artifact directory to `target/release/bundle/appimage`, and print `AppImage ok: version VERSION, artifact PATH`. Invalid options print one usage line and fail.

- [ ] **Step 4: Verify GREEN and existing gates**

```bash
cargo test -p xtask appimage -- --nocapture
cargo run -p xtask -- check-layering
cargo run -p xtask -- check-headers
```

- [ ] **Step 5: Commit**

```bash
git add xtask Cargo.lock
git commit -m "test(release): enforce AppImage package contract"
```

---

### Task 2: Configure the bundle and record the decision

**Files:**
- Create: `LICENSE` if absent on the branch
- Create: `apps/knx-desktop/src-tauri/icons/128x128.png`
- Create: `apps/knx-desktop/src-tauri/icons/256x256.png`
- Modify: `apps/knx-desktop/src-tauri/tauri.conf.json`
- Create: `docs/adr/0021-appimage-is-the-first-linux-package.md`
- Modify: `docs/adr/README.md`

**Interfaces:**
- Produces Tauri configuration for exactly one AppImage and accepted ADR 0021.

- [ ] **Step 1: Prove current config fails**

Create empty `target/release/bundle/appimage`, run `cargo run -p xtask -- check-appimage`, and expect rejection of `bundle.active: false`.

- [ ] **Step 2: Add the approved canonical licence unchanged**

If absent, copy `/mnt/daten-i/Sourcecode/KNXBench/LICENSE` to the branch. Verify 34,523 bytes, 661 lines, and SHA-256 `0d96a4ff68ad6d4b6f1f30f713b18d5184912ba8dd389f86aa7710db079abcb0`. Never download or rewrite it here.

- [ ] **Step 3: Create required icon sizes from the existing icon**

```bash
magick apps/knx-desktop/src-tauri/icons/icon.png -filter point -resize 128x128 apps/knx-desktop/src-tauri/icons/128x128.png
magick apps/knx-desktop/src-tauri/icons/icon.png -filter point -resize 256x256 apps/knx-desktop/src-tauri/icons/256x256.png
identify apps/knx-desktop/src-tauri/icons/{128x128,256x256}.png
```

Expected: RGBA images of exact named dimensions. Keep the current visual; branding is outside scope.

- [ ] **Step 4: Activate and describe the bundle**

Set the `bundle` object to:

```json
{
  "active": true,
  "targets": ["appimage"],
  "category": "Development",
  "shortDescription": "KNX-compatible engineering application",
  "longDescription": "Linux-first KNX-compatible project engineering application.",
  "licenseFile": "../../../LICENSE",
  "icon": ["icons/icon.png", "icons/128x128.png", "icons/256x256.png"],
  "resources": {
    "../../knx-web/dist": "frontend",
    "../../../LICENSE": "LICENSE"
  }
}
```

Do not add a Tauri version; ADR 0018 keeps Cargo as its source.

- [ ] **Step 5: Write ADR 0021 and update the ADR index**

Record AppImage as the first Linux package, the tested x86_64/glibc boundary, `.deb` and Flatpak alternatives, one-file delivery benefit, manual-update cost, and requirement for real build/install evidence before adding a format.

- [ ] **Step 6: Verify config with a synthetic artifact and commit**

Create executable `target/release/bundle/appimage/KNXBench_0.1.0-alpha.1_amd64.AppImage`, run `check-appimage`, remove it, and run `git diff --check`. Then commit:

```bash
git add LICENSE apps/knx-desktop/src-tauri docs/adr
git commit -m "build(desktop): configure Linux AppImage bundle"
```

---

### Task 3: Add package and tag-release automation

**Files:**
- Create: `.github/workflows/linux-appimage.yml`

**Interfaces:**
- Produces a read-only build job and tag-only release job.

- [ ] **Step 1: Add the build workflow**

Trigger on `workflow_dispatch` and pushed tags `v*`; default permissions are `contents: read`. On `ubuntu-latest`, install `build-essential`, `curl`, `file`, `libayatana-appindicator3-dev`, `libgtk-3-dev`, `librsvg2-dev`, `libssl-dev`, `libwebkit2gtk-4.1-dev`, `patchelf`, `wget`, `xauth`, and `xvfb`. Set up Node 22 and Rust, install `tauri-cli` exactly `2.11.4` with `--locked`, run `npm ci --prefix apps/knx-web`, then from `apps/knx-desktop` run:

```bash
APPIMAGE_EXTRACT_AND_RUN=1 cargo tauri build --bundles appimage --ci
```

Use separate conditioned validation steps: tag builds pass `--tag "$GITHUB_REF_NAME"`; manual builds omit it. Run the bounded Xvfb smoke test and require timeout status 124:

```bash
appimage=$(find target/release/bundle/appimage -maxdepth 1 -type f -name '*.AppImage')
set +e
timeout 15s xvfb-run -a env APPIMAGE_EXTRACT_AND_RUN=1 "$appimage" >appimage-startup.log 2>&1
status=$?
set -e
test "$status" -eq 124 || { cat appimage-startup.log; exit "$status"; }
```

Upload `target/release/bundle/appimage/*.AppImage` with `actions/upload-artifact@v4` and `if-no-files-found: error`.

- [ ] **Step 2: Add the isolated release job**

The job needs the build, runs only when `github.ref_type == 'tag'`, and alone has `permissions: contents: write`. Download the matching Actions artifact and use `GH_TOKEN: ${{ github.token }}`:

```bash
if gh release view "$GITHUB_REF_NAME" >/dev/null 2>&1; then
  gh release upload "$GITHUB_REF_NAME" ./*.AppImage --clobber
else
  gh release create "$GITHUB_REF_NAME" ./*.AppImage --verify-tag --generate-notes
fi
```

- [ ] **Step 3: Validate and commit**

Statically assert the workflow contains both triggers, both permission levels, the exact build command, `check-appimage`, startup timeout, upload-artifact v4, and tag condition. Run focused xtask tests and `git diff --check`. Commit:

```bash
git add .github/workflows/linux-appimage.yml
git commit -m "ci(release): build Linux AppImage artifacts"
```

---

### Task 4: Build, inspect, and launch the real artifact

**Files:**
- Modify Tasks 1-3 files only when measured evidence requires a packaging fix.

**Interfaces:**
- Produces concrete evidence required before documentation claims completion.

- [ ] **Step 1: Install and build**

```bash
npm ci --prefix apps/knx-web
cd apps/knx-desktop
APPIMAGE_EXTRACT_AND_RUN=1 cargo tauri build --bundles appimage --ci
```

Expected: exactly one executable under `target/release/bundle/appimage`. Record filename, size, SHA-256, build host, Rust, Node, and Tauri versions. Do not commit the artifact.

- [ ] **Step 2: Validate and inspect contents**

Run `cargo run -p xtask -- check-appimage`. Resolve the artifact with `appimage=$(realpath target/release/bundle/appimage/*.AppImage)`, change into an empty `/tmp/knxbench-appimage-inspect`, and extract with `"$appimage" --appimage-extract` (extraction mode does not require FUSE). Verify the AppDir includes the desktop executable, `.desktop` metadata, icons, `frontend`, and canonical `LICENSE`.

- [ ] **Step 3: Run a bounded local startup**

```bash
set +e
timeout 15s env APPIMAGE_EXTRACT_AND_RUN=1 "$appimage" >/tmp/knxbench-appimage-startup.log 2>&1
status=$?
set -e
test "$status" -eq 124
```

Expected: process remains alive until timeout and the log has no missing-library, resource, panic, or server-start failure. Close the visible window. Do not discover or write to a KNX gateway.

- [ ] **Step 4: Fix only evidenced packaging failures**

For each failure, add a focused regression where static validation can catch it, apply the narrow fix, and repeat build, inspection, and startup. Stop when all pass.

- [ ] **Step 5: Commit evidence-driven fixes if any**

```bash
git add xtask apps/knx-desktop/src-tauri .github/workflows/linux-appimage.yml
git diff --cached --quiet || git commit -m "fix(release): make AppImage self-contained"
```

---

### Task 5: Document installation and verified status

**Files:**
- Modify: `README.md`
- Modify: `docs/ROADMAP.md`
- Modify: `docs/IMPLEMENTATION_STATUS.md`
- Modify: `goal.md`
- Create: `.ai/logs/2026-09-17_codex_linux_appimage.md`
- Modify: `.ai/CURRENT_STATE.md`

**Interfaces:**
- Consumes exact evidence from Task 4; produces user instructions and synchronized status.

- [ ] **Step 1: Document build, install, launch, update, and removal**

Replace the generic release command with the workflow-faithful AppImage command. Document how to make the file executable and launch it, the Actions artifact/tag-release path, x86_64/glibc and GTK/WebKitGTK boundary, app-data location, removal, and absence of updater, signature, ARM64, and native dependency management.

- [ ] **Step 2: Update status from measured evidence only**

Mark Linux packaging delivered in Roadmap and Goal only after Task 4 passes. Add a dated Implementation Status entry with exact artifact filename, size, SHA-256, host, inspected members, launch duration/exit, and focused test results. Claim only the distribution actually tested.

- [ ] **Step 3: Write the handover log**

Record decision/ADR, exact commands, environment versions, artifact identity, inspection results, startup proof, all checks, limitations, and that no tag or release was published. Update `.ai/CURRENT_STATE.md` in its required format.

- [ ] **Step 4: Verify and commit docs**

```bash
cargo test -p xtask
cargo run -p xtask -- check-appimage
cargo run -p xtask -- check-layering
cargo run -p xtask -- check-headers
git diff --check
git add README.md docs/ROADMAP.md docs/IMPLEMENTATION_STATUS.md goal.md .ai
git commit -m "docs(release): record verified Linux AppImage"
```

---

### Task 6: Final verification and review

**Files:** Review every file changed since `f7893ec`.

- [ ] **Step 1: Run final gates**

```bash
cargo fmt --all --check
cargo clippy -p xtask --all-targets -- -D warnings
cargo test -p xtask
cargo run -p xtask -- check-layering
cargo run -p xtask -- check-headers
npm test --prefix apps/knx-web
npm run build --prefix apps/knx-web
cargo run -p xtask -- check-appimage
git diff --check f7893ec..HEAD
```

Rerun the Task 4 bounded startup against the final artifact. Do not run the full Rust workspace unless packaging changed runtime dependencies or a focused failure points there.

- [ ] **Step 2: Review scope and leave a clean branch**

Inspect `git log`, `git diff --stat`, and the full diff. Confirm one package target, no updater/signing/bus change, no private data, conditional tag publication, and evidence-backed claims. Leave `linux-appimage` clean; do not merge, push, tag, or publish without a separate user instruction.
