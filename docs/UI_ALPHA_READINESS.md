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
| KL-82 | Open: companion freshness still relies on the existing browser-profile record; `busContext.ts`, actual server `GroupAddressContext`. | Compare current authoritative project with the actual session context across clients/restarts; settings refresh does not fix project-context staleness. |
| KL-127 | Site/property Ground workflow exists under ADR-0038; synthetic hierarchy/native storage tests retain unknown types honestly. | Independent ETS Ground/multiple-installation samples remain absent; do not infer ETS semantics from synthetic fixtures. |
| MODEL-01 | Open domain/application dependency: first-installation structural mutations and link creation remain bounded; later installations are preserved. | Installation rename/selection and correctly scoped command/API/history contracts before a general multi-installation editor. |
| MODEL-02 | Retained safe refusal of ambiguous IDs, multiply placed devices and inconsistent topology; original imported values remain intact. | No automatic lossless repair/renumbering workflow; define reference/opaque-data preservation and undo before offering repair. |
| MODEL-03 | Retained line-relative address editor and supported links/flags; imported `.0` remains intact and new `.0` assignment is refused. | Verified normalized coupler discriminator/semantics before a special-address editor; no guessed device classification. |
| KL-133 | Existing close guard protects unsaved project state; normal quit tests are not a dead-renderer test. | Native unresponsive/crashed-WebView reproduction and a data-safe close/recovery policy; never bypass the guard by inference. |
| UI-03 | Retained read-only device-checks UI and explicit unsupported readiness/recovery states; commissioning owns complete recovery. | No full-image backup or universal device semantics follows from the UI; preserve device-specific commissioning prerequisites. |
| KL-130-ZOOM | New native WebKitGTK static Inspector geometry evidence at three widths/scales; Chromium interaction evidence remains distinct. | Full Tauri zoom shortcuts, pane resize/hide/restart and hover workflow are not covered by this static probe. |
| KL-20 | Existing shared modal shell, keyboard trap and list semantics remain; native static geometry is not an accessibility audit. | List auto-scroll/background virtual-cursor exclusion and real screenreader/native modal interaction remain open. |
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

After this candidate is reviewed/gated/delivered, prioritize the open KL-82
cross-client context contract, then remaining concrete keyboard/modal gaps.
Keep domain/sample-dependent rows and deliberate bounded features distinct from
those ready UI changes. Do not mark all 24 rows DONE, create an alpha tag, reopen
U13, or accept release exceptions merely because this owner receipt exists.
