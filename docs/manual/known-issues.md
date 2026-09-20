← Previous: [FAQ](reference/04-faq.md) · [Manual index](README.md)

# Known issues

This chapter collects the limitations most likely to be noticed by someone
who actually uses KNXBench, grouped by the part of the application they show
up in. Each entry says what is affected, what the limitation is, what it
means in practice, and whether a workaround exists.

It is a selection, not the catalogue. The full engineering record lives in
[`docs/KNOWN_LIMITATIONS.md`](../KNOWN_LIMITATIONS.md), which currently holds
119 numbered entries — including ones that are already resolved, ones that
only a maintainer would care about, and ones about the reasoning behind a
design decision rather than about a defect. Where an entry below has a
counterpart there, the **Details** line links straight to it.

The software described here is version `0.1.0-alpha.1`. No release has been
published, and the version number is not a promise that anything is finished.

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
  import itself completes and the project is fully usable; in the measured
  case all 2 areas, 2 lines, 4 devices, 13 group addresses and 75
  communication objects arrived.
- **Workaround:** read the error, confirm it is this one, and continue. There
  is nothing to repair in the project.

### Everything KNXBench knows about `.knxproj` comes from a small corpus

- **Affected:** import of any project not resembling the ones this project has
  measured.
- **Limitation:** the parser was built against a handful of real files (an
  ETS4 schema-11 project, an ETS 6.3.0 schema-23 project, a vendor schema-21
  demo). No authoritative XSD for the format is publicly available, so
  "correct" here means "agrees with the files we have".
- **Consequence:** a construct that never appeared in those files may be
  reported as unknown, or mapped conservatively, even though ETS considers it
  ordinary.
- **Workaround:** none, but nothing is silently dropped — unknown attributes
  and elements are preserved and listed in the import report.
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
- **Details:** [`docs/GAP_ANALYSIS_ETS.md`](../GAP_ANALYSIS_ETS.md), row C3

## Export

### Every export is unsigned, and no real ETS has ever re-imported one

- **Affected:** exporting a `.knxproj`.
- **Limitation:** KNXBench does not sign the containers it writes, and
  whether a real ETS installation accepts an unsigned third-party
  `.knxproj` has never been tested. Every export carries this warning; it is
  constructed before anything else can fail, so no export exists without it.
  Signature entries copied through from the source file are reported as
  stale, because they no longer match what they sign.
- **Consequence:** treat export as a one-way door until you have verified it
  against your own ETS. It may work. Nobody here can tell you that it does.
- **Workaround:** keep the original file. Do not make an exported container
  the only copy of anything.
- **Details:** [§5 exports are unsigned](../KNOWN_LIMITATIONS.md#5-exports-are-unsigned-and-ets-acceptance-is-untested),
  [§85 a `.signature` member is stored, never verified](../KNOWN_LIMITATIONS.md#85-a-signature-package-member-is-stored-with-role-signature-never-verified)

### A round trip is semantic, not byte-exact

- **Affected:** import followed by export of the same project.
- **Limitation:** the exported file carries the same meaning as the imported
  one, not the same bytes. Element order, formatting and other incidental
  details are regenerated.
- **Consequence:** `diff` between the original and the export is not a
  useful check, and the two files will not have the same checksum.
- **Details:** [§4 round trips are semantic, not byte-exact](../KNOWN_LIMITATIONS.md#4-round-trips-are-semantic-not-byte-exact)

### Read on init is editable, and is not exported

- **Affected:** the sixth communication flag, `I`, when you set it yourself.
- **Limitation:** the flag is stored, undoable and persisted in the `.knxdb`
  project file, but no measured ETS project file states it per communication
  object, so the exporter does not write it.
- **Consequence:** it does not survive a trip through `.knxproj`. The export
  warns and names how many communication objects are affected, so the loss is
  never silent.
- **Workaround:** keep the native `.knxdb` as the master copy. The behaviour
  is described in full under
  [communication objects](user-guide/05-devices-and-products.md#communication-objects).
- **Details:** [§117 `read_on_init_flag`](../KNOWN_LIMITATIONS.md#117-read_on_init_flag-is-parsed-and-stored-then-discarded-before-it-reaches-knx-core)

### Exporting at schema 21 or newer drops a few known-but-unmapped attributes

- **Affected:** projects imported from ETS5/ETS6 and exported again.
- **Limitation:** a small set of per-device and per-line attributes that the
  newer schemas carry are recognised on import but not written back.
- **Consequence:** a narrow, named category of information does not survive
  the round trip.
- **Workaround:** none; the attributes are listed in the linked entry.
- **Details:** [§34 schema-21 export drops a handful of attributes](../KNOWN_LIMITATIONS.md#34-schema-21-export-drops-a-handful-of-known-but-unmapped-per-deviceper-line-attributes)

## Devices and product data

### The catalog file picker offers `.vd2`, and `.vd2` is always rejected

- **Affected:** installing a product database.
- **Limitation:** the catalog browser's file input accepts
  `.knxprod`, `.vd2` and `application/zip`, and the command-line help lists
  `file.vd2` as an argument. The package reader rejects every file whose name
  ends in `.vd2` before looking inside it — the legacy ETS3 format is out of
  scope by decision, not by accident.
- **Consequence:** you can select a `.vd2` file and will then be told it
  cannot be imported. The refusal is deliberate; the offer is a leftover.
- **Workaround:** obtain the product as a `.knxprod` package.
- **Details:** [`docs/COMPATIBILITY.md`](../COMPATIBILITY.md)

### `.knxprod` packages at master-data scheme 12 and newer are not installable directly

- **Affected:** product data for newer devices.
- **Limitation:** a standalone `.knxprod` package installs at master-data
  scheme 11 or 20, verified against four real files. No tested sample exists
  at schemes 12 to 19, 21 or 22, so those are not supported. Encrypted
  packages are refused permanently, by decision rather than by omission.
- **Consequence:** for many current devices the manufacturer's own download
  cannot be installed.
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
- **Limitation:** resolution works for communication objects; three
  documented gaps remain elsewhere.
- **Consequence:** a device may show an identifier where you expect a
  product name.
- **Details:** [§12 manufacturer data resolution](../KNOWN_LIMITATIONS.md#12-manufacturer-data-resolution--lifted-for-communication-objects-three-gaps-remain)

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

### There is no drag and drop anywhere

- **Affected:** the whole user interface.
- **Limitation:** no tree item, device or group address can be moved by
  dragging, and the file picker takes no dropped files.
- **Consequence:** every structural move is done through menus, buttons and
  inline editing.
- **Details:** [§24 `FsPicker` has no drag and drop](../KNOWN_LIMITATIONS.md#24-fspicker-has-no-drag-and-drop-or-multi-select),
  [`docs/GAP_ANALYSIS_ETS.md`](../GAP_ANALYSIS_ETS.md), row B10

## Group addresses

### The New project dialog promises a restyle that does not exist

- **Affected:** choosing a group-address style when creating a project.
- **Limitation:** the dialog's hint reads "Pick the one you think in; the
  project properties can restyle it later." The project properties cannot.
  The group-address style is shown there read-only, and the server route
  that would change it has no entry point in the user interface.
- **Consequence:** the style you choose when you create the project is the
  style you keep.
- **Workaround:** choose deliberately at creation time. There is no way back
  through the interface.

### CSV import never re-addresses, deletes, or manages ranges

- **Affected:** the group-address CSV import.
- **Limitation:** import adds and updates; it does not move an address,
  delete one, or create or restructure group ranges. Export-only columns are
  ignored on the way back in, and there are no description or comment
  columns.
- **Consequence:** CSV is a bulk-entry tool, not a synchronisation tool.
  Round-tripping a file through a spreadsheet will not reproduce everything
  the export showed.
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

### Commissioning cannot program a device

- **Affected:** everything a user would call "downloading to the bus".
- **Limitation:** the commissioning protocol exists in the core library and
  is verified against a simulator this project wrote. It has never addressed
  a real device, and there is no user-facing command that would let it. Read-
  only verification against a real installation is the furthest this has gone.
- **Consequence:** KNXBench cannot commission your installation. Plan for ETS
  to do that.
- **Workaround:** none.
- **Details:** [§7 commissioning and device download are blocked](../KNOWN_LIMITATIONS.md#7-commissioning-and-device-download-are-required-but-blocked),
  [§92 phase 2 is verified against a simulator](../KNOWN_LIMITATIONS.md#92-commissioning-phase-2-is-verified-against-a-simulator-this-project-wrote-and-has-never-addressed-a-device)

### KNX Secure is not implemented

- **Affected:** secured installations, both Data Secure and IP Secure.
- **Limitation:** not implemented. IP Secure was scoped and then shelved
  indefinitely.
- **Consequence:** a secured installation cannot be monitored or worked on
  over the bus features here, and secured projects are outside what this
  application handles.
- **Details:** [§8 KNX Secure is not implemented](../KNOWN_LIMITATIONS.md#8-knx-secure-is-not-implemented),
  [§26 `BusConnection` does not support KNX IP Secure](../KNOWN_LIMITATIONS.md#26-busconnection-does-not-yet-support-knx-ip-secure)

### The DPT codec infers the format of what you type, and some encodings are this project's ruling

- **Affected:** writing a value to a group address, and reading the decoded
  value of a telegram.
- **Limitation:** the codec covers thirty main types. It infers the format of
  the input rather than being told it, and for several encoding questions the
  KNX standard's printed text is ambiguous enough that this project made a
  documented ruling instead of following it literally.
- **Consequence:** a misread input produces a valid telegram carrying the
  wrong value. The bus cannot tell you that it was wrong.
- **Workaround:** check the write confirmation, which echoes the decoded
  value rather than only the raw bytes.
- **Details:** [§61 the DPT codec](../KNOWN_LIMITATIONS.md#61-the-dpt-codec-covers-thirty-main-types-infers-rather-than-reads-its-input-and-leaves-several-encoding-questions-to-a-stated-ruling-rather-than-the-standard),
  [§95 six places where the printed standard must not be followed literally](../KNOWN_LIMITATIONS.md#95-six-places-where-the-knx-standards-printed-text-must-not-be-followed-literally)

### The group monitor is tunnelling only, one session at a time

- **Affected:** the bus monitor in the user interface.
- **Limitation:** tunnelling connections only — no routing, no discovery from
  the monitor — a single session, and filters applied in the browser rather
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

### A running monitor keeps the group-address style it started with

- **Affected:** a bus session left open while the project changes.
- **Limitation:** telegram rows are rendered in the style the project had when
  the session started.
- **Consequence:** the monitor and the tree can disagree about how an address
  is written.
- **Workaround:** restart the session.
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
- **Limitation:** there is no TLS, one password rather than accounts, no
  roles, no audit trail, and no CSRF protection. A failed attempt is delayed,
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
  sign in, API answers `200` — verified end to end.
- **Workaround:** set `KNX_AUTH_PASSWORD_HASH` (or `KNX_AUTH_PASSWORD`), or on
  Linux run the credential-less form on the host network instead:
  `docker run -d --name knxbench --network host -e KNX_PORT=8484 -v "$(pwd)/data:/data" knxbench-server`.
  That form is reachable from that machine only, which is also why it is safe
  without a password.

### Discovery does not work inside Docker's default network

- **Affected:** KNXnet/IP gateway discovery from a container.
- **Limitation:** discovery needs IP multicast, which the default bridge
  network does not carry.
- **Workaround:** the host network, as above, or configure the gateway
  address manually.
- **Details:** [§79 discovery needs IP multicast](../KNOWN_LIMITATIONS.md#79-discovery-needs-ip-multicast-which-dockers-default-bridge-network-does-not-carry)

### One project, one undo stack, no matter how many browsers

- **Affected:** two people opening the same server.
- **Limitation:** there is no multi-user support of any kind: one shared
  project, one shared undo stack, no conflict detection.
- **Consequence:** a second browser is a second pair of hands on the same
  keyboard. One person's undo reverses the other person's edit.
- **Workaround:** one editor at a time. This is a discipline, not a lock —
  nothing enforces it.
- **Details:** [§63 no multi-user support](../KNOWN_LIMITATIONS.md#63-knx-server-has-no-multi-userconcurrent-edit-support--one-shared-project-one-shared-undo-stack-no-conflict-detection-at-all)

### The project file cannot be downloaded through the browser

- **Affected:** getting a `.knxdb` off a remote server.
- **Limitation:** the server has a download route; nothing in the frontend
  calls it. "Save as" writes into the server's own data directory.
- **Consequence:** your project lives where the server lives. Copy it off with
  the tools you use for that machine.
- **Details:** [§30 `/api/project/download` has no frontend caller](../KNOWN_LIMITATIONS.md#30-apiprojectdownload-has-no-frontend-caller)

### A browser that loses the import response has to reload

- **Affected:** importing a large project.
- **Limitation:** if the response that carries the imported project is lost —
  a closed tab, a dropped connection — the project cannot be re-fetched.
- **Workaround:** reload the page; the project was stored server-side.
- **Details:** [§96 a browser that loses the import response](../KNOWN_LIMITATIONS.md#96-a-browser-that-loses-the-import-response-cannot-get-the-project-back-without-reloading)

## Desktop

### One host, one architecture, one build

- **Affected:** the desktop application.
- **Limitation:** the AppImage has been built and launched on a single Linux
  host. It is unsigned, has no auto-update, is x86-64 only, and there are no
  distribution packages.
- **Consequence:** treat the desktop build as an early convenience, not as a
  supported installation route.
- **Workaround:** build from source, or run the server and use a browser.

### The desktop shell has no login, on purpose

- **Affected:** nothing you can do about it, but worth knowing.
- **Limitation:** the desktop application talks to an in-process server that
  is deliberately not behind the password check — there is no network
  exposure to protect.
- **Consequence:** anyone at the machine has the project. The protection is
  the machine's own.

### The Linux backend depends on archived GTK3 bindings

- **Affected:** the desktop build's long-term maintenance.
- **Limitation:** Tauri v2's Linux backend uses Rust GTK3 bindings whose
  upstream repository is archived.
- **Consequence:** a future distribution change could break the desktop build
  in a way this project cannot fix quickly.
- **Details:** [§16 Tauri v2's Linux backend](../KNOWN_LIMITATIONS.md#16-tauri-v2s-linux-backend-depends-on-archived-gtk3-bindings)

## Reports and comparison

### Documentation export is HTML, and less than ETS's

- **Affected:** the project documentation export.
- **Limitation:** HTML only — no native PDF, no print preview, no section
  selection, one language per export. It does not resolve manufacturer,
  product or program names, and does not list parameter values or module
  instance arguments. No parity with ETS's reports has been measured, and
  with no ETS installation here it cannot be.
- **Consequence:** useful as a record, not as a substitute for an ETS report.
- **Workaround:** print the HTML from a browser for PDF.
- **Details:** [§44](../KNOWN_LIMITATIONS.md#44-project-documentation-export-t13-has-no-ets-report-parity-and-none-can-currently-be-measured),
  [§45](../KNOWN_LIMITATIONS.md#45-project-documentation-export-has-no-native-pdf-output),
  [§46](../KNOWN_LIMITATIONS.md#46-project-documentation-export-does-not-resolve-manufacturer-product-or-program-names),
  [§47](../KNOWN_LIMITATIONS.md#47-project-documentation-export-does-not-list-parameter-values-or-module-instance-arguments),
  [§48](../KNOWN_LIMITATIONS.md#48-project-documentation-export-renders-in-one-language-only),
  [§49](../KNOWN_LIMITATIONS.md#49-project-documentation-export-has-no-in-application-print-preview),
  [§50](../KNOWN_LIMITATIONS.md#50-project-documentation-export-has-no-section-selection)

### Project comparison tells you what changed, rarely what it became

- **Affected:** comparing two projects.
- **Limitation:** for most entity types the renderers name the fields that
  differ rather than showing before and after values, and the web panel shows
  grouped counts only. A comparison cannot read a raw `.knxproj`, cannot
  merge or apply a difference back onto a project, does not do a three-way
  comparison, and has no exit code for use in a pipeline.
- **Consequence:** it answers "did anything change, and where", not "what
  exactly does it say now".
- **Details:** [§55](../KNOWN_LIMITATIONS.md#55-project-diff-cannot-merge-or-apply-a-diff-back-onto-a-project),
  [§56](../KNOWN_LIMITATIONS.md#56-project-diff-does-not-do-a-three-way-comparison),
  [§57](../KNOWN_LIMITATIONS.md#57-project-diff-cannot-compare-against-a-raw-knxproj),
  [§58](../KNOWN_LIMITATIONS.md#58-project-diff-has-no-ci-friendly-exit-nonzero-on-any-difference-flag),
  [§59](../KNOWN_LIMITATIONS.md#59-project-diffs-text-and-web-renderers-show-which-fields-changed-not-their-beforeafter-values-for-most-entity-types),
  [§60](../KNOWN_LIMITATIONS.md#60-project-diffs-web-panel-shows-grouped-counts-only)

## Language and accessibility

### The interface is English and German; some text is neither

- **Affected:** everyone not reading English.
- **Limitation:** the interface ships English and German. Translations
  imported from product data are stored but not used to translate the
  interface, and `Languages` blocks outside an application program are
  discarded on import. Documentation export renders in one language.
- **Details:** [§37 imported translations are stored but never read](../KNOWN_LIMITATIONS.md#37-imported-translations-are-stored-but-never-read-and-the-ui-is-english-only--partially-resolved-2026-09-12),
  [§64 `Languages` blocks outside an application program](../KNOWN_LIMITATIONS.md#64-languages-blocks-outside-an-application-program-are-discarded-on-import),
  [§66 server-composed prose and documentation export](../KNOWN_LIMITATIONS.md#66-server-composed-prose-and-the-documentation-export-are-not-translated-by-any-ui-language-or-pack--partially-resolved-2026-09-14-t14)

### Screen reader support is incomplete

- **Affected:** use with assistive technology.
- **Limitation:** accessibility work has closed some gaps and left others. A
  project load that succeeds announces nothing to a screen reader, and the
  command palette and search share an overlay with a remaining gap.
- **Consequence:** the application has not been validated with assistive
  technology end to end. It is not claimed to be accessible.
- **Details:** [§20 command palette and search overlay](../KNOWN_LIMITATIONS.md#20-command-palette-and-search-share-overlay-css-and-an-accessibility-gap--partially-resolved),
  [§118 a succeeded project load announces nothing](../KNOWN_LIMITATIONS.md#118-a-succeeded-project-load-announces-nothing-to-a-screen-reader)

### Animations follow the operating system, with no in-application switch

- **Affected:** motion in the interface.
- **Limitation:** the reduced-motion preference of the operating system is
  respected; there is no setting inside the application.
- **Details:** [§43 animations have no in-app switch](../KNOWN_LIMITATIONS.md#43-animations-have-no-in-app-switch-only-the-os-reduced-motion-preference)

## Documentation defects

Documentation can be wrong too, and when it is, that is a defect like any
other. These are the ones found while writing this manual.

### `COMPATIBILITY.md` understates datapoint type coverage

- **Affected:** readers of the compatibility document.
- **Limitation:** its section 2 excludes "main types this codec does not
  implement (20 and above)". The codec implements and tests main types up to
  30 — types 20, 21, 25, 29 and 30 among them.
- **Consequence:** the document is more pessimistic than the software. Where
  the two disagree about DPT coverage, the code is right.
- **Details:** [`docs/COMPATIBILITY.md`](../COMPATIBILITY.md), section 2

### Some repository documents predate the server login

- **Affected:** statements about authentication elsewhere in the repository.
- **Limitation:** the manual was rewritten on 2026-09-20 for the password
  login and the browser login screen that arrived the same day. Older
  engineering documents — `docs/PROJECT_ANALYSIS_2026-09-15.md`, for one —
  still describe a server with no authentication at all, because that was
  true when they were written. The internal triage list also still carries
  the Read-on-Init entry as open, although it was closed the same day.
- **Consequence:** where a dated analysis document contradicts this chapter
  about authentication or about the sixth flag, this chapter is the newer one.
  [`KNOWN_LIMITATIONS.md` §22](../KNOWN_LIMITATIONS.md#22-knx-server-authenticates-with-one-password-or-refuses-to-leave-loopback)
  is current and is the engineering record.

---

If you find something this chapter does not mention, it is worth reporting
even if you suspect it is known. The list above is what has been found, which
is not the same as what exists.

[Manual index](README.md) · Next: [Implementation status](implementation-status.md) →
