# KNXBench goal — everything still open, minus commissioning

Written 2026-09-20. Use this file as the instruction passed to `/goal`. Its
creation alone starts no run.

**Execution boundary (2026-09-23):** User requested completion of the already
open T13 through verified merge, then a pause. T14 and every subsequent task
remain unstarted; the read-only T14 survey is not implementation approval.
Do not resume the standing goal without a new explicit user instruction.
T13 is now merged as `7f9c8c4`; all eleven merged-main gates passed. The standing
Hermes goal remains paused, not completed.

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
7. **This `ntfs3` mount has served a stale binary from a current fingerprint
   once** (`KNOWN_LIMITATIONS.md` §119). A green gate alone is not proof:
   sanity-check that the `knx-net` lib test count matches the source you just
   edited, and `cargo clean -p <crate>` when it does not.
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
13. **Weekly usage pause threshold: 95%** (user amendment 2026-09-23,
    replacing 60%). Check usage read-only before starting a task; at or above
    95%, finish the running task safely and pause before starting another.
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
15. **Current execution boundary (user amendment 2026-09-23):** finish the
    already-open T13 task, including review fixes, Sol re-review, verification
    and merge. Then stop the goal before T14 or any new task. This limited
    resumption does not authorize the final whole-goal review.
    After the second correction/re-review cycle, the user explicitly authorized
    another targeted restart-lifetime correction round on 2026-09-23; the
    same verification, merge and subsequent pause boundary still applies.

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
- **§1 / schema evidence** — schemas 12-19 and 22 rest on no evidence; only
  actionable when a sample appears. Do not synthesize one.
- **`.vd2` and `.knxprod` schemes 12-19/21/22** — out of scope by user
  decision 2026-09-11, no further sample-hunting.
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
4. **Parked finding F-T30-1** (confirmed 2026-09-20, not yet owned):
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
- all nine gates pass, with the `ntfs3` freshness check from rule 7 done;
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
