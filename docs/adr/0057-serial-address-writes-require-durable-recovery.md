# ADR-0057: Serial-address writes fail closed until durable recovery is demonstrable

- Status: Accepted (2026-09-30)
- Scope: KNXBench's production CLI and HTTP entry points for MP §2.5; no live operation

## Evidence and problem

The existing `serial_number_write` procedure (MP §2.5) reads the device's
current individual address by serial number, checks the requested address for
occupancy, broadcasts the address write when needed, then reads by serial
again. That protocol readback is **not** a persistent pre-write backup. Before
this decision, `POST /api/device-address/by-serial` and `knx device
address-by-serial --confirm` could enter the write without persisting even
the previous address and serial number. If the request was cancelled or the
process died, its in-memory read and eventual HTTP/CLI result were not durable
recovery evidence (ADR-0056).

The KNX Architecture v3.0, §2.6, cautions that specification identifiers and
formats describe a network interface, not necessarily a device's internal
memory map (KNX Association, `03_01_01 Architecture v3.0`, p. 9;
https://pahl.de/download/dissertation/ds2os.lab/files/03_01_01_architecture_v3.0.pdf).
KNX Association's Device Reader documentation says memory readout needs an
individual address, a *specified memory range and address space*, and
possibly an access key; interpreting the result needs
product/system knowledge
(https://support.knx.org/hc/en-us/articles/115001822070-Device-Reader).
These sources do **not** identify every manufacturer-specific storage region
that changes on an MP §2.5 write. We therefore cannot assert that recording
just the old individual address—or an unrelated application-memory dump—backs
up **all affected storage** on arbitrary devices. No access key may be guessed.

## Decision

1. A shared application-layer precondition refuses every confirmed
   serial-address write until KNXBench can durably persist and read back a
   complete, action-specific pre-write recovery record for the affected
   storage **before** the broadcast. There is no environment switch, HTTP
   flag or CLI option to bypass this gate. The server returns HTTP `412
   Precondition Failed`; the CLI returns failure. Both refuse before opening
   a tunnel. Validation still rejects bad addresses, serials and confirmation
   phrases first, without contacting a gateway.
2. Even a request for the device's current address is refused: it cannot be
   identified as a no-op without first opening a tunnel, and this public
   write entry point must remain fail-closed until its recovery contract is
   proven. The read-only `find-serial` route/command and CLI dry-run plan
   remain available; the plan explicitly warns that `--confirm` is blocked.
3. Keep the MP §2.5 protocol procedure and its direct simulator tests. They
   establish telegram and readback semantics, **not** a permission to use the
   production CLI or HTTP route on hardware. This gate is at the application
   entry points; a custom Rust caller invoking the low-level protocol API is
   outside this HTTP/CLI guarantee.
4. Do not emit a fabricated `serialAddress` write-success activity entry.
   Keep ADR-0056's `untracked`/partial coverage until a recovery-safe route
   has route-specific send/backup/readback/cancellation evidence.

## To unblock, not an automatic authorization

Research the affected regions for each supported device class/mask and
identify a complete, read-only backup method, including any required
operator-supplied key. Define a versioned receipt binding serial, old/new
address, device identity, storage scopes, exact pre-write values, persistence
and readback; reject incomplete/unsupported/ambiguous scopes. Add same-session
pre-broadcast callback and cancellation/transport tests, with fsync and a
separate deliberate recovery/abort procedure. Re-evaluate the two live MDT
refusals before attempting another device: this decision does **not** grant a
new hardware run. Do not rely on a post-write response, a confirmation phrase,
or an `A_Memory_Read` of arbitrary ranges as a substitute for the backup.

## Consequences

This deliberately reduces public write availability to preserve data
integrity. Simulated CLI/HTTP tests now assert pre-tunnel refusal, including
possible no-op and a successful service-control bit-2 change; protocol-layer
simulator tests retain the actual MP §2.5 behavior. No web code or live bus
was touched. Other address-programming paths and their separate recovery
contracts are not changed by this ADR.
