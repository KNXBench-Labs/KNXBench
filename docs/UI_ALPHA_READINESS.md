# UI alpha-readiness owner audit and follow-ups

## UI owner closure receipt — 2026-10-06

Owner: the Claude `goal-ui.md` owner session. Asked for by the user for
`RELEASE-03` / AR16 ([ALPHA_SCOPE_MATRIX](ALPHA_SCOPE_MATRIX.md)). This is the
owner's own receipt, **not an independent review**: AR16 verifies it and
AR18 reviews the whole product independently. It is not an alpha tag or a
release claim.

**Candidate.** `origin/main` at `1f4aca11` plus the residue package
`892b9948` (UI-04 Web half, KL-61 binding wording), gated as one tree before
publication; this receipt follows as a docs-only commit.

### Completion condition (`goal-ui.md` §4)

| Condition | State | Evidence |
| --- | --- | --- |
| U0–U13 done | Met | U13 receipt `dfa0cc79` (independent GPT-6.1-Sol review, findings fixed); [owner status history](#owner-status-history) |
| Issue-plan checkboxes | Met | [Issue plan](superpowers/plans/2026-09-21-user-reported-issues.md): 68 ticked, 0 open |
| U14–U18 | Met, one user-approved change | U18 `1964fd6b`; the user's 2026-10-05 request replaced U17's preview with one dropdown and retired two palettes ([ADR-0079](adr/0079-theme-choice-is-one-dropdown.md), `03609b60`) |
| U19–U21 and AR21 hand-off | Met | AR21 accepted `FLOW-01` after Alpha's independent reruns of findings 1–7 ([TELEGRAM_FLOW_VISUALIZATION §22](TELEGRAM_FLOW_VISUALIZATION.md#22-ar21-rerun-of-findings-6-and-7-and-acceptance-alpha-2026-10-05)) |
| §2.5 gates green on the merged result | Met | Closing gate below |
| Closing review, findings fixed | Met (self-review) | Below; two findings fixed in `892b9948` |
| Web lock released | Met | Released in `892b9948`'s handover entry |

### Delivered since the U18 receipt

UA1–UA8 web halves (`596697a6`, `45e1299f`, `c6da3c78`, `0dd9add8`,
`e4737129`, `efe7fb53`, `bd2e5a3a`, `9bc36499`, `0438abed`, `ab1b87b7`);
U19–U21 (`51a6004e`, `4525c36e`, `dc298b78`, `9d432d17`, `fb40a99a`) with the
AR21 corrections (`595d8d2e`, `0d5da787`, `104916d6`, `6fa10eb8`); user
requests: sidebar splitters `4804982b`, theme dropdown and shipped CRT
`03609b60`; routed halves: KL-142 download scope `2f2a6892`, ADR-0080
write-authority reasons `81946dbc`, AR10/KL-37 language markers `f5494094`,
`ba190b6e`; closure residue `892b9948`. Each package: RED first, mutants,
gate under the shared leases, log under `.ai/logs/`.

### Source-ID rows owned by UI

26 rows in the [ledger](status/LEDGER.md): 15 `DONE`, 10 `ACCEPTED_BOUNDARY`,
1 `LATER` (`KL-43`, owner decision 2026-10-04). Rows owned elsewhere whose Web
half was handed to this owner: `KL-142` (delivered, `DONE`), `KL-37`
(delivered, `ACCEPTED_BOUNDARY`), `UI-04` (Web half delivered; closing the
row is the commissioning owner's), `KL-61` (binding wording delivered;
the declared-versus-linked display needs a projection field from the backend
owner first).

### Closing review (self-review, 2026-10-06)

Mechanical, over every Web change since `1964fd6b` (32 commits, 136 files):
no focused or skipped tests, no scratch specs, no new `console.log`, no new
TODO/FIXME. Documentation, searched for work still waiting on the UI owner:

1. **Fixed — `UI-04` Web half.** The handover in
   [COMMISSIONING_ALPHA_LEDGER](COMMISSIONING_ALPHA_LEDGER.md) asked for the
   live snapshot and refreshed running rows; `GET /api/bus/activity` had no
   Web reader. Delivered as *Live activity* (`892b9948`).
2. **Fixed — `KL-61` binding wording.** `GroupAddressNode.dpts`' generated
   doc comment still described the linked-only rule (ADR-0078 hand-over).
3. **Not this owner's — `KL-61` display.** Showing declared versus linked
   needs the projection to carry the declaration and outcome; recorded in
   KNOWN_LIMITATIONS §61 for the backend owner.
4. **Stale wording elsewhere, not changed here:** the commissioning ledger's
   "Open: `KL-142` and `UI-04`" sentence now has a delivery note beneath its
   handoff table; `ALPHA_SCOPE_MATRIX`'s `UI-04` row is Alpha's to update.

### Tested surfaces and exceptions

Tested: happy-dom Vitest through the real parents; headless Chromium against
locally intercepted HTTP (`page.route`), EN and DE, 360–1440 px; the
production Web build. **Not tested, accepted boundaries by the user decision
of 2026-10-04:** native WebKitGTK/Tauri workflows (beyond the static zoom
probe of KL-130-ZOOM), Orca or any real screen reader, native file choosers,
real multicast/firewall discovery (KL-79), a dead-WebView close (KL-133), any
live KNX bus. Telegram-flow motion: Chromium only, Motion Off for large maps
(AR21 envelope). No hardware, bus or KNX socket was used for this receipt.

### Closing gate

Run under the three shared leases, own target directory, corpus linked
(`OriginalData`), inputs hashed at start and end. Attempt 1 was refused
(Vitest 1 failed: the companion import-inventory guard, fixed in the
package; inputs changed during the run). **Attempt 2, head `03d8a7fb` plus the
candidate, inputs frozen:** `cargo fmt --check` 0; `cargo clippy --workspace
--all-targets -D warnings` 0 (incremental in that target: 350 units on
`4459e310`, then 6, then 1); `cargo test --workspace --no-fail-fast` 188 suites,
3,311 passed, 0 failed, 177 ignored; ts-rs bindings regenerate identically (17
files, trailing spaces ignored as in CI); check-layering, check-headers
(155 / ceiling 155), check-anchors 603, check-ledger 190, check-corpus-gates
all 0; `git diff --check` 0; `tsc --noEmit` 0, `tsc -b` 0; production Web build
0; Vitest 2,064 / 117 files; Chromium 139 (intercepted HTTP). **After rebasing
onto `1f4aca11`** (upstream: docs, Python tools, two manual-screenshot tooling
files outside `src/` and the suites, no Rust): headers 155/155, anchors 606,
ledger 190, diff-check 0, `tsc -b` 0, Vitest 2,064 / 117; Rust and Chromium
carry over. Published as `892b9948`, read back equal on `origin/main`.

### Left for others

`UI-04` row closure (commissioning owner); `KL-61` projection field (backend
owner); AR16's manual location/screenshot policy (user, `RELEASE-03`); AR18's
independent whole-product review.

**Addendum, later on 2026-10-06 (UI owner).** Two of these have since been
done by others, recorded here so the receipt is not read as current on them:
`KL-61`'s declared-versus-linked display was delivered by Alpha under its own
Web lock (`5adeb61a` → `4b9e913e`: `GroupAddressNode.dpt_detail` and the
group-address Inspector; ledger `DONE`), and `UI-04` was closed as
`ACCEPTED_BOUNDARY` by the user's acceptance (ledger, `f2b31538`). AR16 is
done. Still outside this owner: AR18's independent review. Two interface
follow-ups handed back by Alpha — the out-of-date `newProject.styleHint` and
styling for the new `.dpt-outcome` line — are UI-owner work prepared on the
branch `ui/style-hint-dpt-outcome`. Merging any code change moves `main` past
the revision AR18 reviews, so when it lands is decided with the release
owner and the user, not by this receipt.

## Telegram-flow owner addition — user decision 2026-10-04

[The approved Alpha feature](TELEGRAM_FLOW_VISUALIZATION.md) adds UI U19–U21
and alpha AR20/AR21 without reopening completed UI source-ID/UA/theme receipts.
The UI owner resolves a tested design/handoff in U19, consumes integrated AR20
in U20, and closes productive layout/pulse/value/theme/motion/load evidence in
U21. The alpha owner adopts it in AR21 before final readiness. Coordinate the
existing Web lock and active package; this plan takes/releases no lock and
starts no bus operation. Current source-ID status remains in its canonical
ledger, not duplicated here. No implementation evidence is claimed by this note.


Date: 2026-10-02. Candidate: isolated `ui-alpha-readiness` checkout, based on
`65b91777`, with reservation `bfb6fec1`. This is new owner evidence, not a
reopening of the completed U0–U13 issue queue or an alpha-release approval.

The execution contract in [ALPHA_READINESS](ALPHA_READINESS.md) explicitly says
owner rows are retained boundaries, not newly assigned copies of closed U
packages. This audit separates real implementation gaps, intentional bounded
contracts, and independent platform/sample evidence. **A retained boundary is
not a user-approved alpha exception.** No row disappears or changes primary
owner because its frontend foundation exists.

## Concrete follow-ups implemented

### Keyboard, modal background and viewport-safe help follow-up

The isolated `ui-alpha-keyboard` candidate at reservation `8656ffa1` adds
nearest-edge active-option scrolling to the three actual lists without moving
combobox focus. Catalog highlight is not product selection or device creation.
Document-local modal ownership excludes background branches through `inert`
and `aria-hidden`, preserves existing values, handles newly added DOM and
out-of-order close, and restores focus only after the final modal closes.
Focus filtering also excludes descendants of hidden/inert/aria-hidden branches.

HelpTip's permanent local description remains the trigger's `aria-describedby`
target; a separate decorative body portal escapes clipped/transformed ancestors.
Its fixed viewport bounds and placement account for application zoom; resize
and captured scroll re-place it, while cleanup removes its listeners/portal.
DE/EN Chromium cases verify geometry and the actual accessible button description,
including a tooltip inside a modal. This closes the earlier invisible-tooltip
overflow mechanism in the bounded fixture, not all application layout gates.

Seventeen new fully intercepted Chromium cases pass. Eighteen behavioral
negative controls are caught and restored exactly; an initially surviving
early-background-release mutant required a synchronous assertion before
MutationObserver repair and is now caught. The new hook's deliberate TypeScript
error is also detected. Restored focused verification: nine files / 117 tests
plus TypeScript pass. Renewed twelve-step candidate acceptance proc_870fe2d19835
passes: Web 84 files / 1,357 tests, 52 fully intercepted Chromium cases, Rust
146 result blocks / 2,890 passed / zero failed / 163 ignored, 576-source freeze.
The first header-grammar failure is retained; purpose/SPDX order was corrected
and the measured ceiling lowered to 157, never relaxed. Frontend alpha.2 has
identical dependency records. All twelve gates repeated on integrated source as
proc_490a156044df with identical counts/fingerprints. Published source 2e57f8e5
and exact remote/tree/all 28 artifacts plus zero outgoing commits verified.
No independent external review, native/Orca or bus evidence is inferred.

### KL-82 authoritative monitor context follow-up

The separate `ui-alpha-context` candidate (reservation `79dc56dd`) now compares
the actual server session's `GroupAddressContext` with the current server
project's interpretation snapshot: address style, group-address names and
resolved DPTs. Browser records are only early invalidation hints, never proof.
Monitor polling returns additive `contextStatus` (`current`, `stale`,
`unavailable`), nullable `projectOpen` and the opaque server incarnation.
Missing/malformed/legacy evidence, lock contention and failed polling remain
unverified and disable compose, including explicit-DPT sends. A server restart
cannot inherit an old browser record's project/session proof.

Pause still polls context/status via HTTP `contextOnly=true`, but neither returns
telegrams nor advances the held cursor. Resume continues from that cursor.
Late replies cannot erase a newer invalidation or a replacement session; a
late reattach rejection cannot overwrite a successful connection. Historical
rows retain their original decoded values: this does not reinterpret capture
history, provide project collaboration, authorize hardware or transactionally
bind a later write to this point-in-time comparison.

Focused backend/UI suites and the monitored Chromium fixture pass. Eleven
compiled/runnable behavioral negative controls were rejected, with byte-exact
restoration. The separate in-session review found busy/poisoned context coverage,
an obsolete reattach rejection and duplicate setters; those are corrected.
Complete candidate acceptance `proc_4258b3542021` passed all twelve steps:
83 Web files / 1,343 tests, 35 fully intercepted Chromium cases, 146 Rust result
blocks / 2,890 passed / zero failed / 163 ignored. Strict Clippy, type/build,
fmt, all four repository gates and whitespace pass; 570 source/configuration
fingerprints match. The first candidate failed its missing diagnostic hint CSS
rule; that failure is retained, the rule corrected without weakening the test,
and its removal also detected. Eleven controls restore exact source.
The final in-session review's non-runtime comment clarifications are included
in published source `8ceacf49b515aa2ee174ae2f9b77ec0d52e0d654`. All twelve gates
were repeated on that integrated source as `proc_f5cf67729adf`, with the same
counts and unchanged 570-source fingerprints. Exact HEAD/origin/main and
complete-tree equality plus all twenty owned artifacts were read back; outgoing
range is zero. These results do not transfer the previous package's evidence
or imply ignored private-corpus/native/live-bus execution.

- **UX-02:** the catalog picker offers `.knxprod` and ZIP packages, not the
  deliberately unsupported `.vd2`. Backend rejection and compatibility scope
  remain unchanged; an accept filter is not security validation.
- **UX-03:** the project Properties inspector exposes the existing undoable
  `POST /api/project/group-address-style` operation for `ThreeLevel`, `TwoLevel`
  and `Free`. It consumes the authoritative returned tree, prevents overlapping
  requests, shows failures, and retains unknown imported style text. This is a
  project command, not a cosmetic display preference or address renumbering.
- **KL-121:** every authenticated main/companion frontend periodically rereads
  the same server settings record and rechecks on focus/visibility. Reads do not
  write remote preferences back. Queued edits, failed local patches, unknown
  keys and deletion intent survive refresh; cancelled/obsolete replies cannot
  overwrite newer edits or restart a stopped timer. Background hidden windows
  defer their read; the interval is five seconds, not instantaneous push.
- **KL-124:** decoded Device Info is retained by `DiscoveredGateway`, projected
  as additive nullable HTTP `deviceInfo`, and displayed in expandable,
  keyboard-accessible details. Medium/status remain raw octets; project ID,
  serial, routing multicast and MAC are preserved. Missing adapter metadata is
  explicitly unavailable, not a fabricated zero. No medium/status capability,
  programming-mode, target-identity or write-permission claim is inferred.
  Existing CLI default output and protocol codecs are unchanged.

**Technical acceptance:** all eleven renewed offline gate steps passed, including
83 Web files / 1,331 tests and 146 Rust result blocks / 2,885 passed / zero
failed / 163 ignored. Type/build, strict workspace Clippy, fmt, layering,
headers, intended-root anchors, corpus gate configuration and whitespace pass.
The 570 tracked/new source/configuration fingerprints match before and after the
run. The first complete candidate gate failed the diagnostic CSS guard; missing
metadata layout rules were added without weakening it. That failed predecessor
remains failed. Rebased publication `6c16fe5aaf764d78f62382f867f59b6ae77f8dce`
matches the remote ref and complete tree. All eleven gates were repeated on
the integrated source with the same counts and unchanged source fingerprints;
these results do not imply execution of ignored private-corpus cases.

The default browser-test harness was also repaired: it now serves the actual
fixture HTML through an isolated loopback Vite configuration without the
development API proxy or a KNX backend process. All 33 existing browser cases
pass; no test was removed. The three legacy full-app new-project cases now
intercept every API request and assert actual select/region semantics. Two
configuration regressions pass; restoring the production proxy made the named
guard fail, and the fixture configuration was restored byte-exactly.

The initial old harness attempt failed all 33 cases: 30 fixture navigations
were HTTP failures, while three full-app cases loaded the backend and failed
at the obsolete language-input operation before project creation. That failed
attempt was **not purely mocked**. The existing full-app automatic discovery
can reach the backend on this path; its network traffic was not retained, so
no no-network assertion or live-discovery acceptance is made for that attempt.
No tunnel or device-write action was exercised. The corrected harness cannot
forward requests to the development backend and every test intercepts API calls.

## All UI-routed inventory rows

These are the exact 24 UI-owned IDs of the [source-ID ledger](status/LEDGER.md). `Open` names absent behavior,
not merely missing validation. `Retained` describes the current contract, not
release consent. Final ledger status follows verified delivery.

Status of these rows: [source-ID ledger](status/LEDGER.md) (AR14D D2).

| ID | Owner evidence | Remaining condition |
| --- | --- | --- |
| KL-79 | Existing offline UDP exchange and UI discovery remain verified at U13 scope; new metadata projection has focused coverage. | Native Search click and real multicast/firewall paths are not proved by fixtures; no deliberate live Search was accepted. The old-harness attempt has the unretained-traffic qualification above. |
| UI-01 | Already DONE in U13; no duplicate implementation. | Preserve original native/network qualifications. |
| UI-02 | Already DONE in U13; no duplicate implementation. | Preserve original native/network qualifications. |
| DATA-03 | Delivered 2026-10-04 with ADR-0069: each catalog submit carries one `requestId`; after a lost or 5xx response the catalog offers a retry with the same id, only while the server incarnation is unchanged, and treats `replayed: true` as success. No new request is ever sent for an unconfirmed batch. | A restarted server forgot the request, so no retry is offered; a pre-ADR-0069 server would ignore the id (mixed-version setups only). |
| KL-137 | Retained bounded monitor capture, explicit server/client dropped counters and 16 MiB export bound. | Native dialog and full retained-window workflow remain unverified; full-history streaming needs its own privacy/storage design. |
| KL-36 | Retained searchable/exportable bounded session log and atomic native writer, ADR-0047. | No reconstruction of evicted entries or lifetime audit; native chooser acceptance remains open. |
| KL-82 | Delivered authoritative interpretation comparison, unavailable/legacy fail-closed state, pause/cursor and delayed-reply regressions at 8ceacf49; twelve integrated gates and eleven behavioral controls verified. | Point-in-time interpretation is not collaboration, historical reinterpretation, native/live-bus evidence or transaction-bound write authorization; retained §82 boundaries remain explicit. |
| KL-127 | Site/property Ground workflow exists under ADR-0038; synthetic hierarchy/native storage tests retain unknown types honestly. | Independent ETS Ground/multiple-installation samples remain absent; do not infer ETS semantics from synthetic fixtures. |
| MODEL-01 | Delivered 2026-10-04 (ADR-0070): core/server half, web part 1 (creation, drag and drop and installation rename in every installation) and web part 2 (Inspector gates and move/link lists, bulk moves and CSV installation choice follow the owning installation). Nothing connects two installations. | An unassigned catalog device always lands in the first installation; an id not owned by exactly one installation stays read-only. |
| MODEL-02 | Delivered 2026-10-04 (ADR-0071): the Inspector repairs a multiply placed device ("Keep this placement" per slot) and a line listed by several areas of one installation ("Keep under this area"), each one undoable step; nothing is repaired automatically. | Duplicate ids are not renumbered; ambiguous building-part or group-range placement has no repair; two different lines sharing an id get no repair button. |
| MODEL-03 | Delivered 2026-10-04: the line-relative editor submits device number `0`; the server accepts it only for a product with `Hardware/@IsCoupler="true"` (RESEARCH §25) and its refusal is shown otherwise. Imported `.0` stays intact. | No guessed device classification: a product missing from the product database is still refused. |
| KL-133 | Existing close guard protects unsaved project state; normal quit tests are not a dead-renderer test. | Native unresponsive/crashed-WebView reproduction and a data-safe close/recovery policy; never bypass the guard by inference. |
| UI-03 | Retained read-only device-checks UI and explicit unsupported readiness/recovery states; commissioning owns complete recovery. | No full-image backup or universal device semantics follows from the UI; preserve device-specific commissioning prerequisites. |
| KL-130-ZOOM | New native WebKitGTK static Inspector geometry evidence at three widths/scales; Chromium interaction evidence remains distinct. | Full Tauri zoom shortcuts, pane resize/hide/restart and hover workflow are not covered by this static probe. |
| KL-20 | Delivered at 2e57f8e5: active-list scrolling, background inert/AX exclusion, stacked/dynamic modal ownership and viewport-safe HelpTip; twelve integrated gates and eighteen behavioral controls verified. | Actual native/Orca interaction and whole-app visual accessibility audit remain open, not a release waiver or full-source acceptance. |
| KL-124 | Delivered raw Device Info retention, exact nullable HTTP projection, six labelled UI values and explicit unavailable state at 6c16fe5a. | Native/live Search remains KL-79, not new protocol or identity acceptance. |
| MODEL-04 | Delivered 2026-10-04: the catalog offers two opt-in checkboxes, free-address allocation on the target line (disabled without one) and unique names; defaults keep indexed names and no address. Allocated addresses are listed per created device; a short supply is refused as a whole and shown as an error. | The allocator knows only the project, not devices on the real bus. |
| KL-121 | Delivered authoritative cross-client settings refresh with write-generation and cleanup regressions at 6c16fe5a. | Not general project collaboration or instantaneous synchronization. |
| KL-43 | Retained global motion level/style and OS-reduced-motion precedence; existing guard scope remains explicit. | Per-category motion/parser-backed wider guards require separate scope; no real-animation or assistive-technology conformance claim. |
| KL-97 | Retained truthful phase/count progress under ADR-0023. | No guessed percentage for streaming work whose total is not known. |
| KL-98 | Retained decorative, accessibility-excluded flavour text; slow rotation is intentional. | No artificial slowing or invented progress merely to show more jokes. |
| UX-01 | Delivered 2026-10-04: besides the two device gestures, a group address can be dragged from the Project Explorer onto a communication object's link row and is linked once in the direction shown there; the keyboard selects stay. | Structural drag gestures (lines, parts, ranges) remain select-only; native WebKitGTK drag is not verified. |
| UX-02 | Delivered supported-only catalog picker at 6c16fe5a. | `.vd2` backend refusal remains deliberate. |
| UX-03 | Delivered project style selector through the existing command-backed route at 6c16fe5a. | Unchanged/unknown/refused/pending/Undo cases remain covered. |

## Evidence and native qualifications

Focused restored run: five Web files, 230 tests passed. The preceding picker /
restyle mutation sweep rejected four behavior changes and restored exact source
bytes. The follow-up sweep rejected six more behavior changes: stopped refresh,
failed local edit, queued-write generation, discovery retention, swapped HTTP
serial/MAC and wrong UI medium/status mapping. Each failure was an assertion,
not a compiler error. Focused HTTP discovery and actual unicast UDP loopback
checks pass. The complete candidate gate reran successfully after the CSS
correction; its failed predecessor is not relabelled successful.

Six additional real Chromium fixture cases (English/German, 360/640/1440 px)
verify keyboard expansion, exact raw values, no clipped metadata cells and no
unexpected API/runtime error. Every API request is locally intercepted. This
is metadata-disclosure evidence, not whole-page layout acceptance: a separate
before/after comparison found the same pre-existing invisible `HelpTip` overflow
at 640 px (document 671 px) both with and without metadata. The description is
intentionally kept in the accessibility tree; hiding it with `display:none`
would violate the current contract. A viewport-safe placement fix needs its
own keyboard/description regression, not a weakened global layout assertion.

Local GJS/Gtk/WebKitGTK **2.52.6** loaded only the static device-editor fixture
from the owned Vite server. Under `GDK_BACKEND=x11`, nine cases passed: viewport
360/640/1440 px, each at CSS zoom 0.8/1/1.5. Each checked actual viewport and
absence of document overflow, displayed device octet, two links and six flags.
Computed zoom values matched the requested scales. Optional fixture parameter
GETs were locally intercepted; no Tauri IPC, real backend or KNX operation was
used. Default Wayland failed; an early X11 attempt reported the wrong width,
and a later 641-versus-640 comparison failed before the one-pixel allocation
tolerance was added. Only the final matching-width matrix is valid evidence.
GPU/display warnings remain recorded; DOM/layout success is not a visual audit.

Orca and WebKitWebDriver were absent. No packages, system settings or firewall
rules were changed. No real screenreader, native file chooser, dead-WebView
close, live gateway or full Tauri workflow was accepted. These constraints do
not excuse unrelated offline implementation work and do not authorize writes.

## Continuing work

KL-82 and keyboard/modal/help-tip implementation are delivered with complete
candidate/integrated acceptance and exact publication readback.
After delivery, keep native/Orca, real network, independent samples and unresolved
domain/application contracts open and distinct. Group-address/structural drag,
general multi-installation editing, allocation/repair and other retained absent
behaviors are not silently waived by this package. Do not mark all 24 rows DONE,
create an alpha tag, reopen U13 or infer release consent from this owner receipt.

**Owner checkpoint 2026-10-04.** The user decided that native/live evidence
(KL-79, KL-137, KL-36, KL-133, UI-03, KL-130-ZOOM and the native part of KL-20)
leaves the Alpha scope as `ACCEPTED_BOUNDARY`; these remain disclosed,
unverified boundaries, not claims. MODEL-03 and KL-127 get research first and
close as known gaps without reliable evidence. DATA-03, MODEL-01, MODEL-02,
MODEL-04 and UX-01 are now implementation packages UA2–UA6 in
[goal-ui.md](../goal-ui.md). The [source-ID ledger](status/LEDGER.md) carries
the per-row status (until 2026-10-04 the parent ledger in
[alpha-release-goal.md](../alpha-release-goal.md) did).

**Handoff 2026-10-04 11:27.** The backend halves of MODEL-01/02/03/04 and DATA-03 are
published (ADR-0069, ADR-0070, ADR-0071, RESEARCH §25); UX-01 needs no
backend change. By user decision every remaining web half is handed over to
the Web-lock holder (commissioning session); the task table with API
contracts and acceptance criteria is the *UI owner handoff* in the parent
ledger. None of these rows is `DONE` before its web half is published.

## Owner status history

Moved verbatim from `goal-ui.md` on 2026-10-04 (AR14D D5, agreed by the
goal-ui owner); only relative links changed. Every ID and status named here
has its row in the [source-ID ledger](status/LEDGER.md), which is current.

### Where things stood (goal-ui.md, updated 2026-10-02)

U0–U12's UI slices are delivered: host/port discovery fields, catalog and
device/structure editors, channel labels, monitor control, read-only device
checks, Site/Property creation and the ADR-0051 Debug property action.
The Debug route remains default-off and its durable backup is *property-only*;
no new live device check or whole-image recovery follows. K6 confirmed public
address writes now refuse before a tunnel without device-specific durable
recovery (ADR-0059); the Web tab shows this availability rather than asking
for consent prematurely. CLI/HTTP discovery succeeded after the user's
firewall rule; native WebKitGTK Search remains unverified. ISSUE-04 and both
ISSUE-12 acceptance rows are verified and ticked. **U0–U13 are complete**:
the operator accepted the independent GPT-6.1-Sol review because Claude was
unavailable; its three P1 findings are fixed with behavioral RED/GREEN and
restored guard mutations. Actual offline UDP discovery roundtrip/no-response
tests close the remaining transport-evidence gap, not the real-network or
native Search boundaries. Integrated gates: 139 Rust suites / 2,820 passed /
zero failed / 161 ignored / zero corpus skips; Web 82 files / 1,312 tests;
30 mock-only Chromium tests, strict Clippy/fmt/type/build/repository gates
green. Evidence: `.ai/logs/2026-10-01_codex_ui-u13-fixes.md`.
The current top of `.ai/CURRENT_STATE.md` owns the Web-lock/publication state.

#### UI-owned alpha follow-up (user request, 2026-10-02)

The user separately requested the `goal-ui` items from
`docs/ALPHA_READINESS.md`. This does not reopen U0–U13 or authorize commissioning.
All 24 routed rows have a current-source audit in
[UI_ALPHA_READINESS](UI_ALPHA_READINESS.md). Four concrete gaps are
implemented and fully offline-gated: UX-02 supported-only catalog picker,
UX-03 command-backed project style selector, KL-121 cross-client settings
refresh, and KL-124 lossless Device Info projection/disclosure. Publication
`6c16fe5a` and complete remote/tree readback are verified; native/multicast/AT
and domain-dependency qualifications stay explicit.
KL-82's authoritative interpretation comparison, fail-closed uncertainty and
pause/cursor/race guards are implemented; twelve complete candidate gates pass
(Web 1,343, Chromium 35, Rust 2,890 / zero failed / 163 ignored). Integrated
acceptance repeated with the same counts on published `8ceacf49`; remote
ref/tree/twenty artifacts and zero outgoing range verified. No hardware or
transactional write proof follows. The separately reserved `ui-alpha-keyboard`
candidate now implements list auto-scroll, stacked/dynamic modal-background
exclusion and viewport-safe HelpTip with a permanent local description.
Seventeen mocked Chromium cases and eighteen restored behavioral controls pass;
the new hook is demonstrably checked by TypeScript. Twelve renewed candidate
gates pass (Web 1,357, Chromium 52, Rust 2,890 / zero failed / 163 ignored,
576-source freeze); first header failure remains recorded. All twelve gates
repeated on integrated source as proc_490a156044df with the same counts; published
2e57f8e5 and exact ref/tree/all 28 artifacts/zero outgoing commits verified.
No ready keyboard/modal/help-tip implementation remains in this package.
Documentation receipt/owned cleanup follow. Native/Orca, real-network,
independent-sample and domain/application dependencies remain separate and open.
Do not turn retained design boundaries into silently accepted alpha exceptions.
