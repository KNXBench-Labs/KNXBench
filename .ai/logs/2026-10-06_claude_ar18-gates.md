# AR18 final gates (Claude, 2026-10-06)

Worktree `alpha-ar18`, detached `4b9e913e`, scratch `alpha-release/ar18`.
gate.sh (evidence `ar18-candidate-20261006`): leases 7/8/9, KNX_* unset,
offline namespace for Rust/browser/corpus tests, inputs frozen, 07:46–08:06.

Results: see docs/ALPHA_FINAL_GATES.md. Corpus selection: python scan of
`#[ignore = "...OriginalData..."]` → 144 hits, 2 are doc comments
(knx-testsupport `corpus_available`, knx-etsproj doc) → 142 tests, 31 targets.
Red pair diagnosed by reading: http_device_compare harness uses
`AppState { ..Default::default() }` (no history store) while
http_device_download uses `AppState::new(dir)`; the 503 is the AUDIT-01
fail-closed path. Not fixed (commissioning-owned test file).

AppImage: RUSTFLAGS remap worktree→/knxbench, ~/.cargo→/cargo,
~/.rustup→/rustup; 515 neutral paths, 0 home paths; 337 files; SHA-256
70bbb6b6…f81b. Smoke via native Wayland recipe (KL-158): window rendered the
start page (screenshot cropped to the app window only), API steps as AR17.
