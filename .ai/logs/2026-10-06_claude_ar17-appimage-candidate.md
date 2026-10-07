# AR17: local AppImage candidate (Claude, 2026-10-06)

- User allowed AR17 before AR16 (2026-10-06) and set the AR16 manual policy:
  GitHub, with screenshots.
- Build 1 (`f728b0d3`): `cargo tauri build` failed in linuxdeploy;
  `check-appimage` refused desktop 0.1.0-alpha.1 vs web 0.1.0-alpha.4.
  Commit `6b9b6818` raises knx-desktop (+ Cargo.lock) to alpha.4.
- Build 2 (`6b9b6818`, clean tree, KNX_REQUIRE_CLEAN_TREE=1): ok; artifact
  107,829,752 bytes, sha256 4c778104a11112f233dca2b82fb03111e259127b8f7b93c4357ccbf77e480ef2;
  check-appimage ok; commit hash string present in the binary.
- Smoke (unshare --net, lo only, uid 1000 mapped, private XDG/TMPDIR):
  - Unmodified AppImage on host display: `Failed to initialize GTK` (101).
    Host Xwayland socket refuses connections (xprop fails too). Hook forces
    GDK_BACKEND=x11.
  - Wayland override without DMABUF setting: protocol error 71. With
    WEBKIT_DISABLE_DMABUF_RENDERER=1: runs; full API smoke passed; screenshot.
  - Unmodified AppImage under private Xvfb 21.1.24-1 (Arch package, gpg Good
    signature): full API smoke passed; screenshot shows v0.1.0-alpha.4.
  - Both: new/save-as/import/save-as/reopen 200; missing file 400; non-ZIP
    500 with clear text; no non-loopback sockets.
- Packaging: 337 files; extracted AppImage == AppDir; no project/product/db/
  md/key/env files, no `OriginalData`, no source-tree path; ~540 home-dir
  paths from ~/.cargo and ~/.rustup (privacy item 6).
- Evidence: `~/.hermes/profiles/knxbench/evidence/alpha-release/ar17-candidate-20261006`.
