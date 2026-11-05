# Passive Linux USB-serial candidate listing (5 November 2026)

## Primary source and scope

Pinned primary source: systemd **v257**, [`rules.d/60-serial.rules`](https://raw.githubusercontent.com/systemd/systemd/v257/rules.d/60-serial.rules),
read on 5 November 2026 (SHA-256
`50bb6f79da19b52e82d273b8e142d49905bc69bdf765bf0545f2a20349d7d0aa`).
Lines 11–26 gate the serial aliases on `ttyUSB*`/`ttyACM*`
and require nonempty `ID_BUS`, `ID_SERIAL` and `ID_USB_INTERFACE_NUM` for by-id
names, with an optional port suffix. This supports consulting USB-prefixed
`/dev/serial/by-id` aliases; it does **not** establish that all USB devices have
aliases, that aliases uniquely identify physical hardware, or that any listed
port implements KNX/TP-UART. The implementation admits only decimal tty suffixes,
a deliberately narrower policy than the rule's shell pattern.

## Implementation contract (candidate; acceptance pending)

`GET /api/bus/serial-candidates` is a read-only, server/container-local operation.
It runs only when requested, off the async reactor, independently of the bus
connector/session. Production roots are fixed, not configurable by HTTP or an
environment variable. Filesystem operations are directory enumeration, symlink
metadata, canonicalization and target metadata only. No device open, read, tty
configuration, ioctl, network discovery or permission probe is performed.

The response is exactly one of:

```json
{"status":"available","candidates":[{"path":"/dev/serial/by-id/usb-fictional-if00","target":"/dev/ttyACM0"}],"truncated":false,"omittedEntries":0}
{"status":"unavailable","reason":"missingDirectory"}
{"status":"unavailable","reason":"permissionDenied"}
{"status":"unavailable","reason":"io"}
{"status":"unsupported"}
```

`available` may have no candidates. Missing directories, permission denial and
other filesystem errors are **not** reported as successful empty lists. Unsupported
means a non-Linux server, not an assessment of an individual adapter. A worker
failure maps conservatively to `unavailable/io` without filesystem error text.

Only UTF-8 `usb-*` symlink basenames of at most 255 bytes, without controls,
path separators or surrounding whitespace, qualify. The canonical target must
be an immediate `/dev/ttyUSB<digits>` or `/dev/ttyACM<digits>` character device.
Dangling/disappearing links and other excluded entries are omitted; other I/O
errors discard the partial listing and report unavailable. No existence or
metadata observation proves that a later Connect will work; devices and symlinks
can change immediately afterward.

At most 512 directory entries are inspected, plus one lookahead for truncation.
Within that prefix, aliases to the same canonical target are collapsed to their
lexically smallest alias. Rows are sorted by alias and capped at 64. `truncated`
discloses either bound; a bounded directory prefix can vary between requests.
`omittedEntries` counts inspected entries not represented in returned rows,
including filtering, duplicates and the row limit; it does not count the unvisited
tail. Neither a non-truncated result nor a zero count is a complete USB census.

The strict web decoder refuses malformed/extra fields, unsafe paths, repeated
aliases or canonical targets and exceeded bounds. The shared settings/footer/
monitor control lists only on explicit request. Completion never selects a path;
only an explicit row action sets the existing serial path preference. Manual
edits made while pending, source, mode, transport, IP preference and session state
are retained. Unmounted/retired requests cannot publish into a replacement control.
No fallback discovery, auto-connect, promotion, retry or reconnect is added.

## Verification boundary

Tests use injected entries/metadata, synthetic filesystem roots and intercepted
HTTP, never production enumeration of the host's `/dev` or `/sys`. The HTTP test
holds session/operation locks and uses a connector that panics on connect/discover.
EN/DE mounted tests cover pending edits, explicit selection, stale/unmounted
responses, limits and all error states. The existing USB/serial browser fixture
adds settings/footer/monitor cases at 400 and 1440 pixels with unexpected-request
refusal. Independent review and final selected package gates remain required;
this note is not acceptance or a physical hardware result.
