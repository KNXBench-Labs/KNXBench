# Plug-in data adoption

Status: implemented for the application layer and the CLI, and since package P2
for the server and the web app (device panel and command palette), all on
17 October 2026.
Decision: [ADR-0136](../adr/0136-plugin-data-adoption-is-audited-opaque-replacement.md).
Limits: [KL-186](../KNOWN_LIMITATIONS.md#catalogue-kl-186).

## What it does

A manufacturer plug-in editor (for example the econ SmartSensor editor) keeps
its configuration in ETS in two places. KNXBench retains both byte for byte on
import and never interprets or executes them ([ADR-0025](../adr/0025-extension-is-data-not-code.md)):

| Channel | Where | How the device refers to it |
| --- | --- | --- |
| `BinaryData` | `<P-xxxx>/BinaryData/<guid>.dat` | the device's `BinaryData` reference (`DeviceInstance.binary_data`) |
| `ExtraData` | `<P-xxxx>/ExtraData/<n>.<ext>` | a device parameter whose value is the number `<n>` (observed, not a documented ETS rule) |

*Adoption* replaces the destination project's bytes for those entries with the
bytes of the same entries in a source `.knxproj` of the same lineage, for one
chosen device pair. Nothing is decoded, converted or merged; the bytes are
copied verbatim and their SHA-256 is recomputed.

```text
knx adopt-plugin-data <source.knxproj> --project <target.knxdb>
    # lists device pairs (same ETS device id) and why each can or cannot take bytes
knx adopt-plugin-data <source.knxproj> --project <target.knxdb> \
    --source-device <id> --target-device <id>            # preview + confirmation token
knx adopt-plugin-data ... --confirm <token>               # apply
```

Listing and preview write nothing. The token binds the source bytes, the
request, the destination's exact state and its file; any change makes it stale.

## When it refuses

Because the bytes are opaque, only what the model knows can be checked. The
plan refuses, naming the one failed fact, unless the two devices have

1. the same product reference and the same application program reference,
2. the same `BinaryData` references (id and name),
3. the same ExtraData associations (parameter reference and file number),
4. the same group-address links (by communication object number, group address
   value and direction; internal ids are not compared),

and every named entry exists exactly once on both sides. It also refuses when
the destination already holds these bytes (nothing to do). A different
individual address is reported in the preview notes, not refused.

Remapping a blob id or ExtraData number would be a guess about the plug-in's
format, so it is never attempted.

## What stays

- Every other entry of the destination is carried unchanged; the model is not
  touched.
- The bytes a replacement supersedes remain in the project as
  `KnxBenchPluginDataSuperseded` entries under
  `knxbench-native/plugin-data/superseded/<sha256>/…`.
- One `KnxBenchPluginDataAdoption` record per adoption
  (`knxbench-native/plugin-data/adoption-<sha256>.json`) names the source
  archive digest, both devices and each replacement by digest — not the bytes.
- **Rollback is the saved version, not command undo.** The opaque store is
  outside the command undo stack. Saving an adoption records the previous
  saved state (with its bytes) as a project version; restore that version to
  go back.

## Server and web app

Same contract, live project. Every route sits behind the server's normal
session check; nothing below decodes, runs or sends anything.

| Route (`POST`) | Body | Answer |
| --- | --- | --- |
| `/api/project/plugin-data/candidates` | `path` (server-side, from the file picker), optional `password` | `sourceHash`, `persistence` (`native` / `session`) and the pairs, each with `refusal` (text) and `refusalDetail` (stable `code` plus its facts) |
| `/api/project/plugin-data/preview` | `path`, `password`, `sourceHash`, `request` (`sourceDevice`, `targetDevice`), `serverIncarnation`, `projectIncarnation`, `snapshotRevision` | the `preview` and a `confirmationToken`; the server keeps this one plan |
| `/api/project/plugin-data/apply` | `confirmationToken` only | the `preview`, the new `project` tree and `rollbackVersion` (`id`, `label`) |
| `/api/project/plugin-data/cancel` | `confirmationToken` | `cancelled`: whether it dropped the shown plan |

| Situation | Status, `kind` | Effect |
| --- | --- | --- |
| No project open | `422`, `pluginDataTarget` | none |
| Source unreadable (not a ZIP, wrong password, …) | `422`, `pluginDataSource` | none |
| Source bytes differ from the listed `sourceHash` | `409`, `pluginDataSourceChanged` | none |
| Binding fields differ from the live project | `409`, `pluginDataStale` | none |
| Project has no `.knxdb` file yet | `422`, `pluginDataNativeRequired` | none (listing still works) |
| A plan refusal (see *When it refuses*) | `422`, `pluginDataRefused`, plus `refusal` (`code`, `message`, facts) | none |
| Token unknown, replaced by a newer preview, cancelled, or the project changed since the preview | `409`, `pluginDataStale` | none; a matching but stale plan is dropped, never retried |
| The way-back version cannot be written | `422`, `pluginDataRollbackUnavailable` / `409`, `projectHistoryStale` | none |
| Misspelled, unknown or missing body field | `422` | none |

**Apply, in order** (all under the project lock): take the shown plan if the
token matches; check the binding and that the project still equals the plan's
starting state; record the current working state as the project version
*Before plug-in data adoption*; persist the adopted working state with the
undo stack unchanged; publish the opaque store and a new revision. So there is
**no Undo step**, the way back is that version (Project history → restore), and
the superseded bytes stay in the project as well. If the second write fails,
the extra version of the unchanged state remains and nothing else changes.
Decision: [ADR-0136, amendment of 17 October 2026](../adr/0136-plugin-data-adoption-is-audited-opaque-replacement.md).

**Web app.** *Adopt plug-in data…* in the device panel header (pairs for that
device first, *Show pairs for every device* for the rest) and in the command
palette. Steps: optional password → *Choose source project…* (the server's file
picker) → pairs with *Can adopt: n entries differ* or *Cannot adopt:* and the
refusal worded in the UI language → *Preview* (each replaced entry by channel,
archive path, size and both SHA-256 digests; unchanged entries; the notes,
including a different individual address) → explicit consent → *Adopt plug-in
data*. No byte content is shown or decoded. The dialog offers no Undo; it names
the version and opens Project history. A `409` keeps the chosen pair, says that
nothing was adopted and offers *Reload project*; a lost answer is reported as
uncertain and the project is re-read, never re-sent. Closing the dialog or any
project change elsewhere cancels the shown preview on the server. Each finished
step moves focus to its result (pairs, preview, message), so the keyboard stays
inside the dialog. EN and DE copy (German addresses the user with *du*).

Not done: the unsaved-changes indicator follows the model, which adoption does
not touch (the adoption is already in the working journal and survives
reopening); no session-log line (the adoption record in the project is the
audit trail).

## What it does not claim

- That the plug-in, ETS or a device accepts the adopted bytes. Not verified.
- That the source was produced by ETS. A source made with the original ETS6
  libraries outside the ETS application (no licence signature) is accepted
  like any other archive; see its provenance note.
- Any `.knxproj` output: KNXBench writes none ([ADR-0028](../adr/0028-no-knxproj-export.md)).
  The adopted bytes can be read back from the project.

## Tests

- `crates/knx-app/tests/plugin_data_adoption.rs` — 19 synthetic cases: exact
  replacement, retention, record, save/reopen, each refusal, stale apply,
  confirmation token, candidate listing, one stable code per refusal class.
- `apps/knx-server/tests/http_plugin_data_adoption.rs` — 12 HTTP cases: listing
  and preview write nothing; confirmed apply (no undo step, durable before
  published, version restore brings the old bytes back); reopen with and
  without Save; each plan refusal by code; unknown device; stale binding,
  changed source, partial or unknown body fields; a project change, another
  project, a wrong, replaced or cancelled token; session-only project; missing
  project and unreadable source; authentication. Four mutants (no incarnation
  check, no way-back version, no durable write, cancel as no-op) each turn a
  named case red.
- `apps/knx-web/src/pluginData.test.ts`, `PluginDataDialog.test.tsx` — refusal
  wording (EN/DE, every code), error classes, pair filtering, the dialog's
  states (bound preview, token-only apply, session-only, `409` with reload,
  lost answer, cancel on close or change, focus hand-off, device-panel entry).
- `apps/knx-web/e2e/plugin-data-adoption.e2e.ts` — strict intercepted fixture,
  EN/DE at 1440 and 400 px through the command palette and the server file
  picker: readable pair list, digests, consent, token-only apply, no sideways
  overflow; plus Escape after a preview cancels it on the server.
- `apps/knx-cli/tests/plugin_data_cli.rs` — the real binary: listing, preview
  without writes, confirmed apply with a restorable version, wrong token,
  incompatible devices, usage errors.
- `crates/knx-app/tests/ets6_plugin_data_pair.rs`
  (`the_edited_plugin_bytes_can_be_adopted_into_the_unchanged_project`,
  ignored, needs the private corpus): on the real ETS6 export pair exactly one
  device is adoptable, with exactly its two plug-in entries replaced by the
  edited archive's bytes.
