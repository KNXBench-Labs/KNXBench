# Allocator safeguards and deferred enforcement

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-129)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 129. A stale id-allocator snapshot can duplicate ids, and saving then drops one entity

**Current reading.** The data-loss path is closed: phases 1–2 of ADR-0039
landed on 2026-09-27. A colliding id is refused, and no caller rewinds the
counters any more. AR02 also refuses allocator exhaustion. Structural
phases 3–5 were explicitly deferred past the alpha by the user on 2026-10-04;
they are not an unanswered activation request. See the pinned
[AR03 audit](https://github.com/KNXBench-Labs/KNXBench/blob/bca2d3336b96/docs/archive/alpha-0.1/ADR0039_ENFORCEMENT_AUDIT.md) at `e691bc13`.

**Historical limitation (before phases 1–2, at `7b64496`).**
`Command::SetIdAllocators` replaces the id counters
absolutely, and no `Create*` command refuses an id that is already in use.
A caller that snapshots the allocator, releases the project lock and applies
later can therefore lower the high-water mark and insert a second entity
with an existing id. `save_project` upserts by id (`ON CONFLICT(id) DO
UPDATE`), so on save one of the two entities silently disappears.

**Historical cause (at `7b64496`).** `Project`'s six fields are `pub`, so ARCHITECTURE §6's "every
mutation is a `Command`" is held by review, not by the type system
(goal.md §8.4, F-T30-1). Eight live server handlers advance `project.ids`
outside any command, and `create_device_impl` enriches the live project
after `CreateDevice` was applied. Those nine points are safe today only
because they increment the live counter directly. The non-destructive
group-address CSV import plans under one lock acquisition and applies under
another without a revision check (`apps/knx-server/src/domain.rs:965-1025`
at `7b64496`), so it is exposed.

**Historical evidence.** A scratch probe against `knx-core`, `knx-csv` and `knx-store`
at `7b64496` interleaved `create_area_impl`'s and
`create_group_address_impl`'s command sequence between `plan_import` and the
apply. It produced two group addresses with id 1 and an area counter of 0
while area 1 existed. `save_project` returned `Ok`, and after reload only
one of the two group addresses was left **[V]** for the library path. The
race has not been reproduced over HTTP; it is reachable by construction on
tokio's multi-threaded runtime.

**Historical cost.** Silent loss of a user's group address (or another entity) on save,
with no diagnostic. It needs two concurrent edits against one project, so
it is rare with one user and one browser tab.

**Partly mitigated (ADR-0039 phase 1, DIN-11, 2026-09-27).** Every
id-inserting command (`CreateDevice` and its com objects, `CreateArea`,
`CreateLine`, `CreateGroupRange`, `CreateGroupAddress`,
`CreateBuildingPart`, a new-instance `SetParameterValue`) now refuses an id
already in use with `CommandError::IdInUse`, so the appendix interleaving
ends in a typed refusal and a `Batch` rollback instead of a silent loss on
save (`crates/knx-app/tests/id_allocation_integrity.rs`, which fails with the
check removed **[V]**). `Command::ReserveIds` exists, and `load_project`
raises a stored counter below an id in use and reports it
(`load_project_reporting`; the server logs a warning on open, CLI
`ga-import` prints it). **Phase 2:** the CSV planner and scan reconciliation
emit the never-rewinding `ReserveIds` instead of `SetIdAllocators`. Every
applied CSV plan is bound to the revision it was planned against, so an edit
in between refuses it with "import or preview again" before it gets to the
id backstop (`domain.rs::a_csv_plan_is_refused_when_the_project_changed_after_planning`,
red without the binding **[V]**). Undoing an import or scan apply keeps the
counters' high-water mark, so an undone stable id such as `KB-GA-n` is never
reissued. Current remaining phases 3–5: six server allocation paths still
advance live `project.ids`; catalog creation already allocates from a clone,
but single creation assigns that clone after command success and seed
enrichment still runs after the command. The ID field is still public and no
mutation-source gate enforces the rule. These are architectural enforcement
boundaries, not a newly demonstrated silent-loss regression.

**Lifted when.** [ADR-0039](../adr/0039-project-mutation-goes-through-commands.md)
(Accepted) phases 1–2 are already merged. This residual entry closes only
after explicit activation and verified phases 3–5 remove the current live
bypasses, seal the agreed ID surface and add `check-project-mutation`, or
after the user explicitly accepts the disclosed continued deferral. Neither
decision is inferred here; until then this entry stays open.

**Alpha decision (user, 2026-10-04).** Phases 3–5 stay deferred past the Alpha. Phases 1–2 and the AR02 exhaustion refusal are the Alpha boundary.
<!-- END retained catalogue record -->
