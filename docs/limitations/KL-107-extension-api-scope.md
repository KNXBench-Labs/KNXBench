# Extension API scope

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-107)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 107. There is no plugin API — a third party cannot add a format, a protocol, a report template or a UI panel without forking

**Limitation.** KNXBench loads no extensions of any kind: no dynamically
loaded library, no WebAssembly module, no script, no out-of-process plugin
protocol. A third party who wants a new import or export format, a new bus
protocol adapter, a different documentation template, or an extra panel or
command in the user interface has exactly one route — fork the repository,
add a workspace crate, and rebuild. Nothing can be dropped into a directory
and picked up at start-up.

Four things *are* extensible without compiling anything, and they are the
supported story rather than a consolation prize: **language packs** (a JSON
file, [LANGUAGE_PACKS.md](../LANGUAGE_PACKS.md)), **product databases**
(imported at runtime, [ADR-0005](../adr/0005-separate-product-database.md)),
**group-address CSV** ([IMPORT_EXPORT.md §11](../IMPORT_EXPORT.md)) and the
headless **`knx` CLI**, which anything that can run a process can drive.
What cannot be added this way is behaviour.

`knx-server`'s `/api/*` routes are **not** a public interface either. They
exist for this application's own frontend, they change whenever it needs them
to, and they are documented nowhere as a contract. Code written against them
will break without notice.

**Cause.** Decided in [ADR-0025](../adr/0025-extension-is-data-not-code.md) on
the evidence in [PLUGIN_FEASIBILITY.md](../PLUGIN_FEASIBILITY.md), and the cause
is that there is nothing to expose. The whole 16-crate workspace contains
eight traits; six are single-implementer or test seams, one is private, and
the two that are dynamically dispatched say in their own doc comments that
their second implementer is a test fake. There is no importer, exporter or
template trait anywhere, and every candidate seam has exactly one
implementation — so a plugin interface would have to be generalised from a
sample of one, which is the speculative abstraction CLAUDE.md's rules
forbid. Three further facts close the route independently: all 16 manifests
are `publish = false`, so nothing can depend on `knx-core`;
`xtask check-layering` is a hand-written allowlist of crate names built from
`cargo metadata`, so a third-party crate is invisible to the one mechanical
architecture gate this project has; and `knx-core` depends on `chrono` alone,
with no `serde`, so `Command` has no serialisable form for any boundary to
carry.

Data integrity is the part that would be hardest to fix rather than merely
tedious. `Project`'s six fields are all `pub`
(`crates/knx-core/src/project.rs:181-186`), so the rule that every mutation goes
through `Command::apply` — with its validation, its typed errors and its
inverse for undo — is held by review, not by the type system. `Layer`
([ADR-0004](../adr/0004-provenance-model.md)) has five variants and none of them
means "a plugin did this", so a plugin write would either forge `UserEdit`
provenance, telling the user they made a change they did not, or force a
sixth variant through the persistence schema and its migration chain. And a
plugin that rewrote a project without carrying the opaque passthrough store
([ADR-0006](../adr/0006-opaque-passthrough-store.md)) would break roundtrip
fidelity **silently**, because no validator can catch an absence.

The AGPL licence is deliberately *not* the cause. Copyleft hosts sustain
large third-party ecosystems elsewhere; what AGPL rules out is *proprietary*
in-process addons, and even that with decreasing certainty as the boundary
moves from a dynamically linked library to WebAssembly to a separate process.
See PLUGIN_FEASIBILITY.md §3, which states the licence mechanics and is
explicit that it is not legal advice.

**Consequence.** An extender needs a Rust toolchain, a build, and the
discipline of rebasing onto upstream. That is a real barrier and this entry
does not pretend otherwise. There is also no answer at all for someone who
wants a closed-source addon — though under the licence analysis above that
answer would have been "no" or "unsettled" for every in-process mechanism
anyway, so less is foreclosed than it looks. In exchange, nothing a third
party ships can corrupt a project file, crash the application, forge
provenance, or silently lose the data the opaque store exists to preserve.

**Lifted when.** Four conditions, all of which are worth reaching for other
reasons: the `Command` layer is complete and has a serialisable form — the
same blocker [ROADMAP.md](../ROADMAP.md) already records for MCP capabilities
and for the in-app LLM surface; §22 (no authentication on `knx-server`) is
answered; §63 (one shared project, one shared undo stack, no conflict
detection) is answered; and at least two concrete third-party extensions
exist that the four data surfaces above genuinely cannot express. If a code
seam is then wanted, ADR-0025's recommended shape is an out-of-process helper
over a documented protocol. Before any of that, the cheap falsifying test is
to write a *second* implementation of one seam as an ordinary workspace crate
and see whether a shared trait falls out of it — if one does, this entry and
its ADR are wrong and should be revised.
<!-- END retained catalogue record -->
