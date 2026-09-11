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

**Impact.** Schema 21 is implemented and round-trip verified against one
sample (the KV project) — a first import of a *structurally different*
schema-21 project (e.g. one with `Functions`, multiple areas/lines for real,
or a `GroupObjectTree` shape this session never saw) will still likely
produce unknown-construct entries. Schema 23's module-based handling
specifically remains *inferred, not evidenced* — it reuses schema 21's
measured element/attribute set by inference (`known.rs`'s `SCHEMA_23`
table, commented as such), with no independent module-using schema-23
sample to confirm the inference. Schema 12/13/14/20/22 remain fully
undocumented-by-evidence, unaffected by this work.

**Implemented (schema 21/23 import support).** Schema 21 import and export
shipped, round-trip verified on one sample (`knx-etsproj`'s
`importing_the_kv_schema_21_project_succeeds_with_zero_unknown_constructs`).
Schema 23 import shipped, with no round-trip claim; its module handling is
flagged as inferred both here and in `ImportReport.unsupported` at runtime.
[ADR-0013](adr/0013-module-instance-representation.md) and
[ADR-0014](adr/0014-group-object-tree-authoritative-source.md) record the
design decisions this rests on.

**Lifted when.** A second, independent, module-using schema-21 or schema-23
sample project has been imported and its unknown-construct report
reconciled to empty — this would upgrade schema 23's module handling from
inferred to evidenced, and schema 21's claim from one-sample to
cross-validated. Schema 12/13/14/20/22 still need their own first sample
each, unrelated to this upgrade.

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

**Lifted when.** Per [ADR-0015](adr/0015-native-output-drops-ets-reimport-goal.md)
(Session 7), it isn't going to be: ETS reimport of our export is no longer
a project goal, so this is not scheduled to be verified. The `.knxproj`
exporter keeps working as-is and keeps saying it is unsigned; the native
`.knxdb` file (ADR-0003) is the supported round-trip format.

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

**Limitation.** Manufacturer product files in the `.knxprod` container are
fully readable, as a standalone package independent of any `.knxproj`, for
master data scheme 11 and scheme 20 (2026-09-10,
`knx_productdb::install_package`). Schemes 12-19, 21 and 22 remain unread as
a *standalone package* — they can still reach the product database bundled
inside a `.knxproj` that already contains them (see Impact below;
`knx_productdb::ingest_file` performs no scheme/namespace gating). `.vd2`,
a pre-2013 ETS2-era legacy container (SFX/`.vd_`-style, not
the same ZIP/XML family as `.knxprod`/`.knxproj` at all — confirmed by
inspection, it has no `knx_master.xml`), is explicitly and permanently
rejected: `PackageError::LegacyVd2 { sha256, len }` → `"legacy .vd2 product
data is unsupported (sha256 <64 hex chars>, <len> bytes)"`, identified by
filename suffix alone — the archive is never opened as a ZIP, never
decrypted, never parsed. As of 2026-09-11 its bytes are hashed (bounded by
the same `MAX_PACKAGE_SIZE` guard every package is subject to) so the
rejection carries the archive's hash and length as evidence; before that
date the filename check reported no evidence at all. This is a named,
external blocker (a genuinely different, undocumented legacy format), not
an untested general failure.

Note the scope: this is about standalone `.knxprod` *product packages*
(`knx products ingest`, `POST /api/catalog/install`,
`CatalogBrowser.tsx`'s install picker). Full `.knxproj` *project* import
still only has evidenced coverage at schema 11/21/23 (see
[COMPATIBILITY.md](COMPATIBILITY.md) §2/§3) — a `.knxproj` at schema 20 is
still an "expected but unverified" claim, not the same thing as this row's
now-verified scheme-20 `.knxprod` package support.

**Cause.** `.knxprod` is the same XML family as `.knxproj` (both root at
`knx_master.xml`, `http://knx.org/xml/project/{scheme}`, per *Project
Schema23 v01.00.00* §4.2.2-§4.2.3's MasterData/`M-iiii` layout); the earlier
assumption that all schemes ≥ 12 needed a still-unresolved encryption layer
(RESEARCH §10) has been disproven for schemes 11 and 20 specifically — the
5 real-world corpus files at those two schemes contain no encryption at all,
they simply hadn't been exercised through a standalone installer before.
Schemes 12-19/21/22 remain unread only because no sample of those schemes as
a *standalone `.knxprod` package* (as opposed to bundled inside a `.knxproj`)
has been acquired and tested yet — not because a new blocker was found.

**Impact.** A manufacturer's standalone `.knxprod` at scheme 11 or 20 can now
be installed directly via `knx products ingest`, the HTTP endpoint, or
`CatalogBrowser.tsx`'s install picker, without needing a `.knxproj` that
bundles it. A `.knxprod` at any other scheme, or a legacy `.vd2`, still has
to reach the product database another way (in practice, from a `.knxproj`
that already contains the application programs it references) or not at
all for `.vd2`.

**Lifted when.** For the remaining schemes: a standalone `.knxprod` sample at
that scheme becomes available and is exercised the same way
`installs_the_readable_corpus` exercises 11/20
(`crates/knx-productdb/tests/standalone_packages.rs`). For `.vd2`: never —
it is a structurally different, pre-standard legacy container, not a
variant of the current format needing decryption.

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

**Related (2026-09-10, T10).** `export_project` used to be a second
consumer of a stale `store_path`: it re-opened `store_path` off disk to
read the opaque passthrough table and manufacturer manifest, so the same
stale-pointer scenario above could attach one project's opaque/manifest
data to a different project's export. Closed for that one code path by
reading `AppState.opaque`/`AppState.manufacturer_refs` (the live,
in-memory copies) instead of re-opening the file — see
[GAP_ANALYSIS_ETS.md](GAP_ANALYSIS_ETS.md)'s C4 row. The underlying gap
above (`store_path` itself can point at the wrong file) is unchanged.

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
only emits a group address nested inside its `GroupRange`.
`apps/knx-web`'s Project Explorer now has both halves the previous
version of this entry was waiting on: a "Group Ranges" tree branch
(create/rename/delete main and middle ranges, T23 first slice,
2026-09-07) and a range `<select>` on the group-address create row, so a
user *can* pick a range at creation time. The picker's default is
"(no range)", not a forced choice, so a range-less group address remains
one click away — the gap is now "the UI allows skipping it", not "the UI
has no way to do it at all".

**Originally.** [as before — the empty-`ets_id`/`range: None` behavior
this entry first documented].

**Lifted when.** A deliberate product decision to require a range at
creation time (defaulting the picker to the first available range rather
than "none", or rejecting the create with no range chosen) — not
attempted this cycle, since forcing it changes today's already-shipped
range-less creation behavior for existing users, not just adds a new
option.

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

## 34. Schema-≥21 export drops a handful of known-but-unmapped, per-device/per-line attributes

**Limitation.** `crate::known::SCHEMA_21`/`SCHEMA_23` list several
attributes with no dedicated field on `SourceDevice`/`SourceLine`:
`DeviceInstance`'s `Comment`, `SerialNumber`, `LastUsedAPDULength`,
`ReadMaxAPDULength`, `Puid`; `Segment`'s own `Id`,
`Number`, `Puid`; and `Puid` generally, on every element that carries it.
`map.rs` folds all of these into one project-wide
`Vec<RetainedAttribute>`, keyed only by their schema-shaped xpath (e.g.
every device's `Comment` collapses to the single key
`(".../DeviceInstance", "Comment")`, indistinguishable between devices).
Confirmed against `KV v2.5 - demo.knxproj`: all 4 devices carry a
distinct `SerialNumber` and `Puid`. `knx-etsproj`'s schema-≥21 exporter
(`export/schema21.rs`) does not reconstruct any of these on export — not
because they are unrecoverable in principle, but because the flat bucket
cannot say *which* device or line a given value belongs to, and writing
one device's real hardware serial number onto every other device would
be silent data corruption, worse than the loss.

**Cause.** `installation_v21.rs`'s parser (Task 6) retains known-but-
unmapped attributes at the same schema-shaped-xpath granularity
`schema11.rs`'s own module doc already documents and accepts for
document-wide singletons like `Installation/@BCUKey` — a granularity
that was never a problem for schema 11 (every `DeviceInstance` attribute
there has a dedicated field, so no leftover ever occurs), but surfaces
for the first time at schema ≥21, where several genuinely do not.

**Impact.** Round-tripping a schema-≥21 project through this
application loses `Comment`, `SerialNumber`, `LastUsedAPDULength`,
`ReadMaxAPDULength` and `Puid` on every device, and
`Id`/`Number`/`Puid` on every `Segment` — cosmetic/bookkeeping data in
most cases (nothing else in the file refers back to a `Segment`'s own
`Id`), except `SerialNumber`, which is real hardware identification a
technician may care about.

**Lifted when.** `installation_v21.rs`'s parser gains a per-instance
xpath for `DeviceInstance`'s and `Segment`'s own leftover attributes —
the same fix Task 5 already applied to `Security` (per-device
`SourceDevice::security_raw`, not a document-wide bucket). Out of scope for
the schema-21/23 import/export plan's Task 7 (export only); tracked here
for a future fast-follow.

## 35. Device-creation `EnrichmentIssue`s are silently dropped — RESOLVED (2026-09-10)

**Resolved.** `POST /api/devices` now returns
`CreateDeviceResponse { tree, diagnostics }`. `resolve_catalog_item_program`
(`knx-productdb::query`) validates the full catalog item → product →
hardware → hardware2program → program chain before `create_device_impl`
builds a `Command::CreateDevice` at all — only a hardware row that
explicitly declares itself programless may skip program seeding; every
other dangling relation is a typed 400 before any command is applied. The
`EnrichmentIssue`s produced by seeding the ones that do go through are
mapped to typed `CreationDiagnostic`s (`ProgramlessProduct`/`AmbiguousDpt`/
`ComObjectRefMissing`/`ProgramRefMissing`/`DynamicOrModuleNotEvaluated`),
each carrying a server-computed `.detail()` string, and `CatalogBrowser.tsx`
renders them in-modal with a "Done" button instead of auto-closing when
diagnostics exist. The original limitation text is kept below for context.

**Limitation (as it stood before 2026-09-10).** `apps/knx-server`'s `create_device_impl` seeds a newly
created device's communication objects from the product database via
`knx_productdb::enrich::apply`, exactly like import's own `enrich()`
pass — except the `Vec<EnrichmentIssue>` it collects (ambiguous DPT
lists, a `ComObjectRef` id the resolved program doesn't have) is
discarded rather than surfaced anywhere. A device created against an
application program with an ambiguous DPT list on one of its
communication objects gets that communication object with no DPT set
and no visible warning.

**Cause.** Import has `ImportReport` as an existing, already-wired
channel for this; `POST /api/devices` has no equivalent yet — building
one was out of scope for this slice (see
[docs/superpowers/specs/2026-09-07-device-create-delete-design.md](superpowers/specs/2026-09-07-device-create-delete-design.md)).

**Impact.** Silent: the affected communication object is
indistinguishable, from the API's response alone, from one whose DPT
was never set on purpose. Recoverable by hand via the existing
`SetComObjectDpt` command/UI once a user notices, but nothing prompts
them to look.

**Lifted when.** Done, 2026-09-10: `create_device_impl` returns its
`diagnostics` alongside the projected `tree`, and `CatalogBrowser.tsx`
surfaces them in-modal — the same role import's own report screen (T11,
still open) would play for import.

## 36. Session log (T11): the Log tab was unreachable without an open project, and had no growth cap — resolved (2026-09-10)

**Resolved.** Two independent fixes: Part A in one commit, Part B in two —
its drop counter needed a follow-up correction, described at the end of
Part B below.

Part A: `apps/knx-web/src/App.tsx`'s "Log" toolbar button is
unconditionally enabled, and the `.workspace` slot now renders whenever
`tree` *or* `logOpen` is truthy, rather than `tree` alone —
`ProjectExplorer` still genuinely needs a project and stays gated on
`tree`, but `LogPanel` does not, so with no project open the Log tab is
the only thing in that area. With a project open, nothing changes: the
Log tab still takes the same slot it always did, and closing it returns
to Inspector/Dashboard as before. `LogPanel`'s `tree` prop is now
`ProjectTree | null`; it stays a `useEffect` dependency (so the panel
still refetches after a successful operation), and `refreshKey` —
bumped by `App.tsx`'s `reportError()` on every failed operation — is
untouched.

Part B: `apps/knx-server/src/session_log.rs` gained a documented
`MAX_ENTRIES: usize = 1000` const (not a bare literal at a call site).
Past it, `SessionLog::push` evicts the oldest real entries and pins a
synthetic `Severity::Warning`/`source: "log"` entry at index 0 naming
how many real entries have been dropped so far, refreshed on every
subsequent drop — CLAUDE.md's "never silently discard information" rule
applies to the log itself, not just to import data. That entry is never
itself dropped or duplicated, and it counts against the cap, so
`entries().len()` never exceeds 1000. `dropped` counts real entries
actually removed: the push that first exceeds the cap removes two (the
oldest real entry, plus one more to make room for the synthetic entry
itself), and every push after that while still over capacity removes
one more — an earlier draft of this counter tracked overflowing calls
instead of removed entries and read one low from the first drop
onward, caught before merge and fixed to match what actually happened
to the data. `reset()` clears the dropped count along with everything
else, so a freshly opened project starts with a genuinely empty log.
`GET /api/log`'s wire shape (`Vec<LogEntry>`, a bare JSON array) is
unchanged, so T12's own `session_log::from_csv_import_report` writer
and the existing `apps/knx-server/tests/http_log_route.rs` integration
tests needed no changes.

New tests: 6 in `session_log.rs`'s own `#[cfg(test)]` module (under the
cap, exactly at the cap, one past it — names 2 dropped — well past it —
cap + 250, names 251 dropped — reset-after-a-drop, and an invariant
test pinning "dropped named in the synthetic entry plus real entries
retained equals total pushes" at two different overflow sizes), plus a
new `apps/knx-web/src/App.test.tsx` (the first App-level test in this
project: reachable with no project open, unchanged behaviour with one
open). Gates: `cargo fmt --all --check`, `cargo clippy --workspace
--all-targets -- -D warnings`, `cargo test --workspace`, `cargo run -p
xtask -- check-layering`, `npx tsc --noEmit`, `npm test -- --run`
(139/139), `npm run build` all clean.

**Originally.** `apps/knx-web/src/App.tsx`'s "Log" toolbar button was
`disabled={!tree}`, and the whole `.workspace` div — the only place
`LogPanel` rendered — was itself gated on `tree` being non-null. But
`GET /api/log` deliberately worked with no project open (`routes.rs`
returned `200 []`, not `404`), specifically so a failed import with
nothing open yet still left an inspectable trail. Separately,
`SessionLog` had no cap on how many entries it accumulated, and
`LogPanel` refetched and re-serialized the whole log on every
`tree`/`refreshKey` change while the tab was open.

Both gaps traced to the same cause, per the plan's own text: the Log
tab's UI slot was scoped to "a project is open" from the start, since
every other panel in that slot (Inspector, Dashboard, Project Explorer)
needs one; a log entry cap was never in the design spec's stated
surface. Neither gap was caught until T11's final whole-branch review,
and both were parked as a follow-up rather than fixed in the
final-review-fix round that closed the rest of that review's findings.

Impact while open: the single highest-value scenario for this feature —
"my import just failed and no project is open, why?" — produced a
correct error entry on the server that the UI could not show. Unbounded
growth was never a problem at the usage levels this feature actually
saw (a single server process, one project at a time, log never
persisted), but nothing stopped it from becoming one over a very long
session.

## 37. Imported translations are stored but never read, and the UI is English-only

**Limitation.** `knx-productdb` parses
`Languages`/`TranslationUnit`/`TranslationElement`
out of every application program it ingests and writes them to a
`translation (program_id, language, ref_id, attribute_name, text)` table
(`crates/knx-productdb/src/migration.rs:248`, written by
`parse/translation.rs`). `knx-core` has carried the matching model
indirection since day one: `Language`, `TranslationKey`,
`LocalizedString`, and a `StringTable` that resolves against an active
language with a `default_language` fallback, with `Project` owning a
`strings: StringTable` (`crates/knx-core/src/project.rs:182`).

Nothing reads any of it for display. `grep` for `translation` across the
workspace finds the parser that writes the table, the migration that
creates it, and nothing else — no query, no join, no resolution at any
render site. Separately, every user-facing string in `apps/knx-web` is a
hard-coded English literal, and `apps/knx-web/package.json` has no i18n
dependency of any kind.

**Cause.** The storage side was built where it belonged (Session 4's
product-database ingestion, Session 2's domain model) and the reading
side was never scheduled, because no screen that needed it existed yet.
The frontend was built English-first and no cycle since has revisited
that. Neither is a bug in anything that shipped; both are simply
unbuilt halves.

**Impact.** A German-language product catalog imported from a `.knxprod`
displays whatever single string the parser happened to put in the
non-translated attribute, with the translations sitting unread in the
database beside it. Users outside English see an English application. No
data is lost — this is the good case for the "never silently discard"
rule, since the translations *are* preserved on disk — but preserved and
unreachable is not the same as available.

**Lifted when.** Open. Tracked as **T25** (UI chrome) and **T26** (KNX
data) in [GAP_ANALYSIS_ETS.md](GAP_ANALYSIS_ETS.md)'s Tier 6, added
2026-09-10, closing gap **D10**. Neither has a design spec or a
scheduled cycle. Until then the translations remain queryable directly
from the `.knxdb` product database with SQL, which is a developer
workaround and not a feature.

## 38. Group-address CSV export/import (T12) has no verified ETS interoperability

**Limitation.** "KNXBench group-address CSV v1" (`crates/knx-csv`,
[IMPORT_EXPORT.md §11](IMPORT_EXPORT.md#11-group-address-csv-exchange)) is
a format this project defines and documents itself. It is not, and cannot
currently be shown to be, compatible with ETS's own "Export Group
Addresses" CSV feature, or with the legacy `.esf`/OPC export format.

**Cause.** No sample of either format exists anywhere in this repository,
and searching all 179 documents of the extracted KNX Standard v3.0.0
corpus for `csv`, `esf`, `OPC export`, and group-address-export
terminology turned up nothing but two incidental prose hits (a
data-security test report and an RF application note) — there is no
standardized group-address exchange text format at all. Group-address CSV
export is an ETS *application* feature, not something the KNX Association
specifies, so there is nothing to read except a real file, and none has
been obtained.

**Impact.** A file exported by KNXBench is not guaranteed to open sensibly
in ETS, and a CSV exported from ETS is not guaranteed to import cleanly
here — the importer is column-name-driven and separator-detecting
specifically so a foreign file has a *fair chance*, but that is a design
mitigation, not a tested claim. Nothing in the UI, CLI output, or this
documentation set may say "ETS CSV" or imply interoperability, and none of
it does.

**Lifted when.** A genuine ETS-produced group-address CSV export is
obtained. At that point, adding a second, ETS-shaped column profile to
`crates/knx-csv`'s reader is the stated upgrade path — header matching is
already isolated in one function (`map_headers`), so the profile would go
there rather than spreading through the parser, though that function is a
hard-coded `match` and would itself have to be edited. `.esf`
import is a separate, larger undertaking (writing a parser against
remembered syntax with no sample to check it against is exactly what
CLAUDE.md's "do not invent technical facts" forbids) and would need its
own task, gated the same way on first obtaining a real file.

## 39. CSV import never re-addresses, deletes, or manages group ranges

**Limitation.** Importing a "KNXBench group-address CSV v1" file can only
create new group addresses and update the `Name`/`Central`/`Unfiltered`
fields of existing ones. Three related things it deliberately does not do:
it never re-addresses an existing entry (changing the `Address` cell for a
row that matched an existing entry is read as "create a new entry at the
new address," leaving the old one in place, because the address is the
row's match key); it never deletes an entry that exists in the project but
is simply absent from the file; and it never creates, renames, or targets
group ranges — a newly created address is placed into whatever existing
range already contains it by bounds, or left without a range if none does,
but the ranges themselves are untouched by a CSV import.

**Cause.** A deliberate design choice
(`docs/superpowers/specs/2026-09-10-csv-group-address-exchange-design.md`
§4), not a missing feature: the address is the only stable identity a CSV
row has (names are not unique), so treating an address edit as a move
would require guessing intent from a spreadsheet diff; treating "absent
from the file" as "delete this" would make a partial or filtered export
catastrophic to re-import; and group-range CRUD is an unrelated, already
separately-modelled concern (`Command::CreateGroupRange`/
`RenameGroupRange`, T5/T23) that a bulk name/flag editor has no business
reaching into.

**Impact.** Re-addressing a group address still requires the existing
delete-then-recreate workflow in the group-address view, or hand-editing
via the group-address commands directly — a CSV round trip cannot do it in
one step. Someone who deletes rows from an exported file before
re-importing it, expecting a "sync to this file" semantics, will find the
deleted rows' addresses untouched in the project rather than removed.

**Lifted when.** Open. No task currently proposes changing this — it is
recorded here as a boundary of the feature, not a gap awaiting a fix.

## 40. CSV export-only columns are never applied on import, and there are no `Description`/`Comment` columns

**Limitation.** `DatapointType`, `MainGroup`, and `MiddleGroup` appear in
an exported CSV so the file is useful to read and edit, but importing that
same file back never applies any of the three — they are recognized and
reported as ignored, never rejected and never silently dropped, but never
written to the project either. Separately, the CSV format has no
`Description` or `Comment` column in either direction, even though the
`.knxproj` schema itself defines `GroupAddress/@Description` and
`@Comment` attributes.

**Cause.** A group address in this domain model (`GroupAddressEntry`,
`crates/knx-core/src/group.rs`) carries no datapoint type at all — a DPT
belongs to the communication objects linked to the address, several of
which may legitimately disagree, so there is no single value a CSV row
could write back onto the address itself. `MainGroup`/`MiddleGroup` name a
*containing* group range, which is structure, not a field of the address,
so writing one back would mean silently moving the address between ranges
from a rename-focused editor. `Description`/`Comment` are simply not
modelled anywhere in `GroupAddressEntry` yet — the CSV cannot round-trip a
field the domain model does not have.

**Impact.** A user who edits the `DatapointType`, `MainGroup`, or
`MiddleGroup` cell of an exported row and re-imports it will see that edit
reported as ignored rather than applied — surprising the first time, but
never silent. There is no way to bulk-set or bulk-view a description or
comment for a group address via CSV, because there is nowhere in the
project for it to live yet.

**Lifted when.** `MainGroup`/`MiddleGroup` becoming applicable is tied to
group-range assignment gaining its own dedicated editing UI/command rather
than being folded into a name-and-flags import. `Description`/`Comment`
becoming available is tied to `GroupAddressEntry` gaining those fields in
the domain model — no task currently schedules either.

## 41. A CSV file saved from Excel under a German locale may still surprise a user

**Limitation.** The importer auto-detects `,` and `;` as the field
separator per file, specifically because Excel's own CSV export/import
behavior depends on the OS list separator setting: under a German
(or otherwise comma-decimal) locale, Excel writes `;`-separated CSV and
expects `;` back on open, while under an English locale it uses `,`. Both
are accepted here. What is not handled is everything else Excel can do
to a file beyond the separator — most notably re-saving with a different
encoding, a different quoting style for edge-case cells, or altering
numeric-looking cells (an `Address` value or a boolean-looking cell) in
locale-specific ways during a manual edit.

**Cause.** The separator auto-detection in `crates/knx-csv/src/read.rs`
covers the one Excel behavior this project could concretely name and test
against (`parses_the_same_file_semicolon_separated`). Excel's broader
locale-dependent quirks are not enumerated anywhere in this codebase or
its research, and guessing at more of them without a concrete failing
sample would be exactly the kind of unverified assumption CLAUDE.md rules
out.

**Impact.** Most Excel round trips work because of the separator
detection. A user on a German-locale machine who hand-edits an exported
file in Excel and hits an import error on a cell Excel silently reformatted
should not assume the importer is broken — it is a known category of risk
with this specific tool, not a claim that every Excel edit is safe.

**Lifted when.** A concrete Excel-induced parse failure is reported with a
reproducing file, at which point it becomes a specific, testable case
rather than a general caution.

## 42. `command_sync.rs`'s module doc overstates its own role — pre-existing, not introduced by T12

**Limitation.** `crates/knx-store/src/command_sync.rs`'s module-level doc
comment describes `sync_after_command` as *the* incremental persistence
mechanism for command edits ("writes only the row(s) that command's own
target id(s) name … Incremental command sync"). Grepping `crates/` and
`apps/` for `sync_after_command` finds exactly three kinds of hits: the
function's own definition and tests inside `command_sync.rs`, a bare
re-export at `lib.rs:18`, and three doc-comment mentions in `devices.rs`.
There is no actual caller anywhere in either `crates/` or `apps/`.

**Cause.** Pre-existing — this function predates T12 and was never wired
into the server's or CLI's actual save path, both of which persist a
command's effect by calling `save_project` (a full project write) after
`Command::apply`, not by calling `sync_after_command`. Not caused by this
task. T12's own `Command::UpdateGroupAddress` gained a `command_sync.rs`
match arm that is itself a documented no-op stub — the same pattern
already used there for the topology/group-range/group-link and
device-create/delete variants — which sits in the same file as the
overstated module doc and makes the discrepancy easier to trip over for
the next person reading that file top to bottom.

**Impact.** None on correctness today: every command-driven edit this
application makes is actually persisted via `save_project`, which is
unconditional and does not depend on `sync_after_command` at all. The risk
is purely to a future reader who trusts the module doc at face value,
concludes `sync_after_command` is live, and either relies on it being
called somewhere it isn't or spends time looking for a caller that does
not exist.

**Lifted when.** Open. Either the module doc is corrected to say
`sync_after_command` is currently unused and persistence runs through
`save_project`, or `sync_after_command` is actually wired in as the
faster incremental path its doc already claims to be (at which point
every no-op stub arm, including T12's new one, would need a real
implementation too). Neither is scheduled; flagged here so the gap is
findable without re-deriving it from a grep.

## 43. Animations have no in-app switch; only the OS reduced-motion preference

**Limitation.** The web UI's transitions and hover animations cannot be
turned off, slowed, or otherwise controlled from inside the application.
`apps/knx-web/src/styles.css` declares `--knx-transition-duration: 250ms`
and wraps every transition in one of three `@media (prefers-reduced-motion:
no-preference)` blocks, so the operating system's reduced-motion setting is
the only switch a user has — and it is all-or-nothing. No `.ts` or `.tsx`
file in `apps/knx-web/src` references motion, duration, or that token at
all.

**Cause.** Regression, not an omission. Session 5 cycle 11 shipped a
three-level `off`/`subtle`/`standard` motion setting that wrote the
duration token, living in `ThemePanel.tsx` alongside four user-colorable
theme tokens. Cycle 13 replaced the whole theming approach with a named
`ThemeDef`/`THEMES` registry and deleted `palette.ts`, `palette.test.ts`
and `ThemePanel.tsx` outright, because a four-token override does not map
onto a 12+-token theme package. The motion setting was collateral: it had
nothing to do with color overrides but happened to share their surface.
Cycle 13's own [design spec](superpowers/specs/2026-09-08-bitcoin-defi-theme-design.md)
records the outcome plainly — "Motion: no user-facing setting (that was
`palette.ts`'s job, now gone)" — so this was noticed at the time and
accepted, not overlooked.

**Impact.** Small today and growing. The current animations are short
CSS transitions on hover and theme change, and a user bothered by them
can set the OS preference. It matters more as soon as motion-heavy
features arrive — a live Group Monitor, a line-scan progress display,
graphical topology views, and the deferred "who talks to whom" telegram
animation are all on the backlog and all inherently animated. A user who
wants a still UI without telling their whole desktop environment so has
no way to ask for one, and an engineer working in front of a customer
has a reasonable claim to that.

**Lifted when.** T27 (GAP_ANALYSIS_ETS.md Tier 7, gap D11) restores an
in-app motion control and binds future animated features to it, with
`prefers-reduced-motion: reduce` still overriding the in-app choice
rather than the reverse. Requested explicitly on 2026-09-10
("die Animationen sollen togglebar sein, wenn sie implementiert werden");
not scheduled into a cycle yet, and deliberately written down before the
animated features exist rather than after.

## 44. Project documentation export (T13) has no ETS report parity, and none can currently be measured

**Limitation.** `crates/knx-report`'s HTML document
([IMPORT_EXPORT.md §12](IMPORT_EXPORT.md#12-project-documentation-export))
is KNXBench's own document. It is not, and cannot currently be shown to
be, similar in content or layout to any report ETS's own printing feature
produces.

**Cause.** No ETS-produced report sample — no PDF, no printout, no
exported document of any kind — exists anywhere in this repository, and
`docs/RESEARCH.md` has no section describing ETS's report layout. This is
the same evidence gap [§38](#38-group-address-csv-exportimport-t12-has-no-verified-ets-interoperability)
records for T12's CSV format: printing/reporting is an ETS *application*
feature, not something the KNX Association standardizes, so there is
nothing to read except a real sample, and none has been obtained.

**Impact.** Nothing in the UI, CLI output, or this documentation set may
say "ETS report" or imply compatibility with one, and none of it does. A
user expecting the document to resemble an ETS printout in section order,
wording, or completeness has no basis for that expectation from anything
KNXBench ships.

**Lifted when.** A genuine ETS-produced report sample (PDF or printed
export) is obtained. At that point a content-set comparison becomes
possible for the first time; whether that motivates layout changes is a
separate decision to make once evidence exists.

## 45. Project documentation export has no native PDF output

**Limitation.** `crates/knx-report` produces HTML only. There is no Rust
PDF renderer anywhere in this workspace, and none is planned.

**Cause.** A deliberate scope decision
(`docs/superpowers/specs/2026-09-10-project-documentation-export-design.md`
§2, §9): every modern browser already prints to PDF, the document ships
`@media print` rules for exactly that, and a Rust PDF-rendering dependency
would be a large addition serving a button the operating system already
provides. CLAUDE.md: avoid unnecessary dependencies.

**Impact.** Producing a PDF requires opening the exported `.html` file in
a browser and using its print-to-PDF path. There is no `knx doc-export
... --pdf` or equivalent, and no headless/server-side PDF generation for
automation that cannot drive a browser.

**Lifted when.** Open. No task currently proposes a native PDF renderer —
recorded here as a boundary of the feature, not a gap awaiting a fix.

## 46. Project documentation export does not resolve manufacturer, product, or program names

**Limitation.** The Devices section of the exported document prints
`product_ref` and `program_ref` as the raw, opaque identifiers stored on
each `DeviceInstance` (`device.rs:27-31`) — never a resolved manufacturer
or product name.

**Cause.** `crates/knx-report` depends only on `knx-core`, `knx-projection`,
and `chrono` (`xtask check-layering` enforces this, the same rule
`knx-csv` is held to). Resolving those identifiers to a human-readable
name requires querying `knx-productdb`, a separate, independently
versioned database this crate must not reach.

**Impact.** A reader has to cross-reference `product_ref`/`program_ref`
against the product database (or the `CatalogBrowser` UI) by hand to learn
what a device actually is beyond its own name/description.

**Lifted when.** Open. A future task could pass an already-resolved
lookup table into `ReportOptions` from a caller that *does* have
`knx-productdb` access (`apps/knx-server`, `apps/knx-cli`), without
`knx-report` itself gaining the dependency.

## 47. Project documentation export does not list parameter values or module-instance arguments

**Limitation.** Parameter values and module-instance arguments are
counted in the Summary section's totals but never listed individually
anywhere in the document.

**Cause.** Both are stored uninterpreted in this domain model — parameter
values as raw strings (RESEARCH R3, no `when`/`choose` grammar
interpretation yet, [§3](#3-device-parameters-are-preserved-but-not-interpreted));
module-instance arguments as opaque data. Printing raw `RefId`/value pairs
by the hundreds or thousands would be volume without meaning until T18's
parameter interpretation work exists to give them one.

**Impact.** The document cannot answer "what is this device configured
to do" beyond its communication objects' flags and DPTs — the same
limitation the rest of the application has toward parameters, now visible
in the exported document's own text (its "What this report does not
contain" section states this explicitly).

**Lifted when.** T18 (parameter interpretation and editor,
`GAP_ANALYSIS_ETS.md` Tier 5) exists and a follow-up task extends
`knx-report` to use it. Not scheduled.

## 48. Project documentation export renders in one language only

**Limitation.** The document renders text in the project's default
language only — there is no language selector and no per-string
translation lookup.

**Cause.** [§37](#37-imported-translations-are-stored-but-never-read-and-the-ui-is-english-only)
already applies to this document: `knx-productdb`'s `translation` table
has no reader anywhere in the codebase, and `knx-report` in particular
must not reach `knx-productdb` at all (see §46).

**Impact.** A multi-language project's translated strings never appear in
the exported document, regardless of which language a user might prefer.

**Lifted when.** T26 (language-aware display of imported KNX data,
`GAP_ANALYSIS_ETS.md` Tier 6) gives the application an active-language
concept and a translation reader; `knx-report` would need its own
follow-up to consume it, since it cannot reach `knx-productdb` directly.

## 49. Project documentation export has no in-application print preview

**Limitation.** There is no preview of the exported document inside
KNXBench itself, on the web frontend or the CLI. "Export documentation…"
writes a file; seeing it means opening that file in a browser.

**Cause.** A deliberate scope decision
(`docs/superpowers/specs/2026-09-10-project-documentation-export-design.md`
§9): the browser already provides a preview (the page itself, and its own
print-preview dialog), so building a second one inside the application
would duplicate it.

**Impact.** A user cannot see the rendered document without leaving the
application and opening the written file in a browser tab.

**Lifted when.** Open. No task currently proposes an in-app preview pane.

## 50. Project documentation export has no section selection

**Limitation.** `render_html` always renders every section — Header,
Contents, Summary, Topology, Buildings, Group addresses, Devices, and
"What this report does not contain." There is no way to request, say,
"just the group addresses" or "just the devices."

**Cause.** A deliberate scope decision
(`docs/superpowers/specs/2026-09-10-project-documentation-export-design.md`
§6, §9): `ReportOptions` intentionally carries only `generated_at`.
CLAUDE.md: avoid speculative abstractions — a selection knob is easy to
add later if someone actually asks for a partial report; adding it before
then is a guess about a feature nobody has requested.

**Impact.** Exporting documentation for a large project always produces
the full document, even if only one section is of interest — on the
reference project, roughly 249 KB of HTML for 36 devices, 907
communication objects, and 514 group addresses.

**Lifted when.** Open. A real request for partial reports would motivate
adding a selection parameter to `ReportOptions`; none has been made.

## 51. Project diff (T14) has no ETS-comparison parity, and none can currently be measured

**Limitation.** `crates/knx-diff`'s output — a "KNXBench project diff" —
is KNXBench's own comparison. It is not, and cannot currently be shown to
be, similar in matching rules, content, or presentation to whatever
ETS's own project-compare feature produces.

**Cause.** No ETS-produced comparison output — no screenshot, no exported
report, no printed diff — exists anywhere in this repository, the same
evidence gap [§44](#44-project-documentation-export-t13-has-no-ets-report-parity-and-none-can-currently-be-measured)
records for T13's HTML report and [§38](#38-group-address-csv-exportimport-t12-has-no-verified-ets-interoperability)
records for T12's CSV format: project comparison is an ETS *application*
feature, not something the KNX Association standardizes, so there is
nothing to read except a real sample, and none has been obtained.

**Impact.** Nothing in the UI, CLI output, or this documentation set may
say "ETS compare" or imply compatibility with it, and none of it does —
`knx-diff`'s own module doc and its design spec (§1) state this
explicitly. A user expecting the diff to match what ETS's own compare
screen would show — which entities it matches, which fields it compares,
how it presents a rename — has no basis for that expectation from
anything KNXBench ships.

**Lifted when.** A genuine ETS-produced comparison sample is obtained. At
that point a content-set comparison becomes possible for the first time;
whether that motivates changes to the matching rules or the rendered
output is a separate decision to make once evidence exists.

## 52. Project diff cannot correlate a device with no individual address and no matching `ets_id`

**Limitation.** A device's natural key
(`docs/superpowers/specs/2026-09-10-project-diff-design.md` §3.4) is its
individual `address`, and only when `Some`. A device with no individual
address relies entirely on an `ets_id` match; if that also fails to line
up between the two projects being compared, `diff_projects` cannot
correlate the two at all — the device surfaces as an unrelated `removed`
on one side and `added` on the other, never as a match with field
changes.

**Cause.** A deliberate scope decision (design spec §3.4, §9): there is
no stronger identity to fall back on. Guessing would risk a false match
between two genuinely different devices, which CLAUDE.md's
never-silently-discard/never-guess posture rules out.

**Impact.** Two saves that differ only in, say, a description edit on an
address-less device can be reported as one device removed and a
different device added, obscuring what was actually a single edit.

**Lifted when.** Open. No stronger per-device identity exists in the
domain model today; recorded as a boundary of the natural-key approach,
not a bug awaiting a fix.

## 53. Project diff can collide two same-named sibling building parts

**Limitation.** A building part's natural key is the path of names from
the root (design spec §3.4). Two siblings under the same matched parent
that share a name produce the identical path and therefore collide under
the ambiguity rule (design spec §3.3 step 3): both are reported as
individual `added`/`removed` entries, plus one `AmbiguityNote`, rather
than matched to each other.

**Cause.** The same limitation `knx-etsproj::compare`'s
`semantic_building_part` already accepts for its own single-parent-hop
identity (design spec §3.4's own note): a name-based key has no way to
distinguish same-named siblings, and building parts carry no other
stable identity once their `ets_id`s also fail to correlate.

**Impact.** Renaming, or otherwise editing, one of two same-named sibling
building parts between two saves can render as an ambiguous add/remove
pair instead of a clean field change.

**Lifted when.** Open. Recorded as a boundary of the path-based key, not
a bug awaiting a fix.

## 54. Project diff does not detect an ETS re-import's regenerated `RefId`s as "the same project"

**Limitation.** ETS may regenerate `RefId` strings on a fresh re-import
of a `.knxproj` it has seen before. `diff_projects` has no special case
for this: if the natural key also does not line up for a given entity, a
re-import can present as widespread adds/removes rather than "nothing
changed" or "one field changed".

**Cause.** Design spec §3.2, §9: no special-case re-import detection is
built. The corpus test in `crates/knx-app/tests/project_diff.rs`
demonstrates the property that *does* hold — two independent imports of
the *same* `.knxproj`, by this repository's own importer, produce an
empty diff, because this importer's own `RefId` mapping is stable
run-to-run. Whether ETS's own `RefId` regeneration would break that
stability is untested — no such case has been observed in this
repository's corpus.

**Impact.** A `.knxdb` re-created from a re-exported `.knxproj` whose
`RefId`s changed may compare as a large, misleading set of adds/removes
against the original `.knxdb`, even where nothing meaningful changed.

**Lifted when.** Open. Would need either a documented, stable KNX
`RefId`-regeneration rule to compensate for, or a demonstrated real-world
case to design against; neither exists yet.

## 55. Project diff cannot merge or apply a diff back onto a project

**Limitation.** `diff_projects` computes and shows what changed; it does
not turn a `ProjectDiff` back into a `Command` sequence that could replay
one project's changes onto another.

**Cause.** Design spec §9: a materially larger feature — every
field-level change would need an inverse `Command`, and some fields (a
device's `product_ref`/`program_ref`) have none today — not asked for by
the T14 backlog line.

**Impact.** Reviewing a diff and then manually re-applying the same
edits to another project remains a manual, error-prone step; there is no
"apply this change" control anywhere in the diff panel.

**Lifted when.** Open. No task currently proposes it.

## 56. Project diff does not do a three-way comparison

**Limitation.** `diff_projects` takes exactly two projects. There is no
common-ancestor-aware three-way comparison the way a VCS merge does one.

**Cause.** Design spec §9: nothing in this codebase tracks project
ancestry or a common base to diff against.

**Impact.** Reconciling two independently edited copies of the same
original project has no tool support beyond running the two-way diff
twice, once against each candidate.

**Lifted when.** Open. Would need a project-ancestry or version-history
concept that does not exist today.

## 57. Project diff cannot compare against a raw `.knxproj`

**Limitation.** Both sides of a comparison must already be `.knxdb`
files. `knx diff <a.knxdb> <b.knxdb>` on the CLI takes two `.knxdb`
paths; `POST /api/project/diff {path}` compares the server's open,
in-memory project against one `.knxdb` file at `path`. Neither accepts a
`.knxproj` on either side.

**Cause.** Design spec §7, §9: `knx-diff` must not depend on
`knx-etsproj`, and importing a `.knxproj` first would need the surface
layer to do it, doubling the failure modes a comparison route has to
explain (a bad `.knxproj` fails for import reasons; a bad `.knxdb` fails
for store reasons) for a use case the T14 backlog line does not ask for —
"what changed between these two **saves**" is a `.knxdb` question, not a
`.knxproj` one.

**Impact.** Comparing an ETS-exported `.knxproj` directly against a
KNXBench `.knxdb` save — or two `.knxproj` files against each other —
requires importing each one into a `.knxdb` first (`knx import`), outside
the diff feature itself.

**Lifted when.** Open. No task currently proposes accepting a raw
`.knxproj` as a comparison side.

## 58. Project diff has no CI-friendly "exit nonzero on any difference" flag

**Limitation.** `knx diff` always exits `0` when it successfully produces
a comparison, whether or not the two projects differ. There is no flag
to make a nonempty diff a nonzero exit code.

**Cause.** Design spec §9: mirrors `knx doc-export`'s own reasoning
(`apps/knx-cli/src/main.rs`) — a diff with changes is not a failed diff.

**Impact.** A script cannot currently gate on "these two `.knxdb` files
differ" using `knx diff`'s exit code alone; it would need to parse the
printed text instead.

**Lifted when.** A real feature request for scripted gating arrives; a
small, well-scoped addition at that point, not built speculatively now.

## 59. Project diff's text and web renderers show which fields changed, not their before/after values, for most entity types

**Limitation.** For every entity table below the project/installation
level (areas, lines, devices, group ranges, group addresses, building
parts, communication objects, parameters), both `knx diff`'s plain text
and the web diff panel print only the *names* of the fields that changed
(`changed_fields`, e.g. `name, commissioning`) — never the old and new
values themselves. Project-level (`ProjectDiff.info_changes`) and
installation-level (`InstallationDiff.field_changes`) changes are the
exception: both render as `FieldChange { field, left, right }`, so those
two levels *do* show both values.

**Cause.** A rendering-only scope decision, not a data-loss one:
`knx_diff::EntityChange<K, F>` and `knx_diff::DeviceChange` retain the
full matched pair (`left: F`, `right: F`) alongside `changed_fields` —
nothing is discarded computing the diff (CLAUDE.md: never silently
discard information). Neither the CLI's plain-text renderer
(`apps/knx-cli/src/main.rs`) nor the web panel
(`apps/knx-web/src/ProjectDiffPanel.tsx`) currently walks `left`/`right`
field by field to print a value pair for these tables; only the summary
list is rendered.

**Impact.** Seeing what a changed device's `name` actually changed *to*
means reading the JSON response from `POST /api/project/diff` directly,
or extending the renderer — the CLI and web panel today answer "what
changed" at the field-name level, not "what changed to what," for
anything below project/installation scope.

**Lifted when.** A renderer change (CLI and/or web) walks
`EntityChange`/`DeviceChange`'s retained `left`/`right` and prints both
values per changed field; the data to do so already exists in
`knx-diff`'s own types today.

## 60. Project diff's web panel shows grouped counts only

**Limitation.** `ProjectDiffPanel.tsx` renders one summary line per
non-empty entity table (e.g. `Devices: 1 added, 2 changed`) across the
whole report. There is no tree view of individual added/removed/changed
entities, and no inline before/after value highlighting anywhere in the
panel.

**Cause.** Design spec §9, explicit out-of-scope: "no tree view, no
inline before/after text highlighting" — the same visual register as the
existing Log tab (`LogPanel.tsx`), not a richer side-by-side diff view.

**Impact.** A user who wants to see *which* device was added, or the
actual old/new value of a changed field, cannot do so from the web panel
alone — only counts per table, per installation.

**Lifted when.** Open. A richer visual diff view is a real, larger
feature a future task could propose; not built speculatively now.
