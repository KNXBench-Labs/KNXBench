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
- P2: explicit legacy indexed-association codec and evaluator-backed System2
  offline source-image assembly integrated and published as `79156a5d`.
  Fourteen-stage branch and fifteen-stage merged acceptance pass, including
  isolated corpus152/152 in33targets; exact main publication equality verified.
  [Source-image contract](offline-download-system2-image.md) separates successful
  synthetic assembly from the actual Siemens multi-octet source refusal.
- P3a: normalized offline property-plan validation and concrete-simulator
  execution integrated and published as `7632fb1a`. Fifteen-stage branch and
  sixteen-stage merged acceptance pass, including isolated corpus152/152 in33
  targets; exact main publication equality verified.
  This is not yet a ProductDB source translator or durable recovery. See the
  [bounded simulation contract](offline-property-plan-simulation.md).
- P3b research: [source-semantics investigation](../research/2026-10-16-system2-source-semantics.md)
  directly checked the original four-octet table-reference definition against
  two-octet master-resource declarations. The bounded49procedure/264segment
  census does not establish flag polarity or missing encoding/recovery semantics.
  No production mapping is guessed; this individual translation slice remains
  evidence-blocked, not hardware-blocked. Continue independent System-B and
  remaining-family work under the owner scope.
- P4a: [pure System-B Format-1 object-table codec](offline-systemb-object-table.md)
  integrated and published as `adb52990`, with ten passing byte/boundary tests
  and a discriminating omitted-flag mutant. All 17 branch and 18 actual merged
  stages pass (Rust 4649/0, Vitest 3626/0, isolated corpus152/152 in33targets).
  Local/tracking/live remote main equality was verified after publication.
  [Three real-source resolver results](../research/2026-10-16-systemb-offline-prerequisites.md)
  remain partial/non-executable; relative images, scope/substitution mapping and
  complete recovery are not implemented by the codec.
- P4b: [relative source view and absolute-image refusal](offline-relative-source-segments.md)
  integrated and published as `4bfc3780`, calibrated against three original
  XML/base64 oracles.
  This retains source metadata/bytes; generated tables, relative-image assembly,
  source scopes/substitution and recovery remain unfinished. All twelve corrected
  preflights,18branch and19actual merged stages pass, including the explicit
  three-source regression and isolated corpus152/152 in33targets. Remote
  equality verified; no new family admission.
- P3 onward: source-to-property mapping and whole-mutation recovery remain
  unimplemented; their dependencies above still need their own proof.

Status is deliberately per capability; issue #22 and broader KL-007 remain open.
