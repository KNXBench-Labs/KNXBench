# ADR 0033: Destructive CSV imports require revision-bound confirmation

Date: 2026-09-23
Status: Accepted
Session: 7

## Context

A group-address CSV is an externally editable file. Inferring deletion from a
missing row would make filtered exports dangerous, while interpreting an edited
address as a move would guess identity from a value that is also the lookup key.
Task 16 therefore requires every move/delete to be visible before mutation and
requires affected communication-object links to be named. The application
already has a pure `parse -> plan -> Command::Batch` path, stable
`GroupAddressId` links, a monotone server project revision, and reversible core
commands.

## Decision

Destructive CSV intent is explicit: `Action=readdress` uses `Address` as the
source and requires `NewAddress`; `Action=delete` names an existing source.
Absence never means delete. Readdressing changes address/range on the same
`GroupAddressId`; deletion is rejected while links still reference that id.
The pure CSV plan reports every requested move/delete and every affected
communication-object id/direction pair; send and receive links are not collapsed.

A destructive plan is never applied by its first request. Server/web callers
receive an opaque SHA-256 token bound to the exact CSV bytes, server incarnation
and project revision, then must send that token in a second request. Application
holds the project lock while checking the expected revision and applying the
single batch. CLI follows the same preview/confirm shape with a token bound to
CSV and the loaded semantic project snapshot, then reloads and compares that
project before apply (so SQLite WAL changes are included while bookkeeping-only
file-byte changes are ignored). Changed input or state invalidates confirmation.

## Alternatives considered

- **Infer moves from changed addresses or deletes from missing rows.** Rejected:
  partial files and non-unique names make intent ambiguous and potentially
  catastrophic.
- **Apply immediately after printing the plan.** Rejected: output is not proof
  that a person saw or approved it.
- **Confirm with a bare boolean.** Rejected: it would approve a different plan
  after the CSV or project changed.
- **Delete linked addresses and remove/orphan links.** Rejected: either silently
  discards engineering intent or leaves corruption. Unlinking must be a
  separate explicit operation.

## Consequences

Readdressing retains links and is undoable; server application is one undo step.
Consumers must support a preview response and a second confirmed request.
Confirmation tokens are deliberately opaque and session/state specific, not
credentials. Group-range CRUD remains outside the address-row contract; moved
addresses are assigned only to an already existing containing range. Tests must
cover malformed actions, duplicate source/target addresses, occupied targets,
linked-delete rejection, preview-before-apply and stale confirmations.
