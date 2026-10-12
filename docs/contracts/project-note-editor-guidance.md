# Project notes: bounded editor guidance

Availability: mounted as the workbench **Notes** view in source newer than
Alpha 9 ([host](project-note-editor-host.md#workbench-host-12-october-2026)). No
Alpha/release/container/native availability is implied. The authoritative host contract is
[project-note-editor-host.md](project-note-editor-host.md); authenticated writes
remain [server-authorized](project-note-http.md).

## Safe editing workflow

- Read the ordered note list. Filter changes visibility, never project order.
- Select a note or start a new draft. Discarding another dirty draft requires
  deliberate confirmation; cancelling discard retains it.
- Title and body are literal text. Leading/trailing whitespace, Unicode and
  untouched mixed CR/LF/CRLF spelling remain data, not formatting instructions.
  HTML/Markdown-looking content does not execute, render resources or fetch URLs.
- Select the target by an authoritative host label plus stable domain ID. Missing
  labels remain visibly identified, never silently remapped. A physical/group
  address string is not a substitute target ID.
- Documentation inclusion is explicit and per-note. A note is not automatically
  an instruction, report section, device parameter or bus operation.
- Request **Review changes**; inspect exact Before/After, target, opt-in and order.
  Review is private. Remove/Move are also preview-first, not immediate deletion.
- **Confirm apply** is a separate gesture using the server-issued preview only.
  Cancel invalidates that exact preview. Editing or changing project context
  revokes it; expired previews must be deliberately regenerated.

## Recovery and persistence

`session-only` means the current in-memory project. `native-working-history`
means confirmed changes participate in recoverable native working history and
Undo/Redo. Neither label means the root project was explicitly saved. Use the
host's existing Save policy separately; the isolated component never issues it.

A successful server commit with failed host refresh is not a failed mutation.
An unknown transport/response outcome is not a rollback and must not be silently
retried. Reload authoritative state before further editing; keep the draft for
comparison. Same-project revision changes retain the dirty draft but invalidate
review. Project/server changes cannot inherit old preview authority or responses.

## Help, keyboard and limits

Existing application help uses the workbench topic; GroupRange uses the existing
HelpTip. Static feature guidance is not authored project text or a second help
database. Native controls remain keyboard accessible through the supplied host;
whole-workbench F1/navigation, Orca/native GTK and accessibility certification are
not established by the isolated Chromium fixture. EN/DE feature copy uses the
existing informal German voice and current KNX theme tokens without literal
fallback colours or inherited button hover scaling.

The host supplies real incarnation/revision and allowed stable-ID targets, handles
navigation dirtiness and refreshes authoritative state after Apply; the workbench
host does this since 12 October 2026. That is still not whole AP4 or FUTURE-03.
No KNX bus communication, device programming, ETS export/report rendering,
manufacturer database edit, new permission system or dependency is implemented
by the component.
