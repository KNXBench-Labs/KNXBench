# Alpha 16 — unreleased scope draft

> **Preparation, not a release announcement.** This note describes integrated
> source at `5dc47aeced255a9d3d10e787dd7940082cb0cad1`, compared locally with
> `v0.1.0-alpha.15`. Alpha 15 remains the latest released version. Blocking review
> corrections, frozen-candidate acceptance, version alignment, packaging and
> artifact smokes are still pending. Nothing here claims publication or tag readiness.

The proposed Alpha 16 scope connects experimental configured System-2 download,
cancellable catalogue updates, comparison adoption and catalogue-authored native
source retention. The [release plan](../goals/alpha16-release-plan-2026-10-23.md)
remains the acceptance authority; source integration is not feature acceptance.

## Experimental configured System-2 download

The integrated device-download UI, authenticated HTTP service and shared App/CLI
logic provide a separate experimental path for **UP 258D12 application
`M-0001_A-9205-04-9270` only**. This is not support for every System-2 device,
all `MV-0025` applications, another firmware variant or ETS-equivalent commissioning.

- Preparation uses the selected project's actual parameters, group links and flag
  overrides, including an unsaved project. The UI requires retained original
  sources; unavailable, ambiguous or unsupported sources refuse rather than
  substitute product defaults. Offline preparation does not contact a bus or
  turn a simulator bundle into hardware admission.
- The UI shows configuration/plan identities, counts, ordered steps and residual
  risks. Starting requires ordinary programming consent **and four exact typed
  acknowledgements**: target/plan, experimental hypothesis, ordinary download and
  best-effort release risk. Neither remembered consent nor a client boolean
  replaces them. The recovery-track choice is explicit; external ETS remains
  operator-managed, not invoked or certified by KNXBench.
- Hardware admission checks the exact target and installed application. Fresh
  actual affected prestate must be persisted and read back **before setup as well
  as load controls and data writes**. Product defaults and an earlier identity
  observation are not that backup. A blank device, different application or
  non-Loaded required resource is refused.
- The server retains the real task, original journal and backup across view
  navigation and HTTP loss. Cancellation is cooperative and does not undo prior
  writes. Restart reopens unfinished evidence as interrupted; there is no
  automatic resend, resume or restore.
- Status reattaches the durable selected original. Authenticated HTTP endpoints
  can inventory/select an intact historical operation; there is **no historical
  operation dropdown** in the UI. Recovery uses the original target, backup and
  three separately typed original acknowledgements, not the current project's
  desired configuration. Each operation admits at most one explicit recovery
  command; a lost reply is not permission to repeat it.

**Evidence boundary: experimental, software-tested scope only; hardware outcome
unverified.** Earlier focused software evidence does not establish acceptance of
this integrated UI/server candidate. The backup covers declared affected regions,
not a whole-device/allocator image. Source-shape replay is best-effort: historical
allocator/task commands, masked/unused-byte survival and indirect firmware effects
remain unresolved. Only full required verification can report `Restored`; even
that is not physical recovery certification or a functional hardware check.

[Configured service](../contracts/alpha16-system2-configured-download.md) ·
[UI contract](../contracts/alpha16-system2-ui.md) ·
[Production mount and restart selection](../contracts/alpha16-system2-http-mount.md).

## Catalogue updates owned beyond the picker

The production catalogue/wizard path now mounts the bounded job service over the
existing managed installer, with one App-root controller and one process-owned
manager. Source includes upload → immutable preview → explicit confirmation →
apply, complete paged public reports, cooperative cancellation and retained task
ownership when a wizard or workspace closes. A refusal does not silently choose
a synchronous installer, another profile or different source.

The safety contract distinguishes native COMMIT evidence, cancellation request,
settlement, cleanup/quarantine and recovery-backup retention. A witnessed successful
COMMIT must not become “cancelled” or “rolled back” because later cleanup fails;
an unknown settlement must retain uncertainty and recovery evidence. The source
includes pre-live durable backup publication, full live-result admission before
COMMIT claim, and standalone/embedded/desktop close-and-drain composition. Blocking
review corrections and combined execution are still pending: these mechanisms
are not an accepted crash-recovery or rollback guarantee.

- Standard expansion remains **4 GiB**; explicit large-profile opt-in permits
  **16 GiB**. Source/member bytes remain bounded at **1,000,000,000** each.
- Existing catalogue copy admission is **8 GiB of page bytes**; working main
  databases have a separate **64 GiB** ceiling. An install can leave the catalogue
  above the input bound and cause a later job to refuse. There is no automatic
  budget increase or fallback.
- Reports are bounded and paged; a page is not the whole report. Raw/private
  artifacts are not offered for download. Retained signatures are not
  cryptographically verified.
- Bounds are **not aggregate RSS or disk quotas**, hard cancellation latency,
  universal large-catalogue throughput or power-loss guarantees. Native/XML and
  filesystem spans can temporarily delay cooperative cancellation.
- Browser handles and server report retention are finite, not a durable outcome
  database. Losing status does not prove rollback. Report disposal is separate
  from recovery-backup deletion; retain uncertain/committed backups for inspection.

[Runtime](../contracts/catalogue-jobs/alpha16-runtime.md) ·
[B10 arbitration/evidence](../contracts/catalogue-jobs/alpha16-b10.md) ·
[Process lifetime](../contracts/catalogue-jobs/alpha16-process-lifetime.md) ·
[Root-owned UI](../contracts/catalogue-jobs/alpha16-web-controller.md).

## Device → project adoption, not another download route

A genuine, current, application-checked full comparison can now feed a separate
preview and confirmed project edit. Only **named parameters with exact typed
source attribution and unambiguous decoding** are eligible. Apply rechecks source,
project/native state and undo/redo history; a saved native working state is
persisted before publication. A changed edit is one atomic undoable command.

Raw, masked, unread and unknown rows remain visible but cannot become values.
Address/count tables, individual addresses, group-address and association rows
are not adoptable; no GA/object identity is inferred or created. Unsupported mixed
selections refuse the whole edit. Partial comparisons and System-2 raw-only rows
do not gain adoption authority. Handles expire and can be evicted earlier; a lost
apply reply requires project/history inspection, not automatic reapply.

Adoption does not mark the device Synced or program it. **Project → device** opens
the existing validated download review with the stable device ID; comparison bytes
never become an alternative write plan.
[Exact contract](../contracts/alpha16-comparison-adoption.md).

## Catalogue-authored native source retention

Save and Save As acquire newly needed product dependencies from the installed
**winning source hashes and unique recorded package membership**. The native file
carries the **complete exact original `CataloguePackage`**, plus its required
direct/master/catalogue members, instead of reconstructing a smaller manufacturer
export. The original can contain other products and opaque baggage; those bytes
remain unchanged. On server/desktop open, carried originals use the existing
validated installer before individual source replay, including into an empty
product database.

Missing/corrupt originals or members, ambiguous ownership and inconsistent mappings
refuse new acquisition before destination publication or clean-state updates.
Existing manifests/opaque entries are preserved. Historical missing sources cannot
be invented, and legacy or unmeasured ownership is not inferred. Without a product
database, existing carried sources can be preserved but new dependencies cannot be
acquired. Hash equality is not manufacturer authenticity or signature verification.

**No schema bump does not mean backward compatibility.** Older readers' handling
of these new carried originals has not been verified; do not promise that an older
build can reopen a new save. Retain original packages and independent backups.
[Portability contract](../contracts/alpha16-catalogue-portability.md).

## Other post-Alpha-15 source changes

- **Product database v27/v28:** master declaration provenance and diagnostics,
  followed by promotion repair for source-derived columns/evidence. Historical
  winning-source/install authority that was never recorded remains unavailable;
  this is not reconstructed provenance or broader executable encoding support.
- **Caption language before placeholder filling:** supported block/separator `Text`
  translations use the selected product language, then existing module/positional
  resolution. Explicitly empty translations stay empty. Row/column
  `TextParameterRefId`, broader inline/nested/CollapseIfEmpty rules and in-cell
  buttons are not newly admitted. [Layout slice](../contracts/alpha16-parameter-layout.md).
- **Monitoring snapshot export:** a reduced current snapshot with explicit omission
  diagnostics, not an ETS/native backup or a complete external schema. Receiver
  compatibility is unverified. [Snapshot contract](../contracts/monitor-project-export.md).
- **ZIP integrity:** additional product/project directory and file-stream refusals
  reject trailing or inconsistent members; they do not establish universal ZIP,
  encrypted-package or AES support.

The Alpha-15 System column, differences-first comparison display, catalogue cleanup,
parameter workspace and suffix labels are baseline features, not newly invented
Alpha-16 outcomes.

## Upgrade and remaining acceptance

Inspected source constants are **native/store 16**, **core model 16** and
**product database 28**. Writable catalogue open migrates Alpha 15's product26
through v27 to v28. Alpha 15 cannot read product28; there is no downgrade path.
Read-only CLI readers do not migrate the on-disk catalogue. This is separate from
unverified older-reader compatibility for newly saved native files.

Before upgrading, close all catalogue users and keep independent copies of native
projects, the **closed product catalogue** and persistent deployment data. Keep
matching original `.knxproj`/`.knxprod` files. In-file history and catalogue update
backups do not replace pre-upgrade disaster backups; rollback uses those copies.

Before final release wording, the candidate still needs closed blocking review
findings, a frozen source/version identity, the planned combined software
acceptance, required workflow/opt-in coverage, packaging dry runs and isolated
artifact smokes. This documentation pass runs none of them. Publication requires
separate explicit approval and public artifact verification. No real-device trial,
ETS/recovery certification or issue closure follows from this draft.
