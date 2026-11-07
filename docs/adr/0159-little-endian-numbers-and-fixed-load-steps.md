# ADR 0159: LittleEndian numbers and fixed load steps follow observed ETS images

Date: 2026-11-07
Status: Accepted

## Context

KL-170 refused every memory image of a program declaring
`ParameterByteOrder="LittleEndian"`: no available source defines the order per
parameter type, and the lift condition asked for a read-only probe of an
ETS-programmed device ([ADR-0119](0119-read-only-byte-order-probe.md)). No such
probe output arrived, while a tester's installation has 32 push-buttons on one
such program (issue #10). The same programs also use two load-procedure steps
KNXBench did not translate, `LdCtrlWriteProp` and `LdCtrlDelay`.

ETS 6.4's own download images, recorded against KNXBench's simulator, settle
what the tool writes for these programs
([research note](../research/2026-11-07-ldctrlwriteprop-and-byte-order.md)):
all 9 discriminating whole-octet numbers low octet first; a property write as
one element as wide as the device's description, zero-filled; a delay as a
wait. That is evidence of the tool, not of a device.

## Decision

- In a memory (`070nh`) layout, a `LittleEndian` number whose field starts on
  an octet and is whole octets wide is written low octet first
  (`knx_core::commissioning::parameter_image::reverse_octets`); the readback
  comparison applies the same reversal. A field inside one octet has no order.
  Every other field crossing an octet boundary, legacy layouts and any other
  declared order keep the typed `UnsupportedByteOrder`/`Unsupported` refusal.
- `LdCtrlWriteProp` with `InlineData`, `Verify="false"` and no other
  attribute becomes `MemoryDownloadStep::WriteProperty`: read the property's
  description, write one element of the described width (`InlineData` from
  its first octet, zero-filled), and refuse without writing if the
  description does not admit it. Other shapes stay unmodelled.
- `LdCtrlDelay` becomes `MemoryDownloadStep::Delay`, a wait that sends
  nothing.
- These downloads stay `untested` (`--accept-untested`) until hardware
  confirms them.

## Alternatives considered

- **Keep waiting for probe output.** Leaves the reported devices without any
  download and gives no better evidence about what ETS writes than ETS itself.
- **Swap octets for every LittleEndian field.** Unobserved for fields across
  octet boundaries, floats and negative values; refused instead.
- **Write `InlineData` verbatim.** Would send the wrong width when the device
  describes a wider or narrower element.

## Consequences

The tester's push-buttons plan completely. Readiness category `byte-order`
remains for the refused shapes. Every newly translated step is covered by
exhaustive matches (failure classification, partial download, restore,
programming state, support report). A device that rejects the write or
describes the property otherwise stops the download before the unloads.
