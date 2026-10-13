# Manufacturer resolution gaps

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-012)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 12. Manufacturer data resolution — one of three gaps closed (2026-09-20)

**Lifted (Session 4) for communication-object defaults.** `ProductRefId`
and `Hardware2ProgramRefId` now resolve: the shared product database
([ADR-0005](../adr/0005-separate-product-database.md),
[ADR-0011](../adr/0011-product-database-storage.md)) ingests `<M-xxxx>/*`
once, keyed by content hash, and `knx_productdb::enrich` fills a
communication object's `text`, `description`, `dpt`, all six flags and
`size`
from the application program wherever the instance itself left the slot
`Absent` (IMPORT_EXPORT §10). `ComObjectInstance` values now carry
`Program`/`ProgramRef` in addition to `Instance` where the source project
did not itself state a value.

Three gaps were named after that lift. Re-measured against the reference
project (goal-completion Task 4, 2026-09-20; every figure below is exact,
reproduced by `crates/knx-app/tests/enrichment_gap_measurement.rs`): one is
now closed, one was already smaller than its own headline made it sound
and is tracked precisely elsewhere, and one is deferred with its blocker
named plainly.

**Closed: a program value behind an `Empty` instance slot is no longer
invisible ([ADR-0027](../adr/0027-program-defaults-side-table.md)).** 497 of
the reference project's 907 `ComObjectInstanceRef` elements carry
`DatapointType=""` — present, explicitly cleared, not unstated — spanning
23 of 36 devices; 82 more carry an empty `VisibleDescription`, and `text`
is never empty (a program always states one). Enrichment still never
writes into an `Empty` slot (ADR-0012's export-fidelity rule is unchanged
and re-verified by its own tests), but the program's own value behind that
slot — 122 of the 497 dpt cases and all 82 of the description cases
resolve to exactly one value; the remaining 375 dpt cases genuinely have
nothing behind them either — now lands in `Devices::program_defaults`, a
side table `knx-store` persists (`com_object_program_default`, schema 9)
and the exporter never reads. `ProgramDefaults` and its map replace
ADR-0012's originally-rejected "extend `Override<T>` into a layer stack"
alternative with an additive side table instead — see ADR-0027 for why.
**Current display scope:** the projection exposes a `program_dpt` fallback
beside an unstated/cleared instance DPT, and the Inspector reads it without
rewriting the instance override. This is not a generic UI inventory of every
stored program-default field.

**Narrower than it read: parameter interpretation.** The paragraph this
section used to carry duplicated [§3](../KNOWN_LIMITATIONS.md#3-device-parameters-are-preserved-but-not-interpreted),
which is the authoritative, continuously-updated account and had already
moved past what this section still said. In short, as of §3's latest
entry: top-level parameter fields are read and writable (T18 slice 3);
module-scoped fields are also now read and writable, but only when their
section has exactly one authoritative `ModuleInstance` (T18 slice 4) — the
residue is three named, individually-tracked cases
([§68](../KNOWN_LIMITATIONS.md#68-repeated-module-instantiation-is-refused-not-supported),
[§69](../KNOWN_LIMITATIONS.md#69-a-module-with-no-id-cannot-be-matched-to-a-project-instance),
[§71](../KNOWN_LIMITATIONS.md#71-a-project-imported-before-store-schema-6-has-no-module-instance-ids-to-write-with)),
not one undifferentiated "module-scoped editing" gap. Nested modules,
module arguments (text substitution only) and `AllocatorRef` reporting
have each moved since this section was last written too — §3 is where
all of that lives now; this section stops duplicating it and points there
instead, so the two cannot drift apart silently again.

**Deferred: an ambiguous, space-separated `DatapointType` list still fills
nothing.** `ComObjectRef/@DatapointType` can hold several acceptable
alternatives (RESEARCH §4.2, e.g. `"DPST-9-21 DPST-9-1"`). Enrichment
refuses to guess between them; it records `EnrichmentIssue::AmbiguousDpt`
and leaves the slot as it was — and must keep doing exactly that, per
CLAUDE.md's data-integrity rule: picking one alternative silently would be
invented compatibility presented as data. Re-measured: 107 issues raised
in total, but 85 of those land on a slot the instance already stated its
own value for (informational noise — nothing was ever going to be filled
there regardless), 0 land on `Empty`, and only 22 land on a genuinely
`Absent` slot that actually stays unfilled because of the ambiguity,
spread across 11 devices; 51 distinct `ComObjectRef`s carry an ambiguous
list in total. **Blocker.** The brief for this task floated one way to
close it — surface `AmbiguousDpt` to the user as a choice, rather than
resolve it automatically — but no dedicated import-ambiguity alternative-selection workflow exists: `EnrichmentReport` reaches the
CLI's summary line and, for device creation only, `apps/knx-server`'s
`CreationDiagnostic` ([§35](../KNOWN_LIMITATIONS.md#35-device-creation-enrichmentissues-are-silently-dropped--resolved-2026-09-10)),
but a project-level *import*'s `EnrichmentReport` is not surfaced to
`apps/knx-web` at all, and there is no domain concept of a user-recorded
DPT choice to write it into even if it were. Building that (a new
surfacing path, a place to store the choice, a UI) is a design of its own
scope — deliberately not absorbed into this task, the same call §3 makes
for module-scoped editing's own residue. Given the narrow measured impact
(22 slots, 11 devices, of 907 communication objects), it did not rank
ahead of gap 2. *Lifted when* that design exists; re-guessing from a
linked group address's own datapoint type, floated in an earlier draft of
this section, was not re-considered this session and is not assumed to be
the answer.

**Impact on the measured reference scope.** Native storage retains the
manufacturer source bytes. This is not ETS export or a universal successful
import promise. Communication-object defaults resolve where the
instance did not override them, and now also where it explicitly cleared
them and a program value exists — kept separately in the model rather than
overwriting the cleared instance slot. A top-level or single-instance module-scoped
parameter value can be read and written from the parameter editor. An
ambiguous DPT list is visible in the product database and in
`EnrichmentReport`, but still fills nothing, and nothing in the current
stack offers that dedicated ambiguity workflow. The Inspector's ordinary
typed DPT edit is already available as an explicit user override; it does not
evaluate the manufacturer's whole alternatives list or make a hardware claim.

**Lifted when.** Gap 2 is closed. For the parameter-interpretation
residue, see §3's own "Lifted when". For the ambiguous-DPT gap, see the
blocker above.
<!-- END retained catalogue record -->
