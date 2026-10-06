# http_device_compare harness fix + UI-04 acceptance (Claude, 2026-10-06)

User decisions (clarify): fix the corpus pair myself, test-only with a
mutant counter-check; accept UI-04 as an Alpha boundary.

Fix: `apps/knx-server/tests/http_device_compare.rs` harness_timed uses
`..knx_server::AppState::new(dir.path().to_path_buf())` instead of
`..Default::default()` (Default swaps in a non-durable OneShotLog, so the
AUDIT-01 download gate answers 503). Same pattern as http_device_download.rs.
Side effect shared with that harness: AppState::new opens the default
product DB path once; the harness then replaces product_db.

Gate (cmp/gate.sh, leases 7/8, offline, OriginalData linked): fmt 0, clippy
-p knx-server 0, compare --include-ignored 8/0, revert mutant 6/2 with both
503 messages, restored by cmp, knx-server 617/0/44, five xtask checks 0,
diff --check 0.
