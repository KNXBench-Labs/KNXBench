# CLI missing-destination parent aliases

Timestamp: 2026-10-05 08:58

Self-review IMPORTANT: activity_history_paths.rs compared identical requested strings, then returned on a missing history file. Existing parent traversal or a directory symlink could disguise the same missing product/history destination. The named parent-alias entrypoint regression compiled and failed at the preserved creation assertion before the correction.

Resolve existing parents without creating paths before comparing requested destinations. Keep existing canonical and Unix hard-link identity checks and fail on non-NotFound metadata errors. This is admission-time protection, not hostile replacement/race confinement.

Focused GREEN: compare units6, entrypoints7, readers7, strict CLI Clippy and fmt. Six controls compiled, registered and failed at intended runtime assertions; canonical bytes restored and positive suites passed. Receipt: [parent alias evidence](../../docs/evidence/cli-history-parent-alias-offline-2026-10-05.json).

The original fresh workspace3236/0/177/179-block receipt belongs to f7cb57ed before this change. Renewed integration/browser/private/release gates and publication PENDING. No hardware, Web edits, independent or whole-track acceptance.
