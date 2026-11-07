# ADR 0160: A choose whose placed controller is inactive selects nothing

Date: 2026-11-07
Status: Accepted

## Context

The `Dynamic` evaluator walked every `choose` with its controller's value,
whether or not that controller was itself active. Push-button programs keep
several hidden "LED status" controllers per button, one shown per function,
each with its own `choose` setting the same union member. With the hidden
controllers at their defaults, two members of one parameter became active with
different values and the image was refused (`ConflictingValues`).

Two ETS 6.4 download images ([research note](../research/2026-11-07-ldctrlwriteprop-and-byte-order.md))
discriminate the readings: ETS wrote the visible controller's branch both when
the hidden one selected a smaller and a larger value. Neither "last wins" nor
"largest wins" explains both.

## Decision

A `choose` whose controlling `ParameterRef` is placed in the tree (some
`ParameterRefRef` names it) but not active in that scope activates nothing,
reports nothing and is not descended. `evaluate` runs in passes: each marks
the controllers the previous pass consulted and left inactive; it stops when
none is added. Skipping only removes activations, so the set only grows and
the passes end; all passes share one `MAX_EVALUATION_WORK`, and exhausting it
keeps the explicit budget diagnostic. A controller no `ParameterRefRef` names
is read as before: that case was not observed.

## Alternatives considered

- **Admit the later or larger member in the image builder.** Contradicted by
  one of the two images each.
- **Order-dependent single pass.** A controller placed after its `choose` in
  document order would decide differently from one placed before.
- **Treat unplaced controllers as inactive too.** Hidden constant
  controllers are common and unobserved; it would change many programs
  without evidence.

## Consequences

Activation now depends on controller visibility, so parameters, objects,
labels and the image follow ETS for these products. Programs without such a
pattern evaluate in one pass, unchanged. The corpus activation gates measure
any effect on other programs.
