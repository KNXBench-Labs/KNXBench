# Modules without identifiers

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-069)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 69. A `Module` with no `@Id` cannot be matched to a project instance

**Limitation.** `Module/@Id` is an optional XML attribute
(`ModuleScope::module_id: Option<String>`). When a program instantiates a
`Module` with no `@Id`, that instantiation's stored per-channel values are
unreachable — there is nothing to decompose an `MI-` authority against —
so the section evaluates against the program default and stays read-only,
with a diagnostic: "A module instance has no identifier and cannot be
matched to stored values." (`Diagnostic::ModuleWithoutId`,
`apps/knx-server/src/domain.rs:1827-1829`).

**Cause.** Design D37: `Module/@Id` is present on 102/102 `<Module>`
elements across the corpus's seven module-bearing program files (E2), but
nothing in the Standard extraction guarantees that for a package not yet
seen. The diagnostic is emitted per instantiation, unconditionally — a
`Module` this slice cannot name is worth reporting even when no stored
value would have applied to it.

**Impact.** Unexercised in the corpus today (0/102). If it ever fires,
that `Module`'s channel behaves exactly as every module-scoped channel did
before T18 slice 4: displayed where a value happens to already resolve,
never editable.

**Task 12 (2026-09-14): unaffected, and marginally better reported.** An
`@Id`-less `Module` still cannot be matched to a project instance — an
argument binding names an `Argument`, not the `Module` carrying it, so it
supplies no identity. It does now produce a *different-looking* section:
its labels are substituted like any other instantiation's, so a nameless
module is at least distinguishable on screen from its siblings even while
remaining unmatchable and read-only. Nothing about the matching rule
changed.

**Lifted when.** Never by invention — a synthesised id would be a
fabricated identifier that looks like project data and matches nothing
real (D37's own reasoning). Only the manufacturer's own application
program, carrying its own `@Id`, lifts this.
<!-- END retained catalogue record -->
