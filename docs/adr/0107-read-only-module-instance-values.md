# ADR 0107: Read stored module-instance values without interpreting RepeatIndex

Date: 2026-10-10
Status: Accepted (owner decision; Repeat evaluation remains research-bound)

## Context and evidence

[KNX Project Schema23 v01.00.00, §1.2.5.18](https://support.knx.org/hc/en-us/article_attachments/17389755651474)
describes a shortened ModuleInstance Id and a RepeatIndex list: each entry
pairs XmlOrder with a repeat counter, and nested repeats require an entry per
level. Its Id description refers to RepeatIndex, but does not give a complete
application-program iteration-to-identity algorithm for list-valued indices,
subsets or omitted controlling values. The nominal reference is not treated
as proof of a numeric MI-to-counter equality.

The module-qualified parameter-id decomposition already recorded in
[product research §4.4](../research/product-database.md#44-modulemoduledef-expansion-semantics--r4-spike-session-4-2026-09-11)
and ADR-0013 distinguishes stored MI components from RepeatIndex text.
Authorized private evidence motivates the regression but is not a normative
rule. In particular, any correlation between an MI component and a counter
remains a hypothesis. No private identifiers or measurements enter fixtures.

## Decision

The owner chose instance-keyed read-only values, not Repeat expansion.

- Add a separate ValueMap key `(module_id, instance_ets_id, declared_ref_id)`.
  The instance id is verbatim source identity; do not parse RepeatIndex to
  generate it, renumber sparse instances, or synthesize missing iterations.
- Validate identity against exactly one owning imported instance and exactly
  one program-side Module declaration and its own ModuleDef parameter refs.
  The new map requires the recognized ModuleDef/ref namespace association.
  Existing alias-tolerant single-instance evaluation remains separate; this
  extra association check must not change its prior read/write behavior.
  Walking declarations is not evaluating a skipped Repeat or hidden branch.
- Read this map explicitly. No fallback to another instance, the unscoped
  program default, or a controller default is allowed for this read.
- Preserve first-row/duplicate diagnostics and raw stale rows. Duplicate or
  malformed instance authority cannot earn a validated instance value.
- The existing single-authority scoped evaluation/write behavior remains;
  repeated instances never feed one chosen sibling into that evaluator.
- Repeated and skipped module values are inspectable stored evidence, with
  activation explicitly not evaluated. This is not an editable expanded
  Dynamic tree, a download value, or evidence of ETS activation parity.
- One-pair/two-pair text, subsets, missing counters and apparent out-of-range
  counters are retained unchanged. Since counter semantics are not accepted,
  no numeric N-check or count-derived refusal is implemented. Malformed
  *identity* is separately refused for validated reads.

## Consequences and boundaries

No core/native/product storage schema, new runtime dependency or protocol change. The
map is derived per device, preventing cross-device source-id collisions.
Synthetic fixtures use nested Channel/ParameterBlock/choose/when/Repeat
wrappers, not a root-only Module shortcut. An optional private comparison
must refuse missing input and assert booleans only; complete original import
acceptance waits for the independent import-integrity package.

KL-68's value association can improve without lifting PDB-01's Repeat
expansion boundary. A future expansion needs documented iteration matching,
subset and absent-controller behavior, an explicit new owner decision and
its own regression/activation/write-authority evidence. Until then, the
repeat counter does not get promoted from witness to engineer.
