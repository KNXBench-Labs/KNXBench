# ADR 0136: Plug-in data adoption is an audited opaque replacement

Date: 2026-10-17
Status: Accepted (CLI and application layer; server and web app by the amendment below)
Session: 7

## Context

KNXBench keeps a manufacturer plug-in's stored configuration (`BinaryData`
blobs and `ExtraData` tables) as opaque bytes and never executes the plug-in
([ADR-0025](0025-extension-is-data-not-code.md)). A real ETS6 pair showed the
need to move such bytes between projects: after one text was changed in the
SmartSensor editor, exactly two entries differ between the archives, and the
destination project should be able to take the newer bytes without importing
anything else ([contract](../contracts/plugin-data-adoption.md)).

Three facts shape the design. The bytes have no model: only the references
around them (product/application refs, blob ids, parameter values, group
links) can be compared. Selective import ([ADR-0106](0106-selective-project-import.md))
only ever *adds* opaque entries; replacing an entry's bytes is new. And command
undo restores the `Project` only — the opaque store is shared context that every
undo state is bound to — so an undo would silently keep the new bytes.

## Decision

1. **Data only, verbatim.** Adoption copies bytes and recomputes the digest. It
   decodes nothing and has no plug-in host.
2. **Refuse on what is checkable.** Same product and application reference, same
   blob references, same ExtraData associations and same group links, else
   refuse naming the failed fact. No remapping of ids or file numbers.
3. **Nothing is discarded.** Superseded bytes are retained as opaque entries
   and each adoption writes a digest-only record.
4. **Rollback is the saved version.** The operation does not pretend to be a
   command; it is atomic (one snapshot swap behind a stale check and a bound
   confirmation token) and saving records the previous saved state as a
   restorable version.
5. **Layering.** The rules live in `knx-app` (`plugin_data`); the CLI is an
   adapter. No `knx-core` change and no schema change.

## Alternatives considered

**Make it a `Command` with undo.** Would need opaque bytes inside the undo
states (history images are bound to one retained opaque context) and a
serialisable form of the command; both are larger changes than this decision
warrants, and neither is needed for the stated use.
**Extend selective import with a "refresh plug-in data" mode.** Selective import
adds entities and maps ids; this replaces bytes of existing ones. Mixing the two
would blur the conflict semantics that import refuses to guess.
**Remap blob ids / file numbers when they differ.** Would be a guess about a
format we deliberately do not interpret.

## Consequences

Easier: the edited plug-in data of a device reaches the working project with a
reviewable preview, an audit record and a way back. Harder: the plug-in bytes
are not part of Undo/Redo; the server and the web UI do not offer the operation
yet ([KL-186](../KNOWN_LIMITATIONS.md#catalogue-kl-186)). Not claimed: that any
plug-in or device accepts the adopted bytes.

## Amendment, 2026-10-17 — the server route and the web app

Package P2 brings the operation to `knx-server` and the web app
([contract](../contracts/plugin-data-adoption.md#server-and-web-app)). A live
server differs from the CLI in three ways: several clients share one open
project, the project may never have been saved, and Undo is one click away.
Decision 4 above ("rollback is the saved version") is kept and made
unconditional:

1. **A native project is required.** Preview and apply refuse a project that
   has no `.knxdb` file yet (`pluginDataNativeRequired`). Without a file there
   is no version store, so there would be no way back. Listing still works.
2. **The way back is created before the change.** Apply first records the
   current working state as the named project version *Before plug-in data
   adoption*, then persists the adopted working state. Both use the existing
   native history services (`change_history`, `persist_working`); the command
   undo stack is carried unchanged, so **no Undo step exists** and the UI does
   not offer one. It names the version instead. If the version cannot be
   written (history limit, another writer), nothing is adopted. If the second
   write fails, the extra version of the unchanged state remains (harmless,
   nothing lost) and nothing is published.
3. **What the user saw is what is applied.** The server keeps the one plan it
   last previewed, the way `device_download_plan` keeps a shown download
   plan. Apply sends only the confirmation token; the server applies that
   exact plan if the token matches, the binding (`serverIncarnation`,
   `projectIncarnation`, `snapshotRevision`) is still current and the
   project still equals the plan's starting state. Otherwise `409`
   `pluginDataStale`, nothing written; the plan is dropped and the client
   previews again. A newer preview replaces the plan; cancel drops it. At most
   one plan is held per server, so a preview from a second client makes the
   first client's token stale (safe, one more click).
4. **Refusals keep their name.** `knx_app::plugin_data::Refusal` gains a
   stable `code` and its facts (`RefusalDetail`), so the web can word every
   class in each language; the English text is unchanged for the CLI.

Not done, deliberately: making adoption a `Command` (would need the opaque
bytes inside the history images, see Alternatives), marking the project as
modified (the indicator follows the model, which adoption does not touch; the
adoption is already persisted in the working journal and reopens with it), and
a session-log line (the adoption record in the project is the audit trail).
