# UP 258D12: first real-device diagnostic export

18 October 2026; issue #22. A reporter-supplied, read-only
[diagnostic export](../contracts/device-diagnostic-export.md) from the Siemens
presence detector UP 258D12 (application `M-0001_A-9205-04-9270`), sent by
email as requested in the issue. This is **identity evidence from one device,
not a programming result**: nothing was written, and no System-2 download
support changes.

## The archive

The ZIP stays in the private evidence collection; it is not committed. Only the
values below are reproduced. The archive's `projectReference`, timestamps and
file digests are deliberately left out.

| Field | Value |
| --- | --- |
| Kind / format | `knxbench-device-diagnostics`, format 1 |
| Build | `0.1.0-alpha.14+g0341483c` (a source build of `main` after Alpha 14) |
| Profile | `generic-identity-v1`; no asserted mask or manufacturer |
| `simulated` / `written` / `complete` | `false` / `false` / `true` |
| Memory | `not-requested` (the contract's identity-only boundary) |
| Disconnect | attempt completed |

Integrity: all three manifest entries (`README.txt`, `properties.json`,
`report.json`) match their declared size and SHA-256, and `properties.json`
equals the `items` of `report.json`. Per the archive's own README, checksums
detect corruption, not deliberate rewriting.

The reporter's issue comment calls the emailed file the offline device support
report with no bus contact. The archive received is the diagnostic export, which
does contact the device (three read requests). Whether a separate offline report
was also sent is not known here; the difference changes nothing below.

## Observed replies

| # | Request | Reply | Reading |
| --- | --- | --- | --- |
| 1 | Device Descriptor Type 0 | `00 25` | Mask `0025`, as the application's `MV-0025` declares |
| 2 | Object 0 / PID 12, start 1, count 1 | `00 01` | Manufacturer `0001` (Siemens), as `M-0001` declares |
| 3 | Object 0 / PID 78, start 1, count 1 | `00 01 00 00 00 5B` | One six-octet element (hardware type) |

Every read returned the expected byte count and one element; no refusal and no
timeout occurred. Until now the mask was a reporter and project assertion
([vertical-path audit](2026-10-16-up258-vertical-path-audit.md)); it is now
observed on the device. Agreement of mask and manufacturer is still not
product or application identity, and the firmware string remains unknown.

## Against the declared hardware comparison

Read again from the digest-pinned public package
(`520d3fcee5b63d079ac7d0bd5e7ce9467ff84cb5cde06ac341b77aba47e3ee0f`): position 1
of the application's load procedure is the application's only PID 78
comparison,

```xml
<LdCtrlCompareProp InlineData="00010000005B00000000" ObjIdx="0" PropId="78" />
```

The device's six octets are exactly the first six of those ten; the other
four are zero. The existing assumed rule `property_matches`
(`crates/knx-core/src/commissioning/memory_download.rs`, labelled `[A]`)
accepts precisely this shape: the device's octets are the start of the data
and every later octet is zero. On this device it gives **match**. A byte-for-byte
comparison over all ten octets could never match a six-octet reply.

What this settles, and what it does not:

- It is a second manufacturer and mask (`0025`) for `property_matches`, next
  to the rule's MDT `0701` source example (`A-0027-15-0BAC`, ten octets
  `00 00 00 00 01 27 00 00 00 00`). Same shape: six hardware-type octets plus a
  four-octet zero tail. The rule gives the expected answer here too.
- It does **not** define how a manufacturer tool compares (whether it ignores,
  pads or checks the tail), and a single device cannot. The rule stays `[A]`.
  The source-to-wire question in blocker 2 of the
  [System-2 prerequisites](../contracts/system2-download-prerequisites.md) is
  narrowed (the device side is now measured), not closed.
- The hardware type identifies the hardware variant, not the loaded
  application, its version or the firmware.

No code changes: the existing rule and its tests already cover this
six-versus-ten-octet shape, and the values above are recorded here as evidence.

## The collector on hardware

This is the export's first run against a physical device. All three reads
answered, which [KL-184](../limitations/KL-184-device-diagnostic-export.md)
and the contract had listed as unconfirmed. Still unobserved: a device that
refuses or ignores property reads, and whether the detector kept working
normally after the run; the archive only records that the disconnect attempt
completed, and the reporter has not said either way yet.

## Still blocking a download

Unchanged: the [vertical-path audit](2026-10-16-up258-vertical-path-audit.md)
and the [production decision](2026-10-16-up258-production-decision.md) list the
remaining source semantics (patch/default/activation, encoding, Union), the
complete source-to-executor procedure mapping and identity-bound recovery for
every property, allocation and memory effect. The download refusal stays in
place.
