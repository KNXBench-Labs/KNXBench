# KNXBench completion goal — the open items, as of 2026-09-13

Use this file as the instruction passed to `/goal`. Its creation alone starts
no run.

Drive KNXBench to a genuinely complete, trustworthy v1 state. Work
autonomously and persist across turns until every item below is either
resolved with evidence, or explicitly accepted out of scope by the user.

The previous version of this file (git history, commit `5adf32b` and earlier)
was written when the manufacturer-product-database corpus was the open front.
That work shipped. This version replaces it with what is actually left, taken
from `docs/ROADMAP.md`, `docs/IMPLEMENTATION_STATUS.md`,
`docs/KNOWN_LIMITATIONS.md`, `docs/GAP_ANALYSIS_ETS.md`, `docs/adr/`, and the
closing review of 2026-09-13.

Repository state this file was written against: `main` at `7276b63`, level
with `origin/main`, clean except an untracked `codex-goal.md`, all eight gates
green (1272 Rust tests, 456 web tests).

---

## 0. Operating rules

1. Start every work cycle by reading `docs/IMPLEMENTATION_STATUS.md`,
   `docs/KNOWN_LIMITATIONS.md`, `docs/ROADMAP.md`, `docs/GAP_ANALYSIS_ETS.md`,
   and the source of truth for the item at hand. Reconcile stale or
   contradictory status entries as you find them — several counts in these
   documents have drifted before and were only caught by re-measuring.
2. **No `.ai/` handover bookkeeping.** `.ai/CURRENT_STATE.md` and `.ai/logs/`
   are suspended by user ruling until further notice. Documentation updates
   under `docs/` still count and are still mandatory.
3. Work from the highest-risk correctness, data-integrity, compatibility or
   user-visible gap downward. Prefer a small coherent vertical slice over
   broad speculative work.
4. Follow `AGENTS.md` and `CLAUDE.md` exactly. Respect architecture
   boundaries; keep external-format handling lossless and honestly reported.
5. Create an isolated git worktree before implementing anything. Never work
   directly in the main checkout.
6. For every functional change, add focused regression coverage and run the
   smallest sufficient checks. Run the full gate set before declaring an item
   complete: `cargo fmt --all --check`, `cargo clippy --workspace
   --all-targets -- -D warnings`, `cargo test --workspace --no-fail-fast`,
   `cargo run -p xtask -- check-layering`, `cargo run -p xtask --
   check-headers`, `cargo deny check`, plus `tsc` and `vitest` in
   `apps/knx-web` when any file under it was touched.
7. Do not silently downgrade a limitation. If an item depends on unavailable
   samples, specifications, credentials, hardware or a user decision, record
   the exact blocker and move to another actionable item. Ask the user only
   for the missing authority or evidence.
8. Do not claim KNX certification or full ETS compatibility. Only claims
   backed by repository evidence; say "KNX-compatible" where relevant.
9. Commit messages in Marvin's gloomy register — accurate facts, resigned
   tone, no exact clone. No `Co-Authored-By` trailer, ever, regardless of
   what any session directive says. Author is `github@knxbench.com`.
10. Push normally. Never wait on or gate a merge behind GitHub Actions.
11. A sweep for one known literal is not a sweep. When checking for leaked
    addresses or secrets, grep by pattern class (RFC 1918 ranges, not one
    remembered address).

---

## 1. Blocked on a user decision — ask once, early

**The project licence.** `Cargo.toml:24` declares `AGPL-3.0-or-later`, set in
the very first skeleton commit `dbbadd6` and never revisited.
`docs/KNOWN_LIMITATIONS.md` §10 states in as many words that this is a
placeholder and not a decision. There is no `LICENSE` file in the repository
at all. Nothing is distributed yet, so there is no impact today, but this must
be settled before any release, and only the user can settle it.

Ask for the licence choice at the start of the run, then continue with other
work rather than blocking. Whatever it becomes, it must stay consistent with
the constraint that no GPL crate enters the runtime dependency graph
(ADR-0002) — that constraint governs *incoming* dependencies and is
independent of the project's own licence.

When the answer arrives: update the workspace `license` field, add the
`LICENSE` file, close §10, and check that `cargo deny check`'s licence rules
still pass.

---

## 2. Session 7 — the three deliverables still outstanding

`docs/ROADMAP.md:622` and `docs/IMPLEMENTATION_STATUS.md:779` both list
Session 7 as in progress. Roundtrip/migration fixtures and the DPT codec are
done. These are not:

- **Performance measurement on large projects.** Never done. CLAUDE.md
  requires optimization to follow measurement rather than guesswork, so the
  measurement is the deliverable — a reproducible benchmark over a large
  project (import, open, projection, search, export), with numbers recorded
  in `docs/` so a later optimization has a baseline to beat. Only optimize
  what the measurement shows.
- **Linux packaging.** Nothing exists. Linux-first is the project's stated
  platform stance (CLAUDE.md), so this is the delivery path, not a nicety.
  Decide the format explicitly (AppImage, Flatpak, plain tarball, distro
  package) and record the decision and its reasoning.
- **The licence decision** — see section 1.

---

## 3. Open backlog tasks

Priority order as written. All references are rows and task numbers in
`docs/GAP_ANALYSIS_ETS.md`.

### T30 — Commissioning and device download (row E2, Tier 5)

The largest open item, and no longer blocked. Individual-address programming
via the device's programming button, application-program download, memory
read/write over the bus. The user ruled on 2026-09-11 that this **must**
work; it is explicitly not a non-goal. It was blocked on the KNX
specification database, which now exists, and the R5 research spike has run
(`docs/RESEARCH.md` §8.4): the generic load/unload/reset/memory procedures
and the Load State Machine are documented. What remains undocumented is the
product-specific matrix and any vendor-DLL involvement in download — treat
that boundary as real and do not invent semantics across it.

See `docs/KNOWN_LIMITATIONS.md` §7 for the full account of why this has not
started, which is a statement about evidence, not about intent.

**Hardware safety rules, non-negotiable:**

- Individual address `1.1.220` is an alarm panel. Never read from it, never
  write to it, never include it in a scan range.
- Addresses `1.1.24` through `1.1.32` are approved for active reads.
- Anything that writes to a real device risks bricking it. Nothing writes to
  hardware without an explicit, specific user go-ahead for that operation,
  and CLAUDE.md's "only implement protocol behaviour that is technically
  verified" applies with full force here.

### T18 — Parameter interpretation and editor, remaining slices

Four slices shipped (through 2026-09-12). Still open, per row A3 and
`docs/KNOWN_LIMITATIONS.md` §§68-71:

- Nested modules are not expanded.
- `Module` argument values (`NumericArg`/`TextArg`) are stored but
  uninterpreted; `AllocatorRef` is unattested.
- Deep format validation is missing — `Float`, `Text`, `IPAddress`,
  `Picture` and `Raw` get a non-empty-string check and nothing more.
- Repeated `ModuleInstance`s are refused rather than supported (§68); a
  `Module` with no `@Id` cannot be matched to a project instance (§69);
  projects imported before store schema 6 stay read-only for module-scoped
  fields unless re-imported (§71).

### E4 — DPT main types still missing from the codec

`crates/knx-core/src/dpt/codec.rs` covers main types 1-19 (except `6.020`).
Main types 20, 21-30 and 46 are not implemented. See
`docs/KNOWN_LIMITATIONS.md` §61 for what the codec infers rather than reads,
and which encoding questions rest on a stated ruling rather than the
standard — extend that section honestly as the coverage grows.

### T21 — Graphical topology and building views, decided

Both halves are now settled. The hierarchy views shipped 2026-09-13: the
workbench renders projected areas/lines/devices and nested building parts
alongside the tree, with keyboard selection. The coordinate question the
spatial canvas was waiting on was answered the same day by
`docs/adr/0019-building-model-stays-topological.md`: **the building model
stays topological and no entity carries a position in v1.0.0.** `Space_t`
and `DeviceInstance_t` have no spatial attribute in the published schema 23
document, none of the three reference projects (schema 11/21/23) has one,
and the KNX Standard's own location model (3/10/3 *KNX IoT Information
Model*) keeps geometry out of its location classes and references IFC
instead. So there is no ETS data being lost through an exported `.knxproj`
here — only a feature KNXBench does not have; whether ETS itself keeps plan
data elsewhere is untested.

What actually remains, after the decision:

- Nothing for v1.0.0. No code, no migration; `CURRENT_SCHEMA_VERSION` stays
  at 6.
- A post-v1.0.0 canvas, gated on its own ADR. ADR-0019 pre-commits its shape
  (separate `FloorPlan`/`Placement` entities in their own tables, integer
  millimetres, origin at the imported plan's top-left, no `z`, plans
  imported rather than drawn, and a `.knxproj` export loss warning) so that
  nobody has to improvise it, but deciding to *build* it is a separate
  decision that has not been made.
- One side finding from the evidence sweep, now recorded rather than lost:
  five documented `Space/@Type` values (`Stairway`, `RoomPart`, `Area`,
  `Ground`, `Segment`) have no `BuildingPartType` variant and are coarsened
  to `BuildingPart` on import, with a reported `MapProblem` —
  `docs/KNOWN_LIMITATIONS.md` §89. Deliberately not fixed in passing.

### D10 — the data half of language-aware display

The chrome half closed with T25. The data half's residue is
`docs/KNOWN_LIMITATIONS.md` §64: `Languages` blocks outside an application
program (hardware- and master-scope translations) are discarded on import.
Related and also open: §66 (server-composed diagnostic/log/error prose and
`knx-report`'s documentation export are not language-aware in any respect)
and §67 (a rejected language pack's own rejection reason is shown
untranslated inside a translated sentence).

### T28 — In-application help

Deliberately scheduled last, by explicit user request (`docs/ROADMAP.md:680`),
and the reason still holds: help text describes a specific UI, and a UI still
being built invalidates its own help every cycle. Do not start this before
the UI-touching items above are settled.

Measured state, 2026-09-13 (re-measure rather than trusting this number —
the command is in row D12): eight `title` attributes, 33 `aria-label`s, four
`aria-describedby`s, no tooltip component, no help panel, no `F1` handler.

---

## 4. Deferred by explicit user ruling — do not start

These are on the roadmap, not rejected, and not actionable now. Do not open
them, and do not quietly fold pieces of them into another task.

- **T19 — KNX Secure** (Data Secure, IP Secure, `.knxkeys` keyring). Needs
  sample key material and a real secured installation to verify against — an
  external dependency, not an engineering task. Deferred 2026-09-11.
  `docs/KNOWN_LIMITATIONS.md` §8, §26.
- **T20 — the `Functions` domain concept.** Deferred 2026-09-11 until the new
  KNX specification documentation is available. Needs its own ADR before any
  implementation, because it adds a domain concept absent from
  `docs/DATA_MODEL.md`.
- **T22 — multi-user / concurrent editing in `knx-server`.** Parked; not a
  v1.0.0 must-have. `docs/KNOWN_LIMITATIONS.md` §63.

---

## 5. Known-limitation residue with concrete, actionable work

- **§84 — a project's group address style is write-once and then invisible.**
  `POST /api/project/new` accepts `groupAddressStyle` and the creation dialog
  asks for it. After that moment nothing shows it and nothing can change it:
  `knx_projection::ProjectTree` has no field for it, no route restyles a
  project, and `knx-core` has no restyle operation at all.
- **§87 — `linkable` stays NULL forever in databases built before
  2026-09-13.** `install_package` short-circuits on a known sha256 and
  `migrate_v5_to_v6` adds the column without re-deriving it. No data is lost
  (the XML is still in `source_file`) and rebuilding is cheap. The real fix
  is a v7 migration that re-parses — deliberately not done, because it would
  be the first migration in the chain to call the parser, and that coupling
  deserves an ADR rather than a reflex. **Write the ADR, then decide.**
- **§86 — duplicate identifiers inside one file are dropped with no record.**
  Closing this needs `first_winner` to hash something finer than "the whole
  file", which is a behavioural change rather than a counter.
- **§1 — schema evidence gaps.** Schema 11 (ETS4) is fully known; 21 and 23
  are known in part; 12-19 and 22 rest on no evidence at all, and
  `ModuleInstances` was flagged as the next major format-support task. Only
  actionable when a sample appears; do not synthesize one and call it
  evidence.
- **§13 / row A6 — password-protected projects.** Partially closed
  2026-09-13: ETS6 AES/PBKDF2 derivation lives in `crates/knx-secure`. The
  ZipCrypto (ETS4/5) side remains.
- **§62 — passive Group Monitor real-gateway verification completed
  2026-09-16.** Two bounded production-path sessions received 52 and 65
  telegrams with no drops; with the real reference project open, 65/65 names
  resolved and 10 values decoded. No bus read, write, response, management
  request, or scan was sent. Tunnelling-only, single-session, client-side
  filtering, no auto-reconnect, and unverified transmit behavior remain.

---

## 6. Parked review findings — small, no owner assigned

From the whole-branch closing review of 2026-09-13. Each is small enough that
it keeps losing to larger work, which is exactly why they are listed here:

1. `bool_flag` returns `None` without recording the discard through
   `UnknownCollector` (narrowed, not closed — the corpus happens to contain
   only the four canonical spellings).
2. The `first_winner` helper is copy-pasted between
   `crates/knx-productdb/src/parse/hardware.rs` and `.../catalog.rs`.
3. `master.rs`'s `manufacturer` table uses `ON CONFLICT(id) DO UPDATE` —
   last-writer-wins, a different mechanism from `first_winner`.
4. `program.rs` has a bare `_ => {}` arm that swallows the unmatched case.
5. `style_from_str` falls back silently to `ThreeLevel`.
6. `api.setParameterValue` has a publish hole — one edit path does not
   publish, and never has.
7. Task 6's screenshots need regenerating.

---

## 7. Open items from `ideas.md`

`ideas.md` is an informal wish list, not a backlog, but three of its entries
are actionable now and appear nowhere else. The project logo, listed there
under "in Arbeit", is deliberately **not** part of this goal — the user is
handling it separately. Do not start it, and do not fold it into packaging.

- **Humor templates are far too thin.** The explicit request is a minimum of
  **30 distinct sentences per part**, in the register of Dungeon Keeper II or
  Marvin from *The Hitchhiker's Guide to the Galaxy*. Measured today in
  `apps/knx-web/src/toastCopy.ts`: `ERROR_WRAPPERS` has 7, `LATE_NIGHT_MESSAGES`
  has 4, `HOLIDAYS` has 7. The mechanism is finished and shipped (Session 5
  cycle 10) — only the copy is missing, which makes this cheap, parallelizable
  work. The original backend message must keep appearing verbatim inside the
  wrapper, exactly as it does now.
- **Theme support.** `ideas.md` asks for themes in the plural. What exists is
  a System/Light/Dark toggle. A theme *system* — named palettes, a defined
  token surface, user selection persisted — does not. Decide the token
  boundary before writing any palette, and keep it in step with T27's motion
  settings rather than building a second, parallel settings mechanism.
- **Stale entry, fix the document itself.** `ideas.md`'s "Schema 21/23
  vollständiger Import-Support" entry still reads "Noch nicht implementiert".
  That is out of date: schema 21 import+export shipped and is round-trip
  verified against one sample, and schema 23 import shipped
  (`docs/IMPLEMENTATION_STATUS.md:931`). The real residue is narrower and
  belongs with section 5's §1 — schema 23's module handling is inferred from
  schema 21's measured shape rather than independently evidenced, so there is
  no round-trip claim for it, and schema 23 manufacturer-data ingestion
  remains its own gap (`docs/KNOWN_LIMITATIONS.md` §12). Correct the entry
  rather than letting it keep contradicting the status document.

The remaining `ideas.md` entries — MCP capabilities, automation of repetitive
tasks, the "who talks to whom" animation, in-app project documentation,
mobile, multi-OS — are already covered by sections 8 and 9 below, or shipped
(device discovery, animations, the project status dashboard).

---

## 8. Research before design — do not implement

`docs/ROADMAP.md:527` records an LLM / natural-language interaction item: an
in-app chat surface over the project, *plus* MCP capability from outside —
not decided, not designed, no research done. Both rest on the same
prerequisite: a mature, near-complete `Command` layer. Automation of
repetitive tasks sits on the same foundation.

If any of this is picked up, the first deliverable is a written research
section in `docs/RESEARCH.md` covering capability scope, authorization
against a live project, how natural language maps onto the `Command` layer,
which model and whether local or remote, and what it must never be allowed to
do unsupervised to project data. A design spec comes after that, never
before.

Also needing an ADR before implementation: an in-app project notes/documentation
feature (a new domain concept absent from `docs/DATA_MODEL.md`).

The "who talks to whom" visualization — group addresses animated to the
devices they reach, with the reason visible — sits in the same bucket:
deferred, not designed, not started. It needs a mature UI base and, to be
worth more than a static group-link diagram, live telegrams from Session 6's
bus monitor feeding it. `ideas.md` and `docs/ROADMAP.md` both place it after
completion.

---

## 9. Durable non-goals — do not reopen

- `.vd2` legacy databases: out of scope, user decision 2026-09-11.
- `.knxprod` schemes 12-19, 21, 22: no further sample-hunting, user decision
  2026-09-11. Whether they are genuinely encrypted was never established
  either way, and that is accepted.
- ETS reimport of KNXBench-written projects: dropped as a goal by ADR-0015.

Not durable non-goals, but out of scope for *this* run — deferred until the
Linux-first desktop is finished, and listed here only so nobody mistakes them
for either shipped work or permanent refusals: a mobile app (feasible over a
KNX IP interface) and multi-OS desktop support. Both are new-platform efforts;
CLAUDE.md's Linux-first stance makes them premature, not unwanted.

---

## 10. Parallel, subagent-driven delivery

Use Subagent-Driven Development for planned work. Decompose into small,
testable items; give each agent a narrow brief, an isolated worktree when it
will edit files, explicit acceptance criteria, and a report path. Keep a
durable ledger so completed work is never redispatched after a context
compaction.

Subagents in this environment cannot reliably write report files — ask for
findings as returned text and persist them yourself.

Every dispatch carries a task counter in both its description and the first
line of its prompt: `Task x von y` and `Txx, rest N offen`. Status lines
carry the dispatch's own start date and time, taken from `date` at dispatch
time.

Maximize parallelism only where tasks are genuinely independent: they must
not edit the same files, depend on an unfinished interface, or need the same
mutable environment. Keep at most three subagents active alongside the
coordinator.

Name the model and reasoning effort explicitly on every dispatch; never
inherit the coordinator's default. Use the cheapest tier that carries the
item's risk.

| Work type | Model | Effort |
| --- | --- | --- |
| Mechanical, fully specified 1-2-file edit; focused test or re-review | `gpt-5.6-luna` | `low` |
| Multi-file implementation, integration, ordinary debugging, code review | `gpt-5.6-terra` | `medium` |
| Compatibility research, architecture/domain decisions, data-integrity or bus-facing work, difficult debugging | `gpt-6-astra` | `high` |

Escalate one tier after a repeated blocker or a failed fix round.

Two distinct closing reviews, and they do not share a model:

- **Per branch, before merging: a whole-branch review on Opus
  (`claude-opus-5`).** Every branch gets one, every time. This is the review
  that has earned its place empirically — on 2026-09-13 it caught a leaked
  address a targeted sweep had walked straight past, and six documentation
  claims that did not survive checking.
- **Once, at the very end: the final review over the entire goal on Fable
  (`claude-fable-5-1`).** Exactly one Fable run in the whole effort, dispatched
  only when the completion condition in section 11 is believed met and every
  branch has already had its own Opus review. Nothing else in this run uses
  Fable, and no branch-level review is ever escalated to it.

If the goal runner cannot dispatch a Claude model, do not substitute a cheaper
one and do not skip either review: stop, say so, and hand the outstanding
review back to be run from a Claude session.

Before integration, the coordinator reviews each subagent's diff, test
evidence, documentation and report. Do not take a subagent's summary at face
value — verify the claim, especially a green one.

---

## 11. Completion condition

Finish only when:

- `docs/IMPLEMENTATION_STATUS.md`, `docs/ROADMAP.md`,
  `docs/GAP_ANALYSIS_ETS.md` and `docs/KNOWN_LIMITATIONS.md` are reconciled
  with each other and with the code;
- every actionable item above has proof of completion, or a recorded,
  non-actionable external blocker;
- all eight gates pass;
- every remaining exception carries the user's explicit out-of-scope
  acceptance;
- and the single Fable final review (section 10) has run over the finished
  whole and its findings are resolved. That review is the last thing that
  happens, not a formality on the way out — if it opens something, the goal
  is not done.

Report the completed work, the verification evidence, the remaining external
blockers, and the next required user decision, if any. If the licence
question from section 1 is still unanswered at the end, say so plainly —
it is the one item that cannot be closed by any amount of engineering.
