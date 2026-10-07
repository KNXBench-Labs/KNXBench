# UI-owned alpha-readiness follow-ups — 2026-10-02

## Scope and source authority

User request: inspect `alpha_readiness.md` for `goal_ui` tasks and complete them.
The current file is `docs/ALPHA_READINESS.md`. Started isolated
`ui-alpha-readiness` at `65b91777`, reservation `bfb6fec1`; inherited U0–U13
closures are not reopened. All 24 UI-routed IDs have a source-based disposition
in `docs/UI_ALPHA_READINESS.md`; other owners and release approval stay separate.

## Implemented, tested contracts

- UX-02: supported-only catalog accept filter. VD2 refusal remains intentional.
- UX-03: existing undoable project-style command in Properties; authoritative
  returned tree, unknown values, refused requests and overlap protection.
- KL-121: periodic/focus/visibility refresh of authoritative server settings;
  local pending/failed writes and unknown/deleted keys survive stale replies;
  generation guards and cleanup protect ordering and stopped timers.
- KL-124: raw decoded Device Info retained through network client, nullable
  HTTP projection and six labelled expandable values; no fabricated metadata,
  reinterpretation of octets, default CLI change or protocol codec change.

Behavioral RED/GREEN traces and focused regressions cover each implementation.
Four picker/style negative controls and six settings/metadata controls failed
with actual assertions and exact byte restoration, not compilation failures.
The restored five-file focus passes 230 tests.

## Separate review and actual acceptance

In-session full-feature review recorded an IMPORTANT missing-CSS-rule finding
and MINOR inadequate label/generation assertions before remediation. Actual
first complete run `proc_9b56dc2c7143` exited 1 at the diagnostic shell guard
(Web 1,328 passed / one failed). It remains failed, not rewritten as success.
Both new classes gained bounded, wrapping, theme-aware layout; the guard was
not weakened. Final source review has no outstanding package-blocking finding;
this is not an independent external whole-product verdict.

Renewed `proc_fc86fe016b73` exited 0 and all eleven expected steps pass:
fmt, Web type/tests/build, workspace tests, strict workspace Clippy, layering,
headers, intended-root anchors, corpus-gate configuration and whitespace.
Web: 82 files / 1,329 tests. Workspace: 146 result blocks / 2,885 passed /
zero failed / 163 ignored. The 568 tracked source/configuration fingerprints
match before/after. Own target directory, both recognized gate locks and the
read-only original corpus location were used. A textual corpus gate is not
execution of ignored private-corpus tests.

Six intercepted Chromium cases (English/German × 360/640/1440) pass metadata
keyboard expansion, exact raw values, cell/bounds checks and zero unexpected
API/runtime errors. Whole-page overflow is explicitly NOT accepted: a
before/after comparison found the same existing invisible HelpTip overflow at
640 px with/without metadata, and German probes also exposed overflow. Hiding
the AT description would violate the existing contract; no such fix was made.

## Native evidence limits

Local GJS/Gtk/WebKitGTK 2.52.6 static device-editor fixture passed nine X11
cases: 360/640/1440 px × CSS zoom 0.8/1/1.5, actual matching viewport, no
document overflow, expected octet, links and flags. Default Wayland, an early
mis-sized X11 run and strict 641-versus-640 allocation comparison failed.
Only the final matching-width matrix is accepted, with one-pixel tolerance.
GPU/display warnings remain recorded. No Tauri IPC or real backend/bus was
used; optional fixture parameter requests were locally intercepted.
Orca/WebKitWebDriver were absent. No native file chooser, dead-WebView close,
AT announcements, real multicast or full Tauri shortcut/resize workflow is
accepted by this probe. No system/package/firewall changes were made.

## Delivery boundary and next scope

### Browser harness and final source renewal

Additional actual default E2E execution failed all 33 cases: 30 fixture pages
were not served, three legacy full-app cases failed their obsolete language
input operation. Those three initially ran against a real backend process;
the existing automatic discovery can reach it, and network traffic was not
retained. No assertion of pure mocks/no sockets or acceptance of live discovery
is made for that failed attempt. No tunnel/device-write action was exercised.

Added two RED configuration regressions, then an isolated loopback-only Vite
fixture server that replaces, rather than merges, the production API proxy.
The default Playwright command no longer starts the KNX backend. Converted all
three full-app cases to full API interception and current language-select,
style-select and catalog-region semantics. Unknown requests fail the test;
the automatic search is explicitly locally intercepted. All 33 browser cases
pass, none removed. Reintroducing the production proxy fails the named guard
with an assertion; exact fixture-file restoration and focused rerun pass.

New source required renewed complete acceptance. `proc_410a77847241` exited 0:
all eleven required steps pass; Web 83 files / 1,331 tests, workspace
146 result blocks / 2,885 passed / zero failed / 163 ignored, and all 570
tracked/new source/configuration fingerprints match. Earlier 82-file/568-source
acceptance belongs to the pre-harness candidate and is not transferred.

Integrated acceptance `proc_a6a7c68092c2` exited 0 with all eleven gates,
identical counts and source freeze. One accidental one-second foreground start
was interrupted (124), not accepted; the complete background renewal supplies
the integration evidence. Only six documentation/handover paths differ from
the gated source commit; all product source bytes match. The sole rebase
conflict retained both visible handover blocks, newest first. A subsequent
exact inherited-suffix audit discovered the generic replacement had dropped
the older archive in published 6c16fe5a. The closing corrective receipt restores
the whole byte-exact 768d53a2 handover, preserving all own new entries (3,134
lines before the explicit correction entry). Earlier complete-preservation
wording is disproved, not silently treated as proof. Product code is unaffected.
Cause identified in the resolution script: the final `>>>>>>> .*` marker
was matched with DOTALL enabled, so it consumed the whole inherited suffix.
This was a script bug, not Git discarding history or the patch tool losing
data. A closing-marker pattern must be line-bounded (`[^\n]*`), and whole
inherited-suffix equality is now a required pre-commit check.

Published source `6c16fe5aaf764d78f62382f867f59b6ae77f8dce` matches both
remote ref and complete tree, with zero outgoing commits, required
author/committer and no co-author trailer. Four scoped owner dispositions are
DONE; remaining IDs keep their existing states. Closing receipt publication
and owned cleanup remain pending. Preserve upstream
AR05/AR06 handovers and non-UI dispositions, all inventory IDs/priorities/routes
and inherited limitation identities. Root has foreign CURRENT_STATE, RESEARCH
and stats edits plus untracked telemetry/Paperclip files; do not synchronize or
regenerate its statistics here.

Continue KL-82 authoritative project/session-context and keyboard/background
modal/viewport-safe HelpTip contracts as separate packages. Domain/API/sample
prerequisites and explicit user release decisions remain separate. No subagents,
quota probes, real KNX access, bus/key/write authority, release tag, alpha-ready
claim or full ETS compatibility is inferred.
