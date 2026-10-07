# Commissioning: plan-scoped restore load-state coverage — 2026-10-01

Agent: codex. Scope: offline K7/download backup integrity only (`goal-commission.md`); isolated `iaw-load-state-coverage` worktree. Root checkout and Web source untouched; no gateway, tunnel, key, live target or hardware write.

## Finding and fix

`restore_plan` previously checked target, mask/manufacturer and the lengths/addresses of saved memory regions, but did not verify the saved load-state machine list against the original plan. A backup file with one machine's state omitted, duplicated or supplied for an unrelated machine could still yield a restore plan with matching memory regions. `prepare_restore` rejects non-Loaded states but its `was_loaded` check alone does not identify a *missing* Loaded machine when other records remain.

The core restore now compares `machines(plan)` (sorted, deduplicated from the plan's load records) with a sorted list of *all* saved load-state machine names (duplicates retained) and rejects mismatches as `RestoreError::OtherLoadStates`. Existing read-only `download_changes` is unchanged. The positive restore still follows the original plan and backed-up memory, and CLI restore continues to call `prepare_restore` before the write path. No serializer format changed and no address-write gate was lifted.

## Evidence

- RED before implementation: two focused core refusal tests failed against the permissive `restore_plan` (missing state, duplicate state). After implementation: three focused tests for missing, duplicate and extra state passed. A temporary `false &&` bypass of the new comparison made all three fail (exit 101); the guard was restored and the mutation was not committed.
- Core backup tests: 11 passed. App backup tests passed. The existing explicitly ignored CLI simulator/product-project fixture regression ran 1/1 with explicit local private-corpus inputs and no KNX gateway environment. It now also removes one recorded load state and expects `OtherLoadStates`; the complete backup still produces a restore plan and restores the old memory in the simulator. No private bytes/names were added to the repo or this log.
- On the isolated worktree with freshly built Web assets: `cargo fmt --all -- --check`, strict `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --no-fail-fast` (139 suites, 2,798 passed, zero failed, zero `SKIP:`), `xtask` layering/headers/anchors/corpus-gates (375 links over 225 Markdown files) and `git diff --check` all passed. Static added-line security scan found zero hits. The post-rebase anchor gate also passed.
- Code/docs commit `521ad13c10638aeec33cc476d32764d12e7c4d4c` was published to `origin/main` and exact remote SHA readback matched. Only the task's temporary symlinks and scratch artifacts remain for cleanup after handover delivery.

## Boundary and next work

This is stricter validation of an *existing plan-scoped* memory backup; it does not capture additional memory/properties, identify `1.1.32`'s exact installed application, prove hardware restoration, or satisfy ADR-0057/0058/0059 durable recovery. Public confirmed K6, serial-address and K13 reset entries remain pre-tunnel fail-closed. Next independently actionable commissioning work should examine the pre-write persistence/readback contract for already-supported download paths or continue other offline safety evidence; no speculative whole-device restore or key guess. Respect the shared Git-common-dir tunnel lock before *any* later live read and require a fresh per-target/per-operation go before any write.
