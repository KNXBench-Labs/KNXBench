# Architecture decision records

An ADR here records a decision that has already been made and approved,
together with the evidence that forced it. The point is not to justify the
decision after the fact but to make it re-openable: a later session that
disagrees can go back to the cited measurement, licence or standard and check
whether it still holds, instead of re-arguing from memory.

New records follow [template.md](template.md) and take the next free number.
An ADR is never edited to reverse its decision — a new ADR supersedes it, and
the old one gets `Status: Superseded by ADR-NNNN`.

| ADR | Title | Status | Date |
| --- | --- | --- | --- |
| [0001](0001-technology-stack.md) | Technology stack — Rust core, Tauri, React, SQLite | Accepted | 2026-09-02 |
| [0002](0002-own-knxproj-parser.md) | Own `.knxproj` parser; `xknxproject` as a test oracle only | Accepted | 2026-09-02 |
| [0003](0003-sqlite-project-format.md) | SQLite as the native project format | Accepted | 2026-09-02 |
| [0004](0004-provenance-model.md) | Provenance and override-chain model | Accepted | 2026-09-02 |
| [0005](0005-separate-product-database.md) | Separate, shared product database | Accepted | 2026-09-02 |
| [0006](0006-opaque-passthrough-store.md) | Opaque passthrough store | Accepted | 2026-09-02 |
| [0007](0007-roundtrip-fidelity.md) | Roundtrip fidelity definition | Accepted | 2026-09-02 |
| [0008](0008-key-material-isolation.md) | Key material isolation | Accepted | 2026-09-02 |
| [0009](0009-ui-boundary.md) | UI boundary via generated projections | Accepted | 2026-09-02 |
| [0010](0010-per-attribute-override-representation.md) | Overrides are represented per attribute with an explicit empty state | Accepted | 2026-09-02 |
