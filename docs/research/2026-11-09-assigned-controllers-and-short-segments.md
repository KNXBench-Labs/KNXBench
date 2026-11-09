# Assigned controllers, unnamed controllers, short segments and mid-octet unions (ETS 6.4)

Date: 2026-11-09. Scope: three devices of the tester project of issue #10 that
stayed `unsupported` after ADR-0167: a one-button push-button sensor (program
`M-0006_A-E0CC-11-CCA5`, mask `0701h`) and two binary inputs (program
`M-0009_A-2080-10-A8CD`, mask `0701h`; the related `M-0009_A-206A-12-E24D`
shares the union shape).
Decision: [ADR-0168](../adr/0168-assigned-and-unnamed-controllers.md). Builds on
[ADR-0160](../adr/0160-inactive-choose-controllers-select-nothing.md).

## Method and provenance

ETS 6.4 downloaded the unchanged product programs to KNXBench's own simulator.
The devices sat in a new fictional project with three devices: two binary
inputs and one push-button sensor, each with a few parameters changed from
their defaults and five group links. The project was exported, and KNXBench
imported that export to build its own images for the same configuration.
ETS's memory image comes from its own download log; the frames come from the
simulator capture. Logs, captures and the export stay private. This is tool
behaviour against a simulator, not device evidence.

A second, independent oracle for the activation questions is the tester's own
project: for every device ETS records which group objects are active
(`GroupObjectTree`); 51 of its devices carry one.

## The programs

1. **Assigned controller.** The push-button program decides "push-button
   1-2 independent or rocker" with a controller (`Push-button 1-2`) that is
   placed only on a "using mode" page. A constant hidden parameter selects a
   using mode whose branch does not show that page, but carries an `Assign`
   that sets the controller from another hidden constant (`0`, independent
   push-buttons). KNXBench does not execute `Assign` (design D10); ADR-0160
   then counted the controller as inactive, so its `choose` selected nothing
   and the button channel with objects 13 and 18 vanished.
2. **Unnamed controllers.** The same program keeps several "virtual"
   controllers that no `ParameterRefRef` names anywhere: a third LED-status
   function per button (`ChC_Function_of_LED_Status_Heating_virtual`), whose
   `choose` sets the same union member as the shown LED-status controller,
   and `Push-button 5-6`, whose only `when` (value 0) shows two channels with
   visible function parameters. ADR-0160 read such controllers as before
   (unobserved).
3. **Short segment.** The procedure allocates 6,182 octets at `43FEh`; the
   program's segment there holds 1,457.
4. **Mid-octet unions.** The binary-input programs place 6-bit unions at
   `BitOffset="2"` whose members sit at their own `BitOffset` 0, 3, 4 or 5.
   No published source defines how the two offsets combine.

## Findings

1. **`[V]` An assigned controller is evaluated.** ETS shows the button
   channel and its objects: in the tester project ETS records objects 13 and 18
   active on the device whose links KNXBench refused, and in the fictional
   project ETS offered both objects for linking. With the assignment's value
   (equal to the stored one) evaluated, KNXBench's active objects equal ETS's
   on all 51 tester devices; without it, on 50.
2. **`[V]` An unnamed controller competing with a shown one selects
   nothing.** The unnamed LED-status controller's branches set the same
   references as the shown one's. With the shown controller at "always on"
   (member value 1) and the unnamed one at its default (member value 0), ETS
   wrote `01`. ETS also listed exactly four channels for the push-button
   device (general, LED management, button 1, information): the two channels
   under `Push-button 5-6`, an unnamed controller with no competitor, were not
   shown although its value 0 selects them. KNXBench keeps evaluating that
   kind (see *What stays open*); its objects stay inactive either way.
3. **`[V]` An allocation longer than its segment writes the segment.** ETS
   logged the allocation `Memory[$43FE-$5C23]` but sent memory writes for the
   segment's 1,457 octets only; the simulator capture contains no write past
   `49AEh`. That allocation sets checksum control (`SegFlags="128"`), so the
   short write was observed with it on.
4. **`[V]` A mid-octet union adds the bit offsets.** With the union at bit 2,
   members at bit 3 (3 bits, value 4), bit 4 (2 bits, values 3 and 2) and
   bit 5 (1 bit, value 1) were written at bit 5, 6, 6 and 7 of the union's
   octet, in all four cases. The two other readings (member offset alone,
   union offset alone) disagree with every one of them.

## KNXBench against these images

For the same configuration, KNXBench's parameter segment equals ETS's in every
octet except the communication-enable flag of unlinked active objects (KL-145):
one octet for one binary input, two for the other, three for the push-button
sensor. Both union octets and the LED-status octet match.

In the tester project four devices move from `unsupported` to `untested`: the
push-button sensor 0.2.49 and the binary inputs 0.2.7, 0.2.63 and 0.2.66
(10 unsupported, 65 untested). No other device's plan or image changes.

## What stays open

- `Assign` is still not executed: a controller whose assignment yields a value
  other than the stored one, or an unknown one, is reported
  (`AssignedControllerDiffers`) and the download refused.
- The unnamed-controller rule rests on one program and is limited to
  controllers that compete with a shown one for the same reference. Unnamed
  controllers without a competitor, such as `Push-button 5-6`, are still
  evaluated, so KNXBench lists two channels ETS hides (no objects or octets
  differ). A broader rule lost the refusal of illegal hidden selector values
  in review and was not taken.
- The mid-octet union reading was observed for mask `0701h` programs only,
  with members inside one octet; members crossing an octet and the legacy
  System-1 and System-2 image layouts keep refusing.
- No device confirmed any of this; the downloads stay `untested`.
