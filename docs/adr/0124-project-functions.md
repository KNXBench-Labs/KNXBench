# ADR 0124: Project Functions are placed in one building part and refer to group addresses of its installation

Date: 2026-10-13
Status: Accepted implementation decision (Goal A4, self-reviewed; no ETS round trip)
Session: Goal A4 (`docs/goals/unassigned-data-integrity.md`)

## Context

MODEL-07 asks for ETS "Functions": named groups of group addresses that ETS
shows in its building view. RESEARCH §15 found them specification-grounded
for *Project Schema23* and asked for this ADR before any implementation. A
parked work-in-progress commit (`1154b72f`, schema 12, unreviewed, failed
acceptance) was read as a reference only; nothing was merged from it.

Evidence is marked **[D]** for *Project Schema23 v01.00.00* (SHA-256 prefix
`8fe9193e70135064`, `pdftotext -layout`, PDF page numbers), **[M]** for a
measurement made for this ADR on 2026-10-13, **[C]** for code.

- **[D] Placement.** §1.2.6.4 `Space_t` lists `Function` children ("List of
  functions in this building part"); §1.2.6.7 types them `Function_t` (p. 55–56).
  Spaces nest, so a Function sits under any `Locations/Space[/Space…]`.
- **[D] Function_t** (§1.2.6.9, pp. 56–57): required `Id`, `Name`
  (`String255_t`), `Puid` ("After deletion of the element, no other element
  will receive the same Puid"); optional `Type` (`String255_t`, "the optional
  type of the function"), `Implements` (IDREFS "to the function types this
  function implements"), `Number`, `Comment`, `Description`,
  `CompletionStatus` (default `Undefined`) and an IDREF spelled
  `DefaulGroupRange` ("the RefId of the default GroupRange"). Children:
  `GroupAddressRef`.
- **[D] GroupAddressRef_t** (§1.2.6.10, p. 57): required `Id`, `RefId`
  (the group address), `Name`, `Puid`; optional `Role`.
- **[M] One real instance, in an unsupported schema.** xknxproject's public
  test resource `testprojekt-ets6-functions.knxproj` (checkout `f97db068`,
  GPL-2.0, inspected, not copied) is ETS 6.1.5686, schema 22. It holds one
  `Function` under a `Space Type="Room"`: `Id="P-05C0-0_F-1"`,
  `Name="LivingroomLight"`, `Type="FT-1"`, `Puid`; two `GroupAddressRef`s with
  `Id` `…_GF-n`, `Name=""`, `Role` `SwitchOnOff`/`InfoOnOff`, a full `RefId`
  `P-05C0-0_GA-n` and `Puid`. No `Implements`, `DefaulGroupRange`, `Number`,
  `Comment`, `Description` or `CompletionStatus` occur. KNXBench refuses
  schema 22, so this sample cannot be imported; it confirms the shape only.
- **[M] No supported-schema sample.** None of the 6 readable private-corpus
  `.knxproj` files and none of the other 7 readable public samples contains
  a `Function` element.
- **[C] Before this ADR** a schema-23 `Function` was an unknown element:
  reported, kept verbatim in the opaque store, not modeled.

## Decision

1. **Model.** `Project.functions` is an ordered list of `Function { id,
   source, location: BuildingPartId, details, members }`;
   `FunctionDetails { name, function_type, number, comment, description,
   completion }`; `FunctionMember { id, source, group_address, name, role }`.
   IDs are new project-wide allocators (`function`, `function_member`). A
   Function owns only its members; it *refers to* one building part and to
   group addresses, which keep their own owners.
2. **Invariant** (`function::validate_functions`): non-zero unique IDs; the
   location exists in exactly one installation; every member's group address
   exists exactly once and in that same installation. Commands publish only
   states that satisfy it; the store refuses to save or load anything else.
3. **Not modeled, kept verbatim.** `Type` is stored as text and not resolved
   against master-data function types. `Implements`, `DefaulGroupRange` (its
   spelling is evidenced only by the schema text) and `Puid` are known
   attributes but not fields: the importer keeps the whole `Function` element
   byte for byte in the opaque store, keyed
   `…/Locations/Space/Function[@Id='…']`. There is no ETS exporter (ADR-0028),
   so PUID reuse rules do not arise.
4. **Commands.** Create, delete/restore, update details (edit admission:
   non-blank single-line name ≤ 255 code points, ≤ 255 for type/number/member
   text, 64 KiB comment/description) with an unvalidated undo form, move
   within the installation, add/remove/restore members. A group address may
   appear once per Function when edited; imported duplicates are kept.
   Cross-installation moves and members are refused (`CrossInstallation`).
5. **Deletes refuse, never cascade.** Deleting a group address that is a
   member (`GroupAddressInFunction`) or a building part that holds Functions
   (`BuildingPartHasFunctions`) is refused, like existing in-use refusals.
6. **Native schema 15.** Tables `project_function_state`, `project_function`,
   `project_function_member`; additive migration from 14. History images of
   schemas 12–14 decode and migrate in memory; the semantic identity of an
   image without Functions ignores the empty new tables, so predecessor
   history keeps its identity.
7. **Import scope.** Only schema 23 maps Functions (spec-backed). Schemas 20
   and 21 keep reporting `Function` as unknown and retaining it: their
   content model is not inferred from schema 23 (RESEARCH §15.2). A member
   whose `RefId` does not resolve, or resolves into another installation, is
   reported (`UnresolvedReference`/`CrossInstallationReference`) and left out
   of the model; the raw copy still holds it. The import report counts
   `Function` and `GroupAddressRef` read/mapped.
8. **Neighbours.** Selective import (model 15) keeps destination Functions
   and refuses a source with Functions instead of dropping them. The project
   diff lists Functions as one comparable summary (no internal IDs), so a
   Functions-only difference is never "equal". The documentation report lists
   them per installation in the Buildings section.
9. **Projection and API.** `knx_projection::build_functions_view` is the read
   shape; the server offers it at `GET /api/project/functions` and runs every
   command through `POST /api/project/functions/edit`, bound to the snapshot
   the client read (typed 409/422/500, no effect on refusal), as an undoable
   step that goes through the native working history.

## Alternatives considered

- **Take over `1154b72f`.** Rejected by the goal and on design grounds: it
  stored source payloads, issue lists, coverage flags and PUID tombstones
  inside the domain model, on schema 12. Raw retention already exists in the
  opaque store.
- **Model `Implements`/`DefaulGroupRange` as references.** Rejected: no real
  sample uses them, the second's spelling is unverified, and resolving
  `Implements` needs master-data semantics nobody has evidenced.
- **Cascade deletes into Functions.** Rejected: silently removing a member
  hides an engineering change; refusing matches how linked group addresses
  and non-empty building parts already behave.
- **Map schema 21 too.** Rejected until a schema-21 sample with Functions or
  its published schema exists.

## Consequences

- Goal B gets a tested contract: model, commands, storage, import,
  projection and HTTP API (`docs/contracts/project-functions.md`). The web UI
  and a per-Function diff table are later work; until then Functions are
  neither visible nor editable in the web UI.
- Every new consumer must respect the invariant and the delete refusals.
- Like notes, a command on a project that has Functions is applied to a
  clone and published only if the invariant still holds. That costs one
  project clone per command; it has not been measured on large projects and
  is the first place to look if editing such a project feels slow.
- Lifted further when a supported-schema ETS export with Functions (ideally
  with `Implements`/`DefaulGroupRange`) is added to the corpus.

## 2026-10-17 amendment: comparison, desktop and MCP surfaces

The three surfaces this ADR left for later now exist (MODEL-07 residue):

- **Comparison.** `knx_diff::InstallationDiff::functions` is a per-Function
  table (`added`/`removed`/`changed`/`ambiguous`), not the earlier single
  `functions` project-info line. A Function matches by its ETS id, else by
  the root-first path of its building part plus its name; two candidates for
  one such key are reported as ambiguous, never guessed. Changed fields are
  `name`, `function_type`, `number`, `comment`, `description`, `completion`,
  `location` and `members`. Members compare in order by raw group address,
  name and role, so a project that only spells addresses in another
  `GroupAddressStyle` shows no Function change. A Function whose building
  part no installation lists (impossible in a valid project) still makes the
  diff non-empty through an `unplaced_functions` project-info line. The CLI,
  the HTTP comparison response, the web comparison panel and MCP
  `diff_projects` all render the table.
- **Desktop.** By [ARCHITECTURE](../ARCHITECTURE.md) rule 3 the desktop shell
  has no domain IPC: its window loads the web UI from the in-process
  `knx-server`, so the Functions panels and routes are the desktop surface.
  `knx-desktop`'s `functions_surface_tests` start the shell's own
  `spawn_server` on a loopback port and drive read, create, member add, a
  stale `409` and undo over plain HTTP. No Tauri command was added.
- **MCP.** A ninth read-only tool, `list_functions`, returns
  `build_functions_view` per project, optionally one installation, paged
  (ADR-0090 amendment of the same date).

Still not modeled: schemas 20/21 (decision 7), `Implements` and
`DefaulGroupRange` (kept verbatim), master-data names for `Type`, and a
Functions overview outside building-part and group-address Properties.
Lifted as stated above, by a supported-schema export that uses them.
