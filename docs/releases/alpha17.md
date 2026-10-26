# Alpha 17 — pre-release

Alpha 17 covers source changes since `v0.1.0-alpha.16`. It answers issue #30 for
the experimental UP 258D12 System-2 download, makes the regular memory download
stricter about what it writes and claims, keeps the cause of failed downloads and
adds association links to comparison adoption. None of this is new hardware support.

## For reporters first

### Issue #30: System-2 download for UP 258D12 / `M-0001_A-9205-04-9270`

[Issue #30](https://github.com/KNXBench-Labs/KNXBench/issues/30) asked why the
experimental System-2 path was not offered. The scope is unchanged: **UP 258D12
application `M-0001_A-9205-04-9270` only, experimental, software-tested, hardware
outcome unverified.** Issue #30 stays open until its actual outcome is established.

- **Finding the mode.** The **Download mode** selector now sits directly below the
  device selector. The ordinary memory-route mark and notice apply to memory mode
  only and read *(memory download: not supported yet)*; an excluded device stays
  marked in both modes. The mark does not say System-2 works; the server decides.
- **A different device prestate no longer refuses (owner decision; Alpha 16
  refused a different application or a non-Loaded resource).** Before writing,
  only hardware type, mask version and manufacturer stay hard. A different serial
  number or installed application, a non-Loaded resource, a differing PID 7 table
  pointer or a refused prior memory read is listed under **Recorded device
  observations** with expected and observed values. Memory, final Loaded state and
  the written program version stay verified.
- **No fake backups.** When the captured prestate does not describe the device
  (differing shape or application, refused reads, all-zero or all-FF serial),
  original-state recovery is **unavailable**; zero placeholders are never offered
  as a backup. Recovery stays bound to the captured serial number.
- **Status says what happened.** **Refused before any write — nothing was
  written** appears only when the server confirms it. The gateway hint no longer
  calls the run's own locks a foreign management operation, and an early refusal
  shows the **reason reported by the server** (a stable error kind), for the
  current server run only. `gatewayBusy` still covers both shared-lock conflicts
  and process-lease failures.
- **PID 7 pointer width.** A new revision of the exact-UP258 configured hardware
  record admits one element of **two or four octets** on objects 1/2/3 and keeps
  the raw width through capture, private recovery records, reopening and explicit
  recovery. Object 1 has one reported two-octet field observation; objects 2/3 are
  a source-declared hypothesis. Operations recorded under Alpha 16 keep their
  original four-octet interpretation. Wrong-width refusals name object, element
  count, byte count and at most 16 octets, in the private journal only.
  [Width record](../status/2026-10-26-up258-pid7-width.md).
- **Validation workflow.** Defaults profile v2 writes the 15 independent source
  octets beyond the declared association-table capacity; a new physical run with
  a v1 bundle refuses before connecting. `knx device system2-validation` records
  its own activity kind, journals refusals before the send boundary as `notSent`
  and keeps acknowledging tunnel frames while checkpoints are written.

PART-B-PENDING: the S2-01 safe diagnostic projection (structured, privacy-bounded
failure reason in status, report and EN/DE UI) is summarized here from its own
delivered commits and contract once it lands.

Do not post private journals, memory or bundles in a public issue. S2-02 (the
original start failure) and S2-03 (two image-interpretation differences) remain open.
[Service](../contracts/alpha16-system2-configured-download.md) · [UI](../contracts/alpha16-system2-ui.md) ·
[HTTP](../contracts/alpha16-system2-http-mount.md) · [Validation](../contracts/up258-system2-validation.md) ·
[Guide](../manual/user-guide/05-devices-and-products.md#experimental-configured-system-2-download-alpha-16) ·
[S2-01](../open-work/S2-01-pid7-raw-response.md) · [S2-02](../open-work/S2-02-gateway-busy-after-failed-runs.md) ·
[S2-03](../open-work/S2-03-up258-image-interpretation.md).

### Issue #22: Siemens presence detector (`MV-0025`)

Nothing new since Alpha 16 for issue #22 itself. The issue #30 changes above
touch the same experimental UP 258D12 path, but no hardware result exists and
issue #22 stays open.

## Regular download and programming correctness

Software-tested refusals and observations on the existing regular memory download;
no device family or hardware claim is added ([KL-007](../limitations/KL-007-bounded-commissioning-support.md)).

- **Identity widths.** PID 78 hardware type must answer exactly six octets and the
  partial-download application identity (object 3, PID 13) exactly five, before
  backup or mutation. [Hardware](../contracts/hardware-identity-comparison.md) ·
  [application](../contracts/partial-identity-comparison.md).
- **Scope advice.** The download tab gets a fifth choice, **Only what the project
  marks pending**, and the CLI gets `knx device download --changes` (plans only).
  The programming flags resolve to a concrete scope or to nothing; they are not
  read from the device. [Contract](../contracts/device-programming-support-guidance.md).
- **Read-only check after the download.** A read-only session on the same tunnel,
  while the download still holds the gateway, observes reach, identity and load
  states, kept apart from transfer and restart in CLI, status, report, history and
  EN/DE views. Runtime stays unknown; flags do not
  change; restore does not run it. [Contract](../contracts/post-download-check.md).
- **Backups record the prior application.** Backed-up executions store the actual
  pre-write application identity in **backup format 2**; restore refuses a backup
  whose bytes belong to another or an unrecorded application, unless the partial
  plan carries its own exact identity check. Complete downloads over a different
  application stay possible. [Contract](../contracts/backup-application-identity.md).
- **Settlement checks the source.** A finished download credits programmed parts
  only if the current catalogue still plans the identical download
  ([contract](../contracts/download-settlement-source-freshness.md)). History rows
  show a run-scoped receipt of the plan and the settlement, including *not applied*
  when saving failed ([contract](../contracts/download-settlement-receipt.md)).
- **Stop before the first change.** A server shutdown stops a regular download only
  before its first mutation; afterwards the run continues unchanged.
  [ADR-0153](../adr/0153-regular-download-stop-ends-at-first-change.md).
- **Programs with their own ETS download plug-in are refused** for every scope and
  restore (`vendor-download-plugin`); read-only compare and diagnostics still work
  ([KL-006](../limitations/KL-006-devices-dependent-on-vendor-plug-ins.md)).
- **Wire budgets and replies.** Region reads keep one memory service and its count
  limit; user memory carries one data octet less; an unproved hardware route is
  capped at 12 data octets per request, which can slow transfers. Other-service
  memory replies are ignored, a load-control write response's load state is
  checked, and frames longer than a standard frame clear the cEMI standard-frame bit.
  [Budget](../contracts/memory-transfer-budget.md) ·
  [reads](../contracts/memory-region-read-service.md) ·
  [replies](../contracts/memory-response-service-pairing.md) ·
  [load state](../contracts/load-control-response-state.md).
- **Planner refusals instead of silent loss:** untranslated `MergeId` fragments
  beside a regular procedure, two data segments starting at one allocation, and
  declared `TypeNumber` bounds that cannot be read.
  [Fragments](../contracts/regular-merge-fragment-admission.md) ·
  [segments](../contracts/duplicate-image-segment-start.md) ·
  [bounds](../contracts/parameter-number-bound-admission.md).
- **CLI catalogue reads stay read-only.** `device download`, `compare`,
  `restore`, `byte-order-probe` and `device diagnostics --product-db` no longer
  migrate an older default catalogue in place.

## Download failures say why

A failed regular download keeps one payload-free cause: category, code, phase,
step, restart uncertainty and octets read back unchanged. Server status, the
session log, durable activity history and the tester's report carry it, and the
panel shows **Download failed** or **Download stopped on request**. `knx device
download` and `knx device restore` print one `diagnostic:` line and record the
same value. Error text, memory octets, keys, gateway and backup paths stay out of
report and history. System-2 runs are outside this contract.
[Contract](../contracts/download-failure-diagnostic.md).

`knx device download`, `restore` and `service-control` now run on the same
multi-thread tunnel runtime as the System-2 commands, so tunnel acknowledgements
continue while backups are written. Backup and send barriers are unchanged.
[Acceptance and limits](../status/2026-10-26-cli-durable-tunnel-runtime.md).

## Diagnostics

- A device's **Diagnostics** tab offers **Analyse device** and **Send report**,
  opening the existing dialogs with that device selected; consent is unchanged.
- The web diagnostic dialog adds the read-only `system2-declared-segments-v1`
  profile; memory is read only after the device proves its identity.
- `knx device diagnostics --device <area.line.device>` selects by address; unknown
  or shared addresses refuse. [Contract](../contracts/device-diagnostic-export.md).

## Workbench

- **Comparison adoption of association links.** Association rows from a complete,
  decoded mask-`0701h` association table can be adopted as object ↔ group-address
  links, per whole object, with a preview of link changes in EN/DE. No group
  address or object is created and nothing is written to a device.
  [Amendment](../contracts/alpha16-comparison-adoption.md#association-link-adoption-wp-19-amendment-24-october-2026).
- **Saved object folders.** The object table names the saved project folder of
  each object by its declared `RefId` chain and offers a **By saved project
  folder** view; folder names are never guessed.
  [Contract](../contracts/communication-object-folders.md).
- **Flow map** shows the single unambiguous project group name after an address
  ([record](../status/2026-10-26-flow-group-names.md)); **project history** shows
  native versions on a timeline, where selection alone never writes
  ([record](../status/2026-10-26-project-version-timeline.md)).
- **Catalogue table** columns Product, Manufacturer, Order number and Application
  sort on click. [Contract](../contracts/catalogue-table.md).
- **Matrix object columns** share one resizable, remembered width. **Desktop
  AppImage** save and open dialogs start in the launch directory, not the image mount.

## Data integrity

Project and product ZIP readers refuse contradictory WinZip AES metadata and
strong-encryption flags without the encryption flag. Consistent AES remains
unsupported. [Admission](../contracts/project-encrypted-directory-admission.md).
Test tooling only, not product features: a KNXnet/IP simulator gateway for
loopback tests, plus gate and browser-runner hardening.

## Upgrade

Inspected source constants are unchanged from Alpha 16: **native/store schema 16**,
**core model 16**, **product database 28**. No migration is needed. Opening files
saved by Alpha 17 in Alpha 16 is **not tested**.

New record versions live outside the native project file, and Alpha 16 refuses
them rather than dropping data:

- **Device backups** (CLI `<project>.backups/`, server `<data_dir>/device-backups/`)
  are now written as format 2. Alpha 16 only knows format 1 and refuses format 2
  files. Alpha 17 still reads format 1 backups; complete backups among them no
  longer restore, legacy partial backups restore through their plan's identity check.
- **Activity history** (`activity-history.sqlite` in the data directory) keeps
  storage format 2, but rows with the new optional fields or the new
  `system2Validation` kind make an older reader refuse the page and treat history
  as unavailable. Nothing is rewritten. [Contract](../contracts/COMMISSIONING_ACTIVITY_HISTORY.md).
- **System-2 operation records** of the new width revision or with recorded
  observations are refused by an Alpha 16 binary. Operations recorded by Alpha 16
  still load and keep their original interpretation.

Before upgrading, close KNXBench and keep independent copies of native projects,
the closed product catalogue and all Docker/server data directories, plus the
original `.knxproj`/`.knxprod` files. In-file history and backups do not replace
those copies; going back means restoring them. Update `knx-mcp` with the app.

## Boundaries

- **Not verified:** no physical-hardware certification, no recovery guarantee and
  no ETS equivalence follow from this release. All new download, System-2 and
  diagnostic behaviour is tested with simulated or loopback transports only.
- System-2 remains experimental and limited to UP 258D12 application
  `M-0001_A-9205-04-9270`. Masked-byte survival, allocator state and complete
  physical recovery remain open; `gatewayBusy` does not identify its mechanism.
- KNXBench is KNX-compatible, not KNX-certified. No KNX Secure, new `.knxproj`
  export or broader device-family admission is added.

## Distribution

The GitHub pre-release `v0.1.0-alpha.17` carries `KNXBench_0.1.0-alpha.17_amd64.AppImage`,
`knx-mcp-x86_64-linux` and `SHA256SUMS`, which lists both binaries: verify a single
download with `sha256sum -c --ignore-missing SHA256SUMS`. Docker
`knxbench/knxbench-server:0.1.0-alpha.17` (without `v`) is published for
`linux/amd64` and `linux/arm64`; `latest` follows published releases, including
alphas. [Installation](../manual/getting-started/04-installation.md) ·
[Docker update](../manual/user-guide/11-web-and-docker.md#updating-a-docker-hub-installation) ·
[Known issues](../manual/known-issues.md).
