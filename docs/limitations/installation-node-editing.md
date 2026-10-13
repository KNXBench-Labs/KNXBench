# Installation-node editing

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-installation-node-editing)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

## Update 2026-10-13 (Goal A2): bound installation renames

`PATCH /api/installations/{id}` accepts the same binding as the device and
group-address renames (`expectedName` plus server/project incarnation and
snapshot revision); stale or changed names are `409 renameConflict` without
effect, a partial binding is `422 revisionBindingIncomplete`
([contract](../contracts/revision-bound-writes.md)). The web Properties editor
still sends the unbound legacy form until Goal B1 migrates it. Renaming to the
current name (after trimming) is now a no-op without an undo step in both
forms, as for devices and group addresses.

<!-- BEGIN retained catalogue record -->

## Local installation-node UI boundary (2026-10-11)

[Fresh main-integration acceptance](../evidence/installation-node-publication-2026-10-11.json)
supersedes local-only availability, not the operation/compatibility bounds below.

[The focused contract](../contracts/installation-node.md) makes unnamed nodes readable
and selectable with command-backed name editing. This is local source, not an
Alpha.8 artifact update. The existing installation rename API remains a legacy,
non-revision-conditional endpoint; ignoring a superseded UI reply does not undo
or cancel a server operation. No installation create/delete/move capability,
fresh private ETS import, ETS export or native screen-reader acceptance is added.

### Recorded device state view boundary (2026-10-11)

[Read-only recorded state](../contracts/recorded-device-state.md) is locally accepted. Persisted project flags and optional model-held reports
are separate; absence is not unloaded or never queried. Reports lack observation
timestamps here, may be stale and are not restored by native reopening. The
existing exact-native journal refuses SaveAs snapshots containing retained session
reports rather than silently losing them; inspection does not clear this state.
An explicit test-only session reset permits the separate intent/date roundtrip. No fresh
hardware check, programmable image, authority or stored-endpoint connection is
established. Future native13, historical observation storage and write workflows
remain separate. Early native admission still depends on another owner's
reconciliation/acceptance; no full matrix or ledger completion is inferred.


### Local modeled project information boundary (2026-10-11)

[Read-only metadata inspection](../contracts/modeled-project-metadata.md) shows only
existing normalized ProjectInfo values; it does not close MODEL-07 or add an
editor for Guid/version/copyright/contract/task/trace/usage/address fields.
Dates are stored UTC model values, not last-successful-save bookkeeping or raw
source spelling. Recorded completion is not new hardware verification or write
authority. Null versus empty remains exact. A cumulative 64-KiB model-text bound
refuses the entire view rather than truncating; model/retained bytes are not
removed. No automatic source parsing, clipboard sharing or network/bus action.
Final local gates pass; integration/publication remains separate.
No ledger status is promoted by this read-only scalar view.


### Local native-export privacy boundary (2026-10-11)

[The reviewed native export dialog](../contracts/native-export-privacy.md) warns that
original/private retained content survives visible renames and compression. It
is consent and snapshot binding, not anonymization or encrypted export. Query-free
authenticated direct clients remain possible; only the web confirmation requires
a complete reviewed binding. A server-side stale conflict fails the browser
transfer and requires fresh review, not automatic retry. Existing native-shell
Save As, report/payload downloads and wider export limits remain separate.
No ledger ID or ETS-return-path limitation is closed by this privacy warning.


### Local retained-payload access boundary (2026-10-11)

[Retained project payload catalogue](../contracts/project-payload-catalogue.md)
provides read-only labels and explicit inert local downloads, not attachment
editing or ETS-App support. Scope is complete bytes actually in native opaque
context; shared product-database assets remain separate. Up to 10,000 records
and 4 MiB cumulative path/kind/hash text are catalogued without partial success;
selected downloads over 64 MiB or with a hash mismatch refuse while bytes remain
retained. Leading-byte classes and path roles are not full validation, signature
verification or authenticity. Downloads can contain foreign private/executable
data. No rendering, extraction, execution, replacement or privacy scrubbing.
Final local acceptance passes; integration/publication remains separate.
No ledger IDs are promoted by this bounded payload reader.


### Local durable import-report boundary (2026-10-11)

Primary server imports now retain the complete original report in native files
([contract](../contracts/native-import-evidence.md), [ADR-0113](../adr/0113-durable-native-import-evidence.md)).
This does not reconstruct reports absent from old files, capture every CLI or
selective-import timeline, recompute current compatibility, verify signatures or
prove hardware support. Envelope/report serialization has a 16-MiB bound;
excessive or corrupt/unsupported evidence is refused explicitly, never truncated
into a successful report. Native saves and complete local JSON downloads can
contain private source data; the bounded preview is not anonymization. Final
owning acceptance/integration remains separate; existing ledger statuses are not
promoted by report persistence.
<!-- END retained catalogue record -->
