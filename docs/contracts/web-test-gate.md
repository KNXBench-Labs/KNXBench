# Measured web test gate

## Scope

Linux-first developer tooling, not an application/KNX-domain change. The web profile
is a complete declared web/docs package scope, **not** the Rust workspace, private
corpus, real-device acceptance, platform certification or a release gate.

## Entry points

From apps/knx-web, after npm ci:

- npm run test:e2e: one app build plus complete default browser inventory.
- npm run test:gate: measured web/docs package profile.
- npm run test:benchmark: two executions each at 1, 2 and 4 workers; functional
  lane only, counterbalanced order 1/2/4/4/2/1. Not full acceptance.

Root entry: python3 tools/run_web_gate.py --profile web --workers 4 --output PATH.
The change-scoped package gate (`tools/run_package_gate.py`, [ADR-0144](../adr/0144-change-scoped-gates-tagged-browser-lanes-bundled-rust-tests.md))
calls `--profile targeted --specs a.e2e.ts,b.e2e.ts|all --production x,y`: build,
every typecheck and the complete Vitest suite, then only the selected default
spec files and production suites. `--profile web` remains the complete web gate
of a release.
Outputs must be a new directory outside sources or below a Git-ignored directory.
An existing receipt path is refused. No silent npm installation occurs in a gate.

**Rust stages after the web gate (17 October 2026).** This profile's `build`
stage leaves `apps/knx-web/dist` in place, and that bundle is what
`knx-desktop`'s Tauri build script needs. A package gate therefore runs the web
profile (or `npm ci && npm run build`) before workspace Clippy and tests, which
then include `knx-desktop`; excluding the crate is not an accepted shortcut.
The order is the [Rust workspace gate](../VERIFICATION.md#rust-workspace-gate)
and is held by `tools/tests/test_rust_gate_order.py`.

## Coverage and execution invariants

- Application tsc stays in npm run build. Every tsconfig.*.json is retained once;
  the extra app/flow/theme invocations are removed, not their coverage.
  `tsconfig.workbench-fixtures.json` includes the existing workbench-splitters
  browser spec and its imported helpers; runner discovery/inclusion has a
  regression in `tools/tests/test_web_gate.py`. This adds static coverage,
  not another browser execution or a change to test selection.
- Original test selection, server/locale settings and every browser assertion are
  unchanged; all Playwright entries now import the common temp guard.
- Lanes by tag (ADR-0144, replaces the per-file hash admission): every case runs
  with several workers unless it carries `{ tag: "@exclusive" }` on the test or
  an enclosing `test.describe`; tagged cases run once with one worker.
  `tools/browser_lanes.py` (gate stage `browser-lanes`, before the build) refuses
  an untagged use of a real-time primitive (`waitForTimeout`, `performance.now`,
  `Date.now`, `new Date().getTime()`, `requestAnimationFrame`, `setTimeout`,
  `setInterval`, also in string code handed to `page.evaluate` and its
  relatives, and Node's timer modules under any name), also through module,
  describe-local and imported test-side helpers below `e2e/` (named, default,
  namespace, dynamic and side-effect imports, re-exports, object and class
  methods, solved as one fixed point). A primitive in a helper module outside
  any named function, for example in a Playwright fixture or a hook registered
  on import, is refused in the helper; a test-side `.tsx`/`.jsx` import is
  refused. Application modules are not followed. Destructured clocks,
  re-bound aliases, getters and bracket calls are known blind spots (ADR-0144).
  The `--grep` for the tag matches the whole tag (`@exclusive(?![\w-])`).
- Compile the entire default case inventory before execution and split it by
  tag. Require exact case identity equality for each lane, passed expectations,
  one successful result, retry zero and no skips. Lane union preserves all cases.
  A lane without cases is recorded as `empty` with the inventory as its reason;
  it is never asked to run nothing.
- Dedicated production modes remain separate and serial: catalogue,
  communication, device-download-support, devices, dialogs, ga-members-download,
  group-addresses, instructions, notes, parameters, reload-guard, structure,
  table-columns, task-status, updates and ux-feedback. Each runs once with one
  worker, as its config pins. No dev-versus-production deduplication.
- Existing common-directory workspace/alpha/browser leases are respected in fixed
  order and retained for the whole package in this first version. Shorter locks,
  cross-suite parallelism, Rust/nextest/corpus work and GPU work are deferred.
- Browser launch is probed before large suites. Network namespace is isolated,
  loopback brought up, and no exported KNX gateway, capture or mutation switches
  reach the browser suite. Canonical fixture port 4173 is unchanged; no listener
  reuse. Per-stage output paths avoid collisions.

## Recurring Chromium socket-path protection

Python 3.10+ is required for these Linux developer test entries. Every maintained
Playwright config imports `browserTmp.ts`, which reexports the shared Node guard
`tools/browser_tmp.cjs`. The same guard is loaded before Playwright by the
standalone workbench, CRT and story verification launchers; no TS loader is
needed for those scripts. Root resolution follows the loaded helper's checkout,
not the caller's working directory. All allocation and byte-budget decisions
remain in `tools/browser_tmp.py`; there is no second JavaScript path policy.
Application production code and the base app type surface do not import the
Node-only helper. A separate browser-tooling tsconfig checks the config/helper
interface using already locked types.

The selector budgets Chromium's socket suffix plus a fresh temporary component
against Linux AF_UNIX's 107-byte pathname limit. A short supplied root is kept.
For a deep root, **only** an ancestor named `scratch`, owned by the current
user and not group/world writable, may be selected automatically. It never climbs
to an arbitrary cache/home/system temp directory. If none fits, it refuses before
the suite rather than attempting hundreds of failed launches. Use `--temp-root
PATH` with the Python gate or `KNXBENCH_BROWSER_TMP_ROOT=PATH` with direct
Playwright for an explicitly approved root. Missing roots, non-directories and
paths that cannot fit even as a leaf still fail closed; no silent fallback from
an invalid explicit root is introduced.

**Symlinked scratch (17 October 2026):** the configured path is searched before
its canonical form, nearest `scratch` first. A `scratch` component may therefore
be a symlink (for example a scratch moved to another disk). The link itself must
belong to the current user, and its canonical target must be an owned directory
that is not group/world writable; the target, not the link, is used and budgeted.
Previously the canonical lookup lost the `scratch` name and refused every deep
`TMPDIR` below such a link. The browser temp tests build their own anchor
(`tools/tests/browser_tmp_fixtures.py`) instead of depending on one in the
runner's `TMPDIR`.

**Private leaf versus allocation parent (17 October 2026):** an explicit root can
fit Chromium's socket but fail the larger budget for an additional `/wXXXXXXXX`
component. In that case the allocator validates current-user ownership and mode
0700 (or stricter) and uses the existing leaf without creating another directory.
It never deletes caller-owned data. Short allocation parents still get a fresh
private directory. The Python gate uses the same context-managed allocator;
its receipt's `temporaryRoot.selected` is the actual browser directory. The Node
guard exports `KNXBENCH_BROWSER_TMP_DIR` to child processes so their loaders reuse
that validated directory, with cleanup owned by the allocating parent only.

For one-off Python/Node/browser-CLI launchers, from the repository root:

```sh
python3 tools/browser_tmp.py --run COMMAND ARGUMENTS...
# Or preload just the shared Node guard, including for CJS scripts:
node --require ./tools/browser_tmp.cjs SCRIPT ARGUMENTS...
```

The command wrapper waits for its child process, propagates its exit status,
forwards SIGTERM/SIGINT/SIGHUP to that owned process group, and cleans its own
directory after child shutdown, including nonzero exits. It is intended for
foreground test commands, not daemon supervision. It does not itself provide
network isolation or permission for hardware operations: retain the existing
namespace, leases and bus safety gates around the wrapped command. For dynamic
launchers, the Python `browser_directory` context manager exposes the same
allocation/cleanup policy; `--allocate` is a structured JSON protocol for a parent
that assumes cleanup ownership, not a shell-substitution shortcut.

Direct Node/Playwright normal exit removes only directories created by that
process. Python context cleanup covers ordinary failure and managed signal
shutdown. SIGKILL/host failure can still leave an owned directory; neither helper
claims cleanup after forcible termination. Never remove a live parent's leaf.

Regression coverage includes all-config/standalone wiring, the exact explicit-leaf
refusal reproduced before the correction, actual CJS config loading, nested Node
inheritance, UTF-8 byte versus character limits, newline-safe JSON paths,
non-private/missing/oversize-root refusal, nonzero-command and SIGTERM cleanup,
and an offline real Chromium persistent-context launch at the validated pathname
boundary. Original caller-directory sentinels survive. A launch probe verifies
browser setup only, not application behavior or the entire UI suite. No persistent
profile/OS settings, extra unsafe Chromium flags or system-temp fallback are
introduced.

## Receipt

receipt.json is checkpointed atomically after real commands. It records the
expected stage plan, separate queue wait, command/cwd/actual exit, wall seconds,
wait4 CPU seconds and peak RSS, compiled browser counts/IDs in retained reports,
Git-selected source commitment, installed locked tool versions, actual Node/
Chromium versions and production dist hash at start/end. Incomplete, interrupted,
changed-source/build/tool or failed-proof runs are not success. Positive freeze
metadata is never classified as a failed command.

Peak RSS is the Linux wait4 high-water accounting for the waited subprocess tree,
not the sum of concurrently resident Chromium workers; do not use it to claim total
suite memory consumption. Installed dependency versions are bound, not every
byte of node_modules or a maliciously modified system tool. No universal machine
attestation is claimed. Host load is recorded; performance remains host-dependent.

SIGTERM/keyboard interruption terminates only the owned child process group,
keeps a failed checkpoint, releases held leases and cleans the owned short temp
directory. SIGKILL/host failure can leave an incomplete receipt; it is never green.

**Interrupt at the PID checkpoint (17 October 2026).** A stage records its
child's PID in the receipt before waiting. That checkpoint used to sit *before*
the guarded block, so a signal arriving right after the receipt was written —
exactly when a supervisor reading the receipt reacts — left the child's whole
session running with no owner. `test_sigterm_terminates_the_owned_child_group`
failed 14 of 15 isolated runs on an idle machine for this reason and passed
under load. The checkpoint now runs inside the guarded block;
`test_interrupt_at_the_pid_checkpoint_still_terminates_the_child_group` raises at
that exact point and fails on the old code. The SIGTERM test then passed 25 of 25.

## Evidence and admission

The initial launch-path refusal remains failed. A corrected real launch probe is
not application acceptance. Scaling requires exact equal-case successful runs,
then a complete web-profile branch and merged-result gate on frozen inputs. The
measured worker default is at most four, reduced to two/one when process CPU
affinity is smaller. Container CPU quotas are not inferred; use `--workers`
explicitly where needed. The complete matrix passes with 276 cases per execution;
retain a one-worker option and immutable failed reports. No assertion, timeout
or retry weakening.

Measured matrix: [aggregate receipt](../evidence/web-test-optimization-2026-10-14.json).
The functional-lane mean is 331.18/195.66/165.15 seconds for 1/2/4 workers.
The approximately 2.01× improvement applies to that lane on this host, not the
whole gate or other CPUs. The matrix predates the owner's recurring-temp-guard
expansion: test/fixture bodies and original config settings remain identical, but
all configs now load the common guard. Final expanded-source web acceptance is
recorded separately, not retagged as that earlier matrix. Original failures and
the intentionally superseded branch attempt remain retained.

Final expanded-source branch and merged web/docs gates each passed all sixty
stages. Merged runtime source is `5db49560`: 554 default cases, 313 production
cases and 3,559 Vitest cases. The initial merged exclusive stage briefly overlapped
an unsuccessful evidence-archive attempt, so its duration is not clean performance
proof. A separately source/build/tool-bound followup reproved the exact 278
exclusive cases under the shared leases, with zero retries/skips. This is a scoped
followup, not another full gate or scaling benchmark.

**Lanes by tag (19 October 2026).** The hash admission closed again within a day
of the 17 October re-admission (21 of 45 admitted specs and the shared fixtures
changed), so every gate ran all 713 default cases on one worker: 1,172.9 s in
the `browser-exclusive` stage of a full package gate on 19 October, plus 505.6 s of
serial production suites (inventories included). The review criteria were mechanical (no fixed port,
no file outside Playwright's output, no environment switch, no shared server
state, no real-time budget), so the owner replaced the review by the tag guard.
Every spec was scanned: 35 test declarations in 17 files measure real time and
carry the tag; the guard reports 0 untagged uses. Measured results are in the
[test-suite speed dossier](../research/2026-10-19-test-suite-speed.md).

**Re-admission (17 October 2026, superseded by the tag lanes).** The temp-path tooling changes of 17 October
moved the fixture fingerprint, so every file ran in the exclusive lane. The
[lane review](../evidence/browser-lane-review-2026-10-17.md) classified every
shared-input and spec change since the 14 October admission, re-admitted the
same 45 files with their current hashes (none added) and recorded six measured
benchmark runs with identical 276-case identities, no skips or retries.

Primary runner references: Playwright [parallel execution](https://playwright.dev/docs/test-parallel)
and [JSON reporter](https://playwright.dev/docs/test-reporters#json-reporter),
retrieved 2026-10-14; actual installed 1.63.0 behavior also exercised.
