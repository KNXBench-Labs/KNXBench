# ADR 0051: Individual Address Write Enable is an opt-in debug action, never automatic

Date: 2026-09-30
Status: Accepted
Session: commissioning follow-up to the user's K12 decision

## Context

`[D]` RES §4.2.8 Table 10: bit 2 of `PID_SERVICE_CONTROL` (Device Object,
PID 8, `PDT_UNSIGNED_INT`) is *"Individual Address Write Enable"*, default
`0 = disable`. *"If this bit is cleared, it shall not be possible to change
the Individual Address of the device"* via programming mode or KNX Serial
Number services. Mask `0021h` codes the bit inversely. Profiles A.2.3.1
makes the property optional for mask `0701h`.

`[V]` KNOWN_LIMITATIONS §139: the test device `1.1.67` keeps the bit clear
(`0000h`). It ignored an MP §2.5 serial-number write, and KNXBench reported
it as *"sent, but NOT confirmed"*. Yet the programming-button path worked on
it twice. So the bit gates only the serial path on this device.

No Management Procedure asks a client to write the bit. The user's decision
(2026-09-30) was: KNXBench keeps never setting it on its own, but an option
in Settings, if sensible in a debug section, may allow the operator to do it
explicitly.

## Decision

1. **Nothing in KNXBench sets the bit on its own.** Neither the MP §2.5
   procedure, nor a download, nor an address programming touches
   `PID_SERVICE_CONTROL`.
2. **A separate procedure and scope.**
   `knx_net::commissioning::service_control` reads the property read-only,
   and changes it under a new `WriteScope::IndividualAddressWriteEnable`
   with its own confirmation phrase (`I confirm individual-address write
   enable to <address>`). The change is a read-modify-write of bit 2 only:
   the other fifteen bits go back as read. It is checked by exact-octet
   read-back. It is refused by name for mask `0021h` (inverse coding), for
   a device without the property, and for a value that is not two octets.
   If the bit already has the requested value, nothing is written.
3. **Opt-in, enforced by the server.** `GET` and `POST
   /api/device/service-control` answer `403` unless the settings file holds
   `debugIndividualAddressWriteEnable: true`. An unset key, any other value,
   or an unreadable or newer settings file all count as off. The gate is in
   the server, not only in the UI, so a request that bypasses the panel is
   refused too. Refusals happen before a tunnel is opened.
4. **The Settings panel gets a Debug section** carrying the toggle. That
   is a web change and waits for the web lock (goal-ui §3); until then the
   key can be set with `PUT /api/settings`.

## Alternatives considered

- **Set the bit automatically before a serial-number write.** Rejected by
  the user. It is a permanent control field, and changing it silently
  changes the device's behaviour beyond the one write.
- **A CLI command only.** Rejected: the user asked for a Settings option.
  A CLI command was added alongside (`knx device service-control`,
  2026-09-30); it needs no setting, as the CLI already demands the typed
  phrase.
- **Gate only in the UI.** Rejected: an HTTP request would reach the write
  without the opt-in.

## Consequences

- The serial-number path can now be completed on a device that keeps the bit
  clear, in two explicit steps, each with its own phrase.
- `hardware_write_is_authorised` allows the new scope. The scope's phrase is
  distinct from every other scope's, so no existing confirmation covers it.
- The simulator models the property: its bit 2 decides whether a
  serial-number write takes effect, and a device can be configured without
  the property.
- **Not run on hardware in this implementation package.** Setting the bit on
  `1.1.67` needs the user's explicit go. Clearing it back afterwards is the
  same route with `enable: false`.

## Amendment (2026-09-30): pre-write recovery for the property

The service-control route and CLI formerly offered the write with only an
in-memory pre-read. That was insufficient recovery evidence if the client
crashed or the write/readback became ambiguous. Before exposing the HTTP
setting in the UI, both production entry points now use a *property-specific*
backup gate: the connected session reads the mask and **both octets** of
Device Object `PID_SERVICE_CONTROL`; if the requested bit differs, a callback
must persist exactly those octets, mask and target in a versioned JSON record,
read it back and fsync the file and directory **before** `A_PropertyValue_Write`.
An I/O/readback failure refuses before the write. The file is new-only and
owner-only on Unix. CLI defaults to `./device-backups/` (or `--backup-dir`);
HTTP uses `<data_dir>/device-backups/` and returns `backupPath` on success.
A no-op performs neither backup nor property write. After a protocol failure,
the path is reported for manual diagnosis; no automatic rollback is inferred.

This record covers **the complete property value this procedure overwrites**,
not an application-memory image. The protocol write names only object 0, PID
8, one two-octet element; no known `A_Memory_Write` is in this procedure.
Neither this fact nor a property readback proves that a manufacturer has no
side effects elsewhere. Recovery requires an operator to verify the device's
identity and mask, inspect the recorded original octets, and deliberately
request the reverse operation with its own new backup and confirmation. The
current API only changes bit 2; if any other original bit has since changed,
do **not** use this API as a whole-property restore. K13/reset and downloads
remain separate workflows; this property record does not satisfy their backup
requirements. No hardware operation was run to validate this amendment.
