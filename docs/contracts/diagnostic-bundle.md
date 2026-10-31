# Operation recordings and the diagnostic bundle (DIAG-03)

Owner: DIAG-03 ([work package](../open-work/DIAG-03-unified-diagnostic-bundle.md),
[ADR-0156](../adr/0156-operation-recorder-and-diagnostic-bundle.md)).

## Candidate implementation and remaining privacy admission

**29 October 2026, local candidate; not a release or whole-package acceptance.**
The opt-in saved-project section now calls `native_project_without_secrets`
(adapter `aef35f4b1`, local integration `f017dc248`). It exports the saved root,
not unsaved working edits; default selection stays off. The helper omits history,
removes its recognized credential fields and reports withheld/unsupported data.
HTTP tests cover preview → review/consent → section/ZIP → actual native reopen,
source/history immutability and review drift. WAL/sidecar refusal is exercised at
the route; budget/oversize, symlink-sidecar and concurrent-writer boundaries have
helper-level coverage only, not new route/browser/hardware acceptance.

The App source now also exposes bundle-only admission foundations for the saved
native project and historical import report. `native_project_for_bundle` and
`import_report_for_bundle` reuse the existing bounded sanitizers and return either
private owned body/notes or one payload-free unavailable result. A private,
non-serialized witness is set before recognized non-empty credential fields and
`BusAccess` credential components are changed; an observed credential refuses the
whole source instance before body or notes leave the helper. The documented free-
access `BCUKey` sentinel alone is exempt. This is a narrow source-instance witness,
not arbitrary opaque or encoded secrecy certification. Ordinary helper success is
not bundle admission. A bounded 30 October consumer candidate now adopts these
APIs in the server and CLI: the helper-owned copy is moved once into the collector,
and the same admitted bytes feed text, review and ZIP output. This is local source
and execution evidence, not independent review, integration acceptance or
publication.

**Private System-2 comparison bytes remain unfinished.** Replay validates the
recorded comparison, not credential freedom of arbitrary expected/readback
memory. Consent cannot override unconditional key/password exclusion. Full raw
export requires both byte-side classification and consistent handling of every
overlapping recording/range/error/archive copy. The current APDU key redactor
and project-copy helper do not establish that wider guarantee. The existing
App-only metadata reader is now paired in source with `admit_snapshot`. It consumes
one available snapshot, checks the fixed format/kind, exactly one Download plus an
optional ordered Restore consistent with the same progress, and serializes the
closed projection once with a 4 KiB maximum. It returns the original owned minimized
progress unchanged beside private projection bytes; the admitted type implements
neither `Serialize` nor `Debug`. The bounded consumer candidate attaches this
closed projection at both server hooks and in the System-2 CLI simulator path.
Raw comparison arrays, recording bodies and arbitrary report fields remain
unadmitted and unavailable.
This remains explicitly incomplete: neither raw-evidence delivery nor
credential-safety certification of the rest of the ZIP is implied.
[Source evidence, P1/P2 and proposed acceptance cases](../research/2026-10-29-system2-bundle-credential-boundary.md).

Bounded App-only verification completed on 30 October 2026. The unavailable
scaffold ran 92 focused tests (81 passed and the 11 positive-admission cases failed
as intended); the first implementation passed all 92. Nine added edge cases then
produced the intended 94-pass/7-fail regression result, and the correction passed
101/101. A single temporary C10 final-gate bypass produced 26 passes and 75 intended
credential-refusal failures: all 24 `Password123` body/`source_path`/`xpath`
witnesses across literal, hex, base64 and percent forms failed while all seven
benign controls passed. Byte-exact production restoration was rebuilt and again
passed 101/101.

The default App run passed 431 library tests, 584 integration tests and nine
doctests (1,024 total, without adding the overlapping focused run); 47 integration
tests remained ignored. Six new trait-boundary compile-fail doctests produced
`E0277` for the intended absence of `Serialize`/`Debug` on the actual newtypes.
App all-target Clippy with `-D warnings` covered the library, library tests, example
and integration target, and the formatting, header, layering, test-layout,
documentation, local-info, anchor, diff, allowlist and guard checks passed without
suppressions. Ignored, corpus and hardware cases were not executed. These results
verify this App foundation only; they are not independent approval of consumers,
release or the whole package.

The parent-reported bounded R4 ambient-identity prerequisite is independently
approved for the real preview/unselected-recording regression. That prerequisite
result does not authorize export or close any consumer, raw-evidence or whole-goal
acceptance.

Earlier consumer receipts cover focused/server-library and CLI runs, strict
Clippy/format/static checks, Web Vitest/type/build and an intercepted-API
Playwright feature lane. They are earlier-source evidence, not acceptance of the
current complete server target set. The Web decoder accepts only the closed
six-field recording summary and known, active-missing, inactive-missing and
withheld ends in English and German.

**30 October full-target recheck: not green.** After four initializer-only test
fixture repairs, formatting passed and the full server default run compiled. Its
library target passed 740 tests (15 ignored), the binary target passed five, and
the integration target passed 763 but failed nine (52 ignored). The current M01
route regression passed in that same default run. All nine failures concern
retained read-diagnostics/support evidence and its originating-device/report
handoff. The collector deliberately withholds those still-unadmitted sources;
the positive retained-result requirements below remain unfulfilled, not obsolete.
Neither unsafe raw-export restoration nor assertion weakening is an acceptance
route. The four initializer additions have an independent source-only approval
with no findings; that approval is not a runtime or whole-package approval.
CLI default, owning Clippy and later static checks were not executed after this
first failing command. Counts overlap earlier selections. Hardware and private
corpus were not accessed. Raw P1/P2, Original14, the real-router/built-Web join,
independent integrated package acceptance and publication remain open.

The requirements below remain the target contract, not a claim that every part
has already passed integrated acceptance.

This contract describes two artefacts: the **operation recording**, a JSONL file
written for every device operation that opens a KNXnet/IP tunnel, and the
**diagnostic bundle**, a ZIP the user builds deliberately and sends by mail.

## Operation recordings

### Which operations are recorded

Server: ordinary download, compare, read-only device diagnostics, the support
live read, System-2 download and recovery, service-control read and write,
serial-number lookup and write, and line scans. Every route that opens a tunnel
goes through `operation_recordings` in `knx-server`. Routes that refuse before a
tunnel opens (individual-address programming today) leave no recording.

CLI: only with `--diagnostic-bundle <new.zip>`, see [CLI](#cli).

The free-running bus monitor is not recorded. Its last-ten-minutes section is
offered but withheld until its content is admitted as credential-free.

### Location, switching off, retention

- Server: `<data dir>/diagnostics/<operation id>.jsonl`. The directory is reserved:
  the file routes (`/api/fs`) neither list nor resolve it. Without `KNX_DATA_DIR`
  the server's data directory falls back to the system temporary directory, and
  so do the recordings; the server Docker image sets `KNX_DATA_DIR=/data`.
- `KNX_DIAGNOSTIC_RECORDING=off` (also `0`, `false`, `disabled`; case-insensitive)
  disables recording at server start. Existing recordings remain listable,
  exportable and deletable; the wizard and every bundle manifest say plainly
  that recording is off.
- Retention after each finished recording: at most 100 recordings and 512 MiB.
  Successful recordings are evicted before failed ones, oldest first within each
  group. A recording still being written is never evicted or deleted.
- `DELETE /api/diagnostics/recordings` (wizard: "Delete recordings") removes every
  finished recording and reports the active ones it kept.

### Writer guarantees

- The tap in `knx_net::wire_tap` observes datagrams; it never alters, delays or
  fails them. An outgoing datagram is recorded immediately before it is handed
  to the socket, under a per-tunnel order lock that exists only when a tap is
  attached, so an answer can never appear ahead of its request; a failed send
  follows as a `socket-error` tunnel event. An incoming datagram is recorded
  right after it was received, including datagrams the client then discards.
- Records go into a bounded queue (`try_send`). One writer thread per recording
  does all file I/O. A full queue or a failing write never reaches the device
  operation; it becomes a counted `gap` line and the `end` line's counters.
- A recording that cannot start (directory not creatable, disabled) leaves the
  operation unchanged; the reason is kept and reported.
- A torn last line (process killed mid-write) is reported as `tornTail` when the
  recording is listed and noted in the bundle; it is not an error.

### Line format (`knxbench-operation-recording`, format version 1)

One JSON object per line. Every line has `type`; event lines have `seq`
(per-recording counter), `t_us` (microseconds since the recording started,
monotonic) and `wall` (UTC RFC 3339 with milliseconds).

| `type` | Content |
| --- | --- |
| `header` | first line: `format`, `format_version`, `operation_id`, `operation_kind`, `started_at`, `writer` (`program`, `version` = `GET /api/version`, `commit`, `build_profile`), `platform` (OS, family, architecture, container flag; never host or user name), `target`, `gateway`, `related` ids (`downloadId`, `operationId`, `diagnosticsToken`, `serverIncarnation`, ...), effective `settings`, `redaction` policy |
| `datagram` | `dir` (`sent`/`received`), `octets`, `raw` (lower-case hex of the exact UDP payload), decoded `service`, `channel`, `tunnel_seq`, `status`, and for cEMI frames `message`, `confirm_error`, `source`, `destination`, `tpci`, `apci`, `summary`; `redacted` when key octets were removed |
| `tunnel` | connection events: connect refused, ACK timeout and repeat, heartbeat attempts, discarded frames (foreign channel, duplicate), close reason, socket errors |
| `note` | operation-level facts: plan, step results, full error chains, results projected from typed reports |
| `gap` | `lost_records` and `reason` when records could not be written |
| `end` | last line: `outcome` (`succeeded`, `failed`, `cancelled`, or `abandoned` when the operation ended without reporting one), `error_chain`, `summary`, `records_written`, `records_dropped`, `failed_writes` |

Older recordings are kept and exported unchanged after an update; readers use
`format_version` and never backfill.

### Redaction

Always applied, also with the user's consent, before a record reaches the
writer: the 4 key octets of `A_Authorize_Request` and `A_Key_Write` are zeroed
in `raw` and named by `"redacted": {"kind": "authorize-key" | "key-write-key",
"offset": n, "octets": 4}`. KNXBench implements only unsecured KNXnet/IP tunnelling,
so no tunnel password or KNX Secure session key is on the wire; project copies in the bundle are made
secret-free separately (see [Project](#sections)).

## Diagnostic bundle

### HTTP

All routes use the authenticated session; none opens a device connection.

| Route | Purpose |
| --- | --- |
| `GET /api/diagnostics/recordings` | `{enabled, recordings: RecordingSummary[]}` newest first |
| `DELETE /api/diagnostics/recordings` | `{removed, keptActive}` |
| `POST /api/diagnostics/bundle/preview` | sections the problem type offers, availability with reasons, sizes, entry counts, content markers, notes, file name, mail draft |
| `POST /api/diagnostics/bundle/section` | one section (`key`) as text |
| `POST /api/diagnostics/bundle` | the ZIP; refused with `bundleConsentRequired` unless `consent: true` |

The request names `problemType`, optionally `operation` (`{recordingId}` or
`{relatedKey, relatedValue}`, for example `downloadId` of this server run or a
System-2 `operationId`), `earlierRecordingIds`, `deviceId`, `sections` (absent:
the problem type's defaults), `client` (browser state, at most 64 KiB) and
`description` (at most 8000 characters). Recording ids are validated before any
file access. A section the problem type does not offer is refused.

### Reviewed-content binding (required before acceptance)

The initial candidate's live re-collection is not sufficient for consent. The
following wire contract is required by the review correction, before DIAG-03 can
be accepted:

- `ProjectContext` is `{serverIncarnation, projectIncarnation, revision}`. An
  initial preview request may carry `context` when its choices came from a bound
  project tree; a mismatching context is refused, not applied to current ids.
- Each preview returns mandatory `review: {fingerprint, context}`. `fingerprint`
  is lower-case SHA-256 over a deterministic description of the **effective**
  problem type, resolved operation/result and device, selected-section
  availability, metadata and exact file contents. It also binds the observed
  server/project context. Manifest creation/consent timestamps and the user's
  final description are not section contents and are excluded.
- Section-text and consented export requests must carry that `review` object.
  Missing review is `bundlePreviewRequired`; a changed context or fingerprint
  is HTTP 409 `bundlePreviewChanged`. No ZIP or section text is returned on
  refusal. No new access roles or automatic upload are introduced.
- Collection must observe a stable context from start to end, and the bytes
  compared with the review must be the very bytes rendered/exported. A matching
  digest followed by another live read would not establish this guarantee.
- The browser pins the returned `operationRecordingId` and `deviceId` for later
  text/export requests. This is additional identity protection, not a substitute
  for hashing mutable log, monitor, device, import or project contents.
- The existing explicit operation references remain valid even without a
  recording: retained diagnostic jobs, download results and System-2 journals
  are independent evidence sources. Missing/expired results are reported, never
  replaced by an unrelated latest run. `{selection: "none"}` explicitly selects
  no operation for a pre-plan/refusal report; absence still means latest matching.
- Ended-run shortcuts use their captured target, not the next preparation's
  selection. Project-scoped device extracts require a proven matching source
  context; unavailable attribution is disclosed instead of guessing by numeric id.
- A substantive project/server/revision change withdraws browser consent and
  invalidates pending results. An equal tree refresh does not. HTTP validation
  remains necessary for changes made by another authenticated workbench.

### Retained diagnostic results

The `legacyReports` section must preserve existing structured diagnostic files:
`report.json`, `properties.json`, and `memory.json` for the memory profile. A
completed recording's typed summary is one source; the retained job/result is
another when recording is disabled, unavailable or deleted. The recorder's
unavailability must not conceal an acquired result that still exists. Exact
operation references, partial-byte coverage and stop/refusal reasons remain
bound to their own run. The same rule applies to a retained System-2 journal or
download result without a recording. The projectless/manual-target identity
acquisition remains separately consented and hands its result to the one wizard;
centralised export does not remove acquisition.

Browser-state payloads must stay within the server's 64 KiB serialized UTF-8
limit, with visible truncation/omission metadata. Unchecking `clientState` omits
`client` from all requests; it must remain possible to select the section again.

New server recordings add `related.projectContext`: a serialized
`ProjectContext` captured at operation start (or the acquisition's validated
binding). If the caller already holds the project lock and supplies no binding,
provenance is left unavailable rather than guessed. Older recordings are never
backfilled. A device identified only by individual address is resolved to a
project device only with the same proven context and one unambiguous match.

### Retained-result availability

The bundle shares the existing bounded diagnostic and support result slots; it
creates no snapshot cache and starts no read. `diagnosticsToken` and
`supportToken` name exactly that retained acquisition. A replaced/missing,
expired, unfinished or non-exportable job is visibly unavailable, not the latest
unrelated job. A known version-1 diagnostic recording's strict typed end summary
can supply its historical structured report after the job slot changes. Unknown
recording versions and malformed summaries are not interpreted. When such a
recording is reached through its `diagnosticsToken`/`supportToken` lookup, the
operation section carries its raw `recording.jsonl` exactly as written (never
backfilled or rewritten), because the owner decided on 31 October 2026 that
redacted raw recordings may leave after consent (see below). Explicit
`recordingId`, implicit selection and earlier operations still pass the summary
admission first. The historical consumer reads at most its fixed limit plus one
overflow byte, then validates header and end from that single owned snapshot against
the listed id, size, format, writer, operation kind, token, target and source context.

Without a recording, `downloadId` can project only this server's exact retained
session; `operationId` can project its retained System-2 journal. A retained
result without proven project attribution still exports its own report/status,
but its device and catalogue extracts are unavailable, whether the explicit
device id equals the recorded one or not. With proven attribution equal to the
unchanged current context, the device and catalogue extracts project exactly the
result's own device; with no operation source selected at all, they project only
the device the user explicitly chose. Device ids are comparable only when
the result's attribution equals the proven, unchanged current project context;
only explicit conflicting targets in that context are HTTP 409
`bundleTargetMismatch`. Recorder unavailability is separate from result
unavailability. Diagnostic `report.json`, `properties.json`, optional `memory.json`
and their existing README live under `reports/`; manual support preserves its
existing `report.json` and README there too.

The consumer boundary is profile-specific, not an archive-wide trust decision.
The generic identity producer can be admitted only after its existing closed
service/length grammar, archive writer and independent reader agree byte for
byte. Format-1 admission additionally requires the producer's fixed public
memory-reason sentence and refuses captured bytes from partial or
ambiguous-refusal replies before archive construction; those writer-valid bytes
are not credential-classified. Universal-support evidence additionally needs a
closed whitelist for every
field, origin, status and value shape before its fixed report and README are
materialised. Declared-memory bytes and raw
recordings are **not** credential-classified. By owner decision of 31 October 2026
they may nevertheless leave, but only through the bundle's explicit consent and
with the visible note "contains device configuration, possibly passwords;
recognised KNX keys are redacted; keep private". Recognised keys (authorize and
key-write payloads) are redacted by the recorder at capture time; no other value
is claimed safe. Declared memory keeps the producer's own `memory.json` and README
bytes. A non-System-2 recording larger than 64 MiB stays unavailable rather than
truncated; repeated earlier-recording ids are read once, so retention (512 MiB)
bounds the total. System-2 recordings, including one whose journal id is missing,
keep their admitted private projection or stay unavailable and never export the
raw recording. A chosen device without any operation source is projected from the
current project, as before.

### Sections

Defaults per problem type follow owner table O1 in the work package and are
implemented once in `knx_app::diagnostic_bundle::default_offer`.

| Section | ZIP path | Source |
| --- | --- | --- |
| environment (fixed) | `environment/environment.json` | writer identity, platform, server incarnation, effective timings, product database and project state |
| session log | `session-log/session-log.json` | withheld: not yet admitted as credential-free, always unavailable with the "withheld" reason |
| operation / earlier operations | `operations/<id>/recording.jsonl`, or `system2-private.json` / a retained typed result file | non-System-2: the recorder-redacted raw recording after consent; System-2: the admitted private projection only |
| minimised legacy reports | `reports/...` | the existing download and System-2 reports of that operation |
| device extract | `device/<id>/device.json`, `parameters.json` | device detail and parameter panel projection |
| import report | `import/import-report.json` | the project's stored import evidence |
| original import file | `import/...` | not kept by the server; the section says so |
| catalogue | `catalogue/catalogue.json` | product database availability (enum only, never the startup reason) and the device's product identity |
| whole project | `project/project.knxdb` | last saved native project, secret-free copy; unsaved changes are not included |
| bus monitor | `bus-monitor/bus-monitor.json` | withheld: not yet admitted as credential-free, always unavailable with the "withheld" reason |
| client state | `client/client.json` | withheld: not yet admitted as credential-free, always unavailable with the "withheld" reason; the browser's facts are still bounded and validated |

### Unavailability reasons

Every offer that cannot be delivered carries one fixed English reason; the web
translates each by exact text and a server test fails when a reason or fixed
note has no web translation. Reasons that do not depend on what a source holds
are specific: a section withheld until admitted ("this section is withheld until
its content is admitted as free of credentials"), no fitting or no chosen
operation, a recording without a retained typed report, no open project, no
stored import report, an unsaved project, no chosen device, an operation whose
proven context identifies no single project device (none, or an address shared
by several devices), the original import file the server does not keep, and the
retained-result causes. The generic "the selected diagnostic source is
unavailable" remains where a named source is gone or rejected, and wherever a
reason could hint at content (a credential observed in a project or import
report, an unreadable stored report, an unsafe or unadmitted recording, a failed
secret-free copy).

The import-report section uses the shared secret-free report projection on the
admitted historical evidence. Identified secret samples and their digests are
removed from the exported copy, and redaction/withholding notes are visible in
the section metadata. Successful projections preserve interpreted row identity,
count units and historical totals. If removing a learned value would change an
interpreted identity, the report is visibly unavailable instead of being exported
raw or with altered meaning. The stored historical evidence and input file are
not rewritten by preview or export.

`manifest.json` lists format (`knxbench-diagnostic-bundle`, version 1),
`createdAt`, `consentAt`, writer, problem type, description, recording state,
redaction policy (what is always removed and what is not), and per section its
files with size and SHA-256. The ZIP writer refuses duplicate or unsafe paths,
unoffered sections and a bundle without the environment section.

### Hand-off

The user requests the ZIP download and verifies that the file was actually
saved before opening a `mailto:issue@knxbench.com` draft whose subject names the
version and operation id. Clicking a blob link alone proves only initiation,
not a completed save: the desktop save dialog can still be cancelled or fail.
Where no typed completion result exists, the UI must say "download requested"
and require an explicit user acknowledgement of the saved file before enabling
mail or claiming save-dependent success. A retry, refusal or context/content
change withdraws that acknowledgement.

The draft never carries bundle content and cannot attach the file; email is not
encryption. Bundles above
10 MiB get a hint about common mail limits; there is no splitting. Public
GitHub issues keep using the minimised reports only.

## CLI

`--diagnostic-bundle <new.zip>` is accepted anywhere on a `knx device ...` or
`knx bus ...` command line and refused elsewhere. The file must not exist; that
is checked before any device contact. Recordings of the invocation are written
to a hidden directory beside the bundle and removed after the ZIP is written
(also when writing fails). Without the flag the CLI records nothing.

The flag is the user's consent for that one bundle. Following the 31 October
2026 owner decision, each non-System-2 recording of the invocation goes into the
ZIP exactly as recorded (keys redacted at capture), as
`operations/<id>/recording.jsonl` with the keep-private note; only the newest
listed recording can be the operation, the others are earlier operations. No
recording of a `system2-*` subcommand leaves raw, whatever its kind; the
System-2 command contributes only its admitted private projection and report.
Every recording left out (unreadable header, System-2 kind, unreadable or larger
than 64 MiB, or any recording of a `system2-*` command) is named in a note in the
environment section. The help text, the success message, the web mail draft,
the recordings note, the wizard's "not removed" text and the manifest's
`notRemoved` say that the ZIP may contain device configuration and passwords and
must be kept private. A new recording header's redaction policy states only what
capture does (authorize and key-write keys zeroed, see [Redaction](#redaction))
and that no tunnel password or KNX Secure key is exchanged; older headers keep
their original text. Checked on 31 October 2026 against an installed
release build on loopback: without the flag nothing is written and no hidden
directory appears; a doubled flag (also `--diagnostic-bundle=` mixed with the
spaced form, same or different file), a flag without a file name and a flag
outside `device`/`bus` are refused before any datagram; a repeated invocation
with the same bundle path is refused before any datagram and leaves the first
ZIP unchanged.
