# Serial-number address-write evidence

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-139)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

<a id="139-a-device-can-be-addressed-by-its-serial-number-and-nothing-has-done-it-live"></a>
## §139 Serial-number address write was ignored by the tested device

MP §2.4 serial-number read was verified on MDT `1.1.67`. The MP §2.5
address write was sent and ignored: `1.1.68` remained free. The user also
authorized a guarded `PID_SERVICE_CONTROL` bit-2 set/read-back/clear; the
write remained ignored. Correcting the four address broadcasts to SYSTEM
priority did not change that outcome (RESEARCH §19.15). The CLI reports
"sent, not confirmed"; do not represent the simulator implementation as
a successful live write. For this device the programming-button procedure
(MP §2.3) is the historically verified route. Public confirmed serial
writes now fail closed before a tunnel (ADR-0057): an action-specific
durable pre-write backup of all affected storage is unavailable. That
gate is separate from device support and from the property-only backup
below. A different identified device or manufacturer statement would
address support, but not waive recovery or fresh authorisation.

The Debug bit-2 server route remains default-off. ADR-0051 now requires a
durable, versioned pre-write record of the original two property octets,
device address and mask. CLI and HTTP persist, read back and sync the record
and directory before any bit-2 change; backup failure refuses the write and
a no-op writes neither file nor property. HTTP reports `backupPath`; CLI
uses `./device-backups/` or `--backup-dir`. Simulator/route/CLI coverage
exists, but this *new recovery gate* has not run on hardware. It is a
property-specific manual recovery aid, not a full device image or automatic
restoration of manufacturer side effects. After an ambiguous write, inspect
the device and backup rather than retrying blind. The Settings Debug toggle
and explicit service-control tab now expose this narrow action; the toggle
reads the server setting back, and the panel requires an explicit read and
typed phrase before sending a write. A returned property result is not a
whole-device receipt; bounded `serviceControlWrite` activity is not a
durable audit trail (ADR-0055/0056). The new UI/backup gate is covered only
by local mock/simulator tests, **not a new live hardware run**.

**Recovery ordering correction, 2026-10-02.** The original property backup
was taken after connection setup had already asserted Verify Mode. The
service-control path now postpones that setup write, validates the original
PID_DEVICE_CONTROL byte, and durably keeps both original properties in
format 2 before any property write. Legacy format 1 is preserved, not silently
filled with an invented byte. No-op and wrong-scope calls do not write;
malformed widths and failed backups refuse. This does not close whole-device
recovery, durable history or hardware-support residues (ADR-0051 amendment).
<!-- END retained catalogue record -->
