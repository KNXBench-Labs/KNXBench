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
no runtime tests. A metadata-only withheld projection is a bounded possible next
increment, not closure of the requested raw evidence. The
[source-grounded boundary](../research/2026-10-29-system2-bundle-credential-boundary.md)
records the two server hooks, missing CLI typed attachment and fourteen future,
unexecuted acceptance cases. Final integrated review/gate, main-first delivery
and task-owned cleanup remain open.

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
