# Download readiness and pre-write backup

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-download-readiness-and-pre-write-backup)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## Commissioning readiness and pre-write backup boundary (ADR-0049)

Offline coverage of 103 locally installed product packages found 55/181
`0701h`/`0705h` application programs plannable with product **defaults** and
no group links; 126 were refused with reasons. A plan is not a verified
hardware download, and a one-device live run does not validate every
configuration or device revision. `verified` is scoped to application *and*
download type; `untested` needs a second exact acknowledgement. Other masks
remain refused. The catalogue is a sample, not an all-device support list.

The automatic backup is read **after** the target identity check but before
the first mutation, and records only memory regions the plan is about to
overwrite plus affected load states. It is **not a complete device memory
dump**, and does not preserve properties, keys held outside those regions,
masked/unwritten memory or a non-Loaded load state. The JSON can contain
private device configuration and must be protected and not attached to
diagnostics. Only a backup with all parts Loaded and a product procedure
that rederives the same steps and memory shape can become a restore plan.
Backup creation and restore were confirmed live once, on one device and
program (MDT `1.1.67`, `0701h`, `A-0027-15-0BAC`, complete download; RESEARCH
top entry, 2026-09-29): backup before the first write, restore
byte-identical over 180 dump lines; the parameters-only partial download
likewise (backup of `4400h`, K7 parameters written, restore to a
byte-identical dump). The backup/restore path for a group-address partial plan, other devices
and the refuse-before-write path on a failing backup remain simulator-only;
none can be promised as a universal rollback. The existing manual read-only
baseline required by `goal-commission.md` before live tests is separate and
still required when live tests are authorised again.

**Device-checks UI boundary (2026-09-30).** Readiness uses only the open
project's installed product database; `verified` applies to one application
and download operation, not general device compatibility. Comparison uses
the complete plan only (the CLI/API partial option is not shown there),
requires an explicit read-only tunnel confirmation and may be refused by a
read-protected device. It is neither a backup nor proof that a later write
will work. Web evidence is mocked/EN-DE and server evidence simulated, not a
live test of this view.
<!-- END retained catalogue record -->
