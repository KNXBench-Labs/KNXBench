# Known limitations

Each entry states the limitation, its cause, what it costs the user, and the
condition under which it would be lifted. Nothing here is a defect to be fixed
by trying harder — these are consequences of evidence we do not have or of
decisions recorded in [docs/adr/](adr/).

The companion document is [COMPATIBILITY.md](COMPATIBILITY.md), which states
what is verified. Nothing may appear as verified there and as a limitation
here.

## 1. Single-sample bias

**Limitation.** Everything verified about the `.knxproj` format comes from one
installation: schema 11 (ETS 4.1.8) and schema 23 (ETS 6.3.7959.0) — the same
project, exported twice (risk R1).

**Cause.** No independent ETS5 or ETS6 sample project has been available. The
second export of the ETS4 reference project (RESEARCH §2.4/§3.3) confirms the
schema-11→23 format diff for this one installation, but says nothing about
schema 12, 13, 14, 20, 21, 22, and nothing about a differently-structured
project on schema 23 (e.g. one using `Functions`, KNX Secure, or multiple
areas/lines for real).

**Impact.** Support for schema 12, 13, 14, 20, 21 and 22 is still derived from
documentation and from reading `xknxproject`, not from evidence. Schema 23
support is derived from evidence but only from one project shape; a first
import of a structurally different schema-23 project will still likely
produce unknown-construct entries.

**Session 3 status.** `knx-etsproj`'s known-element table covers schema 11
only, built from this one reference project. Schema 23 is **detected and
refused by name** (`ImportFailure::NoKnownSchemaTable { version: 23 }`) —
never silently misread through the schema-11 table, which would undercount
communication objects by about 24% and misread every boolean flag as false
(RESEARCH §3.3). Building the schema-23 known-element table is Session 4
work, not attempted here.

**Lifted when.** Real ETS5 projects and further, independent ETS6 projects
have been imported and their unknown-construct reports reconciled to empty.
This is a prerequisite for claiming more than schema 11 and this one
schema-23 shape.

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

## 17. Deleting a group address can leave a dangling `GroupLink`

**Limitation.** `knx_core::command::Command::DeleteGroupAddress` removes the
`GroupAddressEntry` from `Installation::group_addresses` but does not scan
`Devices` for any `ComObjectInstance.links` entry that pointed at it. A
communication object can therefore end up with a `GroupLink` naming a group
address id that no longer exists in the project.

**Cause.** The command was written against the group-address list alone;
finding every com object that might reference a given group address needs
either an index the domain model does not maintain or a full device scan,
and neither was in scope when the command was added.

**Impact.** In memory, nothing visibly breaks — the dangling link is just
an id that resolves to nothing if looked up. Persisting the project is
where it surfaces: `knx-store`'s `sync_after_command` deletes the group
address row correctly (`group.rs::delete_group_address` also removes its
own `group_link` rows), but a later *full* `save_project` re-derives every
`group_link` row straight from each `ComObjectInstance.links` in memory —
including the dangling one — and fails with a foreign-key violation against
`group_address(id)`.

**Lifted when.** `Command::DeleteGroupAddress` (or a helper it calls) also
walks `Devices` and removes/flags every `GroupLink` naming the deleted
address — a `knx-core` change, not a `knx-store` one, and its own design
decision (cascade-delete the links silently, or surface them as an import/
edit-report finding first) rather than a one-line fix.

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

**Impact.** None reachable through the current UI: `apps/knx-desktop/src/
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

**Limitation.** Picking a result from `Ctrl+K` search (`apps/knx-desktop/
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

**Limitation.** `apps/knx-desktop/src/Search.tsx` and `CommandPalette.tsx`
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

