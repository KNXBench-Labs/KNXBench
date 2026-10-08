# KNXBench Alpha — brief for the independent whole-product review (AR18)

You are reviewing **KNXBench**, a Linux-first KNX engineering application (an
independent alternative to ETS), for its first public pre-release
`0.1.0-alpha`. The release owner prepared the candidate; **you did not build
it and owe it nothing**. Your verdict decides whether the user may tag it.

The user started you in a fresh session on purpose: you have no history with
this code. Keep it that way — judge from the repository, the running program
and your own measurements, not from the project's own status claims.

## 1. The candidate

| Item | Value |
| --- | --- |
| Repository | `/mnt/daten-i/Sourcecode/KNXBench` (remote `origin`, branch `main`) |
| Candidate revision | The `origin/main` commit that added this brief. Its code is the gated `4b9e913e5ee2d5241bbb06ca932f02bf29a4e888` plus one test-only fix; verify with `git diff --name-only 4b9e913e HEAD` (documentation, handover files and `apps/knx-server/tests/http_device_compare.rs`) |
| AppImage | `/home/knxbench/.hermes/profiles/knxbench/evidence/alpha-release/ar18-candidate-20261006/KNXBench_0.1.0-alpha.4_amd64.AppImage`, SHA-256 `70bbb6b640dec6d77340c702dc4e1baad6b46c1e6516c6523bd74b898f72f81b` (copy it before you run it; do not modify the evidence directory) |
| Versions | CLI, desktop, web `0.1.0-alpha.4`; server `0.1.0-alpha.1` (each program counts separately, ADR-0018) |

Work in your own Git worktree of that revision:
`git fetch && git worktree add --detach ../KNXBench.worktrees/alpha-review <candidate>`.
Use your own scratch directory and a fresh `CARGO_TARGET_DIR` (the repo's
`xtask` gates bake their checkout path in at build time — a shared target
directory makes them audit the wrong tree).

## 2. Ground rules

1. **Read-only on product code.** Do not fix anything. A finding is a
   finding; the owners fix it and you (or a new reviewer) look again.
2. **No live KNX bus, no hardware writes.** Never run ignored tests that need
   a gateway or device, never set gateway/device environment variables. Run
   test and browser suites without network:
   `unshare --user --map-root-user --net sh -c 'ip link set lo up && exec "$@"' sh <cmd>`
   (keep `TMPDIR` short, e.g. under `~/.hermes/profiles/knxbench/cache/scratch/rv`,
   or Chromium fails on its socket path).
3. **Private corpus.** `OriginalData/` (gitignored, only in the root checkout)
   holds real customer projects. Use it only read-only and only report
   aggregates — never names, ids, paths or values. Never copy it. Corpus tests
   may be linked into your worktree with a symlink; remove it afterwards.
4. **One heavy Cargo run at a time** across all sessions. Wrap every workspace
   build/test in both leases, in this order:
   `flock /mnt/daten-i/Sourcecode/KNXBench/.git/knxbench-alpha-gate.lock flock /mnt/daten-i/Sourcecode/KNXBench/.git/knx-workspace-gates.lock <cmd>`.
5. **No subagents** (user rule for this repository). Do the review yourself.
6. **Treat documentation as claims.** `docs/IMPLEMENTATION_STATUS.md`,
   `docs/status/LEDGER.md`, `.ai/`, receipts and dossiers were written by the
   people being reviewed. Verify what matters; say when you only read it.
7. Commit identity if you commit your verdict file:
   `-c user.name='KNXBench' -c user.email='github@knxbench.com'`, no
   co-author lines. Do not push to `main`; push a branch `review/alpha-independent`.

## 3. What to review

Priorities of the project, in order: **correctness → data integrity →
compatibility → maintainability → UX → performance.**

1. **Gates, measured by you** on the candidate (exit codes *and* counts;
   ignored/skipped separately). Build the web app first
   (`cd apps/knx-web && npm ci && npm run build`), then:
   `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets -- -D warnings`;
   `cargo test --workspace --no-fail-fast` (offline);
   `cargo run -p xtask -- check-layering | check-headers | check-anchors | check-ledger | check-corpus-gates`;
   `cargo deny check`; in `apps/knx-web`: `npx tsc --noEmit`, `npx vitest run`,
   `npx playwright test` (offline);
   `cargo run -p xtask -- check-appimage --artifact-dir <dir of the AppImage>`.
2. **Data integrity of ETS import** (`crates/knx-etsproj`, `crates/knx-app/src/import.rs`,
   `crates/knx-productdb`, `docs/IMPORT_EXPORT.md`, `docs/COMPATIBILITY.md`): is anything silently
   dropped? Are unknown elements preserved or reported? Try malformed,
   truncated, duplicate-id and password-protected inputs you build yourself.
   Run the corpus-gated import tests (`cargo run -p xtask -- check-corpus-gates`
   lists the conventions) from a worktree with the corpus linked.
3. **Project store** (`crates/knx-store`): schema 10 migrations, atomic
   upgrade, save/load roundtrip, refusal paths leave files untouched.
4. **Writes to real devices** (`crates/knx-core/src/commissioning`,
   `crates/knx-net/src/commissioning*`, `crates/knx-app/src/device_download.rs`
   and the recovery modules, `knx device …`, `docs/KNOWN_LIMITATIONS.md` §7): plan first, device-specific
   confirmation phrase, backup before write, refusal before a tunnel opens
   where documented. Verify by reading code and running the simulator tests —
   not on hardware.
5. **KNXnet/IP and DPT codec** (`crates/knx-net`, `crates/knx-core/src/dpt`):
   framing, sequence handling, timeouts, codec widths against the standard
   where you can check them. KNX Secure (`crates/knx-secure`) is in scope as
   far as the docs claim it.
6. **Layering and boundaries**: KNX core independent of UI; importers do not
   dictate the domain model; no manufacturer products hard-coded.
7. **The application as a user meets it**: run the AppImage offline with the
   fictional sample (`tools/manual_sample_project.py` generates one), and the
   web build; compare with the manual (`docs/manual/`, accepted 2026-10-06 in
   `docs/archive/alpha-0.1/MANUAL_ACCEPTANCE.md`). Note that the AppImage needs an X server
   (accepted boundary KL-158; a Wayland recipe is in `docs/archive/alpha-0.1/ALPHA_CANDIDATE.md`).
8. **Privacy and packaging**: no corpus, secrets or maintainer files in the
   AppImage; the known build-path strings (`docs/archive/alpha-0.1/ALPHA_CANDIDATE.md` §4) —
   judge whether they block a public release.
9. **Honesty of claims**: README, manual and `docs/archive/alpha-0.1/ALPHA_SCOPE_MATRIX.md` must
   not claim more than the code does (no "full ETS compatibility", no
   verified hardware path beyond what was verified).

### What the release owner already measured

[ALPHA_FINAL_GATES](../ALPHA_FINAL_GATES.md) is the release owner's own gate
dossier for `4b9e913e`. Reproduce what you rely on rather than quoting it.
It records two corpus tests in
`apps/knx-server/tests/http_device_compare.rs` that failed with 503 "activity
history unavailable" and were then fixed in the test harness only. The
dossier calls this a stale test harness, not a product defect. Judge that
claim yourself, including whether the fix hides a real compare/download
problem. It also records the AppImage built with `--remap-path-prefix`, which
removes the builder's home directory from the binaries.

Feature acceptances made by the release owner (self-reviews, not independent),
which you should look at again: the telegram-flow view (AR21,
`docs/TELEGRAM_FLOW_VISUALIZATION.md` §13–§22, accepted on a Chromium-only,
Motion-Off-for-large-maps envelope), the manual
(`docs/archive/alpha-0.1/MANUAL_ACCEPTANCE.md`), the declared-versus-linked group-address type
(`KL-61`, ADR-0078), and the scope matrix (`docs/archive/alpha-0.1/ALPHA_SCOPE_MATRIX.md`).

## 4. Accepted boundaries — not findings by themselves

The user accepted these for the Alpha on 2026-10-06 and the earlier owner
receipts; report them only if the code or the docs **misstate** them:
`docs/archive/alpha-0.1/ALPHA_SCOPE_MATRIX.md` (all `ACCEPTED_BOUNDARY` / `USER_ACCEPTED` rows),
in particular `KL-1`, `KL-11`, `KL-125` (other ETS schemas / AES projects),
`KL-31` (live routing multicast), `PDB-01`, `R-DYNAMIC-01`, `R-MODULE-03`,
`R-MODULE-04` (parameter logic, nested modules), `KL-158` (AppImage needs X),
plus the UI owner's native WebKitGTK, screen-reader and live-bus exceptions.

## 5. Your output

Write `docs/review/<date>-alpha-independent-review.md` (and say the same in
your final answer):

1. **Verdict:** `READY`, `READY_WITH_CONDITIONS` (list them) or `NOT_READY`.
2. **Findings**, each `CRITICAL` / `IMPORTANT` / `MINOR`, with `file:line`,
   a concrete failing scenario and how you established it (ran / read).
   CRITICAL = data loss, unsafe device write, false compatibility or safety
   claim, release-blocking crash. IMPORTANT = must be fixed or explicitly
   accepted by the user before tagging.
3. **Gate table**: command, exit, counts, your revision.
4. **Coverage**: what you checked deeply, what only by reading, and what not at
   all. An honest "not checked" is worth more than a vague "looks fine".
