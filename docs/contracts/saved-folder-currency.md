# Saved project folder currency

Status: implemented; acceptance in the
[delivery record](../status/2026-11-07-saved-folder-currency.md).
Scope: WP-06 open item 3, "staleness of the saved tree after edits", as one
conservative read-only marker. Owner decision of 7 November 2026: mark, do not
recompute, and compare against the unchanged original XML without new storage.

## Problem

Saved project folders come from the imported `GroupObjectTree`
([folder contract](communication-object-folders.md)). After KNXBench edits a
device's parameters or object set, that tree still shows the import state.
Whether ETS would rebuild the tree after such a change is not established (no
ETS execution), and KNXBench admits no manufacturer `ShowInComObjectTree`
mapping to rebuild it. Silently presenting the import tree as current would
overstate it.

## Decision

The device detail gains an optional, server-only
`DeviceDetail.project_folder_currency`, set when at least one object of the
device still shows an Assigned saved folder after the existing inactive refusal:

| Value | Meaning |
|---|---|
| `unchanged` | The device's parameter values and object set equal the original import |
| `changedSinceImport` | At least one parameter value or the object set differs from the original import |
| `unknown` + reason | The original could not be read or attributed; no claim either way |

Folder placement itself is never altered, hidden or recomputed by this marker.
The object table shows one notice when the marker is not `unchanged`, saying
that the device has been changed in KNXBench since import and that the folders
shown are still those saved at import. An unknown marker names its reason. A state this build does not know is shown as unknown, never as
current.

## Comparison

- **Original**: the device's retained `ContainerEntry` (exact source member,
  empty locator/name, SHA-256 verified before parsing), read with the importer's
  existing schema-20/21/23 parser (`parse_installation_v21` with the project's
  stored source schema). The device is matched by its exact `DeviceInstance/@Id`;
  zero or several matches are unknown.
- **Parameters**: the multiset of `(RefId, Value)` from `ParameterInstanceRefs`,
  with a missing `Value` normalised to the empty string exactly as the importer
  maps it, against the multiset of the device's current parameter instances
  across all installations (`source.ets_id`, `raw`).
- **Objects**: the multiset of the original `GroupObjectTree` object references
  (the importer's authoritative object list, ADR-0014) against the device's
  current objects' `source.ets_id`.
- Order is ignored. An edit that is later undone returns to `unchanged`. A value
  set back to its original text is `unchanged`; writing an explicit entry equal
  to a product default that the original omitted is `changedSinceImport`
  (conservative: KNXBench does not decide product-default equivalence here).
- Link, name, description, flag, DPT and priority edits do not count: they do not
  change which objects exist or which parameters select them. Module-instance
  arguments are outside this slice; module objects already get no folder.

## Reasons for unknown

`NoRetainedSource`, `AmbiguousSource`, `SourceIntegrity`, `SourceLimit`,
`UnsupportedSchema`, `Unreadable`, `DeviceNotInSource`, `AmbiguousDevice`.
The original member is refused above 64 MiB before hashing or parsing.

## Not admitted

Recomputing folder membership, hiding folders, a parameter-level diff view,
ETS rebuild behaviour, module-argument comparison, any write or bus action.

## Cost

Every device-detail request with a shown saved folder hashes the retained
original member. The parsed baselines of the last member are kept in a
session-only memo keyed by that verified digest, source path and schema, so a
member is parsed once until its bytes change. The memo is derived data, is
never saved and cannot answer for bytes that fail their checksum. Measurements
are in the delivery record.

## Executable obligations

1. Reader: device match by exact id (line, segment, unassigned), zero/duplicate
   devices, missing `Value`, schema refusal, malformed XML.
2. Comparison: unchanged, changed value, added and removed parameter, added and
   removed object, reordering, undo to the original; integrity and attribution
   refusals.
3. Mounted GET device route on an actually imported synthetic project: unchanged
   after import, changed after a real `SetParameterValue` command, unchanged
   after undo; absent when the device has no saved folder.
4. Web: notice for changed and every unknown reason, none for unchanged/absent,
   unknown-state fallback, German text.
