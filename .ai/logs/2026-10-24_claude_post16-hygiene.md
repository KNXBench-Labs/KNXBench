# Post-Alpha-16 hygiene (Claude, 24 October 2026)

## Read-only device planners
`apps/knx-cli`: `device download`, `compare`, `restore`, `byte-order-probe`
and `device diagnostics --product-db` only plan from the catalogue but used
`open_products_db` (`open_and_migrate`). An older default product database
was migrated in place by a mere plan, locking older builds out. They now use
`open_products_db_for_reading` like the `products` readers. Regression:
`cli_native_readers_read_only::product_database_readers_leave_the_default_product_database_untouched`
gains the four device commands (Older/Current/Missing default DB). RED on the
unchanged main.rs at `device download`; GREEN after. Writers (`products
ingest`, legacy import) still migrate.

## OPEN_WORK reconciliation
Delegated (Claude Opus subagent, docs-only) and parent-reviewed: dated
post-Alpha-16 paragraph, queue cells with delivering commits and bounds,
stale current-tense blocks relabelled as historical. Documentation and
anchor checks pass.

## Paused retained-COMMIT branch
The paused WP-04.3 retained-COMMIT worktree was completed locally (trace
around COMMIT, TailGuard, review by a Claude subagent: ACCEPT, IMPORTANT
COMMIT-failure assertion added), then found superseded: origin/main already
delivered the same witness/trace through the shared server arbitration
(e287f2050 ff.) with a different tail design (no fallible call after the
clean disarm). Not merged; retained as
`refs/cleanup-retained/wp043-retained-commit-claude-20261024`.

## Gate
Change-scoped `tools/run_package_gate.py` on 4b2389269: 14/14, 1755 Rust
passes/77 ignored, corpus 68/68, inputs frozen, user-data guards empty.
