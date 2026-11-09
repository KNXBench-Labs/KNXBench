# WP-11: current-source positive editor cells — 9 November 2026

Scope: the instruction editor and one declared checked integer slider at 80/100/150% on current source, in an actual isolated Wayland/WebKitGTK window with fictional data. One ordinary close and restart. Counterpart of the [27 October positive record](2026-10-27-wp11-positive-native-editors.md). Not whole-WP-11 or KL-130 acceptance; no ledger change. Measurement: [JSON](../evidence/wp11-current-positive-editors-2026-11-09.json).

## Binding

- Actual build source `0b182054a`; admission source `bf6065f0f`. The tracked diff between them is three documentation files, so the retained build is reused (tracked-input equivalence only). No new build; the UI still reports `0.1.0-alpha.20+g0b182054a`. This is not a freshly built `bf6065f0f` binary.
- Executable SHA-256 `8ea1c2f0a20ce94ff7c387148bd277fa97ddd519e94cdcdba3270d0ac666dc67`; 147-file frontend manifest `acb0e387d927f571ab40fc6752e1e0374b0f61f4d04d580b0471064b4406338c` (recipe in the JSON); all 149 artifact files authenticated against the retained manifest. Both launches recorded the same executable digest.
- Fixture: the original fictional `native-slider.knxprod` (SHA-256 `32e8a5b4cf21a14c0e8dda2c861a5babbac54559707d82353074aae02d06f2e1`), decoded from the public 27 October fixture record, member hashes re-verified. `M-7FF2`, default 2, bounds 0…240, declared Slider hint. The no-hint control is not used.
- Setup was native UI only: new project, Add device, product package chooser, consent, install, create device, Save through the native Save chooser. No SQL or CLI edits. WebKitGTK 2.54.1, GTK 3.24.52, libsoup 3.8.0, AT-SPI core 2.62.1.
- Isolation, per launch: nonzero UID, loopback-only network namespace, masked user-data paths, private HOME/XDG, dropped capabilities. Exceptions stay: shared Wayland/session-bus/AT-SPI, direct chooser (`GTK_USE_PORTAL=0`), in-memory GSettings, software-render flags, no WebKit-internal sandbox attestation.

## Results per zoom

Zoom was set with Ctrl+0 and Ctrl+minus/Ctrl+= and observed as persisted `uiScale` (1 / 0.8 / 1.5) and control geometry (for example Save button width 66 → 52 → 98 logical px).

| Cell | 100% | 80% | 150% |
| --- | --- | --- | --- |
| Slider draft Cancel | 2 → draft 9 → 2, file unchanged | 9 → 13 → 9, unchanged | 13 → 21 → 13, unchanged |
| Apply + Save | raw 9 | raw 13 | raw 21 |
| Undo + Save | no override row (default 2) | raw 9 | raw 13 |
| Redo + Save | raw 9 | raw 13 | raw 21 |
| Instruction preview/Cancel/discard | saved empty, unchanged | saved 100 text, unchanged | saved 80 text, unchanged |
| Two-line text preview + Apply change + Save, reopen | exact | exact (after Remove instructions) | exact (after Remove instructions) |

"Unchanged" means the read-only SQLite oracle shows identical file SHA-256 and mtime. The saved text has umlauts, `&`, `<ok>`, an en dash and quotes; the oracle row and the reopened editor equal it exactly.

Semantics seen: Apply value / Apply change write file bytes before the toolbar Save, but the parameter/instruction row appears only after Save. An unapplied instruction draft survives tab switches and preview Cancel; leaving to All devices and reopening discards it. Later zooms replaced the old text through the real Remove instructions preview (Apply change, Save, zero rows), then typed into a verified empty editor; no virtual Ctrl+A/Delete replacement was used.

## Restart

First launch closed with the app Close button (launcher exit 0, no owned process or window left). Second launch, same private profile: native Open chooser, device list, Installation instructions, Parameters, Checked editor. Readback: instruction equals the 150% text exactly, slider entry and value 21, `uiScale` 1.5, saved file SHA-256 `a6a546abc64bb4bf2ba76cd7b5dcbf4d640790ebd8061624d619bbf56c057a1e` unchanged by the read-only reopen. Second close exit 0; process, window, rule and runtime directory absent.

## Retained failures

- 80% instruction Cancel, attempt 1: Ctrl+End did not move the caret, so the marker went to the start; the assertion refused before preview. The successor expects the prefix.
- 150% instructions: two unsuccessful batches of four attempts (original and `b` labels) refused on `SHOWING=false`; neither delivered text/key input, although `b` attempts performed native navigation. Successful `c` successors followed AT-SPI `scroll_to`, with SHOWING still required. Pipeline `grep`/`tail` exits masked child failures; `finally` seat restoration was not cell success.
- `cells.py` and `closeapp.py` lacked the task driver `sys.path` under isolated Python; import failures occurred before any action.

## Counts and limits

118 structural tree snapshot files (including retry names), 56 oracle files, 86 screenshots, 173 action records, 66/66 seat-restore predicates ok. Counts are artifacts, not independent tests. The predicate is active-window/active-workspace restoration; all recorded active-window addresses were null, so restoring another focused application is not demonstrated. Cursor and full workspace inventory were not restored or claimed.

Not claimed: other checked controls, no-hint control as positive, imported-original instructions, rich formatting, capture export, physical-keyboard comparison, full-tree/pixel parity, Orca, Firefox, AppImage/GPU, X11, other desktops. Raw trees, actions, input and images are permanently retained in the private evidence archive identified by the measurement. Independent read-only review confirmed the retained native outcomes; its initial BLOCK concerned the public newline/failure descriptions, corrected from unchanged raw oracles and traces without replay. Restart text proof uses AT-SPI plus read-only SQLite; its screenshot is partly clipped.
