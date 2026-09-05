---
name: run-knx-desktop
description: Build, launch, and drive the KNXBench desktop app (Tauri + React) on Hyprland/Wayland. Use when asked to run, start, test, or screenshot the KNXBench desktop app, verify a UI change actually works, or check that a change to apps/knx-desktop renders correctly.
---

Paths below are relative to `apps/knx-desktop/` (this app's root).

KNXBench's desktop app is a Tauri 2 shell (Rust, `src-tauri/`) around a
React/TypeScript frontend (`src/`, served by Vite in dev). It's a real
native GTK/WebKit2GTK window, not a browser tab, so it's driven with
OS-level tools (`hyprctl`, `grim`, `wtype`), not `chromium-cli`.

## Run (agent path)

Use `.claude/skills/run-knx-desktop/driver.sh`. It builds, launches,
waits for the window, and gives you a screenshot + keyboard handle.

```bash
cd apps/knx-desktop
.claude/skills/run-knx-desktop/driver.sh start                 # build + launch, waits for window (~1-2 min cold, ~seconds warm)
.claude/skills/run-knx-desktop/driver.sh status                 # window geometry + process check
.claude/skills/run-knx-desktop/driver.sh shot /tmp/out.png       # screenshot the app window (region grab via hyprctl geometry)
.claude/skills/run-knx-desktop/driver.sh key ctrl+shift+p        # send a key combo (opens the command palette)
.claude/skills/run-knx-desktop/driver.sh key ctrl+k              # (no-op until a project is loaded - see Gotchas)
.claude/skills/run-knx-desktop/driver.sh type "some text"        # type literal text
.claude/skills/run-knx-desktop/driver.sh stop                    # kill cargo-tauri, the app binary, and the vite dev server
```

Verified end-to-end flow: `start` → `shot` shows the toolbar with
`Save`/`Undo`/`Search…` greyed out (no project loaded) → `key
ctrl+shift+p` → `shot` shows the command palette open with a filtered
command list → `wtype -k Escape` closes it.

## Build

```bash
cd apps/knx-desktop
npm install                 # frontend deps (React 19, Vite 8)
```

Rust deps resolve via the workspace `Cargo.lock` on first `cargo
tauri dev`/`build` - no separate step needed. First build compiles
~466 crates (webkit2gtk, gtk, tauri, the KNX crates) and takes ~1-2
minutes; incremental rebuilds are seconds.

## Test

```bash
cd apps/knx-desktop
npm run test                       # vitest: 6 files, 46 tests (frontend logic - toast, search, command registry, tree utils, theme, dashboard stats)
cargo test -p knx-desktop          # backend: command_dispatch, device_detail, open_reference_project, save_load_roundtrip
```

Both ran clean in this session. These are the fast, headless check -
prefer them over launching the GUI for logic-only changes.

## Direct invocation

Most command/domain logic lives behind Tauri commands in
`src-tauri/src/` and is covered by the `cargo test -p knx-desktop`
integration tests above (they call the command implementations
directly, no window needed). Reach for the GUI driver only when the
change is actually about rendering, layout, or interaction - not
every PR needs a screenshot.

## Gotchas

- **Blank/black window on launch** - WebKit2GTK fails to allocate a
  GBM buffer on this GPU/compositor combo (`Failed to create GBM
  buffer of size 1200x800: Invalid argument`, or a Wayland "Error 71
  (Protocol error)" crash on the native Wayland backend). Fix:
  `WEBKIT_DISABLE_DMABUF_RENDERER=1` before `cargo tauri dev`. The
  driver already sets this - if you bypass the driver and launch
  manually, you'll hit this.
- **No `npm run tauri` script** - this app's `package.json` doesn't
  define one. Use the `cargo tauri` CLI directly (`cargo tauri dev` /
  `cargo tauri build`), not `npm run tauri -- dev`.
- **Toolbar buttons start disabled** - `Save`, `Save As…`, `Undo`,
  `Redo`, and `Search… (Ctrl+K)` are greyed out until a project is
  opened via `Open project…` or `Open (.knxdb)…`. Sending `ctrl+k`
  before that is a silent no-op - don't mistake it for a broken
  driver. `Ctrl+Shift+P` (command palette) works regardless.
- **This is Hyprland-specific.** Window discovery uses `hyprctl
  clients -j` (title match on `KNXBench`), screenshot uses `grim -g`
  with that geometry, and input uses `wtype`. Plain `hyprctl dispatch
  focuswindow ...` did **not** work in this environment (a
  Lua-scripted hyprctl build expects a different dispatch syntax) -
  the driver avoids it entirely; newly-spawned windows get focus by
  default so keyboard input reaches them without an explicit focus
  call. On X11 or a stock Hyprland build you'd swap in
  `xdotool`/`scrot` or plain `hyprctl dispatch`.
- **Toast notifications are real and can appear unprompted** - a
  "Gremlins in the wiring: no known-element table for schema version
  21" toast surfaced during testing from an async check unrelated to
  the keypress that triggered the screenshot. Don't assume every
  visible toast was caused by your last action.

## Troubleshooting

| Symptom | Fix |
|---|---|
| `npm error Missing script: "tauri"` | Use `cargo tauri dev`, not `npm run tauri -- dev`. |
| `error: manifest path ... does not exist` from `cargo tauri dev -- --manifest-path ...` | Don't pass `--manifest-path` - run `cargo tauri dev` from inside `apps/knx-desktop/`; it finds `src-tauri` on its own. |
| Window process running but screenshot is solid dark grey/black | GBM allocation failure - relaunch with `WEBKIT_DISABLE_DMABUF_RENDERER=1` (see Gotchas). |
| `driver.sh shot` errors "window not found" | App hasn't finished launching yet, or `stop` was called - re-run `driver.sh start` and check `driver.sh status`. |
