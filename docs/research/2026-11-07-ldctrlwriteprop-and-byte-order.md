# ETS 6.4 download images: LittleEndian numbers, property writes, delays, controllers

Date: 2026-11-07. Scope: KL-170 and the load steps that kept memory
(`0701h`) push-button and room-controller programs from planning.

## Method and provenance

ETS 6.4 programmed devices added from unchanged product packages into a fresh
project; every application download went to KNXBench's own simulator gateway
(`crates/knx-net/examples/simulator_gateway.rs`), which answered as the device
and recorded every frame. ETS logs the memory image it is about to write and
each load-procedure step it executes; those logs and the simulator captures
are the evidence. No manufacturer data, project file or raw capture is
published; the counts below are taken from them. This is tool behaviour
against a simulator, not hardware: KNXBench's resulting downloads stay
`untested` until a device confirms them.

Programs: three memory programs declaring `ParameterByteOrder="LittleEndian"`
(two push-button programs, one room controller, manufacturer `M-0006`) and two
memory programs of a second manufacturer (`M-0009`) for the load steps.
Defaults first, then two configured variants of one push-button program.

## Findings

1. **`[V]` LittleEndian whole-octet numbers are written low octet first.**
   The three `M-0006` images hold 10 octet-aligned multi-octet number fields;
   9 of them discriminate (their two octet orders differ), and all 9 hold
   their value low octet first, e.g. 4354 (`1102h`) as `02 11`. KNXBench's
   encoder, reversing whole octets for such fields only, reproduces all 10
   octet for octet. Every other differing octet in these images is the
   communication-enable bit below. Fields crossing an octet boundary without
   being whole octets, floats and negative values were not exercised and
   stay refused.
2. **`[V]` `LdCtrlWriteProp` writes one element of the described width.**
   For `<LdCtrlWriteProp ObjIdx="5" PropId="51" Verify="false" InlineData="01"/>`
   ETS first read the property's description (`A_PropertyDescription_Read`),
   then wrote one element (`NumberOfElements=1`, `StartIndex=1`). With the
   simulator describing datatype `02h` (`PDT_UNSIGNED_CHAR`) it wrote `01`;
   with `04h` (`PDT_UNSIGNED_INT`) it wrote `01 00`, the given octet followed
   by a zero. The procedures write `01` before the unloads and `02` before the
   restart (`02` and `02 00` respectively). A longer `InlineData` was not
   observed; KNXBench accepts one only if everything past the element is zero.
   Element widths follow *Standard Identifier Tables* (`03_07_03` v01.04.01)
   §4.1 Table 6. KNXBench writes only when the description admits it
   (write-enabled, at least one element, a fixed width) and refuses otherwise
   without writing.
3. **`[V]` `LdCtrlDelay` is a wait.** `MilliSeconds="10"` before
   `LdCtrlRestart`: ETS logged the delay step and sent the restart 10 ms
   later. Nothing is sent for it.
4. **`[V]` A `choose` whose controller is placed but inactive selects
   nothing.** The push-button program keeps three virtual "LED status"
   parameters per button; the button's function shows exactly one of them, and
   each has its own `choose` that sets the same union member.
   - Variant A: function "toggle", visible controller "status indication",
     LED behaviour "blink on 1" (member value 5); the hidden controllers keep
     their default 0 (member value 0). ETS wrote `05`.
   - Variant B: the hidden controller set to "always on" (member value 1)
     while it was visible, then the function switched back to "toggle" and the
     visible controller set to "always off" (member value 0). ETS wrote `00`.

   So neither "last wins" (A would be `00`) nor "largest wins" (B would be
   `01`) explains ETS; ignoring the `choose` of an inactive controller does,
   in both. KNXBench's evaluator now does that for controllers that some
   `ParameterRefRef` places in the tree; a controller no `ParameterRefRef`
   names was not observed and is read as before. Both variant images match
   KNXBench's image in every parameter octet.

5. **`[V]` A segment longer than its allocation is written up to the
   allocation.** The room controller's code segment holds 8,292 octets, its
   `LdCtrlAbsSegment` allocates 1,792; ETS wrote exactly `43FEh`–`4AFDh`. No
   parameter or union of that program lies past octet 1,792. KNXBench writes the
   allocated prefix only while every written parameter field and placed table
   lies inside it, and says so as inference `segment-beyond-allocation-not-written`.
6. **Order, not observed as ETS does it.** The room controller's procedure
   writes property 5/51 *before* comparing the hardware type; ETS follows that
   order. KNXBench checks the device before changing anything, so it runs such a
   comparison first (inference `compare-before-property-write`): a read moved
   earlier can only refuse a download, never write to the wrong device.

7. **`[V]` Text of a program without `TextParameterEncoding` is
   Windows-1252.** The room controller declares no encoding. ETS wrote
   "Hallo Büro!" as `48 61 6C 6C 6F 20 42 FC 72 6F 21`, zero-filled, and
   `€ßÄö` as `80 DF C4 F6`. `80h` for the euro sign exists in Windows-1252
   only. The ETS installation runs a Western European Windows; whether ETS
   uses the Windows code page and would differ elsewhere is not known.

## Not settled here

- Five group-object-table config octets differ in the defaults and both
  variants: ETS clears communication enable on unlinked active objects
  (KL-145), KNXBench keeps the product's flag.
- Property-placed parameters, `TypeTime`, `LdCtrlMasterReset` and a union
  starting mid-octet with members at other bit offsets are separate
  boundaries; the mid-octet union was settled on 9 November
  ([note](2026-11-09-assigned-controllers-and-short-segments.md)).
