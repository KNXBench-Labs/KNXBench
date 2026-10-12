# Installation instruction editor — isolated host contract

This is the AP2 handoff to the UI-shell owner, not a claim that the workbench
already exposes this feature. The component is
`apps/knx-web/src/features/rtf/InstructionEditor.tsx`.

## Host inputs and outcome

Mount `InstructionEditor` with the selected stable `deviceId` and authoritative
`context`: `serverIncarnation`, `projectIncarnation`, `snapshotRevision`.
The existing `ProjectTree` uses **snake_case** names:
`server_incarnation`, `project_incarnation`, `snapshot_revision`. Map those three
fields explicitly to this component's camelCase context. Pure/offline projections
may omit them: do not mount a writable editor with missing identity, and never
invent a local revision counter. Require a nonempty server identity and safe
nonnegative integer load/revision values. The isolated fixture obtains these
fields from the admitted instruction read DTO instead of fabricating a tree.
 `context: null` mounts a read-only view (it reads, never previews or applies).
 Optional `language` is `en` or `de`; all feature copy is isolated in
`features/rtf/copy.ts`. Its CSS module uses the existing theme variables without
modifying global styles or installing new dependencies.

The default client uses the existing same-origin session authentication and
refusal notification hook. A supplied `client` is an injection seam for tests or
another explicitly compatible adapter, not an alternate persistence policy.

- On `onApplied(outcome)`, refresh the authoritative project snapshot/history
  projection and supply its new revision. Even a no-op response needs the host
  refresh, so reset/draft/provenance settle against authoritative state.
- `onDirtyChange` reports an unsaved local draft, not a saved project or committed
  native root. Integrate it into the shell's existing navigation policy; do not
  auto-apply on selection/navigation/close. If switching selection is allowed,
  that switch intentionally discards the old component-local draft.
- A revision change for the same dirty device revokes its preview and re-reads.
  If the device's instruction content (source digest, override, effective document,
  unsupported list) is unchanged, the draft moves onto the new revision and stays
  editable; otherwise it is retained for comparison and requires deliberate
  discard/reload. Saving the project bumps the revision, so this keeps Save from
  destroying drafts (amended 2026-10-12). A different device/project or
  server never inherits another selection's draft or late response.
- A confirmed request already sent to the server is not undone by unmounting.
  Late completion must not publish callbacks into a different selection. Refresh
  authoritative project state after any pending-operation navigation; the
  server's source/revision checks remain the mutation authority.

## Supported source and editing scope

The reader admits exact-device Schema23 UTF-8 XML1.0 source originals and
`DeviceInstance/@InstallationHints`; it does not treat every string as RTF.
Missing hints, explicit empty hints and a native override are distinct.
Ambiguous/missing source identity, other schemas/encodings/prefixed XML and
malformed admission are explicit refusals, not manufactured native provenance.
A natively authored device without retained imported source is not editable via
this imported-instruction workflow.

Supported normalized editing is plain text, Unicode, paragraphs and
bold/italic/underline runs, limited to 64 KiB UTF-8 text and 4,096 runs.
Unsupported formatting is reported. Replacing unsupported/malformed source's
interpreted subset requires explicit conversion consent; the exact imported
logical value and whole XML bytes remain retained. Reset removes only the native
override, returning to the imported original. There is no ETS project exporter
or verified ETS re-import claim for edited overrides.

Rendering is escaped text, never HTML/RTF execution. Embedded images, links and
resources are neither executed nor fetched. Original-reader parsing has separate
bounded XML and RTF limits. The feature does not call KNX bus/protocol services.

## HTTP lifecycle

| Operation | Endpoint | Effect |
| --- | --- | --- |
| Read | `GET /api/devices/{id}/instructions` | Source/provenance, supported document and identity; no mutation |
| Preview | `POST /api/devices/{id}/instructions/preview` | Exact context plus proposed normalized document, or `null` reset; one bounded pending plan |
| Apply | `POST /api/devices/{id}/instructions/apply` | Only plan token plus explicit confirmation/conversion choice; never a substitute document |
| Cancel | `POST /api/devices/{id}/instructions/cancel` | Exact token cancellation; no project/history mutation |

The one router-local plan expires after 300 seconds. A newer preview replaces the
older plan, including another browser's preview: the old browser must preview
again. Source/model/revision changes or another load invalidate apply. Draft
changes cancel any displayed plan; late previews after selection change/unmount
are cancelled rather than offered on the new selection.

For imported projects not yet saved natively, edits are session-only. With a
native target, the application persists a private working snapshot and both
history stacks before publishing memory changes. `native-working-history` means
recoverable working edits, not an implicit Save of the committed native root.
Reopening a native file restores its validated working snapshot and both history
stacks through the existing native-load path; it does not silently Save that
working state over the committed root. The shell must preserve the authoritative
unsaved-change indicator and the existing explicit save/discard policy.
Persistence refusal leaves
model/originals/undo/redo unchanged. The API does not bypass application services.

## Acceptance and deliberate limits

The [actual combined-source acceptance](../evidence/instruction-editing-2026-10-11.json)
is recorded separately from earlier source-bound native, HTTP and UI receipts. The isolated fixture exercises the
real HTTP/native implementation with synthetic input in a loopback-only network
namespace; it is not a production shell integration, deployment, physical bus
operation or general format compatibility test. Self-review is in-session, not
independent approval.

## Workbench host (12 October 2026)

`apps/knx-web/src/DeviceInstructionsPanel.tsx` mounts the editor as the sixth device
editor tab, **Installation instructions**. It mounts on the tab's first visit and
then stays (hidden), so a draft survives other tabs. `snapshotIdentity.ts` maps the
tree identity explicitly; incomplete identity gives the read-only view. After every
Apply (no-op included) the host re-reads `GET /api/project` through the workbench's
own snapshot publication; because the editor does not await that callback, a failed
re-read is shown by the host as "applied, reload failed". The draft flag joins the
notes draft in App's guard before Open/Import, New project, Quit and window close.
Evidence: unit tests, 11 intercepted production-build Chromium cases (EN/DE,
Porcelain/Graphite, 1440/400, stale and persistence refusals, read-only) and a
loopback-only real-server smoke with a synthetic RTF project; see
[IMPLEMENTATION_STATUS](../IMPLEMENTATION_STATUS.md). Self-review only.
