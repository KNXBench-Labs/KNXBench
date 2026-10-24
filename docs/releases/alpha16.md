# Alpha 16 — published pre-release

> **Published 24 October 2026** as `v0.1.0-alpha.16` at immutable
> `f2e3ec6e4b227b49dc74fdba4cdadc3ab91e7c3d`. The normal AppImage/MCP/checksum
> and amd64+arm64 Docker publication workflows succeeded. Downloaded public
> asset sizes/SHA-256/provider digests and both native OCI revisions are verified
> against this tag; version/`latest` shared the verified index at readback.
> [Public verification record](../evidence/release-alpha16-2026-10-24.json).
> Runtime/test acceptance and earlier **local** amd64 packages remain bound to
> `86b4a7fc68b3d3206c00511b14f41e025e33de4d`; their hashes are not CI-built
> download hashes. Extra local ARM64 preacceptance remains deferred. This is a
> pre-release, not a stability or physical-device certification.
> **Final Pages/live website verification of the docs/site successor is pending.**

The Alpha 16 scope connects experimental configured System-2 download,
cancellable catalogue updates, comparison adoption and catalogue-authored native
source retention. The [release plan and evidence matrix](../goals/alpha16-release-plan-2026-10-23.md#current-acceptance-disposition--24-october-2026)
remain the acceptance authority. Historical full-gate and later focused receipts
retain their actual revisions; no full-gate result is relabelled as this successor.
The explicit **24 October 2026 bundled acceptance amendment** accepts historical
Gate9 plus reviewed affected-successor checks/owning builds and final artifact
smokes instead of a same-final-SHA full-gate replay. It supersedes the remote
packaging dry-run criterion with **required actual normal publication on the
final tag and public verification**; no remote dry run is claimed.

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
unverified.** The final authenticated source review accepts the operation-local
recovery-plan reuse correction. Its two new App regressions and the original
exact mounted HTTP/worker recovery scenario pass at the frozen revision above.
The unchanged scenario reaches all 105 assertions: exact original TestHardware
state, reopened `Restored` journal, immediate once-only refusal and refusal after
a second server restart without further connection or mutation. TestHardware is
not a physical KNX device; UI fixture evidence is separate from this mounted proof.

Fresh execution and recovery reuse fully validated immutable plans only within
the original operation. Independent current-source/project admission, exact
consent, target/provenance checks, durable actual-state backup/readback, original
recovery binding and once-only barriers remain intact. This removes duplicate
reconstruction, not a measured general speedup or proof of earlier timeout cause.

The backup covers declared affected regions,
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
COMMIT claim, and standalone/embedded/desktop close-and-drain composition.
Blocking software corrections have source review and affected regression evidence.
One actual mounted Large-profile install also passes preview/apply/commit,
retained-backup disposal/drain checks and scoped resource observation. That input
does **not** exceed the Standard 4 GiB decoded ceiling; this is not a >4 GiB
boundary result, general throughput claim, crash-recovery or rollback guarantee.

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

Application versions are aligned to **0.1.0-alpha.16**. The historical full gate
at `14c9214b68cbe10186c0b7e8b1112c810ce5cff1` passed; later corrections and
mounted System-2/comparison/Large outcomes retain their own focused acceptance
identities in the release plan. There is no aggregate total of overlapping runs.

Accepted package boundaries and remaining delivery scope:

- **Two formerly blocked opt-ins passed:** parent-verified legacy-member-name and
  isolated/shared-order product-matrix checks each returned `ok`/exit 0 at
  documentation HEAD `a9295a52c08fa642c55d33294b5c043bdd98a0b0`, using the owning
  release-mode integration build. The owner authorized restoration of only the
  authenticated historical selection: **111 files** (one non-package file),
  **115 package instances / 113 unique payloads**. Exact archived bytes and mapping
  were preserved; originals, source, user data and historical evidence remained
  unchanged. This limited authorization is not permission for further copies,
  repinning or substitution. The matrix used the required different output
  filesystem; consumers ended and leases were released. The historical 24/26
  receipt remains historical; its remaining two criteria passed separately,
  without an overlapping aggregate total or a new full-gate claim.
- **Local amd64 artifacts accepted:** the parent independently read back all four
  final artifact hashes/sizes for the frozen runtime above and accepted the
  complete/success packaging-verification and final-preservation receipts.
  Earlier packages retain their original revisions; they are not relabelled.
- **Platform decision:** local preacceptance is explicitly **amd64**; extra ARM64
  preacceptance is deferred and unverified, not a current local blocker. The owner
  requests normal Alpha-15-style publication, not an architecture-special release:
  AppImage/MCP/checksums and the unchanged standard amd64+arm64 container workflow.
  Both remote native build lanes and publication now succeed; exact public
  revisions are verified separately below. Local amd64 evidence is not ARM64
  runtime acceptance, and no ARM64 lane is disabled.
- **Final artifact import/upgrade criterion satisfied:** the final standalone
  amd64 server passed the bounded import/save/switch/reopen and native13 upgrade
  routes described below. This is one artifact route, not a source-suite replay
  or certification of every package/architecture.
- **Final delivery:** parent review of the bounded manual/roadmap/contract-status
  reconciliation is complete. The dated bundled amendment retains original
  criteria as history without inventing a same-final-SHA full gate or remote dry
  run. The owner-authorized normal final-tag publication and public artifact,
  checksum, digest and revision criterion is complete. Final Pages deployment
  and live generated-content verification of this docs/site successor, parent
  handover and owned cleanup remain separate; website completion is not claimed.

### Verified public release artifacts

The published [GitHub pre-release](https://github.com/KNXBench-Labs/KNXBench/releases/tag/v0.1.0-alpha.16)
contains the following **CI-built public assets**, independently downloaded and
checked against SHA256SUMS and provider digests. AppImage workflow **40084841192**
and Docker workflow **40084841249** both completed successfully at publication
revision `f2e3ec6e4b227b49dc74fdba4cdadc3ab91e7c3d`.

| Public asset | Bytes | SHA-256 |
| --- | ---: | --- |
| `KNXBench_0.1.0-alpha.16_amd64.AppImage` | 91716088 | `c3c1f26b4e0ab774e060184ee1398f4ce42d656ee4ed42233e9cd73041fe4ea1` |
| `SHA256SUMS` | 194 | `c970f6b62bf704b2e26a5b4ffcc6e180ef53a97955fe3fff3f49bc25eedfa456` |
| `knx-mcp-x86_64-linux` | 13347680 | `d83a2e3ac3c6af8bbe245709ecc09e592cf0dc41fa3674788a5ac9d8a5d3b88e` |

Docker `0.1.0-alpha.16` and `latest` shared index
`sha256:911a71cfae935d0a03a02c593c664b5818820684837102705015ef95009a9bfb`
at readback. Both linux/amd64 and linux/arm64 native configs carry exact revision
`f2e3ec6e4b227b49dc74fdba4cdadc3ab91e7c3d` and version `v0.1.0-alpha.16`.
This is remote manifest/config identity verification, not a public-image runtime,
ARM64 functional or AppImage GUI/FUSE smoke. The downloaded MCP version was
`knx-mcp 0.1.0-alpha.16+gf2e3ec6`, run once in isolated state. Exact native manifest
and config digests and remaining probe limits are in the
[public verification record](../evidence/release-alpha16-2026-10-24.json).

### Parent-verified local amd64 artifact identities

These are **LOCAL** packaging results for `0.1.0-alpha.16` at runtime
`86b4a7fc68b3d3206c00511b14f41e025e33de4d`, **not CI/public download hashes**
or ARM64 evidence. They remain distinct from the published assets above.

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| `KNXBench_0.1.0-alpha.16_amd64.AppImage` | 118675960 | `4df7a14a7294a93451ae38f057d78322247003ac9be1460ec551862e771cd000` |
| `knx-mcp-x86_64-linux` | 14008472 | `00ebf81e016d7b17227c6051cd2479080d8efbb8ca0dedabe9059458c615dad8` |
| `knx-server-x86_64-linux` | 34531216 | `6cbfcfe28c180172eb361c1fbb9904221965ba8cd012e14c0cf7f5f1aaf651cc` |
| Maintained linux/amd64 container archive | 45217280 | `b3ef1b66a004c4d353f81c6f85df335b485e155715e636c8fbadf284261d9b2c` |

The retained minimal smokes cover MCP version/help; extracted AppImage startup,
version API and frontend response; and standalone/container isolated authenticated
health/version plus synthetic project create/save/replacement/reopen. Both saved
stores read back as schema16 with SQLite integrity OK; standalone/container
graceful stops passed. The parent read these smoke receipts, not a second run.
AppImage extraction/Broadway startup does not verify FUSE, native Wayland/EGL or
visual desktop behaviour. Local container archive/config verification is not a
registry push or public digest readback.

The additional logical receipt **Final-artifact import/upgrade / final-receipt**
is complete/success for the same final standalone server identity above, with
**23 checks in one attempt**. Through its isolated authenticated API:

- The committed public synthetic ETS fixture is imported, saved as native16,
  switched away from and reopened. Topology/readback and exact retained fixture
  XML bytes are preserved; this is not broad ETS compatibility.
- An authentic committed native13 predecessor fixture is opened, saved as
  native16, switched away from and reopened. Compared common-column rows match
  across all 14 inspected tables; history counters are observed, not undo/redo
  operation or both-stack working-state proof.
- Both saved stores report SQLite integrity OK. Original/copied fixtures,
  source/runtime inputs, prior artifact evidence, user data and user container
  are unchanged; server exit is 0, owned processes end and leases are released.

This closes the named artifact import/upgrade criterion under the bundled
amendment. It is not a per-AppImage/container replay, browser/native-GUI/FUSE,
physical KNX, network-peer, ETS-parity or ARM64 result. It changes neither the
four local artifact identities nor the separate public release verification
identities above.

This documentation/site successor adds no product tests, builds or smokes. No real-device
trial, ETS/recovery certification, general throughput claim or ledger/issue closure
follows from these software receipts.
