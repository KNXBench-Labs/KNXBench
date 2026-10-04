# UI alpha-readiness owner audit and follow-ups

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

These are the exact 24 IDs from the parent ledger. `Open` names absent behavior,
not merely missing validation. `Retained` describes the current contract, not
release consent. Final parent-ledger status follows verified delivery.

| ID | Disposition and current evidence | Remaining condition |
| --- | --- | --- |
| KL-79 | Existing offline UDP exchange and UI discovery remain verified at U13 scope; new metadata projection has focused coverage. | Native Search click and real multicast/firewall paths are not proved by fixtures; no deliberate live Search was accepted. The old-harness attempt has the unretained-traffic qualification above. |
| UI-01 | Already DONE in U13; no duplicate implementation. | Preserve original native/network qualifications. |
| UI-02 | Already DONE in U13; no duplicate implementation. | Preserve original native/network qualifications. |
| DATA-03 | Retained atomic catalog batch, one undo step, no blind retry after ambiguous response; `CatalogBrowser` and current batch routes. | No server replay/idempotency contract or ability to roll back a different legacy server; separate application/API design, not two frontend requests. |
| KL-137 | Retained bounded monitor capture, explicit server/client dropped counters and 16 MiB export bound. | Native dialog and full retained-window workflow remain unverified; full-history streaming needs its own privacy/storage design. |
| KL-36 | Retained searchable/exportable bounded session log and atomic native writer, ADR-0047. | No reconstruction of evicted entries or lifetime audit; native chooser acceptance remains open. |
| KL-82 | Delivered authoritative interpretation comparison, unavailable/legacy fail-closed state, pause/cursor and delayed-reply regressions at 8ceacf49; twelve integrated gates and eleven behavioral controls verified. | Point-in-time interpretation is not collaboration, historical reinterpretation, native/live-bus evidence or transaction-bound write authorization; retained §82 boundaries remain explicit. |
| KL-127 | Site/property Ground workflow exists under ADR-0038; synthetic hierarchy/native storage tests retain unknown types honestly. | Independent ETS Ground/multiple-installation samples remain absent; do not infer ETS semantics from synthetic fixtures. |
| MODEL-01 | Open domain/application dependency: first-installation structural mutations and link creation remain bounded; later installations are preserved. | Installation rename/selection and correctly scoped command/API/history contracts before a general multi-installation editor. |
| MODEL-02 | Retained safe refusal of ambiguous IDs, multiply placed devices and inconsistent topology; original imported values remain intact. | No automatic lossless repair/renumbering workflow; define reference/opaque-data preservation and undo before offering repair. |
| MODEL-03 | Retained line-relative address editor and supported links/flags; imported `.0` remains intact and new `.0` assignment is refused. | Verified normalized coupler discriminator/semantics before a special-address editor; no guessed device classification. |
| KL-133 | Existing close guard protects unsaved project state; normal quit tests are not a dead-renderer test. | Native unresponsive/crashed-WebView reproduction and a data-safe close/recovery policy; never bypass the guard by inference. |
| UI-03 | Retained read-only device-checks UI and explicit unsupported readiness/recovery states; commissioning owns complete recovery. | No full-image backup or universal device semantics follows from the UI; preserve device-specific commissioning prerequisites. |
| KL-130-ZOOM | New native WebKitGTK static Inspector geometry evidence at three widths/scales; Chromium interaction evidence remains distinct. | Full Tauri zoom shortcuts, pane resize/hide/restart and hover workflow are not covered by this static probe. |
| KL-20 | Delivered at 2e57f8e5: active-list scrolling, background inert/AX exclusion, stacked/dynamic modal ownership and viewport-safe HelpTip; twelve integrated gates and eighteen behavioral controls verified. | Actual native/Orca interaction and whole-app visual accessibility audit remain open, not a release waiver or full-source acceptance. |
| KL-124 | Delivered raw Device Info retention, exact nullable HTTP projection, six labelled UI values and explicit unavailable state at 6c16fe5a. | Native/live Search remains KL-79, not new protocol or identity acceptance. |
| MODEL-04 | Retained local 1–32-device batch with deterministic indexed names and no spontaneous individual-address allocation. | Unique-name/address-allocation policy and core validation before a new opt-in allocation workflow. |
| KL-121 | Delivered authoritative cross-client settings refresh with write-generation and cleanup regressions at 6c16fe5a. | Not general project collaboration or instantaneous synchronization. |
| KL-43 | Retained global motion level/style and OS-reduced-motion precedence; existing guard scope remains explicit. | Per-category motion/parser-backed wider guards require separate scope; no real-animation or assistive-technology conformance claim. |
| KL-97 | Retained truthful phase/count progress under ADR-0023. | No guessed percentage for streaming work whose total is not known. |
| KL-98 | Retained decorative, accessibility-excluded flavour text; slow rotation is intentional. | No artificial slowing or invented progress merely to show more jokes. |
| UX-01 | Retained two validated device drag gestures with keyboard selects, GAP_ANALYSIS_ETS B10. | Group-address-to-object/structural drag gestures remain absent; not implied by the existing two gestures. |
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
[goal-ui.md](../goal-ui.md). The parent ledger in
[alpha-release-goal.md](../alpha-release-goal.md) carries the per-row status.

**Handoff 2026-10-04 11:27.** The backend halves of MODEL-01/02/03/04 and DATA-03 are
published (ADR-0069, ADR-0070, ADR-0071, RESEARCH §25); UX-01 needs no
backend change. By user decision every remaining web half is handed over to
the Web-lock holder (commissioning session); the task table with API
contracts and acceptance criteria is the *UI owner handoff* in the parent
ledger. None of these rows is `DONE` before its web half is published.
