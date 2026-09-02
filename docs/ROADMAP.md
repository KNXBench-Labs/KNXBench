# Roadmap

Seven sessions after the research phase. Each states its goal, its
deliverables, and the entry condition that must hold before it starts. A
session that starts without its entry condition met produces work that has to
be redone.

Current position: **Session 1 complete**. See
[IMPLEMENTATION_STATUS.md](IMPLEMENTATION_STATUS.md).

## Session 0 — Technical research

**Done.** See [RESEARCH.md](RESEARCH.md).

## Session 1 — Architecture

**Done.** The stack, the layering, the domain model design, the import and
export contract, the compatibility statement, nine ADRs, and a Cargo workspace
with two mechanically enforced rules and CI.

## Session 2 — KNX core

**Goal.** Implement the domain model of [DATA_MODEL.md](DATA_MODEL.md).

**Deliverables.** Entities (project, installation, area, line, device, group
address, group range, communication object instance, parameter instance,
building part); typed addresses with parsing and formatting; datapoint type
references; the override resolution that turns three source layers into
`Resolved<T>`; validation rules; the string table; the command layer with
inverses for undo and redo; the schema version and the migration chain
skeleton.

**Entry condition.** The workspace builds, and both gates pass —
`cargo run -p xtask -- check-layering` and `cargo deny check`. Met.

## Session 3 — ETS project import

**Goal.** Read the reference project into the model, and write it back.

**Deliverables.** The six-stage pipeline of
[IMPORT_EXPORT.md](IMPORT_EXPORT.md) for schema 11; the opaque passthrough
store; the `ImportReport`; the golden test asserting the reference project's
counts (36 devices including the unassigned one, 514 group addresses, 907
communication object instances, 1390 parameter values, 569 send and 27 receive
links); the oracle comparison against `xknxproject` where it is not known to be
lossy; the three roundtrip guarantees; the malformed-input suite.

**Entry condition.** The core model exists and is testable without IO.

**Blocking risk.** No ETS5 or ETS6 sample project is available (risk R1).
Acquiring one is a prerequisite for claiming support beyond schema 11 —
without it, the tolerant parser reports unknown constructs and that is all we
can honestly say.

## Session 4 — Manufacturer databases

**Goal.** The shared product database, and the research the parameter editor
depends on.

**Deliverables.** The product database schema (manufacturer → product →
application program → version → parameters, communication objects, DPTs);
ingest from `.knxproj` manufacturer data, keyed with a content hash and
skipping existing entries; indexed and cached access so that a project open
never re-parses 22 MB; graceful degradation when the database is missing.

Plus the **`when/@test` expression grammar spike** (risk R3). This is research,
not a feature, and its output is a documented grammar plus a decision on
whether a parameter editor is feasible.

**Entry condition.** Import produces application program references worth
resolving.

## Session 5 — UI and UX

**Goal.** The desktop application.

**Deliverables.** `apps/knx-desktop` — the Tauri shell and the React
application, scaffolded here rather than in Session 1; the projection layer
(`ProjectTree`, `DeviceList`, `GroupAddressTable`, `Inspector<T>`) with `ts-rs`
bindings and a CI check that they are not stale; Project Explorer, properties
inspector, search, command palette, dark and light mode. Rule 3 of
[ARCHITECTURE.md](ARCHITECTURE.md) section 4 becomes a mechanical gate in this
session, since a UI finally exists to check.

**Entry condition.** Import produces a model worth displaying.

## Session 6 — KNXnet/IP

**Goal.** Talk to the bus.

**Deliverables.** The `BusConnection` trait implemented against ISO 22510:
discovery, tunnelling, routing, cEMI and telegram encoding; the bus monitor as
a consumer that resolves telegrams against the open project; connection
management and diagnostics.

**Entry condition.** A project can be opened and its group addresses resolved,
so that captured telegrams have something to resolve against.

## Session 7 — Integration and hardening

**Goal.** Make it trustworthy on real projects.

**Deliverables.** The full roundtrip and migration suites with frozen fixtures
per schema version; performance measurement on large projects, with
optimization driven by those measurements rather than by guesswork; packaging
for Linux; the licence decision.

**Entry condition.** All earlier sessions' deliverables exist and are tested.

## Open questions and where they land

Carried forward from [RESEARCH.md](RESEARCH.md) §12. None of them block the
architecture; each has a defined landing place.

| Question | Lands in |
| --- | --- |
| ETS5 and ETS6 schema deltas (13, 14, 20, 21+) | Session 3 — the tolerant parser reports unknown constructs; per-version known-element lists |
| `Functions` element semantics | Session 2 — a domain model addition; absent from the reference sample |
| `when/@test` expression grammar | Session 4 — a research spike, prerequisite for the parameter editor |
| Whether ETS re-imports an unsigned third-party `.knxproj` | Session 3 — the export warning stays until it is tested |
| Whether Data Secure runtime keys are readable from `.knxproj` | Session 7 or later — `knx-secure` |
| `.knxprod` encryption for master data scheme ≥ 12 | Session 4 — product database ingest; out of v1 scope |
| The project licence | Session 7 — currently a placeholder, see [KNOWN_LIMITATIONS.md](KNOWN_LIMITATIONS.md) |
