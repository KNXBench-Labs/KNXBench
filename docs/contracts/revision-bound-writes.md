# Revision-bound parameter writes and installation renames

Goal A2, 13 October 2026. Server contract for clients that edit a parameter
value or an installation name while other clients may change the same project.
It reuses the binding the device/group-address rename routes already use
(`apps/knx-server/src/rename_routes.rs`); nothing here is a user or account
model. The consumer side (web workbench) belongs to Goal B1.

## The binding

A client takes the three context fields from the last project tree it
received (`GET /api/project` or the tree returned by any mutation):

| Field | Meaning |
| --- | --- |
| `serverIncarnation` | This server process run. Changes on every restart. |
| `projectIncarnation` | This opened/created/imported project. Changes when any client opens, creates or imports another project, even if the revision number happens to match. |
| `snapshotRevision` | The project state. Every published change of project content (command, undo, redo, save, import, note/instruction apply, scan reconciliation, history restore) increases it; only id counters reserved ahead of a create change without it, and they are not content. |

Plus the exact thing it expects to replace:

| Endpoint | Expected field |
| --- | --- |
| `POST /api/device/{id}/parameters` | `expectedValue`: the `value` of that field in the panel the client showed — a string, or `null` when the panel showed no value. The key must be present. |
| `PATCH /api/installations/{id}` | `expectedName`: the installation's current name exactly as the tree showed it (untrimmed). |

A bound request carries **all four** fields. The server compares them under
the same project lock that applies the command, persists the native working
state and publishes the new revision, so no other change can interleave.

## Outcomes

| Situation | Status, `kind` | Effect |
| --- | --- | --- |
| Binding current, value/name as expected, write valid | `200` with the new panel (parameters, `tree` included) or tree (installation) | One undo step; revision +1 |
| Any of the three context fields differs | `409`, `parameterWriteConflict` / `renameConflict` | None: model, ids, undo/redo, revision and file unchanged |
| Expected value/name differs from the current one | `409`, same kinds | None |
| Installation name equal to the current one (after trimming) | `200`, current tree | No command, no undo step |
| Blank installation name, unknown installation | `422`, `renameInvalid` | None |
| Field not writable, value out of bounds, unknown parameter | `400` (as before) | None |
| Some but not all binding fields | `422`, `revisionBindingIncomplete` | None |
| A misspelled or unknown body field | `422` (body rejected) | None |
| Native persistence fails (e.g. another writer moved the file's history generation) | `400` parameters / `500` bound installation rename; never `409` | None: the change is applied to a copy and published only after it was persisted |

A conflict means: refresh, show the user the current value, keep the user's
draft, and let them decide. It never means "retry the same body".

## Placement (both forms)

- A parameter value is written into the installation that already stores it
  for that device, else into the device's own installation — never into the
  first installation by assumption. Overwriting an existing value keeps its
  row id and allocates no new parameter id; a new row's id is allocated on the
  same copy that is persisted, so a refused or failed write spends no id.
- More than one stored row for the same device/parameter is refused as
  ambiguous rather than resolved to the first.
- The installation is resolved by its stable ID, never by name or order.

## Legacy (unbound) requests

A request with **none** of the binding fields keeps the previous behaviour:
it is applied against whatever the project holds at that moment (the
placement fixes above apply to it too). This exists only so that the current
web client keeps working until Goal B1 migrates it; it is not a second
policy. After B1, the owner may refuse unbound requests (a separate,
documented decision). A partially bound or misspelled request is never
downgraded to an unbound one.

## For Goal B1 (consumer contract)

- Send the binding from the tree that rendered the edited field; do not derive
  it from local counters, timestamps or labels. The parameter editor host's
  `revision` (server/project/snapshot) is exactly these three fields.
- `expectedValue` is the panel field's `value` as received (including
  `null`), not the draft and not a reformatted number.
- Publish the returned panel and tree; on `409` keep the draft, reload panel
  and tree, and show that the value changed elsewhere. Never resend the same
  body automatically.
- Installation rename: send the name the user typed; the server trims it.

## Evidence

`apps/knx-server/tests/http_revision_bound_writes.rs`: two clients on one
revision, a project switch at an equal revision, another server run, a
changed expected value, a native persistence failure (history generation moved
by a second writer), partial and misspelled bindings, the unbound legacy form,
placement in a second installation without spending an id, and the same set
for installation renames. Not covered: real browser clients (Goal B1), more
than one server process on one file (the native history generation guard
covers that separately), any bus or device action.
