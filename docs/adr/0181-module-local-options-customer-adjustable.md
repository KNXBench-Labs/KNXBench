# ADR 0181: Module-local Options joins the bounded customer-adjustable refusal

Date: 2026-11-20
Status: Proposed
Session: PRG-01-N3

## Context

ADR-0160/0161/0168/0170/0171 and the
[customer-adjustable contract](../contracts/customer-adjustable-write-admission.md)
admit uninterpreted `CustomerAdjustable` in root parameters, direct Union members
and explicitly traversed module parameter contexts, plus
`CustomerAdjustableParameters` only in the own root `Static/Options`.
The [primary-source investigation](../research/2026-10-30-customer-adjustable-write-admission.md)
records module-local Options as an explicitly excluded matcher limit — a
refusal-side gap, not a semantic claim. The first `ModuleDefs` may sit directly
beneath the application program or beneath its own `Static`; both anchors are
explicit. The app-level plugin-path test
`admitted_module_local_options_reach_the_public_writer_and_keep_reads_unchanged`
exercises the root-`Static`-anchored module shape
(`<Static><ModuleDefs>…</ModuleDefs></Static>`), which the existing `ModuleDefs`
arm of the static walk already reaches. No parser change is needed or made: a
`Static` nested inside the root `Static` stays skipped, and no real catalogue
sample in `docs/research/` shows that shape.

## Decision

Admit `CustomerAdjustableParameters` on an `Options` element that is a direct
child of an explicitly entered module definition's own `Static`
(`ModuleDefs/ModuleDef/Static/Options`, including nested admitted
`ModuleDefs/ModuleDef` edges and both module anchors). Count it in a new closed
context class `ModuleOptions` alongside `root_options`; every present value
refuses equally and the raw token stays private, exactly as for the root
option. The parser walk is otherwise unchanged: a `Static` nested inside the
root `Static` is not entered, so nothing beneath it (`Options`, `Parameters`,
`Code`, `ModuleDefs`, `Extension`) is observed. An `Options` behind a wrapper or
an extra `Static` inside a module stays excluded.

## Alternatives considered

- Leave module-local Options excluded: keeps the historical contract but leaves
  the simulator-verifiable plugin-path shape (`Static`-anchored `ModuleDefs`)
  unhandled, so the semantic RED stays red.
- Universal flag detection over all descendants: rejected by the contract —
  wrappers, `Dynamic`/`Extension` and foreign-prefixed attributes must not
  acquire the category, and no positive token class may be inferred.

## Consequences

The public writer refusal (`customer-adjustable-parameters`) now reports a
`module Options` count; read-only preparation, comparison and diagnostics are
unchanged. No ETS customer-write right, preservation algorithm, hardware or
status-dependent behavior is claimed; excluded contexts remain matcher limits,
not harmlessness proofs. Tests: unit (both anchors, nested module edges,
excluded descendants, foreign prefix, markerless-sibling), integration
(`knx-app` plugin-path RED→GREEN with unchanged read image), malformed input
keeps the existing XML error.
