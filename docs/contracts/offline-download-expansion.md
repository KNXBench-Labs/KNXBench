# Offline-first device download expansion

## Owner scope (15 October 2026)

Implement as much of the documented download-path expansion as is supported by
verified evidence without device access. The owner's explicit continuation lifts
the 21:58 pause for this scope. Physical validation can happen in the field; no
live bus operation or automatic promotion to hardware-verified support follows.
The [direct-PDF audit](../research/2026-10-15-device-download-paths.md) defines the
family inventory and separates software acceptance from field verification.

## Completion contract

For each family, implement independently verifiable software slices until the
remaining dependency is an identified missing semantic/evidence contract, not
merely lack of an owned device. Keep a reviewed per-family disposition: delivered
code/tests, reused existing substrate, or exact research blocker. A generic
simulator, empty plan, report-only stub or widened mask allowlist cannot close
this goal. Unknown inputs remain preserved and explicitly refused.

Work incrementally in this order, revisiting later slices as evidence permits:

1. System-2 source metadata and table/image codecs, independent byte fixtures,
   strict malformed/unknown handling and real-product offline regressions.
2. System-2 property procedure planning, independently specified simulator state,
   whole-mutation recovery design and executable offline acceptance where proven.
3. System-B existing sequencer integration with verified product/image fixtures
   and the same recovery and negative public-route boundaries.
4. Remaining System1/System300/System7/coupler/RF profiles: implement exact proven
   pure codecs/plans; record source gaps individually rather than borrowing another
   family's semantics. Secure/PL110/Easy/USB are separate cross-cutting contracts,
   not ordinary S-mode masks or implicit permission to use vendor executables.
5. Review, deterministic tests/build, compatible caller integration and relevant
   corpus tests; document residual boundaries and publish reviewed packages.

This ordering is not a promise that all listed families have sufficient source
material. Do not mark a path implemented merely because its metadata is parsed.

## Invariants

- Core codecs do not depend on product XML, storage, networking or UI.
- ProductDB adapters preserve source order, pointers and unsupported constructs.
- Existing simulator/management transport is reused; expected values must not be
  generated from the production codec/plan being tested.
- Any new executable hardware path requires a separate demonstrated safety and
  complete recovery contract, not a verification badge bypass.
- No device access, vendor approval, manufacturer contact, proprietary DLL
  execution or full-spec/corpus publication is needed or authorised here.
- Private real-product fixtures are opt-in and missing inputs are explicit skips.
- Source/release/running-build availability and field evidence stay distinct.

## Package status

- Discovery: direct-PDF scope audit complete; selected shared root preserved.
- P1: System-2 metadata projection, independent legacy table and absolute-property
  codecs — integrated and published
  as `ad5f0e49`; twelve-stage merged gate and isolated corpus152/152 pass.
  [Contract and residual scope](offline-download-codecs.md).
- P2 onward: not implemented; dependencies above still need their own proof.

Status is deliberately per capability; issue #22 and broader KL-007 remain open.
