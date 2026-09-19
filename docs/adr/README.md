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
| [0011](0011-product-database-storage.md) | Product database storage — blobs and parsed tables, content hash as identity | Accepted | 2026-09-03 |
| [0012](0012-enrichment-into-absent-slots.md) | Enrichment fills only `Override::Absent` slots | Accepted | 2026-09-03 |
| [0013](0013-module-instance-representation.md) | `ModuleInstance` is a first-class entity; its arguments stay uninterpreted | Accepted | 2026-09-06 |
| [0014](0014-group-object-tree-authoritative-source.md) | `GroupObjectTree` is the authoritative communication-object list for schema ≥ 21 | Accepted | 2026-09-06 |
| [0015](0015-native-output-drops-ets-reimport-goal.md) | Native output format drops the ETS-reimport goal | Accepted | 2026-09-08 |
| [0016](0016-dpt-codec-in-knx-core.md) | The DPT codec lives in `knx-core`, and `GroupValue` moves down into it | Accepted | 2026-09-11 |
| [0017](0017-knx-server-depends-on-knx-net.md) | `knx-server` depends on `knx-net` directly, no crate interposed | Accepted | 2026-09-11 |
| [0018](0018-program-versions-and-file-headers.md) | Programs carry SemVer pre-release versions; files carry a one-sentence header and no version | Accepted | 2026-09-12 |
| [0019](0019-building-model-stays-topological.md) | The building model stays topological — no spatial coordinates in v1.0.0 | Accepted | 2026-09-13 |
| [0020](0020-migrations-may-rederive-from-stored-bytes.md) | A product-database migration may re-derive what the stored bytes determine, and must not invent what only the install knew | Accepted | 2026-09-14 |
| [0021](0021-appimage-is-the-first-linux-package.md) | AppImage is the first Linux package | Accepted | 2026-09-17 |
| [0022](0022-theme-token-boundary.md) | A theme owns the palette tokens; the settings own the rest | Accepted | 2026-09-19 |
| [0023](0023-load-progress-operation.md) | A project load is one server-side operation, and the browser polls its phase | Accepted | 2026-09-19 |
