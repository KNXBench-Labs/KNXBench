# ADR 0039: A live project changes only through `Command::apply`, and ids are reserved by a command that never rewinds

Date: 2026-09-26
Status: Accepted (approved per goal.md §12; phase 1 implemented in DIN-11, 2026-09-27)
Session: DIN-10 (Paperclip), design only — no product code

## Context

`Project`'s six fields are all `pub` (`crates/knx-core/src/project.rs:185-190`
at `7b64496`), so ARCHITECTURE §6's rule "every mutation is a `Command`" is
held by review, not by the type system (goal.md §8.4, parked finding
F-T30-1; [KNOWN_LIMITATIONS §107](../KNOWN_LIMITATIONS.md#107-there-is-no-plugin-api--a-third-party-cannot-add-a-format-a-protocol-a-report-template-or-a-ui-panel-without-forking)).
The ledger calls this T22. It is not the out-of-scope multi-user T22.

### Re-measured mutation sites (production code outside `knx-core`, `7b64496`)

The count is test code excluded (`#[cfg(test)]` modules, `tests/`,
`testutil.rs`). It matches the 2026-09-22 audit (`.ai/CURRENT_STATE.md`)
exactly: 22 field-root mutation sites and 9 live bypass points **[V]**.

| Class | Sites | Where |
| --- | --- | --- |
| Construction of a not-yet-live `Project` | 12 | `apps/knx-server/src/domain.rs:550,551,553,555` (`new_project_impl`'s `replacement`); `crates/knx-etsproj/src/map.rs:157,165,170,173` and `:206,215,220,223` (schema-11 and schema-≥21 mappers) |
| Shared enrichment implementation | 2 | `crates/knx-productdb/src/enrich.rs:301` (`com_object_mut`), `:399` (`set_program_defaults`) |
| Live allocator mutation (bypass) | 8 | `apps/knx-server/src/domain.rs:1693` GA, `:1732` area, `:1769` line, `:1829` group range, `:1903` building part, `:2252` device, `:2256` com object, `:3784` parameter instance |
| Live post-command enrichment (bypass) | 1 call point | `domain.rs:2327`, `knx_productdb::enrich::apply` on the live project after `CreateDevice` was applied |

Parser pushes in `knx-etsproj/src/parse/installation*.rs` build a
`SourceDocument`, not a `Project`. They are not counted. Import-time
enrichment (`crates/knx-app/src/import.rs:172`) runs on a project that no
`CommandStack` owns yet, so it belongs to construction. The rest of this
record uses **live** for a `Project` that a `CommandStack` owns
(`AppState::project`, or a CLI-loaded project that it will save).

Two production sites already use the pattern the audit recommended. They
clone the allocator, allocate from the clone, and prepend
`Command::SetIdAllocators { ids }` to an atomic `Command::Batch`:
`crates/knx-csv/src/plan.rs:99,236` (group-address CSV import) and
`domain.rs:2448,2476` (`reconcile_scan_impl`).

### What the recommended design gets wrong

The audit recommended cloned allocators plus
`Command::Batch([SetIdAllocators, domain command])` for all eight live sites.
A scratch probe against `knx-core`, `knx-csv` and `knx-store` at `7b64496`
(outside the repository; [reproduction below](#appendix-reproduction)) shows
that the pattern, as it stands, is not safe to generalise **[V]**:

1. **`SetIdAllocators` is an absolute replace.** A snapshot taken before
   another edit lowers the high-water mark when it is applied. The probe
   applied a stale snapshot after a live `CreateArea`. The area counter went
   back from 1 to 0 while area 1 still existed.
2. **No `Create*` command rejects an id that is already in use.**
   `CreateGroupAddress` checks for a duplicate *address* but not a
   duplicate *id* (`validation.rs:147-162`), and `CreateArea` behaves the
   same way. Both probes accepted a second entity with an id that was
   already taken. `CreateDevice` inserts through `Devices::insert`, which is
   a `BTreeMap::insert` (`devices.rs:30`). By code reading, it overwrites an
   existing device of the same id and does not refuse it **[code-read]**.
3. **Saving collapses the duplicate without any report.** `save_project`
   rewrites every row, and `upsert_group_address` uses
   `ON CONFLICT(id) DO UPDATE`. The probe saved a project that held two
   group addresses with `GroupAddressId(1)`. `save_project` returned `Ok`,
   and after a reload only the second one was left. The first one was lost
   with no warning.
4. **The server can already reach this.** `import_group_addresses_csv_impl`
   plans under the project lock (`domain.rs:965-973`), releases it, writes
   the session log, and then applies. A non-destructive import applies
   through `apply(state, cmd)` with no revision check (`:1025`). Only a
   destructive import is revision-bound (ADR-0033). The probe interleaved
   `create_area_impl`'s and `create_group_address_impl`'s exact command
   sequence between `plan_import` and the apply. That produced two group
   addresses with id 1, and then two areas with id 1. The probe drove the
   library APIs directly. The race has not been reproduced over HTTP. It is
   reachable by construction because the axum server runs on tokio's
   multi-threaded runtime (`#[tokio::main]`).

The eight live sites are *not* exposed today, and that is the pitfall. Six
of them (GA, area, line, range, building part, parameter) allocate under one
lock acquisition, release the lock, and apply under a second one (`apply()`
re-locks). They are safe only because the live allocator increment consumes
the id immediately. If those six moved to a cloned snapshot plus
`SetIdAllocators`, as recommended, they would inherit defect 1–3.
`create_device_impl` holds the lock throughout (`domain.rs:2248-2331`).

### Undo history and persistence, as they are

- The undo history is in memory only. `CommandStack` lives in `AppState`.
  It is replaced whenever the project is replaced (`domain.rs:1410`), and it
  is never written to the store. No `knx-store` table or migration mentions
  undo **[V]**.
- `Command` has no serialised form. `knx-core` depends on `chrono` only, and
  none of the 16 workspace manifests is publishable (§107) **[V]**. The only
  external rendering of a command is the session log. `command_name` takes
  the first token of its `Debug` form (`domain.rs:678`). Tests pin
  `source == "CreateGroupAddress"` and `"CreateDevice"`
  (`http_group_address_csv.rs:374`, `http_documentation_export.rs:214`,
  `domain.rs:4778`).
- The allocator is persisted as the single row of `id_allocators`
  (store schema v9) and is restored verbatim by `load_project`
  (`knx-store/src/project.rs:409-426`). Nothing checks it against the
  largest id actually stored.
- Allocator undo semantics are split today. The eight live sites never
  rewind on undo. CSV and scan reconciliation rewind on undo, and
  IMPLEMENTATION_STATUS and GAP_ANALYSIS E2 document "apply → undo restores
  allocator … exactly". `Project::same_user_content_as` exists to hide that
  split from the unsaved-changes flag.

## Decision

1. **Rule.** A live `Project` is mutated only by `Command::apply`, through
   a `CommandStack`. A `Project` that no stack owns yet may be built
   directly. That covers construction, the import mappers, import-time
   enrichment and `load_project`. Replacing the whole project value is a
   session transition, and it resets the stack together with the project.
2. **Ids are reserved by a command, and the reservation never rewinds.** A
   new variant, `Command::ReserveIds { through: IdAllocators }`, raises each
   counter to `max(current, through)`. It never lowers one. Its inverse is
   itself, so re-applying it is a no-op. Undo therefore restores user
   content exactly and leaves the high-water mark where it was, and a
   stable id such as `KB-GA-7` never names two different entities in one
   project's history. `SetIdAllocators` stays in the enum as an
   exact-restore form, but no production path emits it.
3. **Every command that inserts a caller-chosen id refuses one that is
   already in use.** That covers `CreateGroupAddress`, `CreateArea`,
   `CreateLine`, `CreateGroupRange`, `CreateBuildingPart`, `CreateDevice`
   (including each carried com object) and a `SetParameterValue` that
   creates a new instance. They return the new error
   `CommandError::IdInUse { kind, id }`, and a `Batch` rolls back as it
   already does. The `Restore*` inverse forms are exempt, because they
   re-insert ids that their own forward command just freed. This check is
   the backstop that turns defect 1–3 into a typed refusal for every
   caller, present and future.
4. **Callers allocate from a clone taken under the lock that applies the
   command.** Each of the eight live sites clones `project.ids`, allocates
   from the clone and submits `Batch([ReserveIds { through: clone }, X])`.
   The clone and the apply happen in one project-lock acquisition. The
   server's `apply()` gains a form that takes a builder closure. The CSV
   plan and scan reconciliation switch from `SetIdAllocators` to
   `ReserveIds`. The CSV plan-then-apply race is closed by planning under
   the applying lock, or failing that by binding non-destructive imports to
   the planned revision as destructive ones already are. Either closes it.
   The implementer picks one and tests it.
5. **Seed enrichment travels inside the command.** `create_device_impl`
   enriches the detached `ComObjectInstance`s *before* it builds
   `CreateDevice`, and passes the resulting side-table entries through the
   existing `CreateDevice::program_defaults` field. `knx-productdb` gains a
   per-instance form of `enrich::apply` that returns the defaults instead
   of writing them into a project. The `&mut Project` form becomes a thin
   wrapper over it, so import enrichment stays byte-identical.
6. **Enforcement is split by what is cheap to enforce.** The `ids` field is
   sealed by the type system: it becomes private, gets a
   `pub fn ids(&self) -> &IdAllocators` read accessor and one constructor
   for `load_project` and the mappers, and gets **no** public `&mut`
   accessor. The other five fields stay `pub` and are guarded by a new
   `xtask check-project-mutation` source gate. The gate scans non-test code
   outside `knx-core` for field-root mutation of `Project` and for any
   function that takes `&mut Project`. It fails on anything not named in an
   allowlist of exact `(file, function)` pairs. The first allowlist is the
   construction and enrichment sites in the table above. The gate is a
   **source heuristic, not a type-system seal**, and its own documentation
   and tests have to say so.
7. **Load repairs a stale high-water mark.** `load_project` raises each
   allocator counter to at least the largest stored id of its kind, and
   reports whenever it had to. A file saved after defect 1 may hold a
   counter below an id that is in use. Without this repair, rule 3 would
   make every later create of that kind fail.

### Board decision points

- **B-1:** Adopt the rule in Decision 1 as ARCHITECTURE §6's contract,
  including its construction and session-transition exemptions.
- **B-2:** Adopt never-rewind allocation (Decision 2). It supersedes the
  documented "undo restores the allocator exactly" guarantee of CSV import
  and scan reconciliation. The rejected alternative is to rewind everywhere
  ([Alternatives](#alternatives-considered), R).
- **B-3:** Accept the new public `knx-core` surface: `Command::ReserveIds`
  and `CommandError::IdInUse`, the id-uniqueness refusal on every
  id-inserting command, and a private `Project::ids` with its accessor and
  constructor.
- **B-4:** Accept a source gate plus sealing `ids` only, not full sealing,
  with the gate documented as heuristic.
- **B-5:** Approve the load-time allocator repair (Decision 7). It is a
  behaviour change in `knx-store` without a schema change.

## Alternatives considered

**Status quo, documented.** This is rejected. The invariant that review is
supposed to hold has already failed in a way that loses data. The CSV race
is the proof.

**The audit's design as written: cloned allocators plus `SetIdAllocators`
for all eight sites.** This is rejected. It is the right shape, but the
absolute replace plus the missing id check make it unsafe for any caller
that plans and applies under separate lock acquisitions. Six of the eight
sites do exactly that. It would turn today's safe live increments into the
CSV import's defect. Decisions 2–4 keep its structure (clone, atomic batch,
reservation inside the command) and remove the hazard.

**R — rewind everywhere.** This option keeps `SetIdAllocators` semantics
and makes undo byte-identical for the eight live sites as well. It is
uniform with today's CSV and scan behaviour, and `same_user_content_as`
could eventually be retired. It is not chosen because undo followed by a
create *reuses* an id. A `KB-*` source id then names different entities
before and after the undo. `knx-diff` matches entities by `ets_id` first
(`knx-diff/src/key.rs:82`), so a project compared with a file saved before
the undo would pair two unrelated entities as "the same", and the session
log's detail dumps would name one id for both. Correctness and data
integrity outrank a byte-identical allocator that the user cannot see. R
remains a coherent choice if the Board values uniform byte-identical undo
more. It also needs Decision 3.

**Allocate inside `apply`.** In this option, each `Create*` loses its id
field, `apply` allocates, and the inverse carries the id for redo. Ids
could then never be stale by construction. It is not chosen now because
every `Create*` variant would change shape and each would need a second
"with explicit id" form for redo. Callers also need the fresh ids *before*
the command exists: `CreateDevice` lists its com-object ids, seed
enrichment keys by them, and a CSV batch may create several related
entities. That means a bigger public-surface change for a guarantee that
Decisions 2–4 already give.

**Seal all six fields.** This would make the invariant a type-system fact.
It is not chosen now because of proportion. A heuristic count finds about
130 production and about 470 test field accesses outside `knx-core` across
60 files. Enrichment would have to become a command or a `knx-core`
function, and every fixture crate would need a builder. Sealing `ids` alone
covers the class that actually corrupted data, at a small fraction of that
churn (21 production and 29 test lines outside `knx-core`). If the gate
proves leaky, a later ADR can revisit full sealing.

## Consequences

**Store schema.** There is no change. `CURRENT_SCHEMA_VERSION` stays 9, the
`id_allocators` table keeps its shape and meaning, and files written before
and after the change can be read by either build. The load-time repair
(Decision 7) only ever raises a counter, and a counter is persisted state
that holds no user content.

**Undo history.** Nothing is persisted, so nothing needs migrating. A
server restart or a project replacement already clears the stack. The
behaviour changes as follows:

- For the eight live sites, undo does not rewind the allocator, the same as
  today. The allocator advance now *is* part of the undoable batch. A create
  that fails validation no longer consumes an id.
- For CSV import and scan reconciliation, undo restores all user content
  exactly but no longer rewinds the allocator. The scan test that compares
  `{:#?}` dumps (`http_bus_scan.rs:484-487`) must be rewritten to compare
  `same_user_content_as` and assert the retained high-water mark. The
  IMPLEMENTATION_STATUS and GAP_ANALYSIS E2 sentences that promise an exact
  allocator restore must be changed explicitly in the same commit, not
  silently.
- `same_user_content_as` stays. It is now the documented definition of
  "unchanged".

**Public surface.** `Command` and `CommandError` gain one variant each.
Both enums are exhaustive, so `knx-store::command_sync`'s match and the
`Display` implementation need arms. The `Project` struct literal in
`knx-store/src/project.rs:469` and the mappers' assignments to
`project.ids` move to the new constructor. No crate outside the workspace
can depend on `knx-core` (`publish = false`).

**Session log.** A single edit becomes a `Batch`. `command_name` has to
report the domain command of a `Batch([ReserveIds, X])` so that the entries
stay `"CreateGroupAddress"`, `"CreateDevice"`, and so on. The existing
tests listed above pin this.

**HTTP, web UI, CLI output.** There is no change to `ProjectTree` or to any
route. A request that loses a planning race now gets a 400 with an
`IdInUse` message instead of silently producing a duplicate.

**What has to be tested.** Each phase needs:

- for every id-inserting command, a refusal of an id in use;
- `ReserveIds` never lowering a counter, and being self-inverse;
- the appendix's stale-plan interleaving as a regression test, which must
  end in `Err`, leave the project untouched and lose nothing on save;
- create → undo → redo → create at each migrated site, with no id issued
  twice;
- enrichment through `CreateDevice` matching today's post-command result,
  including a corpus run with `KNXBENCH_PRODUCT_CORPUS` set;
- the gate's own positive and negative fixtures, including `&mut Project`
  parameters and exemption of `cfg(test)` code.

## Migration plan

Each phase is one issue, one task branch and one independent review. It
passes the full DIN-3 gate set and can be reverted on its own, because no
phase changes persisted data.

1. **Phase 1 — `knx-core` backstop (B-3, B-5).** Add `IdInUse` checks and
   `ReserveIds`. Add the load-time allocator repair in `knx-store`. Add the
   appendix's scenario as a failing-first regression test. No caller
   changes yet. This alone turns the CSV race from silent loss into a typed
   refusal.
2. **Phase 2 — the two snapshot callers.** Move `knx-csv` plan and scan
   reconciliation to `ReserveIds`. Close the CSV plan/apply window. Update
   the scan test and the two documented guarantees (B-2).
3. **Phase 3 — the nine live bypass points.** Add the builder form of
   `apply()`. Migrate the eight allocator sites. Move seed enrichment into
   `CreateDevice`, with the per-instance `enrich` API. Add the
   `command_name` shim.
4. **Phase 4 — seal `ids`.** Make the field private, add the accessor and
   the constructor, and move tests to fixed ids or a clone followed by
   `ReserveIds`.
5. **Phase 5 — `xtask check-project-mutation` (B-4).** Add the gate with an
   exact `(file, function)` allowlist. Wire it into CI and into DIN-3's gate
   list. Update ARCHITECTURE §6 and close
   [KNOWN_LIMITATIONS §129](../KNOWN_LIMITATIONS.md#129-a-stale-id-allocator-snapshot-can-duplicate-ids-and-saving-then-drops-one-entity).
   Note that the §107 plugin analysis still stands: a gate is not a seal.

Phases 1 and 2 are the data-integrity fix and should not wait for 3–5. If
the Board rejects B-2 in favour of R, phase 2 shrinks to closing the CSV
window, and phase 3 emits the rewinding form instead.

**Progress.** Phase 1 was merged on 2026-09-27 (`43f68a0`). Phase 2 is on
the same task branch: `knx-csv` `plan_import` and scan reconciliation now
emit `ReserveIds`. Every applied CSV plan is bound to the revision it was
planned against, destructive or not. The scan undo test now asserts
same-content-plus-high-water-mark, and both documented guarantees are
updated. Phases 3–5 remain open.

## Appendix: reproduction

The probe was a scratch binary that depended on `knx-core`, `knx-csv` and
`knx-store` by path at `7b64496`. It is not committed. The core of it:

```rust
// Request A plans a non-destructive CSV import, then releases the lock.
let plan = knx_csv::plan_import(&p, &knx_csv::parse_group_addresses(
    "Address,Name\n4/4/4,From CSV\n", GroupAddressStyle::ThreeLevel));
// Request B interleaves, exactly as create_area_impl / create_group_address_impl do.
let a = p.ids.next_area_id();          stack.do_command(&mut p, create_area(a, 1))?;
let g = p.ids.next_group_address_id(); stack.do_command(&mut p, create_ga(g, "5/5/5"))?;
// Request A resumes: apply(state, cmd), no revision check.
stack.do_command(&mut p, plan.command.unwrap())?;  // Ok
// -> group addresses [(1,"live"), (1,"From CSV")], area counter 0 while area 1 exists
// -> next create_area gets AreaId(1) again and is accepted: areas [1, 1]
knx_store::save_project(&conn, &p)?;               // Ok; reload keeps one GA with id 1
```
