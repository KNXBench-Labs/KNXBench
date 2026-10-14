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
Outputs must be a new directory outside sources or below a Git-ignored directory.
An existing receipt path is refused. No silent npm installation occurs in a gate.

## Coverage and execution invariants

- Application tsc stays in npm run build. Every tsconfig.*.json is retained once;
  the extra app/flow/theme invocations are removed, not their coverage.
- Original test selection, server/locale settings and every browser assertion are
  unchanged; all Playwright entries now import the common temp guard.
- Source-hash admission: 45 reviewed, independently page-scoped functional files
  can use multiple workers. All other files run once with one worker.
- New or edited files fall back to exclusive execution. Changed shared fixtures,
  configs or package lock revoke all parallel admission. Deleted policy entries
  are a visible policy error, not an omitted test.
- Compile the entire default case inventory before execution. Require exact case
  identity equality for each lane, a nonempty intended scope, passed expectations,
  one successful result, retry zero and no skips. Lane union preserves all cases.
- Dedicated production modes remain separate and serial: catalogue,
  communication, device-download-support, devices, dialogs, ga-members-download,
  group-addresses, instructions, notes, parameters, reload-guard, structure,
  task-status, updates and ux-feedback. No dev-versus-production deduplication.
- Existing common-directory workspace/alpha/browser leases are respected in fixed
  order and retained for the whole package in this first version. Shorter locks,
  cross-suite parallelism, Rust/nextest/corpus work and GPU work are deferred.
- Browser launch is probed before large suites. Network namespace is isolated,
  loopback brought up, and no exported KNX gateway, capture or mutation switches
  reach the browser suite. Canonical fixture port 4173 is unchanged; no listener
  reuse. Per-stage output paths avoid collisions.

## Recurring Chromium socket-path protection

Python 3.10+ is required for these Linux developer test entries. All thirty
Playwright configs import `browserTmp.ts`, so direct `npx playwright test` gets
the same protection as the Python gate. The helper delegates root selection to
`tools/browser_tmp.py`; it works with CJS and ESM config loaders, allocates only
its own private temporary directory and cleans that directory on normal process
exit. In the measured gate it reuses the validated parent-managed mode-0700
directory, so cancellation cleanup remains with the Python context manager and
no extra socket-path component is added. Direct forcible termination can leave
its own private directory; it never removes the caller's original directory.
Application production code
and the base app type surface do not import this Node-only helper; a separate
browser-tooling tsconfig checks the config/helper code using already locked types.

The selector budgets Chromium's actual socket suffix plus a fresh temporary
component against Linux AF_UNIX's 107-byte pathname limit. A short supplied root
is kept. For a deep root, **only** a canonical ancestor named `scratch`, owned by
the current user and not group/world writable, may be selected automatically.
It never climbs to an arbitrary cache/home/system temp directory. If none fits,
it refuses before the suite rather than attempting hundreds of failed launches.
Use `--temp-root PATH` with the Python gate or `KNXBENCH_BROWSER_TMP_ROOT=PATH`
with direct Playwright for an explicitly approved short root. The value must
exist and fit; an invalid explicit root is not silently replaced.

Run direct Playwright from the workspace/app directory, or set
`KNXBENCH_TEST_ROOT` explicitly when the config is launched from elsewhere.
The gate binds that workspace itself. Temporary/root variables do not authorize
network or device operations; existing hardware gates are unchanged.

Regression evidence includes all-entry wiring, missing/long-root refusal, an
actual scratch-located CJS config loader with a deep path (owned mode 0700 and
cleanup), and a real Chromium launch from a 245-byte requested TMPDIR relocated
to a 59-byte owned path. The original directory and its sentinel survived.
No persistent profile/OS settings, unsafe Chromium flags or `/tmp` workaround
were introduced in this environment.

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

Primary runner references: Playwright [parallel execution](https://playwright.dev/docs/test-parallel)
and [JSON reporter](https://playwright.dev/docs/test-reporters#json-reporter),
retrieved 2026-10-14; actual installed 1.63.0 behavior also exercised.
