# Flow Visualizer usability follow-up — implementation receipt

Agent: codex (Hermes), 2026-10-07. User explicitly approved the grill-me synthesis
and implementation. Self-review, not independent review. Delivered as local
uncommitted changes in the main checkout; no commit/push/release/deployment.

## Delivered

- Measured responsive canvas/growing world; no empty Inspector reservation;
  closable details and maximized view with keyboard confinement/Escape restore.
- Readability-first component/rank layout, compact wide-star placement, automatic
  structural rearrangement, explicit Rearrange, Freeze, routed curves/pulses.
- Initially-on switchable auto zoom, manual override, one-shot Show all including
  curve/label bounds. Camera/selection survive main/table/diagnostic navigation.
- Dedicated browser/Tauri Flow role. Already accumulated graph and updates arrive
  from the source via a same-origin channel; no second poll/tunnel/editor.
- Exact device/GA links to the main editor, including Inspector value/connection
  rows; project scope, uniqueness, server incarnation/revision and async identity
  guards. Source/satellite link failures explain unavailable targets.
- Original age/expiry retained across document clocks; same-session cloned models
  do not replay pulse events; delayed snapshots cannot renew a live-source claim.
- Source loss preserves an explicit non-live map with disabled links. Capture-gap,
  pruning, stale/unverified context, error/end diagnostics survive maximization
  and window sharing. Hidden renderers stop frames/refresh timers.
- LAN-safe opaque UI identities use random bytes, not secure-context-only UUIDs.
  Existing UUID-formatted load-client tokens were kept unchanged.
- ADR-0085, Architecture, Flow §23, KL §154, Roadmap, implementation status and
  user guide updated. No new dependency, core/server/storage/schema change.

## Real execution

Main checkout `/mnt/daten-i/Sourcecode/KNXBench`, base/head unchanged:
`1df94bedb15c0a1d57b483189e1195fb1bdc75a1`.

- Root Vitest: 2,162 passed, 125 files; root `npm run build`: exit 0.
- Full root Chromium: 155 passed in a loopback-only namespace, including nine
  added end-to-end cases and real-parent device/GA navigation/replacement races.
- Native `cargo check -p knx-desktop`: exit 0 on the identical candidate sources;
  browser/native adapter creation/focus/refusal unit controls pass.
- Five xtask checks, explicitly runtime-targeted at the root: all exit 0;
  `git diff --check`: exit 0. Header initially refused one 105-column purpose
  sentence; shortened and rechecked. Security scan: zero credential literals,
  unsafe eval/raw HTML/shell-injection matches in added app lines/new files.
- 41 task source/doc files transferred from the isolated worktree only after all
  root originals matched the base. All 41 root/candidate bytes compared equal;
  inherited GitHub/README work and handover entries preserved.
- Two production load samples: 232 nodes/239 edges at 200 telegrams/s with motion,
  main-thread share 0.523 / marker lag max 33 ms; 502 nodes/2,490 edges at 1,000/s
  with Motion Off, share 0.335 / lag 327 ms. One short sample each; no peer-transfer,
  long-session, native or all-hardware performance certification.

## Regression/root-cause evidence

Retained RED controls cover missing Inspector/camera/maximize controls, replay on
cloned same-session models, foreign-node routing, tall fan-out, delayed live claims,
LAN UUID absence and missing source warnings. The first browser attempt exposed
SVG intrinsic-aspect/ResizeObserver feedback: measured canvas height grew while
node coordinates stayed fixed, making pointer activation impossible. Constrain
canvas height/SVG flex basis; stable-rectangle and real-click guards then passed.
Changed legacy tests reflect agreed semantics (Freeze also with Motion Off,
retained hidden view/timer stop, actual session identity), not relaxed data checks.
All intermediate failed receipts remain distinct from the final green gates.

## Boundaries and lifecycle

No bus/hardware contact; the live house screenshot was input only. The already
running root knx-server was neither stopped nor replaced. No private project,
manufacturer corpus or credential touched. Existing Alpha tag/versions untouched.
No real WebKitGTK/Orca run; this host has no Xvfb/Weston for an isolated GUI test.
Dense fitted maps can have small text/crossings; detailed routing is reduced
visibly beyond 80 nodes/250 edges, without dropping recorded edges. Diagnostic-only
sources lack a bound editor scope; links refuse rather than guess. Source reload
is not persisted history and can require opening a fresh Flow window.

Execution logs/compact receipts: task-owned Hermes scratch `flow-ux/` (not Git).
Task worktree and its installed dependencies/build scratch are removed after
source/receipt preservation; no other worktree, server or scratch directory is
owned by this package. Code delivery remains local/uncommitted by instruction.

## Explicit publication/shutdown request (2026-10-07)

The user subsequently authorized commit/push and explicitly selected the running
application/server, not the agent session. Identified production Docker container
`knxbench`; project/activity GETs returned 401, so no authentication or credential
inspection was attempted. The user confirmed the project was saved and no device
programming was active. `docker stop --time 30 knxbench` succeeded, but the process
required timeout termination: readback `Running=false`, `Pid=0`, `ExitCode=137`.
Only that container was stopped; unrelated isolated test servers were left alone.
Publication preflight: origin/main and root HEAD both the original base; 41 owned
source/doc hashes still match final acceptance. Four foreign commissioning source/
doc changes plus shared handover were backed up and excluded from feature staging.
Feature publication verified: `938a73c498a45f355fa132944bfae6950910c1f0` on origin/main; local feature HEAD,
fetched tracking ref and live `ls-remote` agreed after push. The isolated publication
checkout was rebased over already-published unrelated Rust work and then a docs-only
ADR-0086 commit; the 41 feature source/doc hashes and product-code gate inputs did
not change. Fresh publication Vitest 2,162/125, Web build and doc/header gates pass.
Native compilation remains evidence on the original implementation candidate, not
new native runtime or certification of unrelated commissioning work. Metadata
closure records this already completed code publication.
