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

## P4b production evaluation and source-reader follow-up [V]

A second network-isolated Release run on the same three applications in one
package used the existing production importer, `resolve_values`,
`load_program_trees`, `evaluate`, `load_program_code` and `build_download_image`.
It activated 186 parameter references and 12 object references across the three
applications. All 54 diagnostics were `NoBranchMatched`, classified by the
existing evaluator as not hiding references. This is a measured existing
behaviour, not independent proof of every activation decision.

Before the P4b reader change, all three code reads succeeded with zero absolute
segments, and every image attempt refused at `ComObjectTable: no CodeSegment`.
That is an observed absolute-builder boundary, not a malformed-source finding
or proof that relative images cannot be implemented. The declared generated
tables have no explicit absolute CodeSegment/Offset placement in these witnesses.

The new [relative source view](../contracts/offline-relative-source-segments.md)
retains each relative segment's metadata and Data/Mask separately from absolute
addresses. A second calibrated run matched all three source records to an
independent original-XML/strict-base64 oracle; source packages and the product
database remained unchanged. Both absolute-image entry points now refuse
relative source data explicitly rather than silently omitting it from mixed
input. Generated-table construction, relative-image assembly, parameter encoding,
source merge/substitution and recovery remain distinct unfinished work.
No source-view result grants executable readiness or device support.

## Direct original-PDF inspection [D]

Original **Configuration Procedures 03_05_03 v02.01.01** pages were rendered
and inspected, not inferred from source node names:

- §3.5.1.3, p. 40: System-B memory layout ordering is a recommendation, not the
  same contract as normative download ordering. Avoid imposing one on the other.
- §3.5.4, p. 44: relative allocation, use of the table reference, memory loading
  and optional memory-control-table handling are separate client responsibilities.
  A relative parameter segment alone is not a complete plan.
- **Corrected by P4c original-body reinspection:** §3.9.3 begins on p71 and
  explicitly concerns mask57B0h, not an unqualified System-B modern XML mapping.
  Its merge points and example p75 require application-specific data and current
  generated-table sizes; raw source reconstruction does not perform substitution.
- §3.9.3.2 p72 defines relative image-memory reads; the continuation p73 lists
  property read/write/compare controls. It does **not** directly define the
  modern `LdCtrlLoadImageProp` node as previously attributed here. That name
  occurs in partial-parameter examples (§3.9.3.6 p79, §3.9.3.7 p82) for
  PID_TABLE_REFERENCE, not as proof of every modern PID_MCB_TABLE capture or
  XML `AppliesTo` token-to-scope mapping. Original source nodes remain preserved.
- §3.9.3.3 p73 gives fixed table/AP object indices1–5 for the specified57B0
  procedure; this does not by itself admit the selected07B0source applications.

Original **Resources 03_05_01 v01.10.01** §§4.18.6–4.18.6.2.4.1.4,
printed/PDF pp. 269–272, independently specifies the System-B Type-7 / Format-1
object table. Its two-octet descriptors and one-based indices differ from the
existing Easy-3 and legacy Type-1/Type-2 grammars. The independent
[pure codec contract](../contracts/offline-systemb-object-table.md) records that
implemented prerequisite without assigning it to an unverified product layout.

This is a bounded chapter-body inspection, not a complete family/profile audit.
Raw product XML, exact source identifiers and licensed page renders remain private.

## P4c independently specified link grammar [D / V]

Original Resources §4.16.8.2.4 Table78 pp238–239 and §4.17.5.2.5 Tables81–82
pp249–250 specify a different address/association grammar from the existing
Easy-3 and indexed legacy codecs: word counts, no individual-address slot,
one-based TSAP/ASAP, explicit narrow/wide association formats and first-match
sending. The new [pure link-table codec](../contracts/offline-systemb-link-tables.md)
implements those explicit bytes without inferring a product layout or public
admission. Twelve literal/boundary tests and a rejected sender-order mutant
pass; all18branch and19actual merged stages pass and the codec is published
as `c74038cd`. This does not establish the product integration contracts below.

The original57B0procedure p75 uses current generated-table image sizes, while
retained modern07B0controls include source literals whose substitution cannot
be derived merely from their integer type or similar names. Keep those controls
uninterpreted until their source/profile transform is proven. This is a specific
integration gap, not lack of access to a physical device.

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
