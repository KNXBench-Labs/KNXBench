# Project-local device and group-address naming

## Scope

The project name of one device instance or one individual group address can be
edited in every installation. Imported and newly created objects share the same
mutation path. Product/application/catalogue names, descriptions, numeric
addresses, DPTs, flags, links, placement, IDs and source refs do not change.
No KNX network operation belongs to naming.

[ADR-0101](../adr/0101-project-local-device-and-ga-names.md) records the design.

## Admission and editing

- New names must not be empty/Unicode-whitespace-only and must contain at most
  1,024 Unicode scalar values. C0/C1 controls and U+2028/U+2029 are refused.
- Names remain exact: no trimming, normalization, truncation or uniqueness rule.
  Imported exceptions stay preserved and exactly restorable by undo.
- Enter or leaving an edited field applies once; Escape cancels without a write.
  Invalid/network-failed drafts remain visible with an error and explicit actions.
  Invalid paste is refused before text-input normalization. A pending commit
  cannot be dismissed as though it were an unsubmitted draft.
- A no-op creates no history/revision change and preserves the redo stack.
- Name field: canonical device editor and properties inspector. F2/context-menu:
  explicit device/GA target in project explorer and central tables; no bulk action.
- Pending replies/drafts cannot leak across entity, project or editor-generation
  changes. A read-only refresh retains the draft, but refuses a different server
  or successful-load incarnation and never retries a write automatically.

## Wire contract

PATCH `/api/devices/{id}/name` and `/api/group-addresses/{id}/name` accept exactly:

```json
{
  "name": "  Kitchen light 🛠  ",
  "expectedName": "Old name",
  "serverIncarnation": "opaque-current-server",
  "projectIncarnation": 1,
  "snapshotRevision": 5
}
```

All fields are required; these sample context values are not usable credentials
or a real server snapshot. The success body is the canonical ProjectTree.
Read the actual `server_incarnation`, `project_incarnation`, `snapshot_revision`
from that tree. Conflicts return 409/`renameConflict`, target/name refusal
422/`renameInvalid`; malformed/unknown fields use extractor 422 and failed
native commits return 500. Refused requests do not mutate the project or history.

The application validates context/name while holding the project lock, then
reuses the command stack and native durable mutation. An identical resend with
its old revision is refused, not re-applied. No source-format or storage-schema
migration is introduced by name editing.

## Evidence matrix

| Contract | Check |
| --- | --- |
| Scalar limits, Unicode White_Space, exact admission | `knx-core::names::tests` and `rename.test.ts` |
| Device name-only mutation, imported exceptions, no-op/redo | `device_name_restores_imported_exception_exactly_and_noop_keeps_redo` |
| Second installation, linked GA/DPT/flags/source refs unchanged | `http_rename.rs` full normalized-model equality |
| Ambiguous IDs, malformed/invalid/stale/context requests | `http_rename.rs` refusal and history assertions |
| Native unsaved recovery, saved reopen, exact undo/redo | `native_history_recovers_unsaved_rename_and_exact_original_after_reopen` |
| Enter/blur/Escape/error retention/refresh/late generations | `rename.field.test.tsx` |
| Name field and F2/context-menu entry | `rename.inspector.test.tsx`, `rename.entry.test.tsx`, built browser |
| Actual production UI/server, DE/EN and wide/narrow, conflict/reopen | `e2e/rename.native.ts`, isolated runner |

## Reproduce the native browser check

Build the frontend and server, then run on Linux with user namespaces, `ip`,
Chromium and the repository's installed Playwright available:

```sh
npm --prefix apps/knx-web ci
npm --prefix apps/knx-web run build
cargo build -p knx-server
python3 tools/verify_name_editing.py --server target/debug/knx-server --scratch "$TMPDIR/name-native"
```

For Chromium keep TMPDIR short and task-owned. The runner starts its own
loopback-only network namespace, rejects any extra interface, uses independent
HOME/XDG/project storage, and stops its own server on success or failure. It
never attaches to an installed server or real KNX interface. Review screenshots
and the exact result count; a successful command is not a substitute for the
named assertions. The default intercepted Playwright fixture suite does not
collect this explicitly isolated `.native.ts` spec.

## Retained limits

No bulk rename/new CLI/MCP write/.knxproj export. Before native Save As history is
session-local. Existing native history bounds and save/autosave behavior apply;
versions share the project file. Browser checks are not native shell/Orca,
private-corpus equivalence, KNX hardware verification or ETS compatibility.
Measured local acceptance belongs to the delivery receipt, not this contract.
