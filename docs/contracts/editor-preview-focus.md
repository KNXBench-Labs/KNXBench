# Notes and instructions: preview focus ownership

Status: implemented source contract, 14 November 2026. Focused component and
production-browser checks passed; bounded native focus observations are recorded
across two execution epochs. Final integration acceptance remains separate; this
does not upgrade earlier source-bound native editor or persistence evidence.

## Required behaviour

- Keyboard Preview remains the initiating action, not an implicit confirmation.
  After its request settles and the native-disabled button is re-enabled, return
  focus to that exact button if the request still owns focus. Never focus Apply
  or Confirm automatically, including after Return or its key release.
- Explicit Cancel removes only the review and returns focus to its exact
  initiator: Preview, Notes Remove/Move, or Instructions Reset/Remove. Do not
  substitute the main Preview button when it is disabled for those actions.
- The same editor instance, client, server/project incarnation, revision and
  selected device must still own the request. A new request, passive revocation,
  identity change, rebase or unmount retires the old focus ticket.
- User navigation while waiting permanently withdraws completion-driven focus:
  another focused control, Tab, outside pointer input or window blur. Merely
  losing focus to the document when the browser disables the button does not.
  Explicitly focusing and activating Cancel is a new user intention.
- Restoration is a post-commit operation, never a timeout. The original element
  must remain connected, enabled and outside hidden/inert ancestry. There is no
  fallback to another button. A non-focused Cancel dispatch cannot displace a
  different actively focused control.
- Reads, reloads, draft edits, expiry and other passive revocation do not request
  focus. A stale/aborted response cannot restore focus into a successor editor.
- Draft bytes, conversion consent, busy/duplicate guards, token cancellation,
  preview-to-confirm separation and mutation authority are unchanged. Preview
  and Cancel never apply a project edit or write to a device.

The two editors use `apps/knx-web/src/features/usePreviewFocus.ts` for this narrow
request-local lifecycle. It is not an application focus manager or modal policy.
Existing native-disabled controls and focus-visible styles remain unchanged.

## Evidence boundary

Mounted component regressions model only WebKit's disabled-button blur before
settling a held request. Production-browser fixtures use actual Tab and Enter,
a held intercepted Preview response, exact element/focus-visible/outline checks,
explicit keyboard Cancel, no Apply requests and moved-focus controls. Neither
kind is actual GTK/WebKitGTK proof. The separate native observations in the
[source-bound result](../status/2026-11-14-wp11-editor-focus-candidate.md) retain
their measured source epochs and delivery limits. Unrelated dialogs and
checked-parameter controls are outside this source change.
