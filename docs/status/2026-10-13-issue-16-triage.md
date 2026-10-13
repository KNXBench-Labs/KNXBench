# Issue #16 triage — channel collapse and open object editors

## Scope and verified source

Triage on 2026-10-13, session `922727d3ed74`, source `090ab85c`.
No production implementation or project data was changed.

The reporter's attached video shows the device communication-object table: after
opening a channel and an object editor, collapsing the channel hides the other
children but leaves the open object summary/editor visible. This is distinct from
issue #11, which concerns initial expansion of the group-address tree. The video
crop does not establish the reporter's exact application version.

## Cause and existing contract

In `apps/knx-web/src/ComObjectTable.tsx:125`:

```tsx
const visible = open || matches && (flat || groupVisible && isExpanded(channelKey(com)));
```

An open editor therefore bypasses the channel's collapsed state. The header still
sets `aria-expanded` from the channel expansion set (:119–120), so it reports a
collapsed channel while a child remains visible. The same rule is present in
`v0.1.0-alpha.10`, `v0.1.0-alpha.11` and the inspected main source.

This is not an undocumented accidental rule: the existing
[table contract](../COMMUNICATION_OBJECT_TABLE.md#editor-and-request-lifetime)
explicitly says open editors remain visible when their channel is collapsed.
[ADR-0102](../adr/0102-communication-object-table-view-state.md) also requires stable
editor ownership. Hiding an editor and unmounting it are different operations;
correcting collapse must retain the latter protection.

## Executed regression evidence

In a detached worktree of the inspected main source, an isolated synthetic
component probe rendered `ComObjectTable` with one channel/object and a draft
input. It expanded the channel, opened the object, set the draft, and collapsed
the channel. No reporter project or device identifiers were used.

Command (the temporary probe is not committed):

```text
npm test -- src/ComObjectTable.test.tsx src/Issue16.repro.test.tsx
```

Observed state after collapse:

```json
{"channelExpanded":"false","childSummaryHidden":false,"childEditorHidden":false,"draftPreserved":true}
```

All seven existing table tests passed. The new collapse assertion failed exactly
at `collapsed parent must hide the editor row: expected false to be true`.
Thus the bug is reproduced at the component/DOM seam, not just inferred from the
video. React's existing `act` environment warnings were emitted; the probe reached
its named visibility assertion, and the baseline tests passed. This does not claim
native WebKitGTK/Orca or browser end-to-end acceptance.

## Owner-approved changed behaviour and acceptance

The owner approved routing a separate bugfix package to Goal B, session
`ba673d45ff9d` (suggested id B13; ordering remains that session's decision):

- Explicit channel collapse hides **all** child summary and editor rows, including
  open children and editing exceptions under filters.
- The keyed editor components stay mounted; child expansion state and unsaved
  drafts survive and are restored on reopening the channel.
- Already admitted requests are not cancelled, restarted or duplicated by
  collapse. Results/refusals remain available on reopening.
- Retain editing exceptions for filters when a channel is expanded and retain
  flat-view behaviour, stable editor identity, same-device refresh and removal/
  navigation guards.
- Update the old table contract (and ADR-0102 as appropriate) deliberately rather
  than silently contradicting the documented acceptance.
- Add regression tests for multiple children, collapse/reopen, dirty drafts,
  filters, pending requests, success/refusal while hidden and no duplicate writes.
  Add browser visibility coverage; mere text presence is insufficient for mounted
  but hidden rows.

Goal B has ownership of the web surface. Its active B1a/B1b work also changes
`Inspector.tsx`; this triage does not edit that surface or its goal file. No
core, application, storage, schema, protocol or hardware change is required by
this proposal. Implementation and full acceptance remain pending.

## Reporter communication

Acknowledgement comment `6154329863` on issue #16 was read back exactly. Assignment
`KNXBench-Labs` and label `bug` were read back as well. The public plan matches the
approved behaviour above; no fix or release date was promised. Interim workaround:
close the open object editor first, then collapse its channel (the existing pending
request guard still applies). Close the issue only once the actual fix is on main.
