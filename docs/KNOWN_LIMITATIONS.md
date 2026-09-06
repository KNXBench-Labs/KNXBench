# Known limitations

Each entry states the limitation, its cause, what it costs the user, and the
condition under which it would be lifted. Nothing here is a defect to be fixed
by trying harder — these are consequences of evidence we do not have or of
decisions recorded in [docs/adr/](adr/).

The companion document is [COMPATIBILITY.md](COMPATIBILITY.md), which states
what is verified. Nothing may appear as verified there and as a limitation
here.

## 1. Single-sample bias

**Limitation.** Everything verified about the `.knxproj` format comes from two
installations: the "Unser Zuhause" project (schema 11, ETS 4.1.8, and schema
23, ETS 6.3.7959.0 — the same project exported twice, risk R1) and, since
Session 7 (2026-09-06), the KNX Association "KV v2.5" demo project (schema
21, ETS 5.7 — a genuinely different installation).

**Cause.** Independent ETS5/ETS6 sample projects remain scarce. The second
export of the ETS4 reference project (RESEARCH §2.4/§3.3) confirms the
schema-11→23 format diff for one installation; the KV demo project (RESEARCH
§2.5/§3.4) independently confirms most of that same diff already exists at
schema 21, on unrelated data. Together they say nothing about schema 12, 13,
14, 20 or 22, and nothing about a differently-structured schema-23 project
(e.g. one using `Functions`, KNX Secure, or multiple areas/lines for real).

**Impact.** Support for schema 12, 13, 14, 20 and 22 is still derived from
documentation and from reading `xknxproject`, not from evidence. Schema 21
and 23 support is derived from evidence but from only two project shapes; a
first import of a structurally different schema-21+ project will still
likely produce unknown-construct entries or new deltas.

**Session 3 status.** `knx-etsproj`'s known-element table covers schema 11
only, built from the reference project. Schema 23 is **detected and refused
by name** (`ImportFailure::NoKnownSchemaTable { version: 23 }`) — never
silently misread through the schema-11 table, which would undercount
communication objects by about 24% and misread every boolean flag as false
(RESEARCH §3.3). Building the schema-23 known-element table is Session 4
work, not attempted here.

**Session 7 status (2026-09-06).** Schema 21 is likewise **detected and
refused by name**, now proven against a real, independent schema-21 file
(`knx-etsproj`'s `importing_a_second_independent_schema21_project_fails_...`
test), not just a synthetic namespace string — the refusal mechanism is
schema-version-generic (`known_schema` returning `None`), so no code change
was needed, only the test. Full schema-21/23 import support is **not**
implemented; see the "next big task" note below.

**Next big task.** Building real schema 21 (and by extension 23) import
support is the next major format-support undertaking, flagged explicitly
(2026-09-06) rather than left as a vague "whenever a sample turns up" — a
sample now exists (RESEARCH §3.4). It needs its own brainstorming/design
pass before implementation, not a drive-by table entry: `ModuleInstances`
(modular application programs) has no domain-model representation yet
(absent from [DATA_MODEL.md](DATA_MODEL.md), needs its own ADR), and
`GroupObjectTree` vs. `ComObjectInstanceRefs` as the authoritative
communication-object source needs a resolution rule. This is ETS Import
(Session 3) / KNX Core (Session 2) territory reopened, not Session 7
hardening — see [ROADMAP.md](ROADMAP.md).

**Lifted when.** Real ETS5 projects and further, independent ETS6 projects
have been imported and their unknown-construct reports reconciled to empty.
This is a prerequisite for claiming more than schema 11 and these two
schema-21/23 shapes.

## 2. No authoritative XSD is publicly available

**Limitation.** Imports are tolerant, not schema-validating (risk R2).

**Cause.** The official schemas ship with the Manufacturer Tool via the KNX
GitLab account, which requires KNX membership [D].

**Impact.** We cannot tell "this file is invalid" from "this file uses
something we do not know". A malformed file may be read as far as it parses,
with the rest reported rather than rejected.

**Lifted when.** Authoritative schemas become available to the project. Note
that the tolerant parser would still be worth keeping — it is what turns a new
schema version into a report instead of a crash.

## 3. Device parameters are preserved but not interpreted

**Limitation.** All 1390 `ParameterInstanceRef` values in the reference project
are imported, stored and exported unchanged, but their meaning is not
evaluated. There is no parameter editor in v1 (risk R3).

**Cause.** Parameter visibility and semantics are driven by the `Dynamic` tree,
whose `choose`/`when` expression grammar is unresearched.

**Impact.** Device configuration must still be done in ETS. This application
will not corrupt parameter data, but it will not let you change it either.

**Lifted when.** The Session 4 research spike establishes the `when/@test`
grammar, and a parameter editor is built on top of it.

## 4. Round trips are semantic, not byte-exact

**Limitation.** An exported file is not byte-identical to the imported one
(risk R4).

**Cause.** Signatures cannot be regenerated, attribute ordering is not
guaranteed stable, and ETS assigns internal identifiers.

**Impact.** Comparing an export against the original with `cmp` will show
differences. That is expected and is not evidence of data loss.

**Lifted when.** Never — this one is structural. What replaces it are the three
guarantees in [IMPORT_EXPORT.md](IMPORT_EXPORT.md): semantic model equality,
hash equality of all opaque bytes, and an explicit unsigned-export statement.
See [ADR-0007](adr/0007-roundtrip-fidelity.md).

## 5. Exports are unsigned, and ETS acceptance is untested

**Limitation.** Every file this application writes is unsigned, and whether ETS
re-imports it is unknown (risk R9).

**Cause.** Signatures are RSA over manufacturer and project data; the signing
keys are KNX's.

**Impact.** An export may or may not open in ETS. The application says so at
export time rather than implying it will work.

**Session 3 status.** Every export carries `ExportWarning::Unsigned` — always
constructed before anything else can fail, so no export is produced without
it. Every `*.signature` entry (one per manufacturer plus one for the
project — five in the reference project) is copied through unchanged and
reported as `ExportWarning::StaleSignature { source_path }` per entry: it
no longer matches the content it signs, since it cannot be regenerated
without KNX's signing keys.

**Lifted when.** Someone exports a project and opens it in a real ETS
installation. Either outcome is useful: if ETS rejects it, the export is
documented as one-way, which is a different product decision than a bug.

## 6. Devices behind vendor plug-in DLLs

**Limitation.** Devices whose configuration depends on a vendor plug-in DLL
cannot be configured by this application (risk R5).

**Cause.** The configuration logic lives inside a Windows PE binary shipped in
the project file — for example `econEts3.dll` (641 KB) and `FastDownload.dll`
(160 KB) in the reference project [V].

**Impact.** Affected devices are detected, marked read-only, and reported in
the import report's `unsupported` list. Their data round-trips unchanged; it
simply cannot be edited here.

**Lifted when.** Never by us. Executing vendor binaries is not something this
application does, on any platform.

## 7. Commissioning and device download are out of scope

**Limitation.** The application does not program devices (RESEARCH §8.3).

**Cause.** Bricking risk on real hardware, an undocumented `Legacy*`
compatibility matrix, and vendor DLL involvement in download procedures.

**Impact.** Planning and documentation happen here; downloading happens in ETS.

**Lifted when.** A deliberate decision to take it on, with hardware to test
against. The architecture does not block it: load procedures, memory layout and
mask data are all present in the product database.

## 8. KNX Secure is not implemented

**Limitation.** No Data Secure, no IP Secure, no keyring handling (RESEARCH
§9).

**Cause.** No sample key material was available in Session 0, so nothing about
it could be verified.

**Impact.** Secured installations cannot be fully represented or monitored.

**Lifted when.** Sample material and a real secured installation are available.
The isolation boundary already exists — `knx-secure` is a separate crate with
no dependency on `knx-core` — precisely so that this can be built without
retrofitting secret handling into the model
([ADR-0008](adr/0008-key-material-isolation.md)).

## 9. Project files are not diffable

**Limitation.** A project is a SQLite file, so version control tools cannot
show a meaningful diff of it.

**Cause.** A deliberate trade for transactional, incremental saving and indexed
access ([ADR-0003](adr/0003-sqlite-project-format.md)).

**Impact.** Projects can be versioned as binaries only. Reviewing what changed
between two versions requires the application.

**Lifted when.** A textual export and import format is added, if a demonstrated
need arises. It is deliberately not built speculatively.

## 10. The project licence is not decided

**Limitation.** The Cargo workspace declares `AGPL-3.0-or-later` as a
placeholder. This is not a decision.

**Cause.** The licence has not been chosen yet.

**Impact.** No practical impact today, since nothing is distributed. It must be
settled before any release.

**Lifted when.** The licence is chosen and the workspace `license` field is
updated. Whatever it becomes, it must remain consistent with the constraint
that no GPL crate enters the runtime graph — that constraint is about *incoming*
dependencies and is independent of our own licence
([ADR-0002](adr/0002-own-knxproj-parser.md)).

## 11. `.knxprod` files for master data scheme ≥ 12 cannot be imported directly

**Limitation.** Manufacturer product files in the `.knxprod` container are only
readable for master data scheme 11.

**Cause.** `.knxprod` is the same XML family as `.knxproj`, but newer master
data schemes add an encryption or obfuscation layer that Session 0 research did
not establish (RESEARCH §10) [D, open].

**Impact.** Product data for newer devices has to reach the product database by
another route — in practice, from a `.knxproj` that already contains the
application programs it references.

**Lifted when.** The container layer for scheme ≥ 12 is understood, or an
official route to that data becomes available. Out of v1 scope either way.

## 12. Manufacturer data resolution — lifted for communication objects, three gaps remain

**Lifted (Session 4) for communication-object defaults.** `ProductRefId`
and `Hardware2ProgramRefId` now resolve: the shared product database
([ADR-0005](adr/0005-separate-product-database.md),
[ADR-0011](adr/0011-product-database-storage.md)) ingests `<M-xxxx>/*`
once, keyed by content hash, and `knx_productdb::enrich` fills a
communication object's `text`, `description`, `dpt`, five flags and `size`
from the application program wherever the instance itself left the slot
`Absent` (IMPORT_EXPORT §10). `ComObjectInstance` values now carry
`Program`/`ProgramRef` in addition to `Instance` where the source project
did not itself state a value.

What remains, each with its own cause:

**Parameter interpretation is still absent.** The `Dynamic` tree
(`choose`/`when`, visibility logic) is not parsed at all; the `when/@test`
expression grammar that would make it interpretable is unresearched
(RESEARCH R3). *Lifted when* that grammar is documented and a parameter
editor is judged feasible — its own research spike, not a byproduct of
this session.

**A program value behind an `Empty` instance slot stays invisible in the
model.** 497 of the reference project's 907 `ComObjectInstanceRef`
elements carry `DatapointType=""` — present, explicitly cleared, not
unstated. Enrichment deliberately never overwrites `Empty` (ADR-0012): the
program's own value stays queryable in the product database
(`knx_productdb::query::com_object_view`) but is not baked into
`ComObjectInstance`. *Lifted when* `Override<T>` grows a layer stack that
can hold a program value and an instance-level `Empty` on the same
attribute without conflating them — a domain-model change with a
migration, deliberately deferred rather than rushed into this session.

**An ambiguous, space-separated `DatapointType` list fills nothing.**
`ComObjectRef/@DatapointType` can hold several acceptable alternatives
(RESEARCH §4.2, e.g. `"DPST-9-21 DPST-9-1"`). Enrichment refuses to guess
between them; it records `EnrichmentIssue::AmbiguousDpt` and leaves the
slot as it was. *Lifted when* the alternative to select can be determined
from context (e.g. from a linked group address's own datapoint type) — not
attempted this session.

**Cause.** All three are, respectively: unresearched grammar (RESEARCH R3);
a domain-model change intentionally scoped out of this session
(ADR-0012); and a genuine ambiguity in the source data this session does
not attempt to resolve.

**Impact.** A project opens completely and round-trips its manufacturer
data byte-for-byte, with communication-object defaults now resolved where
the instance did not override them. Parameter values remain preserved but
uninterpreted; an `Empty`-slot program default and an ambiguous DPT list
are both visible in the product database and in `EnrichmentReport`, but
neither is written into the domain model.

**Lifted when.** See each gap above individually; none of the three shares
a single condition.

## 13. Password-protected projects are refused, not decrypted

**Limitation.** A `.knxproj` whose project part is nested as `<P-xxxx>.zip`
(IMPORT_EXPORT §2) is detected and named
(`ContainerError::PasswordProtected`), but the file is never opened.

**Cause.** Both decryption schemes (ZipCrypto for schema < 21, AES/PBKDF2
for schema ≥ 21) are documented from `xknxproject` source but unverified
against a real protected project — the reference project is unprotected.
Shipping an untested decryption path would claim support this repository
cannot demonstrate.

**Impact.** A protected project cannot be imported at all today, by design
rather than by omission: refusing cleanly is preferred over a decryption
path nobody has run against a real encrypted file.

**Lifted when.** A real password-protected ETS4/5 project (ZipCrypto) and a
real password-protected ETS6 project (AES) are available to verify each
scheme against.

## 14. The project's default language is a placeholder

**Limitation.** Every imported project is created with
`Language("en")` as its `StringTable`'s default language, regardless of the
language the project was actually authored in.

**Cause.** A schema-11 project file carries no project-wide language tag:
`DefaultLanguage` belongs to an application program's `RegistrationInfo`
(RESEARCH §4.1), not to `ProjectInformation`. Session 3 imports no
application program, so there is nothing in the imported data to derive a
real value from, and inventing one from, say, the project name would be a
guess presented as a fact.

**Impact.** Nothing observable in Session 3: instance-level `@Text` and
`@Description` are literal, not translated, so every `Text` this importer
produces is `Text::Literal` and resolves without consulting the table at
all. The default language only starts to matter once localized program
text exists. Export and semantic comparison both ask the project's own
`StringTable::default_language()` rather than naming a language
themselves, so when a real value arrives there is exactly one place that
sets it.

**Lifted when.** Session 4 ingests application programs and their
`RegistrationInfo`, giving the importer a measured language to set instead
of a placeholder.

## 15. Unparsable values survive only on `Override` fields

**Limitation.** A present attribute whose value cannot be parsed keeps its
raw text — and is written back verbatim on export — only where the field
is modelled as `Override<T>` (`Override::Malformed`, ADR-0010's
amendment). On a field modelled as a bare value or an `Option<T>`, an
unparsable value falls back to the type's default and only the report
records what the source actually said.

**Cause.** `Override<T>` exists to carry an attribute's presence state, so
a fourth state costs nothing structurally. A bare `u8` or an
`Option<DateTime<Utc>>` has nowhere to put a string, and widening every
such field would push presence bookkeeping into parts of the model that do
not otherwise need it.

**Impact.** For the affected fields (timestamps, individual addresses,
numeric ids, enums such as `CompletionStatus`) a malformed source value is
reported but not written back: the export is a correct file that differs
from the original in exactly that attribute. No such value occurs in the
reference project — the case is reachable only with hand-broken input.

**Lifted when.** A real project is found that carries unparsable values on
those fields, making the added model surface worth its cost. Until then
the asymmetry is deliberate, documented, and reported at import time.

## 16. Tauri v2's Linux backend depends on archived GTK3 bindings

**Limitation.** The desktop shell's Linux runtime depends on `tauri` 2.11,
which pulls in the archived gtk-rs GTK3 bindings; `cargo deny check` flags
16 upstream "unmaintained" notices in the advisory database, all of which
must be suppressed in `deny.toml` to build.

**Cause.** The gtk-rs project archived its GTK3 bindings repository in 2024.
The bindings are not vulnerabilities — every advisory explicitly states "no
safe upgrade is available" — but they are no longer maintained upstream.
Tauri's own GTK4 migration is in progress and not yet shipped.

**Impact.** Each `tauri` or `tauri-*` dependency bump requires manual
re-check of the 16 suppressed IDs: RUSTSEC-2024-0370, -0411 through -0420
(minus one gap), and RUSTSEC-2025-0075, -0080, -0081, -0098, -0100. As
Tauri moves to GTK4, some or all of these may disappear from the advisory
database. Until then, the `deny.toml` ignore list is permanent infrastructure.

**Lifted when.** Tauri v3 or a later `tauri` 2.x release ships its GTK4
backend and becomes the default on Linux.

## 17. Deleting a group address can leave a dangling `GroupLink` — resolved

**Resolved (Session 5, cycle 9).** `knx_core::command::Command::
DeleteGroupAddress` now scans `Devices::com_objects()` for any
`ComObjectInstance.links` entry naming the group address being deleted,
and refuses the whole command (`CommandError::GroupAddressInUse`) if one
exists, rather than removing the entry and leaving the link dangling.
This is the "surface them as ... finding first" resolution this entry
originally anticipated, in its strictest form: the delete simply does not
happen until the user removes the link first. A future cycle could soften
this into removing/flagging the links automatically instead of refusing
outright — that remains a design choice, not a defect.

**Originally.** `DeleteGroupAddress` removed the `GroupAddressEntry` from
`Installation::group_addresses` without scanning `Devices` for any
`ComObjectInstance.links` entry that pointed at it, so a communication
object could end up with a `GroupLink` naming a group address id that no
longer existed. In memory nothing visibly broke; a later full
`save_project` re-derived every `group_link` row from
`ComObjectInstance.links` and failed with a foreign-key violation against
`group_address(id)`.

## 18. `open_project` does not clear the previous `.knxdb` `store_path`

**Limitation.** `AppState.store_path` (the `.knxdb` file a subsequent plain
`save_project` writes to) is only ever set by `save_project_as` and
`open_native_project`. The Tauri `open_project` command — ETS `.knxproj`
import — loads a fresh in-memory project but never touches `store_path`.
If a `.knxdb` was open and the user then imports a `.knxproj`, `store_path`
still points at that old `.knxdb` file.

**Cause.** `open_project` and `open_native_project` were added in
different cycles (`.knxproj` import predates the native `.knxdb` format)
and were never made to share a single "what file, if any, backs the
in-memory project" invariant.

**Impact.** None reachable through the current UI: `apps/knx-web/src/
App.tsx` resets its own `hasStorePath` flag to `false` on ETS import, so
"Save" always falls back to "Save As…" in that state. But the backend has
no equivalent guard — `save_project` just writes wherever `store_path`
points, with no check that the loaded project actually originated from
that path — so a future UI change that calls `save_project` without first
re-deriving `hasStorePath` from a real backend query could silently
overwrite the old `.knxdb` with the newly-imported project's data.

**Lifted when.** Either `open_project` clears `store_path` to `None`, or
`save_project` verifies the in-memory project actually originated from
`store_path` before writing.

## 19. A search result inside a collapsed tree branch is not revealed

**Limitation.** Picking a result from `Ctrl+K` search (`apps/knx-web/
src/Search.tsx`) selects the matching device, group address, or building
part and shows it in the Inspector, but if the Project Explorer tree has
the ancestor branch containing it manually collapsed, the tree itself does
not expand or scroll to reveal the row — only the Inspector reflects the
new selection.

**Cause.** An explicit, approved scope decision recorded in
[the search design spec](superpowers/specs/2026-09-04-search-design.md),
not an oversight: tree auto-expand/scroll-into-view needs its own
expand/collapse/reveal logic, which the spec deliberately kept out of this
cycle's surface to keep search and tree-navigation state disjoint.

**Lifted when.** A future cycle adds tree auto-expand and scroll-into-view
for a selection that originates outside the tree itself (search today,
potentially a future command palette too).

## 20. Command palette and search share overlay CSS and an accessibility gap, unaddressed

**Limitation.** `apps/knx-web/src/Search.tsx` and `CommandPalette.tsx`
are two near-identical modal-overlay implementations — an overlay div, a
click-outside `stopPropagation` panel, an autofocused input, and
`Escape`/arrow-key/`Enter` handling — kept as separate components rather
than one shared shell. Neither overlay's result list has proper ARIA
semantics either: `CommandPalette.tsx`'s disabled rows carry
`aria-disabled="true"` but nothing backs it with `role="option"`/
`role="listbox"` on the containing list, and neither overlay announces the
highlighted row via `aria-activedescendant`.

**Cause.** (a) The two overlays' keyboard-traversal semantics differ —
`Search.tsx` navigates a grouped-by-kind list (both stop, rather than wrap,
at the ends); `CommandPalette.tsx` navigates a flat list that additionally
skips disabled rows — enough divergence that extracting a shared
`<ModalOverlay>` shell was judged premature after only two consumers;
`styles.css`'s `.search-overlay`/`.search-panel`/`.search-empty`/
`.search-results`/`.search-result` classes are shared today, but the
component logic is not. (b) Accessibility semantics for a custom
listbox-like widget were out of scope for both the search and command
palette design cycles, which focused on keyboard/mouse behavior, not
screen-reader support.

**Impact.** Two call sites to keep in sync by hand whenever overlay
structure changes (e.g. a future scroll-into-view fix would need applying
twice). A screen reader user gets no indication of which row is disabled
or currently highlighted in either overlay.

**Lifted when.** (a) A third overlay is added — the dark/light mode
picker is next on [ROADMAP.md](ROADMAP.md) and would be that third case —
at which point extracting a shared shell stops being speculative
abstraction over two data points. (b) A joint accessibility pass covers
both overlays together, not a palette-only or search-only fix, since the
gap and its fix are identical in both.

## 21. A UI-created group address without a range is still dropped on export — partially resolved

**Partially resolved.** `create_group_address_impl`
(`apps/knx-server/src/domain.rs`) now writes a synthetic, stable
`ets_id`/`path` (`KB-GA-<id>`) instead of the empty string it used to —
the "colliding `Id=""` attribute if export were ever wired up" half of
this limitation is fixed regardless of whether a range is given.

**Still open.** `range_id` stays optional at the HTTP boundary — a
UI-created group address with no range assigned is still silently
omitted by `crates/knx-etsproj/src/export/schema11.rs`'s exporter, which
only emits a group address nested inside its `GroupRange`. Forcing every
creation through a range needs a range *picker* in the UI, which does not
exist yet; `knx_core::Command::CreateGroupRange` (this cycle) makes
ranges creatable, but the frontend has no screen to create or choose one
from. `Command::CreateGroupAddress` now validates a *given* range
(`check_group_address_in_range`), so a range, once chosen, cannot
disagree with the address — only the choice itself isn't enforced yet.

**Originally.** [as before — the empty-`ets_id`/`range: None` behavior
this entry first documented].

**Lifted when.** The frontend gains a group-range create/pick UI
(Sub-Project 2 or later, see
[GAP_ANALYSIS_ETS.md](GAP_ANALYSIS_ETS.md)) and `range_id` becomes a
required argument to group-address creation at that point — not before,
since making it required today would break the already-shipped
range-less creation UI with nothing to replace it.

## 22. The web/Docker deployment target has no authentication

**Limitation.** `apps/knx-server` serves its HTTP API and the frontend
with no login, session, or authorization layer of any kind — anyone who
can reach the container's port can open, edit, and save the project.

**Cause.** A deliberate scope decision recorded in
[the design spec](superpowers/specs/2026-09-05-web-docker-deployment-design.md):
the stated use case is a self-hosted container on a trusted LAN, not
internet exposure, and auth is not free to bolt on afterward for a
stateful, single-project server — retrofitting it later is a separate
design, not an oversight to patch incrementally.

**Impact.** The container must not be exposed to the internet or to an
untrusted network. Nothing in `knx-server` itself enforces that boundary;
it is a deployment-time responsibility (firewalling, a reverse proxy with
its own auth, or simply staying LAN-only), not something the application
checks or warns about.

**Lifted when.** A deliberate decision to add an auth layer is made, with
its own design covering session/multi-user implications for the
single-`Mutex`-guarded-project state model this server already has.

## 23. `/api/project/download` buffers the whole `.knxdb` file in memory

**Limitation.** The route that lets the web UI save a project as a
downloaded `.knxdb` file reads the entire file into memory before writing
the HTTP response body, rather than streaming it.

**Cause.** Simplicity for the common case: `axum`'s streaming-response
plumbing (a `Body` backed by an async byte stream over a file handle)
is more code for a project file that, for every project measured so far
(including the reference project), is small enough that buffering it
costs nothing observable.

**Impact.** None for typical project sizes. A very large `.knxdb` file
would hold its full byte size in server memory for the duration of one
download request — a real cost only if project sizes grow well past what
this repository's reference project or any tested project represents.

**Lifted when.** A demonstrated need arises from a project large enough to
make buffering measurably costly; real streaming is a contained change
local to this one route, not an architectural one.

## 24. `FsPicker` has no drag-and-drop or multi-select

**Limitation.** `apps/knx-web/src/FsPicker.tsx` — the mount-directory
listing/upload UI shown in the web build when `window.__TAURI__` is
absent — supports browsing directories and picking or uploading one file
at a time. It has no drag-and-drop file upload zone and no multi-select
for batch operations.

**Cause.** YAGNI for this iteration: the design's stated goal was parity
with the desktop's native-dialog UX for opening and saving one project at
a time, not a general-purpose file manager. Neither capability was needed
to meet that goal.

**Impact.** A web user uploads files one at a time through a standard
file-input control rather than dragging one in, and cannot batch-upload
or batch-delete multiple files from the mount listing. No functional gap
for the single-project workflow the server is built around.

**Lifted when.** A demonstrated need arises — e.g. a workflow that
regularly moves several files into the mount at once — at which point
drag-and-drop and multi-select can be added to `FsPicker.tsx` without
touching the underlying `/api/fs/*` routes, which already accept one file
per request by design.

## 25. `apps/knx-web`'s declared Node version and the Docker build's Node image disagree

**Limitation.** `apps/knx-web/package.json` declares `engines.node:
">=22.12.0"`, but `apps/knx-server/Dockerfile`'s frontend build stage
(`FROM node:20-alpine`) builds it with Node 20. `npm ci` in that stage
prints a non-fatal `EBADENGINE` warning; the build still succeeds today.

**Cause.** The `engines` field was set to match the Node version already
in use for local development and CI (Node 22, per `.github/workflows/
ci.yml`'s `actions/setup-node@v4`) when `apps/knx-web` was created; the
Dockerfile's frontend stage was written independently and pinned to
`node:20-alpine` without cross-checking that declaration.

**Impact.** None today — `EBADENGINE` is a warning, not an error, and
nothing in the built frontend has been observed to need a Node
22-specific feature. It is a latent risk, not a live bug: if Node 20
reaches its upstream EOL, or a future change enables `engine-strict` in
either `npm ci` invocation or an `.npmrc`, the same build would start
failing outright instead of warning.

**Lifted when.** The Dockerfile's frontend stage is bumped to a Node 22
(or later, matching `engines.node`) base image — a one-line change,
deliberately not made speculatively ahead of an actual failure, but worth
fixing before Node 20's EOL removes the option of doing it calmly.

## 26. `BusConnection` does not yet support KNX IP Secure

**Limitation.** `crates/knx-net`'s `BusConnection` trait implements
tunnelling and discovery: `discover` multicasts a `SEARCH_REQUEST`
(original form only, not Core v2's `SEARCH_REQUEST_EXTENDED`) and
collects `SEARCH_RESPONSE`s; `connect_tunnel` opens a tunnel to a
gateway by known IP; `subscribe` receives telegrams; `TunnelClient::send`
writes one (`GroupValueWrite` or any other `ApplicationService`, no DPT
interpretation — raw bytes only, same scope cut as the receive side).
`connect_routing` (Cycle 4) sends/receives unconfirmed `ROUTING_INDICATION`
frames over the standard multicast group — no custom multicast address
override, and `ROUTING_BUSY` is decoded and logged but never used to
throttle sends (see the two new limitation entries below). Secure-protocol
paths remain unimplemented. A reader should not assume the trait is
feature-complete because it compiles.

**Cause.** Session 6 Cycle 1 delivered read-only tunnelling as the
foundation for bus monitoring; Cycle 2 added sending; Cycle 3 added
discovery. Cycle 4 added routing. Secure protocols are out of v1 scope, handled by the isolated `knx-secure` crate.

**Impact.** A real KNX installation's gateways can be found on the LAN
without a known IP once `discover()` sends a valid discovery HPAI (a
final-review fix: the discovery socket must stay unconnected to receive
unicast `SEARCH_RESPONSE`s from any gateway, so a naive `local_addr()`
read off it reported the invalid `0.0.0.0:<port>` — see the fix commit
for the resolved-IP/real-port workaround), then handed by control
endpoint to `connect_tunnel` for monitoring/actuation by group address
over a tunnel. Live-hardware verification of the full discover-then-connect
flow was left for the user to run, same as Cycle 2's `send` — this sandbox has no real KNXnet/IP gateway to discover. KNX IP Secure
remains unreachable regardless; routing (unencrypted multicast) is
reachable as of Cycle 4. Discovery does not work unmodified inside the
`knx-server` Docker container (needs
`--network host`) — untouched by this cycle, since `knx-server` doesn't
call `discover` yet.

**Lifted when.** Shelved indefinitely as of 2026-09-06 — no fixed
session or cycle owns it. Plain tunnelling/routing covers the common
case; IP Secure only matters for secure-only gateways or installations
with it explicitly enabled. Revisit on demand (a real gateway needing
it), doing the RESEARCH.md §9 spike first, not speculatively. See
[ROADMAP.md, Session 6](ROADMAP.md).

## 27. `TunnelClient` heartbeat retry has a narrow race condition — resolved

**Resolved (Session 6, cycle 5).** `crates/knx-net`'s heartbeat and
`TunnelClient::send`'s ack wait both used the same pattern — reset a
shared `Mutex<Option<T>>` reply slot, send a request, `timeout(...,
notify.notified())` once, then check the slot — which is exactly what
made the race possible: a `Notify` permit left over from a reply that
arrived just after a previous attempt gave up would wake this attempt
immediately with nothing useful in the slot, burning it without waiting
out its real budget. The shared `wait_for_reply` helper both call sites
now use loops on the same deadline instead of waiting once: a stale or
non-matching wakeup is discarded and waited past, so only a genuine
timeout or a matching reply ends the wait. Covered by
`wait_for_reply_survives_a_stale_non_matching_wakeup` and
`wait_for_reply_times_out_when_nothing_ever_matches` in `client.rs`.

**Originally.** `crates/knx-net`'s `TunnelClient` managed heartbeat
timeouts with a `tokio::select!` and a `tokio::time::sleep`. A stale
wakeup from a cancelled sleep could race the timeout branch, burning one
retry attempt unnecessarily before the real retry fired on the next cycle.

## 28. `TunnelClient` subscribers receive no signal when the tunnel closes — resolved

**Resolved (Session 6, cycle 5).** `subscribe()` now returns
`broadcast::Receiver<TunnelEvent>` instead of `Receiver<LDataFrame>`, where
`TunnelEvent` is `Telegram(LDataFrame)` or `Closed`. `receive_loop` sends
exactly one `TunnelEvent::Closed` as its last action, right after its
`select!` loop exits — reached from every exit path (explicit
`disconnect()`, the heartbeat loop exhausting its retries, a dead socket,
or a server-initiated `DISCONNECT_REQUEST`) since they all funnel through
that same loop. `apps/knx-cli`'s `bus monitor` matches on it and prints
"gateway closed the tunnel" instead of sitting in indefinite silence.
Chosen over closing the channel itself (the `Sender` lives inside the
`Arc<TunnelState>` shared by the client and the receive loop, so there is
no single owner that could drop it) or a second dedicated status channel
(one enum keeps subscribers to a single `recv()` loop).

**Originally.** `crates/knx-net`'s `TunnelClient::subscribe()` returned a
broadcast receiver that yielded telegrams. When the tunnel died — either
because the heartbeat loop exhausted its retries or the gateway went
silent — subscribers received no signal; `telegrams.recv()` simply stopped
yielding anything forever, indistinguishable from a quiet KNX bus.

## 29. `apps/knx-cli bus monitor` has formatting limitations

**Limitation.** The `knx bus monitor` subcommand, added in Session 6 Cycle 1,
always formats group addresses as three-level (e.g. `1/2/3`) regardless of
the project's configured style, and merges group-address names from all
installations into one flat namespace (last-seen wins on collision).

**Cause.** Deliberate scope decision for Cycle 1: the tool is built for
dev/smoke-testing use against the reference project, which has one installation
and uses three-level addressing throughout. Generalizing to multi-installation
projects and honouring the configured style requires mapping infrastructure
not needed for this cycle's verification workflow.

**Impact.** A project with multiple installations or a non-three-level
group-address style will see misformatted or incorrectly-merged names in the
monitor output. Data is not lost — telegrams still resolve by address internally
— only the human-readable label is approximate.

**Lifted when.** A future cycle adds full formatting respect and per-installation
name resolution, either bundled into a general bus-monitor redesign or as a
targeted enhancement to the CLI subcommand.

## 30. `/api/project/download` has no frontend caller

**Limitation.** `apps/knx-server`'s `/api/project/download` route is
implemented and covered by server-side tests (`tests/http_fs_routes.rs`),
but no code under `apps/knx-web/src` calls it — `FsPicker.tsx` wires up
directory listing and upload only. A web user has no UI path to download
a `.knxdb` file to their local machine; "Save As…" in the web build
writes to the server's mounted `data_dir` (via `saveMountPicker` in
`filePicker.ts`), not to the browser's downloads folder.

**Cause.** Out of scope for the web/Docker deployment plan as specified:
the plan's goal was serving the same editing UI over HTTP with the
mounted volume as the file store, not a download-to-browser workflow.
The route was added and tested ahead of a UI because the desktop build's
`save_project_as` needed the same underlying logic either way.

**Impact.** None for the mounted-volume workflow the deployment targets
(files already land on the server's disk, which is what's backed up/
mounted). It matters only if a user wants a local copy of a project that
lives solely on the server's `data_dir` — today they'd need direct
filesystem or `docker cp` access to the volume instead.

**Lifted when.** A demonstrated need arises for browser-side downloads;
wiring a "Download" button to the existing, already-tested route is a
small, contained `apps/knx-web` change.

## 31. KNXnet/IP routing has no custom multicast address override

**Limitation.** `RoutingClient::connect_routing` always joins the standard
KNXnet/IP System Setup Multicast Address, `224.0.23.12:3671` (Routing
v01.05.02 AS §2.3.1). No CLI flag or API parameter selects a different
group.

**Cause.** Session 6 Cycle 4's design spec deliberately hardcoded it,
same call as Cycle 3's discovery multicast address — no environment here
needs a non-default group.

**Impact.** A KNX installation using a custom routing multicast address
(needed only past 180 KNX subnetworks, or when multiple installations
share one IP network, per §2.3.2) cannot be reached by `route-monitor`/
`route-send` yet.

**Lifted when.** A real setup needs a non-default group — no fixed cycle.

## 32. `ROUTING_BUSY` is logged, not honored, by `RoutingClient` — resolved

**Resolved (Session 6, cycle 5).** `RoutingState` gained a `busy_until:
Mutex<Option<Instant>>` deadline. On receiving `ROUTING_BUSY`,
`routing_receive_loop` merges its `wait_time_ms` into that deadline via
`merge_busy_deadline` — the higher of the remaining time on any deadline
already in effect and the new frame's `tw`, exactly as Routing v01.05.02
AS §2.3.5's "device receiving ROUTING_BUSY" rule requires. `send()` now
calls `wait_out_routing_busy()` first, which sleeps until the deadline
clears (re-checking after waking, in case a later `ROUTING_BUSY` extended
it meanwhile) before transmitting. The spec's additional random back-off
after `tw` (`trandom`, driven by a moving count of recent `ROUTING_BUSY`
frames) is a `MAY`, not a `SHALL`, and is not implemented — the mandatory
stop-and-wait behavior is. Covered by
`merge_busy_deadline_keeps_the_later_of_the_two` and
`routing_client_send_waits_out_a_routing_busy_deadline` in `client.rs`.

**Originally.** Routing v01.05.02 AS §2.3.5 requires any KNX IP device to
stop sending `ROUTING_INDICATION` for a received `tw` after a
`ROUTING_BUSY` frame. `RoutingClient` decoded and logged `ROUTING_BUSY`
(and `ROUTING_LOST_MESSAGE`) but never reacted to either.

## 33. `RoutingClient`'s loopback round-trip test cannot prove correctness in every environment

**Limitation.** `routing_client_sends_and_receives_a_group_value_write`
(`crates/knx-net/src/client.rs`) sends a real telegram between two
`RoutingClient`s over UDP multicast on loopback and asserts the receiver
decoded it correctly. In a sandbox or CI runner whose network namespace
does not deliver multicast loopback locally, the test detects the
timeout and skips gracefully (logs to stderr, returns `Ok`) rather than
failing — but that skip fires *after* `connect`/`send` have already run,
so it cannot tell "this environment has no multicast loopback" apart
from "there's a real regression in `RoutingClient`'s send/receive path."
A `cargo test` pass in such an environment does not, by itself, prove
the routing round trip actually works.

**Cause.** Confirmed during Session 6 Cycle 4 implementation: this
project's own dev sandbox does not deliver multicast loopback traffic at
all (`ip route get 224.0.23.12` resolves via the physical interface, not
`lo`; reproduced independently with plain Python UDP sockets outside any
Rust code), regardless of the `IP_MULTICAST_LOOP` socket option. This is
an environment property, not a `RoutingClient` bug.

**Impact.** A real regression in `RoutingClient` could pass CI silently
in any similarly network-restricted runner. Check the test's stderr
output (a skip message is logged) or run it on a host with working
loopback multicast delivery before trusting a green `cargo test -p
knx-net` as proof that routing round-trips still work.

**Lifted when.** A `#[ignore]`-style marker or a CI capability probe
distinguishes "skipped, no proof either way" from "passed, proof
obtained" in tooling/reporting — no fixed cycle.

