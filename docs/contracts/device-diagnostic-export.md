# Read-only device diagnostic export

[ADR-0134](../adr/0134-generic-read-only-diagnostic-export.md). A source-build
feature after Alpha 13; not a released-binary claim. Simulator-verified only:
**no physical device has been read with it.**

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
- **Memory: `not-requested`** in format 1. There is no admitted safe memory
  profile; product source offsets are not runtime addresses.
- Bounds: 3 logical reads, 30 s acquisition deadline (checked before every
  read), 10 s gateway connect, 3 s cleanups, 512 retained octets per reply.

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
CRC/length/hash mismatches and inconsistent copies. Checksums detect corruption,
not deliberate rewriting. Raw replies can still identify equipment: no anonymity
is claimed.

## Entry points (one application service)

- Web: **File → Export diagnostic data** (also beside *Device support report*).
  Preview (no bus traffic) → explicit read consent → progress, *Stop* (keeps
  partial data, still disconnects) → content preview → explicit save consent →
  local download. Nothing uploads; the user sends the ZIP to
  `issue@knxbench.com` or shares a private link.
- HTTP: `POST /api/device-diagnostics/preview {deviceId,gateway,context}`,
  `POST …/start {token,consent:true}` (202), `GET …/{token}`,
  `POST …/{token}/cancel`, `POST …/export {token,consent:true}`. Stale
  project revision, unknown token, false consent, excluded/ambiguous/missing
  target, non-IPv4 endpoint and unknown fields refuse before any tunnel.
  The acquisition runs as a server-owned task, so a closed browser cannot cut
  cleanup short.
- CLI: `knx device diagnostics --project p.knxdb --device-id N --gateway
  host:port --output new.zip [--product-db db] [--confirm-read]`; without
  `--confirm-read` it only previews. `--simulate` uses the built-in simulator
  and never constructs a network client. Ctrl-C stops cooperatively and saves
  the partial result. Write/key/memory flags are refused.

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
unanswered read, wrong start index; exhaustive outgoing-service allowlist),
deadline unit test; `knx-app` `device_diagnostics` (7: export/reread/0600/no
overwrite/privacy, exclusion before adapter, lease, cancelled partial, corrupt
archives, durable activity kind, free-text error refusal);
`knx-server` `http_device_diagnostics` (5, including durable history and token
separation); `knx-cli`
`device_diagnostics` (2, real binary); web component/client tests; built shell in
Chromium against the real HTTP handlers and the regular simulator (EN/DE,
1440/400 px, stop-with-partial), every downloaded ZIP independently re-verified.

## Required before any hardware claim

A separately authorised, read-only run against one named device, through the
user's own interface, with the raw report reviewed: does the device answer PID12/
PID78, refuse them, or stay silent (devices without interface objects), and does
`T_Disconnect` leave it in normal operation? Until then hardware compatibility is
unconfirmed.
