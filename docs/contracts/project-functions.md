# Project Functions contract

Status: backend, projection and HTTP API delivered by Goal A4 (ADR-0124); the
web workbench consumes them since Goal B1b (13 October 2026, below). The
per-Function comparison table, the desktop surface and the read-only MCP tool
followed on 17 October 2026 (MODEL-07 residue, below).

## What exists

| Layer | Contract | Tests |
| --- | --- | --- |
| Model | `knx_core::function::{Function, FunctionDetails, FunctionMember}` on `Project.functions`, ordered; IDs from `IdAllocators::next_function_id`/`next_function_member_id` | `crates/knx-core/tests/functions.rs` |
| Invariant | `function::validate_functions`: unique non-zero IDs, location in exactly one installation, every member's group address in that installation | same |
| Commands | `CreateFunction`, `DeleteFunction`/`RestoreFunction`, `UpdateFunction`/`RestoreFunctionDetails`, `MoveFunction`, `AddFunctionMember`, `RemoveFunctionMember`/`RestoreFunctionMember`; all undoable, refusal leaves the project unchanged | same |
| Refusals | `GroupAddressInFunction`, `BuildingPartHasFunctions`, `CrossInstallation`, `InvalidFunction(DuplicateMember / InvalidName / TooLong / ControlCharacter / InvalidPosition)`, `FunctionInvariant`, `FunctionNotFound`, `FunctionMemberNotFound`, `IdInUse{Function, FunctionMember}` | same |
| Storage | native schema 15, `project_function*` tables; allocator repair through `load_project_reporting`; history images 12–14 migrate in memory | `crates/knx-store/tests/functions.rs` |
| Import | schema 23 only; raw element retained in the opaque store; counts `Function`/`GroupAddressRef` | `crates/knx-etsproj/tests/functions.rs`, `crates/knx-app/tests/functions_import.rs` |
| Selective import | destination Functions kept; a source with Functions is refused | `crates/knx-app/tests/selective_import.rs` |
| Diff | per installation `functions: EntityTable<FunctionKey, FunctionFields>`, matched by ETS id, else building-part path + name; ambiguity reported; `unplaced_functions` info line only for a Function no installation lists (17 October 2026, below) | `crates/knx-diff/tests/functions.rs` |
| Report | Buildings section, per installation, escaped | `crates/knx-report/tests/functions.rs` |
| Projection | `knx_projection::build_functions_view` → `FunctionsView` (TS bindings `FunctionsView`, `FunctionNode`, `FunctionLocationNode`, `FunctionMemberNode`): resolved room, member addresses in the project style, model order; `sourceId` on Functions and members is the verbatim stored source identity (ETS id, or `KB-Function-<n>`/`KB-FunctionMember-<n>` for KNXBench-made ones), never parsed | `crates/knx-projection/src/functions.rs` |
| HTTP | `GET /api/project/functions`; `POST /api/project/functions/edit` (below); `POST /api/project/diff` carries `installations[].functions` | `apps/knx-server/tests/http_functions.rs`, `http_project_diff.rs` |
| Desktop | the shell's embedded server (`spawn_server`) serves the same routes to its window; no Tauri command (ARCHITECTURE rule 3) | `apps/knx-desktop/src-tauri/src/functions_surface_tests.rs` |
| MCP | read-only `list_functions` (paged, optional installation); `diff_projects` entity `function` | `apps/knx-mcp/tests/tools.rs` |

## HTTP

`GET /api/project/functions` answers `{functions, serverIncarnation,
projectIncarnation, snapshotRevision}`.

`POST /api/project/functions/edit` takes the three binding fields from that
read plus one `action`; unknown fields are refused:

| `kind` | Fields |
| --- | --- |
| `create` | `location` (building part id), `details` |
| `update` | `id`, `details` (replaces all of them) |
| `delete` | `id` |
| `move` | `id`, `location` |
| `addMember` | `functionId`, `groupAddressId`, optional `name` (default `""`), `role`, `position` (default: end) |
| `removeMember` | `functionId`, `memberId` |

`details` is `{name, functionType?, number?, comment?, description?,
completion?}` with `completion` one of the ETS spellings (default
`Undefined`). The answer is the new read plus `createdId` for `create` and
`addMember`. A binding that no longer names the live project is
`409 functionStale`; a refused command `422 functionRefused`; a failed
persistence `500`. None of them changes the model, undo history, revision or
file, and a refusal allocates no ID. Each success is one undoable step
(`POST /api/undo`).

## Rules a consumer must keep

- Edited text goes through the commands; never write `Project.functions`
  directly. Imported text is verbatim and may fail edit admission; show it,
  and use `RestoreFunctionDetails` only for undo.
- A Function is shown in its building part; its members are references.
  Deleting a member's group address or the Function's room is refused, so a
  UI must offer "remove from Function" / "move Function" first instead of a
  cascade.
- Nothing crosses installations.
- `function_type` (e.g. `FT-1`) is not resolved to a master-data name yet.

## Web consumer (Goal B1b, 13 October 2026)

`apps/knx-web/src/features/functions/` (`client.ts` strict admission of reads
and edit answers, `model.ts` pure selection, `FunctionsPanel.tsx`):

- **Where:** a **Functions** disclosure in a building part's Properties lists
  the Functions located there; a group address's Properties lists the
  Functions it belongs to, with *Remove from Function*. Nothing is read until
  the disclosure is opened; nothing is cached across projects.
- **Binding:** every edit sends the binding of the read the panel shows.
  `409 functionStale` keeps every input, says nothing was applied and offers
  **Reload current state**; the edit is never resent by itself. `422
  functionRefused` shows the server's reason verbatim beside a fixed lead-in.
  A tree without a complete snapshot identity shows no edit controls.
- **After success** the project tree is re-read, so undo/redo, Save and the
  snapshot revision are the project's own; the open panel follows undo/redo.
- **Details editing** sends the complete `details` (update replaces all of
  them). If the read underneath changes while the form is open, the form keeps
  the user's text and shows the current values with *Keep my input* / *Start
  again from the current details*. Blank names are refused client-side with the same
  rule as the server; imported text that would fail admission is shown as is.
- **Provenance:** Function and member `sourceId` are shown verbatim, labelled
  *Source ID*; `functionType` and `role` are shown as stored, not resolved.
- **Deletes and moves** follow the rules above: deleting a Function asks once
  and counts its memberships; a member's group address and a Function's room
  are never deleted by these panels; moves offer only building parts of the
  same installation.
- **Evidence:** Vitest `src/features/functions` (client admission, model,
  panels incl. stale/refused/plural cases); real server through
  `tools/verify_name_editing.py --spec functions.native.ts` (loopback-only
  namespace, fictional demo copy): create, add member, an edit raced by a
  second client gets the real `409`, reload and apply, undo/redo, save and
  reopen with identical `GET` results, no bus/download/address write, nothing
  clipped inside the panel; EN/DE × 1440/400.

## Comparison, desktop and MCP (17 October 2026)

- **Comparison.** Each installation's diff has a `functions` table. Key:
  `etsId` (null for a Function made in KNXBench), the root-first `location`
  path and `name`. A Function without an ETS id matches by location and name;
  a rename of such a Function therefore shows as one removal plus one
  addition, and two same-named Functions in one room are *ambiguous*. Fields:
  `name`, `functionType`, `number`, `comment`, `description`, `completion`
  (`Debug` form), `location` and ordered `members` (`address` in that side's
  style, `name`, `role`). Members compare by raw address, name and role, so
  a different group-address notation alone is no Function change.
  `knx diff` prints `~ function <name> in <path> (<ETS id>): <field>: … -> …`;
  the web panel adds a **Functions** summary line and table; MCP
  `diff_projects` uses entity `function`.
- **Desktop.** The desktop window loads the web UI from its in-process server,
  so the Functions panels and routes above *are* the desktop surface. A
  `knx-desktop` test starts the shell's own `spawn_server` on a loopback port
  and drives read, create, member add, a stale `409` and undo over plain
  HTTP. No native WebKitGTK window was run for this; the panels' browser
  evidence is Goal B1b's.
- **MCP.** `list_functions {project, installation?, limit?, offset?}` returns
  `{installation, functions: {total, offset, limit, truncated, items}}`, the
  items being `FunctionNode`s; an unknown installation is refused. Nothing in
  MCP edits Functions.
- **Evidence:** `crates/knx-diff/tests/functions.rs` (10 tests),
  `apps/knx-server/tests/http_project_diff.rs`
  (`a_changed_function_is_one_row_of_the_functions_table`),
  `apps/knx-cli/tests/cli_project_diff.rs`
  (`diff_prints_a_changed_function_as_its_own_line`),
  `apps/knx-mcp/tests/tools.rs` (three Function tests),
  `apps/knx-web/src/projectDiffView.test.ts` and `ProjectDiffPanel.test.tsx`;
  8/8 targeted mutants killed.

## Not done (recorded boundaries)

- A Functions overview outside building-part/group-address Properties.
- Schema 20/21 mapping, `Implements`/`DefaulGroupRange` semantics, ETS export
  (ADR-0124 decisions 3 and 7; lifted by a supported-schema export that uses
  them).
- Any MCP or CLI edit of Functions; edits stay with the web workbench.
