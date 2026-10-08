# Structural Drag-and-Drop Design

**Status:** Proposed, 2026-09-22  
**Goal task:** T11 / B10

## 1. Intent and success

KNXBench already moves a device to a topology line or building part through
validated, undoable domain commands. T11 adds direct manipulation to the
Project Explorer without creating another mutation path.

Success means:

1. a device in the first installation can be dragged onto a line;
2. the same device can be dragged onto a building part;
3. each drop calls the same API route and domain command as the existing
   Inspector field;
4. the Inspector fields remain discoverable keyboard equivalents;
5. valid targets are apparent before release, rejected mutations leave the
   displayed project unchanged, and success or failure is announced;
6. no generic drag framework, new server route, or KNX traffic is introduced.

## 2. Scope ruling

T11 implements exactly two gestures:

- device → topology line;
- device → building part.

The third candidate from `goal.md`, group address → communication object, is
deliberately not included. `LinkComObject` requires a `Send` or `Receive`
direction. A drop has no existing unambiguous direction, and silently choosing
one would change engineering semantics. A direction chooser would be a separate
cross-panel interaction rather than the small structural move delivered here.
This omission is recorded in `docs/IMPLEMENTATION_STATUS.md`; T11 does not
create a half-working default.

No other tree item becomes draggable or droppable.

## 3. Existing command ownership

The frontend continues to call:

- `api.moveDeviceToLine(deviceId, lineId)` → existing device-line route →
  `Command::MoveDeviceToLine`;
- `api.moveDeviceToBuildingPart(deviceId, partId)` → existing building-part
  route → `Command::MoveDeviceToBuildingPart`.

The server remains the mutation authority. Both commands run through the
existing command stack, validation, persistence synchronization, and undo/redo
path. Drop handlers only identify source and target and dispatch one of these
existing calls. They never mutate a `ProjectTree` locally.

The existing `LineMoveField` and `BuildingPartMoveField` in `Inspector.tsx`
remain the keyboard path. Selecting a device in the Project Explorer exposes
both labelled selects; changing one invokes the same API function used by the
matching drop. No keyboard-only shadow command or clipboard state is added.

## 4. Eligibility and trust boundary

The current commands and Inspector fields operate against the first
installation. Drag affordances follow that same boundary:

- only a device found in the first installation's topology is an eligible
  drag source;
- only lines and building parts belonging to the first installation are drop
  targets;
- second-installation nodes remain ordinary selectable tree nodes;
- absence of an installation yields no drag affordance.

This makes the pre-drop affordance honest for every constraint already visible
in the projected tree. The server still revalidates at drop time because the
project can change between drag start and the request.

The native `DataTransfer` payload is untrusted browser input. It uses one
application-specific MIME type and carries only a base-10 device id. A drop is
accepted only when the payload parses completely as a safe positive integer
and that id is still an eligible device in the current tree. Unsupported or
malformed payloads never call an API and never enable a target.

## 5. Frontend interaction

`ProjectExplorer` owns the short-lived drag state because source and both target
kinds are in that tree. `TreeNode` gains only the optional native event props
needed by its label button; there is no drag registry or reusable framework.

On drag start, an eligible device label:

1. writes the typed device payload;
2. records the eligible device id in component state;
3. exposes valid line and building-part targets through attributes/classes.

A valid target prevents the native `dragover` default and sets move semantics.
On drop it rechecks the payload against the current tree and calls exactly one
existing API function. Drag end clears all transient state. Invalid nodes never
prevent the default and never appear ready to accept the device.

After a successful response, `ProjectExplorer` forwards the returned tree to
the existing `onTreeUpdate` callback and sends a translated summary through
`App`'s existing `pushFun` toast path. A rejection calls the existing error
toast path with `api.errorMessage(error)` and does not call `onTreeUpdate`.
`ToastStack` already exposes success as `role="status"` and failure as
`role="alert"`, so screen readers receive the outcome without a second live
region implementation.

Dropping onto the device's current container is allowed because the domain
command accepts it; the frontend does not invent a different validation rule.

## 6. Visual and language rules

Drag source and valid-target states use existing `--knx-*` design tokens only.
No per-theme selector or hard-coded colour is added. Target feedback uses a
static border/background/cursor treatment. If any transition is useful, it is
defined only inside `@media (prefers-reduced-motion: no-preference)` and remains
compatible with both shipped motion styles; static feedback must remain when
motion is reduced.

Every label, success summary, and error wrapper comes from the typed English
and German catalogues. Group addresses retain fixed slash notation. Test data
uses synthetic identifiers and documentation addresses only.

## 7. Failure and concurrency behavior

- A malformed or foreign drag payload is ignored without an API call.
- A source or target that is no longer eligible at drop time is ignored.
- A server rejection shows the existing error toast and retains the current
  projected tree.
- A successful server response replaces the projected tree exactly as other
  Inspector mutations do and remains undoable.
- The UI does not optimistically move a node.
- Dragging, hovering, and cancelling cause no persistence or network request.
  Only the explicit drop sends the existing HTTP mutation; no KNX, multicast,
  discovery, tunnelling, or hardware activity occurs.

## 8. Test contract

Implementation follows RED → GREEN behavior.

1. Project Explorer tests prove an eligible device exposes native drag data and
   second-installation devices do not.
2. Per gesture, a real drag/drop event proves the exact existing API function
   receives the device and target ids and the returned tree reaches
   `onTreeUpdate`.
3. Per gesture, an API rejection proves `onTreeUpdate` is not called and the
   error announcement callback receives the error.
4. Malformed, foreign, stale, and second-installation payloads prove no API is
   called and invalid targets do not advertise acceptance.
5. Inspector tests prove each labelled select remains a keyboard-operable route
   to the same API function.
6. App integration proves a successful drop renders a translated
   `role="status"` toast and a rejection renders the existing alert path.
7. `motionGuard.test.ts`, all five theme-token checks, TypeScript, the complete
   frontend suite, and repository gates remain green.

## 9. Expected files

Expected product changes are limited to:

- `apps/knx-web/src/ProjectExplorer.tsx` and its tests;
- `apps/knx-web/src/App.tsx` and focused integration tests for toast wiring;
- `apps/knx-web/src/Inspector.test.tsx` only where keyboard equivalence is not
  already proved;
- `apps/knx-web/src/messages/en.ts` and `messages/de.ts`;
- `apps/knx-web/src/styles.css` and `motionGuard.test.ts` if styling needs new
  selectors;
- `docs/IMPLEMENTATION_STATUS.md` and the normal `.ai` handover evidence.

No Rust production file, API route, dependency, generic drag service, or
`docs/archive/alpha-0.1/LIMITATION_TRIAGE.md` edit is expected.
