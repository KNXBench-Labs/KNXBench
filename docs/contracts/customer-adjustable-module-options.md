# Customer-adjustable module-local Options: bounded admission extension

Status: **bounded contract for PRG-01-N3**, 20 November 2026. Extends
[customer-adjustable-write-admission.md](customer-adjustable-write-admission.md)
by one closed context class; every other clause of that contract stands.
[ADR-0181](../adr/0181-module-local-options-customer-adjustable.md) records the
decision; the [primary-source investigation](../research/2026-10-30-customer-adjustable-write-admission.md)
remains the rationale base. This contract implements neither ETS
customer-write rights nor any preservation semantics. It adds no device,
family or hardware compatibility claim.

## What changes

For `CustomerAdjustableParameters`, the admitted scope is now:

1. Own root `Static/Options` (unchanged, context class `root_options`).
2. `Options` as a **direct child of an explicitly entered module
   definition's own `Static`** — `ModuleDefs/ModuleDef/Static/Options`,
   including both module anchors (first `ModuleDefs` directly beneath the
   application or beneath its own `Static`) and nested admitted
   `ModuleDefs/ModuleDef` edges beneath a `ModuleDef` or its own `Static`
   (context class `module_options`).

Parser reach: unchanged. `ModuleDefs` directly beneath the application program
or directly inside the root `Static` is traversed; a `Static` nested inside the
root `Static` is not entered and nothing beneath it is observed.

## What stays excluded

Module-local Options behind a wrapper element, under the module's
`Dynamic`/`Extension`, under an extra nested `Static` inside the module's own
`Static`, and every foreign-prefixed attribute
(`x:CustomerAdjustableParameters`) remain excluded matcher limits. An
`Options`, `Parameters` or `ModuleDefs` inside a `Static` nested in the root
`Static` is not observed at all. These limits are not semantic-safety claims.

## Semantics

Identical to the root option: unqualified raw key match via
`Attrs::evidence_value`; every present value (`false`, `0`, `1`, `true`,
`Sync`, whitespace, empty, arbitrary text) refuses equally; no positive token
class, Boolean conversion, trimming, status exemption or inferred right.
Counts-only public diagnostics gain a `module Options` counter; raw tokens,
IDs and paths stay private. Read-only preparation, comparison/adoption and
all existing diagnostics are unchanged; write chokepoints and precedence
(kind → plug-in → customer-adjustable → image → plan) are unchanged.
