# WP-11: first bounded native acceptance slice — 27 October 2026

**Later bounded follow-up:** the [positive native editor cells](2026-10-27-wp11-positive-native-editors.md)
now cover one declared integer slider and native-created-device plain
instructions after actual process restart. The results/exclusions below remain
the historical first slice, not positive editor or whole-WP-11 evidence.

## Delivery and package checks

Published on main as `cc9d8fab2168d10e788e89026dbc58c1d229bd94`
(feature `f4f112fe5`) after independent Claude review and correction re-review
APPROVE. Branch and main-first integrated change-scoped package gates each
passed all 13 planned stages with frozen inputs and no touched/unchecked user
data. Integrated owning Rust checks ran 103 xtask tests with zero failures or
ignored cases; tooling reported 461 methods, including six explicit skips
(455 executed). The selected corpus lane had no cases and was a recorded
no-op, not corpus execution. Web/browser/bindings were not selected by this
documentation package; native artifact observations above remain their own
execution evidence. Local, tracking and live main readback matched after push.

Review corrected the original count wording: 19 is a thematic label count,
not independently counted assertion groups. The expanded retained verifier
passed on unchanged native observations without repeating product execution;
the original review and verifier result remain retained separately.

## Authority and target

The owner confirmed the 27 October decisions and expressly admitted native
KNXBench interaction with isolated test data, without bus access or changes to
real projects. This removes WP-11's consent blocker; it does not close WP-11 or
any native/accessibility limitation.

The actual target was the published **Alpha 17 AppImage**, not a mock Tauri
runtime or a Chromium substitute: `KNXBench_0.1.0-alpha.17_amd64.AppImage`,
92,047,864 bytes, source `a9ea32d72364478b25e158d0b95465af7a4702df`, SHA-256
`cd1f0cdf129bfad9e143062e4395c52f9376fa49a51c35145a5c0046a49e1d8a`.
Both the release asset digest and downloaded `SHA256SUMS` match. The native UI
reported `0.1.0-alpha.17+ga9ea32d`. This is acceptance of that artifact, not of
all later source on main. No application code or released artifact was changed.

## Isolation and method

The app ran in a distinct user/network namespace with only loopback and no
external route. The host root was read-only; the real home and project mount
were masked. HOME, XDG data/config/cache/state, temporary files, selected save
paths and product catalogue belonged to the test sandbox. Live process/root
and network observations verified the isolation while the app was running.
Only the desktop's Wayland/accessibility infrastructure was shared.

Actions used the **actual native Wayland WebKitGTK window**, AT-SPI and guarded
keyboard input. A keyboard action required the sandbox app's process to own
the active window. Native chooser/overwrite actions were scoped to the
application's top-level native chooser or alert, not a similarly named web
button. No mocked responses, live bus operations, private corpus or real
projects were used. The additional device fixture was an untouched copy of
[the original fictional community demo](../COMMUNITY_DEMO_PROJECTS.md) and its
fictional product package, installed only into the sandbox catalogue.

## Observed outcomes

| Cell | Actual native evidence | Boundary |
| --- | --- | --- |
| Startup | Verified AppImage opened an actual Wayland window and rendered the app. | Renderer warnings retained; not vendor-specific EGL/GPU acceptance. |
| Keyboard overlays | New fictional project created; Ctrl+K opens Search with its combobox focused; Ctrl+Shift+P opens the command palette. | Not every overlay, theme or keyboard layout. |
| Settings focus | Theme receives initial focus; Shift+Tab wraps to the resize button, Tab wraps back; Escape restores the Settings opener. | AT-SPI/native input evidence, not spoken screen-reader output. |
| File Save As / Cancel / Replace | Native chooser produced a valid SQLite `.knxdb`; cancelled overwrite kept its SHA-256 and mtime; explicit Replace settled and the file reopened. | Tested project Save As path, not every native export caller. |
| Notes | Draft and preview preserved the saved root's hash/mtime; confirmation and Save persisted the exact title/body; the native UI reopened them before and after process restart. | One plain-text project note, not every target or conflict branch. |
| Zoom and preferences | Shortcuts reached 80% and 150%; zoom and navigation width survived process restart. | Stored settings plus native control observations, not full visual/layout parity. |
| Table/pane controls | Keyboard Right increased the named table and navigation separator values at 80% and 150%; navigation hide/show worked at 150%. | No pointer-drag distance or complete fit/announced-width proof. |
| File submenus | Import/export, history/comparison and help/diagnostics exposed their items at 80% and 150% in the tested window. | No universal clipping/viewport claim. |
| Fictional catalogue | Native file selection, sealed preview, explicit consent and sandbox installation completed; fictional demo opened. | Fixture preparation, not manufacturer compatibility. |
| Checked parameter | `unsupportedXml` refusal displayed the unchanged original value `2` and offered the plain editor. | **Positive checked-source editor remains untested.** |
| Installation instructions | `Original instruction source unavailable: MissingSource` was exposed natively. | **Positive instruction editing remains untested.** |
| Shutdown | Two ordinary native closes exited zero; no owned app remained. | No pending-job/forced-exit or power-loss claim. |

The retained verifier passed its assertions over the saved observations,
organised under **19 thematic labels**, with **130 logged operations**. The
labels are not independently counted tests, and a logged operation is not by
itself proof of a successful user workflow. Neither number establishes a
complete matrix or a new full application gate. SQLite integrity and exact
stored note values were checked independently of the rendered UI. Fictional
input copies stayed byte-identical.

## Retained harness outcomes

The initial root-mapped startup could not authenticate to the host AT-SPI bus.
The accepted interaction sessions retained the caller's identity, initialized
loopback with namespace capabilities, dropped inherited/ambient capabilities
before the filesystem sandbox, then ran the app in a nested user namespace.
Failed mount/capability preflights are retained separately.

An early unscoped Save action selected the toolbar rather than the chooser and
opened a second chooser. Those choosers were cancelled; accepted Save/Replace
steps use explicit native top-level scope. Another discovery check counted a
chooser as a second application window; main-window identity was corrected.
A focus-dispatch refusal sent no keyboard input. The first ordinary close
succeeded; the driver's subsequent tree capture failed because the app was
already gone, and separate exit/window evidence establishes the actual result.
None of these harness outcomes is relabelled as a product failure or erased.
The driver evolved during the steps: this is retained manual-step evidence,
not an immutable, single-run automated native suite.

## Still open / next cells

- Orca and Firefox were not installed; no installation or acceptance is claimed.
- Reuse or author a separately reviewed **synthetic admitted-source fixture**
  for positive checked-parameter and installation-instruction editing. The
  older fictional demo's unsupported/missing source is not such a witness.
- Native bus-monitor capture export still needs a synthetic/loopback capture;
  no real gateway or installation may be used under this authority.
- Pointer dragging, hover, fuller window/theme coverage and the remaining
  [KL-130](../limitations/KL-130-ZOOM-application-zoom.md) matrix stay open.
- Newer-Mesa vendor-specific startup and issue #6's reporter environment remain
  separate. The sandbox's fallback-rendering startup is not that proof.
- [KL-020](../limitations/KL-020-overlay-accessibility-evidence.md) retains
  whole-application and actual screen-reader boundaries. No WCAG or universal
  native parity claim follows.

[Public receipt](../evidence/wp11-native-2026-10-27.json) records the target,
validated outcomes, exclusions and private evidence locators. Raw trees,
actions, attempt logs and result files remain private under the named evidence
set. Review/package-gate and publication status are separate from this native
artifact run; this record does not claim an unexecuted gate.
