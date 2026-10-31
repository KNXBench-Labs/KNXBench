# DIAG-03 — One report button, full diagnostic bundle, operation recorder

**Priority:** P1. **Status:** In progress, local candidate; not delivered.
All original owner decisions remain recorded below (29 October 2026, rounds 1–2,
O1/O2). The Alpha 18 tag prerequisite is met (`80386a154`); the tag stays immutable.
Main-first delivery still requires final review, the integrated package gate and
publication verification. No such acceptance or push is implied here.

**29 October progress:** the opt-in saved-project adapter `aef35f4b1` is locally
integrated as `f017dc248`: existing native-copy helper, saved rather than unsaved
state, unchanged privacy/context/review/consent guards, twelve synthetic HTTP
cases through native reopen. Owning server execution: 1,493 passed, 67 ignored;
this is not a complete package/browser/hardware gate.

**Raw System-2 evidence admission remains NOT READY:** classification of both
expected/readback bytes (P1) and consistency across overlapping recording,
range/error and archive copies (P2) are unproved. The separate static scout ran
no runtime tests. The current App source adds a bounded admission layer around the
closed metadata reader: one consumed snapshot, fixed format/kind, one Download and
optional ordered Restore consistent with the same original progress, and one
deterministic serialization capped at 4 KiB. It also adds narrow bundle-only
admission entry points for historical import reports and saved native projects.
Those entry points reuse the existing bounded sanitizer, refuse the whole source
instance generically when recognized credential material was observed before
mutation, and exempt only the documented free-access `BCUKey` sentinel. Ordinary
helper success is not bundle admission, and the short-credential witness is not
arbitrary opaque/encoded secrecy certification.

Bounded App-only verification completed on 30 October 2026. The unavailable
scaffold ran 92 focused tests (81 passed and 11 intended positive-admission cases
failed), the first implementation passed 92/92, the nine added edge cases produced
the intended 94-pass/7-fail result, and corrected source passed 101/101. A single
temporary C10 final-gate bypass produced 26 passes and 75 intended
credential-refusal failures: all 24 `Password123` body/`source_path`/`xpath`
literal, hex, base64 and percent witnesses failed, while all seven benign controls
passed. Byte-exact production restoration was rebuilt and passed 101/101 again.

The default App run passed 431 library tests, 584 integration tests and nine
doctests (1,024 total without adding focused/default overlap); 47 integration tests
remained ignored. Six new compile-fail doctests emitted `E0277` for the intended
absence of `Serialize`/`Debug` on the actual newtypes. App all-target Clippy with
`-D warnings` covered library, library-test, example and integration targets, and
formatting, header, layering, test-layout, documentation, local-info, anchor, diff,
allowlist and guard checks passed without suppressions. Ignored, corpus and hardware
cases were not executed. This verifies the bounded App foundation only and is not
independent approval of consumers, release or publication.

The fresh independent App-foundation review on 30 October 2026 returned
**APPROVE: 0 Critical, 0 Important, 0 Minor**. Its exact 16-path candidate is
committed as `9e1622795` and locally integrated as `02bea6405`; the merge has the
same source tree. Existing source-bound App execution is retained without a
cosmetic replay. This closes only the C05+C10 foundation review, not the consumer,
Raw P1/P2, original matrix, integrated package or publication gates.

Public DTOs and journal grammar remain unchanged. A bounded local consumer
candidate now attaches the admitted closed projection at both server hooks and in
the System-2 CLI simulator path, and uses admitted saved-project/import-report
copies in text, review and ZIP output. It also adds a strict Web decoder and
English/German lifecycle labels. This is not closure of the requested raw evidence
or the fourteen-case whole-boundary matrix. The
[source-grounded boundary](../research/2026-10-29-system2-bundle-credential-boundary.md)
records the still-open Raw P1/P2 and original fourteen-case acceptance matrix.
Retained earlier-source execution includes the history RED, focused/server-library
and CLI suites, Clippy/format/statics, Web Vitest/type/build and an intercepted-API
Playwright feature lane. Those receipts do not prove the current complete server
default target set green.

**30 October full-target result:** four manual integration-test `AppState`
initializers were missing `tunnel_return_path`; precisely those four enum-field
additions repaired the compile failure without production or assertion changes.
Their fresh independent source review returned **SOURCE_APPROVE, 0/0/0**.
The ensuing full server default run compiled and passed 740 library tests,
five binary tests and 763 integration tests, but failed nine integration tests;
15 library and 52 integration cases remained ignored. The M01 route regression
passed in this same current-source default run. Formatting passed. No subsequent
CLI default, owning Clippy or static stage was executed after the first failing
command. These results are not a green verification chain.

The nine failures expose the remaining retained read-diagnostics/support evidence
admission gap: the collector deliberately withholds the unadmitted reports/device
source, while the target still requires retained results without a recording,
correct originating-device attribution and exact report/property/memory content.
They must not be dismissed as stale fixtures or made green by releasing
unclassified raw bytes or weakening assertions. Next is bounded positive
admission under the unchanged credential-exclusion contract. Raw P1/P2 and
Original14 remain open, as do the real-router/built-bundle captured join and
screenshots, final independent review/package gate, main-first delivery and
publication.

**30 October retained-result candidate:** source now contains a narrow admission
for the generic identity profile and projectless/manual universal-support reports.
Actual focused policy execution produced the intended two App privacy failures and
six Server history/availability failures while 16 controls passed. The corrected
source now refuses unclassified partial/refusal payload bytes and arbitrary format-1
reason text before writer materialisation, uses the diagnostic writer's own strict
readback once, validates support before one writer invocation, and moves each owned
file vector once into preview/text/review/ZIP sections. Historical typed results use
a bounded single-snapshot read and bind copied header, token, target and optional
context. Missing or unproven attribution retains only the independently admitted
report; it never enables device or catalogue projection. Direct counted reader
controls for exact limits, truncation, growth overflow and path rejection are
authored but not yet executed. Focused GREEN, defaults, Clippy and static gates
have not yet run on these corrected bytes. **31 October owner decision and follow-up:** declared memory and raw
non-System-2 recordings now leave after consent with a "possibly passwords; keep
private" note (recognised keys stay redacted at capture); they remain
unclassified. Device/catalogue extracts are restored for proven-current-context
results and for an explicitly chosen device without any source. Support lookups
report support-specific causes. Executed on the final candidate source
(31 October, isolated loopback/synthetic, single job): App default 1,045 passed /
47 ignored; Server default 1,538 passed / 67 ignored, **including all nine
original target tests**; CLI default 437 passed / 21 ignored; fmt, owning Clippy
`-D warnings`, layering/headers/anchors/ledger, documentation and local-info
checks green. Fresh independent read-only reviews went from CHANGES_REQUIRED
(0 critical / 5 important / 5 minor) through two correction rounds to APPROVE
0/0/0. Still open before delivery: the integrated main-first package gate, the
real router/built-web join with screenshots, CLI installed-binary boundaries,
raw System-2 P1/P2, Original14 and publication.

**31 October real join and installed CLI:** on `origin/main` `2bec7dd8a` a
private loopback namespace ran a simulator gateway, the real server and the
production web build. A real serial-number lookup was recorded; the wizard
(problem type programming) picked it by default and exported a ZIP whose
`recording.jsonl` was byte-identical to the server's file, with sent/received
datagrams equal to the simulator's capture and the header version equal to
`GET /api/version`. The mail draft was checked but not launched. English 1440 px
and German 400 px passes left 36 screenshots. The run exposed three defects,
fixed in the follow-up: every undeliverable section said "the selected
diagnostic source is unavailable" (also for the withheld session log, bus
monitor and client state, an unsaved project, or a lookup that names no device);
German showed that reason and the keep-private note in English; and the CLI
bundle dropped every non-System-2 recording despite the 31 October decision,
adding a misleading System-2 note. A toast no longer paints over the wizard's
buttons: the wizard sits above the toast stack, which is inert behind it anyway
(pixel comparison at 1440/400 px, since hit tests skip inert elements). The
trade-off: toasts and the autosave countdown that appear while the wizard is
open sit dimmed behind it and their timers keep running; errors stay in the
notification log. Still open, and older than this fix: in the desktop shell,
closing the window with unsaved changes while the wizard is open opens the quit
confirmation underneath the wizard. The installed CLI's flag boundaries passed:
no flag writes nothing, doubled/empty/misplaced flags and a repeated invocation
are refused before any datagram.

The parent-reported bounded R4 ambient-identity prerequisite is independently
approved for the real preview/unselected-recording regression. It is prerequisite
closure only and does not authorize raw export. Consumer source is a local
candidate; Raw P1/P2, Original14, independent integration/publication and the whole
DIAG-03 goal remain open.

Origin: issue #30 report analysis, owner request 29 October 2026.

## Why

Every field failure so far needed another report extension and another device attempt
(#30: pointer width, busy reason, tunnel cause, final-step category, now the allocation
mismatch). At the checked starting point below, System-2 final verification discarded
actual readback, so the #30/#22 mismatch could not be localised from existing artifacts.
Part B now persists the private comparison evidence through journal and fresh reopen;
its credential-safe bundle/CLI attachment remains open under P1/P2. The owner's decision:
for debugging, data minimisation is counterproductive. Record as much as possible for
every device operation, always with the KNXBench version, and let the user decide by
checkbox what goes into one ZIP that they may send by mail.

## Checked starting point (release candidate `80386a154`)

- Report-like entry points today, each with its own privacy rule and handoff:
  `DebugReportButton.tsx` (File menu, local ZIP: report.md, environment.json, log,
  project summary, bus telegrams), `UniversalSupportButton.tsx` (ADR-0131 support report),
  `DiagnosticExportButton.tsx` (ADR-0134 read-only diagnostic ZIP), download report
  "Help verify this device" (ADR-0121), DIAG-02 failure diagnostic, System-2 summary
  (`Bericht speichern`, contract `system2-download-result.md`), `DownloadSupportNotice.tsx`.
  Mail links to `issue@knxbench.com` are scattered over four components.
- No component records wire traffic of a device operation. `knx-net` has no logging;
  `ManagementTransport` (`crates/knx-net/src/management.rs`) is the single tap point
  every management session passes through.
- `PropertyRecoverySnapshot::verify_outcome` returns a generic error and drops the actual
  bytes; the journal keeps only `local_diagnostic` text.
- Not in scope as "reports": `ContributionButton` (product contribution, own contract),
  `MonitoringExportButton`, `DocumentationExportButton` (exports, not diagnostics).

## Owner decisions (29 October 2026, all recommendations accepted)

1. **One diagnostic bundle** instead of extending five report types. Existing reports
   become sections inside it.
2. **Recorded operations:** every operation that opens a device connection — read/
   diagnostics, ordinary download, System-2 download and recovery, compare, individual
   address programming, line scan. The free-running bus monitor is not recorded wholesale;
   only telegrams belonging to the recorded operation.
3. **Content: everything except secrets.** Version, git commit, build profile, OS/container,
   effective timeouts and settings, gateway identity and endpoint, every telegram with
   monotonic and wall-clock timestamps (raw hex and decoded), tunnel events (ACK status,
   sequence numbers, heartbeat, disconnect cause), the full plan, expected/actual bytes for
   every comparison, complete error chains, addresses and names.
   **Always redacted, even with consent:** BCU keys (`A_Authorize`, `A_Key_Write`),
   KNX Secure keys and tool keys, tunnel/login passwords. Recorded as
   `"redacted": "authorize-key", "octets": 4`, never the value.
4. **Project data:** the affected device's extract (application, all parameter values,
   linked group addresses) is part of the bundle; the whole project file only through its
   own checkbox, default off.
5. **Local retention:** recording is always on; keep the last 100 operations or 512 MB,
   whichever is reached first, oldest first; failed operations are evicted last. UI action
   "Delete recordings".
6. **Handoff:** save ZIP locally plus a prefilled mail draft to `issue@knxbench.com`
   (subject with version and operation id). The user attaches the ZIP; no upload, no
   receiver service. Export needs a content preview and an explicit consent checkbox.
7. **Public minimised reports stay** for GitHub issues; the full bundle is for mail only.
   New ADR documents this two-tier rule and supersedes the relevant privacy clauses of
   ADR-0121/0131/0134 for the mail bundle only.
8. **Shared core, all front ends:** recorder in `knx-net`/`knx-app`; server, web and CLI
   use it (`--diagnostic-bundle <file>`); MCP later.
9. **Exactly one report button** in the UI. It opens a wizard where the user selects by
   checkbox what goes into the ZIP, with sensible defaults preselected per problem. The
   wizard is modular: sections depend on the problem type. All other report/save-report/
   mail buttons are removed or become shortcuts that open this wizard preselected
   (e.g. from a failed download panel: problem "download failed", that operation selected).

### Round 2 (29 October 2026, all recommendations accepted)

10. **Format:** JSONL, one file per operation, one event per line, each telegram raw hex
    plus decoded. A pcap converter may follow later; not part of this package.
11. **Location:** data directory `diagnostics/`, next to the operation journals, so Docker
    keeps recordings on the mounted volume across restarts.
12. **Old recordings after an update:** kept and exported unchanged; every file header
    carries its format version and the writing KNXBench version. No backfill.
13. **Switching off:** no UI toggle, only "Delete recordings". A documented environment
    variable lets operators of shared/customer servers disable recording; the bundle and
    the UI state plainly when recording is disabled.
14. **Bus monitor:** extra checkbox "bus monitor, last 10 minutes", default off, only when
    the monitor ran; uses its existing buffer.
15. **Size:** live size per section and total in step 2; hint above 10 MB about common mail
    limits; no splitting.
16. **Problem types:** device programming; device reading/diagnostics; project import;
    product catalogue/product installation; user interface/other. Each has its own
    defaults (open item O1).
17. **Preview:** per section size, entry count and content markers ("contains addresses",
    "contains names", "contains parameter values"); one click shows the section as text.
18. **Who may export:** the same authenticated session as every other route; desktop and
    server alike; no new roles.

### O1/O2 (29 October 2026, owner accepted both proposals)

The tables below are binding defaults for the implementation.

**O1 — defaults per problem type** (✓ preselected, ○ offered, – not offered):

| Section | Programming | Reading/diag. | Import | Catalogue | UI/other |
| --- | --- | --- | --- | --- | --- |
| manifest + environment (version, commit, OS, settings) | ✓ fixed | ✓ fixed | ✓ fixed | ✓ fixed | ✓ fixed |
| session log | ✓ | ✓ | ✓ | ✓ | ✓ |
| latest matching operation recording + typed result/journal projection | ✓ | ✓ | – | – | ○ |
| earlier operations (picker) | ○ | ○ | – | – | ○ |
| device extract (application, parameters, linked GAs) | ✓ | ✓ | – | ○ | ○ |
| minimised legacy reports of that operation | ✓ | ✓ | – | – | – |
| import report and import diagnostics | – | – | ✓ | – | ○ |
| original import file (`.knxproj`) | – | – | ○ | – | – |
| catalogue install report / product identity | ○ | ○ | – | ✓ | – |
| whole project `.knxdb` | ○ | ○ | ○ | – | ○ |
| bus monitor last 10 min | ○ | ○ | – | – | ○ |
| browser/client state (UI language, theme, viewport, recent errors) | – | – | – | – | ✓ |

**O2 — disposition of existing entry points:**

| Today | Decision |
| --- | --- |
| `DebugReportButton` (File menu) | Becomes the single "Report problem" entry that opens the wizard |
| `UniversalSupportButton` | Shortcut: wizard, type programming/reading, device preselected |
| `DiagnosticExportButton` | Keeps the *live read* action (it performs the read-only acquisition); its ZIP export moves into the wizard |
| Download "Help verify this device" | Shortcut: wizard, type programming, that run preselected; also used for successful runs (verification evidence) |
| System-2 "Save report" | Shortcut: wizard, type programming, that operation preselected |
| `DownloadSupportNotice` mail/issue links | Replaced by a wizard link |
| `ContributionButton` | Unchanged: product contribution, separate address and contract, not a problem report |
| `MonitoringExportButton`, `DocumentationExportButton` | Unchanged: exports, not diagnostics |

## Bounded outcome

### A. Operation recorder (core)
- Recording `ManagementTransport` wrapper plus tunnel/session event hooks; one recording
  per operation id, append-only JSONL (one event per line) with a header record
  (format version, KNXBench version/commit, start time, operation kind, target, gateway).
- Recording must never change protocol behaviour or timing materially: bounded buffering,
  writes off the async runtime (lesson from #30/S2-04), recorder failure is recorded as a
  gap and never aborts or delays the device operation.
- Redaction at the source (before the bytes reach the file) for the secret-bearing
  services in decision 3, covered by a sentinel test that scans every produced file.
- Retention per decision 5, crash-safe (a torn last line is reported, not fatal).

### B. Evidence that the current paths drop
- System-2 `verify_outcome`: on mismatch keep allocation index, object, address, length,
  per-differing-range offset, written/unwritten classification, expected and actual bytes
  (full allocation, both sides). Strict verification stays unchanged; this only records.
  Persist through the journal (new envelope version, strict replay rules as for
  `final_step`) and into the bundle.
- Same principle for the ordinary download's verification/compare and the read-only
  diagnostic: actual readback always recorded.
- Memory service consistency (spec check below): choose the service once per allocation
  for write, read and verify; for System 2 refuse an allocation whose end reaches `FFFFh`
  before the first write. Tests on both sides of the boundary.

### C. Bundle and wizard
- Server builds the ZIP from selected sections: `manifest.json` (format, versions, section
  list with sizes and hashes, redaction summary, consent timestamp), `environment.json`,
  `operations/<id>/recording.jsonl` + existing typed results/journal projections,
  `device/<id>.json` extract, optional `project.knxdb`, session log, existing minimised
  reports as they are today.
- Web: one "Report problem" entry (File/Help menu and contextual shortcuts). Wizard steps:
  1) problem type (decision 16); 2) operations and sections with the type's defaults
  (O1), live sizes (decision 15); 3) preview per decision 17, including what is *not*
  redacted; 4) consent checkbox, save ZIP, open mail draft.
- CLI `--diagnostic-bundle <file>` for device commands, same builder.
- Remove/redirect the old buttons in one sweep; keep their file formats readable as
  bundle sections. Public GitHub issue link offers only the minimised report.

### D. Documentation
- New ADR (next free number), contract `docs/contracts/diagnostic-bundle.md`, user guide
  section (EN/DE wording, what is inside, retention, deleting recordings, mail is not
  encrypted), KNOWN_LIMITATIONS/IMPLEMENTATION_STATUS updates, OPEN_WORK row update.

## Spec evidence already established (memory service)

- KNX Standard v3.0.0, 03_05_03 Configuration Procedures v02.01.01, §3.5.2 (printed p43,
  repeated in §3.5.3 from p45): the service is chosen from base address plus *allocated*
  segment size; below `FFFFh` `A_Memory_Write`, above `A_UserMemory_Write`; exactly `FFFFh`
  is not defined. §3.5 is System B.
- Same document §3.5.1.4 Table 5 (p42): without memory above 64 k the client shall not
  use `A_UserMemory_Write`.
- 06 Profiles v02.01.01 §4.2 matrix (p37, rendered): "DMA on User Memory" not marked for
  profile class System 2 (mask `0025h`, Table 1 p13). No mask-`0025h` procedure section
  exists in this edition.
- Field addresses on the UP 258D12 (`0116h`, `0400h`) are far below; this is a consistency
  fix, not the #30 cause.

## Acceptance

- Simulator/loopback runs of every recorded operation kind produce a recording whose
  telegram sequence equals the transport-level frames sent/received (byte-exact), with
  header version = `GET /api/version`.
- Injected System-2 final allocation mismatch (written byte and unwritten byte variants)
  yields the exact offsets and both byte values in journal, reopen and bundle; strict
  failure outcome unchanged. Old journals reopen with the evidence marked unavailable.
- Secret sentinel: authorize/key-write/Secure/password test values never appear in any
  file, ZIP, mail URL or log; the redaction marker does.
- Recorder fault injection (disk full, slow fsync) does not change the device operation's
  outcome or exceed existing timeouts; the gap is visible in the bundle.
- Retention limits and failed-last eviction tested; delete action removes files.
- Wizard: each problem type preselects the documented defaults; nothing is written or
  mailed without consent; the mail draft carries no attachment and no secret; old entry
  points open the wizard preselected. EN/DE, keyboard, 400 px width.
- Service selection tests at `FFFEh`/`FFFFh`/`10000h` for read, write and verify; System 2
  refuses before the first write.
- Package gate on the branch and after the main-first merge; no real bus or device.

## Boundaries

- No hardware or bus contact for development; field use only through the reporter's own
  consented run.
- No upload service, no telemetry, nothing leaves the machine without the user's action.
- Does not relax any programming gate, does not remove the "experimental" System-2 label
  and does not widen device admission (owner decisions deferred until a fully verified
  27/27 run).
- Email is not encryption: the wizard says so plainly.
