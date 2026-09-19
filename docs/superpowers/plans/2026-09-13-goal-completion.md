# Plan — goal.md completion run (2026-09-13)

**Spec:** `goal.md` at repo root (the binding authority). This plan is its
argument. Where the two disagree, `goal.md` wins.

**Base:** `main` at `7276b63`.

## Global Constraints

1. Every task gets its own git worktree and branch. Never edit the main
   checkout. Worktrees live in `/mnt/daten-i/Sourcecode/KNXBench.worktrees/<branch>`.
2. Gate set before a task is declared complete (run in the worktree):
   `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
   `cargo test --workspace --no-fail-fast`, `cargo run -p xtask -- check-layering`,
   `cargo run -p xtask -- check-headers`, `cargo deny check`, plus
   `npx tsc --noEmit` and `npx vitest run` in `apps/knx-web` when any file
   under it was touched. Judge by exit status, never by a summary line.
3. Architecture boundaries from `CLAUDE.md`/`AGENTS.md` hold. `knx-core` must
   not depend on the UI. Import/export formats must not dictate the domain
   model.
4. Never silently discard external-format information. Unsupported data is
   preserved where possible, otherwise explicitly reported.
5. No `.ai/` handover bookkeeping. `docs/` updates are still mandatory.
6. Source files carry a purpose sentence and no version number (ADR-0018).
   Every program version stays `0.1.0-alpha.1`.
7. Commit messages in Marvin's gloomy register, accurate facts, author
   `github@knxbench.com`, **no `Co-Authored-By` trailer, ever.**
8. No claim of KNX certification or full ETS compatibility. Say
   "KNX-compatible".
9. Any statement that the specification lacks something must name which
   knowledge base was searched (see "KNX specification evidence" below).
10. **Hardware:** `1.1.220` is an alarm panel — never read, never write,
    never in a scan range. `1.1.24`-`1.1.32` are approved for active reads.
    The gateway's address is supplied out of band (`KNX_GATEWAY`), not
    written down here — see rule 11. **Nothing writes to a real device without the
    user's explicit, specific go-ahead for that operation.**
11. Sweep by pattern class, never by one remembered literal (RFC 1918 ranges,
    not one address).

## KNX specification evidence

Two knowledge bases, complementary:

- programming-scoped, **has figures**:
  `/mnt/daten-i/Sourcecode/knx-spec-kb/knowledge_base/knx_spec_kb_programming.sqlite`
- full 177-PDF, text only:
  `/mnt/daten-i/Sourcecode/knx-spec-kb/knowledge_base/knx_spec_kb_full179_clean.sqlite`

```bash
cd /mnt/daten-i/Sourcecode/knx-spec-kb && .venv/bin/python scripts/05_knowledge_base_v1.py \
  -o knowledge_base/knx_spec_kb_programming.sqlite --query "load state machine" --limit 5
# figures (programming base only): --query-figures "load state machine"
```

Also available: `python3 ~/.claude/skills/knx-spec/scripts/knx_spec.py search <keyword>`.

Cite the PDF name and `evidenceText` for every specification claim. Write
query output to a file and read it with the Read tool when exact wording goes
into code, a doc or a citation — piped shell output is not verbatim.

---

## Task 1 — productdb parked findings (branch `productdb-parked-findings`)

Closes `goal.md` §6 items 1-4. All in `crates/knx-productdb`.

1. `bool_flag` (`src/parse/mod.rs:38`) returns `None` for an unrecognised
   spelling without recording the discard through `UnknownCollector`. Record
   it. The corpus contains only the four canonical spellings today, so this
   is about the next corpus, not this one.
2. `first_winner` is copy-pasted between `src/parse/hardware.rs:240` and
   `src/parse/catalog.rs:197`. Extract one shared helper. **Do not change its
   behaviour** — the behavioural change is Task 8's.
3. `src/parse/master.rs:84`'s `manufacturer` table uses
   `ON CONFLICT(id) DO UPDATE SET name = excluded.name` — last-writer-wins,
   a different mechanism from `first_winner`'s first-writer-wins. Either
   align it with `first_winner` or document in `docs/KNOWN_LIMITATIONS.md`
   why master data legitimately wants last-writer-wins. A deliberate choice
   with a citation is an acceptable outcome; an undocumented inconsistency is
   not.
4. `src/parse/program.rs` has a bare `_ => {}` arm that swallows the
   unmatched case. Make the discard explicit and recorded.

**Acceptance:** a regression test per item that fails before the change.
Gate set green. `docs/KNOWN_LIMITATIONS.md` updated for whatever stays true.

---

## Task 2 — humour copy (branch `humour-copy`)

Closes `goal.md` §7 item 1. `apps/knx-web/src/toastCopy.ts` only.

Minimum **30 distinct sentences per part**:
- `ERROR_WRAPPERS` (7 today)
- `LATE_NIGHT_MESSAGES` (4 today)
- `HOLIDAYS` (7 today — 30 distinct holiday entries)

Register: Dungeon Keeper II's narrator or Marvin from *The Hitchhiker's Guide
to the Galaxy*. Gloomy, dry, never cruel to the user.

**Hard constraint:** the original backend message must keep appearing
**verbatim** inside every `ERROR_WRAPPERS` entry, exactly as it does now.
Whatever placeholder mechanism the file uses today, every new entry uses it
too.

**Acceptance:** each array has ≥30 entries, all distinct; a test asserts the
count and the distinctness and that every wrapper contains the placeholder;
`npx tsc --noEmit` and `npx vitest run` green in `apps/knx-web`. No mechanism
change — copy only.

---

## Task 3 — `setParameterValue` publish hole (branch `parameter-publish-hole`)

Closes `goal.md` §6 item 6. One edit path through `api.setParameterValue`
does not publish its change, and never has. See `apps/knx-web/src/App.tsx:196`
and `apps/knx-web/src/ParameterPanel.tsx:36`, which both describe the hole in
comments.

Find the path that does not publish, close it, and delete or correct the
comments that describe the hole as permanent.

**Acceptance:** a `vitest` test that fails before the fix — it must assert
the publish, not merely the call. `tsc` and `vitest` green. Rust gates green
if any Rust file is touched.

---

## Task 4 — group address style, end to end (branch `group-address-style`)

Closes `goal.md` §5 §84 and §6 item 5.

1. `knx-store`'s `style_from_str` (`src/project.rs:42`) falls back silently to
   `ThreeLevel` for an unknown string. A persisted style that cannot be read
   back is data loss; make it an error or a recorded fallback, consistent with
   `POST /api/project/new`'s `400` for an unknown style.
2. `knx_projection::ProjectTree` carries the project's group address style.
3. The properties inspector shows it for the project node
   (`apps/knx-web/src/Inspector.tsx`).
4. A `knx-core` command restyles a project **only when every existing group
   address still fits the target style**, refusing with a specific error when
   one does not. Undo/redo like every other command. Wire it through
   `knx-store`'s `command_sync`, a `knx-server` route, and the UI.

Layer order matters: `knx-core` command first, then store, then projection,
then server, then web.

**Acceptance:** `knx-core` tests for the fits/does-not-fit boundary in all
three styles; a store round-trip test; a projection test; a server route
test; a `vitest` test for the inspector field. §84 closed in
`docs/KNOWN_LIMITATIONS.md`, or its residue restated honestly. Full gate set
including `tsc`/`vitest`.

---

## Task 5 — DPT main types 20-30 and 46 (branch `dpt-main-types-20-46`)

Closes `goal.md` §3 E4. `crates/knx-core/src/dpt/codec.rs` covers main types
1-19 except `6.020`. Add: 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 46, and
`6.020`.

Work from the specification, not from memory — 03_07_02 Datapoint Types, via
the knowledge bases named above. Every bit layout, range and rounding rule in
a doc comment cites a DPT-AS section. Follow the module's existing citation
style exactly; it is already strict about this.

Where the Standard is silent or self-contradictory, state the ruling in the
doc comment and in `docs/KNOWN_LIMITATIONS.md` §61 — that section exists
precisely to carry these, and §61's title ("nineteen main types") must be
re-measured and corrected.

A main type with no documented encoding stays unsupported and returns the
existing "unsupported, not a panic" error. Do not invent semantics.

**Acceptance:** decode and encode round-trip tests per main type, boundary
tests at each range edge, a malformed-payload test per type, and the existing
`unimplemented_main_type_is_unsupported_not_a_panic` test still passing for
whatever genuinely remains unimplemented. `docs/KNOWN_LIMITATIONS.md` §61 and
`docs/IMPLEMENTATION_STATUS.md` re-measured, not edited from memory. Full
Rust gate set.

---

## Task 6 — performance baseline on large projects (branch `perf-baseline`)

Closes the first of `goal.md` §2's three Session 7 deliverables.

CLAUDE.md requires optimization to follow measurement, so **the measurement is
the deliverable.** Do not optimize anything in this task unless the numbers
name the bottleneck, and then only that.

Deliver a reproducible benchmark over a large project covering: import
(`.knxproj`), open (`.knxdb`), projection, search, export. "Large" must be
defined and generated deterministically — a synthetic project generator with a
fixed seed and a stated size (device count, group address count, building
depth) is acceptable and preferable to depending on a sample nobody else has.

Record the numbers, the machine, the toolchain version and the exact command
in a new `docs/PERFORMANCE.md`, and link it from
`docs/IMPLEMENTATION_STATUS.md`. A later optimization needs a baseline to
beat; that is what this file is for.

**Acceptance:** one command reproduces the whole benchmark from a clean
checkout. Numbers in `docs/PERFORMANCE.md` with their conditions. Full Rust
gate set. The benchmark itself must not slow the normal test run — gate it
behind a feature, an `--ignored` test, or its own binary, and say which.

---

## Task 7 — Linux packaging (branch `linux-packaging`)

Closes the second of `goal.md` §2's Session 7 deliverables. Nothing exists
today.

Decide the format explicitly and record the decision and its reasoning in an
ADR under `docs/adr/`. Candidates: AppImage, Flatpak, plain tarball, distro
package (PKGBUILD for the Arch host, `.deb`). Consider that this workspace
ships three user-facing programs — `knx-desktop` (Tauri), `knx-server` +
`knx-web`, and `knx-cli` — and they may not want the same format.

Then implement the chosen path far enough to produce an artifact, and document
how to build it in `README.md`.

**Acceptance:** the ADR exists and names what it rejected and why. A
build produces a real artifact on this machine, and the report states the
command and the resulting file size. `docs/ROADMAP.md` and
`docs/IMPLEMENTATION_STATUS.md` updated. Full gate set for whatever code
changed.

---

## Task 8 — `first_winner` collision granularity (branch `productdb-collision-record`)

Closes `goal.md` §5 §86. **Depends on Task 1** (Task 1 extracts the shared
helper; this task changes its behaviour).

Duplicate identifiers inside one file are dropped with no record at all,
because `first_winner` hashes "the whole file". Make it hash something finer,
so a same-file collision is recorded rather than invisible. This is a
behavioural change, not a counter — say in the report what the new key is and
what it costs.

**Acceptance:** a test that a same-file duplicate identifier is recorded, and
a test that a cross-file duplicate still behaves as before. §86 closed or its
residue restated in `docs/KNOWN_LIMITATIONS.md`. Full Rust gate set.

---

## Task 9 — the `linkable` re-derivation ADR (branch `productdb-linkable-adr`)

Closes `goal.md` §5 §87. **ADR first, then decide** — that is the whole
instruction and it is deliberate.

`linkable` stays NULL forever in databases built before 2026-09-13:
`install_package` short-circuits on a known sha256, and `migrate_v5_to_v6`
adds the column without re-deriving it. No data is lost — the XML is still in
`source_file` — and rebuilding is cheap.

The real fix is a v7 migration that re-parses. That would be the first
migration in the chain to call the parser. Write an ADR on exactly that
coupling: whether a migration may depend on the parser, what it costs when the
parser's behaviour later changes, and what the alternatives are (re-derive
lazily on read; force a rebuild; leave it). Then implement whatever the ADR
decides, in the same branch.

**Acceptance:** `docs/adr/00NN-*.md` with a decision, its alternatives, and
its consequences. Implementation matching the decision, with tests. §87
updated to match reality. Full Rust gate set.

---

## Task 10 — deep parameter format validation (branch `t18-format-validation`)

Part of `goal.md` §3 T18. `Float`, `Text`, `IPAddress`, `Picture` and `Raw`
parameter types get a non-empty-string check and nothing more.

Implement real validation per type, sourced from the `.knxprod`/`.knxproj`
schema's own definitions where they exist, and from the KNX specification
where they do not. `IPAddress` in particular must state which forms it
accepts (v4, v6, both) and why.

Where the format's own definition is unavailable, validate what is
defensible and record the rest in `docs/KNOWN_LIMITATIONS.md` rather than
pretending.

**Acceptance:** per-type accept and reject tests including the boundary cases.
No change to the stored representation. `docs/KNOWN_LIMITATIONS.md` §§68-71
re-measured for what this closes. Full gate set including `tsc`/`vitest` if
the panel's error display changed.

---

## Task 11 — nested module expansion (branch `t18-nested-modules`)

Part of `goal.md` §3 T18. Nested modules are not expanded — a `Module` inside
a `ModuleDef`'s own tree.

Extend the evaluator in `crates/knx-productdb` to expand nesting. Depth must
be bounded and a cycle must be refused with a specific error, not a stack
overflow.

**Acceptance:** a test with a two-level nesting, a test with a cycle, a test
at the depth bound. A corpus check: state how many products in the installed
database actually nest, measured, not guessed. Full Rust gate set.

---

## Task 12 — `Module` argument interpretation (branch `t18-module-arguments`)

Part of `goal.md` §3 T18. `Module` argument values (`NumericArg`/`TextArg`)
are stored but uninterpreted; `AllocatorRef` is unattested.

Interpret `NumericArg` and `TextArg` — that is, make an argument value
actually affect what the evaluator produces for the module instance.

`AllocatorRef` is **unattested**: if the corpus and both knowledge bases give
no evidence for its semantics, it stays unimplemented and explicitly reported,
and the report says which bases were searched. Do not invent it.

**Acceptance:** a test where an argument value changes the evaluated result.
A measured statement of `AllocatorRef`'s presence in the installed corpus.
`docs/KNOWN_LIMITATIONS.md` §§68-71 updated. Full Rust gate set.

---

## Task 13 — translations outside an application program (branch `d10-language-data`)

Closes `goal.md` §3 D10's `docs/KNOWN_LIMITATIONS.md` §64: `Languages` blocks
outside an application program — hardware-scope and master-scope translations
— are discarded on import.

Preserve them. This is a data-integrity item under `CLAUDE.md`'s "never
silently discard information", so retention comes before display: store them
with their scope, then surface them where the existing language machinery
already reads translations.

A store schema bump is likely. If so, follow the existing migration chain
convention exactly and add a frozen fixture for the new version like every
earlier bump did.

**Acceptance:** an import test proving a hardware-scope and a master-scope
translation survive; a migration test from the previous schema version with a
frozen fixture; §64 closed or its residue restated. Full Rust gate set.

---

## Task 14 — translate server prose and the report export (branch `d10-server-prose`)

Closes `goal.md` §3 D10's remaining two: `docs/KNOWN_LIMITATIONS.md` §66
(server-composed diagnostic/log/error prose and `knx-report`'s documentation
export are not language-aware in any respect) and §67 (a rejected language
pack's own rejection reason is shown untranslated inside a translated
sentence).

§67 is the smaller and sharper of the two; do it first, in the same branch.

For §66, decide and state the boundary: which prose is user-facing (and so
must be translatable) and which is developer-facing log text (and so stays
English). Do not translate log text nobody reads in German.

**Acceptance:** §67 has a test showing the reason translated inside the
sentence. §66 has a test per translated surface, and a stated, documented
boundary for what stays English. `docs/KNOWN_LIMITATIONS.md` §66/§67 updated.
Full gate set including `tsc`/`vitest`.

---

## Task 15 — ZipCrypto-protected ETS4/5 projects (branch `zipcrypto-projects`)

Closes the open half of `goal.md` §5 §13 / row A6. ETS6's AES/PBKDF2
derivation already lives in `crates/knx-secure`; the ZipCrypto (ETS4/ETS5)
side does not.

Implement ZipCrypto decryption for a password-protected `.knxproj`, in
`knx-secure`, behind the same interface the ETS6 path uses.

Note honestly: ZipCrypto is weak by design. Say so in the documentation, do
not present it as security, and make sure nothing in the codebase can
*produce* a ZipCrypto-protected file — read support only.

If no ZipCrypto-protected sample exists in `OriginalData/` or the repository,
generate one with a known password using a standard tool, commit it as a
fixture, and say in the report that it is synthetic and why that is
acceptable here (the algorithm is specified and the fixture tests the
algorithm, not a vendor's quirks).

**Acceptance:** a decryption test against a fixture with a known password, a
wrong-password test, and a not-encrypted test. `docs/COMPATIBILITY.md` and
`docs/KNOWN_LIMITATIONS.md` §13 updated. Full Rust gate set.

---

## Task 16 — re-verify the Group Monitor against reality (branch `group-monitor-reverify`)

Closes `goal.md` §5 §62's first clause. §62 says the Group Monitor GUI is
tunnelling-only, single-session, client-filtered, and "had never talked to a
real gateway when that section was written."

**Re-verify before fixing or restating.** A gateway is reachable on the
installation's own LAN; its address comes from `KNX_GATEWAY` at run time and
is deliberately absent from this file. Reading from the bus is allowed.
Individual addresses
`1.1.24`-`1.1.32` are approved for active reads. **`1.1.220` is an alarm
panel: never read it, never write to it, never include it in a range.**

**This task performs no bus writes of any kind.** Group monitoring is passive;
keep it passive.

Then either fix what is fixable or restate §62 with what was actually
measured — and say which it was.

**Acceptance:** a record of what was observed on the real gateway (telegram
count, duration, whether they resolved against an open project). §62 rewritten
against measurement, not memory. Any code change carries a test. Full gate
set for whatever changed.

---

## Task 17 — click the from-scratch launcher in a real browser (branch `launcher-browser-verify`)

Closes `goal.md` §5 §83. The code and its tests exist
(`NewProjectDialog.test.tsx`, `App.test.tsx`); §80's "lifted when" has a
second clause that is a manual verification in a real browser, still unmet.

`chromium` is at `/usr/bin/chromium` and `playwright` at
`/home/knxbench/.local/bin/playwright`. Drive the real `knx-server` +
`knx-web` build in a real browser: create a project from scratch, with each
of the three group address styles, and confirm what the dialog claims.

Prefer an automated Playwright check committed to the repository over a
one-off manual run — a check that can be re-run is worth more than a claim
that it once passed. If a browser dependency cannot be installed, say so
exactly and stop; do not substitute the existing unit tests and call §83
closed.

**Acceptance:** a committed browser-level check, the command to run it, and
its output. §83 and §80's second clause updated to match what was actually
observed. `tsc`/`vitest` green plus the new check.

---

## Task 18 — a theme system (branch `theme-system`)

Closes `goal.md` §7 item 2. `ideas.md` asks for themes in the plural. What
exists is one named theme ("Bitcoin DeFi") in a `ThemeDef`/`THEMES` registry
in `apps/knx-web/src/theme.ts`, selected from `SettingsPanel.tsx`.

**Decide the token boundary before writing any palette** — which custom
properties are a theme's to set and which belong to the component layer — and
write that boundary down. Keep it in step with T27's motion settings rather
than building a second, parallel settings mechanism.

Then ship at least three themes against that boundary. Per the user's roadmap
memo, two of the motion/visual registers to cover are an Apple-subtle one and
a cyberpunk-glitch one, both clean and sleek.

Persist the selection where the existing theme selection already persists.

**Acceptance:** the token boundary documented in `docs/` or a design spec; ≥3
themes; a test that every theme defines every token in the boundary (the test
that actually prevents a half-themed palette); the selection persisted and
restored. `tsc`/`vitest` green. Screenshots of each theme in the report.

---

## Task 19 — spatial coordinates: the domain decision (branch `t21-coordinates-adr`)

`goal.md` §3 T21's remaining half. The workbench already renders projected
areas/lines/devices and nested building parts with keyboard selection. A
spatial canvas / floor-plan editor "remains outside the current model — it
needs a domain decision about coordinates before it needs a UI."

**This task is the ADR, not the UI.** Decide: does a device or building part
carry coordinates in the domain model? In what space, what units, whose
origin? Is a floor plan an imported raster, a vector drawing, or a
KNXBench-native construct? What does ETS itself store, and does
`docs/DATA_MODEL.md` need a new concept (which would need its own ADR anyway)?

Answer from evidence: the `.knxproj` schema, the two knowledge bases, and the
reference projects in the repository. If ETS stores coordinates, say where and
in which schema versions.

**Do not implement the canvas.** The ADR's decision may well be "not in
v1.0.0", and that is a legitimate outcome recorded rather than a task failed.

**Acceptance:** `docs/adr/00NN-*.md` with the decision and its evidence.
`docs/DATA_MODEL.md` and `docs/ROADMAP.md` updated to match. `goal.md` §3
T21's residue restated. No code change expected; gates still run.

---

## Task 20 — T30 commissioning, phase 1: the written procedure (branch `t30-commissioning-research`)

`goal.md` §3 T30, the largest open item. **This phase writes no code that
touches a bus and performs no bus access at all.**

`docs/RESEARCH.md` §8.4 already documents the generic
load/unload/reset/memory procedures and the Load State Machine. Consolidate
that into an implementable specification covering, each with its citation:

- individual-address programming via the device's programming button
  (`A_IndividualAddress_Write`, programming-mode detection, verification)
- the Load State Machine's states and legal transitions
- memory read/write over the bus, including the length limits and the
  verification read
- application-program download: what ETS sends, in what order, and what the
  device does with it
- restart/reset
- error handling, and what an interrupted download leaves behind

Then state the boundary explicitly: **the product-specific matrix and any
vendor-DLL involvement in download are undocumented.** Do not invent
semantics across that boundary; name exactly what is unknown and what that
forbids.

Deliver a design spec under `docs/superpowers/specs/` that a later phase can
implement from, plus a risk section: for each operation, what happens to a
real device if the implementation is wrong.

**Acceptance:** the spec exists, every protocol claim cites a source PDF and
its `evidenceText`, and the unknown boundary is explicit. `docs/RESEARCH.md`
and `docs/KNOWN_LIMITATIONS.md` §7 updated. No bus access, no device writes.

---

## Task 21 — T30 commissioning, phase 2: the protocol, offline (branch `t30-commissioning-protocol`)

**Depends on Task 20.** Implement the procedures Task 20 specified, in
`crates/knx-net` and `crates/knx-core`, with **no live-bus execution**:
encoding, decoding, the Load State Machine as a pure state machine, and the
download sequencer — all testable against a simulated device.

Build a device simulator in `knx-testsupport` good enough to drive the state
machine through a full download and through every failure branch the spec
names.

**Every write-to-hardware entry point must be gated** so it cannot fire
without an explicit caller opt-in, and `1.1.220` must be rejected at the
lowest level that knows an individual address, with a test proving the
rejection.

**Acceptance:** the state machine and the sequencer pass tests for the happy
path and for each named failure branch, against the simulator. A test proving
`1.1.220` is refused. A test proving no code path writes to hardware without
the explicit opt-in. Full Rust gate set. **No bus access.**

---

## Task 22 — T30 commissioning, phase 3: read-only bus verification (branch `t30-commissioning-readonly`)

**Depends on Task 21. Read-only.** Verify against the real installation
exactly what can be verified without writing: device presence, mask version,
descriptor reads, property reads, load-state reads on `1.1.24`-`1.1.32`.

**`1.1.220` is an alarm panel — never touched.** No write of any kind. No
programming-mode manipulation. No memory write. No restart.

Record what the real devices answered, and reconcile it against Task 20's
spec: every place reality disagreed with the documented procedure is a
finding, and the spec is what gets corrected.

**Acceptance:** observed responses recorded in `docs/RESEARCH.md` with the
device addresses and the raw payloads. Discrepancies against the spec listed.
Full Rust gate set. **No writes. The write phase is the user's decision, not
this task's.**

---

## Task 23 — in-application help (branch `t28-help`)

`goal.md` §3 T28, **deliberately last**, by explicit user request. Do not
start this before Tasks 3, 4, 10, 14, 17, 18 and 19 have landed — help text
describes a specific UI, and a moving UI invalidates its own help every
cycle.

Re-measure first; `goal.md` says so and the number has drifted before. As of
2026-09-13: eight `title` attributes, 33 `aria-label`s, four
`aria-describedby`s, no tooltip component, no help panel, no `F1` handler.

`docs/ROADMAP.md`'s "Cross-cutting" section lists the questions this has to
answer: hover/tooltip versus a persistent context panel versus both; where
help text lives so T25's translation machinery can reach it; whether KNX
concepts are explained in-app or linked out; and whether any of `docs/` ships
to the user.

**Acceptance:** a design spec answering those questions, then the
implementation: a tooltip mechanism, a help panel, an `F1` handler, and help
text for the surfaces that exist. Text lives where the translation machinery
can reach it. `tsc`/`vitest` green, accessibility attributes re-measured
after. Row D12 closed in `docs/GAP_ANALYSIS_ETS.md`.

---

## Task 24 — documentation reconciliation (branch `docs-reconciliation`)

The completion condition's first clause. **Runs last, after every other task
has merged.**

Reconcile `docs/IMPLEMENTATION_STATUS.md`, `docs/ROADMAP.md`,
`docs/GAP_ANALYSIS_ETS.md` and `docs/KNOWN_LIMITATIONS.md` with each other
and with the code. **Re-measure every count** — test counts, attribute
counts, main-type counts, crate counts. Several counts in these documents have
drifted before and were only caught by re-measuring.

Also fix `ideas.md`'s stale "Schema 21/23 vollständiger Import-Support" entry
(`goal.md` §7 item 3): schema 21 import+export shipped and is round-trip
verified against one sample, schema 23 import shipped. The real residue is
narrower — schema 23's module handling is inferred from schema 21's measured
shape rather than independently evidenced, so there is no round-trip claim for
it, and schema 23 manufacturer-data ingestion remains its own gap
(`docs/KNOWN_LIMITATIONS.md` §12).

Session 7's status in `docs/ROADMAP.md:622` and
`docs/IMPLEMENTATION_STATUS.md:779` must end up matching whatever is actually
true after this run.

**Acceptance:** every number in those four documents re-derived by a command
whose output is in the report. No contradiction between any two of them. Full
gate set.

---

## Task 25 — visible progress while loading a project (branch `t37-load-progress`)

Closes `goal.md` §3's **T37**, which this plan originally omitted. Added
2026-09-19 on the user's ruling: schedule it after Task 18, before Task 23.
**Runs after `theme-system` has merged** — both own `apps/knx-web/src/App.tsx`.

Both project-entry paths are silent while work is in progress: `App.tsx`
awaits a single response from `POST /api/project/import` for ETS `.knxproj`
files, or `POST /api/project/open` for native `.knxdb` files. A large import
and a stalled request look identical to the user.

**Design before implementation.** The progress transport and the operation
lifecycle are a design decision, not an implementation detail: decide how a
stage reaches the browser (the existing request/response shape, an event
stream, or polling), where the operation's identity lives, and what happens
to an operation whose client disconnects. Write that decision down in
`docs/` — an ADR if it changes the HTTP surface, which it probably does.

Then build one coherent loading-operation model spanning the owning backend
stages and the frontend:

- Show the operation and its truthful current phase. Import stages follow the
  actual external-data pipeline; native open reports store open/migration,
  normalized load and projection as applicable.
- Use a percentage **only** where a real completed/total measurement exists.
  Otherwise show indeterminate progress with a phase label. Never synthesize
  progress from elapsed time — a fabricated bar is the same defect class as a
  test that cannot fail.
- Prevent duplicate open/import actions while one is running.
- Announce phase changes via `aria-live`.
- Retain the old project until the replacement is fully ready, and retain it
  on failure while reporting the error.
- Cancellation is out of scope unless the design proves it cannot publish
  partial state.

**Acceptance:** the design document exists and is referenced from the
implementation; backend tests prove the real stages arrive in order; frontend
tests cover determinate and indeterminate rendering, duplicate prevention,
success and failure; one real large-project run verifies the feedback is
visible for the operation's whole duration, with the measured duration in the
report. Full Rust gate set plus `npx tsc --noEmit` and `npx vitest run`.

---

## Deliberately not in this plan

- `goal.md` §4: T19 KNX Secure, T20 `Functions`, T22 multi-user — deferred by
  user ruling, not to be opened or partially folded in.
- `goal.md` §8: the LLM/MCP interaction surface, the in-app project notes
  feature, and the "who talks to whom" visualization — research before design,
  not implementation.
- `goal.md` §9: `.vd2`, `.knxprod` schemes 12-19/21/22, ETS reimport, mobile,
  multi-OS — durable non-goals or out of scope for this run.
- The project logo — the user is handling it separately.
- **The licence** (`goal.md` §1) — **resolved 2026-09-16, outside this run.**
  The user chose `AGPL-3.0-or-later`; Codex added the canonical `LICENSE`,
  the workspace SPDX expression, and the §10 closure, and `cargo deny check`
  passes. Nothing is left here — do not re-ask the question.
- `goal.md` §6 item 7 (Task 6's screenshots need regenerating) — folded into
  Task 18, which regenerates screenshots anyway.
