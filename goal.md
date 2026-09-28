# KNXBench goal — everything still open, minus commissioning

Written 2026-09-20. Use this file as the instruction passed to `/goal`. Its
creation alone starts no run.

**Handback from Paperclip (2026-09-27): read §12 first.** From 2026-09-25 to
2026-09-27 the backlog ran as a Paperclip experiment (company "DingsBumbs
Labs", issues DIN-1 to DIN-53). That experiment has now stopped, and all of its
agents are paused. This file is once again the only backlog, and the standing
goal returns to the Hermes `knxbench` profile. §12 has the verified status map,
the unmerged Paperclip branches and the order in which to take them over. The
2026-09-23 pause boundary described below, and rule 15, are superseded by the
user's handback. Paperclip's DIN-3 plan meant to cut this file down to a
25-line pointer (branch `din-3-goal-migration`, `90add0c`). **Do not merge
that branch.** It is obsolete.

**Execution boundary (2026-09-23, superseded 2026-09-27 by §12):** User requested completion of the already
open T13 through verified merge, then a pause. T14 and every subsequent task
remain unstarted; the read-only T14 survey is not implementation approval.
Do not resume the standing goal without a new explicit user instruction.
T13 is now merged as `7f9c8c4`; all eleven merged-main gates passed. The standing
Hermes goal remains paused, not completed.

**Backlog amendment (2026-09-23, no resume):** The user supplied a new local
Gira/MDT product-database corpus and explicitly asked that its verified import
findings become future goal work. Section 2.8 now owns PDB-1 through PDB-11;
the legacy `.pr5` prerequisite is in §7. This amendment changes the backlog and
the prior product-scheme exclusion, but does not itself resume the paused goal.

Drive KNXBench toward a trustworthy v1 on every front **except** commissioning.
Work autonomously and persist across turns until every item below is either
resolved with evidence, or explicitly accepted out of scope by the user.

The previous version of this file (git history, commit `7f08b59` and earlier)
drove the 2026-09-13 goal-completion run. That run finished: thirty tasks plus
nineteen commissioning-conformance tasks merged, the documentation
reconciliation (T24) landed, and the single closing Fable review ran on
2026-09-20 and had its findings fixed (`bb1fa66`, `a927691`, `85b5783`). This
file replaces it with what is left over, taken from `docs/ROADMAP.md`,
`docs/GAP_ANALYSIS_ETS.md`, `docs/KNOWN_LIMITATIONS.md`,
`docs/LIMITATION_TRIAGE.md`, `docs/IMPLEMENTATION_STATUS.md`, `docs/adr/`,
`ideas.md`, `codex-goal.md` and the 2026-09-20 status audits.

**Repository state this file was written against:** `main` at `7f08b59`, level
with `origin/main`, clean checkout, no extra worktrees. Product version
`0.1.0-alpha.1` everywhere, no release tag.

---

## 0. What this goal deliberately excludes

**Update 2026-09-28: commissioning has its own goal file,
[`goal-commission.md`](goal-commission.md).** It owns exactly what this
section excludes, plus KNOWN_LIMITATIONS §136, in the separate commissioning
session. The two files do not overlap; `goal-commission.md` §5 is the
boundary table. Items it hands over reach this file through §12.4.

**Update 2026-09-28: the UX/UI issues of §11 have their own goal file too,
[`goal-ui.md`](goal-ui.md)**, run by a separate GPT/Codex session. It owns
the web chain and the web lock. What stays here is listed in §12.3.

**Commissioning and every write to real KNX hardware is out of scope for this
run.** Not cancelled, not downgraded — the user ruled on 2026-09-11 that
commissioning must work, and the ROADMAP's 2026-09-20 ruling defers phase 3
only until dedicated test hardware exists. This file simply does not schedule
it, so a runner working from here never has a reason to open a socket that
writes.

Concretely out of scope here: T30 phase 3 (hardware verification of
individual-address programming, download, unload, recovery), and the
limitations that only a real write can close or that only apply to the write
path — `KNOWN_LIMITATIONS.md` §7, §92, §93, §99, §101, §104, §105, §108, §109,
§111, §112, §113, §114, §115, §116. Leave every one of them exactly as
documented. Do not "prepare" them, do not fold a piece of them into another
task, and do not relax the hardware rules below because nothing in this file
needs hardware.

**Hardware rules still bind, because read-only bus work does appear here (the
T17 diagnostics UI, group-monitor regressions):**

- Individual address `1.1.220` is an alarm panel. Never read it, never write
  it, never include it in a scan range.
- `1.1.24`-`1.1.32` are approved for active *reads* only.
- No write of any kind reaches a real device in this run. Not with a
  confirmation prompt, not "just once", not in a test that happens to be
  pointed at the gateway. Simulated transports only.

---

## 1. Operating rules

1. Start every work cycle by reading `docs/IMPLEMENTATION_STATUS.md`,
   `docs/KNOWN_LIMITATIONS.md`, `docs/LIMITATION_TRIAGE.md`,
   `docs/ROADMAP.md`, `docs/GAP_ANALYSIS_ETS.md` and the source of truth for
   the item at hand. Re-measure counts rather than quoting them; several have
   drifted and were only caught by counting again.
2. **No `.ai/` handover bookkeeping.** `.ai/CURRENT_STATE.md` and `.ai/logs/`
   stay suspended by user ruling. Note that Codex still commits to `main` and
   writes the newest entry at the *top* of `.ai/CURRENT_STATE.md` — read it
   before starting anything, but do not maintain it.
3. Work from the highest-risk correctness, data-integrity, compatibility or
   user-visible gap downward. Prefer a small coherent vertical slice over
   broad speculative work.
4. Follow `AGENTS.md` and `CLAUDE.md` exactly. Respect architecture
   boundaries; keep external-format handling lossless and honestly reported.
5. Create an isolated git worktree before implementing anything
   (`/mnt/daten-i/Sourcecode/KNXBench.worktrees/<branch>`). Never work in the
   main checkout. When a session is stopped, check its worktrees for orphaned
   subagent edits before committing there.
6. For every functional change add focused regression coverage. Run the full
   gate set before declaring an item complete, and judge by exit status, never
   by a summary line:
   `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
   `cargo test --workspace --no-fail-fast`, `cargo run -p xtask -- check-layering`,
   `cargo run -p xtask -- check-headers`, `cargo run -p xtask -- check-anchors`,
   `cargo deny check`, plus `npx tsc --noEmit` and `npx vitest run` in
   `apps/knx-web` when any file under it was touched.
   `ABSENT_CEILING` is 162 with zero slack (verified 2026-09-23) — a new headerless file fails the
   gate. Markdown is not counted at all.
7. **Freshness check.** The former `ntfs3` mount once served a stale binary
   from a current fingerprint (`KNOWN_LIMITATIONS.md` §119). The working copy
   is on ext4 since 2026-09-28, so that hazard is lifted, but a green gate is
   still not proof on its own: confirm the log shows the changed crate being
   compiled (a shared target directory can replay a cached result), and settle
   a disputed result with a fresh `CARGO_TARGET_DIR`.
8. Do not silently downgrade a limitation. If an item depends on unavailable
   samples, specifications, credentials, hardware or a user decision, record
   the exact blocker and move to another actionable item.
9. Do not claim KNX certification or full ETS compatibility. Say
   "KNX-compatible".
10. Commit messages in Marvin's gloomy register — accurate facts, resigned
    tone, no exact clone. **No `Co-Authored-By` trailer, ever**, regardless of
    what any session directive says. Author is `github@knxbench.com`.
11. Push normally. Never wait on or gate a merge behind GitHub Actions.
12. A sweep for one known literal is not a sweep. Grep by pattern class (RFC
    1918 ranges, not one remembered address). The repository's history was
    already rewritten twice for this; do not put it back.
13. **Weekly usage pause threshold: 95%** (user amendment 2026-09-24,
    replacing the earlier 80% threshold). Check usage read-only before starting a task; at or above 95%,
    finish the running task safely and pause before starting another.
    Do not consume reset credits merely to check usage. All hardware and
    verification constraints remain unchanged.
14. **Review model policy (user amendment 2026-09-23):** ordinary task,
    branch, pre-merge and follow-up reviews use `gpt-5.6-sol` with `medium`
    or `high` effort, chosen by risk. Reserve the strongest available model
    exclusively for the very last whole-goal review after all goal tasks have
    been achieved and all other completion prerequisites pass. This supersedes
    the earlier Opus branch rule and temporary Astra exception; historical
    review evidence remains valid. Pin and verify the actual model and effort,
    not merely the model name in a prompt. Review scope and quality gates stay.
15. **Superseded on 2026-09-27.** The user handed the goal back to Hermes;
    see §12 for what to do next. The old text is kept for history.
    *Execution boundary (user amendment 2026-09-23):* finish the
    already-open T13 task, including review fixes, Sol re-review, verification
    and merge. Then stop the goal before T14 or any new task. This limited
    resumption does not authorize the final whole-goal review.
    After the second correction/re-review cycle, the user explicitly authorized
    another targeted restart-lifetime correction round on 2026-09-23; the
    same verification, merge and subsequent pause boundary still applies.
16. **Refresh project statistics after every completed task** (user amendment
    2026-09-24). Once a task has passed review and verification and is integrated
    into `main`, but before starting the next task, run
    `python /mnt/daten-i/Sourcecode/ai-stats.py` from the root checkout
    `/mnt/daten-i/Sourcecode/KNXBench`. Verify that it exits successfully and
    updates the tracked `stats.md`; include that refresh in the completed task's
    bookkeeping commit or in a dedicated immediately-following stats commit.
    Never run it from an isolated feature worktree, because the Git statistics
    must describe the integrated project rather than a temporary branch.

---

## 2. Priority 1 — data integrity, safety and correctness

### 2.1 §22 — the web/Docker target has no authentication at all

`docs/LIMITATION_TRIAGE.md` ranks this K1: no login, no session, no
authorization. Whoever reaches the port owns the project, including its
bus-facing surfaces. This is the single highest-risk non-commissioning item in
the repository.

Decide and document the deployment stance first (an ADR, since it touches
`apps/knx-server`'s public surface and the Docker target's whole premise):
either the server gains real authentication and authorization, or the
container is confined to a loopback/trusted-network deployment that the
product refuses to start outside of. A README paragraph is not a stance.
Whatever is chosen, the bus-facing routes and project mutation must not be
reachable by an unauthenticated caller on a LAN.

### 2.2 §117 — `read_on_init_flag` is parsed, stored, then dropped

The sixth communication-object flag exists in the product database and in the
import path, and does not exist in the domain model, so it is lost at the
`knx-core` boundary. That is a data-integrity gap of the kind CLAUDE.md names
explicitly. Carry it into the model, the projection and the flag UI alongside
the other five, or — if there is a real reason it cannot be a peer of the
others — record that reason with evidence instead.

### 2.3 §34 — schema-≥21 export drops known, unmapped attributes — WITHDRAWN

**Withdrawn by the user on 2026-09-20: export back to `.knxproj` is out of
scope for good.** Import stays; once a project is imported, it lives in
KNXBench's own format and never goes back. The export side of §34 therefore has
nothing to protect, and the retained-attribute machinery built for it no longer
has a consumer. See §6's ruling. The import-side work this task also carried —
schema-≥21 communication-object flags reaching the model instead of being
parsed and dropped — stands on its own and is kept.

### 2.4 §12 — three remaining gaps in manufacturer-data resolution

Product → application-program resolution still has three documented holes.
Read the entry, re-measure against the installed corpus, and close what the
corpus can prove. Do not synthesize a sample to close the rest.

### 2.5 §85 — `.signature` is stored and never checked

The file is kept and no code ever reads it. Either verify it where
verification is defined, or state in the entry that verification is impossible
without the KNX Association's own key material — with the evidence for that
claim, not an assumption.

### 2.6 §2 — import is tolerant rather than validating

No public XSD exists, so the parser accepts and later surprises. Add an
explicit validation stage between parse and normalization, in the data-flow
position `CLAUDE.md` already prescribes, reporting structural violations as
import diagnostics instead of letting them surface as odd behaviour three
layers later. Scope it to what the corpus can actually attest.

### 2.7 §61 — the DPT codec infers its input format, several encodings are rulings

K1, and it is a bus-facing correctness risk: a wrongly encoded value looks
valid on the wire. The 200-series main types are explicitly out (see §6). What
is in scope is the guessing and the rulings for main types 1-30: make the
input format explicit at the call sites, and cite the Standard for every
encoding currently justified by a project ruling — or record the ruling as a
ruling, visibly, at the API boundary.

### 2.8 Product-database corpus follow-up — measured Gira/MDT import gaps

The local ignored corpora under `OriginalData/ProductDatabases/Gira` and `MDT`
now provide direct evidence for 115 modern package instances (113 unique
hashes) across product master schemes 11, 12, 13, 14, 20 and 21, plus one
encrypted legacy MDT `.pr5`. The reproducible inventory and exact boundaries
are in `docs/PRODUCT_DATABASE_CORPUS.md`; use
`tools/inspect_product_corpus.py` instead of re-reading millions of XML nodes
into an agent context.

The real standalone installer accepted 93 inputs. Eleven otherwise-modern
packages failed at the ZIP-name safety boundary, eleven at the namespace gate,
and the `.pr5` belongs to the separate legacy format. Accepted packages still
reported 19,291 unknown constructs. A successful install is therefore not a
losslessness claim. Work through the following slices in order unless a focused
test proves a later slice is the prerequisite:

1. **PDB-1 — safe legacy ZIP member names.** Support the observed legacy/
   CP437-style names only through an explicit decoding policy. Decode before
   normalization, then retain traversal, absolute-path, NUL, size and
   normalized-name-collision rejection. RED tests need UTF-8, legacy umlauts,
   malicious paths and post-normalization collisions; the eleven gated corpus
   packages are regressions, not fixtures to commit.
2. **PDB-2 — a durable corpus compatibility matrix.** Replace the temporary
   probe with an ignored integration test/tool requiring an explicit corpus
   environment variable. Install every package both in isolation and in
   deterministic shared order. Emit machine-readable package hash, scheme,
   outcome, report counts and final DB counts. Absence of `OriginalData` must
   be a visible skip, never a false pass.
3. **PDB-3 — honest, countable install reports.** Separate `read`, `stored`,
   `deduplicated`, `retained-but-uninterpreted`, `unsupported` and `dropped`.
   Report product/program/parameter/com-object/dynamic/module/baggage counts and
   unsupported master sections. Do not reduce the 19,291 unknowns with a broad
   suppression list; classify them by capability and preserve source paths.
4. **PDB-4 — scheme 13.** It has the smallest observed grammar delta. Add a
   synthetic frozen fixture plus a gated real-package regression, atomic
   failure tests and explicit loss accounting before widening the namespace
   gate.
5. **PDB-5 — schemes 12 and 14.** Cover the observed
   `LdCtrlWriteProp/@AppliesTo`, `Property/@Occurrence`, separator metadata and
   scheme-14 `LdCtrlDeclarePropDesc`. Model or explicitly retain/report every
   new semantic field; namespace acceptance alone does not complete the task.
6. **PDB-6 — scheme 21.** Cover `LdCtrlDeclarePropDesc`, variable/null-
   terminated data, optional resources, access policies, RF/coupler
   capabilities and `ApplicationProgram/@HardwareType`. Keep product-scheme
   support separate from `.knxproj` project-schema compatibility.
7. **PDB-7 — secure and version metadata.** Persist and query
   `IsSecureEnabled`, the observed `MaxSecurity*`, tunnelling/user capacities,
   `MinEtsVersion` and `ReplacesVersions`. This is catalogue metadata only; it
   must not imply KNX Data Secure commissioning or runtime support.
8. **PDB-8 — master-data coverage without silence.** For interface-object/
   property, property-data-type, medium, mask, functional-block, datapoint-role,
   resource/access and public-key sections, either add typed storage needed by
   a proven feature or emit section-level retained/unsupported diagnostics.
   Whole-source blob retention is necessary but not a substitute for reporting.
9. **PDB-9 — parameter and Dynamic fidelity.** Add typed/raw coverage for all
   observed parameter kinds (`Restriction`, `Number`, `Picture`, `Float`,
   `Text`, `Color`, `RawData`, `None`, `IPAddress`, `Time`) and synthetic tests
   for Rows/Columns, rename/button nodes, repeat/module nesting,
   transformations and allocator arguments. Unknown Dynamic containers must
   not make their descendants disappear; evaluation semantics and UI layout
   stay separate.
10. **PDB-10 — safe baggage inventory.** Model `Baggages.xml` references and
    report hash, declared/expanded size, media classification, nesting and
    encryption. Images, PDF, MSI, extensionless files and nested ZIPs remain
    opaque and are never executed or blindly extracted. Include bounded-memory
    coverage for the observed 24.2/54.8 MB XML members.
11. **PDB-11 — package identity and versions.** Distinguish byte-identical
    packages, same logical ID with different bytes, product families,
    `ReplacesVersions`, same order number with a different program/scheme and
    deterministic winner/loser sources. Content hashes, not filenames, own
    exact deduplication.

For every slice update `docs/PRODUCT_DATABASE_CORPUS.md`, compatibility and
limitations with what was actually verified. Do not claim ETS-version support
from filenames: the package namespace, `CreatedBy`, `ToolVersion` and
`MinEtsVersion` are separate facts.

---

## 3. Priority 2 — user-visible gaps in the application

### 3.1 T17's missing UI — bus and line diagnostics (gap D6)

The line scan shipped backend-first on 2026-09-13: `ScanPlan`/`ScanPlanBuilder`
in `knx-core`, `probe_address`/`scan_line` in `knx-net`, and the
`knx bus scan` CLI. **There is no frontend for any of it**, and D6 ("no
bus/line diagnostics UI") is the last wholly-unbuilt UI gap in the
gap analysis.

Build it against the existing server API, with the scan's six `ProbeOutcome`
variants shown as six distinct outcomes — never folded into
occupied/vacant. The exclusion list is part of the UI, not a hidden default:
the forbidden address must be visible, pre-filled and impossible to remove by
accident. Show the real cost before starting (a full line is tens of minutes,
`KNOWN_LIMITATIONS.md` §72) and make cancellation work. Everything stays
read-only.

### 3.2 E2 residue — a scan result reconciled back into the project

A scan finds what is really on the line; nothing carries that back into the
project as a diff the user can act on. This was explicitly out of scope for
T17 and is the natural second half. Read-only against the bus, a normal
undoable `Command` against the project.

**Closed 2026-09-21 (T09).** Completed scan evidence is compared with the
current project in three explicit groups. Nothing is selected by default;
excluded and scanner-self addresses stay non-actionable. Only chosen
unexpected/missing findings enter one undoable batch, and no product or
application data is inferred from a bus response. Placement resolves matching
lines across all installations; a removal is refused while dependent project
data still references the device. Project-tree changes refresh the comparison
and clear any stale UI selection.

### 3.3 D8 — settings beyond theme, motion and language

The settings panel covers appearance and language. Everything else ETS-shaped
(defaults for new entities, group-address style handling, bus/gateway
preferences, paths) has no home. Design the settings surface once, then move
the scattered state into it; do not grow a second parallel mechanism beside
`SettingsPanel.tsx`.

### 3.4 B10 — no drag and drop anywhere

`CLAUDE.md`'s UI/UX section names drag & drop as a target capability, and the
application has none: every structural move (device → line, device → building
part, group address → communication object) is form-driven only. Pick the two
or three gestures that carry real weight, implement them through the same
validated commands the forms use, and keep a keyboard-equivalent for each.

### 3.5 Small UI residues, each cheap on its own

Fix as a batch or fold into neighbouring work; they keep losing to larger
items, which is why they are listed:

- **§19** — a search hit inside a collapsed tree branch is never revealed.
- **§24** — `FsPicker` has no drag-and-drop and no multi-selection.
- **§30 / §23** — `/api/project/download` has no frontend caller at all, and
  buffers the whole file in memory when it is called.
- **§96** — a browser that misses the import response can only reach the
  project again by reloading.
- **§118** — a *successful* project load is not announced to screen readers
  (failure already is).
- **§103** — "unsaved" is inferred from the undo stack rather than a real
  dirty flag; **§81** — `new_project_impl` checks "can undo" rather than "is
  modified".
- **§91** — a running bus session keeps the group-address style it started
  with.
- **§89** — five documented `Space/@Type` values are coarsened to
  `BuildingPart` on import; the `MapProblem` is reported, the variants are not
  modelled.
- **§120** — nothing tests whether a theme is readable. The five shipped
  palettes were measured by hand; the sixth will not be. ADR-0022's contrast
  invariant wants a gate, not a paragraph.

### 3.6 KNXnet/IP interface discovery is CLI-only

`crates/knx-net/src/discovery.rs` encodes `SEARCH_REQUEST` and decodes
`SEARCH_RESPONSE`, and `knx bus discover` uses it. The application does not:
`apps/knx-server/src/bus_routes.rs` exposes four bus endpoints and none of them
is discovery, so `BusMonitorPanel`'s gateway field is a bare text input that the
user has to fill from memory.

Give the application the discovery the CLI already has. Two halves: a server
endpoint over the existing `client.discover()`, and a UI that runs it once when
the application starts and offers a button to run it again. Found interfaces
become choices for the gateway field rather than something to retype. A machine
with no interface on the network, or one where multicast does not leave the
container, must degrade to today's manual entry without an error wall — that is
the common case on a developer's laptop, not an exception.

### 3.7 Group-address notation is not selectable

`GroupAddressStyle` (`crates/knx-core/src/address.rs:84`) chooses how many levels
a group address has — Free, TwoLevel, ThreeLevel — and `GroupAddress::format`
(`:168`) always joins them with `/`. Some installations, and some people, write
the same address as `1.1.1`.

Make the separator selectable — `1/1/1` or `1.1.1` — and have the choice apply
to every group address the UI shows: tables, tree, inspector, bus monitor,
search, dialogs. It is a display preference, not project data: what is persisted,
imported and exported stays canonical, so the choice can never change a file's
contents. Address input accepts both notations whatever is selected.

One thing to get right rather than discover later: `1.1.1` is also how an
individual address is written. Dotted group addresses are visually identical to
physical ones, so the UI has to keep the two distinguishable by something other
than punctuation.

---

## 4. Priority 3 — reporting, diff and CSV residue

None of these is a correctness risk; together they are most of what separates
the application from a tool someone would use daily.

- **Documentation export** (`knx-report`): §45 no native PDF, §46 manufacturer
  /product/program names unresolved, §47 parameter values and module
  arguments missing, §48 single-language only, §49 no print preview, §50 no
  section selection. §44 (ETS report parity) is explicitly *not* a target —
  parity is not measurable without an ETS sample.
- **Project diff** (`knx-diff`): §59 shows which fields changed but usually
  not the values, §60's web panel shows grouped counters only, §55 a diff
  cannot be applied back, §56 no three-way compare, §57 no comparison against
  a raw `.knxproj`, §58 no CI-usable non-zero exit. §52/§53/§54 are
  correlation gaps (devices with neither address nor `ets_id`, two same-named
  sibling building parts, regenerated `RefId`s after a re-import). §51 parity
  with ETS's compare is not a target, same reason as §44.
- **Group-address CSV**: §39 (no re-addressing, no deletion, no ranges), §40
  (export-only columns never applied on import), §41 (German-locale Excel
  surprises). §38's ETS interoperability stays an untested assumption and is
  not a task.

### 3.8 Settings live in `localStorage` with no version and no migration

Eight preferences — theme, accent, density, motion level, motion style, UI
language, language packs, product language — are `knx-desktop:` keys in the
browser's `localStorage`, written one key at a time with no schema version
anywhere. Three more keys in the same namespace (`project-context`,
`bus-session-context`, `context-changed`) are session state, not settings, and
should not be confused with them.

Give settings a real home: one versioned file in the server's `data_dir`
(`apps/knx-server/src/paths.rs` already owns that directory and its containment
rule), JSON, with a schema version and a migration chain the way project
storage has one. A newer application version must be able to read an older
settings file and say what it changed; an older application must refuse a newer
file rather than silently discarding what it does not understand.


### Export to `.knxproj` — withdrawn 2026-09-20

The user's ruling: *"drop export zu ets. das brauchen wir nicht. einmal
importiert bleibt es beim KNXBench file format."*

KNXBench reads `.knxproj` and never writes it. The direction of travel is
one-way by design: a project is imported once, and from then on KNXBench's own
storage is the only format that matters. This retires, in one stroke, the whole
class of problems that came from pretending a round trip was a goal — retained
attributes keyed so they land back on the right element, export warnings for
what could not be reconstructed, `CreatedBy`/`ToolVersion` residue, and
ADR-0015's untested question of whether ETS would accept what we wrote.

Import keeps every obligation it had. Nothing about this ruling weakens the
rule that import must not silently discard information: what the opaque store
preserves, it still preserves, and what import cannot map is still reported.
The store's passthrough (ADR-0006) keeps its value as *evidence of what the
source file said* — it simply no longer feeds an exporter.

Out of scope from here: `.knxproj` writing in any schema, ETS re-import
compatibility, and round-trip parity of any kind.

---

## 5. Priority 4 — platform, packaging and the manual

- **D12, the user manual — still explicitly last.** The help half shipped
  2026-09-19 (T28, ADR-0024: `HelpTip`, a ten-topic `F1` panel, prose in the
  message catalogue). ADR-0024 rules `docs/` out as user documentation, so the
  manual is a new document written for users that nobody has written. Start it
  only when the UI-touching items above have settled — the original reason for
  scheduling it last has not changed.
- **Release the alpha.** `0.1.0-alpha.1` is consistent across all 15 Rust
  packages and the web manifest; the x86_64 AppImage was built, inspected and
  launched (ADR-0021). There is no git tag and no published release. Decide
  whether to tag, and say plainly what the AppImage does and does not claim
  (one Arch/XWayland host, no Ubuntu CI, no signature, no auto-update, no
  arm64). Never gate this on GitHub Actions.
- **§16 — Tauri v2 hangs on archived GTK3 bindings under Linux.** `cargo deny`
  reports it; the dependency is unmaintained. Record the exposure, watch the
  upstream, and decide whether the desktop shell can move.
- **§79 — discovery needs IP multicast, which Docker's default bridge does not
  carry.** Already resolved by documentation. Verify the documentation is
  still true; do not reopen the design.

---

## 6. Accepted out of scope — do not start, do not fold in

Each of these is a recorded decision, not an oversight. Reopening one costs
the run its credibility.

- **T19 — KNX Secure** (Data Secure, IP Secure, `.knxkeys`). Deferred
  2026-09-11; needs sample key material and a secured installation. §8, §26.
- **T20 — the `Functions` domain concept.** Deferred 2026-09-11 until the new
  KNX specification documentation is available; needs its own ADR first.
- **T22 — multi-user / concurrent editing.** Parked, not a v1.0.0 must-have.
  §63. Note that the diagnostics companion window already depends on this
  boundary (§82).
- **T21's spatial canvas.** ADR-0019: the building model stays topological, no
  entity carries a position in v1.0.0. A later `FloorPlan`/`Placement` layer
  needs its own ADR and store schema 7; neither exists.
- **E4's eighteen 200-series LTE/system DPT main types.** Accepted out of
  scope 2026-09-20: nothing in `knx-net`/`knx-core` speaks LTE addressing, so
  the codecs would be decoration. Main types 1-30 are covered with no gaps.
  §90 also stands: DPT main type 46 never existed, it was a count.
- **§68/§69/§71 — module handling.** Accepted as documented boundaries
  2026-09-20. §69 is not fixable at all (a synthesized `Module/@Id` would be
  an invention presented as data).
- **§13's AES half** — blocked on a real ETS6 AES-protected sample, not on a
  decision. ZipCrypto already works. A synthesized sample proves nothing.
- **§1 / project-schema evidence** — `.knxproj` schemas 12-19 and 22 still rest
  on no project evidence; product-package schemes with the same numbers are a
  different format boundary. Do not use the new `.knxprod` samples to claim
  `.knxproj` compatibility.
- **`.vd2` and unobserved `.knxprod` schemes 15-19/22** remain out of scope.
  The user reopened the now-evidenced standalone product schemes 12/13/14/21
  on 2026-09-23 through §2.8; no further sample-hunting is needed for those.
- **ETS re-import of KNXBench-written projects** — dropped as a goal by
  ADR-0015; §5's unsigned-export exposure follows from it.
- **§6 — devices behind manufacturer plug-in DLLs.** No verified semantics
  exist across that boundary; do not invent them.
- **A plugin API** — ADR-0025: extension stays data-shaped (language packs,
  product databases, CSV, the headless CLI). §107. Its tripwires are
  greppable; if one fires, that is a signal to revisit, not to build.
- **Online device-catalog update (C6) and an ETS-App-style ecosystem (F4)** —
  no well-formed task exists for either.
- **A mobile app and non-Linux desktop support** — new-platform work, premature
  while the Linux-first desktop is unfinished.
- **The project logo** — the user is handling it. Do not start it and do not
  fold it into packaging.

---

## 7. Research before design — no implementation

Each of these needs a written research or decision artifact *first*. Producing
that artifact is a legitimate deliverable; producing code is not.

- **LLM / natural-language interaction and MCP capability** (`ROADMAP.md:585`,
  `ideas.md`). Two halves of one prerequisite: a mature, near-complete
  `Command` layer. First deliverable is a `docs/RESEARCH.md` section covering
  capability scope, authorization against a live project, how natural language
  maps onto `Command`, which model and whether local or remote, and what it
  must never be allowed to do unsupervised to project data. Design spec after
  that, never before.
- **Automation of repetitive tasks / a macro layer.** Same foundation, same
  order.
- **In-app project notes and documentation** (`ideas.md`). A new domain
  concept absent from `docs/DATA_MODEL.md`; needs its own ADR before any
  implementation. It is *not* T28's help and *not* `knx-report`'s export.
- **"Who talks to whom"** — group addresses animated to the devices they
  reach, with the reason visible. Needs the mature UI base it now has, plus
  live telegrams from the bus monitor to be worth more than a static group-link
  diagram. Deferred, not designed, not started.
- **Legacy VD/PR product import.** The supplied encrypted MDT `.pr5` is a
  legacy `ets.pr_`/EX-IM container, not a malformed `.knxprod`. Reconcile it
  with `docs/VD4_PRODUCT_DATABASE_IMPORT.md` and first produce an independent
  format/security/legal design: user-supplied lawful input, no embedded
  password, bounded decryption/parsing, synthetic fixtures, atomic publication
  and explicit mapping-loss reports. Only that reviewed artifact may authorize
  implementation; do not route PR/VD bytes through the modern XML-package
  parser.

---

## 8. Documentation hygiene and parked findings

Small, real, and each one currently misleads a reader:

1. **`docs/ROADMAP.md`'s T37 section still reads as open.** T37 shipped
   2026-09-19 (ADR-0023, branch `t37-load-progress`,
   `IMPLEMENTATION_STATUS.md:6337`), and its residues are §97 (phase labels
   rather than percentages, by design) and §118. Verify against the code, then
   mark it shipped the way T38's section is.
2. **`codex-goal.md` is stale.** Its last six execution checkboxes are
   unticked, but the redesign shipped: `Workbench`, `StructureWorkspace`,
   `DeviceWorkspace`, `DiagnosticsCompanion`, `PaneSplitter`/`ResizablePane`,
   five themes, two motion styles. Verify the coverage matrix it demands
   against the current `apps/knx-web/src`, then either tick what is genuinely done or
   retire the file — it currently reads as a second, contradicting backlog.
3. **`ideas.md` still lists shipped work as pending.** Animations, themes, the
   status dashboard, device discovery and the humour templates (30+ per part)
   all shipped. Mark them; keep MCP, automation, "who talks to whom", project
   notes, mobile and multi-OS as the genuinely open entries.
4. **Parked finding F-T30-1** (confirmed 2026-09-20). ADR-0039 has been
   written, approved (see §12) and merged, but no
   implementation exists. The finding itself:
   `Project`'s six fields are all `pub`
   (`crates/knx-core/src/project.rs:181-186`), so "every mutation goes through
   `Command::apply`" is an invariant held by review, not by the type system.
   Belongs to whoever next touches `knx-core`'s public surface.
5. **`docs/LIMITATION_TRIAGE.md` must be re-counted, not edited by hand,**
   whenever `KNOWN_LIMITATIONS.md` gains an entry. It drifted three times
   before. 119 entries / 118 classified as of 2026-09-20.

---

## 9. Parallel, subagent-driven delivery

Use Subagent-Driven Development for planned work. Decompose into small,
testable items; give each agent a narrow brief, an isolated worktree when it
will edit files, explicit acceptance criteria, and a report path. Keep a
durable ledger (`.superpowers/sdd/<date>-<name>/progress.md`, git-ignored) so
completed work is never redispatched after a context compaction, ending in a
`RESUME HERE` block.

Subagents in this environment **cannot reliably write report files** — ask for
findings as returned text and persist them yourself.

Every dispatch carries a task counter in both its description and the first
line of its prompt: `Task x von y` and `Txx, rest N offen`. Status lines and
ledger headings carry a real timestamp read from `date` in the same call that
writes them.

At most **two** subagents active alongside the coordinator. Queue the third;
never kill a running one.

Name the model and reasoning effort explicitly on every dispatch; never inherit
the coordinator's default. Cheapest tier that carries the item's risk:

| Work type | Model | Effort |
| --- | --- | --- |
| Mechanical, fully specified 1-2-file edit; focused test | `claude-haiku-4-5` | low |
| Multi-file implementation, integration, ordinary debugging | `claude-sonnet-5` | medium |
| Architecture/domain decisions, data-integrity or bus-facing work, UI and design work, difficult debugging | `claude-opus-5` | high |
| Task, branch, pre-merge and follow-up reviews | `gpt-5.6-sol` | medium or high by risk |

The implementation tiers above do not override the review policy. For ordinary
reviews use medium effort for bounded routine changes and high effort for
cross-layer, concurrency, data-integrity or other high-risk changes. A failed
review/fix round may increase Sol effort from medium to high, but never consumes
the strongest model reserved for the final whole-goal review. Implementation
escalation and design/UI implementation tiers otherwise remain unchanged.

Two distinct review scopes:

- **Per branch, before merging: a whole-branch review on `gpt-5.6-sol` with
  medium or high effort.** Every branch still requires independent review;
  follow-up reviews use the same model policy. Fix and verify findings before
  integration. A branch's closing review is not the final whole-goal review.
- **Only at the very end: one review over the finished whole goal using the
  strongest available model.** Select and verify that model when this final
  stage is reached, not by inheriting the coordinator's default. Dispatch only
  after all goal tasks are achieved (or explicitly accepted out of scope), all
  branches have been reviewed, and every other completion prerequisite passes.
  No individual task or branch review uses this reserved strongest-model slot.
  Findings prevent completion; verification/follow-up uses Sol medium/high.

Before integration, the coordinator reviews each subagent's diff, test
evidence, documentation and report. Do not take a subagent's summary at face
value — verify the claim, especially a green one.

---

## 10. Completion condition

Finish only when:

- `docs/IMPLEMENTATION_STATUS.md`, `docs/ROADMAP.md`,
  `docs/GAP_ANALYSIS_ETS.md`, `docs/KNOWN_LIMITATIONS.md` and
  `docs/LIMITATION_TRIAGE.md` are reconciled with each other and with the
  code;
- every actionable item in sections 2-5 and 8 has proof of completion, or a
  recorded, non-actionable external blocker;
- all nine gates pass, with the freshness check from rule 7 done;
- every remaining exception carries the user's explicit out-of-scope
  acceptance;
- and the final strongest-model review has run over the finished whole and its findings
  are resolved. It is the last thing that happens, not a formality on the way
  out — if it opens something, the goal is not done.

Report the completed work, the verification evidence, the remaining external
blockers, and the next required user decision, if any. Commissioning stays
where section 0 left it: excluded from this run, still owed, waiting on test
hardware.
## 11. User-reported UX and workflow issues — amendment 2026-09-21

The observations formerly collected in `docs/Issues.md` are now normalized into
thirteen independently testable tasks in
`docs/superpowers/plans/2026-09-21-user-reported-issues.md`. They are part of
this goal's backlog. Where that plan names overlap with an existing item
(dirty state, settings, drag/drop, discovery), extend the existing owner and
deliver one coherent implementation rather than creating a competing path.

The issue plan also records which reports describe already-present behavior:
bus-monitor text/service filters, KNXnet/IP discovery, and separate Send and
Receive group links exist today. Those tasks reproduce reachability or
packaging failures and add the missing behavior; they do not reimplement the
existing core. Evidence-gated KNX/domain questions remain investigation-first.

---

## 12. Status after the Paperclip handback (2026-09-27)

Sources for this section:

- The status map from the Paperclip DIN-3 plan, which Steve Smith checked against git and the docs on 2026-09-26 at `main` `7b64496`.
- The eight agent handoffs in `docs/paperclip-shutdown/` (uncommitted) and their summary, `docs/paperclip-shutdown/STATUS.md`.
- A git check of every Paperclip worktree on 2026-09-27.

`main` is still at `7b64496`. Paperclip merged nothing and pushed nothing.

"Board" in the Paperclip sources means the user. A Board approval listed below is therefore a user decision. Do not ask for it again.

### 12.1 What was already done before Paperclip (check it, do not redo it)

| goal.md item | State | Evidence |
| --- | --- | --- |
| §2.1 §22 server auth (T01/T01b) | done | `af12fa2`, `0f28c1e`, ADR-0026 |
| §2.2 §117 read_on_init (T02) | done | `1e74075` |
| §2.3 §34 export | withdrawn | ADR-0028, `a619c13` |
| §2.4 §12 manufacturer data (T04, T29) | done; remaining gaps documented | `8856486`, `240792b` |
| §2.5 §85 signature (T05) | done, as an evidenced boundary | `d8fc36a` |
| §2.6 §2 validation (T06) | done | `258feee` |
| §2.7 §61 DPT (T07) | done | `03c1316` |
| §2.8 PDB-1 to PDB-7 | done | `21e22fa`, `d8f58dd`, `ce6f339`, `6503c97`, `b65cdfa`, `badcc33`, `16ee372` |
| §3.1 D6 diagnostics UI (T08) | done | `bc6e53e` |
| §3.2 E2 reconcile (T09) | done | `3bf8ce7` |
| §3.3 D8 settings (T10), §3.8 settings file | done | `7a874b8`, `d31dd75` |
| §3.4 B10 drag & drop (T11) | done (two gestures) | `02237eb` |
| §3.5 UI residue A/B (T12/T13) | done | `ff73ab9`, `7f9c8c4` |
| §3.6 discovery in the app (T25) | done | `62ff969` |
| §3.7 GA notation (T26) | done, as a fixed slash notation | ADR-0030, `fcf4563` |
| §4 report (T14), diff (T15), CSV (T16) | backend done; web residues open (see 12.3) | `cd10132`, `2403c63`, `0e278f0` |
| §5 §16 Tauri/GTK3, §79 docs (T17) | done | KNOWN_LIMITATIONS §16 |
| §7 MCP/NL, macros, notes ADR, who-talks-to-whom | done, as research artifacts | RESEARCH §13, §14, §16; ADR-0031 |

### 12.2 Paperclip branches: take them over first, in this order

All of these worktrees are under `/mnt/daten-i/Sourcecode/.paperclip-worktrees/KNXBench/<DIN-n>`. Each one had a clean working tree at shutdown. None has been pushed. You can use the worktrees as they are, or check the branch out into a normal worktree under `KNXBench.worktrees/`.

In the Paperclip runtime, `git` on `$PATH` was a wrapper that blanked the author identity. Outside Paperclip this does not apply, but check the author of the wip commits before you build on them.

1. **`din-9-legacy-pr-design` @ `72da572` (docs only, +727 lines).** This is the §7 legacy VD/PR `.pr5` design: RESEARCH §18, KNOWN_LIMITATIONS §128 and the spec `docs/superpowers/specs/2026-09-26-legacy-vd-pr-product-import-design.md`.
   - Independent review DIN-34 gave **APPROVE** and reproduced the measurements.
   - The user **accepted B-1 to B-6 as recommended and approved the merge as-is** (approval `57d5c7a1`, 2026-09-26 17:49Z).
   - To do: fast-forward merge, then run `ai-stats.py` (rule 16).
2. **`din-10-command-apply-adr` @ `cc4012c` (docs only).** This is ADR-0039, "Project mutation goes through commands", which answers §8.4 F-T30-1. It adds KNOWN_LIMITATIONS §129.
   - Review DIN-35 gave **APPROVE**.
   - The user **accepted B-1 to B-5 and approved the merge** (approval `ef64a57b`, 2026-09-26 17:27Z).
   - To do: merge it.
   - The branch edits goal.md §8.4. Keep main's text when that conflicts.
   - At merge time, recount LIMITATION_TRIAGE by command. The branch raised K1 from 7 to 8 by hand.
   - Run `ai-stats.py`.
   - Review nit, not a blocker: goal.md cites `project.rs:181-186`, but the fields are now at 184-190.
3. **`din-11-command-apply-phase1` @ `cc4012c`.** Same commit as DIN-10. There is **no implementation yet**.
   - ADR-0039 phases 1 and 2 close §129 (data loss). This is the next real §8.4 work, and it is now approved.
   - The first implementation commit flips ADR-0039 from `Proposed` to `Accepted`.
   - B-2 changes a documented undo guarantee. The phase-2 commit must rewrite the matching sentences in IMPLEMENTATION_STATUS and in GAP_ANALYSIS E2.
   - It touches `knx-core`'s public surface. Serialize it against any PDB slice that also touches `knx-core`.
4. **`din-4-corpus-manufacturer-dirs` @ `cfe80a1` (2 wip commits).** This fixes the gate blocker "Seven corpus tests assume a flat local corpus directory" (KNOWN_LIMITATIONS). It is a **prerequisite for PDB-8 to PDB-11**.
   - What was done:
     - It adds `knx_testsupport::walk_corpus_files` and `find_corpus_file`.
     - It rewires `dynamic_tree.rs`, `parameter_views_corpus.rs` and `standalone_packages.rs` to use them.
     - It removes assertions that pinned counts.
   - Current state: 8 of 9 corpus tests pass against the real corpus.
   - To do:
     - `a_v6_corpus_database_gets_its_linkable_back_from_its_own_blobs` fails with `duplicate column name: is_secure_enabled`. The manual schema rewind in the test is missing the column added in `migration.rs:297`. This was a pre-existing gap that the old path bug had hidden.
     - Run fmt and clippy.
     - Update the KNOWN_LIMITATIONS entry.
     - Audit the three files for any remaining count pins.
     - Get a review.
   - Always run with `KNXBENCH_PRODUCT_CORPUS=<root>/OriginalData/ProductDatabases`. Otherwise the tests take the skip path and pass for the wrong reason. `corpus_nested_module_measurement_task_11` takes about 400 s, so run it in the background.
5. **`din-16-site-hierarchy-decision` @ `93348bc`.** This is ADR-0038 for §11 ISSUE-06. A site or property is a `Space`/`BuildingPartType::Ground` root, with no new `Site` kind. It adds characterization tests and changes no production code or schema. Its status is `Proposed`.
   - **No review yet.** Paperclip review DIN-33 never started, because of provider errors.
   - To do: an independent review (the scope is in the old DIN-33: spec evidence Project Schema23 §1.1.2.3/§1.2.6.3, 3/10/3 §1.2.3.5, 3/10/4 Table 10, 3/10/2 Table 1; the tests; KNOWN_LIMITATIONS §127; the gates). Then ask the user for approval and merge.
   - It blocks ISSUE-05.
6. **`din-12-dirty-state-autosave` @ `0db4841` (1 wip commit).** §11 ISSUE-04, together with the §3.5 residues §103 and §81.
   - The commit contains `useAutosave.ts`, `autosaveSettings.ts`, SettingsPanel wiring, the dirty-state signal in `App.tsx`, server-side dirty state in `apps/knx-server/src/domain.rs`, and tests.
   - **The tests were not run after the wip commit.** Save-and-continue and the last-save indicator are not finished.
   - No architecture review has been done against AGENTS.md (the UI must not own domain logic). Verify everything before you build on it.
7. **`din-26-board-oos-acceptance` @ `70b683e`.** This is a draft of an out-of-scope decision (`docs/superpowers/plans/2026-09-26-din26-oos-board-decision.md`), and **there is no verdict yet**. It covers:
   - §45 native PDF
   - §48 the full report prose catalogue
   - §52/§53/§54 diff correlation
   - §55 applying a diff
   - §56 three-way compare
   - §39 CSV ranges and renaming
   - §41 spreadsheet transforms
   - §12 remaining manufacturer gaps
   - §85 signature verification
   - §2 XSD

   To do: put it to the user for a decision. Whatever is accepted goes into §6.
8. **`din-3-goal-migration` @ `90add0c`: do not merge it** (see the header). You may delete the branch and its worktree.

### 12.3 Still open (not started)

- **§2.8 PDB-8 to PDB-11:** done. PDB-8 `abf35d3`, PDB-9 `3643e90`, PDB-10 `15b4c56`, PDB-11 `7844590` (ADR-0043, schema v17), all merged and pushed.
- **§4 web residues:** done.
  - §49/§50 lifted by CT-2 (`d9ff0db`).
  - §59 lifted and §60 narrowed by CT-1 (`313489e`).
  - §57 lifted by CT-6 (`826466a`).

  All three came through the cloud chain.
- **§11 UX/UI issues: moved to [`goal-ui.md`](goal-ui.md) on 2026-09-28 (user decision).** A separate UI session (GPT/Codex) owns ISSUE-01, 02, 03, 05, 07, 09, 10, 11, 12 and 13, the UI half of ISSUE-08, the ADR-0038 review (ISSUE-06) and the File-menu rename. Its boundary table is `goal-ui.md` §5; the web lock is `goal-ui.md` §3.
  - The cloud track stopped on 2026-09-28. Its web queue (CT-7 to CT-10) is now `goal-ui.md` U4 to U7.
- **§11 ISSUE-08 data half (stays here):** reproduce each symptom against installed product data, trace it source XML → database → enrichment → projection, count and classify the diagnostics, add language-aware object/DPT names (keeping canonical DPT ids), and carry evaluated active/visible state and evidenced channel ownership into the projection, with corpus regression counts. See the issue plan's ISSUE-08 checkboxes 1–4 and 6.
  - It does not edit `apps/knx-web`. When it is merged, hand the UI session the new projection fields and the merge commit ("For the UI session:"); its U12 waits for that.
- **§8.1 to §8.3 and §8.5, doc hygiene:**
  - ROADMAP T37 still reads as open.
  - `ideas.md` still lists shipped work as open.
  - `codex-goal.md` has already been removed from `main`; verify that.
  - LIMITATION_TRIAGE is at 119 while KNOWN_LIMITATIONS has 125+ entries. Recount by command after the DIN-9 and DIN-10 merges (§128, §129).
- **§5 D12, the user manual (T23):** not accepted. The open points are the location per ADR-0024, removing the screenshots, and a claim-by-claim verification report.
- **§5 alpha release (T18):** a user decision; do not tag.
- **§10 final whole-goal review:** last of all.

Doc reconciliation, then the manual, then the alpha decision, then the final review come last, in that order. The manual and everything after it wait for the end of the UI track (`goal-ui.md` U13), because the manual describes the finished UI. §10's completion condition includes the UI track's result.

### 12.4 Parallel tracks outside this goal

- Parallel track `iaw-settling-delay` (commissioning 1.1.67, worktree `KNXBench.worktrees/iaw-settling-delay`): do not merge, rebase, clean up or `worktree prune` it. It merges into `main` itself.
  - Done 2026-09-28: the commissioning session merged it itself (`95a862c`) and removed branch and worktree. Its scratch `scratch/iaw/` still belongs to that session.
- **Commissioning track, from 2026-09-28: [`goal-commission.md`](goal-commission.md)** (T30 phase 3, K1–K10).
  - Its worktrees `KNXBench.worktrees/iaw-*`, branches `iaw-*` and `scratch/iaw/` belong to that session. Do not merge, rebase, clean up or `worktree prune` them; it merges into `main` itself.
  - Do not start a workspace gate while its gate is running (`pgrep -af cargo`).
  - Before K5 it announces its web part in its handover. Do not start a web task of the §12.3 chain in parallel.
- **UI track, from 2026-09-28: [`goal-ui.md`](goal-ui.md)** (UX/UI issues, U0–U13), run by a GPT/Codex session.
  - Its worktrees `KNXBench.worktrees/ui-*`, branches `ui-*` and `scratch/ui/` belong to that session. It merges into `main` itself.
  - This session no longer edits `apps/knx-web`. If it ever must, it takes the web lock (`goal-ui.md` §3) first.
  - Do not start a workspace gate while another session's gate runs (`pgrep -af cargo`).
  - Items it hands over arrive under "For the goal.md session:" and are adopted into this section.
- **Received from the commissioning session** (its handover entries under "For the goal.md session:"; this session adopts them here and confirms in its next handover):
  - 2026-09-28, from merge `95a862c`: `stats.md` predated that merge (last refresh `cf791b0`). **Done:** refreshed in the commit that adds `goal-commission.md`.
  - 2026-09-28 (K1/K2): `docs/manual/known-issues.md`, `docs/manual/implementation-status.md` and `docs/manual/reference/02-supported-and-unsupported.md` still say KNXBench never wrote to hardware. That is false since 2026-09-26. Fix it with the §12.3 manual work (T23). KNOWN_LIMITATIONS §92 has a new title; the triage recount must pick it up.
  - 2026-09-28 (K2): the File-menu rename "Download project" → save/export (R2, `docs/GLOSSARY.md`) is web work. **Passed on to `goal-ui.md` (U3).**
  - 2026-09-28: KNOWN_LIMITATIONS gained §134–§136 (and now counts 139 `##` headings, versus the triage's 119). Add them to the LIMITATION_TRIAGE recount in §12.3 (§8.5). §136 is commissioning-owned: triage classifies it, and only `goal-commission.md` changes its text.

### 12.5 Lessons from the Paperclip run

- Provider errors (HTTP 400 thinking block, HTTP 429) ate every review run on 2026-09-26. An automatic error comment is not a review verdict.
- Every Paperclip result above that says "not run" or "wip" is unverified. Rule 6 applies without exception: gates by exit status, plus the freshness check from rule 7.
