# KNXBench Project Instructions

## Mission

Build a modern, Linux-first KNX engineering application providing an independent alternative to ETS.

Priority order:

**Correctness → Data Integrity → Compatibility → Maintainability → UX → Performance**

Do not blindly imitate ETS internals. Build a clean, independent architecture.

---

## Understand Before Coding

Before implementing significant changes:

1. Inspect relevant existing code.
2. Read relevant documentation in `docs/`.
3. Check current architecture and implementation status.
4. Identify constraints and dependencies.
5. Do not duplicate existing functionality.
6. Do not assume proprietary formats or undocumented behavior.
7. Research architecture-critical or format-related facts before implementation.

Repository and its maintained documentation are source of truth for project state.

---

## Project Documentation

Important documents include:

```text
docs/
├── RESEARCH.md   (index; topic files in research/)
├── ARCHITECTURE.md
├── DATA_MODEL.md
├── IMPORT_EXPORT.md
├── COMPATIBILITY.md
├── ROADMAP.md
├── IMPLEMENTATION_STATUS.md
├── KNOWN_LIMITATIONS.md
├── OPEN_WORK.md   (everything not done, one page)
├── status/LEDGER.md   (per-ID status of record)
├── contracts/   (focused behavioural contracts)
├── adr/
├── history/   (older IMPLEMENTATION_STATUS entries, verbatim)
└── archive/   (finished goals, alpha dossiers, dated snapshots; read-only)
```

Keep relevant documentation synchronized with implementation.

Important discoveries about KNX formats, compatibility, manufacturer data, protocol behavior, architecture or limitations belong in `docs/`.

Architecture decisions belong in `docs/adr/`.

Do not rely on conversation history as authoritative project state.

---

## Architecture

Maintain strict separation of concerns:

```text
UI
 ↓
Application / Services
 ↓
KNX Domain Core
 ↓
Infrastructure
 ├── Project Storage
 ├── Import / Export
 ├── Device Database
 └── KNXnet/IP
```

KNX Domain Core must not depend on UI.

External formats must not dictate internal domain model.

Preferred external-data flow:

```text
External Format
      ↓
    Parser
      ↓
  Validation
      ↓
Normalized KNX Model
      ↓
 Application
      ↓
   Exporter
```

Use modular adapters/interfaces for external formats and protocols.

Avoid unnecessary frameworks, dependencies and abstraction layers.

---

## KNX Domain

Core model must support applicable KNX concepts including:

* Projects and installations
* Buildings, floors and rooms
* Areas and lines
* Devices
* Individual/physical addresses
* Group addresses and structures
* Communication objects
* Datapoint Types (DPT)
* Parameters
* Application programs
* Manufacturers and products
* Product/application versions
* Connections
* KNX topology

Domain model must be versioned and migratable.

---

## Data Integrity & Compatibility

Data integrity is critical.

Never silently discard information.

For unsupported or unknown information:

* preserve it where technically possible
* otherwise explicitly mark it unsupported
* report it through appropriate diagnostics
* include it in compatibility/import reports where applicable

Import should provide:

* format/version detection
* validation
* warnings/errors
* compatibility analysis
* mapping information
* loss/unsupported-data reporting

Implement roundtrip and regression tests wherever possible.

Never claim compatibility unless verified.

---

## Manufacturer Data

Manufacturer/product data must use a dedicated, versioned data layer.

Expected hierarchy where applicable:

```text
Manufacturer
  → Product
  → Application Program
  → Version
  → Parameters
  → Communication Objects
  → DPTs
```

Do not hard-code manufacturer-specific products into application logic.

Keep manufacturer data separable from application code.

---

## KNXnet/IP

Keep protocol communication separated from UI.

Use dedicated service/adapter layers for:

* discovery
* tunneling
* routing
* bus communication
* diagnostics
* connection management

Implement protocol behavior only when technically verified.

---

## UI / UX

KNXBench is Linux-first.

UX should resemble a modern professional engineering application rather than legacy desktop software.

Prefer consistent use of:

* Project Explorer
* navigation/sidebar
* tabs
* Properties Inspector
* search
* Command Palette
* context menus
* Drag & Drop
* inline editing
* keyboard shortcuts
* Dark/Light mode
* clear validation and error feedback

Complex KNX structures must remain understandable.

Do not solve domain/application problems with UI workarounds.

---

## Development

Work incrementally.

For significant changes:

1. Understand existing implementation.
2. Define required change.
3. Check architectural impact.
4. Implement smallest clean solution.
5. Add or update tests.
6. Run relevant tests.
7. Update affected documentation.
8. Review for regressions.

Keep repository buildable.

Do not perform unrelated refactors while implementing a feature.

---

## Testing

Tests are mandatory for core functionality.

Prioritize:

* unit tests
* integration tests
* import/export tests
* roundtrip tests
* migration tests
* compatibility tests
* regression tests

Explicitly test relevant edge cases such as:

* malformed input
* unsupported data
* duplicate objects
* invalid addresses
* incompatible device/application versions
* large projects

---

## Code Quality

Prefer:

* simple solutions
* explicit behavior
* strong typing
* small focused modules
* clear interfaces
* dependency inversion where useful
* deterministic behavior
* meaningful error handling

Avoid:

* speculative abstractions
* global state
* duplicated logic
* magic constants
* unnecessary dependencies
* giant classes/modules
* UI/domain coupling

Optimize performance only for measured bottlenecks.

---

## Decision Making

Make reasonable technical decisions autonomously.

Always:

* distinguish verified facts from assumptions
* expose uncertainty
* prefer documented evidence
* verify compatibility claims
* prefer maintainability over cleverness
* prefer data integrity over convenience
* prefer simpler solutions unless complexity has measurable benefit

---

## Git

Make focused commits.

Do not mix unrelated changes.

Use concise commit messages describing actual change. A little humor is welcome where appropriate.

Commit using:

```text
KNXBench <github@knxbench.com>
```

Never add:

```text
Co-Authored-By:
```

---

## Completion Criteria

Before considering significant work complete:

* relevant tests pass
* build remains functional
* implementation status is updated when applicable
* important discoveries are documented
* known limitations are documented
* roadmap is updated when scope/status materially changes

Agent identity, responsibilities, model selection, reasoning effort, task assignment, orchestration and cross-agent memory are managed externally by **Paperclip and Hermes** and must not be duplicated here.