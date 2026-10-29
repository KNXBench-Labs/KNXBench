# Operation recordings and the diagnostic bundle (DIAG-03)

Owner: DIAG-03 ([work package](../open-work/DIAG-03-unified-diagnostic-bundle.md),
[ADR-0156](../adr/0156-operation-recorder-and-diagnostic-bundle.md)).

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

The free-running bus monitor is not recorded. Its last ten minutes are an
optional bundle section.

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
recording versions and malformed summaries are not interpreted; the raw recording
remains exportable unchanged.

Without a recording, `downloadId` can project only this server's exact retained
session; `operationId` can project its retained System-2 journal. A retained
result without proven project attribution still exports its own report/status,
but its device and catalogue extracts are unavailable, whether the explicit
device id equals the recorded one or not. Device ids are comparable only when
the result's attribution equals the proven, unchanged current project context;
only explicit conflicting targets in that context are HTTP 409
`bundleTargetMismatch`. Recorder unavailability is separate from result
unavailability. Diagnostic `report.json`, `properties.json`, optional `memory.json`
and their existing README live under `reports/`; manual support preserves its
existing `report.json` and README there too.

### Sections

Defaults per problem type follow owner table O1 in the work package and are
implemented once in `knx_app::diagnostic_bundle::default_offer`.

| Section | ZIP path | Source |
| --- | --- | --- |
| environment (fixed) | `environment/environment.json` | writer identity, platform, server incarnation, effective timings, product database and project state |
| session log | `session-log/session-log.json` | the server's session log |
| operation / earlier operations | `operations/<id>/recording.jsonl` (+ `system2-progress.json`, `download-status.json`) | recording; System-2 journal projection; this run's download status |
| minimised legacy reports | `reports/...` | the existing download and System-2 reports of that operation |
| device extract | `device/<id>/device.json`, `parameters.json` | device detail and parameter panel projection |
| import report | `import/import-report.json` | the project's stored import evidence |
| original import file | `import/...` | not kept by the server; the section says so |
| catalogue | `catalogue/catalogue.json` | product database state and the device's product identity |
| whole project | `project/project.knxdb` | last saved native project, secret-free copy; unsaved changes are not included |
| bus monitor | `bus-monitor/bus-monitor.json` | the running monitor's buffer, last ten minutes |
| client state | `client/client.json` | what the browser sent (language, theme, viewport, zoom, shell, recent errors) |

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
