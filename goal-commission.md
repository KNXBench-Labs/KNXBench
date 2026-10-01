# KNXBench goal — commissioning (T30 phase 3), and nothing else

Written 2026-09-28 against `main` at `a47d168`. Use this file as the
instruction passed to `/goal` in the **commissioning session**. Creating the
file does not start a run. Every device write still needs its own "go" (§1).

This file is the counterpart of `goal.md`. `goal.md` §0 excludes
commissioning and every write to real KNX hardware. This file owns exactly
that excluded part, and **only** that part. If an item would change something
`goal.md` owns, it does not belong here: see §5 for the boundary, and §6 for
how work is handed across.

## Where things stand (2026-10-01)

K1–K19 have historical implementation evidence at their documented scope.
CLI/Web download and button-driven address programming ran on MDT `1.1.67`
with device-specific approval; complete and all three partial download
scopes were read back. A
pre-write region backup and restore ran on that device for complete and
parameters-only download. K13 address reset was performed and recovered on
that device; K14 destructive Master Reset remains hardware-refused. K12 serial
address write was ignored by this device even after a system-priority fix;
read-only identification succeeded. RF K16/K17 is simulator-only and has no RF
hardware or product-facing route. K19 decoded 71 private cEMI frames offline;
no raw frames belong in Git. See [RESEARCH](docs/RESEARCH.md),
[limitations](docs/KNOWN_LIMITATIONS.md) and
[implementation status](docs/IMPLEMENTATION_STATUS.md) for the per-operation
evidence and refusal boundaries.

**Current safety boundary:** the historical address runs do not provide
durable complete recovery for a later device. Confirmed public button
programming (ADR-0059), serial address writes (ADR-0057) and K13 reset
(ADR-0058) now refuse *before tunnel opening* until their respective
device-specific pre-write backup/readback/abort contracts are verified.
Read-only plans, identification and simulator work remain available. The
Debug bit-2 route (ADR-0051) instead has an offline-tested, property-only
backup gate and a default-off explicit UI action, not a full device restore
or new live evidence. K7 has a simulator interrupted-run/retry regression;
this is not a live interruption. No K14 erase or RF hardware test is claimed.
The user approved experimental K6 investigation but no new write. The
previous target `1.1.67` is reportedly off the bus; `1.1.32` is a read-only
identification **candidate**, not a verified model or approved write target.
Confirm its physical role, identity and per-device recovery before asking
for a new operation-specific go. Never transfer the old go or infer complete
storage from a diagnostic dump (RESEARCH §24). The current handover in
`.ai/CURRENT_STATE.md` wins if evidence advances.


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
   - `docs/ROADMAP.md` Session 7's commissioning scope and the
     open-evidence summary;
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
   - Coordinate across worktrees with a nonblocking `flock` on
     `$(git rev-parse --git-common-dir)/knx-gateway-tunnel.lock`, held from
     just before the live command until the tunnel disconnects. Process and
     socket checks still matter for clients not using this advisory lock.
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
   reviews run in this session. The user monitors quota themselves; do not
   pause at package boundaries to ask for a quota reading.
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

## 3. Delivered commissioning packages and current boundary

K1–K19 have their dated outcomes in
[IMPLEMENTATION_STATUS](docs/IMPLEMENTATION_STATUS.md) and
[RESEARCH §19](docs/RESEARCH.md); the former implementation checklists are
not a queue. K11 access-key handling, K15 all partial modes, K16/K17 RF
simulator procedures and K19 offline cEMI capture analysis are delivered at
their verified scope. No RF device was tested, and no raw private telegram
belongs in Git.

**Currently actionable without a device write:** the proposed K6 candidate
`1.1.32` was probed read-only at its exact address (occupied, `0701h`);
one-target identity read returned MDT-compatible manufacturer and a six-octet
hardware-type prefix matching two local `.01` comparison records, but no
`PID_PROGRAM_VERSION` elements (RESEARCH §24). Neither the full model nor
the installed application image, button accessibility or affected storage is
established. The operator's “go” concerned suitability/temporary addressing,
not an exact write request; no press, key, download or write was attempted.
Establish durable pre-write recovery and obtain a new operation-specific go
before reopening K6/serial/K13. ADR-0051's property-only backup is delivered,
and its guarded Debug UI is published, but cannot substitute for these
whole-device recovery contracts. `GET /api/bus/activity` is partial
server-lifetime evidence, not a durable audit. The global status/history,
partial-scope selector and any reset UI still need separate contracts and
the Web lock; read-only Device checks already shipped without a write go.
An offline K7 restore-file guard now rejects missing, duplicate or extraneous
load-state records for the plan; this does not extend the saved storage scope
or reopen any of the address-write routes.
Returned download errors also attempt to close a still-open management
connection without retry/restore; dropped-future/process-crash cleanup and
complete durable address-write recovery remain separate unproven contracts.

**Still hardware-bounded, not queued as automatic retries:** K12 serial
address writes were ignored twice by `1.1.67` (even after SYSTEM priority
and bit-2 changes); confirmed public serial writes now fail before tunnel.
The button-driven MP §2.3 path worked historically but is also currently
fail-closed pending ADR-0059 recovery. K13 reset on `1.1.67` ran and was
recovered, but its public CLI now refuses confirmed runs (ADR-0058);
multi-device recovery and HTTP/UI reset are unproven. K14 erasing Master
Reset remains refused on hardware. A fresh device-specific go and verified
pre-write recovery are both required for a new mutation. The simulator
is not evidence for another device, mask or RF.

## 3c. Later goals (not this stage)

Each gets its own goal file when the user asks for it: Powerline (PL110,
PL132), KNX IP device configuration (CP §3.9, masks `5705h`/`57B0h`),
coupler filter tables (masks `0912h`/`091Ah`, CP §3.10–§3.12),
`NM_Router_Scan`/`NM_SubnetworkDevices_Scan`, KNX Data Secure
(`DM_SecureSync_*`, CP §1.5), Easy Modes (PB §5, Ctrl §6), USB interface
configuration (CP §3.13).

---

## 4. Completion condition

K1–K19 have delivered their documented simulator, product-command and
bounded live evidence. Do not reopen them as an ordered queue. Future
commissioning work is complete only when the **specific** pending safety
contract (§3) is verified in code and tests, docs reflect its actual scope,
the required gates pass on the integrated tree, and the handover names any
remaining hardware-only or user-owned decisions. A new target, mask or
procedure is a new scope with a fresh go; RF remains simulator-only.


---

## 5. Boundary with `goal.md` (no overlap)

| Topic | Owner |
|---|---|
| Group-value sends, bus monitor, scan, discovery, T17 diagnostics UI, E2 scan reconciliation (`goal.md` §3) | goal.md |
| `/api/project/download` (exports a `.knxdb` **file**, KNOWN_LIMITATIONS §23) | goal.md; this is not a device download |
| ADR-0040 consent hook/dialog | built; the commissioning code owns each device-specific server gate |
| Parameter editor, device editor and catalog (ISSUE-07/08/09) | `goal-ui.md` for Web UI, `goal.md` for product/import data |
| `apps/knx-web` and its Web lock | `goal-ui.md` §3; commissioning takes the lock explicitly for its own views |
| Product-database import, schema, PDB-x | goal.md; commissioning only reads stored product data (ADR-0044) |
| `docs/LIMITATION_TRIAGE.md`, `stats.md`, `goal.md` | goal.md session |
| Commissioning code and device-specific evidence | this goal |
| Final whole-product review | goal.md §10; this track has its own closing review |

If a work package turns up something from the goal.md column, do not do it
here. Hand it over (§6).

## 6. Handover to the goal.md session

Anything this track produces that belongs to the goal.md session goes into
your handover entry under the heading **"For the goal.md session:"**. The
goal.md session adopts it into `goal.md` §12.3 and confirms in its next
entry. Typical cases:

- new or renumbered KNOWN_LIMITATIONS entries, for the LIMITATION_TRIAGE
  recount;
- `stats.md` refresh after each of your merges into `main`;
- findings in goal.md areas (UI surfaces, product database, server), as a
  finding, not as a fix;
- changes to shared rules that `goal.md` must reflect.
