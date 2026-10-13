# Installation-node presentation and editing

## Scope and authority

[Actual main-integration verification](../evidence/installation-node-publication-2026-10-11.json)
records the owner-authorized no-ff merge and fresh committed-source acceptance.
It supersedes the initial local-only boundary below; earlier counts remain historical.
This is not a release, live deployment or broader KNX/ETS compatibility claim.

Local source delivery on 11 October 2026, based on `9298242c`; not a commit,
release, deployment or fresh ETS import claim. `Installation` and stable
installation IDs already exist in the domain. `Command::RenameInstallation`,
`rename_installation_impl`, `PATCH /api/installations/{id}`, native history and
storage are reused unchanged. The UI's new `installation` selection kind does
not extend an external format or domain model.

## Behavior

- Missing/empty/whitespace-only names display a localized **Unnamed installation
  (ID …)** label. Display admission checks emptiness only; it never modifies names
  or IDs. Nonblank imported names retain exact spelling and whitespace.
- The label is keyboard/pointer-selectable and selects that exact installation's
  Properties. Its tooltip and Properties explain that an installation groups
  topology, buildings, unassigned devices and group addresses, not a device/line.
  Multiple installations remain independent nodes. The arrow still toggles the
  subtree; selecting the label is no longer a collapse gesture.
- Properties expose the existing name edit. Enter (outside IME composition) or
  blur submits; Escape cancels a draft. No write occurs merely from viewing an
  unnamed installation. Blank new input is refused with localized feedback;
  server errors remain intact. Explicit submissions follow the existing trimmed
  server admission. Selection is resolved by stable ID, never name/order; an
  unknown or duplicate installation ID offers no editor.
- The shared simple name field prevents overlapping submissions while its write
  is pending, including across a snapshot refresh. Unrelated snapshots preserve
  an unsubmitted draft. An old reply cannot update an unmounted/replaced view;
  a superseded success in the mounted installation editor is explicitly reported
  as not applied locally. Such a reply does not prove server cancellation.
- Existing project-level installation fields remain available. The add-device
  command uses the selected installation rather than silently defaulting to the
  first installation. No device is added by selecting or renaming a node.

## Boundaries

No new bus, installation create/delete/move, importer or compatibility capability.
The legacy installation rename endpoint is not the revision-conditional device/GA
rename endpoint; multi-client/server-side conflicts are not newly guarded here.
**Amended 2026-10-13 (Goal A2):** the same endpoint now also accepts that
binding ([revision-bound writes](revision-bound-writes.md)); the Properties
editor adopts it in Goal B1.
Browser execution is Chromium, not an interactive native WebKitGTK/Orca session.
Source-like unnamed fixtures and a real native project are exercised; no private
original archive is imported or changed by this UI delivery. Save/Open acceptance
is native `.knxdb`, not edited-project ETS re-export.

## Verification

Named baseline failures precede implementation for missing labels/selection/editor
and selected-installation wizard placement. A later named RED exposes accidental
draft reset on an unrelated snapshot. Final counts and executed production
browser/native checks are recorded in the dated implementation-status entry;
compact original logs, JSON reports, screenshots and source binding stay in the
ignored task evidence directory. Review is in-session self-review, not independent
approval. Failed initial fixtures (wrong CRT theme ID and boolean rather than
string-valued achievement preference) are setup observations, not final acceptance.
