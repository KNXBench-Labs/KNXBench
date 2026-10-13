Last updated: 2026-10-13 01:10 CEST

# KNXBench Features

| Reference | Value |
| --- | --- |
| Source snapshot | `v0.1.0-alpha.10` |
| Released baseline | `v0.1.0-alpha.10` |
| Main-only | Newer than released baseline |
| Local-only | Unpublished; not accepted on main |
| Scope | Listed boundaries; not universal compatibility |
| Later scope | No delivery dates promised |

## Runtime options

| Runtime | Platform | Availability |
| --- | --- | --- |
| Browser workbench | HTTPS server | alpha.6 |
| Browser uploads | 1024 MiB streamed; product packages 256 MiB | alpha.9 |
| Companion return | Current project; explicit navigation | alpha.7 |
| Docker server | Linux amd64/arm64 | alpha.6 |
| Linux desktop | x86-64 AppImage; bounded verification | alpha.6 |
| Desktop display | Wayland/X11; host-dependent | alpha.6 |
| Command line | Linux engineering tools | alpha.6 |
| HTTP API | Server integration | alpha.6 |
| MCP server | Read-only saved-file tools; opt-in rename writer from alpha.10 | alpha.6 |
| Remote browser | Authenticated server; networking-dependent | alpha.6 |

## Implemented — Projects

| Feature | Scope | Availability |
| --- | --- | --- |
| Project wizard | Structured project creation | alpha.6 |
| Native storage | Open/save `.knxdb` | alpha.6 |
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
| Object table | Channels/search/filter/sort | alpha.7 |
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
| Interface languages | English/German/Bavarian/Klingon | alpha.6 |
| Language packs | Importable JSON translations | alpha.6 |
| Notifications | Status/errors/achievements | alpha.6 |
| Release notifications | Notice only; nothing downloaded; can be switched off | alpha.10 |
| Achievements | Recorded engineering milestones | alpha.6 |
| Optional humour | Error/night/holiday messages | alpha.6 |
| Project notes | Notebook view; review-first edits | alpha.10 |
| Installation instructions | Device tab; imported hints, review-first | alpha.10 |
| Checked parameter editor | Time count, integer slider; source-checked | alpha.10 |
| Catalogue update with preview | Backup before install; restore listed backups | alpha.10 |
| Project information | Modeled facts; read-only | alpha.10 |
| Recorded device state | Load belief; read-only, no bus | alpha.10 |

## Implemented — Server and monitoring

| Feature | Scope | Availability |
| --- | --- | --- |
| Password login | Single shared password | alpha.6 |
| HTTPS | Generated/supplied certificates | alpha.6 |
| Persistent server data | Projects/catalog/settings | alpha.6 |
| Gateway discovery | KNXnet/IP interfaces | alpha.6 |
| Bus monitor | Tunnelling; telegram decoding | alpha.6 |
| Telegram Flow | Read-only; session-local graph | alpha.6 |
| Separate Flow window | Source-bound; shared monitor | alpha.6 |
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
| Read-only device verification | Limited hardware evidence | alpha.6 |
| Byte-order probe | CLI, read only: which octet order an ETS-programmed device holds; little-endian downloads stay refused | Main-only |
| Tester download reports | After a download: report file (program, outcome, values; no names or address), issue form or issue@knxbench.com; CLI and server, tab pending | Main-only |
| Byte-order download warning | Named plan warning when a big-endian program's own data looks little-endian; CLI and server, not yet in the web tab | Main-only |
| MCP integration | Saved files; eight read tools | alpha.6 |
| MCP name-recipe writer | Opt-in; group-address prefix/suffix after preview and confirmation | alpha.10 |
| Accessibility | Full screen-reader validation missing | alpha.6 |

## In progress — Unpublished

| Feature | Scope | Availability |
| --- | --- | --- |
| KNX Functions | Native editing; acceptance pending | Local-only |

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
