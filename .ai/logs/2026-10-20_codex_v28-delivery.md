# Product-column promotion repair — source delivery

- Agent: codex
- Date: 20 October 2026
- Published migration source and final delta acceptance: `8fa0d76b9fd41f81c4e135c29e54f2a98d516b38`.
- Publication readback: local HEAD, tracking `origin/main` and live `refs/heads/main` were equal to that source.
- Final maintained delta: 11/11 parent and 29/29 web stages, web build, 22 derived typecheck configurations, 4,136 Vitest and 863 default-browser cases. No production-browser suite was selected.
- Retained integration: `0be962f9653805d073e8644bde41deee8165e143`, 14/14 maintained stages, 3,162 Rust passes, zero failures, 144 ignored, 97/97 selected corpus. Byte-equal Rust/Cargo/xtask/Rust-app inputs justify carry-forward; these are not new Rust/corpus executions on the final delta.
- The independently authenticated private fresh/migrated comparison covers 46 tables and 7,422,273 rows per side under the sole `ingest_unknown.id` normalization; 21,700 unchanged unknown-row IDs remain stable. This is not byte-identical database equality, universal historical reconstruction or unrestricted primary-key stability.
- Current source: product28, unchanged native/model16. Writable open upgrades; read-only readers migrate a copy only. Older-reader refusal is source-established, not an executed downgrade test. Keep independent pre-upgrade backups; no downgrade is supplied.
- Result-only reconciliation covers the eleven planned documents, preserving the earlier reviewed-candidate checkpoint, release history, aliases and retained catalogue blocks.
- Next CLAIM: WP-04.1 measurement only, one `tools/measure_productdb` module with focused tests, init/resolve example and testing-method documentation. No measurement has run and no installer/parser change is claimed. WP-04 remains open; WP-04.2/.3 and WP-19 remain later packages.
- This tail is not committed or published by its author; parent acceptance/publication and private archival/owned retirement remain pending. Cleanup is not complete. No new release artifact, compatibility, hardware, UI, bus/device, deployment or vendor claim.

[Definitive source-delivery record](../../docs/IMPLEMENTATION_STATUS.md#product-column-promotion-repair--source-delivery).
