# Alpha 18 — pre-release

Alpha 18 covers source changes since `v0.1.0-alpha.17`. It answers issue #31 by
explaining what an experimental System-2 run actually achieved, down to which
operation of the final step stopped, follows up on
issue #30 with a sturdier tunnel and more patient experimental waits, and adds
group-address CSV column profiles, German CLI reports and two product-database
data migrations. None of this is new hardware support.

## For reporters first

### Issue #31: the "Par" mark after a System-2 download

[Issue #31](https://github.com/KNXBench-Labs/KNXBench/issues/31) reported that the
device list kept its plain pending marks after a System-2 run whose transfer had
completed but whose final verification failed. Fixed in feature `d52ee440e`
(merge `8c06ddc96`); closed for the display and integration gap, **not** as
hardware acceptance.

- **The result says what the record proves.** A closed, journal-derived result
  separates *nothing written*, *transfer incomplete*, *transfer complete, final
  verification not started / unfinished*, *verified* and *unknown*. "Unfinished"
  alone does not say whether the last step found a difference, could not read or
  failed its final write; the final-step entry below says which operation stopped
  when it could be classified. The device is never shown as loaded. A later recovery keeps its
  own outcome and never rewrites the download's result.
- **Device list.** A neutral, dashed **S2** chip (EN/DE, with tooltip) sits beside
  the project's pending marks and never replaces them. It appears only for the
  open project, the current server run and the device that still carries the
  recorded address, and is starred once the project was edited.
- **Project marks follow the existing per-part rules (ADR-0129).** A run that may
  have written makes Application, Parameters and Group addresses pending. Only a
  fully verified run, with the whole preparation and retained source unchanged,
  marks the parts programmed. Any edit that changes the preparation leaves them
  pending. Old operations do not replay settlement.
- **Status survives navigation.** One application-level status owner keeps
  watching a running operation across view switches; if reloading the project's
  marks fails, a notice with an explicit **Reload marks** retry stays visible.
- **Save report.** Ended retained operations, including ones from before this
  release, can be saved as a local `knxbench-system2-download-summary` JSON
  (closed facts and counts; no address, device name, parameters, backup bytes or
  paths). Operations from older builds report their execution build as unknown.
  HTTP status schema 4 carries the result fields and the final-step list (below);
  the web client accepts exact versions 1 to 4. Report format 3 is the only one the
  web decoder reads.

[Contract](../contracts/system2-download-result.md) ·
[Guide](../manual/user-guide/05-devices-and-products.md#what-an-ended-experimental-run-proves-issue-31) ·
[Acceptance and boundaries](../status/2026-10-28-system2-result-integration.md) ·
[S2-05](../open-work/S2-05-result-and-settlement.md).

### Issue #30: System-2 download for UP 258D12 / `M-0001_A-9205-04-9270`

[Issue #30](https://github.com/KNXBench-Labs/KNXBench/issues/30) stays open. The
scope is unchanged: **UP 258D12 application `M-0001_A-9205-04-9270` only,
experimental, software-tested.** A reporter-observed run on a source build changed
the device's behaviour but failed final allocation verification; that mismatch is
**unresolved**, and nothing below claims the download works on real hardware.

- **The tunnel says why it closed** (`26634deb5`, `5b333d77b`, merge `1ab4ad6a5`).
  A send on a closed tunnel, or one the gateway closes during the acknowledgement
  wait, now ends at once with a typed cause (gateway disconnect, unacknowledged
  request with its sequence and last ACK status, heartbeat, socket or client)
  instead of a bare timeout. After an unacknowledged request the client sends one
  `DISCONNECT_REQUEST` and keeps listening, so a teardown the gateway answers is
  confirmed. This is the shared KNXnet/IP client. For System-2 the cause reaches
  the private journal's diagnostic text.
- **Persistence no longer blocks the tunnel.** The server runs the System-2
  download and recovery executors off the runtime workers that serve the tunnel
  (`26634deb5`); the ordinary download worker and the service-control backup
  follow (`9103c80db`, `51f7598b5`, merge `ca940e2ed`). The individual-address
  settler's project save does too (`5efcf506a`, merge `c59d14280`), as
  preparatory hardening: that HTTP route still refuses before any tunnel opens.
  Backup, identity, consent and read-back rules are unchanged.
- **More patient experimental waits** (`6f7d22228`, merge `3583ab8e0`). The
  experimental hardware path, its explicit recovery and the experimental CLI wait
  10 s (was 1 s) per gateway acknowledgement attempt, 15 s (was 3 s) per
  management-answer slice after a positive device transport acknowledgement and
  120 s (was 30 s) for a load-state transition. The three-second device
  acknowledgement/repetition timer, repetition limits and all other tunnel users
  keep their timings. A gateway that closes the channel still ends the wait at once.
  [Contract](../contracts/system2-patient-waits.md) ·
  [record](../status/2026-10-27-system2-patient-waits.md).
- **S2-03 decided: invisible parameters stay in the Background** (`3ed49015a`,
  `4f462822f`, merge `d39668393`). Owner decision: defaults profile v3 of the
  validation workflow keeps the source `Data` of inactive parameters, like the
  configured path. On the pinned public package it writes the same 775 octets as
  the configured defaults bundle. This is an observation-supported hypothesis, not
  a defined rule. Template retention now requires an explicit
  `DownloadInvisibleParameters="Background"`; other or missing values refuse before
  an image is built. [Record](../status/2026-10-28-s203-background-defaults-v3.md) ·
  [source search](../research/2026-10-28-background-precedence-source-search.md).
- **Report for the experimental path.** The issue #31 report above also covers
  #30's earlier finding that a System-2 attempt had no exportable report.
- **The last System-2 step says which operation stopped** (feature `96f41c7f1`,
  main-first `309384726`, result record `bb2d8e51e`). The final `Disconnect` step
  hosts the read-only allocation verification, the final Loaded-state and
  program-version reads and the Verify Mode restore write. When it fails in its own
  verification or restore, Net now classifies the stop from the operation in flight
  (never from message text): a completed read that **disagreed**
  (`verificationMismatch`), a read that gave **no usable answer** (`readFailed`) or
  the Verify Mode **write** that did not complete (`writeFailed`), each with its
  stage (limit, allocation, loaded state, pointer, program version, Verify Mode
  read, write or read-back). The category names the operation, not a cause: a read
  failure does not say whether the device was silent or refused. Stops elsewhere,
  such as a journal failure or a cancellation, stay unclassified. A download or an
  explicit recovery gets at most one entry; a recovery adds its own and never
  relabels the download's. The record is journalled as a closed, payload-free
  checkpoint envelope 3, shown in the EN/DE panel and carried by status schema 4 and
  report format 3 (also for the property-validation summary). Older operations and
  uncertain cases show "unavailable"; nothing is backfilled from private diagnostic
  text. Tested with simulator, loopback and intercepted-HTTP transports only. This
  does **not** solve the allocation mismatch, show that a device holds the
  requested configuration or imply per-part equivalence or ETS equivalence.
  [Contract](../contracts/system2-download-result.md#final-step-failure) ·
  [focused proof and acceptance](../status/2026-10-28-system2-final-step-failure.md).

Do not post private journals, memory or bundles in a public issue. S2-02 (the
original start failure) and the field verification of the tunnel changes
([S2-04](../open-work/S2-04-tunnel-loss-after-checkpoint.md)) remain open.
[ADR-0152](../adr/0152-gated-hardware-system2-download.md) ·
[S2-03](../open-work/S2-03-up258-image-interpretation.md).

### Issue #22: Siemens presence detector (`MV-0025`)

Nothing new for issue #22 itself. The issue #30 and #31 changes above touch the
same experimental UP 258D12 path; the allocation mismatch is unresolved and
issue #22 stays open.

## Workbench and import

- **Group-address CSV column profiles** (`301a795b6`, merge `ed8b8ec13`). The
  shared CSV import path (`knx ga-import`, the server import route and MCP
  `validate_ga_csv`, which never applies) accepts four fixed name/address column
  layouts for three-level projects, including split `Main`/`Middle`/`Sub`
  columns. The adapter is address-only: range rows are reported as not imported,
  DPT stays read-only, `Security=Auto` is reported and not imported, and a
  nonempty Description or another Security value refuses the whole file. Native
  CSV export is unchanged. [Contract](../contracts/group-address-csv-profiles.md) ·
  [KL-038](../limitations/KL-038-csv-interoperability.md).
- **German CLI reports** (`fe8b57981`, merge `5679ba3c5`).
  `knx doc-export <store.knxdb> <out.html> --language de` uses the existing
  report renderer; `en`, `en-US`, `de` and `de-DE` are accepted, and without the
  option the report stays English. Invalid or repeated values refuse before the
  store or output is touched. [Record](../status/2026-10-27-cli-report-language.md) ·
  [KL-048](../limitations/KL-048-report-languages.md).
- **Catalogue devices land in the chosen installation** (`8d3ebe69d`, merge
  `a17420239`). Creating a catalogue device without a target line in a project
  with several installations requires an explicit **Installation** choice;
  removing that installation clears the choice instead of picking another.
  Lost-response retries keep the original target. Single-installation projects
  and line-owned targets behave as before.
  [Record](../status/2026-10-27-catalogue-installation-target.md).
- **`TypeDate` parameters** (`b5dc92aa8`, merge `4beabe531`) are stored as kind
  `Date` and edits validate `yyyy-mm-dd` against real calendar days. The field
  stays a plain text field; download images with date parameters stay refused.
  [Record](../status/2026-10-28-typedate-lexical-slice.md).
- **Legacy long-enum binary values** (`68f9da61e`, `895f4f793`, merge `6dbdaf8b9`)
  keep their exact bytes, including leading zeros, in the typed binary-value
  column. Ordinary-enum binary fields stay uninterpreted and diagnosed. This does
  not enable legacy programming.
  [Record](../status/2026-10-28-wp03-legacy-binary-retention.md) ·
  [KL-128](../limitations/KL-128-legacy-product-import-boundaries.md).

## Desktop and web

- **Pane separators no longer pick up text** (`1822a9756`). In the desktop app
  (WebKitGTK), dragging a pane separator could select document text, after which
  later separator drags lost their moves. Both separator kinds now block text
  selection and native drag. A bounded native re-check on a source build passed
  12 cells / 42 drags at 80, 100 and 150 % zoom.
  [Re-check](../status/2026-10-28-wp11-native-separator-recheck.md).
- **Fewer redundant language writes** (`5de6b9ff0`). Translated components no
  longer each rewrite the page's `lang` attribute on mount. In the 5000-device
  browser test case the production median fell from 41.8 s to 21.4 s, mostly in
  project creation; table rendering is unchanged and this is not a general speed
  claim. [Record](../status/2026-10-28-document-language-fanout.md).
- Bounded native acceptance cells (keyboard, save/cancel/replace, notes, zoom
  shortcuts, editor cells, capture export) were exercised on the published
  Alpha 17 AppImage with fictional data and loopback-only networking; the
  separator defect above was found that way. WP-11 stays open.
  [WP-11](../open-work/WP-11-native-linux-acceptance.md).

Test tooling only, not product features: production browser suites run in the
tagged lanes with case-level parallelism
([ADR-0154](../adr/0154-production-browser-suites-use-tag-lanes.md),
[ADR-0155](../adr/0155-production-functional-case-parallelism.md)), event-bound
gate fixtures and further GATE-01 follow-ups. A Docker image signing design was
written ([proposal](../research/2026-10-27-docker-image-signing-design.md)); it is
not implemented, and images remain unsigned.

## Changed defaults

- Experimental System-2 hardware waits are longer (see issue #30); ordinary
  downloads, monitoring and scans keep their timings.
- The UP258 validation workflow prepares with **defaults profile v3**. New
  physical execution of a v1 **or v2** bundle refuses before any socket; prepare
  the bundle again. Recovery of existing v1/v2 operations and simulator runs stay
  available. [Contract](../contracts/up258-system2-validation.md).
- Configured System-2 preparation and profile v3 refuse sources without an
  explicit `DownloadInvisibleParameters="Background"`.
- Multi-installation projects need an explicit installation for line-less
  catalogue device creation.
- No port, data-directory or Docker tag rule changed; `latest` still follows
  published releases, including alphas.

## Upgrade

Inspected source constants: **native/store schema 16** and **core model 16**,
unchanged from Alpha 17 (`crates/knx-store` and `crates/knx-core` have no source
changes since `v0.1.0-alpha.17`). The **product database moves from 28 to 30**
through two data-only migrations without DDL changes: v29 re-kinds `TypeDate`
parameters, v30 backfills legacy long-enum binary values from authenticated
retained source. Where history is ambiguous, the migrations keep the old values
and record a named refusal rather than guessing; v29 also marks the affected
install reports unavailable. Opening files saved by Alpha 18 in Alpha 17 is **not tested**.

What Alpha 17 refuses after an upgrade:

- **Product catalogue.** Writable catalogue open migrates the catalogue in place
  to v30: this includes server, desktop and Docker startup, and catalogue-writing
  CLI commands such as `knx products ingest` and legacy import. Read-only CLI
  readers migrate only an in-memory copy. Alpha 17 refuses a v30 catalogue as newer than
  it supports and starts without products; the file is not touched. There is no
  downgrade path: going back means restoring the pre-upgrade copy.
- **System-2 validation bundles and operations** prepared with defaults profile
  v3 are refused by Alpha 17, which admits only v1/v2 defaults bundles. New
  operations also carry an `execution-build.json` record that older builds do not
  read.
- **Final-step failure records.** A journal with a final-step failure record uses
  checkpoint envelope 3, which a reader that knows only envelopes 1 and 2 (the
  strict Alpha 17 reader, by source and a focused test) refuses instead of dropping
  the record. Journals Alpha 17 wrote carry no such record and reopen in Alpha 18
  with an empty list; nothing is backfilled. HTTP status schema 4 and report format 3
  version the status/report payloads, not the native project-file schema.
  Alpha 17 web clients do not read schema 4 or report format 3.

Device backups, activity history and native project files keep the formats Alpha
17 writes. Old operations stay available and gain no backfilled final-step facts.
A System-2 run settles the project's programming marks through the existing
per-part mechanism without changing the native project model. The final-step
journal entry and execution-build sidecar above are separate operation metadata.

Before upgrading, close KNXBench and keep independent copies of native projects,
the closed product catalogue and all Docker/server data directories, plus the
original `.knxproj`/`.knxprod` files. In-file history and backups do not replace
those copies; going back means restoring them. Update `knx-mcp` with the app.

## Boundaries

- **Not verified:** no physical-hardware certification, no recovery guarantee and
  no ETS equivalence follow from this release. All new tunnel, System-2,
  settlement and report behaviour is tested with simulated or loopback transports
  only; the issue #30/#22 allocation mismatch is unresolved, and the final-step
  category names the operation that stopped, not its cause. Opening Alpha 18 files
  or catalogues in Alpha 17 is not tested and the catalogue is refused.
- System-2 remains experimental and limited to UP 258D12 application
  `M-0001_A-9205-04-9270`. Masked-byte survival, allocator state and complete
  physical recovery remain open. The invisible-parameter reading is an
  observation-supported hypothesis, not a defined rule.
- CSV profiles make no claim of interoperability with any particular application;
  there is no export profile and no hierarchy or security roundtrip.
- KNXBench is KNX-compatible, not KNX-certified. No KNX Secure, new `.knxproj`
  export or broader device-family admission is added.

## Distribution

The GitHub pre-release `v0.1.0-alpha.18` carries `KNXBench_0.1.0-alpha.18_amd64.AppImage`,
`knx-mcp-x86_64-linux` and `SHA256SUMS`, which lists both binaries: verify a single
download with `sha256sum -c --ignore-missing SHA256SUMS`. Docker
`knxbench/knxbench-server:0.1.0-alpha.18` (without `v`) is published for
`linux/amd64` and `linux/arm64`; `latest` follows published releases, including
alphas. [Installation](../manual/getting-started/04-installation.md) ·
[Docker update](../manual/user-guide/11-web-and-docker.md#updating-a-docker-hub-installation) ·
[Known issues](../manual/known-issues.md).

## By the numbers

- **1 frozen release candidate**, **2 successful packaging dry runs** (AppImage/MCP and Docker amd64+arm64), and **0 publish jobs enabled in those dry runs**. We practised the brakes first.
- **7,196 Rust tests passed** and **4,654 frontend unit tests passed**; the Rust run also reports **238 ignored tests**, not executed coverage.
- **951 fixture-browser**, **567 production-browser**, **49 extra-browser** and **24 real-server evidence/edit-spec executions** passed. These are different, partly overlapping suites, not a unique-test total.
- **155/155 conventional corpus tests in 9 targets**, the additional product-corpus runs and **both historical pinned opt-ins** passed. Manufacturer inputs remain private and are not distributed. The tooling suite registered 466 tests with 6 existing optional skips.
- **1 counting hiccup**, fixed without replaying green product suites: Cargo selected five tests where our extra verifier expected four. The original refusal, compiled-inventory proof and bounded continuation were retained separately. The product did not need remedial arithmetic; its counter did.

## Acceptance and public verification

Published on **29 October 2026** from immutable
`80386a154231f327d60ba0d89f1ba6fa2719f137`, with the source cutoff and format
boundaries above unchanged. The [sanitized public receipt](../evidence/release-alpha18-2026-10-29.json)
separates the frozen source campaign, successful dry runs, actual tag publishers
and checks of the final public bytes. Original verifier refusals and their
bounded, read-only reconciliations remain retained; they are not relabelled as
an uninterrupted green runner or as fresh product-test executions.

- AppImage, MCP and checksum downloads match their published sizes and provider
  SHA-256 digests. MCP reports the tag's build prefix. The unchanged AppRun starts
  on an isolated Broadway display with the correct embedded-server version and
  HTTP 200 frontend; this is not FUSE, native visual, Mesa EGL or Orca acceptance.
- Both real tag publishers succeeded. Docker version and `latest` share one index;
  both native platform configs carry the exact revision and version labels.
  Local product execution was amd64 only; arm64 is distribution/identity proof.
- The network-isolated published amd64 image passed authentication/frontend,
  create/save/switch/reopen and a committed tiny password-protected synthetic
  import/save/switch/reopen. Retained imported entries match the source; the
  original archive remains unchanged. This is not blanket encrypted-ETS coverage.
- An authentic legacy schema 13 fixture present in the Alpha 17 Git tree opened,
  saved to a new schema 16 file and reopened with equal engineering content,
  19 common tables and integrity-checked stores. It is not evidence that Alpha 17
  wrote schema 13 or that current Alpha 17 native 16 files need a new format migration.
- **History boundary:** Save As to a new file starts its own saved-version archive,
  not a clone of the source file's archive. The tested original stayed unchanged;
  one prior saved version stayed in that original, not the new file. The probe
  checked history properties but did not execute undo/redo. Keep a closed-file
  copy for full-history backup; see [project history](../PROJECT_HISTORY.md).
- All 42 inventoried live website files, including `.nojekyll`, `CNAME` and the
  build manifest, plus the root alias matched the accepted release build.
  Website/story/workflow inputs equal the successful Pages revision. Its older
  deployment artifact is no longer available, so no ZIP/TAR-provider-digest
  verification is claimed; the public-byte comparison used the retained build.

Verification used isolated fictional projects and loopback. It did not access a
physical KNX installation or upgrade a running user deployment.
