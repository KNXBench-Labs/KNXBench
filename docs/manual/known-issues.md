← Previous: [FAQ](reference/04-faq.md) · [Manual index](README.md)

# Known issues

**Current-source review: 8 October 2026, `608a204b`.** This page separates
remaining defects, intentional safety/scope boundaries and missing evidence.
Historical titles do not mean that their old defect is still present. The
[source/test audit](../status/2026-10-08-known-issues-status-audit.md) records
the reconciliation; formal task dispositions remain in the engineering ledger.

## Settings center page — source newer than Alpha 18

The [settings workspace](../contracts/settings-workspace.md) is tested with
mocked Chromium APIs in EN/DE at wide/narrow sizes and supported zoom bounds.
This does not establish native WebKitGTK/Tauri rendering or hardware acceptance.
At 400px and 150% zoom a native single-line select may abbreviate its option
text; its accessible label and control remain reachable. Published Alpha 18
retains the older modal Settings window; released screenshots are not replaced
by current-source captures.

## DIAG-03 candidate — private comparison export is incomplete

The local diagnostic-bundle candidate is not a release or full-package approval.
The saved-project opt-in copy removes recognized credential fields and reports
withheld data, but is not anonymous. The existing wire redactor handles named
APDU key services; it does not prove arbitrary expected/readback memory or every
overlapping recording/error entry credential-free. Full private System-2 byte
export is not ready; consent does not waive key/password exclusion. A future
metadata-only withheld file would not certify the rest of the ZIP.
[Source-grounded limits](../research/2026-10-29-system2-bundle-credential-boundary.md) ·
[Current work](../open-work/DIAG-03-unified-diagnostic-bundle.md).

## Monitoring export profile (source delivered, 21 October 2026)

The [monitoring ZIP](user-guide/08-reports-and-diff.md#monitoring-export) is a
reduced current snapshot, not a native/ETS backup or a complete external schema.
It omits unknown group-security and object-flag fields with diagnostics instead
of inventing `false`; addressless devices and their objects are counted as omitted.
Manufacturer channel/module metadata and engineering-only data are not projected.
Receiving applications need a separately verified sparse JSON adapter. The CLI
file reader supports quiescent standalone rollback-layout stores only, refusing
WAL mode and any WAL/SHM/journal entries before opening either input. Do not delete
sidecars to bypass the refusal; see [CLI input handling](user-guide/10-command-line.md#monitor-export).
Source delivery and internal runtime checks are complete on `4eee65d8`; a new
release is not implied. No receiver/bus compatibility is claimed.

## Alpha 18 — pre-release scope and evidence limits

Alpha 18 (`v0.1.0-alpha.18`) is a pre-release with source cutoff `bb2d8e51e`
(28 October 2026). Its System-2, tunnel and report behaviour is tested with simulated or
loopback transports only: no physical-hardware certification, recovery guarantee or
ETS equivalence follows. System-2 stays experimental and limited to UP 258D12
application `M-0001_A-9205-04-9270`. The actual allocation mismatch behind issues #30
and #22 is unresolved and both stay open; issue #31 is closed for the display and
report gap only. The final-step category names the operation that stopped, not its
cause. The invisible-parameter reading (defaults profile v3) is an
observation-supported hypothesis, not a defined rule. S2-02 and the field
verification of the tunnel changes (S2-04) stay open. Group-address CSV profiles make
no interoperability claim with any particular application.
The product database moves 28 → 30 and Alpha 17 refuses it; opening Alpha 18 files in
Alpha 17 is not tested and there is no downgrade path.
[Release scope and boundaries](../releases/alpha18.md#boundaries).

<a id="alpha-17--published-pre-release-scope-and-evidence-limits"></a>

## Alpha 17 — previous published pre-release scope and evidence limits

Alpha 17 (`v0.1.0-alpha.17`, source cutoff `20ab2f7ea`) is published as a
pre-release on 26 October 2026. Its new download, System-2 and diagnostic
behaviour is tested with simulated or loopback transports only: no
physical-hardware certification, recovery guarantee or ETS equivalence follows.
System-2 stays experimental and limited to UP 258D12 application
`M-0001_A-9205-04-9270`; masked-byte survival, allocator state and complete
physical recovery remain open, and `gatewayBusy` does not identify its mechanism.
Issue #30 and issue #22 stay open; S2-02 (the original start failure) remains
open work. S2-03's two image-interpretation differences were decided after
Alpha 17 and ship in Alpha 18: the invisible-parameter reading is an observation-supported
hypothesis, not a defined rule. An unproved hardware
route is capped at 12 data octets per request, which can slow transfers.
Device backups (format 2), new activity-history rows and new System-2 operation
records are refused by Alpha 16; opening Alpha 17 saves in Alpha 16 is not tested.
Public artifacts are verified against the immutable tag; that verification is not a
FUSE launch, interactive GUI, hardware or local ARM64 run
([public verification record](../evidence/release-alpha17-2026-10-26.json)).
[Release scope and boundaries](../releases/alpha17.md#boundaries).

<a id="up258-system-2-candidate-source-20-october-2026"></a>

## Alpha 16 — published software scope and remaining evidence limits

**24 October 2026 snapshot:** bounded software outcomes and four final local amd64
artifacts have parent acceptance at frozen runtime/test source
`86b4a7fc68b3d3206c00511b14f41e025e33de4d`; local amd64 software/package acceptance
is complete. Alpha 16 is published at immutable
`f2e3ec6e4b227b49dc74fdba4cdadc3ab91e7c3d`; CI-built public download hashes
remain distinct from the earlier local artifacts. Both formerly blocked pinned
criteria passed at documentation HEAD `a9295a52` using the owning release-mode
integration build. The owner authorized only exact historical restoration of
111 files (one non-package), 115 instances / 113 unique payloads; parent verified
results, mapping and unchanged original/input/archive/source/user/history guards.
No further materialization or repinning is authorized. The owner explicitly limits
local preacceptance to amd64; extra local ARM64 preacceptance is deferred and
unverified, not a blocker. Normal release channels and the standard amd64+arm64
container workflow remain unchanged; both actual remote workflows now succeed
and public asset/OCI identities are separately verified.
The named final standalone artifact import/upgrade smoke now passes: public
synthetic ETS import/save/switch/reopen preserves topology and retained XML bytes;
authentic committed native13 upgrade/save16/switch/reopen preserves compared
common-column rows across 14 tables. Both saved stores have SQLite integrity OK;
23 checks pass in one attempt with unchanged originals/protected data and ended
processes/released leases. History-counter observation is not undo/redo-operation
proof; this is not per-package/browser/hardware/ARM64 acceptance or ETS parity.
Parent documentation review is complete. The explicit 24 October bundled
amendment accepts historical Gate9 plus reviewed affected-successor checks/owning
builds and final artifact smokes, not a same-final-SHA full gate. It supersedes
the remote dry-run criterion with required actual normal final-tag publication
and public verification; no remote dry run is claimed. Owner-authorized final-tag
publication and public asset/checksum/digest/revision readback are complete,
including native Docker amd64/arm64 revisions. This is not a public-container
runtime, ARM64 functional or AppImage GUI/FUSE probe.
[Public verification record](../evidence/release-alpha16-2026-10-24.json). Final
Pages deployment and live HTTP byte verification passed at docs/site revision
`83376509`: 40/40 files and 4/4 aliases matched, with no exclusions. This is not
independent browser-rendering acceptance. The older dated inventory below is not a new audit or
closure of every limitation.
[Source-bound evidence matrix](../goals/alpha16-release-plan-2026-10-23.md#current-acceptance-disposition--24-october-2026),
[published scope and separate public/local artifact identities](../releases/alpha16.md).

### Exact experimental System-2 path, not general device support

The separate [complete-defaults validation workflow](../contracts/up258-system2-validation.md)
remains simulator-only. Current source also mounts a distinct **configured**
UI/HTTP/App/CLI path for UP 258D12 application `M-0001_A-9205-04-9270` only.
It uses actual project parameters, group links and flags; missing retained sources
refuse rather than substitute defaults. Ordinary consent plus four exact typed
phrases are required. This is **experimental, software-tested scope only;
hardware outcome unverified**, not all-System-2 support or ETS certification.

Fresh actual affected prestate must be persisted/read back before setup and every
later mutation. It is not a whole-device/allocator backup: historical allocator
commands, masked/unused-byte survival and indirect firmware effects remain
unresolved. Original-state recovery is explicit, once-only and best-effort;
`Restored` needs full required verification, not merely transport success.
Cancellation or HTTP loss does not mean nothing was written. Retain original
private journals/bundles; do not automatically retry, resume or restore.

The UI reattaches the durable selected original and uses its original recovery
acknowledgements, not the current project. Historical inventory/selection is
available through authenticated HTTP, **not a historical UI dropdown**. Missing
or corrupt selection/evidence refuses; a backup-available label does not promise
physical restorability. The final mounted HTTP/worker/store scenario reaches all
105 assertions, including exact original TestHardware state, reopened `Restored`
journal and immediate/second-restart once-only refusal without further
connection/mutation. UI fixture evidence is separate; neither is physical recovery
or ETS parity. Operation-local plan reuse preserves independent admission,
backup/readback and recovery-integrity barriers; no general speedup is established.
[Exact service and recovery limits](../contracts/alpha16-system2-configured-download.md),
[restart selection](../contracts/alpha16-system2-http-mount.md).

<a id="system-2-safe-failure-diagnostic-unavailable-for-older-and-untyped-failures-26-october-2026-source"></a>

### System-2 safe failure diagnostic: unavailable for older and untyped failures (since Alpha 17)

- **Affected:** the typed *Failure diagnostic* of a refused System-2 table-pointer
  (PID 7) answer, since Alpha 17 (S2-01 projection, `536e492bc`).
- **Limitation:** only a refusal typed at the read carries it. Older journals,
  legacy-policy refusals and every other failure show *unavailable*; nothing is
  reconstructed from the private text. The diagnostic lists observed counts, which
  can be 0, never the raw answer, and a count does not say which firmware or
  interpretation is right. A client that requires the old exact status version
  refuses the new status.
- **Consequence:** an observation is not support: it grants no retry, hardware
  claim or recovery. Objects 2/3 stay source-declared hypotheses. Physical outcome
  and recovery remain unverified; the private journal keeps the raw evidence.
- **Surrounding copy:** the new diagnostic entries are translated EN/DE, but the
  existing static retained-operation status paragraph ("Original retained operation; no
  automatic resume or retry. ...") is inherited English text and still shows in English
  in the German panel alongside the diagnostic section. The panel is not fully translated.
- **Details:** [safe failure diagnostic](../contracts/system2-safe-failure-diagnostic.md),
  [S2-01](../open-work/S2-01-pid7-raw-response.md).

<a id="system-2-run-result-and-project-marks-issue-31"></a>

### System-2 run result and project marks (issue #31, since Alpha 18)

- **What happens:** an experimental System-2 run that ends with the transfer complete and
  its final verification step unfinished is explained in the panel, the device table and a
  saveable report. When a run may have written, the project's Application, Parameters and
  Group-address marks become pending; only a fully verified run, with the project and
  retained source unchanged since the start, marks them programmed. Any edit that changes
  the preparation leaves them pending. These marks are the project's belief, not a device
  read or a compatibility claim.
- **Not known:** why the last step stopped. Since Alpha 18 the panel and report say
  which operation stopped (a comparison disagreed, a read gave no usable answer or the
  final write did not complete) when it could be classified; older operations and
  unclassified stops (for example a journal failure or a cancellation) show
  *unavailable*, and a classified operation is not a cause. The device is never shown
  as loaded, and the actual allocation mismatch of issues #30/#22 is not diagnosed.
- **Table scope:** only the selected operation of the current server session and the
  open project appears there; after a server restart it stays in the panel and report.
- **Details:** [System-2 download result](../contracts/system2-download-result.md),
  [S2-05](../open-work/S2-05-result-and-settlement.md).

### Catalogue jobs: finite resources and uncertain outcomes

The integrated picker/wizard uses the process-owned job service and App-root
tracking, not a component-owned synchronous install. Leaving a view does not
cancel the worker; cancellation is cooperative and may wait on native/XML or
filesystem spans. A lost reply or forgotten/expired handle does not prove rollback.
Witnessed COMMIT, unknown settlement, cleanup/quarantine and backup retention
must remain separate. Blocking corrections and one actual mounted Large-profile
outcome have source-bound software acceptance; the measured input did not exceed
Standard's 4 GiB decoded ceiling. Report disposal is not recovery-backup deletion. Preserve retained
committed/uncertain backups for inspection rather than assuming cleanup recovered
the catalogue.

Standard/large expansion remains 4/16 GiB with explicit large opt-in; source and
member bytes are each bounded at 1,000,000,000. The existing-catalogue copy bound
is 8 GiB of page bytes, separate from the 64 GiB working-main limit. A successful
update can grow beyond the next job's input bound. These are not aggregate RSS,
disk, hard cancellation-latency, throughput or power-loss guarantees. Browser
handles/report retention are finite and do not provide durable outcome history.
[Runtime boundaries](../contracts/catalogue-jobs/alpha16-runtime.md).

### Comparison adoption is named-parameter-only

Only a current, application-checked full observation with exact typed attribution
can supply a separately previewed and confirmed project edit. Tables, individual
addresses, GA/associations, raw, masked, unknown and unread rows cannot be adopted;
a mixed unsupported selection refuses as a whole. Partial comparisons and
System-2 raw-only results do not grant adoption authority. One changed edit is
undoable as a whole; it neither sets Synced nor programs the device. Lost apply
responses require project/history inspection, not automatic reapply.
[Adoption scope](../contracts/alpha16-comparison-adoption.md).

### Catalogue-authored portability is conditional, not universal

Save/Save As now acquire new dependencies only from measured winning sources
with one recorded owning package. The **complete exact original
`CataloguePackage`** is carried with required members, including opaque content,
not a newly reconstructed export. Server/desktop open replays originals through
the normal installer before individual sources. Missing/corrupt/ambiguous new
ownership refuses before publication; existing legacy manifests are preserved
with explicit unavailable-source diagnostics. Missing historical originals cannot
be invented. Keep matching original packages and independent backups. **Older
readers are untested despite unchanged native/model schema 16.**
[Portability boundary](../contracts/alpha16-catalogue-portability.md).

Current product schema is **28**: writable opens migrate Alpha 15's product26
through v27/v28 provenance/promotion repair. The old v23–v25 promotion-residue note
below describes the Alpha-15 baseline, not the current repair boundary. Historical
winning/install authority that was never recorded remains unavailable. Alpha 15
refuses product28; preserve a separate closed-catalogue backup before upgrade.

## Parameter layout (Alpha 14)

The new [parameter workspace](user-guide/05-devices-and-products.md#parameters)
uses declared groups, compact rows and cross-area search. It does not reproduce
all manufacturer grid coordinates or infer a physical button/channel arrangement.
Missing grouping stays visible under Other parameters; unsupported value/script
semantics still have their existing diagnostic/refusal behavior. Rows declared as
indented are indented; since Alpha 15 declared `SuffixText` units appear after
the value and supported caption placeholders resolve with readable fallbacks.
Inline sections and object folders are not shown yet. Alpha 13 and older binaries do not contain this layout.

## Read these before your first serious project

| Boundary | Practical meaning | Next check |
| --- | --- | --- |
| Import is one-way | Keep the original ETS archive; your edited working copy is `.knxdb`, not an ETS export | [Projects](user-guide/02-projects.md#there-is-no-knxproj-export) |
| Native history is not an independent disaster backup | Current releases keep versions inside the same file; alpha.6 had no persistent history. Keep independent copies | [Project history](user-guide/02-projects.md#project-history-restart-safe-undo-and-versions) |
| A new save is not automatically self-contained | Since Alpha 16, Save/Save As acquires complete exact original packages only for uniquely measured catalogue dependencies. Missing/corrupt/ambiguous new ownership refuses; historical missing bytes are not invented. Older readers remain untested despite schema16; keep originals and independent backups | [Current source ownership](../contracts/alpha16-catalogue-portability.md) |
| Browser files live on the server | Save is not a download; Export project gives you a local copy | [Web and desktop differences](user-guide/11-web-and-docker.md#web-build-versus-desktop-build) |
| Shared server, shared project | One password and one undo stack are not collaborative editing | [Deployment](user-guide/11-web-and-docker.md#authentication) |
| Programming marks are project belief (P3, Alpha 12) | Adr/App/Par/GA track edits and server writes, not a live comparison. Recovered snapshot undo after a later write can conservatively show additional pending parts. Reconcile external/CLI programming manually; resolve refresh/storage warnings before trusting flags | [Programming marks](user-guide/05-devices-and-products.md#programming-marks-p3-since-alpha-12) |
| Device comparison is a session observation (P4, Alpha 12) | Relevant edits invalidate tags, but external device changes are not monitored. The verified 0701 tables and named parameter decoder preserve all raw bytes. Named fields need exact active-write attribution and complete coverage; overwritten/masked fields, unknown encodings and negative signed device patterns remain Unknown with a reason. Other table profiles/new image types stay unsupported. Cancel drains the current read. | [Comparison scope](../contracts/device-comparison.md) |
| Hardware support is narrow | A project edit is not commissioning, and undo does not reverse a bus write | [Bus boundaries](user-guide/07-bus-and-interfaces.md#what-knxbench-does-and-does-not-do-on-a-bus) |

This chapter collects the limitations most likely to be noticed by someone
who actually uses KNXBench, grouped by the part of the application they show
up in. Each entry says what is affected, what the limitation is, what it
means in practice, and whether a workaround exists.

It is a selection, not the catalogue. The full engineering record lives in
[`docs/KNOWN_LIMITATIONS.md`](../KNOWN_LIMITATIONS.md), including entries that
only a maintainer would care about, and ones about the reasoning behind a
design decision rather than about a defect. Where an entry below has a
counterpart there, the **Details** line links straight to it.

The following is the **Alpha-15 historical baseline**, not the current Alpha 16/17
boundary described above. [Alpha 15](../releases/alpha15.md) fixes captions and empty
topology cards, explains unavailable catalogues and adds previewed placement
cleanup, byte-level comparison marks and product-declared System labels, without
expanding programming admission. Historical measurements below retain their
original source dates, not new hardware verification. Native/model **16** stays;
product **24 → 26** migrates on writable open, and older builds refuse product26
by their version check. Opening new project saves in Alpha 14 is **not verified**,
particularly at larger byte bounds. Independently back up projects and the whole
Docker data directory, including the catalogue; rollback needs those copies.

**Migration boundary:** v23–v25 promotions still have repeated-program and
evidence-retirement residues; v26 corrects these patterns for its own suffix
promotion, not earlier columns. Legacy EX-IM has no suffix mapping; stored
`Suffix` remains undisplayed. The System-2 declared-memory diagnostic profile is
CLI-only with offline/simulator evidence, not a hardware-verified backup, restore
basis or programming path. The web diagnostic profile stays identity-only in
Alpha 15 and 16; since Alpha 17 the web dialog offers the memory profile too,
simulator-verified only.
[Exact limits](../releases/alpha15.md#still-alpha-explicit-boundaries).

> A long list of known issues is what happens when a project writes its
> problems down, not proof that it has more of them than software that
> writes nothing down. Judge the entries, not their number.

---

## Parameter-driven object refresh (Alpha 13)

Supported manufacturer-mode choices now refresh the object list and append missing
unscoped objects without replacing existing links or overrides. A new active
module object still needs verified instance/number mapping: the whole parameter
edit is refused rather than partially saved. Manufacturer scripts and unsupported
evaluator constructs remain unsupported. [Use and limits](user-guide/05-devices-and-products.md#device-functions-since-alpha-13).

## Projects and import

### An empty default line is reported as an import error

- **Affected:** opening an ETS project file.
- **Limitation:** if the project's `Installation` element carries a
  `DefaultLine` attribute that is present but empty, the import report
  records it as an error — an unresolved reference — rather than as "not
  set". Measured on a real vendor demo project, which imports with exactly
  one error and nothing else:
  `UnresolvedReference { kind: "Installation/@DefaultLine", target: "" }`.
- **Consequence:** the import report looks worse than the import went. The
  import completes, but the empty default-line reference is not resolved. In the measured
  case all 2 areas, 2 lines, 4 devices, 13 group addresses and 75
  communication objects arrived.
- **Workaround:** read the error, confirm it is this one, and continue. There
  is no missing line to invent. Review other diagnostics independently; an
  error-bearing `.knxproj` is refused as input to project comparison.

### ETS 5.7 projects: small evidence, and nested sub-modules are skipped

- **Affected:** projects saved by ETS 5.7 (project schema 20).
- **Limitation:** they import since 12 October 2026, checked against three
  small public sample projects only. Communication objects inside nested
  sub-modules keep their group-address links but lose their object number
  and module; each is listed as an import error.
  ETS 4 schema 12, ETS 5 up to 5.6 and ETS 6.1 projects are still refused.
- **Workaround:** read the import report. For a refused ETS 6.1 project,
  re-exporting from ETS 6.3 gives a supported file.
- **Details:** [§177](../KNOWN_LIMITATIONS.md#177-ets-57-projects-schema-20-rest-on-three-public-samples-sub-module-object-ids-are-not-decoded)

### Everything KNXBench knows about `.knxproj` comes from a small corpus

- **Affected:** import of any project not resembling the ones this project has
  measured.
- **Limitation:** the parser was built against a handful of real files (an
  ETS4 schema-11 project, an ETS 6.3.0 schema-23 project, a vendor schema-21
  demo). Authoritative XSD validation is unavailable to this project;
  structural validation and sample evidence are not complete format conformance.
- **Consequence:** a construct that never appeared in those files may be
  reported as unknown, or mapped conservatively, even though ETS considers it
  ordinary.
- **Workaround:** inspect the report. Opaque source data is retained where
  technically possible; unsupported, malformed and lost mappings are reported.
  Retained bytes are not proof that the application understands or can edit them.
- **Details:** [§1 single-sample bias](../KNOWN_LIMITATIONS.md#1-single-sample-bias),
  [§2 no authoritative XSD](../KNOWN_LIMITATIONS.md#2-no-authoritative-xsd-is-publicly-available)

### AES-protected ETS6 project files are refused

- **Affected:** password-protected `.knxproj` files.
- **Limitation:** the AES protection ETS6 writes is refused outright, with a
  message naming the cipher. The older ZipCrypto protection used by ETS4 and
  ETS5 is implemented and tested against synthetic fixtures, but has never
  been run against a real password-protected ETS export.
- **Consequence:** an ETS6 project saved with a password cannot be opened at
  all.
- **Workaround:** re-save the project from ETS without a password and import
  that copy.
- **Details:** [§13 password-protected projects](../KNOWN_LIMITATIONS.md#13-password-protected-projects-zipcrypto-ets4ets5-is-decrypted-aes-ets6-is-still-refused)

### There is no partial or selective import

**Historical heading retained for links; amended 10 October 2026.** Source
integration has **File → Import selected lines/devices…** and the matching CLI.
[Separate acceptance](../status/2026-10-10-import-expansion-integration.md) is not
a released-artifact or real ETS/hardware claim.

- **Affected:** merging a subsystem into the currently open project.
- **Implemented scope:** choose explicit source/target installations and devices
  or complete lines, inspect dependencies/mappings, preview, then consent. One
  import is one undo step; existing devices/addresses are not silently replaced.
- **Retained boundary:** no building-only selector, automatic address reassignment,
  conflict overwrite or new command-palette entry. Native snapshot limits apply;
  future model versions need a reference audit. Full source evidence can include
  unselected private data and remains even after Undo.
- **Next evidence:** authorized historical ETS revision exports, native/Orca and
  real ETS/device acceptance are separate from synthetic browser/CLI checks.
- **Details:** [Selective import](../SELECTIVE_IMPORT.md),
  [local verification](../status/2026-10-10-import-expansion-verification.md).

## Export

### `.knxproj` export was withdrawn on 2026-09-20

- **Affected:** anyone who expected to write an ETS file out of KNXBench.
- **Limitation:** KNXBench reads a `.knxproj` and never writes one. The
  exporter, the `knx export` subcommand, the `POST /api/project/export` route
  and the "Export to .knxproj…" button were all removed
  ([ADR-0028](../adr/0028-no-knxproj-export.md)). Once a project has been
  imported it stays in KNXBench's own `.knxdb` format.
- **Consequence:** this section used to list four separate export defects — an
  unsigned container no ETS had ever accepted, a round trip that was semantic
  rather than byte-exact, an unexported Read-on-init flag, and a handful of
  schema-≥21 attributes that were not written back. All four are closed for
  the same reason: there is nothing writing a `.knxproj` for them to go wrong
  in. The underlying entries are kept as history in
  [§4](../KNOWN_LIMITATIONS.md#4-round-trips-are-semantic-not-byte-exact),
  [§5](../KNOWN_LIMITATIONS.md#5-exports-are-unsigned-and-ets-acceptance-is-untested),
  [§34](../KNOWN_LIMITATIONS.md#34-schema-21-export-drops-a-handful-of-known-but-unmapped-per-deviceper-line-attributes--resolved-2026-09-20)
  and
  [§117](../KNOWN_LIMITATIONS.md#117-read_on_init_flag-is-parsed-and-stored-then-discarded-before-it-reaches-knx-core).
- **Workaround:** keep the `.knxproj` you imported — KNXBench never changed it
  — and use the group-address CSV or the HTML project documentation to hand
  data to other tools.
- **Details:** [ADR-0028](../adr/0028-no-knxproj-export.md),
  [`docs/COMPATIBILITY.md` §4](../COMPATIBILITY.md#4-not-supported)

## Devices and product data

### The catalog file picker offers `.vd2`, and `.vd2` is always rejected

**UI offer corrected.** This historical heading is retained for links. The
current shared catalog/wizard picker offers `.knxprod`, ZIP and
`.vd3`/`.vd4`/`.vd5`, not `.vd2`. The CLI `products ingest` help still names
`.vd2`, but its importer refuses it. That help text, not the web picker,
is the remaining copy defect.

- **Affected:** installing a product database.
- **Limitation:** the package reader rejects every file whose name
  ends in `.vd2` before looking inside it — that legacy format is out of
  scope by decision, not by accident.
- **Consequence:** a file selected by overriding the operating system's filter
  can still be refused. A picker filter is guidance, not format validation.
- **Workaround:** obtain the product as a `.knxprod` package.
- **Details:** [`docs/COMPATIBILITY.md`](../COMPATIBILITY.md),
  [UI follow-up evidence](https://github.com/KNXBench-Labs/KNXBench/blob/6222e17754f3/docs/archive/alpha-0.1/UI_ALPHA_READINESS.md)

### Some newer `.knxprod` master-data schemes are not installable directly

- **Affected:** product data for newer devices.
- **Limitation:** a standalone `.knxprod` package installs at master-data
  scheme 10 (ETS4 era), 11, 12, 13, 14, 20 or exact-namespace 21 and 23.
  Schemes 10, 12-14, 21 and 23 are covered by synthetic tests and corpus
  measurements, but that proves KNXBench parser/persistence behavior rather
  than complete semantics or ETS parity. The unobserved schemes 15-19, 22 and
  24 are not supported. Encrypted packages are refused
  permanently, by decision rather than by omission.
- **Consequence:** unsupported namespaces and encrypted modern packages are
  refused. Legacy VD3/VD4/VD5 offline import uses a separate supported path; a
  legacy file or payload over 1,000,000,000 bytes (the storage ceiling,
  ADR-0142, since Alpha 15) is refused; Alpha 14 and earlier
  refuse a legacy file over 128 MiB or a payload over 256 MiB. Successful
  import does not guarantee full device semantics.
- **Workaround:** product data that arrives inside a `.knxproj` is imported
  with the project, which covers the common case of working on an existing
  installation.
- **Details:** [§11 `.knxprod` files for master-data scheme ≥ 12](../KNOWN_LIMITATIONS.md#11-knxprod-files-for-master-data-scheme--12-cannot-be-imported-directly)

### Devices configured through a manufacturer plug-in cannot be configured here

- **Affected:** devices whose ETS configuration is done by a vendor-supplied
  Windows DLL rather than by parameters in the product data.
- **Limitation:** KNXBench has no plug-in host and will not grow one —
  there is no plug-in API at all, by decision.
- **Consequence:** such a device can be imported, viewed and addressed, but
  its application-specific settings are opaque. When its application program
  declares an ETS download plug-in, every download scope is refused
  (`vendor-download-plugin`): complete, parameters and group addresses alike.
  Read-only comparison still works.
- **Workaround:** none. Use ETS for those devices.
- **Details:** [§6 devices behind vendor plug-in DLLs](../KNOWN_LIMITATIONS.md#6-devices-behind-vendor-plug-in-dlls),
  [§107 there is no plugin API](../KNOWN_LIMITATIONS.md#107-there-is-no-plugin-api--a-third-party-cannot-add-a-format-a-protocol-a-report-template-or-a-ui-panel-without-forking)

### Some manufacturer references still cannot be resolved to a name

- **Affected:** display of manufacturer, product and application-program
  names.
- **Limitation:** supported object defaults and matching installed
  product/program names resolve. Missing, mismatched or blank product data and
  ambiguous DPT alternatives remain disclosed; these are different problems,
  not three uniformly unimplemented resolution features.
- **Consequence:** a device may show an identifier where you expect a
  product name.
- **Details:** [§12 manufacturer data resolution](../KNOWN_LIMITATIONS.md#12-manufacturer-data-resolution--one-of-three-gaps-closed-2026-09-20)

<a id="legacy-import-is-offline-use-not-legacy-device-download"></a>

### Legacy devices download untested: planned, never run on a device

- **Affected:** `.vd3`–`.vd5` product databases.
- **Available:** web/CLI import, password dialog, optional remembered password,
  catalog placement, supported parameters, visibility and object links, and
  since 2026-10-09 download plans built from the file's own load procedure.
- **Remaining:** no legacy download has run on a real device yet, so
  readiness shows these devices as untested. For the Eibmarkt presence
  detector N000520 the legacy program writes one hidden threshold
  (`4196h`) as `0` where ETS writes nothing and the device keeps `1000`.
  Siemens `07B0h` programs (merged procedures) stay refused. No mapped
  legacy DPT codes. Since 13 October 2026 the legacy `string` and
  `long enum` parameters are shown and editable like ETS4's own conversion
  of them. Alpha 18 retains validated long-enum binary
  values in a typed column and backfills authenticated historical source;
  invalid values, ambiguous backfills and ordinary-enum binary fields are
  explicitly diagnosed. This does not enable legacy programming. A large VD5 takes 20–40 s and over 1 GiB of
  memory to import. (Untyped `TypeNone` spacers are drawn as headings or blank
  space since 2026-10-09, not as fields.)
- **Privacy:** the optional remembered password is one plaintext file with
  restrictive permissions on the **server**, not a keyring. Unencrypted
  originals may still contain source secrets; never post them publicly.
- **Details:** [Legacy product guide](user-guide/05-devices-and-products.md#old-ets3-product-databases-vd3-vd4-vd5),
  [ADR-0094](../adr/0094-legacy-exim-product-files.md), [§128](../KNOWN_LIMITATIONS.md).

### A device preview is not a lock on the whole project

- **Affected:** the add-device wizard's Review → Create step.
- **Limitation:** the server rechecks the previewed **names and addresses**.
  It does not bind the entire project/product database to a revision. Deleted
  placements can cause an ordinary validation refusal; other unrelated edits
  need not invalidate the preview. The wizard configures no parameters or links.
- **Workaround:** re-read the result and its product diagnostics. A computed
  free project address is neither reserved nor checked against the live bus.
- **Details:** [§167](../KNOWN_LIMITATIONS.md), [Add-device wizard](user-guide/05-devices-and-products.md#the-add-device-wizard).

### Three module limits in application programs

- **Affected:** devices whose application program uses modules.
- **Limitation:** repeated instantiation of the same module is refused rather
  than supported; a module with no `Id` cannot be matched to a project
  instance; and a project imported before store schema 6 has no module
  instance identifiers to write with.
- **Consequence:** module-scoped parameters may be unavailable on affected
  devices or on older stored projects.
- **Workaround:** for the third case only — re-import the project from its
  `.knxproj` with the current version.
- **Details:** [§68](../KNOWN_LIMITATIONS.md#68-repeated-module-instantiation-is-refused-not-supported),
  [§69](../KNOWN_LIMITATIONS.md#69-a-module-with-no-id-cannot-be-matched-to-a-project-instance),
  [§71](../KNOWN_LIMITATIONS.md#71-a-project-imported-before-store-schema-6-has-no-module-instance-ids-to-write-with)

### Drag and drop covers only a few gestures

- **Affected:** the Project Explorer and the device's link rows.
- **Limitation:** one eligible device can be dragged onto a line or building
  part of its own installation, and a group address can be dropped on a
  communication object's link row, which links it in the direction that row
  shows. Lines, building parts and several devices at once cannot be moved by
  dragging. The web file picker separately accepts dropped files and
  sequential multi-file uploads.
- **Workaround:** use the Inspector selects or bulk controls for other moves.
- **Details:** [Buildings and topology](user-guide/03-buildings-and-topology.md),
  [Projects](user-guide/02-projects.md)

## Group addresses

### CSV import does not manage group ranges

- **Affected:** the group-address CSV import.
- **Limitation:** native CSV supports explicit, preview-confirmed readdress/delete,
  but import does not create or restructure group ranges. The additional fixed
  column profiles are address-only: range rows are reported as not imported,
  DPT is read-only, and nonempty descriptions reject the entire file.
- **Consequence:** CSV can perform safe bulk address edits, but it is not a
  range-hierarchy or replacement/synchronisation format. Missing rows never
  imply deletion.
- **Details:** [§39](../KNOWN_LIMITATIONS.md#39-csv-import-never-re-addresses-deletes-or-manages-group-ranges),
  [§40](../KNOWN_LIMITATIONS.md#40-csv-export-only-columns-are-never-applied-on-import-and-there-are-no-descriptioncomment-columns)

### CSV export stays native; column-profile import is bounded

- **Affected:** exchanging group addresses with other applications.
- **Limitation:** export remains KNXBench's own CSV format. Import additionally
  accepts four fixed [column profiles](../contracts/group-address-csv-profiles.md)
  for three-level addresses. Range rows and `Security=Auto` are explicitly not
  imported; unsupported metadata refuses the whole file. This is not a full
  hierarchy/security roundtrip, XML exchange or ESF/OPC adapter.
- **Consequence:** do not assume another application will accept the native export.
- **Workaround:** inspect import warnings and keep the original input for fields
  that this address-only adapter cannot transfer.
- **Details:** [§38 no verified ETS interoperability](../KNOWN_LIMITATIONS.md#38-group-address-csv-exportimport-t12-has-no-verified-ets-interoperability),
  [§41 a CSV saved from Excel under a German locale](../KNOWN_LIMITATIONS.md#41-a-csv-file-saved-from-excel-under-a-german-locale-may-still-surprise-a-user)

## The bus

### Commissioning validation boundary

- **Affected:** device programming, download, restart and recovery.
- **Notice:** offline tests and historical runs on particular devices do not
  prove behavior on every device, manufacturer or ETS version. General
  real-hardware, power-loss, vendor and ETS validation is not provided. The
  project owner accepted this validation boundary on 2026-10-04; those
  experiments are not required to complete the commissioning goal.
- **Consequence:** there is no general guarantee of device compatibility or
  complete recovery after a crash or power loss. A finished activity-history
  row is not a recovery backup or a read-back proof of device state. Unsupported
  or uncertain device-specific operations remain refused.
- **Safety:** original values must be saved before any property mutation,
  including the original `PID_DEVICE_CONTROL`. If saving the backup fails,
  no property write may follow. Explicit confirmation cannot bypass this gate.
- **Current implementation:** the Web Live/History views and scoped offline
  caller/recovery checks are delivered. The views show their validation,
  partial-coverage and not-a-recovery notices. Do not read the old lifecycle
  handoff as an active unfinished alpha package. Hardware/power-loss evidence,
  complete journal coverage and universal recovery are still not supplied.
- **Details:** [Commissioning history contract](../contracts/COMMISSIONING_ACTIVITY_HISTORY.md),
  [commissioning goal](https://github.com/KNXBench-Labs/KNXBench/blob/6222e17754f3/docs/archive/alpha-0.1/goal-commission.md).

### A failed System-2 download's cause is not recorded (Alpha 17)

- **Affected:** the cause, step and restart evidence of a failed regular download
  (DIAG-02).
- **Limitation:** the server/web download worker, `knx device download` and
  `knx device restore` record it. Failed System-2 downloads, older history rows
  and older reports say *cause not recorded*. An older KNXBench version refuses a
  bus activity history page that contains a recorded cause (it shows the history
  as unavailable), as it already does for a read-only check.
- **Consequence:** the diagnostic says what KNXBench saw (a timeout, a refused
  write, a lost connection) and where; it does not prove why a device failed, and
  nothing is retried or restored automatically.
- **Details:** [Download failure diagnostic](../contracts/download-failure-diagnostic.md).

### There is no address reset in the app

- **Affected:** resetting a device's individual address to `15.15.255`.
- **Limitation:** the app has no reset view and the server has no reset route.
  The CLI command `knx device reset-address` shows its plan but refuses a
  confirmed reset before opening any connection, because no complete backup of
  every affected device exists. The project owner accepted this as a deliberate
  unsupported boundary on 2026-10-05.
- **Consequence:** reset the address with the manufacturer's tool or the
  device's own procedure.
- **Details:** [§140](../KNOWN_LIMITATIONS.md#140-the-individual-address-reset-needs-the-pressed-devices-named-and-its-restart-is-unconfirmed),
  [ADR-0058](../adr/0058-individual-address-reset-requires-durable-recovery.md).

### Device download has bounded evidence on three device programs

- **Affected:** writing a configuration into devices ("download").
- **Limitation:** `knx device download` and the **Download to device** tab
  work, but only three devices (two different MDT push-button programs and an
  Eibmarkt presence detector, all mask `0701h`) have been downloaded and read back so far. Other devices, application versions and
  masks are unverified; procedures KNXBench cannot plan are refused by name.
  Programming an individual address is currently refused until durable
  recovery exists. Unloading and secure devices are not supported. A successful
  byte read-back does not confirm the final restart; the verified devices'
  restart outcome is explicitly unconfirmed. The read-only check after a
  download reports reach, identity and load states only; it does not prove
  the application runs or that it is the same physical device. A bus
  activity history that holds such a check cannot be read by an older
  KNXBench version: its history view reports the history as unavailable and
  it refuses further bus writes until restarted, and again each time its
  history view reads a page of that history after a restart. Nothing in the
  file is lost; open it with this or a newer version.
- **Consequence:** for most installations, plan for another commissioning
  tool for now.
- **Workaround:** none beyond the verified scope.
- **Details:** [§7 commissioning](../KNOWN_LIMITATIONS.md#7-commissioning-and-device-download-are-required-but-blocked),
  [Downloading to a device](user-guide/07-bus-and-interfaces.md#downloading-to-a-device)

### Devices whose program is "little-endian" cannot be downloaded

- **Affected:** devices whose application program declares
  `ParameterByteOrder="LittleEndian"` (in the public product files KNXBench has
  seen, mostly ABB and Busch-Jaeger, some Hager and Berker programs). Readiness
  marks them unsupported; planning a download says why.
- **Limitation:** no published source defines how such a program stores its
  values. Product files strongly suggest "numbers low octet first", but the same
  pattern also shows up in programs of the same manufacturers that declare
  big-endian, so the files alone cannot say what ETS writes, and no device has
  been checked. KNXBench refuses rather than guesses.
- **Consequence:** configure these devices with another tool for now. Import,
  display and parameter editing are unaffected.
- **Workaround:** none for the download. If you have such a device that ETS
  programmed, `knx device byte-order-probe` can read (never write) which order it
  holds; that output is the evidence needed to lift the limitation.
- **Details:** [§170](../KNOWN_LIMITATIONS.md#170-little-endian-parameter-byte-order-is-refused-for-download-images),
  [the probe](user-guide/10-command-line.md#knx-device-byte-order-probe--help-settle-a-little-endian-device)
- **Related:** some ABB and Busch-Jaeger programs declare big-endian but their
  product data looks little-endian. Their downloads are allowed and the plan says
  so with the warning `base-image-low-octet-first` (command line, server and the
  download tab's **Before you write** box). Check multi-octet values after such a
  download, and report what you saw from **Help verify this device**.

### Stopping the server is not a safe cancellation of a device write

- **Affected:** server/container shutdown during commissioning.
- **Limitation:** the bounded HTTP shutdown window does not wait indefinitely
  for or roll back a device download. A remaining worker ends with the process;
  unsaved project edits are not saved on exit.
- **Workaround:** finish and check the device operation, then save the project
  before stopping. An empty **Live** view is not proof that the physical bus
  is idle. Never test recovery on an occupied installation without a fresh go.
- **Details:** [§163](../KNOWN_LIMITATIONS.md), [Live/History contract](../contracts/COMMISSIONING_ACTIVITY_HISTORY.md).

### KNX Secure is not implemented

- **Affected:** secured installations, both Data Secure and IP Secure.
- **Limitation:** not implemented. IP Secure was scoped and then shelved
  indefinitely.
- **Consequence:** secured bus communication is unsupported. This does not
  mean every project containing secure metadata is refused: readable archives
  can retain unsupported metadata without providing runtime security or keys.
  AES archive protection is a separate import refusal described above.
- **Details:** [§8 KNX Secure is not implemented](../KNOWN_LIMITATIONS.md#8-knx-secure-is-not-implemented),
  [§26 `BusConnection` does not support KNX IP Secure](../KNOWN_LIMITATIONS.md#26-busconnection-does-not-yet-support-knx-ip-secure)

### The DPT codec infers the format of what you type, and some encodings are this project's ruling

**Historical title:** new Web writes declare their grammar explicitly. Legacy
CLI/API callers may omit it and use the compatibility parser; the two paths
must not be described as either universally guessed or universally explicit.

- **Affected:** writing a value to a group address, and reading the decoded
  value of a telegram.
- **Limitation:** the codec covers thirty main types. The input format is
  declared, not guessed (`--input-format` in the CLI, a format choice in the
  interface); a caller that omits it gets a named compatibility parser. For
  several encoding questions the KNX standard's printed text is ambiguous
  enough that this project made a documented ruling instead of following it
  literally.
- **Consequence:** a misread input produces a valid telegram carrying the
  wrong value. The bus cannot tell you that it was wrong.
- **Workaround:** check the write confirmation, which echoes the decoded
  value rather than only the raw bytes.
- **Details:** [§61 the DPT codec](../KNOWN_LIMITATIONS.md#61-the-dpt-codec-covers-thirty-main-types-infers-rather-than-reads-its-input-and-leaves-several-encoding-questions-to-a-stated-ruling-rather-than-the-standard),
  [§95 six places where the printed standard must not be followed literally](../KNOWN_LIMITATIONS.md#95-six-places-where-the-knx-standards-printed-text-must-not-be-followed-literally)

### The group monitor is tunnelling only, one session at a time

- **Affected:** the bus monitor in the user interface.
- **Limitation:** tunnelling connections only — no routing in the monitor —
  a single session, and filters applied in the browser rather
  than at the source. Only the passive receive path has evidence from a real
  gateway.
- **Consequence:** you cannot watch two gateways at once, and a filtered view
  still carries every telegram across the wire.
- **Details:** [§62 the group monitor GUI](../KNOWN_LIMITATIONS.md#62-the-group-monitor-gui-t15-is-tunnelling-only-single-session-client-filtered-and-only-its-passive-receive-path-has-real-gateway-evidence)

### Some project telegrams have no declared datapoint type

- **Affected:** monitor decoding when the session has no project, an address
  is absent from its project, or a known address has no usable DPT.
- **Limitation:** under ADR-0010, an explicitly empty instance DPT is a stated,
  cleared value. The object table can show a retained program default
  (ADR-0027); the monitor names it but does not decode with a value the project
  clears. This is KNXBench's resolver policy. A caption or object size alone
  does not establish an encoding.
- **Consequence:** the four captured unresolved reasons are `noProject` (no
  project in session), `notInProject` (address not in project), `noDpt` (known
  address without a type), and `programDefaultCleared` (no assigned DPT; the
  project clears the named program defaults). Raw payloads remain visible.
  Assign a verified type and reconnect to use it for future telegrams;
  historical rows retain their interpretation and explanation. Older servers
  without a structured reason show that uncertainty.
- **Details:** [Monitor resolution reasons](../contracts/dpt-selection.md#monitor-resolution-reasons-ui-01-18-october-2026).

### A line scan costs real bus time and cannot distinguish every case

- **Affected:** scanning a line for devices.
- **Limitation:** an unthrottled scan is load on a live installation, not a
  theoretical cost. A busy device and an absent one look the same; other
  KNXnet/IP tunnelling endpoints are reported as occupied addresses; a scan
  covers one line and does not cross couplers; and it learns no product
  identity, manufacturer or serial number.
- **Consequence:** the result is a list of addresses that answered, which is
  less than a device inventory.
- **Workaround:** scan outside working hours, keep the default timeout unless
  you have reason to shorten it, and treat surprises as questions rather than
  findings.
- **Details:** [§72](../KNOWN_LIMITATIONS.md#72-line-scan-t17-an-unthrottled-scan-is-a-live-bus-cost-not-a-theoretical-one--shipped-2026-09-13-still-true),
  [§74](../KNOWN_LIMITATIONS.md#74-a-line-scan-cannot-distinguish-a-busy-but-present-device-from-an-absent-one),
  [§77](../KNOWN_LIMITATIONS.md#77-a-line-scan-covers-one-line-at-a-time-it-does-not-cross-couplers),
  [§78](../KNOWN_LIMITATIONS.md#78-a-line-scan-reports-other-knxnetip-tunnelling-endpoints-as-occupied-devices)

### Movement in the flow view is heavy on large installations

- **Affected:** the bus monitor's Flow view with Motion on.
- **Limitation:** the recorded large-map Chromium tests kept the processor
  almost fully busy even at a few telegrams a second, with visible-value lag
  up to about a second. These are source-bound recorded measurements, not
  a new benchmark of every build or host.
  There is no automatic switch. Checked in Chromium only; the packaged desktop
  app and screen readers were not measured.
- **Workaround:** switch *Motion Off* in the settings. Values, arrows, counts
  and the Inspector stay; nothing is lost.
- **Details:** [§154 the telegram-flow view](../KNOWN_LIMITATIONS.md#154-the-telegram-flow-view-is-checked-and-measured-in-chromium-only)

<a id="opening-an-older-project-upgrades-the-file"></a>

### Saving into an older project upgrades the file

- **Affected:** a project saved by an older KNXBench version.
- **Limitation:** the first *write* upgrades the file itself to the current
  format. Writes are: any edit to a project opened from a `.knxdb` (KNXBench
  keeps the working state and its undo history in the file as you edit), Save,
  creating, restoring or deleting a project version, and confirmed changes such
  as `knx ga-import`, a name recipe, a selective import or an approved MCP
  rename. No copy
  is made, and the older KNXBench then refuses the file as "newer". A failed
  upgrade leaves the file unchanged.
- **Not affected (since 2026-10-13):** opening and reading. The application's
  Open, `knx diff`, `knx ga-export`, `knx doc-export`, the device readiness,
  download plan, compare and byte-order probe, the bus commands' `--project`,
  a `knx ga-import` dry run or refusal, and the read-only MCP adapter all read
  an older file through an in-memory copy and leave it byte for byte as it was.
- **Workaround:** copy the project file first if an older version must still
  open it after you change it.
- **Details:** [§157](../KNOWN_LIMITATIONS.md#157-opening-an-older-project-upgrades-it-in-place)

### A running monitor keeps the group-address style it started with

- **Resolved in T13:** changing the open project's group-address style refreshes
  the running session's interpretation context without reconnecting. Undo and
  Redo of that style change refresh it as well.
- **Scope:** subsequent monitor rows and write parsing use the refreshed style;
  already recorded rows are not retrospectively reformatted. This is not a new
  slash/dot notation preference.
- **Details:** [§91 a running bus session keeps its style](../KNOWN_LIMITATIONS.md#91-a-running-bus-session-keeps-rendering-group-addresses-in-the-style-the-project-had-when-it-started)

## Web and Docker

### A server restart logs everyone out

- **Affected:** browser use of a password-protected server.
- **Limitation:** sessions live in the server process's memory only. There is
  no session store on disk, so restarting `knx-server` — or recreating the
  container — invalidates every session immediately. Sessions also expire
  after 12 hours idle.
- **Consequence:** after a restart the login card comes back over whatever you
  had open. Unsaved work is not lost by the login itself, but it is lost if the
  restart also threw away an unsaved project the server was holding.
- **Workaround:** save before you restart anything. Signing in again returns you
  to the same screen.
- **Details:** [§22 `knx-server` authenticates with one password](../KNOWN_LIMITATIONS.md#22-knx-server-authenticates-with-one-password-or-refuses-to-leave-loopback)

### What the server's access control is, and is not

- **Affected:** any deployment beyond a single trusted machine.
- **Limitation:** HTTPS is enabled by default with a password, but a self-signed
  certificate needs a fingerprint check. There is one password rather than accounts,
  no roles or per-user audit trail, and no CSRF token (the cookie uses SameSite=Strict).
  A failed attempt is delayed,
  not locked out. Supplying the password as `KNX_AUTH_PASSWORD` is weaker than
  supplying a hash via `KNX_AUTH_PASSWORD_HASH`, because the plain value is
  visible to anything that can read the process environment.
- **Consequence:** this is a barrier against an accidental visitor, not a
  security model for an untrusted network.
- **Details:** [§22](../KNOWN_LIMITATIONS.md#22-knx-server-authenticates-with-one-password-or-refuses-to-leave-loopback)

### A container without a password cannot be reached through a published port

- **Affected:** running the container.
- **Limitation:** measured against the real image. Without a credential the
  server binds the container's own loopback, so a published port
  cannot reach it — the connection is refused. This is the
  server's deliberate refusal to expose an unguarded API, not a bug.
- **Consequence:** the published-port form only works with a password
  configured. With one, the earlier bridge-image check worked: build, publish the
  listener port, set `KNX_AUTH_PASSWORD`,
  sign in, API answers `200` — evidenced by the earlier image run, not repeated
  by this audit. Current authenticated builds default to HTTPS; follow the
  [certificate/start recipe](user-guide/11-web-and-docker.md#https).
- **Default recipes:** Linux host networking on port 8484 avoids the bridge.
  A password exposes the authenticated listener on all host interfaces; use
  a firewall for trusted LAN/VPN access. For a local-only credential-less server:
  `docker run -d --name knxbench --network host -e KNX_PORT=8484 -v "$(pwd)/data:/data" knxbench-server`.
  That form is reachable from that machine only, which is also why it is safe
  without a password.

### Discovery does not work inside Docker's default network

- **Affected:** KNXnet/IP gateway discovery from a container, including the
  interface **Search** in the web UI's bus monitor.
- **Limitation:** discovery needs IP multicast, which the default bridge
  network does not carry.
- **Workaround:** the host network, as above, or configure the gateway
  address manually.
- **Details:** [§79 discovery needs IP multicast](../KNOWN_LIMITATIONS.md#79-discovery-needs-ip-multicast-which-dockers-default-bridge-network-does-not-carry)

### One project, one undo stack, no matter how many browsers

- **Affected:** two people opening the same server.
- **Limitation:** one shared project and undo stack, with no general
  multi-user conflict-resolution model. Specific CSV/creation previews have
  their own stale-state guards; those are not collaborative editing.
- **Consequence:** a second browser is a second pair of hands on the same
  keyboard. One person's undo reverses the other person's edit.
- **Workaround:** one editor at a time. This is a discipline, not a lock —
  nothing enforces it.
- **Details:** [§63 no multi-user support](../KNOWN_LIMITATIONS.md#63-knx-server-has-no-multi-userconcurrent-edit-support--one-shared-project-one-shared-undo-stack-no-conflict-detection-at-all)

### Browser Save As is not a local download

- **Affected:** saving a `.knxdb` from the web build.
- **Limitation:** Save As targets the server's permitted directory, not the
  browser's Downloads folder.
- **Workaround:** use File → Export project… for a local copy of the open
  in-memory project. Save separately if you want the server-side file updated.
- **Details:** [Projects](user-guide/02-projects.md)

### A reopened tab resumes the server's project, but a server restart forgets it

- **Affected:** the web build (Docker or `knx-server`), source builds after
  v0.1.0-alpha.11.
- **Limitation:** reopening or reloading the tab brings back the project the
  server still holds (ADR-0127), unsaved changes included. Those changes live in
  the server's memory only: restarting the server loses them, and two tabs on
  the same server show and edit the same project.
- **Workaround:** save, or keep autosave on, before restarting the server.
- **Details:** [Projects](user-guide/02-projects.md#closing-the-browser-tab)

### The desktop overwrite question comes from the native dialog

- **Affected:** Save As in the desktop build, source builds after v0.1.0-alpha.11.
- **Limitation:** the desktop build sends the overwrite confirmation directly
  because its native save dialog asks before replacing a file. That was verified
  in the GTK file chooser used on Linux (`do_overwrite_confirmation`), not on
  Windows or macOS builds.
- **Details:** [Projects](user-guide/02-projects.md#saving)

### The Docker Hub image is the last release, and `latest` follows alphas

- **Affected:** `knxbench/knxbench-server` from Docker Hub.
- **Limitation:** images are published for release tags only, starting with
  `v0.1.0-alpha.6`; `latest` moves with every pre-release. Commits after the
  last tag are not in any image.
- **Workaround:** pin a version tag for deliberate updates; build locally for
  source newer than the last release.
- **Details:** [§168](../KNOWN_LIMITATIONS.md#168-the-docker-hub-image-follows-release-tags-only-and-latest-follows-alphas),
  [Ready-made image](user-guide/11-web-and-docker.md#ready-made-image-from-docker-hub)

## Desktop

### One host, one architecture, one build

- **Affected:** the desktop application.
- **Limitation:** the AppImage has been built and launched on a single Linux
  host. It is unsigned, has no auto-update, is x86-64 only, and there are no
  distribution packages.
- **Consequence:** the public AppImage is an available Linux installation
  route, but evidence from one host is not universal compositor/GPU support.
- **Workaround:** build from source, or run the server and use a browser.

### Alpha.5 can start with a different native data folder

- **Affected:** upgrading the desktop application from alpha.4 to alpha.5.
- **Limitation:** the application identifier/data location changed; the new
  native app does not automatically adopt the old folder. An empty start is
  not proof that the old project was deleted. Server/product/settings paths
  are separate and must not be assumed to be one directory.
- **Workaround:** keep independent copies, reopen the old saved `.knxdb` through
  the normal file picker, and inspect which catalog/settings installation you
  are using. Do not merge live SQLite files or discard the old folder blindly.
- **Details:** [§161](../KNOWN_LIMITATIONS.md), [Installation](getting-started/04-installation.md).

<a id="the-appimage-needs-an-x-server"></a>

### Old AppImages may force X11; current builds have an owned display policy

- **Affected:** the exact AppImage/build you are running on Wayland.
- **Historical limitation:** the unchanged alpha.4 image forces X11 and needs
  a reachable X server. Current source and the alpha.5 tagged source include
  the owned launcher hook: automatic Wayland/X11 selection, caller overrides
  and the measured DMABUF workaround. That source comparison is not a new
  native run of the downloaded asset or every GPU/compositor.
- **Workaround for an old forced-X11 image:** enable Xwayland, or start the unpacked image natively on
  Wayland as shown in
  [Troubleshooting](reference/03-troubleshooting.md#the-appimage-stops-with-failed-to-initialize-gtk)
  (tested on one Hyprland machine).
- **Details:** [§158](../KNOWN_LIMITATIONS.md#158-the-appimage-starts-only-with-an-x-server)
  and [launcher contract](../APPIMAGE_LAUNCHER.md).

### AppImages up to alpha.9 can stay empty on newer Mesa drivers — fixed in alpha.10

- **Affected:** `v0.1.0-alpha.9` and earlier AppImages on systems whose Mesa
  EGL driver needs a newer `libwayland-client` (reported on Kubuntu 24.04).
- **Limitation:** the bundled library shadows the system's; WebKit aborts with
  `Could not create default EGL display: EGL_BAD_PARAMETER`.
- **Workaround:** preload the system library as shown in
  [Troubleshooting](reference/03-troubleshooting.md#the-appimage-window-stays-empty-could-not-create-default-egl-display).
  Or update: `v0.1.0-alpha.10` no longer bundles the library
  ([issue #6](https://github.com/KNXBench-Labs/KNXBench/issues/6)).
- **Details:** [§176](../KNOWN_LIMITATIONS.md#176-appimages-up-to-alpha9-abort-on-hosts-with-newer-mesa-egl).

<a id="appimages-up-to-alpha11-cannot-save-the-session-log-or-a-monitor-capture--fixed-in-source"></a>

### AppImages up to alpha.11 cannot save the session log or a monitor capture — fixed in Alpha 12

- **Affected:** *Export log (JSON)…* in the session log and *Export capture* in
  the bus monitor, in the AppImage from `v0.1.0-alpha.5` to `v0.1.0-alpha.11`.
  Web and Docker are not affected.
- **Limitation:** the save fails with `Command save_session_log not allowed by ACL`
  (or `save_bus_monitor_capture`): the desktop window's permission list never
  granted these two commands.
- **Workaround:** open the Docker image in a browser; there both files are
  ordinary downloads. Or update to the Alpha 12 AppImage
  ([issue #7](https://github.com/KNXBench-Labs/KNXBench/issues/7)).
- **Details:** [§181](../KNOWN_LIMITATIONS.md#181-appimages-up-to-alpha11-cannot-save-the-session-log-or-a-bus-monitor-capture).

<a id="appimages-up-to-alpha11-save-downloads-silently-and-can-replace-files--fixed-in-source"></a>

### AppImages up to alpha.11 save downloads silently and can replace files — fixed in Alpha 12

- **Affected:** every *Download* or *Export* in the AppImage that is not a save
  dialog: the full import report, source reports, retained payloads, the download
  verification report, settings export, contribution ZIP, Flow images and the
  confirmed project export (`project.knxdb`). Web and Docker are not affected.
- **Limitation:** nothing visible happens. The file is written straight into your
  Downloads folder, and a file of the same name already there is replaced.
- **Workaround:** before downloading again, rename or move the previous file in
  Downloads; or use the Docker image in a browser. Alpha 12 asks where to
  save each download.
- **Details:** [§182](../KNOWN_LIMITATIONS.md#182-appimages-up-to-alpha11-save-in-app-downloads-silently-into-downloads-and-replace-same-named-files).

### The desktop shell has no login, on purpose

- **Affected:** nothing you can do about it, but worth knowing.
- **Limitation:** the desktop application talks to an in-process server that
  is deliberately not behind the password check. Its HTTP listener binds
  **127.0.0.1**, not the LAN; it is still a local endpoint, not a promise of
  isolation from other software on the machine.
- **Consequence:** anyone at the machine has the project. The protection is
  the machine's own.

### The Linux backend still uses GTK3

- **Affected:** the desktop build's platform lifecycle.
- **Current repository boundary:** the pinned Linux desktop stack uses GTK3.
  The 2026-09-22/2026-10-06 advisory reviews in §16 are dated evidence, not
  a claim about today's upstream releases or advisories.
- **Consequence:** backend migration needs its own verified platform work;
  this documentation audit neither changes the runtime nor certifies its
  whole dependency/security lifecycle.
- **Details:** [§16 Tauri Linux backend](../KNOWN_LIMITATIONS.md#16-tauri-v2-remains-on-gtk3-former-maintenance-advisories-are-resolved)

#### Historical note (superseded in 2026)

- **Affected:** the desktop build's long-term maintenance.
- **Limitation:** Tauri v2's Linux backend uses Rust GTK3 bindings whose
  upstream repository is archived.
- **Consequence:** a future distribution change could break the desktop build
  in a way this project cannot fix quickly.
- **Details:** [§16 Tauri v2's Linux backend](../KNOWN_LIMITATIONS.md#16-tauri-v2s-linux-backend-depends-on-archived-gtk3-bindings)

## Reports and comparison

### Documentation export is HTML, and less than ETS's

- **Affected:** the project documentation export.
- **Limitation:** HTML only — no native PDF. The export dialog previews the
  document, selects sections and prints the preview through the browser, and
  one export can be English or German (following the interface language).
  Printing from the desktop app's webview has not been checked. Installed product data can add manufacturer/product/
  program names, parameter enum labels and module-argument names, but only for
  a hardware-consistent product/program pair. Missing or blank names,
  unformatted parameter kinds and unsupported module semantics remain raw and
  explicitly warned. No parity with ETS's reports has been measured, and with
  no ETS installation here it cannot be.
- **Consequence:** useful as a record, not as a substitute for an ETS report.
- **Workaround:** use *Print…* in the export dialog, or print the exported HTML
  from a browser, for PDF.
- **Details:** [§44](../KNOWN_LIMITATIONS.md#44-project-documentation-export-t13-has-no-ets-report-parity-and-none-can-currently-be-measured),
  [§45](../KNOWN_LIMITATIONS.md#45-project-documentation-export-has-no-native-pdf-output),
  [§46](../KNOWN_LIMITATIONS.md#46-project-documentation-export-does-not-resolve-manufacturer-product-or-program-names),
  [§47](../KNOWN_LIMITATIONS.md#47-project-documentation-export-does-not-list-parameter-values-or-module-instance-arguments),
  [§48](../KNOWN_LIMITATIONS.md#48-project-documentation-export-renders-in-one-language-only),
  [§49](../KNOWN_LIMITATIONS.md#49-project-documentation-export-has-no-in-application-print-preview),
  [§50](../KNOWN_LIMITATIONS.md#50-project-documentation-export-has-no-section-selection)

### Project comparison is read-only in the web panel

- **Affected:** comparing two projects.
- **Limitation:** the web panel lists entities and before/after values;
  long tables scroll and can be filtered by text and status, but there is
  no search across tables. A
  comparison cannot merge or apply a
  difference back onto a project or do a three-way comparison. The web panel
  has no process exit code, but **`knx diff --exit-code` already exists**:
  0 equal, 1 different, 2 comparison failure.
- **Consequence:** finding one entity in a large comparison means
  opening its table and filtering there.
- **Details:** [§55](../KNOWN_LIMITATIONS.md#55-project-diff-cannot-merge-or-apply-a-diff-back-onto-a-project),
  [§56](../KNOWN_LIMITATIONS.md#56-project-diff-does-not-do-a-three-way-comparison),
  [§57](../KNOWN_LIMITATIONS.md#57-project-diff-cannot-compare-against-a-raw-knxproj),
  [§58](../KNOWN_LIMITATIONS.md#58-project-diff-has-no-ci-friendly-exit-nonzero-on-any-difference-flag),
  [§59](../KNOWN_LIMITATIONS.md#59-project-diffs-text-and-web-renderers-show-which-fields-changed-not-their-beforeafter-values-for-most-entity-types),
  [§60](../KNOWN_LIMITATIONS.md#60-project-diffs-web-panel-shows-grouped-counts-only)

## Language and accessibility

### Structure headings shift on hover — fixed in Alpha.10

The Ground/building/floor and topology headings inherited generic button
magnification, moving text left and clipping icons inside their cards
([issue #5](https://github.com/KNXBench-Labs/KNXBench/issues/5)). The
11 October 2026 source fix keeps headings stationary and uses only a colour
highlight. EN/DE Chromium checks cover light/dark palettes and narrow/wide
layouts; native WebKitGTK visual verification remains separate. Alpha.10 contains
the repair; Alpha.9 packages do not.

### The interface is English and German; some text is neither

- **Affected:** everyone not reading English.
- **Available:** English/German UI, bundled Bavarian/Klingon packs and
  importable language packs. Product translations from program, catalog,
  hardware and master scopes are ingested; supported display overlays and
  fallback markers already work. Product data is not a UI language pack.
- **Remaining:** not every source text is translated or has a consumer.
  Some server/error/detail prose stays English; reports have their own bounded
  EN/DE catalogue, not third-party pack support. Project-authored text and
  parameter values are not silently rewritten by display language.
- **Details:** [§37 imported translations are stored but never read](../KNOWN_LIMITATIONS.md#37-imported-translations-are-stored-but-never-read-and-the-ui-is-english-only--partially-resolved-2026-09-12),
  [§64 `Languages` blocks outside an application program](../KNOWN_LIMITATIONS.md#64-languages-blocks-outside-an-application-program-are-discarded-on-import),
  [§66 server-composed prose and documentation export](../KNOWN_LIMITATIONS.md#66-server-composed-prose-and-the-documentation-export-are-not-translated-by-any-ui-language-or-pack--partially-resolved-2026-09-14-t14)

### Shared preferences and achievements are not per-user records

- **Affected:** several people/windows using one server.
- **Limitation:** the first-run guide is remembered per installation and can
  be reopened manually. Achievements are shared, count UI actions rather than
  every CLI/API action, and concurrent increments can merge to the higher value
  rather than their sum. They confer no device-write authority.
- **Details:** [§160](../KNOWN_LIMITATIONS.md), [§164](../KNOWN_LIMITATIONS.md).

### AI answers can lag unsaved work

- **Affected:** agents using the experimental read-only MCP adapter.
- **Limitation:** saved snapshots only; dynamic activation is not full access
  or ETS visibility semantics. An older product database is copied/migrated in
  memory at startup, so newly installed products may require an MCP restart.
  Agent tool results can reach that agent's model provider.
- **Workaround:** save first, check snapshot/visibility reasons, and review CSV
  proposals yourself. The adapter cannot apply them or operate the bus.
- **Details:** [AI agents](user-guide/12-ai-agents.md), [§165](../KNOWN_LIMITATIONS.md).

### Support-gap evidence is not automatically anonymous or submitted

- **Affected:** File → Analyze support gaps… and exported evidence ZIPs.
- **Limitation:** analysis and preview/export are implemented, not support for
  every analyzed format. Selected XML/originals can contain identifying data;
  secret detection is conservative, not exhaustive. Opening an issue/mail draft
  posts nothing; private mailbox operation is unverified.
- **Workaround:** inspect the exact disclosure preview, share only deliberately,
  and never attach private originals to public issues.
- **Details:** [Community evidence](../COMMUNITY_EVIDENCE.md), [contribution guide](../contribution-intake/README.md).

### Screen reader support is incomplete

- **Affected:** use with assistive technology.
- **Limitation:** accessibility work has closed some gaps and left others.
  Successful project loads now announce a localized status through a polite
  live region, but this is not end-to-end assistive-technology validation.
- **Consequence:** the application has not been validated with assistive
  technology end to end. It is not claimed to be accessible.
- **Details:** [§20 command palette and search overlay](../KNOWN_LIMITATIONS.md#20-command-palette-and-search-share-overlay-css-and-an-accessibility-gap--partially-resolved),
  [§118 successful load announcement](../KNOWN_LIMITATIONS.md#118-a-succeeded-project-load-announces-nothing-to-a-screen-reader)

### Motion settings do not override reduced motion

- **Affected:** motion in the interface.
- **Behavior:** Settings offers motion style and level, including Off. The
  operating system's reduced-motion preference still takes precedence.
- **Details:** [Settings and appearance](user-guide/09-settings-and-appearance.md)

## Documentation defects

Documentation can be wrong too, and when it is, that is a defect like any
other. These are the ones found while writing this manual.

### `COMPATIBILITY.md` understates datapoint type coverage

**Resolved documentation defect.** Current compatibility text describes main
families 1–30 and explicitly limits subtype/application semantics. The old
“20 and above” assertion is no longer a current statement; this heading remains
for historical links. The [DPT audit](../spec-audits/2026-10-07-dpt-document-audit.md)
does not turn family coverage into complete KNX conformance.

- **Affected:** readers of the compatibility document.
- **Historical limitation:** its section 2 excluded "main types this codec does not
  implement (20 and above)". The codec implements and tests main types up to
  30 — types 20, 21, 25, 29 and 30 among them.
- **Current consequence:** none from that obsolete sentence. Actual codec
  limitations remain documented separately; code alone is not a conformance oracle.
- **Details:** [`docs/COMPATIBILITY.md`](../COMPATIBILITY.md), section 2

### Some repository documents predate the server login

- **Affected:** statements about authentication elsewhere in the repository.
- **Limitation:** the manual was rewritten on 2026-09-20 for the password
  login and the browser login screen that arrived the same day. Older
  engineering documents — `PROJECT_ANALYSIS_2026-09-15.md`, for one —
  still describe a server with no authentication at all, because that was
  true when they were written. Those snapshots and the former triage have
  since been removed from the active tree and pinned in Git history; they
  are not current task queues. Read-on-Init is stored and undoable in native
  projects; there is no `.knxproj` exporter to discard it.
- **Consequence:** where a dated analysis document contradicts this chapter
  about authentication or about the sixth flag, this chapter is the newer one.
  [`KNOWN_LIMITATIONS.md` §22](../KNOWN_LIMITATIONS.md#22-knx-server-authenticates-with-one-password-or-refuses-to-leave-loopback)
  is current and is the engineering record.

---

If you find something this chapter does not mention, it is worth reporting
even if you suspect it is known. The list above is what has been found, which
is not the same as what exists.



## Table column-width coverage (18 October 2026 source, local-only)

The address/object matrix's per-object columns share one resizable width
(default 180 px, since Alpha 17); individual object columns cannot get
their own widths because project-object identifiers cannot enter browser
preferences. Its select/address/name columns resize and persist by presentation ID. All 18 native table surfaces use the adapter. Widths are
browser-local and are lost when browser storage is cleared; blocked storage
supports resizing only for the mounted session. Manufacturer parameter grids,
trees and catalogue/history lists retain their existing layouts. See
[Table column widths](../contracts/table-column-widths.md).

Automatic word floors stop at 320 px per column. Tokens wider than this cap can
wrap into character fragments, as can ordinary words after explicit user
shrinking. Home sets the stored preference minimum; the rendered width also
depends on pane fitting and surplus distribution. Word floors exclude spanning
group headings/editors. Demo/fixture no-scroll checks do not establish fitting
for every imported project.

[Manual index](README.md) · Next: [Implementation status](implementation-status.md) →
