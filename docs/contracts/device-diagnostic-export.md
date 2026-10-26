# Read-only device diagnostic export

[ADR-0134](../adr/0134-generic-read-only-diagnostic-export.md). A source-build
feature after Alpha 13, released in [Alpha 14](../releases/alpha14.md).
Simulator-verified, plus **46 physical devices from eight manufacturers so far**
(see [Real devices](#required-before-any-hardware-claim)).

## What it is and is not

One selected project device, one explicitly entered IPv4 KNXnet/IP tunnelling
endpoint, one bounded read sequence, one local ZIP. It is evidence for an
investigation, **not** a programming unlock, a product/application match, a
recovery backup or a hardware support claim. It is independent of download
admission: an unsupported device can still be read safely.

Relation to the [universal support report](universal-device-support.md): that
report is a privacy envelope without raw replies and keeps its own reviewed
family recipes. This export carries raw identity replies, needs no product data
and applies one generic recipe to every device.

## Recipe `generic-identity-v1`

| # | Request | Expected |
| --- | --- | --- |
| 1 | `T_Connect` | positive `L_Data.con` |
| 2 | `A_DeviceDescriptor_Read` type 0 | 2 octets |
| 3 | `A_PropertyValue_Read` object 0, PID 12, element count 1, start 1 | 2 octets |
| 4 | `A_PropertyValue_Read` object 0, PID 78, element count 1, start 1 | 6 octets |
| 5 | `T_Disconnect` (always attempted after a sent `T_Connect`) | — |

Session: `ManagementSession::read_only` + `AuthorisationPlan::Skip`. No key,
`A_Authorize`, Verify Mode, memory/property write, restart, reset, group
telegram, scan, discovery or retry beyond the existing Transport Layer ladder.
A response is accepted only for the requested object, PID **and start index**.
The start-index check is confined to this export's raw read; the established
`read_property` callers (download, compare, support report) keep their
existing matching unchanged.

- Project/catalogue mask and manufacturer are **assertions**. A conflicting
  observed value stops further reads (`identity-mismatch`); a descriptor that is
  not exactly read (`descriptor-unverified`) stops them as well.
- **Memory: `not-requested`** in format 1. Generic product source offsets
  are not runtime addresses; the only memory profile is the separate,
  ProductDB-declared one [below](#profile-system2-declared-segments-v1-format-2).
- Bounds: 3 logical reads, 30 s acquisition deadline (checked before every
  read), 10 s gateway connect, 3 s cleanups, 512 retained octets per reply.

### Identity stop-reason contract

As of **25 October 2026**, the archive reader enforces the same identity-item
state/reason mapping for both profiles. The identity-only profile had the same
gap as format 2; both are fixed.

| Collector identity state | Reachable reason |
| --- | --- |
| Format 2 lacks a recorded mask or manufacturer assertion; no item attempted | `identity-assertion-missing` (format 2 only, before any frame) |
| No item attempted with the required assertions present | `cancelled`, `deadline`, `timeout`, `connect-rejected`, `self-addressed`, `connection-lost`, `event-loss` or `transport-error`; format 1 may instead carry `gateway-busy`, `context-changed-before-connect`, `server-stopping`, `gateway-connect-failed` or `gateway-timeout` from its caller |
| An attempted item is `error` | Exactly that item's `cancelled`, `deadline`, `timeout`, `connection-released`, `connection-lost`, `event-loss`, `transport-error` or `response-size-limit-raw-not-retained` token |
| Descriptor has a non-error, non-exact response (`partial`) | `descriptor-unverified` |
| A read descriptor or PID 12 contradicts its recorded assertion | `identity-mismatch` |
| A non-stopping completed prefix is followed by an untouched item | `cancelled` |
| All three items complete without an earlier stop, but at least one is not `read` | `identity-unverified` in format 2; no identity stop in format 1 |
| All three are exactly read; format-2 PID 78 does not `resource_matches` its comparison | `identity-mismatch` in format 2; format 1 makes no PID 78 assertion |
| Identity is admitted | No identity stop; format 2 proceeds to memory and format 1 completes |

For the lower-layer format-2 collector, an empty PID 78 comparison also takes
the pre-frame `identity-assertion-missing` branch; ProductDB derivation refuses
such a plan, so the archive profile has no corresponding valid plan. A caller's
`tunnel-disconnect-unconfirmed` can replace only an otherwise absent final stop
reason. Rewritten archives whose reason contradicts these states are refused
even when every member length and SHA-256 value has been recomputed.
The reader also checks the collector's per-query response shapes: a descriptor
has no returned-element count, a property has one (zero means an ambiguous
refusal), and status-specific errors and retained-byte counts are fixed.

## Profile `system2-declared-segments-v1` (format 2)

[ADR-0143](../adr/0143-declared-segment-memory-evidence-profile.md); offline
and simulator only, no hardware run yet. For stage 0 of the
[UP 258D12 goal](../goals/up258-programming-2026-10-18.md): read the memory
ranges a System-2 application's own load procedure declares, to compare the
ETS-programmed state offline with KNXBench's source interpretation.

- **Ranges from product data only.** `knx_productdb::declared_segments`
  requires the shape `system2::project_tool_metadata` accepts, then takes every
  `LdCtrlAbsSegment` (address, size) in source order and the one Device Object
  PID 78 `LdCtrlCompareProp`. It refuses whole on another shape, zero or
  repeated PID 78 comparisons, empty inline data or more than 64 octets of it
  (`MAX_COMPARISON_OCTETS`, the archive's bound, so every admitted plan is
  exportable), a zero size, a range
  reaching `FFFFh`, overlap, more than 8 ranges or more than 2048 octets. For
  `M-0001_A-9205-04-9270`: `0116h`/330, `0260h`/416, `0400h`/512, `3C00h`/256,
  1514 octets in 128 reads (public-package test).
- **Identity first.** Connect, descriptor, PID 12, PID 78 exactly as above.
  Mask and manufacturer assertions are required (else
  `identity-assertion-missing` before any frame). Memory is read only if all
  three items are `read`, descriptor and PID 12 equal the assertions and
  PID 78 `resource_matches` the comparison; otherwise `identity-mismatch` or
  `identity-unverified` and no `A_Memory_Read` is sent.
- **Memory.** `A_Memory_Read` of 12 octets (standard frame), one item per
  chunk with address, requested length, status, bytes, times and error token.
  A zero-octet answer is `refused-ambiguous` and ends its range (rest
  `not-read`); another length is `partial`, bytes kept, and ends its range;
  timeout, connection loss, cancellation or the 180 s deadline end the run
  with earlier chunks kept. No retry beyond the transport ladder.
  `T_Disconnect` is always attempted. Simulator tests assert the exhaustive
  outgoing service set (identity reads plus in-range `A_Memory_Read` only).
- **Archive format 2:** the four members above plus `memory.json` (profile,
  program id, PID 78 comparison with procedure position, each range with its
  procedure position and coverage in octets read / refused / partial / error /
  not-read, every chunk). Manifest `notice` and README say plainly:
  source-declared candidate locations, not proven runtime addresses; not a
  backup and not a restore basis; device configuration, keep private.
  `read_export` accepts formats 1 and 2 strictly and recomputes the chunk
  plan and coverage. Once any chunk shows an outcome, a time or bytes, it
  re-checks the identity admission from the archive: all three items `read`
  at their exact lengths (one element for PID 12 and PID 78, none for the
  descriptor), descriptor = asserted mask, PID 12 = asserted manufacturer,
  PID 78 `resource_matches` the recorded `inlineData`. Every chunk with an
  outcome carries attempt and completion times in that order; an unread one
  carries neither, except in an observer snapshot (`complete: false`): there
  one chunk may be in flight (attempted, not completed, no bytes, no error),
  with nothing after it attempted and never inside a range or run already
  ended. Identity items (both formats) must follow `identity_reads` exactly:
  an item with an outcome was attempted, then completed; the next item is
  touched only after an outcome that lets the collector go on (no error, no
  read contradicting its assertion, and after the descriptor only an exact
  read; a refused or partial PID 12 still continues to PID 78); after an
  untouched item, the one in-flight item (`complete: false` only) or a
  stopping item, every later item is wholly untouched. So every observer
  snapshot is exportable; complete reports keep the strict rule. The chunk order must be one the collector can produce: a
  refusal or partial answer (at least one octet; an empty answer is a
  refusal) ends its range, an error ends the run and is the stop reason, an
  unread chunk outside an ended range means `cancelled`, and a complete run
  with nothing ended has no stop reason except the caller's
  `tunnel-disconnect-unconfirmed`. Chunk error tokens also follow
  `memory_reads`: `partial` requires `unexpected-length`, `refused-ambiguous`
  requires `access-denied-or-memory-unavailable`, and `error` admits only
  `cancelled`, `deadline`, `timeout`, `connection-released`, `connection-lost`,
  `event-loss` or `transport-error`. These are the bounded wait's outcomes and
  the established session's memory-exchange failures, not every token in the
  collector's shared `failure` mapping: setup/identity reasons and the caller's
  tunnel-teardown reason cannot be chunk errors, even in observer snapshots.
  **25 October 2026:** the reader also
  cross-checks every identity-phase stop reason against the identity items and
  assertions, for both formats. `read_bundle` refuses format 2 rather than
  dropping its memory. Unread octets are never zero-filled. Same create-only
  `0600` writer with reread. The acquisition plan is opaque: only
  `prepare_memory` (ProductDB derivation) builds it, `acquire_memory` takes
  nothing else, and `compile_fail` doc-tests show outside code cannot forge
  or edit its ranges; `collect_declared_memory` stays a lower-layer primitive.
- **Entry points: CLI and web.** CLI: `--profile system2-declared-segments-v1`
  needs `--product-db`; the device's application comes from the project's
  hardware-to-program mapping. The preview lists profile, application, the
  ranges and total, required identity values and that nothing will be
  written, and builds no network client. `--simulate` takes the project and
  product database too (the simulator shows the asserted identity; its memory
  is the simulator's). **Web (26 October 2026 source, DIAG-01):** the same
  dialog offers a *Diagnostic profile* choice before Preview (default generic
  identity). The memory profile needs the server's product database and
  refuses without one (`diagnostics-product-db-required`), exactly like the
  CLI without `--product-db`; every other admission, device-selection and
  identity rule is the same `prepare_memory` call. Changing the profile drops
  the preview and its consent like a device or gateway change. The preview
  lists application, required identity, each declared range (address,
  length, procedure position) and the total; progress counts identity reads
  and memory chunks; the result shows coverage per range. The same read and
  share consents apply. A memory run that stops before its tunnel for a
  setup reason (gateway busy, connect failure or timeout, context change,
  server stopping) has no admitted format-2 archive (the caller setup reasons
  are format 1 only), so the web shows the result without a save action, as
  the CLI writes no archive then. A Stop that lands before the tunnel opens is
  different: the run ends with `cancelled`, a collector reason format 2
  admits with every identity item and chunk untouched, so that result is
  exportable (route test
  `a_memory_run_cancelled_before_connecting_is_exportable_without_any_request`).
  Simulator-only evidence; no hardware run.

## Result per item

`read` (exact expected length) · `partial` (other length or element count;
bytes kept) · `refused-ambiguous` (zero elements: access denied **or**
unsupported, AL §3.4.4.2 cannot tell) · `error` (timeout, release, deadline,
cancel; bytes of earlier items kept) · `not-read` (stopped before the request).
Each item keeps request parameters, raw bytes, received length, returned element
count, attempt/completion time and an error token. Not-requested is never
"successfully empty", refusal is never zeros.

## Archive, format 1

Fixed members `manifest.json` (kind, format, build, time, `simulated`, SHA-256
and length of the other three), `report.json`, `properties.json`, `README.txt`.
No project, device name, individual address, gateway, serial number, key,
password, path or free-text error. Project association is an opaque random
reference shown locally only. `simulated: true` marks any simulator capture.
The writer is create-only (never overwrites), owner-only `0600`, flushes and
syncs, then rereads the held file and verifies it before reporting success. The
reader refuses missing, extra, duplicate or oversized members, unknown formats,
CRC/length/hash mismatches and inconsistent copies. Duplicates are found in the
raw central directory before the `zip` crate, which keeps only the last entry
of a name, sees the archive; ZIP64 is refused (formats 1 and 2). The fixed
format has no ZIP comment: the end record must be exactly the last 22 octets,
so no competing end record can hide in a comment or trailing bytes. The `zip`
crate must then report the same directory start and, entry by entry, the same
name, central header offset and local header offset as the raw walk. Checksums detect corruption,
not deliberate rewriting. Raw replies can still identify equipment: no anonymity
is claimed.

## Entry points (one application service)

- Web: **File → Export diagnostic data** (also beside *Device support report*),
  and since 24 October 2026 (source builds after Alpha 16) **Analyse device** on
  a device's *Diagnostics* tab, which opens the same dialog with that device
  preselected (no new route, consent or gateway handling).
  Preview (no bus traffic) → explicit read consent → progress, *Stop* (keeps
  partial data, still disconnects) → content preview → explicit save consent →
  local download. Nothing uploads; the user sends the ZIP to
  `issue@knxbench.com` or shares a private link.
  The device-tab entry closes analysis on a device/project change, project close
  or entry unmount. Each successful running Start response is owned immediately,
  independently of whether React has committed its progress view; dismissal
  cooperatively cancels that exact token once. A Start arriving after dismissal
  is cancelled by its own token too, never a replacement dialog's job. Cleanup
  rejection is best effort and cannot update the replacement dialog. A revision-
  only refresh keeps the analysis, its stale review and any running acquisition;
  Stop still retains partial results for separately consented export.
- HTTP: `POST /api/device-diagnostics/preview {deviceId,gateway,profile,context}`
  (`profile` required: `generic-identity-v1` or `system2-declared-segments-v1`;
  another value is `400 diagnostics-unknown-profile`, a missing one is refused
  by the strict body parser; `prepare_memory` refusals are `400` with their
  leading token, e.g. `diagnostics-identity-assertion-missing`),
  `POST …/start {token,consent:true}` (202), `GET …/{token}`,
  `POST …/{token}/cancel`, `POST …/export {token,consent:true}`. Stale
  project revision, unknown token, false consent, excluded/ambiguous/missing
  target, non-IPv4 endpoint and unknown fields refuse before any tunnel.
  The acquisition runs as a server-owned task, so a closed browser cannot cut
  cleanup short. Every view carries `report`, `memory` (`memory.json` of format
  2, `null` for `generic-identity-v1`) and `exportable` (the finished result
  passed the archive writer's own validation and readback; `false` before
  completion). Running views are observer snapshots: at most one identity item
  or memory chunk in flight. The result shows the stop reason as a translated
  label with the raw token beside it; a preview refusal leads with translated
  text and keeps the server's detail as secondary evidence. Export uses `bundle_export` for both formats, so a
  format-1 ZIP stays byte-for-byte what `bundle` writes. The browser client
  admits both formats strictly (closed field sets, profile/format/memory
  coupling, recomputed chunk plan and coverage, lowercase hex, chunk outcome
  shapes, one in-flight chunk only while running).
- CLI: `knx device diagnostics --project p.knxdb (--device 1.1.5 | --device-id
  N) --gateway host:port --output new.zip [--product-db db] [--confirm-read]`;
  without `--confirm-read` it only previews. Exactly one selector is required.
  `--device <area.line.device>` must belong to exactly one project device: an
  unknown address is refused, and an address several devices share (even
  across installations) is refused listing their internal ids and names for
  identification only. With **either** selector, the individual address must
  uniquely identify one device across the whole project; `--device-id` does
  not bypass this requirement. Resolve duplicate project address assignments
  before retrying. A device without an address is never selected by address.
  These refusals, like a malformed address, happen before any tunnel or archive
  creation. `--simulate` uses the built-in simulator and never constructs a
  network client. Ctrl-C stops cooperatively and saves
  the partial result. Write/key/memory flags are refused. `--profile
  system2-declared-segments-v1` selects the memory profile above.

## Activity history

Every server-side run is a `deviceDiagnostics` row in the durable activity
history: `finished`, `failed` (also for a refused or timed-out gateway connect)
or `unknown` (request future dropped). The kind is part of the history's closed
kind list, so a run never marks history unavailable or blocks later writes and
downloads. Builds older than this feature do not know the kind and refuse to
list a history page that contains such a row. The CLI keeps no activity history.

The HTTP job token (a capability for status, cancel and export) is drawn
separately from the archive's `projectReference`; a shared ZIP carries no
usable token. Error and stop-reason fields are closed lowercase code tokens on
write and on reimport.

## Exclusivity

The server takes its existing tunnel locks in the established order (address
programming `try_lock`, download holder, monitor, scan), then one per-user
nonblocking file lease (`$XDG_RUNTIME_DIR/knxbench/knx-gateway-tunnel.lock`)
that the CLI also takes. Busy refuses with `gateway-busy`; nothing queues or
forces a second tunnel. Clients outside KNXBench are not covered by the lease.

## Evidence

Simulator only: `knx-net` `diagnostic_export` (6) and `diagnostic_export_protocol`
(5: malformed descriptor, element-count/length mismatch, timeout, cancel during an
unanswered read, wrong start index; exhaustive outgoing-service allowlist; plus 9
for the memory profile: full coverage, each identity mismatch and an inexact
identity read without any memory read, missing assertion before any frame,
refused range, partial, timeout, cancel, deadline),
deadline unit test; `knx-app` `device_diagnostics` (9: export/reread/0600/no
overwrite/privacy, exclusion before adapter, lease, cancelled partial, corrupt
archives, durable activity kind, free-text error refusal, every observer
snapshot exportable, in-flight item shape);
`knx-server` `http_device_diagnostics` (12, including durable history and token
separation; since DIAG-01 also the web memory profile: preview without bus
traffic, consented reads checked chunk by chunk against the simulator's memory,
exhaustive outgoing services, format-2 ZIP reread with `read_export`; unknown or
missing profile, missing product database, missing or inconsistent identity
assertion and excluded targets refused before any tunnel; PID 78 mismatch with
no `A_Memory_Read`; a pre-tunnel setup stop shown but not exportable; with a
gated simulator, running polls showing exactly one chunk in flight and
`exportable: false`, Stop after the first chunk ending `cancelled` with partial
coverage and a format-2 ZIP reread with those chunks, and a Stop before the
tunnel exportable with no request); `knx-cli`
`device_diagnostics` (2, real binary; plus 3 for the memory profile: preview
with a silent loopback gateway, `--simulate` end to end, refusals); memory
profile also `knx-core` `declared_memory` (5), `knx-productdb`
`system2_declared_segments` (5) and the ignored exact public-package test,
`knx-app` `device_diagnostics_memory` (31: derivation, format-2 roundtrip,
0600/no overwrite, strict refusals, format 1 unchanged, size ceiling, identity
admission, chunk order, real fault roundtrips, duplicate and competing-end-record
ZIPs for both formats, ZIP comment, every observer snapshot exportable,
in-flight chunk shape, impossible identity sequences and identity stop reasons
in each format, real collector stop outcomes through both profiles and reader
APIs, every
snapshot of refused-PID12 and mismatch runs, the real deadline and missing-
assertion outcomes, the 64/65-octet PID 78 bound, rehashed phase-inappropriate
chunk errors in complete reports and snapshots, exact partial/refusal error
labels, real memory-exchange failures and their snapshots, cancellation before
and during a chunk, and unfinished device teardown)
and the raw central-directory unit tests (4); web
component/client tests; built shell in
Chromium against the real HTTP handlers and the regular simulator (EN/DE,
1440/400 px, stop-with-partial; memory profile EN 1440 px and DE 400 px with a
real program-missing refusal, declared ranges, coverage and a format-2 ZIP, and
a memory Stop after the first chunk with its translated reason and a format-2
partial ZIP),
every downloaded ZIP independently re-verified.
The browser proof also asserts that a clean run shows no failure banner: the
client once refused the in-flight progress snapshot ("attempted, not completed")
and showed a false "request failed" after a successful read. That snapshot is
admitted only while the report is incomplete.

## Required before any hardware claim

A separately authorised, read-only run against one named device, through the
user's own interface, with the raw report reviewed: does the device answer PID12/
PID78, refuse them, or stay silent (devices without interface objects), and does
`T_Disconnect` leave it in normal operation? Until then hardware compatibility is
unconfirmed.

**First real device (18 October 2026, issue #22):** one reporter-run export from
a Siemens `MV-0025` presence detector on a source build after Alpha 14
(`simulated: false`, `written: false`, complete). Descriptor, PID12 and PID78
all answered with the expected lengths; the disconnect attempt completed; the
reviewed values are in the
[evidence note](../research/2026-10-18-up258-device-diagnostics.md).
**Eleven more devices (18 October 2026, same build, by email):** Hager,
Berker, MDT and Video-Star (GVS) units with masks `0701`, `0705` and `07B0`.
All runs complete, nothing written, every assertion agreed with the device.
Descriptor and PID12 answered everywhere; PID78 answered on nine and was
refused with zero elements on two (`refused-ambiguous`, the path's first real
observation). The private gate `crates/knx-app/tests/device_reports_corpus.rs`
runs today's reader over all twelve archives and requires the writer to
reproduce each byte for byte ([evidence note](../research/2026-10-18-device-diagnostics-eleven-devices.md)).
**The maintainer's house (18 October 2026, owner go, build `g611c5b7a`):** all
34 contactable devices once, `1.1.220` excluded and never contacted. 30 answered
all three reads; the Merten actuator refused PID78 with zero elements (its
application declares no PID78 comparison); both `MV-0012` SmartSensors answered
the descriptor and then ended the connection about 6.3 s after the PID12 request
(`connection-lost`, the released-connection path, now observed); `1.1.253`
rejected `T_Connect` (`connect-rejected`). The gate's house set pins all 34
archives ([evidence note](../research/2026-10-18-house-device-diagnostics.md)).
Afterwards the owner reported that the whole installation keeps working
normally (an operator observation, not a functional test per device).
Still unobserved: a device that stays silent until the session timeout.
46 devices do not make the export hardware-compatible in general.
