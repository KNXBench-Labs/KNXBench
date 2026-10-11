# Login, notification details and file-admission contract

Status: locally implemented source package, 2026-10-11. Publication, release and
running-container deployment are separate decisions. No project/native schema,
KNX protocol, importer identity rule or dependency is changed.

## Login identity

The login page reuses `src/assets/knxbench-mark.png`, imported through Vite,
instead of the letter badge. The adjacent KNXBench name is its accessible label;
the image is decorative, has reserved dimensions and preserves its aspect ratio.
The page shows `v{version}` from the frontend package manifest, without requiring
an authenticated API request. This identifies the built frontend, not a queried
remote server or the newest available release; a cached/older frontend can differ.
Initial login, expiry and explicit sign-out share this rendering.

## Exact notification targets and copyable details

- The existing toast queue records each emitted notification under a monotonic,
  browser-local ID. `ui:toast:*` entries are explicitly UI records, not matched or
  promoted to backend log identities. Equal text does not imply equal identity.
- Click the notification action, or activate it by keyboard, to open Log, clear
  hiding filters/search, scroll to the matching retained UI record and show its
  details. The separate dismiss button still only dismisses the toast.
  Toast and log-row actions suppress the global hover scaling/shadow so their
  interactive geometry remains inside the card or scrollable list.
- Browser history retains the latest 1000 notifications independently of toast
  expiry/dismissal. It is memory-only, scoped to this mounted workbench, not a
  complete or durable audit. Older loss is counted and disclosed. An evicted
  target produces an explicit notice; no same-text replacement is selected.
- Clicking a server or UI log message opens a read-only snapshot with timestamp,
  severity, source, message, nullable location/detail and the full JSON record.
  Raw text is not translated, trimmed, interpreted as HTML or humor-wrapped.
- **Copy message** copies the exact message; **Copy all details** copies the full
  JSON record, preserving line breaks, Unicode, nulls and diagnostic metadata.
  The read-only textarea supports normal selection/Ctrl+C. Clipboard refusal
  reports failure, focuses/selects the raw record and permits manual copying.
- The existing Overlay owns focus trapping, Escape and background isolation.
  Initial focus is the raw text. Closing restores focus to the durable log-row
  action, not a toast that may already be leaving.
- **Session-log JSON export remains backend-only v1**. Filtering selects only
  matching server rows; UI notifications are copied from their details instead.
  The visible history notice states this exclusion. This preserves server loss
  provenance, the native validator's 1000-record bound and the existing format;
  mixed browser/server history must not claim to be a server-run audit.

## File sizes: admission is not parser support

All MiB values below are binary units. Limits are inclusive; one byte above is
refused. Multipart overhead is separate from the actual file-size allowance.

| Boundary | Before | This package | Reason |
| --- | --- | --- | --- |
| Browser Analyze/Evidence selected source | 32 MiB | 1024 MiB | Same raw admission as application service |
| Contribution service and CLI bounded source read | 32 MiB | 1024 MiB | Allows larger input for analysis, preview and consented export |
| Contribution declared expanded inventory total | 64 MiB | 1024 MiB | Inventory can report larger archives without claiming every member was scanned |
| Streamed `/api/fs/upload` file | 256 MiB | 1024 MiB | Streams into staging; publishes only a complete file |
| Project importer expanded archive total | 1024 MiB | Unchanged | Already meets requested total |

HTTP upload and contribution forms permit a separate 16 KiB multipart envelope.
Advertised oversized lengths are refused before polling the body; missing or
understated lengths still meet bounded body/field reads. Oversize upload refusal
leaves no partial publication and never overwrites an existing uploaded file.
The CLI checks metadata before reading and also uses a bounded read.

The following are **not blanket-raised to 1024 MiB**:

| Existing bound | Retained policy and reason |
| --- | --- |
| Contribution XML scan: 8 MiB/member; 4096 members and bounded events/depth/output | Parser CPU/output and disclosure safety; unscanned data yields `partial`/`not-examined`, not complete support or permission to export an original |
| Project archive: 64 MiB/expanded member | A whole-archive admission budget must not replace individual decompression protections |
| Manufacturer package raw input: 256 MiB, plus existing extraction/metadata profiles | Dedicated importer retains resident buffers; no measured full-GiB installation acceptance exists |
| Selective source integration: 64 MiB | Retains/clones source and transaction/history snapshots; separate application operation, not upload or contribution analysis |
| Public evidence ZIP: 24 MB (decimal), at most 16 selected context members | Deliberately reduced shareable artifact, not the analyzed source; disclosure/consent unchanged |
| Native JSON exports: 16 MiB; existing small configuration/language/theme limits | Typed-format/output protections, not KNX project upload limits |

An accepted source can still exceed a typed-import or XML scan budget. Report
coverage honestly and preserve the existing original/context permission checks.
Reverse proxies can impose lower HTTP limits. In-memory contribution processing
can use multiple source-sized buffers: 1024 MiB admission is **not** a 1024 MiB
RSS guarantee or proof that a particular large ETS/vendor file is compatible.

## Verification boundaries

Synthetic regression coverage includes a real stored ZIP above 32 MiB through
application, HTTP analysis/preview/consented reduced export and the real CLI;
an expanded inventory above the previous 64 MiB total; a streamed file exactly
1024 MiB with multipart overhead; oversized declared/chunked upload refusal;
and a sparse CLI source one byte above the limit without output creation.
Existing malformed-input, privacy, consent and archive guards remain exercised.
Unit regressions cover initial/expired/signed-out login and duplicate/evicted
notification targets. Browser cases use intercepted synthetic APIs, loopback-only
namespaces and no bus traffic. They cover initial login, EN/DE, 1440/400 pixels,
Graphite/Porcelain, distinct same-message backend occurrences, real clipboard text,
manual Ctrl+C after an explicitly simulated Clipboard API refusal, backend-only
v1 downloads, Escape/focus return and viewport fit. No native UI, real bus, vendor archive
compatibility, private-corpus or running-container acceptance is claimed.
