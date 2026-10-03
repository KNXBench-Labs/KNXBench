# ADR 0063: Parameter scopes preserve the full evaluation identity

Date: 2026-10-03
Status: Proposed — bounded backend contract public-verified; integrated gates/publication pending.
Session: 4, AR07

## Context and evidence

`ModuleScope::node_chain()` and `ScopeKey` in
`crates/knx-productdb/src/dynamic/evaluate.rs` already use the complete
outermost-first node chain. `assemble_parameter_panel` in
`apps/knx-server/src/domain.rs` groups sections by that same chain.
`module_scope_dto` discards the ancestors, exposing only the innermost node,
optional module ID and definition ID. Nameless nested instances of one definition
can consequently have identical HTTP scopes despite belonging to different
sections. The existing named-inner HTTP regression proves section separation,
not preservation of that identity on the wire.

This is a repository projection contract, not a new KNX format interpretation.
The source of truth is the existing evaluator and its bounded synthetic nested
fixtures. No primary KNX source establishes our node row numbers as ETS IDs or
persistent device identities. Nested manufacturer interoperability remains
sample-gated; the published historical corpus measurement found no nesting.

## Decision

Add `nodeChain: number[]` to each non-null parameter-section and diagnostic
scope, populated solely by the existing `ModuleScope::node_chain()` accessor.
Its ordering is outermost first, ending in the existing `moduleNode`. It is
nonempty for a module scope; a top-level program scope remains JSON `null`.
Keep `moduleNode`, `moduleId` and `moduleDefId` unchanged for older consumers.
Do not serialize argument values or a recursive copy of the entire scope.

The chain identifies an evaluation path within the response's program and
product-data snapshot. It is not a durable ID across package reinstallation,
a project module-instance ID, a secret, a write target or authorization. Existing
write authority, duplicate-ID refusal, retained sources and storage schemas
remain unchanged. No new domain model, migration or second expansion algorithm.

The UI owner must adopt the additive field in its manual API interface and
compare complete chains when matching section diagnostics. Older servers lack
this field; compatibility fallback must be deliberate and tested by that owner,
not silently claimed safe for nested nameless scopes. The active U17 Web lock
is untouched. Delivery of the backend field is not UI misattribution closure.

## Alternatives

- Keep only the innermost scope: preserves the demonstrated identity ambiguity.
- Use module ID as the identity: nameless and duplicate IDs cannot distinguish
  scopes; the evaluator explicitly does not use those IDs as identity.
- Invent qualified ETS IDs or persist row-number chains: changes unsupported
  manufacturer semantics and introduces unstable durable identity.
- Send full recursive scopes including argument values: unnecessary data and
  coupling; a bounded chain already supplies the canonical identity.

## Acceptance and remaining boundary

Public synthetic HTTP regressions must cover nameless nested paths whose legacy
scope fields are identical, complete chain equality with core evaluation, and
one-to-one diagnostic association under the new contract. Supplement with
repeated named IDs, three-level ancestry, single-level and top-level shapes,
deterministic repeated reads and read-only/atomic refusal. Reverting or clipping
the chain must compile and fail the behavioral tests. Preserve original product
bytes, nonempty project data, and every existing write-authority rule.

Full source review, applicable integration gates, exact source restoration and
publication readback remain required. No real nested product, new UI consumer,
ETS parity, complete AR07 or Alpha acceptance follows from synthetic success.
