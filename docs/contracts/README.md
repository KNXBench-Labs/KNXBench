# Contracts

Focused behavioural contracts that code and tests are held to. Each one is
maintained: change it in the same commit as the behaviour it describes.
Moved here from `docs/` on 2026-10-08 (relative links adjusted by script).

| Contract | Scope |
| --- | --- |
| [communication-object-folders](communication-object-folders.md) | WP-06A saved project-tree retention and device identity versus evaluated Dynamic channels; no folder implementation |
| [parameter-number-bound-admission](parameter-number-bound-admission.md) | A declared TypeNumber bound the image builder cannot read refuses the value by name; absent bounds unchanged; not ETS schema validation |
| [parameter-workspace](parameter-workspace.md) | Source-group navigation, compact rows, retained editors, search and metadata; no inferred channel/grid mapping |
| [post-download-check](post-download-check.md) | Read-only check after a successful regular memory download: shared record for CLI, server status, report and history; absent means not checked |
| [hardware-identity-comparison](hardware-identity-comparison.md) | Exact hardware-type response width before regular download/restore mutation; padded product comparisons retained |
| [partial-identity-comparison](partial-identity-comparison.md) | Exact five-octet application identity (object 3, PID 13) before partial-download mutation; other objects generic |
| [backup-application-identity](backup-application-identity.md) | Captured pre-write application identity and restore refusal; strict backup format 2 with read-only format-1 compatibility and explicit downgrade limits |
| [memory-region-read-service](memory-region-read-service.md) | Region-bound memory read service and user-memory count limit for backup, comparison and probes |
| [memory-response-service-pairing](memory-response-service-pairing.md) | A memory read or active-Verify write is answered only by a response of its own service at its address; other-service response is no answer (synthetic nonconforming peer) |
| [load-control-response-state](load-control-response-state.md) | Write answer of a load-control property write is the first state observation; validated before polling, seeds the wait only when changed |
| [device-comparison](device-comparison.md) | P4 bound read-only comparison, volatile tags, decoder/raw integrity and staged parameter scope |
| [IMPORT_BOUNDARY_CONTRACT](IMPORT_BOUNDARY_CONTRACT.md) | AR06 project-import boundary: namespaces, project vs. product-package evidence |
| [MANUFACTURER_REPORT_CONTRACT](MANUFACTURER_REPORT_CONTRACT.md) | AR05 manufacturer-report omissions: design and evidence |
| [PARAMETER_SEMANTICS_BOUNDARY](PARAMETER_SEMANTICS_BOUNDARY.md) | AR07 supported parameter semantics and their research boundary |
| [STORAGE_COMMAND_CONTRACT](STORAGE_COMMAND_CONTRACT.md) | Which project commands persist how (save, expected-saved revision) |
| [PRODUCT_ZIP_COUNT_CONTRACTS](PRODUCT_ZIP_COUNT_CONTRACTS.md) | Enforced product-ZIP entry-count bounds (AR06V) |
| [PRODUCT_ZIP_DECLARED_SIZE_CONTRACTS](PRODUCT_ZIP_DECLARED_SIZE_CONTRACTS.md) | Declared product-ZIP size admission (AR06W) |
| [PRODUCT_ZIP_RAW_INPUT_CONTRACTS](PRODUCT_ZIP_RAW_INPUT_CONTRACTS.md) | Raw-input admission before ZIP parsing (AR06X) |
| [PRODUCT_ZIP_LARGE_PROFILE](PRODUCT_ZIP_LARGE_PROFILE.md) | Measured profile behind the large-package CLI opt-in (KL-151, ADR-0082) |
| [SERVICE_CONTROL_BACKUP](SERVICE_CONTROL_BACKUP.md) | Property-only service-control backup format |
| [COMMISSIONING_ACTIVITY_HISTORY](COMMISSIONING_ACTIVITY_HISTORY.md) | Durable commissioning activity history |
| [table-column-widths](table-column-widths.md) | UI-05 browser-local validated column widths, accessible resizing, reset and explicit table coverage |
| [controls-and-separators](controls-and-separators.md) | UI-02 shared button variants (secondary default, primary, quiet/icon, destructive), their tokens and states; navigation separators visible at rest; routine instance source label hidden |
| [file-menu](file-menu.md) | UI-03 File menu: History, Help and Import / Export as animated submenus opening to the right; ARIA menu roles, keyboard and hover rules, flip and narrow inline expansion, motion |
| [task-status-shimmer](task-status-shimmer.md) | Slow readable text shimmer for active work; sweep/pause, state and accessibility boundaries |
| [RELEASE_NOTIFICATIONS](RELEASE_NOTIFICATIONS.md) | Published GitHub release discovery, semantic version admission, notification-only UX and network/privacy bounds |
| [project-functions](project-functions.md) | Project Functions (ADR-0124): model, invariant, commands, refusals, storage, import, projection, HTTP API, and what a workbench must keep (Goal A4 → B1) |
| [revision-bound-writes](revision-bound-writes.md) | Parameter writes and installation renames bound to the caller's server/project/snapshot and expected value; typed conflicts, legacy unbound form (Goal A2) |
| [device-diagnostic-export](device-diagnostic-export.md) | One-device read-only generic identity export (descriptor, PID12, PID78), loss-aware results, verified create-only ZIP; no memory, no programming unlock (ADR-0134) |
| [universal-device-support](universal-device-support.md) | Unknown/unresolved/unsupported device evidence, fixed read-only profiles, disclosure/IP consent and local ZIP; no programming unlock |
| [download-settlement-source-freshness](download-settlement-source-freshness.md) | A finished download makes memory parts programmed only while the product source still plans the same bytes; a source-bound snapshot at settlement (ADR-0129) |
| [download-verification-reports](download-verification-reports.md) | Tester download reports (format 1), what they never contain, and the maintainer review that ends in `verified_downloads.json` (ADR-0121) |
