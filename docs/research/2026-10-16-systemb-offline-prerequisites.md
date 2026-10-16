# System-B offline prerequisites — 16 October 2026

## Bounded real-source baseline [V]

A deterministic first-three discovery examined direct product XML members in the
private local corpus. It visited **13 of 105 inventoried packages**, examined
47 prefixes (385,024 bytes), and retained three complete `MV-07B0`
`MergedProcedure` application sources (4,969,370 bytes). All three belong to
**one** package. These are selected witnesses, not three independent packages,
a whole-corpus census or a claim of product support. Four synthetic discovery
calibrations passed; original inputs were rechecked by digest.

The actual existing production `resolve_package_ap1` was then exercised on the
three selected applications through the production package importer and an
isolated temporary product database. The opt-in Release test ran offline in a
network namespace. It asserted source binding, no database changes during
resolution, no executable result, and unchanged original package digests.
The runtime input mode/content manifests matched before and after execution.
The temporary test was saved privately and removed from the worktree afterward;
it is not a new production API or a committed private fixture.

Observed results, aggregated from the actual three retained resolver outputs:

- Three `partial` results, zero executable results.
- 21 `uninterpreted-step` diagnostics, seven per application. The exact
  source-local reasons are the `AppliesTo` attributes on otherwise known controls
  and the unsupported `LdCtrlLoadImageProp` nodes; the original nodes remain
  preserved. This is not evidence of 21 unknown KNX wire services.
- Across all results: 3 connect, 12 load, 12 load-completed, 12 image-property,
  6 map-error, 15 relative-allocation, 3 restart, 15 unload, 3 property-write
  and 12 relative-memory-write steps.
- Each selected application declares one relative segment. This does not locate
  or define the separate generated group-object/address/association tables.

No resolver readiness or public-route admission was promoted. The baseline
succeeded as a **measurement of existing partial behaviour**, not as a download.

## Direct original-PDF inspection [D]

Original **Configuration Procedures 03_05_03 v02.01.01** pages were rendered
and inspected, not inferred from source node names:

- §3.5.1.3, p. 40: System-B memory layout ordering is a recommendation, not the
  same contract as normative download ordering. Avoid imposing one on the other.
- §3.5.4, p. 44: relative allocation, use of the table reference, memory loading
  and optional memory-control-table handling are separate client responsibilities.
  A relative parameter segment alone is not a complete plan.
- §3.9.3.1.2, p. 72: application and master control sequences are merged and
  source image placeholders are substituted from context; raw source expansion
  is not equivalent to carrying out those substitutions.
- §3.9.3.2, p. 73: the image-property control obtains a property into the tool's
  image; it is not simply a property-write synonym. These definitions do not by
  themselves establish the modern XML `AppliesTo` token-to-scope mapping.

Original **Resources 03_05_01 v01.10.01** §§4.18.6–4.18.6.2.4.1.4,
printed/PDF pp. 269–272, independently specifies the System-B Type-7 / Format-1
object table. Its two-octet descriptors and one-based indices differ from the
existing Easy-3 and legacy Type-1/Type-2 grammars. The independent
[pure codec contract](../contracts/offline-systemb-object-table.md) records that
implemented prerequisite without assigning it to an unverified product layout.

This is a bounded chapter-body inspection, not a complete family/profile audit.
Raw product XML, exact source identifiers and licensed page renders remain private.

## Remaining integration contracts [V / unresolved]

1. A source-bound relative image builder must reuse the existing product parameter
   and Dynamic evaluation, locate generated tables with explicit format evidence,
   and preserve unsupported fields. The current public image route is not this
   builder; its absolute-memory admission must not be widened to bypass the gap.
2. Source scope selection (`AppliesTo`), image-property capture/substitution,
   LSM/resource/object mapping, identity, allocation flags and restart must be
   translated explicitly. An AP1 reconstruction is not an executable sequence.
3. Match the resulting complete image/plan to the **existing** System-B sequencer
   and stateful simulator, using independent expected byte/state vectors; do not
   write a competing generic executor merely to produce a green test.
4. Before any new public/hardware route, identity-bound durable recovery must
   cover direct properties, memory and indirect setup/allocation effects. A
   memory-only snapshot or mutation inventory is insufficient.

Lack of an owned device is not the blocker for pure codecs or offline simulation;
missing source/image/side-effect contracts remain explicit. No manufacturer
contact, vendor binary execution, bus operation, new compatibility assertion or
schema change belongs to this checkpoint. The broader
[offline expansion](../contracts/offline-download-expansion.md) remains active.
