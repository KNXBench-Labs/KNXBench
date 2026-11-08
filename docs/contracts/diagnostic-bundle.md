# Operation recordings and the diagnostic bundle (DIAG-03)

Owner: DIAG-03 ([work package](../open-work/DIAG-03-unified-diagnostic-bundle.md),
[ADR-0156](../adr/0156-operation-recorder-and-diagnostic-bundle.md)).

<a id="candidate-implementation-and-remaining-privacy-admission"></a>

## Current source and historical validation

The recorder, shared bundle builder, retained-result admission, Web wizard and
CLI export are published on `main` through `bc77abdb2` (31 October 2026), newer
than the immutable Alpha 18 release. The real simulator/server/production-Web/ZIP
join, installed-CLI flag boundaries and original fourteen-case matrix are recorded
in the [work package](../open-work/DIAG-03-unified-diagnostic-bundle.md).
System-2 exports only its closed projection, never raw comparison or recording
bytes. P1/P2 are prerequisites only for a separately commissioned future raw export.
The supplementary/quit/source-review closeout is delivered through `7443e2ea6`;
[final software scope](../status/2026-11-01-diag03-closeout.md) distinguishes source delivery from release and hardware acceptance. An automatic selector that finds no source and explicit `none` bind the same absent identity and unavailability meaning, while named/admitted sources and all changed contents remain discriminated.

The three supplementary sections use typed, reduced projections rather than
arbitrary raw text: session-log chronology with known messages, a reviewed
last-ten-minutes monitor window, and bounded browser facts. Each section reports
its withheld/truncated field counts. Their exact scope is in [Sections](#sections).
They are not comprehensive secret detection or an export of every free error
string.

### Historical foundation checks (29–30 October 2026)

The following dated paragraphs preserve earlier failures and bounded evidence;
their then-open integration/matrix/publication steps are not current status.
Current source status belongs to the linked work package.

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

The free-running bus monitor is not an operation recording. Its optional
last-ten-minutes section reads the existing buffer without starting a session or
contacting the bus; see the [reviewed monitor window](#reviewed-monitor-window).

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
| `datagram` | `dir` (`sent`/`received`), `octets`, `raw` (lower-case hex of the exact UDP payload or retained whole TCP frame, never a stream chunk), decoded `service`, `channel`, `tunnel_seq`, `status`, and for cEMI frames `message`, `confirm_error`, `source`, `destination`, `tpci`, `apci`, `summary`; `redacted` when key octets were removed |
| `tunnel` | connection events: connect refused, ACK timeout and repeat, heartbeat attempts, discarded frames (foreign channel, duplicate), close reason, socket errors; `frame-skipped` retains only service, length and reason, never skipped frame content |
| `note` | operation-level facts: plan, step results, full error chains, results projected from typed reports |
| `gap` | `lost_records` and `reason` when records could not be written |
| `end` | last line: `outcome` (`succeeded`, `failed`, `cancelled`, or `abandoned` when the operation ended without reporting one), `error_chain`, `summary`, `records_written`, `records_dropped`, `failed_writes` |

Older recordings are kept and exported unchanged after an update; readers use
`format_version` and never backfill.

### Redaction

Always applied, also with the user's consent, before a record reaches the
writer: the 4 key octets of `A_Authorize_Request` and `A_Key_Write` are zeroed
in `raw` and named by `"redacted": {"kind": "authorize-key" | "key-write-key",
"offset": n, "octets": 4}`. KNXBench implements only unsecured KNXnet/IP,
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

<a id="reviewed-content-binding-required-before-acceptance"></a>

### Reviewed-content binding

Preview, section text and export enforce the following binding; consent is not
permission to recollect different contents silently:

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
  The fingerprint is an unkeyed content/integrity binding, not authentication,
  authorization or proof that a person read the preview. Existing authenticated
  session checks remain in force.
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
| environment (fixed) | `environment/environment.json` | writer identity, platform, server incarnation, effective timings, product database and project state; `journalWrites`: the last 20 journal writes of project commands (edits, undo and redo, programming settlement, catalogue creation; not Save, versions, notes, instructions or other direct journal writers) of this server process (time, total and per-phase milliseconds, success, undo/redo/version counts, kept/copied/encoded rows, retained-context bytes, reuse flags; no names, paths or values; ADR-0165) |
| session log | `session-log/session-log.json` | ordered typed chronology, known sources/messages, withheld-field counts; no arbitrary location/detail text |
| operation / earlier operations | `operations/<id>/recording.jsonl`, or `system2-private.json` / a retained typed result file | non-System-2: the recorder-redacted raw recording after consent; System-2: the admitted private projection only |
| minimised legacy reports | `reports/...` | the existing download and System-2 reports of that operation |
| device extract | `device/<id>/device.json`, `parameters.json` | device detail and parameter panel projection |
| import report | `import/import-report.json` | the project's stored import evidence |
| original import file | `import/...` | not kept by the server; the section says so |
| catalogue | `catalogue/catalogue.json` | product database availability (enum only, never the startup reason) and the device's product identity |
| whole project | `project/project.knxdb` | last saved native project, secret-free copy; unsaved changes are not included |
| bus monitor | `bus-monitor/bus-monitor.json` | pinned last-ten-minutes window from the existing session; typed group values only, reductions counted |
| client state | `client/client.json` | closed-schema browser facts and reduced recent-error metadata, at most 64 KiB on input; unknown fields counted, not echoed |

### Reduced log and browser facts

The log projection preserves every entry's order, canonical timestamp and severity.
Known server/command sources and fixed messages with bounded integer placeholders
survive, including undo/redo and import counts. Path-bearing success messages keep
the action and withhold the path. Arbitrary messages, locations, details and
quarantine paths are omitted per field and counted in `withheld`; an omitted
message is `null`, not an empty string or a claim that the operation had no error.
The live log is not rewritten. This is a useful chronology, not a full free-text
error-chain export.

Client facts admit known version/shell/language/theme forms, bounded viewport and
zoom, truncation counters and at most 50 reduced recent errors. Unknown names are
counted without echoing them; imported language/theme identities become `other`.
Malformed known values are counted as invalid. Recent-error details and arbitrary
message text follow the same withholding rules as the log. Deselecting client
state excludes its body from text and ZIP. These projections make no claim that
arbitrary text can be certified credential-free.

### Reviewed monitor window

A selected, available bus-monitor section adds optional `review.monitor`:
`{sessionId, firstSeq, endSeq, droppedBefore, statusAtReview, capturedAt}`, plus
`skippedFramesAtReview` for a captured TCP session. The optional skip count is a
nonnegative safe integer and is bound to the same review, not refreshed by later
text/ZIP requests. Its absence is retained for historical, UDP and serial bindings.
Its fields are closed; other counters are nonnegative safe integers, `firstSeq <= endSeq`,
the interval has at most 5000 rows, status is `active` or `closed`, and capture
time is canonical UTC RFC 3339 with milliseconds. A malformed binding is HTTP 400
`bundlePreviewRequired`. No monitor selection requires no binding.

A fresh preview picks the existing session's rows at most 600 seconds old,
including the boundary, and binds the exact `[firstSeq, endSeq)` interval and
session identity. The window can be empty. Text and ZIP requests carry that
review unchanged and project only those rows, even after new traffic arrives or
the rows age past ten minutes. Counters, status and capture time in the body are
those at review, not new live observations. Gateway closure can retain the same
window; explicit stop removes the session and hence the source. A missing or
replaced session, eviction, a missing/duplicate/out-of-order pinned row or a
changed/dropped/unasked binding refuses with HTTP 409 `bundlePreviewChanged` and
no ZIP. There is no server snapshot cache or invented retention guarantee; other
selected sections still use strict current-content review.

The monitor file carries format/version, session identity, `statusAtReview`,
`capturedAt`, window seconds, interval, `droppedBeforeAtReview`, projected rows and
withheld counts. A known IP session includes `transport` (`udp` or `tcp`); a TCP
session additionally includes its review-pinned `skippedFramesAtReview` count.
Unknown and serial sources never invent IP provenance or a TCP counter.
Broadcast/retention losses and later skips after preview are not observations
of that pinned body. No monitor is started or restarted by collection.

Rows retain safe typed transport metadata and capped project group-address
labels (128 characters, truncation counted). Only a successful typed group-value
decode may export payload octets. Character/text DPTs 4/16/24/28 and access-data
DPT 15 export neither their decoded value nor raw payload. Undecoded, unresolved,
conflicting or failed decodes never export raw bytes, regardless of length.
Management services, including memory/key services, are metadata only; they
cannot provide a second copy of raw System-2 comparison data. Without a project
or a resolved DPT, the monitor exports transport metadata only, no values or
octets. Project labels and other admitted typed group values remain installation
content, not comprehensive secret detection. Scope notes and counted omissions
make the reduced views explicit; unknown browser identities are labeled `other`.

### Unavailability reasons

Every offer that cannot be delivered carries one fixed English reason; the web
translates each by exact text and a server test fails when a reason or fixed
note has no web translation. Reasons that do not depend on what a source holds
are specific: no monitor session in this server, no browser facts supplied, no fitting or no chosen
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
