# Project Functions contract

Status: backend, projection and HTTP API delivered by Goal A4 (ADR-0124); no
web UI yet. This page is the hand-over for Goal B's workbench.

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
| Diff | one `info_changes` entry `functions` (summary without internal IDs) | `crates/knx-diff/tests/functions.rs` |
| Report | Buildings section, per installation, escaped | `crates/knx-report/tests/functions.rs` |
| Projection | `knx_projection::build_functions_view` → `FunctionsView` (TS bindings `FunctionsView`, `FunctionNode`, `FunctionLocationNode`, `FunctionMemberNode`): resolved room, member addresses in the project style, model order | `crates/knx-projection/src/functions.rs` |
| HTTP | `GET /api/project/functions`; `POST /api/project/functions/edit` (below) | `apps/knx-server/tests/http_functions.rs` |

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

## Not done (Goal B or later)

- The web UI (Goal B1), the desktop (Tauri) command surface, MCP tools.
- A per-Function diff table and its CLI/web rendering.
- Schema 20/21 mapping, `Implements`/`DefaulGroupRange` semantics, ETS export.
