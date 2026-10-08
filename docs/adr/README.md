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
| [0007](0007-roundtrip-fidelity.md) | Roundtrip fidelity definition | Superseded by [ADR-0028](0028-no-knxproj-export.md) | 2026-09-02 |
| [0008](0008-key-material-isolation.md) | Key material isolation | Accepted | 2026-09-02 |
| [0009](0009-ui-boundary.md) | UI boundary via generated projections | Accepted | 2026-09-02 |
| [0010](0010-per-attribute-override-representation.md) | Overrides are represented per attribute with an explicit empty state | Accepted | 2026-09-02 |
| [0011](0011-product-database-storage.md) | Product database storage — blobs and parsed tables, content hash as identity | Accepted | 2026-09-03 |
| [0012](0012-enrichment-into-absent-slots.md) | Enrichment fills only `Override::Absent` slots | Accepted | 2026-09-03 |
| [0013](0013-module-instance-representation.md) | `ModuleInstance` is a first-class entity; its arguments stay uninterpreted | Accepted | 2026-09-06 |
| [0014](0014-group-object-tree-authoritative-source.md) | `GroupObjectTree` is the authoritative communication-object list for schema ≥ 21 | Accepted | 2026-09-06 |
| [0015](0015-native-output-drops-ets-reimport-goal.md) | Native output format drops the ETS-reimport goal | Superseded by [ADR-0028](0028-no-knxproj-export.md) | 2026-09-08 |
| [0016](0016-dpt-codec-in-knx-core.md) | The DPT codec lives in `knx-core`, and `GroupValue` moves down into it | Accepted | 2026-09-11 |
| [0017](0017-knx-server-depends-on-knx-net.md) | `knx-server` depends on `knx-net` directly, no crate interposed | Accepted | 2026-09-11 |
| [0018](0018-program-versions-and-file-headers.md) | Programs carry SemVer pre-release versions; files carry a one-sentence header and no version | Accepted | 2026-09-12 |
| [0019](0019-building-model-stays-topological.md) | The building model stays topological — no spatial coordinates in v1.0.0 | Accepted | 2026-09-13 |
| [0020](0020-migrations-may-rederive-from-stored-bytes.md) | A product-database migration may re-derive what the stored bytes determine, and must not invent what only the install knew | Accepted | 2026-09-14 |
| [0021](0021-appimage-is-the-first-linux-package.md) | AppImage is the first Linux package | Accepted | 2026-09-17 |
| [0022](0022-theme-token-boundary.md) | A theme owns the palette tokens; the settings own the rest | Accepted | 2026-09-19 |
| [0023](0023-load-progress-operation.md) | A project load is one server-side operation, and the browser polls its phase | Accepted | 2026-09-19 |
| [0024](0024-in-application-help.md) | Help is a tip and a panel, its text is an ordinary catalogue key, and `docs/` never ships | Accepted | 2026-09-19 |
| [0025](0025-extension-is-data-not-code.md) | Extension is data, not code — the plugin API stays unwritten | Accepted | 2026-09-20 |
| [0026](0026-server-authentication-or-loopback.md) | `knx-server` authenticates, or it binds loopback and nothing else | Accepted | 2026-09-20 |
| [0027](0027-program-defaults-side-table.md) | A program-defaults side table, not a layer stack on `Override<T>` | Accepted | 2026-09-20 |
| [0028](0028-no-knxproj-export.md) | KNXBench reads `.knxproj` and never writes one | Accepted | 2026-09-20 |
| [0029](0029-application-settings-file.md) | Application settings live in one versioned file on the server | Accepted | 2026-09-21 |
| [0030](0030-group-address-notation-is-a-display-preference.md) | Group-address notation is a display preference, rendered last | Accepted | 2026-09-21 |
| [0031](0031-project-notes-are-a-project-owned-collection.md) | Project notes are a project-owned collection | Accepted | 2026-09-22 |
| [0032](0032-application-snapshot-ordering.md) | Order project snapshots at the application boundary | Accepted | 2026-09-23 |
| [0033](0033-destructive-csv-imports-require-bound-confirmation.md) | Destructive CSV imports require revision-bound confirmation | Accepted | 2026-09-23 |
| [0034](0034-zip-member-names-follow-declared-encoding.md) | ZIP member names follow their declared encoding before safety checks | Accepted | 2026-09-23 |
| [0038](0038-site-is-a-ground-root-space.md) | A site is a `Ground` space at the root of the building structure — no new kind, no new level | Proposed | 2026-09-26 |
| [0039](0039-project-mutation-goes-through-commands.md) | A live project changes only through `Command::apply`, and ids are reserved by a command that never rewinds | Accepted | 2026-09-26 |
| [0040](0040-programming-requires-release-stage-consent.md) | Programming a device needs a release-stage-aware consent, rememberable per stage | Accepted | 2026-09-27 |
| [0041](0041-unmodelled-kinds-and-dynamic-nodes-are-named-never-hidden.md) | Unmodelled parameter kinds and Dynamic nodes are named, never hidden | Accepted | 2026-09-27 |
| [0042](0042-baggage-is-inventoried-by-content-and-resolved-exactly.md) | Baggage is inventoried by content and resolved exactly, never opened | Accepted | 2026-09-28 |
| [0043](0043-package-identity-is-recorded-per-candidate.md) | Package identity is recorded per candidate, never decided by a new winner rule | Accepted | 2026-09-28 |
| [0044](0044-download-data-is-read-from-the-stored-product-file.md) | An application's download data is read on demand from the stored product file | Accepted | 2026-09-28 |
| [0045](0045-device-download-route-demands-phrase-and-plan.md) | The device-download route demands the device's confirmation phrase and the exact plan the user saw | Accepted | 2026-09-28 |
| [0046](0046-address-programming-route-one-phrase-stoppable-wait.md) | Address programming from the web: one phrase covers write and restart, and the button wait can be stopped | Accepted | 2026-09-28 |
| [0048](0048-commissioning-v1-scope-is-the-verified-memory-path.md) | Commissioning v1 is the verified memory path for mask 070nh; everything else is refused by name | Accepted (decision 5 superseded by ADR-0086) | 2026-09-28 |
| [0050](0050-com-object-activation-is-evaluated-and-four-valued.md) | A communication object's evaluated activation is four-valued and separate from the stored `is_active` | Accepted | 2026-09-29 |
| [0051](0051-individual-address-write-enable-is-opt-in-debug.md) | Individual Address Write Enable is an opt-in debug action, never automatic | Accepted | 2026-09-30 |
| [0053](0053-contributions-come-with-a-license-grant-for-dual-licensing.md) | Contributions come with a license grant, so KNXBench can be dual-licensed | Superseded by ADR-0054 | 2026-09-30 |
| [0054](0054-no-contributor-license-agreement-agpl-only.md) | No contributor license agreement; contributions come in under the AGPL alone | Accepted | 2026-09-30 |
| [0059](0059-button-address-programming-requires-durable-recovery.md) | Button-based address programming needs durable pre-write recovery | Accepted | 2026-10-01 |
| [0062](0062-dynamic-evaluation-work-admission.md) | Dynamic evaluation admits repeated work before performing it | Accepted (bounded contract) | 2026-10-03 |
| [0063](0063-parameter-scopes-preserve-evaluation-identity.md) | Parameter scopes preserve the full evaluation identity | Accepted (bounded backend contract) | 2026-10-03 |
| [0064](0064-durable-activity-history-is-not-recovery.md) | Activity history is durable metadata, not recovery or bus proof | Accepted (bounded backend contract) | 2026-10-02 |
| [0065](0065-dynamic-scalar-copy-admission.md) | Dynamic evaluation admits scalar copies before allocating them | Accepted (bounded Core/HTTP scalar admission; external projections open) | 2026-10-03 |
| [0066](0066-outside-walk-text-refusal.md) | Outside-walk text substitution has explicit request-level refusal | Proposed (source audit and first public HTTP tracer; execution pending) | 2026-10-03 |
| [0067](0067-download-lifecycle-preserves-uncertainty.md) | Download lifecycle receipts preserve uncertainty and legacy history | Proposed (scoped integrated offline gates passed; owner admission pending) | 2026-10-03 |
| [0068](0068-project-evolution-story-is-a-static-offline-companion.md) | The project-evolution story is a static, offline companion with a separate publication gate | Accepted (first private version; publication not designed) | 2026-10-04 |
| [0069](0069-catalog-batch-request-replay-token.md) | A catalog batch request may carry a replay token | Accepted (server half; web client pending) | 2026-10-04 |
| [0070](0070-commands-act-in-the-owning-installation.md) | Commands act in the installation that owns their target | Accepted (core/server; web UI pending) | 2026-10-04 |
| [0071](0071-ambiguous-topology-is-repaired-explicitly.md) | Ambiguous topology is repaired explicitly, never collapsed on save | Accepted (core/store/server; web UI pending) | 2026-10-04 |
| [0072](0072-product-scheme23-namespace-gate.md) | Admit exact product scheme 23 through the existing strict package adapter | Accepted implementation decision; candidate acceptance/publication tracked separately | 2026-10-04 |
| [0073](0073-imported-elements-keep-their-own-ids.md) | Imported elements keep their own ids; ambiguous references are not guessed | Accepted | 2026-10-04 |
| [0074](0074-native-save-is-exact-or-refused.md) | A native save is exact or refused | Accepted | 2026-10-04 |
| [0075](0075-shared-commissioning-activity-lifecycle.md) | Commissioning callers share an application-layer activity lifecycle | Proposed (integrated runtime acceptance pending) | 2026-10-04 |
| [0076](0076-one-ledger-is-the-status-of-record.md) | One ledger is the status of record for tracked source IDs | Accepted | 2026-10-04 |
| [0077](0077-session-local-telegram-flow-view.md) | A session-local telegram-flow view is not physical topology | Accepted (implementation pending) | 2026-10-04 |
| [0078](0078-group-address-declared-dpt.md) | A group address keeps its declared DPT; resolution reports it beside the linked objects | Accepted | 2026-10-05 |
| [0079](0079-theme-choice-is-one-dropdown.md) | Theme choice is one dropdown; shipped packs are data; Neon Grid and Bitcoin DeFi retired | Accepted | 2026-10-05 |
| [0080](0080-parameter-write-authority.md) | Parameter writes honour Access and leave manufacturer calculations alone | Accepted | 2026-10-05 |
| [0081](0081-parameter-attributes-are-reported.md) | `Parameter` and `ParameterRef` report every attribute they do not store | Accepted | 2026-10-05 |
| [0082](0082-large-product-packages-are-a-cli-opt-in.md) | Large product packages are an explicit command-line opt-in | Accepted | 2026-10-05 |
| [0083](0083-admit-exact-product-scheme-10.md) | Admit exact product scheme 10 through the strict namespace path | Accepted | 2026-10-05 |
| [0084](0084-first-run-guide-once-per-release-stage.md) | A first-run guide opens once per release stage and gates nothing | Accepted | 2026-10-07 |
| [0085](0085-readable-flow-and-source-windows.md) | Readability-first flow layout and source-bound secondary windows | Accepted | 2026-10-07 |
| [0086](0086-specs-product-data-and-projects-are-enough-evidence.md) | The specification, product data and project files are enough evidence; a working inference is shipped and disclosed | Accepted | 2026-10-07 |
| [0087](0087-desktop-app-identifier.md) | The desktop app identifier is `com.knxbench.knxbench-labs` | Accepted | 2026-10-07 |
| [0088](0088-server-terminates-tls-itself.md) | `knx-server` terminates TLS itself, with a self-signed certificate by default | Accepted | 2026-10-07 |
| [0089](0089-achievements.md) | Achievements are a frontend catalogue over a grow-only server record | Accepted | 2026-10-07 |
| [0090](0090-read-only-mcp-adapter.md) | A read-only, stdio-only MCP adapter over saved project files | Accepted | 2026-10-07 |
| [0093](0093-wizards-are-views-over-existing-commands.md) | Wizards are views over existing commands; a new project's structure is seeded atomically | Accepted (both wizards implemented) | 2026-10-08 |
| [0094](0094-legacy-exim-product-files.md) | Legacy EX-IM product files get a separate, content-detected path with a user-supplied password | Accepted | 2026-10-08 |
