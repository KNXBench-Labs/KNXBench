# WP-11: current-source native notes cells — 9 November 2026

Scope: one bounded slice of the 8 November acceptance target (notes editor, save/cancel, 80/100/150% zoom) on **current source**, in an actual isolated Wayland/WebKitGTK window with fictional data. Not whole-WP-11 or KL-130 acceptance; no ledger status changes.

## What was bound

- Source `0b182054a` (clean tree), freshly built: `npm ci && npm run build`, then `cargo build --release -p knx-desktop` with private target and scratch XDG. The observed document title was `KNXBench 0.1.0-alpha.20`; build source is authenticated separately, not inferred from a UI Git suffix.
- Executed binary equals the built file; full binary and fictional-fixture SHA-256 values and the 147-file frontend manifest digest are in the [measurement](../evidence/wp11-current-native-notes-2026-11-09.json) (method stated there). WebKitGTK 2.54.1, GTK 3.24.52, libsoup 3.8.0, AT-SPI core 2.62.1.
- Fixture: the same fictional office project as the 3 November records (full SHA-256 in the measurement), copied to a writable sandbox path. It migrated from schema 10 to 19 and had no note table before the first confirmed change.
- Isolation: the restart receipt observes user + network namespaces (loopback only), dropped capabilities, read-only root, direct user-data/mount/runtime/temp filesystem masks, private XDG dirs and a nonzero child UID. This UID is not host-identity separation proof. Direct GTK chooser (`GTK_USE_PORTAL=0`), in-memory GSettings, and shared host Wayland/session-D-Bus/AT-SPI were used; GVFS fell back to the shared session bus. Masking refers to direct filesystem mounts, not host-service isolation; no full WebKit sandbox attestation. Software-render WebKit flags remain explicit. Runtime isolation was inspected on restart only; the first launch has identical recorded argv.

## Executed per zoom cell (genuine native actions)

Zoom was set with the native Ctrl+0 / Ctrl+minus ×2 / Ctrl+= ×5 shortcuts and **observed** two ways: persisted `uiScale` (1 / 0.8 / 1.5) and AT-SPI geometry of the same controls (e.g. Title entry height 60 → 48 → 89 logical px at 100 / 80 / 150%; widths reflow with the window and are not a scale proof).

For each of 100, 80 and 150% (accepted 150% cell: note "Fictional note 150-clean"):

1. **Create**: New note, typed exact title and two-line body (umlauts, `&`, `<ok>`), AT-SPI readback equal to the typed text, Preview change, Confirm change, toolbar Save. Read-only SQLite oracle: row present with exact bytes after Save.
2. **Draft cancel**: select note, append a marker, Preview change, Cancel preview (draft stays dirty), Reload notes + Discard draft. Stored note unchanged in UI; project file SHA-256 and mtime identical before/after.
3. **Edit and save**: append a suffix, Preview, Confirm, Save. Oracle: exact new body. Reload notes + reselect: exact readback.

Observed save semantics: Confirm change rewrites the file but leaves the note row at its old content; the row changes only on the toolbar Save. After Save the editor shows the first note again.

## Restart

Ordinary Close (launcher exit 0, no owned process/window left), relaunch with the same private profile, native Open chooser typed path, Notes: all four notes read back with exact text; `uiScale` still 1.5 with identical control geometry; the file hash was unchanged by the read-only reopen.

## Not accepted / retained

Launcher died with its controller (harness), offscreen Confirm/Preview/Reload controls aborted several runs (AT-SPI `SHOWING=false`; the driver now scrolls with AT-SPI `scroll_to`), and partial 150% runs contaminated the first 150% note body, so that cell is retained but superseded by the clean one. One oracle read hit a transient "readonly database" and now retries. Details in the JSON. No product bug was reproduced; no source changed.

## Independent evidence review

Independent read-only review confirmed the three accepted native cells, their
oracle/snapshot/action bindings, the separate clean 150% note and the exact
restart readback. Its initial BLOCK concerned public claims only: undisclosed
chooser/host-service exceptions, a local desktop detail, a tree count, excessive
seat-restoration wording and an imprecise manifest recipe. Those statements were
corrected against retained evidence; no native run, build or test was replayed.
The payload retains 92 AT-SPI tree snapshot files including retries; counts are
artifacts, not independent passing test counts.

## Not claimed

Other keyboard/zoom/layout policy, physical-keyboard comparison, other targets (only "Project"), move/remove, the documentation-include flag, Orca, Firefox, AppImage/GPU, X11, whole WP-11. Raw trees, actions, raw input, 56 screenshots and oracle files stay in the permanently retained private evidence. All 29 recorded focus/active-workspace predicates passed; cursor position and full workspace inventory were not restored, and the initial empty workspace 1 disappeared. These are limited predicates, not complete seat restoration. Teardown details are in the JSON.
