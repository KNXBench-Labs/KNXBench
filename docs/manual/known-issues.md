← Previous: [FAQ](reference/04-faq.md) · [Manual index](README.md)

# Known issues

**Current-source review: 8 October 2026, `608a204b`.** This page separates
remaining defects, intentional safety/scope boundaries and missing evidence.
Historical titles do not mean that their old defect is still present. The
[source/test audit](../status/2026-10-08-known-issues-status-audit.md) records
the reconciliation; formal task dispositions remain in the engineering ledger.

## Read these before your first serious project

| Boundary | Practical meaning | Next check |
| --- | --- | --- |
| Import is one-way | Keep the original ETS archive; your edited working copy is `.knxdb`, not an ETS export | [Projects](user-guide/02-projects.md#there-is-no-knxproj-export) |
| Autosave is not a backup history | Save once to choose a file, then keep independent copies | [Saving](user-guide/02-projects.md#saving) |
| Browser files live on the server | Save is not a download; Export project gives you a local copy | [Web and desktop differences](user-guide/11-web-and-docker.md#web-build-versus-desktop-build) |
| Shared server, shared project | One password and one undo stack are not collaborative editing | [Deployment](user-guide/11-web-and-docker.md#authentication) |
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

This is the source-level `0.1.0-alpha` series, checked on **8 October 2026**.
The [public alpha.6 pre-release](https://github.com/KNXBench-Labs/KNXBench/releases/tag/v0.1.0-alpha.6)
(AppImage, `knx-mcp`, Docker Hub image) may precede newer changes described here;
the `TypeNone` spacer fix, for example, landed on `main` after the tag. The version number is not a
promise that anything is finished.

> A long list of known issues is what happens when a project writes its
> problems down, not proof that it has more of them than software that
> writes nothing down. Judge the entries, not their number.

---

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

- **Affected:** importing into an existing project.
- **Limitation:** import is whole-file. You cannot pull one line, one
  building part or one device out of another `.knxproj` and merge it into a
  project you already have.
- **Consequence:** the common ETS habit of copying a working subsystem out of
  an old project has no equivalent here.
- **Workaround:** none.
- **Details:** [`docs/GAP_ANALYSIS_ETS.md`](https://github.com/KNXBench-Labs/KNXBench/blob/a584007fc05a/docs/GAP_ANALYSIS_ETS.md), row C3

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
  [UI follow-up evidence](https://github.com/KNXBench-Labs/KNXBench/blob/bca2d3336b96/docs/archive/alpha-0.1/UI_ALPHA_READINESS.md)

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
  legacy file over 128 MiB (payload over 256 MiB) is still refused. Successful
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
  its application-specific settings are opaque.
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
  legacy DPT codes. Parameters
  of the legacy `string` and `long enum` types (1,515 in the measured VD5)
  are reported, not shown. A large VD5 takes 20–40 s and over 1 GiB of
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
- **Limitation:** explicit, preview-confirmed readdress/delete is supported,
  but import does not create or restructure group ranges. Derived columns are
  validated read-only, and there are no description or comment columns.
- **Consequence:** CSV can perform safe bulk address edits, but it is not a
  range-hierarchy or replacement/synchronisation format. Missing rows never
  imply deletion.
- **Details:** [§39](../KNOWN_LIMITATIONS.md#39-csv-import-never-re-addresses-deletes-or-manages-group-ranges),
  [§40](../KNOWN_LIMITATIONS.md#40-csv-export-only-columns-are-never-applied-on-import-and-there-are-no-descriptioncomment-columns)

### The CSV format is KNXBench's own

- **Affected:** exchanging group addresses with ETS.
- **Limitation:** the columns are this application's design. No
  interoperability with ETS's own group-address CSV or XML export has been
  verified.
- **Consequence:** do not assume an ETS import will accept the file.
- **Workaround:** a spreadsheet re-arranged by hand, if you need the other
  direction.
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
  [commissioning goal](https://github.com/KNXBench-Labs/KNXBench/blob/bca2d3336b96/docs/archive/alpha-0.1/goal-commission.md).

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

### Device download is verified on one device only

- **Affected:** writing a configuration into devices ("download").
- **Limitation:** `knx device download` and the **Download to device** tab
  work, but only two devices (an MDT push button and an Eibmarkt presence
  detector, both mask `0701h`) have been downloaded and read back so far. Other devices, application versions and
  masks are unverified; procedures KNXBench cannot plan are refused by name.
  Programming an individual address is currently refused until durable
  recovery exists. Unloading and secure devices are not supported. A successful
  byte read-back does not confirm the final restart; the verified devices'
  restart outcome is explicitly unconfirmed.
- **Consequence:** for most installations, plan for another commissioning
  tool for now.
- **Workaround:** none beyond the verified scope.
- **Details:** [§7 commissioning](../KNOWN_LIMITATIONS.md#7-commissioning-and-device-download-are-required-but-blocked),
  [Downloading to a device](user-guide/07-bus-and-interfaces.md#downloading-to-a-device)

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

### Opening an older project upgrades the file

- **Affected:** a project saved by an older KNXBench version.
- **Limitation:** normal application/project readers and CLI commands such as
  `knx diff` and `knx doc-export`
  upgrades the file itself to the current format. No copy is made, and the
  older KNXBench then refuses the file as "newer". A failed upgrade leaves the
  file unchanged.
- **Exception:** the read-only MCP adapter migrates an older database's
  in-memory copy, not the source file. It sees only saved state, not unsaved UI edits.
- **Workaround:** copy the project file first if an older version must still
  open it.
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
  (`-p 8484:8080`) cannot reach it — the connection is refused. This is the
  server's deliberate refusal to expose an unguarded API, not a bug.
- **Consequence:** the published-port form only works with a password
  configured. With one, it works: build, `-p 8484:8080`, `KNX_AUTH_PASSWORD`,
  sign in, API answers `200` — evidenced by the earlier image run, not repeated
  by this audit. Current authenticated builds default to HTTPS; follow the
  [certificate/start recipe](user-guide/11-web-and-docker.md#https).
- **Workaround:** set `KNX_AUTH_PASSWORD_HASH` (or `KNX_AUTH_PASSWORD`), or on
  Linux run the credential-less form on the host network instead:
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

[Manual index](README.md) · Next: [Implementation status](implementation-status.md) →
