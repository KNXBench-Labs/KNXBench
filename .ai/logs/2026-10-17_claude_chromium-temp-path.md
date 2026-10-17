# 2026-10-17 — Claude: recurring Chromium "socket path too long"

## Symptom

Browser runs kept failing before any assertion with
`explicit browser temporary root exceeds the Chromium socket budget`
(or Chromium's own `Socket path too long`). Each session then hand-built yet
another short TMPDIR in its private launcher.

## Root causes (reproduced RED before the fix)

1. **Leaf vs. parent confusion.** `tools/browser_tmp.py` budgets
   `root + /wXXXXXXXX + /org.chromium.Chromium.XXXXXX/SingletonSocket`.
   A private explicit directory that already fits Chromium as the *leaf* was
   refused because one more `/w…` component would not fit. Launchers that had
   already allocated a short private directory and passed it as
   `KNXBENCH_BROWSER_TMP_ROOT` therefore failed (observed in a parallel
   session's browser1 receipt).
2. **Standalone launchers were unguarded.** `workbench-browser-proof.mjs`,
   `design/verify-crt-*.mjs` and `story/tests/browser/check_story.mjs` launched
   Chromium directly with whatever TMPDIR they inherited.
3. The guard lived in a `.ts` file, so plain Node scripts could not load it.

## Change

- `tools/browser_tmp.py`: `allocate_browser_directory`/`browser_directory`
  reuse a validated private (uid-owned, 0700) explicit leaf without nesting;
  never delete caller-owned data; `--allocate` JSON protocol; `--run CMD…`
  wrapper with exit propagation, signal forwarding and post-shutdown cleanup.
- `tools/browser_tmp.cjs` (+ `.d.cts`): the single Node guard, resolved relative
  to itself; exports `KNXBENCH_BROWSER_TMP_DIR` so nested processes reuse the
  parent's leaf. `apps/knx-web/browserTmp.ts` now reexports it.
- Standalone launchers import the guard before Playwright.
- `run_web_gate.py` uses the shared allocator; fixture fingerprint includes the
  CJS guard (named RED first).

## Verification (actually run)

- RED: 8 named failures in new `test_browser_tmp_launch.py`, fingerprint RED.
- GREEN: 198 tool tests (1 pre-existing skip), incl. 14 temp regressions with
  real offline Chromium launches at the leaf boundary and from a deep root;
  run both directly and nested under `browser_tmp.py --run`.
- Production build, all 20 derived tsconfig typechecks, docs checker,
  `node --check` on all touched launchers, `git diff --check`.
- Real Playwright: `instance-values` + `raw-log` specs 8/8 with a 208-byte
  TMPDIR in an isolated network namespace; caller data intact.

Not run: full web gate, Rust workspace, private corpus (no Rust/app code changed).
