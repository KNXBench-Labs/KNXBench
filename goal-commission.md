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

**Status 2026-09-28: done (simulator).** `knx device download`: a plan by
default, a write only with `--gateway` and the phrase, refusals before any
socket, `written to the device: yes|no|partially`, and a loud unconfirmed
restart. Four end-to-end simulator tests plus five mutants. Not yet run
against `1.1.67`; that needs a go. IMPLEMENTATION_STATUS 2026-09-28 "K4".

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

**Status 2026-09-28: done (simulator).** ADR-0045 (the server demands the
plan's phrase and the exact shown plan). `/api/device-download/{plan,start,status}`
and the "Download to device" / "In Gerät laden" tab with R1 progress. §101
bound: the tab shows counted progress, no time bound. The File menu rename
is `goal-ui.md` U3. No real device written; the UI's
**[W]** run belongs to K7. IMPLEMENTATION_STATUS 2026-09-28 "K5".

### K6 — Individual-address programming as a product command

1. CLI command and UI dialog with a loop "press the programming button on
   exactly one device". This lifts KNOWN_LIMITATIONS §116.
2. **[W]** Live-verify the settling retry (`be91fe3`) in the next
   programming-mode session.

**Status 2026-09-28: CLI half done (simulator).** `knx device
program-address` on the new `programming_button_wait` loop. It says
"press" / "release all but one" when the count changes, gives up after
`--wait`, and ends with `address written: yes | no | yes, but NOT
confirmed`. KL §116's `repeat` is lifted for the CLI. Still open: the UI
dialog (web lock) and item 2 **[W]** (needs a go and a button press).
IMPLEMENTATION_STATUS 2026-09-28 "K6".

**Status 2026-09-28: UI half done (simulator).** Bus → **Program address**
tab and `/api/device-address/{phrase,start,status,stop}` (ADR-0046) on the
same loop:
- press / release-all-but-one prompts;
- the wait can be stopped, the procedure cannot;
- one phrase covers the write and its restart;
- mutual exclusion with the monitor, the scan and the download.

Item 1 is done. Item 2 **[W]** still needs a go and a button press.
IMPLEMENTATION_STATUS 2026-09-28 "K6 UI".

**Status 2026-09-29: live.** `1.1.67` → `1.1.68` on hardware: button
pressed, steps 1–3 done, step 4 read the device at `1.1.68`. The
unacknowledged closing restart was misreported as a step 4 failure. That is
fixed (`AddressRestart`; CLI `restart: NOT confirmed`, server
`restartConfirmed`) and covered by tests plus 7 mutants. Item 2 (the
settling retry) was not exercised: the device answered on the first connect.
The way back, `1.1.68` → `1.1.67`, ran on the fixed build at 07:34:
`finished`, exit 0, restart NOT confirmed. The scan afterwards showed the
device back at `1.1.67`, and the memory is identical to before, so K6 is
done live. Still open, outside this goal: the web panel does not show
`restartConfirmed` yet (web lock, UI session), and the settling retry has
not been exercised. RESEARCH §19, "K6 live".

**Current safety boundary, 2026-10-01 (ADR-0059).** The historical K6
round trip did not make a verified durable pre-send backup/readback of *all*
storage potentially affected on every future pressed device. Confirmed
public CLI and HTTP starts now fail before opening a tunnel; plan/phrase-only
reads remain available. Simulator session, status/stop and one-tunnel tests
remain, but are no longer evidence that public starts may reach hardware.
No new live permission is implied. See RESEARCH §24; this does not retroactively
invalidate the observed `1.1.67` result.

### K7 — Live acceptance of the product path

- **[W]** Download `1.1.67` through **K4's command** (and K5's UI) with a
  configuration from the project that differs from option C. Check the
  function with a read-only bus monitor while the user operates the device.
  Afterwards the fixed test is no longer the only hardware write path.
- **[W], optional, user decision.** Recovery: re-run the same plan after a
  deliberately interrupted download. KNOWN_LIMITATIONS §7 says "a failed run
  undoes nothing".

**Status 2026-09-29: done.** CLI at 06:11: the new project `KNXBench
1.1.67 K7 switch-by-push off.knxdb` (button 1 "Switch by push", Off, 4
octets different from option C). 1416/1416 octets read back, and an
independent dump shows 0 differing octets. Web UI at 06:25: option C
through the real front end (Playwright, consent dialog, one `start` with
the plan's own phrase), again 1416/1416 and 0 differing. **Function check**
at 06:52: the K7 project again, then a read-only monitor while the user
pressed button 1: 11 telegrams, all `0` to `2/0/53`. So the configuration
is active, and without a power cycle, so the unacknowledged restart does
restart the device. Option C was restored afterwards; the read-back shows
0 differing octets. RESEARCH §19, "K7 live acceptance". Recovery (an
interrupted download) remains optional and not done.

### K8 — Rule on the excluded KNOWN_LIMITATIONS entries

For each of §93, §99, §101, §104, §105, §108, §109, §111, §112, §113, §114
and §115: either meet its "Lifted when" condition with evidence, or bring it
to the user as an accepted boundary.

- **Offline and actionable now:** §105, Transport Layer control frames at
  priority `SYSTEM`, with a cEMI test pinning the Ctrl1 octet.
  **Status 2026-09-28: done (encoder, simulator).** `0xB2` for
  `T_CONNECT`/`T_DISCONNECT`, `0xB0` for `T_ACK`/`T_NAK`, test plus 7
  mutants. The live effect is unverified; see KNOWN_LIMITATIONS §105.
- **Spec boundaries, likely to become user acceptances:** §108, §109, §99,
  §114.
- **Needs a use case or a feature:** §111 (unload IA), §112 (`A_Key_Write`),
  §113, §115.

**Status 2026-09-28: decided.** The user answered the K8 question with
*"das was am sinnvollsten ist"*: take the recommendation.

- §105 is lifted (`6a7f03d`).
- §99, §108, §109 and §114 are accepted boundaries.
- §111, §112, §113 and §115 are accepted until a use case exists.
- §93 stays parked: the v1 path does not run `procedure.rs`.
- §101 and §104 are deferred to hardware: measured in the K7 live session
  if they occur, otherwise accepted.

Each entry carries its own dated note in KNOWN_LIMITATIONS.

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

**Status 2026-09-28: decided (ADR-0048).** The recommendation is taken:
- v1 is the memory path for mask `070nh`, verified on `1.1.67`;
- everything else is refused by name;
- the `[A]` rules stay documented assumptions;
- the property-based `Downloader` stays in the tree, simulator-only.

Recorded in KNOWN_LIMITATIONS §7.

### K10 — Close the commissioning track

1. A whole-track review in this session, adversarial, over `git diff`
   between the start and the end of this track.
2. Fix its findings.
3. Write a closing handover, including the items for the goal.md session
   (§6).

**Status 2026-09-29: done.** The review covered `a47d168..dafa2b6`
(37 commits, 62 code files). The hardware paths are sound: the write gate
allowlist, lock order and one-tunnel exclusion across download,
programming, monitor and scan, named refusal of non-`070nh` masks in
productdb, net and CLI, and a tunnel disconnect on every exit. The live
finding (K6 restart misreport) was fixed in `6a71162`. The remaining
findings were stale commissioning rows (E1, ROADMAP, §7/§101/§104/§136,
spec status), corrected in the K10 commit. Handover: `.ai/CURRENT_STATE.md`
2026-09-29 K10 entry.

## 3b. Second stage: the PDF findings (user, 2026-09-29)

User decision 2026-09-29: *"alle Findings sollen bearbeitet werden. funk knx
geraete habe ich nicht, daher kann ich sie nicht testen. trotzdem sollte es
implementiert werden."* Scope, chosen the same day: the five gaps of
RESEARCH §19.5 **plus RF** (domain address, RF serial number and
configuration). Everything else in the "not relevant" list of §19.5 becomes
a later goal (§3c).

Rules on top of §1/§2:

- **RF is simulator-only.** The user has no RF device. Every RF result is
  labelled "simulator-verified, never run on RF hardware" in code, docs and
  CLI output. Claim no RF compatibility.
- **Every [W] step still needs its own go** (§1.2). Master Reset and
  address reset are destructive: offer them for `1.1.67` only if the user
  asks, and plan the re-download that restores option C afterwards.
- **No access key is ever guessed.** A key comes from the operator or from
  the project, never from a default list or a search.

### K11 — Access keys on a download (RESEARCH §19.5 item 4, offline)

**Done 2026-09-29.** Project `BCUKey` and CLI `--key-file` (no key in argv
or HTTP); MP §3.5.2 for `070nh`, §3.5.1 otherwise; a hint on refusal.
Simulator, CLI and HTTP tests; 9/9 mutants killed. KL §138.

The download path authorises with an operator key (CLI option and server
field, never logged) or with a non-default `Installation/@BCUKey` from the
project. A `MemoryRefused`/`PropertyRefused` after `Skip` names a missing
key as one possible cause. RED tests in the simulator against a device with
protected memory. No live step: the test device has no key, and setting one
would be a write this goal does not need.

### K12 — Individual address by serial number, TP (item 1)

**Live 2026-09-30:** bit 2 set and cleared on `1.1.67` (read back); with
it set the serial write was still ignored. KNXBench had sent the address
broadcasts at low instead of system priority (AL §3.2.2–§3.2.5); fixed
(`c451fa95`). Second run with the fix: still ignored, device restored
(RESEARCH §19.15, KL §139). K12's live write is closed as "not supported by
this device" until another device is available.

**Live 2026-09-29:** reads verified on `1.1.67`; the write is ignored by
the device (`PID_SERVICE_CONTROL` bit 2 clear, KL §139, RESEARCH §19.8).

**Done 2026-09-29, simulator only.** Encoders, MP §2.4/§2.5,
`PID_SERIAL_NUMBER`, project serial numbers, CLI and HTTP; 9/9 mutants
killed. KL §139. The live step below is still open and needs its own go.

`A_IndividualAddressSerialNumber_Read/_Response/_Write` encoders (AL),
`NM_IndividualAddress_SerialNumber_Read` and `_Write` (MP §2.4/§2.5), the
serial number read from the project or from `PID_SERIAL_NUMBER`. CLI and
server command next to K6's. **[W]** one live write on `1.1.67`, to a free
address and back, like K6. Reading its serial number first is a read-only
step (§1.3).

**Follow-up 2026-09-30 (user: "so lassen, aber Option in Settings").**
KNXBench never sets `PID_SERVICE_CONTROL` bit 2 on its own. An opt-in
action exists (ADR-0051): `crates/knx-net/src/commissioning/service_control.rs`
(read-modify-write of bit 2 only, read-back, mask `0021h` and missing
property refused), `WriteScope::IndividualAddressWriteEnable`,
`/api/device/service-control` behind the Settings key
`debugIndividualAddressWriteEnable` (403 unless `true`), and
`knx device service-control` on the CLI (`e05e9e1`, `30580fad`). Simulator,
route and CLI tested. Bit 2 was set and cleared again on `1.1.67` in
the authorized K12 live run; serial-number addressing still did not take,
even after correcting broadcast priority (RESEARCH §19.15). The Settings
UI toggle remains with `goal-ui.md` U12.

**Recovery-policy follow-up 2026-09-30 (offline only).** ADR-0051 now gates
both CLI and HTTP property writes on a durable pre-write record of *the entire
property element* (two octets), mask and target. If persistence/readback fails,
no property write is sent. The record is not a complete device image, and no
automatic restore or new hardware test is claimed. UI owner may revisit the
Debug action under its own lock; K13 still needs a separate whole-device
backup contract.

**Recovery availability follow-up 2026-09-30 (ADR-0057, offline only).**
The public confirmed CLI and HTTP serial-address write paths now fail closed
*before tunnel opening* with an explicit missing-backup error, including
possible no-ops and after the separate service-control bit-2 toggle. Neither
previous-address readback nor the simulated K12 result was a durable backup
of all affected storage. Read-only `find-serial`, the CLI plan and MP §2.5's
simulated core tests remain. A future device-specific recovery contract must
precede any reopening; this does not authorize another hardware attempt.

**Further live serial-number tests need a fresh device-specific go and
new evidence on a device that supports the operation.** The two MDT runs
have already shown an ignored write; do not repeat them from this goal alone.

### K13 — `NM_IndividualAddress_Reset` (item 2)

**Live 2026-09-30 on `1.1.67` (historical user request).** `knx device reset-address
1.1.67`: exactly `1.1.67` in programming mode (new guard), one round, then
`1.1.67` vacant and `15.15.255` occupied. The unevaluated restart did not
end programming mode (LED on). Recovered with `program-address 1.1.67`;
compare and dump identical before and after (KL §140, RESEARCH §19.16).

**Recovery safety follow-up 2026-09-30 (ADR-0058, offline only).** The
public confirmed reset CLI now fails closed before tunnel opening. Its phrase
and exact programming-mode device guard do not themselves persist a verified,
complete backup of all storage affected on every pressed device; the one live
run's separate application dump cannot establish that contract for arbitrary
hardware. Plan-only mode and protocol simulator remain. No further reset is
authorized without an implemented per-device durable pre-write backup/readback,
recovery plan and a fresh device-specific go. An HTTP/UI reset remains blocked.

**Done 2026-09-29, simulator only.**

MP §2.18. Resets every device in programming mode to `FFFFh`. **[W]**
optional, only on the user's request.

### K14 — Master Reset (item 5)

**Done 2026-09-29, simulator only; erasing codes refused on hardware (KL
§141).** A live Master Reset needs the user's request and the option-C
re-download afterwards.

`A_Restart` with the master-reset type and erase code (AL, MP `DM_Restart`).
Simulator first. **[W]** only on the user's request, followed by the
option C re-download.

### K15 — Partial download for `070nh` (item 3)

**Live 2026-09-30:** `--partial group-addresses` first interrupted by an
external timeout in step 17/21 and repaired with the complete option-C
download; re-run on "k15 go" without the timeout: 1022 octets read back,
both tables `Loaded`, dump identical. Verified (KL §142, RESEARCH §19.15).
`--partial both` at 16:00: 24 steps, 1416 octets read back, all `Loaded`,
dump identical (RESEARCH §19.17). All three partial scopes verified.

**Live 2026-09-29:** parameters-only partial download on `1.1.67`, 394
octets read back, dump unchanged (KL §142 lifted, RESEARCH §19.8).

**Done 2026-09-29, simulator only (KL §142).** CP §3.9.2.4's transformation
of the complete plan, plus an application and load-state check before the
first write. A live run needs the user's request.

Research first: what the product file's load procedures say about a partial
download for mask `070nh`, against CP §3.5.3. Implement only what is
documented; otherwise record the boundary. **[W]** optional.

### K16 — RF domain address (simulator only)

**Done 2026-09-29, simulator only (KL §143, RESEARCH §19.6).**
`A_DomainAddressSelective_Read` is not implemented: it is PL110-only (AL
NOTE 6), and so are MP §2.11 (unspecified) and §2.13 (Data Security).

`A_DomainAddress_*`, `A_DomainAddressSelective_Read` and
`A_DomainAddressSerialNumber_*` (AL), the domain-address procedures of MP
§2.7–§2.14 for RF, and the cEMI RF additional information they need. A
simulated RF device answers them.

### K17 — RF device configuration (simulator only)

**Done 2026-09-29, simulator only (KL §144, RESEARCH §19.7).** The
link-sequence start of MP §2.6 and the unidirectional parameter view are
PB-Mode and stay with the Easy-mode goal.

CP §2.3 (RF domain), §3.6 (RF bidirectional: identification,
individualisation, parameter download) and §3.7 (RF unidirectional:
identification, individualisation, group-address calculation), on top of
K12 and K16.

### K18 — Close the second stage

Like K10: review all packages, gates on the merged `main`, the
commissioning rows updated, handover.

**Status 2026-09-29: done.** Review over `fc601db^..c04fb53` (K11–K17, 36
code files). Hardware gate: only individual-address programming, restart
and download pass `hardware_write_is_authorised`; IA reset, Master Reset,
domain address and RF configuration are refused by name and pinned in
`hardware_write_gate.rs`. `AccessKey` has no `Display` and a redacting
`Debug`; nothing else in the key path carries a value. RF has no CLI or
HTTP route. Findings: the K16 commit swallowed RESEARCH's `## 20.`
heading (restored in `c04fb53`); two K17 mutants survived the first run
and got tests (`a_device_that_ignores_the_write2…`,
`another_devices_function_answer…`). Gates on merged `main` at `c04fb53`:
Rust 124 suites 2526 passed 0 failed, web 1170/1170 and build, fmt,
clippy, layering, headers (302/161), anchors (397), deny. Every stage-two
result is simulator-verified; the [W] live runs of K12–K15 on `1.1.67`
are deferred until the user gives a device-specific go.

## 3b+. Added to the readiness work (user, 2026-09-30)

### K19 — Offline check of the 71 private telegrams

A private ETS `CommunicationLog` XML
(`{http://knx.org/xml/telegrams/01}`, 71 `Telegram` records, hex
`RawData` starting with `29`, i.e. cEMI `L_Data.ind`) was found read-only
in the off-repository Windows user profile (2026-09-29 inventory). The
user assigned it to the commissioning/readiness session.

- Offline only: decode every `RawData` with `knx_net::cemi::decode_l_data`;
  report decoded / refused / unknown counts and every refusal by reason.
  No bus traffic, no socket.
- Private data: frames, addresses, timestamps and connection names never
  go into Git, docs or logs. Tracked output is aggregate counts only.
- A tracked fixture only as a sanitized, hand-verified subset (synthetic
  addresses), and only if a decoder gap needs a regression test.
- KNXBench does not import this capture format; adding an importer is a
  separate decision, not part of K19.
- **Done 2026-09-30.** 71 of 71 decode, none refused or unknown; all 71
  re-encode octet for octet past Ctrl1/Ctrl2. Found: the decoder drops
  the priority (16 normal-priority telegrams), KNOWN_LIMITATIONS §147.
  RESEARCH §19.14; test `crates/knx-net/tests/private_telegram_log.rs`
  (`KNXBENCH_TELEGRAM_LOG`). No fixture was needed: no decoder gap.
- **Follow-up done 2026-09-30.** §147 lifted: `LDataFrame::control`
  carries priority, repeat, ack request and hop count; 71 of 71 now
  re-encode whole. **Shown 2026-09-30** (`542fb56a`): `knx bus monitor
  --control`, server monitor rows and debug bundle carry `control`. The
  web table column is with `goal-ui.md` U12.

### Commissioning activity status follow-up (2026-09-30)

An additive, read-only `GET /api/bus/activity` now summarizes retained
download/programming sessions and the existing monitor/scan sessions without
opening a tunnel. It explicitly reports **partial coverage** (ADR-0055):
one-shot writes/reads have no retained history, and a locked holder is only
reported as busy, not attributed to an invented target. This is a backend
step toward the requested global status bar, **not** that status bar or full
action coverage. Web UI remains with the separate lock owner; no additional
live write was authorized or performed.

**Backend follow-up, 2026-09-30:** read-only device comparisons now appear as
bounded, server-lifetime one-shot activity with target, time and
finished/failed/unknown outcome; aborted requests never become fabricated
successes. Retention loss is counted. Other one-shot actions remain untracked,
the API still declares partial coverage, and no UI or hardware was changed.

**Read-only lookup follow-up, 2026-09-30:** serial-number lookup now adds
one-shot evidence without leaking the serial or fabricating a physical
address: its activity target is `null`. Witnessed negative reads finish;
cancelled requests remain unknown. Serial-address **writes**, service-control
and group writes remain outside the ledger. The snapshot is still not a
global idle signal; no live hardware was touched.

**Gated property-read follow-up, 2026-09-30:** the Debug-enabled
`PID_SERVICE_CONTROL` read now reports bounded one-shot evidence, without
property bytes, mask or project key. Refusal stays pre-tunnel and cancellation
remains unknown. Its **write** is intentionally still untracked: a no-op or
unverified write is not a generic success/failure receipt. The pre-write
backup, separate confirmation and live hardware go are unchanged.

**Write-evidence prerequisite, 2026-09-30 (ADR-0056):** a generic read
`finished`/`failed` cannot represent a write. Simulated serial-address and
service-control calls can return 200 without sending a write; a failed send
may have changed the target. Group writes have no receiver readback. Keep all
three write routes untracked at that stage until phase-aware,
recovery-linked evidence is verified. The serial-address HTTP path still
lacks a durable pre-write recovery record; do not infer that its response
satisfies the backup policy or grants a live hardware go.

**Service-control write evidence, 2026-09-30:** only the Debug-gated
`PID_SERVICE_CONTROL` POST now has typed one-shot activity. Verified means
the existing exact-octet property response was witnessed; no-op and
pre-send failures have their own states. A transport error after the backed-up
write boundary is `effectUnverified`, not a green receipt, and cancellation
is `unknown` even if a send became possible. The optional `writeEvidence`
flags reveal neither property bytes, mask, key nor backup path. This is
volatile, bounded telemetry, not recovery storage or permission for a live
write. Serial-address and group writes remain untracked; `coverage: partial`
and the Web-lock boundary remain.

**Group-write ownership handoff, 2026-09-30 (offline audit only).**
`bus_routes::write_value` encodes a group value and awaits
`BusSession::send`, which forwards to the held tunnel. Its HTTP 200 payload
is a codec echo, not receiver readback. The existing write-activity guard
has a documented pre-write-backup precondition, but its call sets both
`backupRecorded` and `sendPossible`; without a real backup that would falsely
assert recovery evidence. No receiver identity, complete affected-storage
scope or durable backup is established by that route. Under §5, group-value
sends belong to `goal.md`: send semantics, backup policy and route-specific
activity require that session's decision and implementation. This commissioning
track hands over the finding (§6) and keeps `groupWrite` untracked; no bus or
Web changes were made.

## 3c. Later goals (not this stage)

Each gets its own goal file when the user asks for it: Powerline (PL110,
PL132), KNX IP device configuration (CP §3.9, masks `5705h`/`57B0h`),
coupler filter tables (masks `0912h`/`091Ah`, CP §3.10–§3.12),
`NM_Router_Scan`/`NM_SubnetworkDevices_Scan`, KNX Data Secure
(`DM_SecureSync_*`, CP §1.5), Easy Modes (PB §5, Ctrl §6), USB interface
configuration (CP §3.13).

---

## 4. Completion condition

Finish only when:

- The second stage (§3b, K11–K18) is done the same way: every package
  with evidence, RF labelled simulator-only, every [W] run or deferred.
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
| Parameter editor, device editor, catalog (ISSUE-07/08/09) | goal.md; the UI parts are in `goal-ui.md` since 2026-09-28 |
| `apps/knx-web` in general, and the web lock | `goal-ui.md` §3. K5/K6 take the lock for their own views only, one package at a time |
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
