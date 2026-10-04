# Productive CRT interaction follow-up — 2026-10-03

Agent: codex (in-session implementation/review; no independent reviewer).
Branch: feat/crt-interactions-20261003.
Worktree: /mnt/daten-i/Sourcecode/KNXBench.worktrees/crt-interactions-20261003.
Base: 3337e4ef202d773de067c9a758e0c03fd6804e64.
User request: “bau die animationen ein”. No feature commit/push/main merge requested.

Publication follow-up: the user subsequently authorized commit, main integration
and push with “dann bitte”. The original local-only delivery state below is
historical; publication evidence is recorded in the separate closure log.

## Change and boundary

The real App owns a disposable scoped DOM controller through useCrtInteractions.
ProjectExplorer native labels and GroupAddressTable native rows/buttons supply
presentation markers only; their business selection handlers are unchanged.
Existing delegated explorer/workspace keyboard handling is reused unchanged.
Manual Save/Save As requests emit a presentation signal after their existing
cancellation/stale-snapshot guards. Autosave does not signal, and neither request
admission nor feedback means successful persistence.

Motion style crt is registered in both runtime and actual pre-mount bootstrap.
Standard uses 250ms ease-out fill, light capped at 250ms and activation capped at
120ms. Subtle retains 120ms fill only. Off/OS reduced motion cancels effects.
Each bright-effect stream has a 600ms admission gap without throttling actions.
The inert aria-hidden span sits outside native table markup; its bounds intersect
viewport/ancestor scroll clips and account for existing root UI zoom. Timers,
observers and ephemeral flags are retired on preference/structure changes,
scroll/resize, drag start, window blur and disposal.

No dependency, schema/storage/domain/KNX protocol change, no arbitrary palette
CSS and no private image publication. Theme v1 / palette1.1.0 are unchanged.
Exact #003300, Save-only purple, extra input focus glow, broader component
styling and native WebKitGTK/Orca/full-WCAG acceptance remain separate.

## Tests and diagnostics

TDD covered admission, lifecycle/cancellation, clipping, CSS motion guards and
execution of the actual pre-mount bootstrap. Initial tests failed before their
implementation. An intermediate bootstrap variable-name inconsistency introduced
in this task was corrected; it was not a preexisting repository bug.
A fixed portal initially needed explicit ancestor clipping, now regression-tested.
Browser verifier fixes were test-assumption repairs (real open flow, optional
expectedSettings, correct row locator, scrolling the zoom target, label-associated
installation toggle rather than globally indexed root toggle).

The in-session review found redundant new keyboard handlers on components whose
ancestors already owned delegated navigation. Those handlers/helper/tests were
removed, preserving existing keyboard behavior instead of adding a second owner.
The browser assertions now exercise those real production paths.

Final reviewed/frozen-source gates:
- npm test: 1739 passed / 98 files.
- npm run build: TypeScript and Vite pass (136 modules transformed).
- node --experimental-strip-types design/verify-crt-interactions.mjs: 12 actual-App
  Chromium groups pass; zero page errors, unexpected requests and real backend
  requests; two synthetic Save refusals intercepted.
- git diff --check and verifier JavaScript syntax check pass.
- Narrow secret-literal/dynamic-eval scan of owned controller/hook/verifier: no hits.
- Final fixed-viewport PNG visually inspected: selected first row, visible light on
  row1/0/9, readable inspector and unobstructed footer. The image samples a paused
  native125ms frame; ordinary expiry/cancellation was tested separately in real time.

The verifier loads only a synthetic project via intercepted real UI file-picker/
open flow. Settings/discovery/Save requests are intercepted, all unknown API
requests blocked. No actual filesystem/project/backend/settings/KNX operation.
Reproduce using the no-proxy local Vite fixtures config described in the CRT guide.
The script is executable evidence, not a miniature substitute component fixture.

## Delivery state

Source and production screenshot/receipt remain local and uncommitted in the
feature worktree. The prior design branch is already published but this feature
is not. Root product/foreign changes are preserved; only its additive handover
pointer is updated. Own dependency/build/scratch/runtime cleanup is finalized in
CURRENT_STATE.md after checking the fixture process and listener actually stop.
The .ai/logs directory is ignored: a later authorized commit must explicitly
include only this owned log, not force-add the directory or unrelated history.

Cleanup finalized at2026-10-03 22:38 CEST: manager kill initially left owned npm
PID4163503 alive. Its command/CWD were revalidated before SIGTERM; the PID is now
absent and loopback4173 has no listener. Own node_modules/dist and12 explicitly
named scratch files were removed; source, verifier, PNG, receipt and this log
remain. Receipt totals, final gate logs and linked artifact paths were checked
programmatically before temporary logs were removed. Both handovers are additive.

Documentation: ADR-0022 follow-up, DESIGN_RETRO_GREEN_CRT.md, THEME_PACKS.md,
IMPLEMENTATION_STATUS.md, KNOWN_LIMITATIONS.md and ROADMAP.md.
Evidence: design/retro-green-crt.production.receipt.json and production.png.
Remaining acceptance/publication is separately scoped; no main integration claim.
