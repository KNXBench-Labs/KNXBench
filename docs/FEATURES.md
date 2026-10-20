Last updated: 2026-10-19 (Alpha 15 baseline; WP-17 on main)

# KNXBench Features

| Reference | Value |
| --- | --- |
| Source snapshot | `d18ce65e` (WP-17 after Alpha 15) |
| Released baseline | `v0.1.0-alpha.15` |
| Main-only | Newer than released baseline |
| Local-only | Unpublished; not accepted on main |
| Scope | Listed boundaries; not universal compatibility |
| Later scope | No delivery dates promised |

## New in Alpha 15

The [release scope](releases/alpha15.md) covers changes after Alpha 14 up to
cutoff `21b9d46f`.
Native/model16 is unchanged; product24 migrates to26 on writable open. Older
builds refuse product26; opening new project saves in Alpha 14 is unverified.

| Feature | Scope | Availability |
| --- | --- | --- |
| [File menu](contracts/file-menu.md) | Keyboard submenus, focus restoration, viewport/zoom-aware placement and touch/pen activation; same actions and guards | alpha.15 |
| [Control feedback](contracts/controls-and-separators.md) | Primary/secondary/quiet/destructive variants, visible separators, quieter routine provenance | alpha.15 |
| [Topology metrics](contracts/topology-metrics.md) | Explicit communication-object count labels and bounded narrow/wide layouts | alpha.15 |
| [Monitor DPT reasons](contracts/dpt-selection.md#monitor-resolution-reasons-ui-01-18-october-2026) | No project, unknown address, missing type, cleared program default and codec outcomes stay distinct; no guessed DPT | alpha.15 |
| [Product v25/v26](adr/0146-unknown-evidence-rows-have-an-identity.md) | Placement/enumeration columns; idempotent unknown evidence with repair; `SuffixText`; earlier v23–v25 promotion residues remain open | alpha.15 |
| [Larger measured bounds](adr/0142-file-size-limits-follow-the-storage-ceiling.md) | 1,000,000,000 bytes per retained file/member; 4 GiB standard expansion, 16 GiB CLI large profile; distinct 1024 MiB project upload | alpha.15 |
| [System-2 source preview](contracts/system2-source-probe-preview.md) | Explicit selected fields, offline candidate locations/byte order; cannot override live refusal | alpha.15 |
| [Declared-memory diagnostics](adr/0143-declared-segment-memory-evidence-profile.md) | CLI-only ProductDB-derived profile, preview and separately confirmed identity-gated reads; offline/simulator evidence, no hardware/profile write claim; web remains generic identity-only | alpha.15 |
| Catalogue availability errors | Preserve future-schema/open failure reasons; read-only CLI catalogue/documentation readers do not create or migrate the database | alpha.15 |
| Import hardening | Evidenced uppercase legacy table names; project ZIP empty-directory CRC/stream checks before persistence | alpha.15 |

## Runtime options

| Runtime | Platform | Availability |
| --- | --- | --- |
| Browser workbench | HTTPS server | alpha.6 |
| Browser uploads | 1024 MiB streamed since alpha.9; separate product-package ceiling 1,000,000,000 bytes since alpha.15 (ADR-0142; 256 MiB through Alpha 14) | alpha.9 |
| Companion return | Current project; explicit navigation | alpha.7 |
| Docker server | Linux amd64/arm64 | alpha.6 |
| Linux desktop | x86-64 AppImage; bounded verification | alpha.6 |
| Desktop display | Wayland/X11; host-dependent | alpha.6 |
| Command line | Linux engineering tools | alpha.6 |
| HTTP API | Server integration | alpha.6 |
| MCP server | Read-only saved-file tools; opt-in rename writer from alpha.10 | alpha.6 |
| Remote browser | Authenticated server; networking-dependent | alpha.6 |

## Project status feedback

| Feature | Scope | Availability |
| --- | --- | --- |
| [Project status storage metrics](contracts/project-status-metrics.md) | Current on-disk size of the open project's native file (may include unsaved recovery-journal growth), separate unsaved flag, active product database footprint including SQLite side files, and catalogue counts for manufacturers with products or programs; explicit missing/loading/error states | alpha.15 |

<a id="local-source-changes"></a>

## After Alpha 15 — source changes

| Feature | Scope | Availability |
| --- | --- | --- |
| [Master identity provenance](adr/0147-master-identity-provenance.md) | Product DB v27: retained-source positions for all four master families and both DPT forms, same/cross-file collisions and malformed declarations; historical winner/encounter facts remain unavailable, normalized values unchanged; readable EN/DE catalogue diagnostics | Main-only; `d18ce65e` |

## Data-table columns

| Feature | Scope | Availability |
| --- | --- | --- |
| Resizable data-table columns | Pointer/keyboard resizing below defaults, density/theme-aware cached header/cell word floors capped at 320 px, stable mounted maxima and local scrolling, measured-width announcements, per-column/table reset, validated browser-local preferences; narrow object/address/monitor and EN/DE 1440/1920 fit budgets verified (local scroll where floors exceed the pane); 18 HTML table surfaces, matrix select/address/name included; per-object columns stay fixed — [coverage and limits](contracts/table-column-widths.md) | alpha.15 |

## Topology range visibility

| Feature | Scope | Availability |
| --- | --- | --- |
| [Device-empty ranges](contracts/topology-metrics.md#device-empty-ranges-issue-25-source-builds-after-alpha-14) | Diagram hides areas/lines without devices; original structure and explorer editing remain unchanged | alpha.15 |

## Device list system family

| Feature | Scope | Availability |
| --- | --- | --- |
| [System column](research/2026-10-19-mask-version-system-family.md) | Devices list names System 1/2/7/B and other profiles from the program's declared mask through one cited core table; unknown masks and missing product/program/mask states shown explicitly, never guessed; sortable and filterable by label or mask; device-read masks not used ([guide](manual/user-guide/05-devices-and-products.md#system-column-source-builds-after-alpha-14)) | alpha.15 |

## Parameter caption resolution

| Feature | Scope | Availability |
| --- | --- | --- |
| [Parameter section captions](contracts/positional-text-templates.md#parameter-workspace-captions-issue-26) | Exact text-parameter resolution for block/separator captions; readable fallback labels and source tooltips; no configuration changes | alpha.15 |

## Parameter unit suffix

| Feature | Scope | Availability |
| --- | --- | --- |
| [Unit suffix](contracts/parameter-workspace.md#unit-suffix-wp-02-19-october-2026) | The product data's `SuffixText` after a parameter value (the reference's own over the parameter's, translated); presentation only, never part of the value | alpha.15 |

## Nested module scopes

| Feature | Scope | Availability |
| --- | --- | --- |
| [Nested module scope identity](adr/0063-parameter-scopes-preserve-evaluation-identity.md#web-adoption-wp-15-19-october-2026) | Diagnostics, workspace areas, drafts and Restricted sections of nameless nested modules stay apart by their complete evaluation path; older servers fall back to the innermost fields | alpha.15 |

## Device comparison display

| Feature | Scope | Availability |
| --- | --- | --- |
| [What differs (WP-18)](manual/user-guide/05-devices-and-products.md#what-differs-wp-18-source-builds-after-alpha-14) | Differing and unread raw bytes marked on both sides without relying on colour; differences-only default with a *Show equal rows* toggle revealing every read range; display only ([contract](contracts/device-comparison.md#result-display-wp-18-owner-amendment-19-october-2026)) | alpha.15 |

## New in Alpha 14

[Release scope](releases/alpha14.md) separates offline editing and product-data
work from unfinished programming support. Native/model16 is unchanged; the
product database moves from 23 to 24. Previous-reader compatibility is not verified.

| Feature | Scope | Availability |
| --- | --- | --- |
| [Read-only diagnostic export](contracts/device-diagnostic-export.md) | One device's descriptor, manufacturer ID and hardware type over the existing tunnel lock, saved as a verified local ZIP; no writes, no programming unlock | alpha.14 |
| [Functions in comparisons](contracts/project-functions.md) | Per-Function diff in app, CLI and MCP; read-only MCP `list_functions` | alpha.14 |
| Closed disclosure defaults | Navigation blocks, tree branches, parameter groups and details; manual opening and remembered address views retained | alpha.14 |
| [Parameter workspace](contracts/parameter-workspace.md) | Declared areas (closed until chosen), compact label/value rows, cross-area search, retained editors and source metadata; no inferred device/grid geometry | alpha.14 |
| [Declared parameter grids](adr/0137-declared-parameter-grids-reach-the-workspace.md) | Matrix view for areas whose product data declares a `Table`/`Grid`, same editors and write path; row view with a reason otherwise | alpha.14 |
| [Row indentation](contracts/parameter-workspace.md#row-indentation-census-p4-18-october-2026) | Rows indented by a consistently declared `IndentLevel`; presentation only | alpha.14 |
| [Packed time units fall back](contracts/parameter-editor-admission.md#packed-time-units-fall-back-2026-10-18) | Only plain time units get the count control; packed and undeclared-in-collection units show the original value | alpha.14 |
| [Object-local function dropdowns](contracts/parameter-object-refresh.md) | Manufacturer enum choices below the expanded object's DPT; exact configured-ID mapping, no guessed module mapping; replaces the device-wide disclosure | alpha.14 |

## New in Alpha 13

[Release scope](releases/alpha13.md) separates offline editing and image-integrity
work from unfinished programming support. Native/model16 and product23 are
unchanged from Alpha 12; previous-reader compatibility is not verified.

| Feature | Scope | Availability |
| --- | --- | --- |
| Foldable navigation | Independent sections and explorer-owned scrolling | alpha.13 |
| Grouped File actions | Project/files, imports, exports and support groups | alpha.13 |
| Manufacturer operating modes | Source-declared enum controls beside objects; shared catalogue path | alpha.13 |
| Parameter groups | Declared block ancestry and optional verified captions | alpha.13 |
| Parameter-driven object refresh | Atomic durable undoable creation of missing active unscoped objects; ambiguous/module synthesis refuses | alpha.13 |
| Offline image integrity | Relative-source and unhandled fixup/strict legacy-patch refusal; no new download admission | alpha.13 |
| Offline family codecs/simulation | Explicit table/property bytes and normalized simulator only, not complete product loading | alpha.13 |
| PR contributor recognition | Public counters, explicit XP, earned medals and maintained comments | alpha.13 |

## New in Alpha 12

These additions are included in Alpha 12. [Release scope](releases/alpha12.md)
keeps their limits together; [source acceptance](releases/alpha12-acceptance.md)
records the tested commit separately from artifact publication verification.
| Feature | Scope | Availability |
| --- | --- | --- |
| Project versus real devices (P4) | Confirmed single/filtered batch, progress/cancel, session tags, verified application identity and decoded 0701 address/association tables; named parameter project/device values with exact bit attribution, explicit unknowns and visible raw bytes ([guide](manual/user-guide/05-devices-and-products.md#comparing-the-project-with-devices-p4-since-alpha-12)) | alpha.12 |
| Universal device-support reports | Offline without project/catalogue/plan; optional separately consented fixed read-only diagnostics, local review/ZIP/manual handoff; no programming unlock | alpha.12 |
| Per-part pending programming | Adr/App/Par/GA chips and Pending only filter, explorer dot, same-step edit/undo tracking, full/partial/address settlement and undoable logged manual marks; project belief, not live comparison; recovered snapshot undo is conservative ([guide](manual/user-guide/05-devices-and-products.md#programming-marks-p3-since-alpha-12)) | alpha.12 |
| Active task text shimmer | Readable slow sweep plus pause on existing running-work status text; motion/accessibility gates; no fabricated progress ([contract](contracts/task-status-shimmer.md)) | alpha.12 |
| Resume after reload | A reloaded or reopened web tab shows the project the server still holds, with a notice naming unsaved changes; read-only, no reopen ([ADR-0127](adr/0127-web-workbench-resumes-the-open-project.md)) | alpha.12 |
| Overwrite question | Save As onto an existing file asks first; the server refuses an unconfirmed replacement (`409 fileExists`); the previous state stays in the file's history ([Projects](manual/user-guide/02-projects.md#saving)) | alpha.12 |
| Explorer structure | Chevrons that turn, counts on closed branches, Expand/Collapse all, structure lines that grow and pulse; own motion category "Structure lines" ([user interface](manual/user-guide/01-user-interface.md)) | alpha.12 |
| Mirrored toast entry | Every toast enters with its exit animation played backwards; motion settings apply as before | alpha.12 |

## Implemented — Projects

| Feature | Scope | Availability |
| --- | --- | --- |
| Project wizard | Structured project creation | alpha.6 |
| Native storage | Open/save `.knxdb`; schema 16 in alpha.12, older readers refuse upgraded files | alpha.6 |
| Portable product sources | Native saves embed available manifest-listed manufacturer bytes; missing blobs or absent manifest entries can still require matching product data on reopen; Docker catalogue persists under mounted data ([boundary](limitations/KL-183-knxdb-manufacturer-files.md)) | alpha.12 |
| Save As | Independent project copies | alpha.6 |
| Browser export | Download `.knxdb` | alpha.6 |
| Autosave | Existing project files | alpha.6 |
| Undo/redo | Session-local reversible edits | alpha.6 |
| Persistent undo/redo | Native files; survives restart | alpha.7 |
| Project versions | Named/automatic; same-file storage | alpha.7 |
| Version restore | Confirmed; safety version | alpha.7 |
| Schema migration | Older native files | alpha.6 |
| Device wizard | Preview-confirmed creation | alpha.6 |

## Implemented — Engineering

| Feature | Scope | Availability |
| --- | --- | --- |
| Buildings | Buildings/floors/rooms; structural editing | alpha.6 |
| Topology | Areas/lines; address validation | alpha.6 |
| Device list | Search/sort; central editor | alpha.6 |
| Device names | Properties/F2; name-only edits | alpha.7 |
| Group addresses | Create/delete; validation | alpha.6 |
| Group-address names | Properties/F2; name-only edits | alpha.7 |
| Group-address presentations | Tree table, column browser, address atlas, connection matrix (linked objects plus explicitly loaded device object lists); shared selection | alpha.10 |
| Address styles | Free/two-level/three-level | alpha.6 |
| Group ranges | Structural editing | alpha.6 |
| Communication objects | Values/provenance inspection | alpha.6 |
| Object editing | DPT/description; six flags | alpha.6 |
| Object links | Directional group-address connections | alpha.6 |
| Group-address members | Add from the address (device → object → direction) in all four views and Properties; same link rule | alpha.10 |
| Group-address picker | Link row: type to filter by address, name or DPT; marks a type difference, never hides (#8) | alpha.11 |
| Tree indentation | Address rows by level (capped at 12); channel groups in the object table (#9) | alpha.11 |
| Object table | Channels/search/filter/sort | alpha.7 |
| Literal batch naming | Group-address prefix/suffix; preview, confirmation, one undo step | alpha.12 |
| Module-instance values | Stored per instance; read-only | alpha.8 |
| Parameters | Validated edits; dynamic evaluation | alpha.6 |
| Parameter tabs | Editor/diagnostics/manufacturer fields | alpha.6 |
| DPT resolution | Declared/linked types; conflicts | alpha.6 |
| DPT selection | Named EN/DE choices; exact stored values | alpha.9 |

## Implemented — Files and reports

| Feature | Scope | Availability |
| --- | --- | --- |
| ETS project import | Schemas 11/21; bounded compatibility | alpha.6 |
| Import reports | Warnings/errors/unsupported data | alpha.6 |
| Selective import | Merge chosen lines/devices; preview, one undo step | alpha.8 |
| [Plug-in data adoption](contracts/plugin-data-adoption.md) | One device's stored plug-in bytes from a same-lineage archive; preview by digest, way back = a project version, no Undo; acceptance unverified | alpha.14 |
| Opaque preservation | Retained source; disclosed limitations | alpha.6 |
| Product installation | `.knxprod`; evidenced schemes | alpha.6 |
| Embedded products | From imported projects | alpha.6 |
| Product catalog | Manufacturers/products/programs; inspection | alpha.6 |
| Group-address CSV | KNXBench-format import/export | alpha.6 |
| CSV re-addressing | Confirmed preview; guarded deletion | alpha.6 |
| Project documentation | HTML export; section selection | alpha.6 |
| Report preview | Sandboxed; browser printing | alpha.6 |
| Project comparison | Two-project; before/after values | alpha.6 |
| ETS comparison | Import-based `.knxproj` comparison | alpha.6 |
| Support-gap analysis | Read-only; disclosure preview | alpha.6 |
| Programming refusal help | EN/DE reason/action, local details, docs/unsent issue/mail before any plan; no new device support | alpha.12 |
| Evidence ZIP | Consented export; manual submission | alpha.6 |
| Offline AP1 diagnostics | Unresolved steps; no execution | alpha.7 |
| Original import report | Kept in native files; local download | alpha.10 |
| Source-report catalogue | Selective-import reports; read-only | alpha.10 |
| Retained payloads | Read-only list; inert downloads | alpha.10 |
| Native export review | Privacy notice; reviewed snapshot | alpha.10 |
| Offline demos | Home/residential/office practice projects | alpha.6 |

## Implemented — Workbench

| Feature | Scope | Availability |
| --- | --- | --- |
| Project Explorer | Scoped project navigation | alpha.6 |
| Properties Inspector | Selected-object inspection/editing | alpha.6 |
| Search | Project entities | alpha.6 |
| Command palette | Searchable actions | alpha.6 |
| Context menus | Object actions | alpha.6 |
| Keyboard shortcuts | Documented navigation/editing | alpha.6 |
| Pane resizing | Saved widths; responsive layout | alpha.6 |
| Interface zoom | 80–150%; shared preference | alpha.6 |
| Themes | Light/dark/system; LCARS/CRT | alpha.6 |
| Theme packs | Import/export; declarative only | alpha.6 |
| Appearance controls | Accent/density/motion | alpha.6 |
| Reduced motion | OS preference takes precedence | alpha.6 |
| Motion per category | Toasts/achievements/Flow; can only reduce | alpha.11 |
| Interface languages | English/German/Bavarian/Klingon | alpha.6 |
| Language packs | Importable JSON translations | alpha.6 |
| Notifications | Status/errors/achievements | alpha.6 |
| Notification queue | Three on screen; pause on hover/focus; errors stay | alpha.11 |
| Release notifications | Notice only; nothing downloaded; can be switched off | alpha.10 |
| Achievements | Recorded engineering milestones | alpha.6 |
| Optional humour | Error/night/holiday messages | alpha.6 |
| Project notes | Notebook view; review-first edits | alpha.10 |
| Notes on objects | Communication objects as targets; tab-close prompt for drafts | alpha.11 |
| [Notes when deleting](adr/0138-notes-get-an-explicit-decision-before-a-delete.md) | Keep, move or delete each affected note; one undo step with the delete | alpha.14 |
| Installation instructions | Device tab; imported hints, review-first | alpha.10 |
| Checked parameter editor | Time count, integer slider; source-checked | alpha.10 |
| Catalogue update with preview | Every package uses review; backup before install, restore/delete listed backups (alpha.12) | alpha.10 |
| [Vendor ZIP install](adr/0140-vendor-zips-of-product-packages-install-as-a-whole.md) | A manufacturer ZIP of `.knxprod` packages installs all inner packages together (CLI and web preview); other files listed, not stored; large inner packages via the CLI flag | alpha.14 |
| [Catalogue cleanup](adr/0145-catalogue-cleanup-removes-placements-only.md) | Catalogue rows show application, version, section and source; selected entries removed after a preview and a full backup; products, programs and package files kept | alpha.15 |
| Project information | Modeled facts; read-only | alpha.10 |
| Stored bus access | Redacted by default; reveal one saved endpoint, no connection | alpha.12 |
| Recorded device state | Load belief; read-only, no bus | alpha.10 |

## Implemented — Server and monitoring

| Feature | Scope | Availability |
| --- | --- | --- |
| Password login | Single shared password | alpha.6 |
| HTTPS | Generated/supplied certificates | alpha.6 |
| Persistent server data | Projects/catalog/settings | alpha.6 |
| Gateway discovery | KNXnet/IP interfaces | alpha.6 |
| Bus monitor | Tunnelling; telegram decoding; since alpha.15 distinct no-project, unknown-address, missing-type, cleared-default and codec outcomes ([contract](contracts/dpt-selection.md#monitor-resolution-reasons-ui-01-18-october-2026)) | alpha.6 |
| Telegram Flow | Read-only; session-local graph | alpha.6 |
| Separate Flow window | Source-bound; shared monitor | alpha.6 |
| Flow images | PNG or 1–30 s GIF of the viewport, with consent | alpha.11 |
| Activity history | Bounded commissioning metadata | alpha.6 |

## Partial — Compatibility and hardware

| Feature | Boundary | Availability |
| --- | --- | --- |
| ETS schema 23 | Independent module evidence missing | alpha.6 |
| ETS schema 20 | ETS 5.7; three public samples; nested sub-module objects undecoded | alpha.10 |
| ETS schemas 12–14 | Documented only; no real project imported | alpha.6 |
| Protected project import | ZipCrypto (schema < 21); public ETS4/5.7 exports | alpha.10 |
| Legacy VD3/VD4 | Offline import; DPTs absent | alpha.6 |
| Legacy VD5 | Some parameter types unsupported | alpha.7 |
| Legacy download plans | No real-device execution evidence | alpha.7 |
| Product schemes | Admission ≠ manufacturer semantics | alpha.6 |
| Product versions | Inspection; no version selector | alpha.6 |
| Module parameters | Repeated instantiation refused | alpha.6 |
| DPT codecs | Families 1–30; subtype boundaries | alpha.6 |
| Product translations | Missing texts use marked fallbacks | alpha.6 |
| Device drag/drop | Same-installation placement only | alpha.6 |
| Dashboard | Counts/diagnostics; warnings open the log from alpha.10 | alpha.6 |
| Report metadata | Matching installed products required | alpha.6 |
| Multicast routing | CLI-only; limited live evidence | alpha.6 |
| Group-value writes | Limited live/gateway evidence | alpha.6 |
| Line scan | Diagnostics; bounded identification | alpha.6 |
| Device downloads | Restricted programs/devices; confirmed operations | alpha.6 |
| Download preparation | Gateway choice from the shared search; device-editor hand-off; step guide | alpha.10 |
| Gateway busy/refused | Names which KNXBench session holds the tunnel, and the gateway's own refusal code (#10) | alpha.11 |
| Read-only device verification | Limited hardware evidence | alpha.6 |
| Byte-order probe | CLI, read only: which octet order an ETS-programmed device holds; little-endian downloads stay refused | alpha.11 |
| Tester download reports | After a download: steps, checklist and report file (program, outcome, values; no names or address), issue form or issue@knxbench.com; download tab, CLI and server | alpha.11 |
| Byte-order download warning | Named plan warning when a big-endian program's own data looks little-endian; download tab, CLI and server | alpha.11 |
| MCP integration | Saved files; eight read tools | alpha.6 |
| MCP name-recipe writer | Opt-in; group-address prefix/suffix after preview and confirmation | alpha.10 |
| Accessibility | Full screen-reader validation missing | alpha.6 |

## Implemented — Project organisation

| Feature | Scope | Availability |
| --- | --- | --- |
| KNX Functions | Room-owned named groups of existing group addresses; native edit/undo, bounded schema-23 mapping | alpha.12 |

## Planned — Later scope

| Feature | Scope | Availability |
| --- | --- | --- |
| Site grouping | Several installations | Unscheduled |
| Task automation | Macros/templates/scheduling | Unscheduled |
| Dashboard navigation | Count-to-object drill-down | Unscheduled |
| Parameter widgets | Verified colour/picture codecs, module scopes | Unscheduled |
| Version selection | Package/application pinning | Deferred |
| Manufacturer updates | Online catalogs | Unscheduled |
| Agent mutation | General write-capable AI integration | Unscheduled |
| Diff application | Merging project changes | Unscheduled |
| Three-way comparison | Conflict-aware comparison | Unscheduled |
| Native PDF | Direct document generation | Unscheduled |
| Mobile application | Dedicated mobile client | Unscheduled |
| Native Windows/macOS | Desktop ports | Unscheduled |

## Unavailable or refused

| Capability | Boundary |
| --- | --- |
| ETS project export | No `.knxproj` writer |
| AES project import | Refused |
| ETS schema 22 | Refused; samples had no devices |
| KNX Secure | Data/IP Secure unsupported |
| USB interfaces | Unsupported |
| Individual-address writes | Recovery prerequisites unmet; refused |
| Multi-user editing | No concurrency/conflict support |
| User accounts | No roles/per-user audit |
| Vendor plug-ins | No proprietary plug-in host |
| Encrypted product packages | Permanently excluded |
| Manufacturer signatures | Not cryptographically verified |
| Independent disaster backups | Same-file versions cannot substitute |
| Automatic updates | Not implemented |

## Details

| Reference | Purpose |
| --- | --- |
| [Manual](manual/implementation-status.md) | Detailed feature inventory |
| [Compatibility](COMPATIBILITY.md) | Verified format boundaries |
| [Known issues](manual/known-issues.md) | Practical limitations |
| [Installation](manual/getting-started/04-installation.md) | Runtime requirements |
| [Web/Docker](manual/user-guide/11-web-and-docker.md) | Server deployment |
| [Open work](OPEN_WORK.md) | Later-scope ownership |
