# Eleven more devices: diagnostic exports from a real installation

18 October 2026. After the [UP 258D12 export](2026-10-18-up258-device-diagnostics.md),
eleven more read-only
[diagnostic exports](../contracts/device-diagnostic-export.md) arrived by
email, collected the same evening with the same source build
(`0.1.0-alpha.14+g0341483c`). This is
**identity evidence, not a programming result**: nothing was written, and no
download admission changes.

The archives stay in the private evidence collection and are not committed.
This note keeps to manufacturer IDs, masks, read outcomes, catalogue
comparisons and generic device classes. It leaves out order numbers, the
archives' `projectReference`, timestamps, digests, the sender's file names and
who sent them. An installation's device list is its owner's business, not ours.

## Integrity: the strict reader and the writer agree with the hardware

All twelve archives in the collection (these eleven plus the detector) pass the
production reader `device_diagnostics::read_bundle`: four fixed members,
manifest sizes and SHA-256 values, `properties.json` equal to the report's
items, `simulated: false`, `written: false`, `complete: true`, memory
`not-requested`, disconnect attempt completed. Today's writer re-bundles every
one of them **byte for byte**. Every run asserted mask and manufacturer from the
project except the detector's, and no assertion was contradicted.

The private gate `crates/knx-app/tests/device_reports_corpus.rs` pins exactly
these aggregates (12 archives, 36 reads, the status and mask distributions,
5 manufacturers), fails on any drift and prints counts only. Negative controls
(a missing directory, a removed archive, one tampered byte) each turn it red.

## What the devices answered

Device classes come from the sender's labels, cross-checked with the
catalogue product that runs the application named in the last column. Matching
a report to an application is an inference from that label plus the observed
mask, manufacturer and hardware type; the report itself names no product.

| Manufacturer (PID 12) | Device class | Mask | PID 78 | Application's PID 78 check | Result |
| --- | --- | --- | --- | --- | --- |
| `0009` Hager | 4-fold dimming actuator | `0701` | 6 octets | 10-octet `LdCtrlCompareProp` | match |
| `0009` Hager | 20-fold switch actuator | `0701` | 6 octets | 10-octet `LdCtrlCompareProp` | match |
| `0009` Hager | 12-fold shutter actuator | `0701` | 6 octets | 10-octet `LdCtrlCompareProp` | match |
| `0009` Hager | presence detector | `0701` | 6 octets | 10-octet `LdCtrlCompareProp` | match |
| `0009` Hager | 2-input binary input | `0701` | refused (0 elements) | none declared | consistent |
| `0009` Hager | heating actuator | `0701` | refused (0 elements) | none declared | consistent |
| `0006` Berker | room thermostat | `0701` | 6 octets | 10-octet `LdCtrlCompareProp` | match |
| `0083` MDT | glass push button | `0705` | 6 octets | 10-octet `LdCtrlCompareProp` | match |
| `0083` MDT | logic module | `0705` | 6 octets | 10-octet `LdCtrlCompareProp` | match |
| `0083` MDT | glass touch panel | `07B0` | 6 octets | hidden `HWT_Check` parameter on PID 78 (below) | equal |
| `0085` Video-Star (GVS) | smart touch panel | `07B0` | 6 octets | none; compares object 4 / PID 66 (three octets) instead | not applicable |

Every descriptor and every PID 12 read answered with two octets. No device
stayed silent, timed out or released the connection. A whole run took 0.35 to
0.52 seconds; single reads took 80 to 250 ms.

### The two refusals

Both Hager devices that refused PID 78 answer with zero elements, the
[AL §3.4.4.2](../contracts/device-diagnostic-export.md#result-per-item) shape the
export records as `refused-ambiguous`. This is the path's first real-device
observation. It stays ambiguous: the reply cannot tell denied access from a
missing property. Both applications declare no PID 78 comparison in their load
procedure, so nothing in the product data expects the device to report one.

### Against the declared hardware comparisons

The comparison values were read from every product archive held locally
(nested ZIPs included) by a bounded text scan of the application XML, not
through the product-database ingest: 398 applications declare an `LdCtrlCompareProp` on
object 0 / PID 78, 378 of them in public vendor packages. For each reply that
was read, the scan looked for applications whose comparison data the existing
assumed rule `property_matches`
(`crates/knx-core/src/commissioning/memory_download.rs`, labelled `[A]`)
accepts: the device's octets are the start of the data and every later octet is
zero.

- **Eight of eight** devices whose application declares a PID 78 comparison
  match it under that rule: the seven above plus the UP 258D12. All eight
  comparisons are ten octets with a four-octet zero tail. A byte-for-byte
  comparison over ten octets would match **none**. Four manufacturers (Siemens,
  Hager, Berker, MDT) and three masks (`0025`, `0701`, `0705`) now support the
  rule's shape; it remains `[A]`, because no source defines how a manufacturer
  tool compares.
- In each case the matching comparison belongs to the application the device's
  class points to, never only to an unrelated one.
- The hardware type is not application identity. The presence detector's
  `00 01 02 03 04 07` is the comparison value of 12 applications from two
  manufacturers in the collection, so it passes the check of any of them.
- Hager's three actuators carry their application number in octets 4 and 5 of
  the hardware type; the Berker thermostat does not (its application number
  differs from those octets). No general structure is inferred.
- **Counterexample in the catalogues:** of the 398 comparisons, 369 are ten
  octets, 28 six octets and one, a Gira application, seven octets with a
  non-zero seventh (`00 00 00 00 20 00 03`). `property_matches` would refuse any
  six-octet reply against it. That fails closed, but it shows the zero-tail
  assumption is not universal. No device of that type has been read, so the
  correct comparison for it is unknown.

### G10: a hidden hardware-type parameter meets its device

[Census 3, G10](2026-10-18-product-database-programmability-census.md#gaps-and-follow-ups)
found 71 `0705` programs whose only property-placed parameter is a hidden
48-bit `BinaryValue` on object 0 / PID 78 with no PID 78 comparison, and asked
what a tool does with it. The MDT glass touch panel's `07B0` application has the
same shape: parameter `HWT_Check`, type `HWTCheck`, `Access="None"`, placed on
object 0 / PID 78 at offset 0, default enumeration `BinaryValue="AIMAAAMo"`. That
Base64 decodes to `00 83 00 00 03 28`, and the device answered PID 78 with
exactly those six octets. One version of the application adds a second
enumeration ("HWT 4 inch", `…03 29`), a different hardware variant.

This is the first device-side data point for the G10 hypothesis that such a
parameter holds the expected hardware type. It does not show what a tool does
with it (write, compare or ignore). KNXBench still never writes PID 78, and the
G10 programs remain refused.

## What changes

- The [contract](../contracts/device-diagnostic-export.md) and
  [KL-184](../limitations/KL-184-device-diagnostic-export.md) record twelve real
  devices from five manufacturers and the first real refusal. Still unobserved:
  a silent device, and any report on device behaviour after a run. The archive
  records only that the disconnect attempt completed.
- Blocker 2 of the [System-2 prerequisites](../contracts/system2-download-prerequisites.md)
  gets the wider device-side measurement; the comparison rule stays an
  assumption.
- No production code changes. The new private gate guards the reader and writer
  against these real archives.
