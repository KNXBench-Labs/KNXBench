# ADR 0177: The accepted WP-07 mapping is the module-instance authority

Date: 2026-11-17
Status: Proposed
Session: WP-07 (unblocked by owner decision 2026-11-17)

## Context

WP-07 was evidence-blocked until an independently counted witness existed. On
17 November 2026 the owner accepted the counted witness
([docs/research/2026-11-17-wp07-module-mapping-witness.md](../research/2026-11-17-wp07-module-mapping-witness.md))
as the first-deliverable evidence and lifted the block
([OWN-01](../open-work/OWN-01-owner-decisions.md#decision-recorded--17-november-2026)).
The witness counts, over eight ETS exports of one fictional project plus two
modular corpus projects: 149 module instances, 292/292 numeric argument values
matching `Allocator/@Start + Argument/@Allocates·(k−1)`, RepeatIndex positions
149/149, and 23 697/23 697 corpus-wide `Memory/@BaseOffset` /
`ComObject/@BaseNumber` references resolving to an argument of the same
`ModuleDef`. The KNX specification corpus (179 volumes) is negative for these
names, so the contract rests on observed ETS exports, not on the specification.
The download-image arithmetic `A-2 + local Number` on the wire and all bus
behavior stay out of scope.

Today the code reads the `_MI-<k>` suffix only ad hoc: `device_evaluation.rs`'s
`resolve_mi_authority` re-derives it by string prefix, and
`com_object_activation.rs`/`parameter_objects.rs` re-derive the
"ends with `_<RefId>`" match by hand. Three copies of one rule is how drift
starts.

## Decision

1. **One authority, one home.** `knx_core::ModuleInstance::instance_counter()`
   is the single reader of the instance index `k`: the all-ASCII digits of
   `instance_ets_id`'s trailing `_MI-<k>` suffix, `None` when the id does not
   decompose as `<source.ets_id>_MI-<digits>`. Never guessed, never defaulted.
   The stride rule `Start + Allocates·(k−1)` is applied wherever a numeric
   argument value is *reconstructed*, and only from stored catalogue
   declarations (`module_def_argument.allocates` plus the allocator range);
   it is never used to overwrite a stored project value.
2. **Ownership matching stays structural.** A program-side `Module/@Id` owns
   an imported instance when it ends with `_<instance.source.ets_id>`; exactly
   one match is a resolved owner, zero is `NoMatch`, more than one is
   `Ambiguous`. This rule moves to one shared function keyed on
   `instance_counter()` rather than three hand-rolled copies; the existing
   `MiAuthority` semantics are unchanged.
3. **Ambiguity refuses the whole edit.** Where a mapping would be ambiguous —
   several imported instances matching one module id, a malformed `MI-`
   suffix, or a reconstructed object number colliding with an existing
   instance — the parameter write / object reconciliation refuses as a whole
   without allocating ids or changing native history. This is the existing
   refusal in `parameter_objects.rs`, now expressed through the shared
   authority.
4. **`repeat_index` stays opaque** (ADR-0108). The witness counts its shape
   149/149 but does not license parsing it; no Repeat expansion is added.
5. **No wire or bus claim.** Nothing here asserts the download-image
   arithmetic, device readback or bus behavior; those remain out of scope.

## Alternatives considered

- **Keep three local copies of the suffix rule.** Rejected: the witness makes
  `k` a counted fact, so it deserves one tested implementation, not three.
- **Parse `RepeatIndex` to derive `k`.** Rejected: ADR-0108 already decided
  against it, and the witness shows `k` is carried directly by the `MI-`
  suffix (index reuse after delete-and-re-add follows the suffix, not a
  global counter).
- **Reconstruct argument values from the stride rule on import.** Rejected:
  imports store ETS's own argument values verbatim; reconstruction is only a
  cross-check, never a source of truth that overwrites stored data.

## Consequences

`knx-core` gains one small pure function plus tests; `knx-productdb` and
`knx-server` call it instead of re-deriving the suffix. Evaluation, parameter
writes and object reconciliation keep their existing refusal semantics, now
expressed through the one authority. WP-37's module-scoped-parameters item can
build on `instance_counter()`. No storage schema, protocol or UI change; no
bus/device/write/release authority is granted or implied.
