# ADR 0136: Plug-in data adoption is an audited opaque replacement

Date: 2026-10-17
Status: Accepted (CLI and application layer)
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
