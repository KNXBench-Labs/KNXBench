# KNXBench goal — commissioning (T30 phase 3), and nothing else

Written 2026-09-28 against `main` at `a47d168`. Use this file as the
instruction passed to `/goal` in the **commissioning session**. Creating the
file does not start a run. Every device write still needs its own "go" (§1).

This file is the counterpart of `goal.md`. `goal.md` §0 excludes
commissioning and every write to real KNX hardware. This file owns exactly
that excluded part, and **only** that part. If an item would change something
`goal.md` owns, it does not belong here: see §5 for the boundary, and §6 for
how work is handed across.

## Where things stand (verified 2026-09-28, read from the repository)

- **Individual address.** A real IA write succeeded on 2026-09-26
  (`03e358f`): MDT push button `1.0.71` → `1.1.67`. The settling retry
  (`be91fe3`, IMPLEMENTATION_STATUS "`individual_address_write` waits once
  for a device still settling") is **simulator-verified only**.
- **Application download.** The first real download succeeded on 2026-09-28,
  merged as `95a862c`, on MDT *Taster 2-fach Plus* `1.1.67`, mask `0701h`,
  program `A-0027-15-0BAC`, fixed configuration "option C". Read-back matched
  all 1418 octets, and a bus-monitor functional check was run. See
  RESEARCH §19.4 and `.ai/logs/2026-09-28_claude_iaw-live-download.md`.
  - Its closing `A_Restart` got no `T_ACK`, so the executor reported failure.
  - The configuration is hard-coded in the doubly gated test
    `apps/knx-cli/tests/live_memory_download.rs`. No product command exists
    (CLI, server or UI).
- **The chain.** Product file → `knx_productdb::image::build_download_image`
  → plan → `knx_net::commissioning::run_memory_download`. It runs end to end
  in the simulator, and on hardware only through that one path:
  `hardware_write_is_authorised` allows `Download`, `Restart` and
  `IndividualAddressProgramming`. The property-based `Downloader` still
  refuses hardware.
- **Consent gate.** ADR-0040's programming consent (`useProgrammingConsent`,
  dialog, persisted stage) is built and tested, and nothing calls it yet.

---

## Standing requirements (user, 2026-09-28)

These apply to every package below that writes to a device, and to the
product commands in K4 to K6 in particular.

- **R1 — Status and data are shown while programming.** A download or
  programming run shows what it is doing while it does it: the current step
  (n of m), and every block of data that goes to the device (address and
  octets), with a running count against the total. A run that says nothing
  until it ends is not acceptable. The library side is
  `run_memory_download_observed` / `Progress` (K2); K4 prints it, K5
  shows it.
- **R2 — "Download" names its direction.** "Download" means **to the
  device**, and every output says so and names the target device. A file
  going from KNXBench to the user is *saved/exported*, never "downloaded".
  Definition and evidence: `docs/GLOSSARY.md`.

## 0. Scope: what this goal owns

1. **T30 phase 3.** Individual-address programming, application download,
   restart, recovery, and unload *if* the user asks for it, on real hardware,
   as product features rather than tests.
2. **The KNOWN_LIMITATIONS entries `goal.md` §0 excludes:** §7, §92, §93,
   §99, §101, §104, §105, §108, §109, §111, §112, §113, §114, §115 and §116,
   plus **§136**, which was added after `goal.md` was written.
3. **The commissioning rows of the shared documents.** This goal edits only
   these rows:
   - `docs/GAP_ANALYSIS_ETS.md` row **E1**;
   - `docs/ROADMAP.md`'s Session 7 commissioning paragraph ("Update,
     2026-09-20 — commissioning (T30)");
   - `docs/ROADMAP.md`'s decision row "Whether v1.0.0 writes to real KNX
     hardware at all (**T30 phase 3**)";
   - the KNOWN_LIMITATIONS entries in item 2;
   - `docs/superpowers/specs/2026-09-13-commissioning-download-design.md`;
   - RESEARCH §8.8.x and §19.x;
   - new ADRs for commissioning decisions.
4. **Code.** `crates/knx-core/src/commissioning/`,
   `crates/knx-net/src/commissioning/`, `crates/knx-productdb/src/image.rs`
   and `code.rs`, the `live_*.rs` tests, and the new CLI/server/UI entry
   points for programming (§3, K4 to K6).

---

## 1. Hardware rules (binding, and never relaxed by this file)

1. **`1.1.220` is an alarm panel.** Never read it, write it or include it in
   a scan range. `knx_core::EXCLUDED_INDIVIDUAL_ADDRESSES`/`ContactableAddress`
   enforce this. Every live test also asserts it itself.
2. **Every write needs a fresh, specific "go"** naming the device and the
   operation.
   - "Weiter", "continue" or a standing-goal continuation prompt authorises
     offline work only.
   - A go covers **one** attempt. After a failed write, re-running it needs a
     new go, even with the same plan and target.
   - Stop before the first writing frame and show the user the exact plan.
3. **Read-only work.** Active reads are approved for `1.1.24`–`1.1.32` and for
   `1.1.67` (the commissioning test device).
4. **Setup for a live run.**
   - The gateway accepts one tunnel only. Before any live run, check that no
     server, monitor or CLI of either session holds it.
   - Gateway and target come from environment variables; never commit a
     literal.
   - Destructive live tests are gated twice: `#[ignore]` **plus** an env var
     naming the target (the pattern of `KNX_DOWNLOAD_CONFIRM`).
5. **Report what the wire did.** An `Err` with `wrote: true` is "written,
   then failed", not "nothing happened".
   - Raw logs go to `OriginalData/DeviceBackups/` (gitignored).
   - Before every write to a device, take a read-only dump of its memory.
6. **Procedure.** Follow the `knx-live-bus-operations` skill, including its
   references (`hardware-write-gate.md`, `application-download.md`).

## 2. Operating rules

1. **Start.** Read `.ai/CURRENT_STATE.md` (the newest entry is at the top)
   and this file. Then read KNOWN_LIMITATIONS §7/§136, RESEARCH §19 and the
   last `*_claude_iaw-*.md` logs. Re-measure counts instead of quoting them.
2. **Isolation.**
   - Work in your own worktree,
     `/mnt/daten-i/Sourcecode/KNXBench.worktrees/iaw-<topic>`, on branch
     `iaw-<topic>`. Never work in the root checkout, which belongs to the
     goal.md session.
   - Scratch goes only in `~/.hermes/profiles/knxbench/cache/scratch/iaw/`.
   - Logs go only in `.ai/logs/YYYY-MM-DD_claude_iaw-<topic>.md`.
3. **Handover.** Write your entries into the `.ai/CURRENT_STATE.md` of your
   own branch, tagged "(iaw commissioning session)". They reach `main` by
   merge. Never write into the root checkout's working copy.
4. **Merging.** Merge your own branch into `main` yourself.
   - Immediately before the merge, rebase onto the then-current `main`.
   - For conflicts in `.ai/CURRENT_STATE.md`, `docs/IMPLEMENTATION_STATUS.md`
     and `docs/KNOWN_LIMITATIONS.md`, keep both sides. CURRENT_STATE entries
     go in chronological order.
   - A new KNOWN_LIMITATIONS number is the next free one at merge time;
     renumber as needed (as with §134 → §136).
5. **Workload.** No subagents (user decision 2026-09-28). Implementation and
   reviews run in this Claude session itself.
   - Both sessions share the same Claude 5h and weekly limits. At each work
     package boundary, ask the user for the current usage and pause before
     100 %.
6. **Gates.**
   - Only one workspace gate at a time across both sessions. Before
     `cargo test --workspace`/`clippy`, check with `pgrep -af cargo` that the
     other session is not running one.
   - Use a fresh `CARGO_TARGET_DIR` under `scratch/iaw/`, and delete it after
     the package.
   - Required: `cargo fmt --all --check`,
     `cargo clippy --workspace --all-targets -- -D warnings`,
     `cargo test --workspace --no-fail-fast`, and the xtask checks
     `check-layering`, `check-headers`, `check-anchors` and
     `check-corpus-gates`.
   - Add `git diff --check`, and `npx tsc --noEmit` and `npx vitest run` when
     `apps/knx-web` was touched.
   - Judge a gate by its exit status and the amount of work in its log.
7. **Evidence.** Every functional change gets a RED test first (simulator)
   and a mutation check of every new guard. Label every encoding or timing
   fact `[D]`/`[V]`/`[A]`. Never close a timing gap with a guessed constant.
8. **Commits.**
   - Author `github@knxbench.com`, no `Co-Authored-By`, humour welcome.
   - Push after each reviewed, merged and fully green package.
   - Do not refresh `stats.md` yourself; it is run from the root checkout
     (§6).
9. **Wording.** Claim nothing beyond "KNX-compatible". Claim no ETS
   compatibility and no certification.

---

## 3. Work packages, in this order

Each package ends with: tests and mutation check, gates, docs, merge, push
and a handover entry. **[W]** marks steps that write to a device and
therefore need their own go (§1.2). Everything else is offline, against the
simulator.

### K1 — Make the commissioning documents true again (offline, first)

Several passages still say that no device has ever received a write, which
has been false since 2026-09-26:

- the title and text of KNOWN_LIMITATIONS §92 ("has never addressed a
  device");
- GAP_ANALYSIS_ETS row E1 ("no real device has ever received a write",
  "stays open until a real device accepts a write");
- ROADMAP Session 7 ("no write has been sent to a real device");
- ROADMAP's decision row "Out of scope until test hardware exists".

Also check the older layers of KNOWN_LIMITATIONS §7 ("The application does
not program devices"). Correct each statement in place with a dated addendum.
Do not append a contradiction below it.

The decision row needs the user: on 2026-09-28 they authorised live writes to
`1.1.67`. Record that as a new, dated decision **only after they confirm
it**. Until then, mark the row "narrowed by the go for `1.1.67`, decision
pending".

### K2 — "Loaded, restart unconfirmed" as a separate outcome

`run_memory_download` currently cannot tell a download whose closing
`A_Restart` went unacknowledged from a failed one.

1. **Offline.** Search MP §3.7.3 and RES for whether a Basic Restart must be
   acknowledged at the Transport Layer. Reproduce the case in the simulator
   (RED).
2. **[W]** Take a frame trace of a closing `A_Restart` on `1.1.67`. This is
   a restart, so it needs a go.
3. Add an outcome of its own, carried as far as the report and every caller:
   data loaded, all load states `Loaded`, restart unconfirmed. **Never** send
   a second restart on our own initiative. Pitfall: an `Err` must not become
   "nothing happened".

### K3 — Download configuration from the project, not from the test

`ImageRequest { program_id, individual_address, values, links }` must come
from a project device:

- `values` from its `ParameterInstance`s (`RefId` → `raw`);
- `links` from its com-object/group-address links;
- `program_id` from its application program.

1. Establish whether the imported project for `1.1.67` carries these values
   and links. Do not assume it.
2. Add a pure mapping function (domain/application layer, no bus access) with
   tests. Refuse missing or contradictory data by name; never fill it with
   defaults.
3. Acceptance: the mapping, fed option C's project values, produces
   **exactly** the image of the hard-coded test (0 differing octets).

**Status 2026-09-28: done.** No imported project has `1.1.67`; KNXBench
built it (catalog → device → panel → link → Save As). `image_request` in
`knx-productdb`; acceptance test `project_download_request.rs`: same request,
0 differing octets. IMPLEMENTATION_STATUS 2026-09-28 "K3".

### K4 — Download as a CLI command, with a dry run

1. The default is a plan without a write: target, program, segments,
   changed octets, steps.
2. Writing needs the library's `WriteAuthorisation` confirmation phrase
   (ADR-0040 "Consequences": the CLI has no dialog). Excluded addresses are
   refused before the first socket opens.
3. Test against the simulator end to end.
4. Pitfalls: report `wrote: true` honestly, and include K2's outcome.
5. R1 and R2: while writing, print each step and each data block sent to
   the device; name the command and its output as a download **to the
   device** (e.g. `knx device download <address>`), with the target in
   every heading.

### K5 — Download in server and UI

1. **Decision first.** ADR-0040 leaves it open whether the server must demand
   proof of consent. Settle that with an ADR before the first programming
   route.
2. A server route or job for plan, execution and progress. The UI calls
   `useProgrammingConsent` and writes only on `true`.
   - R1: the progress view shows step n of m, the data blocks sent (address,
     octets) and octets sent against the total, live.
   - R2: the UI labels this "Download to device" / "In Gerät laden" and
     names the device. The File menu's "Download project" is renamed to
     *save/export* by the goal.md session (handed over, see GLOSSARY).
3. **Web boundary.** `goal.md` runs all `apps/knx-web` work as one serial
   chain (§12.3). Before starting K5's web part, announce it in your handover
   and check that the goal.md session is not in a web task. Otherwise wait.
   - Do not change the parameter editor (`ParameterPanel`, ISSUE-09) or other
     goal.md UI surfaces; only call into them.
4. KNOWN_LIMITATIONS §101 lifts, or gets its bound, once the UI shows a
   progress bound.

### K6 — Individual-address programming as a product command

1. CLI command and UI dialog with a loop "press the programming button on
   exactly one device". This lifts KNOWN_LIMITATIONS §116.
2. **[W]** Live-verify the settling retry (`be91fe3`) in the next
   programming-mode session.

### K7 — Live acceptance of the product path

- **[W]** Download `1.1.67` through **K4's command** (and K5's UI) with a
  configuration from the project that differs from option C. Check the
  function with a read-only bus monitor while the user operates the device.
  Afterwards the fixed test is no longer the only hardware write path.
- **[W], optional, user decision.** Recovery: re-run the same plan after a
  deliberately interrupted download. KNOWN_LIMITATIONS §7 says "a failed run
  undoes nothing".

### K8 — Rule on the excluded KNOWN_LIMITATIONS entries

For each of §93, §99, §101, §104, §105, §108, §109, §111, §112, §113, §114
and §115: either meet its "Lifted when" condition with evidence, or bring it
to the user as an accepted boundary.

- **Offline and actionable now:** §105, Transport Layer control frames at
  priority `SYSTEM`, with a cEMI test pinning the Ctrl1 octet.
- **Spec boundaries, likely to become user acceptances:** §108, §109, §99,
  §114.
- **Needs a use case or a feature:** §111 (unload IA), §112 (`A_Key_Write`),
  §113, §115.

### K9 — Scope boundary v1: [A] rules and device coverage (user decision)

KNOWN_LIMITATIONS §7 lists what the image builder refuses by name (modules,
`Property` placement, unions starting mid-octet, other parameter types,
`High`/`Alert`, `ReadOnInitFlag`), and the `[A]` rules with no PDF source
(`Mask`, `CompareProp`, the 12 s wait, the strict load record, no rollback).
There are also 40/310 `LdCtrlMerge` programs and 59 programs with steps
nothing executes.

Put one decision to the user: **what counts as done for v1?** The
recommendation is: mask `070nh` verified on one device, everything else
refused by name and documented. Also decide whether to keep the
property-based `Downloader` as simulator-only or retire it. Implement
further families only with corpus evidence plus a device.

### K10 — Close the commissioning track

1. A whole-track review in this session, adversarial, over `git diff`
   between the start and the end of this track.
2. Fix its findings.
3. Write a closing handover, including the items for the goal.md session
   (§6).

---

## 4. Completion condition

Finish only when:

- K1–K7 are done with evidence. Every **[W]** step has run with the user's
  go, or the user has explicitly deferred it.
- Every entry from §0.2 is lifted with evidence or accepted by the user as a
  boundary (K8/K9).
- The commissioning rows from §0.3 agree with each other and with the code.
- All gates from §2.6 are green on the merged `main`.
- The closing handover names the items for the goal.md session (§6).

Report at the end: what is done, the evidence, the deferred **[W]** steps,
and the next user decision.

---

## 5. Boundary with `goal.md` (no overlap)

| Topic | Owner |
|---|---|
| Group-value sends, bus monitor, scan, discovery, T17 diagnostics UI, E2 scan reconciliation (`goal.md` §3) | goal.md |
| `/api/project/download` (downloads the `.knxdb` **file**, KNOWN_LIMITATIONS §23/§30) | goal.md; this is not a device download |
| The ADR-0040 consent gate itself (hook, dialog, settings) | goal.md, built and closed. This goal only calls it; the server-side enforcement decision belongs to K5 |
| Parameter editor, device editor, catalog (ISSUE-07/08/09) | goal.md |
| Product-database import, schema, PDB-x | goal.md; this goal only reads the stored product file (ADR-0044) |
| `docs/LIMITATION_TRIAGE.md` recount (`goal.md` §8.5), `stats.md`, `goal.md` itself, the root `.ai/CURRENT_STATE.md` working copy | goal.md session |
| Commissioning rows and entries from §0.2/§0.3, code from §0.4 | this goal |
| The goal.md final whole-goal review (`goal.md` §10) | goal.md; it excludes commissioning. K10 is this track's own review |

If a work package turns up something from the goal.md column, do not do it
here. Hand it over (§6).

## 6. Handover to the goal.md session

Anything this track produces that belongs to the goal.md session goes into
your handover entry under the heading **"For the goal.md session:"**. The
goal.md session adopts it into `goal.md` §12.4 and confirms that in its next
entry. Typical cases:

- new or renumbered KNOWN_LIMITATIONS entries, for the LIMITATION_TRIAGE
  recount;
- `stats.md` refresh after each of your merges into `main`;
- findings in goal.md areas (UI surfaces, product database, server), as a
  finding, not as a fix;
- changes to shared rules that `goal.md` must reflect.

The handover from 2026-09-28 is already in `goal.md` §12.4.
