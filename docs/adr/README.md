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
| [0035](0035-product-package-install-evidence.md) | Persist product-package install evidence | Not stated in the ADR (added with its implementation, `991824ff`) | 2026-09-24 |
| [0036](0036-scheme21-namespace-gate.md) | Scheme-21 package XML fails closed on foreign namespaces | Accepted | 2026-09-24 |
| [0037](0037-product-program-catalog-metadata.md) | Preserve application-program catalogue metadata as source values | Accepted (PDB-7) | 2026-09-24 |
| [0038](0038-site-is-a-ground-root-space.md) | A site is a `Ground` space at the root of the building structure — no new kind, no new level | Proposed | 2026-09-26 |
| [0039](0039-project-mutation-goes-through-commands.md) | A live project changes only through `Command::apply`, and ids are reserved by a command that never rewinds | Accepted | 2026-09-26 |
| [0040](0040-programming-requires-release-stage-consent.md) | Programming a device needs a release-stage-aware consent, rememberable per stage | Accepted | 2026-09-27 |
| [0041](0041-unmodelled-kinds-and-dynamic-nodes-are-named-never-hidden.md) | Unmodelled parameter kinds and Dynamic nodes are named, never hidden | Accepted | 2026-09-27 |
| [0042](0042-baggage-is-inventoried-by-content-and-resolved-exactly.md) | Baggage is inventoried by content and resolved exactly, never opened | Accepted | 2026-09-28 |
| [0043](0043-package-identity-is-recorded-per-candidate.md) | Package identity is recorded per candidate, never decided by a new winner rule | Accepted | 2026-09-28 |
| [0044](0044-download-data-is-read-from-the-stored-product-file.md) | An application's download data is read on demand from the stored product file | Accepted | 2026-09-28 |
| [0045](0045-device-download-route-demands-phrase-and-plan.md) | The device-download route demands the device's confirmation phrase and the exact plan the user saw | Accepted | 2026-09-28 |
| [0046](0046-address-programming-route-one-phrase-stoppable-wait.md) | Address programming from the web: one phrase covers write and restart, and the button wait can be stopped | Accepted | 2026-09-28 |
| [0047](0047-session-log-export-is-a-versioned-local-json-snapshot.md) | Session-log export is a versioned local JSON snapshot | Accepted | 2026-09-28 |
| [0048](0048-commissioning-v1-scope-is-the-verified-memory-path.md) | Commissioning v1 is the verified memory path for mask 070nh; everything else is refused by name | Accepted (decision 5 superseded by ADR-0086) | 2026-09-28 |
| [0049](0049-download-readiness-is-per-plan-and-backups-are-pre-write.md) | Download readiness is evidence-scoped; pre-write backups fail closed | Accepted | 2026-09-29 |
| [0050](0050-com-object-activation-is-evaluated-and-four-valued.md) | A communication object's evaluated activation is four-valued and separate from the stored `is_active` | Accepted | 2026-09-29 |
| [0051](0051-individual-address-write-enable-is-opt-in-debug.md) | Individual Address Write Enable is an opt-in debug action, never automatic | Accepted | 2026-09-30 |
| [0052](0052-channel-name-and-number-are-stored-verbatim.md) | A channel's `@Name` and `@Number` are stored verbatim, and the v18 migration keeps every other unknown row | Accepted | 2026-09-30 |
| [0053](0053-contributions-come-with-a-license-grant-for-dual-licensing.md) | Contributions come with a license grant, so KNXBench can be dual-licensed | Superseded by ADR-0054 | 2026-09-30 |
| [0054](0054-no-contributor-license-agreement-agpl-only.md) | No contributor license agreement; contributions come in under the AGPL alone | Accepted | 2026-09-30 |
| [0055](0055-bus-activity-snapshot-is-partial-session-evidence.md) | The bus-activity snapshot reports only observed sessions | Accepted | 2026-09-30 |
| [0056](0056-write-activity-needs-evidence-states.md) | Write activity needs evidence states, not generic success/failure | Accepted | 2026-09-30 |
| [0057](0057-serial-address-writes-require-durable-recovery.md) | Serial-address writes fail closed until durable recovery is demonstrable | Accepted | 2026-09-30 |
| [0058](0058-individual-address-reset-requires-durable-recovery.md) | Public individual-address reset fails closed without durable recovery | Accepted | 2026-09-30 |
| [0059](0059-button-address-programming-requires-durable-recovery.md) | Button-based address programming needs durable pre-write recovery | Accepted | 2026-10-01 |
| [0060](0060-versioned-declarative-theme-packs.md) | Versioned declarative theme packs, never arbitrary CSS | Accepted; preview and per-theme management superseded by ADR-0079 | 2026-10-02 |
| [0061](0061-unsupported-choose-controller-kinds.md) | Unsupported declared choose controllers fail closed, not as missing references | Accepted (published at `2d9aaeb8`) | 2026-10-02 |
| [0062](0062-dynamic-evaluation-work-admission.md) | Dynamic evaluation admits repeated work before performing it | Accepted (bounded contract) | 2026-10-03 |
| [0063](0063-parameter-scopes-preserve-evaluation-identity.md) | Parameter scopes preserve the full evaluation identity | Accepted (bounded backend contract; web adoption WP-15) | 2026-10-03 |
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
| [0091](0091-community-evidence-analysis.md) | Maintainer-reviewable community evidence, not automatic compatibility | Accepted | 2026-10-08 |
| [0092](0092-lcars-built-in-presentation.md) | LCARS is a built-in palette with a controlled presentation | Accepted (owner-approved study) | 2026-10-08 |
| [0093](0093-wizards-are-views-over-existing-commands.md) | Wizards are views over existing commands; a new project's structure is seeded atomically | Accepted (both wizards implemented) | 2026-10-08 |
| [0094](0094-legacy-exim-product-files.md) | Legacy EX-IM product files get a separate, content-detected path with a user-supplied password | Accepted | 2026-10-08 |
| [0095](0095-static-marketing-companion.md) | Marketing is a static companion, not a hosted engineering application | Accepted (site deployment gated) | 2026-10-08 |
| [0096](0096-devices-navigation-and-catalogue-batch.md) | Devices navigation and a snapshot-bound catalogue batch | Accepted implementation decision; local delivery/publication separate | 2026-10-08 |
| [0097](0097-docker-hub-release-image.md) | Release tags publish a multi-arch server image to Docker Hub | Accepted (first publication needs credentials and the next tag) | 2026-10-09 |
| [0098](0098-offline-ap1-procedure-resolution.md) | Offline AP1 sequence resolution is not download capability | Accepted; on main | 2026-10-09 |
| [0100](0100-persistent-native-project-history.md) | Persistent native undo and project versions | Implemented; on main since 2026-10-09 | 2026-10-09 |
| [0101](0101-project-local-device-and-ga-names.md) | Guarded project-local device and group-address names | Implemented; on main since 2026-10-09 | 2026-10-09 |
| [0102](0102-communication-object-table-view-state.md) | Object editors keep one parent while the table changes its mind | Accepted; on main | 2026-10-09 |
| [0106](0106-selective-project-import.md) | Selective project import into the open project | Accepted; on main since 2026-10-10 | 2026-10-09 |
| [0107](0107-import-source-integrity.md) | Source-bound import integrity and completion vocabulary | Implemented; on main since 2026-10-10 | 2026-10-10 |
| [0108](0108-read-only-module-instance-values.md) | Read stored module-instance values without interpreting RepeatIndex | Accepted (Repeat evaluation research-bound) | 2026-10-10 |
| [0109](0109-opaque-payload-content-and-candidate-roles.md) | Opaque payload content is evidence, not authenticated App identity | Accepted (bounded scope) | 2026-10-10 |
| [0110](0110-retained-raw-header-candidates.md) | Retained raw-header candidates, not RTF interpretation | Accepted, bounded; on main since 2026-10-10 (KL-175 DONE) | 2026-10-10 |
| [0111](0111-bounded-large-history-images.md) | Bounded compression for large native history images | Implemented; on main since 2026-10-11 | 2026-10-11 |
| [0112](0112-native-instruction-overlays.md) | Native instruction overlays keep imported text authoritative | Accepted; on main, editor mounted 2026-10-12 | 2026-10-11 |
| [0113](0113-durable-native-import-evidence.md) | Durable historical import evidence in native passthrough | Accepted (on main 2026-10-12) | 2026-10-11 |
| [0114](0114-client-release-discovery.md) | Client release discovery is notification-only | Accepted implementation decision | 2026-10-11 |
| [0115](0115-offline-application-edit-boundary.md) | Offline application edits live in `knx-edit`, below App | Accepted implementation decision | 2026-10-12 |
| [0116](0116-opt-in-mcp-name-recipe-writer.md) | Opt-in, human-confirmed MCP writer for literal group-address names | Accepted implementation decision | 2026-10-12 |
| [0117](0117-previewed-recoverable-catalogue-updates.md) | Previewed, recoverable catalogue updates from manufacturer files | Accepted implementation decision | 2026-10-12 |
| [0118](0118-catalogue-updates-through-the-web-server.md) | Catalogue updates through the web server | Accepted | 2026-10-12 |
| [0119](0119-read-only-byte-order-probe.md) | A read-only byte-order probe, never an encoder | Accepted implementation decision | 2026-10-13 |
| [0120](0120-base-image-byte-order-warning.md) | A named warning when a big-endian program's base image looks little-endian | Accepted implementation decision | 2026-10-13 |
| [0121](0121-tester-download-reports.md) | Tester download reports, and a maintainer review before "verified" | Accepted implementation decision | 2026-10-13 |
| [0122](0122-category-motion-preferences.md) | Reduction-only motion preferences per category (Toasts, Achievements, Flow) | Accepted (on main 2026-10-13; written by Codex as local 0109/0112) | 2026-10-10 |
| [0123](0123-structured-gateway-refusals.md) | Gateway refusals are structured, and name only what is known | Accepted implementation decision | 2026-10-13 |
| [0124](0124-project-functions.md) | Project Functions are placed in one building part and refer to group addresses of its installation | Accepted implementation decision | 2026-10-13 |
| [0125](0125-multi-installation-site-is-blocked.md) | A site over several installations is not modelled; its invariants are fixed and it waits for evidence | Proposed — blocked design | 2026-10-13 |
| [0127](0127-web-workbench-resumes-the-open-project.md) | The web workbench resumes the server's open project on every mount | Accepted implementation decision | 2026-10-14 |
| [0128](0128-native-project-files-carry-their-manufacturer-files.md) | A native project file carries its manufacturer files | Accepted implementation decision | 2026-10-14 |
| [0129](0129-per-part-pending-programming.md) | Per-part pending programming uses the existing loaded flags | Accepted | 2026-10-14 |
| [0130](0130-session-device-comparison.md) | Device comparison is bound, decoded, session-only evidence | Accepted; package 1 under acceptance | 2026-10-15 |

| [0131](0131-universal-device-support.md) | Universal support evidence is not download admission | Accepted | 2026-10-15 |
| [0133](0133-parameter-edits-reconcile-object-inventory.md) | Parameter edits reconcile the configured object inventory | Accepted for offline editing | 2026-10-16 |
| [0134](0134-generic-read-only-diagnostic-export.md) | Read-only diagnostic export uses one generic identity recipe | Accepted (simulator scope) | 2026-10-17 |
| [0135](0135-repository-carries-no-local-machine-details.md) | The repository carries no local machine details | Accepted | 2026-10-17 |
| [0136](0136-plugin-data-adoption-is-audited-opaque-replacement.md) | Plug-in data adoption is an audited opaque replacement | Accepted (CLI and application layer) | 2026-10-17 |
| [0137](0137-declared-parameter-grids-reach-the-workspace.md) | Declared parameter grids reach the workspace as versioned geometry | Accepted | 2026-10-17 |
| [0138](0138-notes-get-an-explicit-decision-before-a-delete.md) | Notes get an explicit decision before a delete or a selective import | Accepted | 2026-10-17 |
| [0139](0139-block-access-and-separators-reach-the-parameter-tab.md) | Block access and separators reach the parameter tab | Accepted | 2026-10-17 |
| [0140](0140-vendor-zips-of-product-packages-install-as-a-whole.md) | Vendor ZIPs of product packages install as a whole | Accepted | 2026-10-18 |
| [0141](0141-placements-and-enumeration-attributes-reach-columns.md) | Placements and enumeration attributes reach columns; the report names the rest | Accepted | 2026-10-18 |
| [0142](0142-file-size-limits-follow-the-storage-ceiling.md) | File-size limits follow the storage ceiling | Accepted | 2026-10-18 |
| [0143](0143-declared-segment-memory-evidence-profile.md) | A ProductDB-declared, read-only memory evidence profile | Accepted (simulator scope) | 2026-10-18 |
| [0144](0144-change-scoped-gates-tagged-browser-lanes-bundled-rust-tests.md) | Change-scoped gates, tagged browser lanes and bundled Rust tests | Accepted | 2026-10-19 |
| [0145](0145-catalogue-cleanup-removes-placements-only.md) | Catalogue cleanup removes placements only | Accepted | 2026-10-19 |
| [0146](0146-unknown-evidence-rows-have-an-identity.md) | Unknown-evidence rows have an identity; `SuffixText` reaches the parameter row | Accepted | 2026-10-19 |
| [0147](0147-master-identity-provenance.md) | Master declarations have provenance; historical winners are not reconstructed | Accepted | 2026-10-19 |
| [0148](0148-repair-product-column-promotions.md) | Repair released product-column promotions from retained source facts | Accepted for implementation; runtime acceptance pending | 2026-10-20 |
| [0149](0149-source-bound-system2-validation.md) | Isolate the source-bound System-2 validation candidate | Accepted for simulator-only validation | 2026-10-20 |
| [0150](0150-cancellable-catalogue-jobs.md) | Owned cancellable catalogue jobs over the existing installer | Accepted for implementation; runtime acceptance pending | 2026-10-20 |
| [0151](0151-monitoring-export-is-a-sparse-snapshot.md) | Monitoring export is a sparse snapshot with a bound loss report | Accepted for implementation; runtime acceptance pending | 2026-10-20 |
| [0153](0153-regular-download-stop-ends-at-first-change.md) | A regular download may be stopped until its first change, and not after | Accepted for implementation; runtime acceptance pending | 2026-10-25 |
