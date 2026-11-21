# PRG-01-N14 acceptance — 21 November 2026

Scoping of the whole-mutation recovery axis ([research](../research/2026-11-21-prg01-n14-recovery-axis.md), [contract](../contracts/prg01-whole-mutation-recovery-scope.md), [ADR-0188](../adr/0188-prg01-whole-mutation-recovery-scope.md)). Inventory: 7 items already guaranteed and pinned, 1 closable now, 4 needing evidence the admitted class cannot supply.

Code: two tests in `knx-core` `commissioning/device_backup.rs` (exhaustive per-step restore-coverage table; a fixed property write is replayed as planned). Characterization/drift pins: they pass on current behaviour and are not a RED; no runtime, consumer or hardware change. Simulator/offline evidence only; no bus, device, VM or hardware contact. `cargo test -p knx-core --lib device_backup`: 25 passed.

Gate outcome: see the PR description (package gate receipt). PRG-01 stays open; whole-mutation recovery stays OPEN.

## Gate (partial)

`python3 tools/run_package_gate.py --base origin/main` (change-scoped): fmt, documentation, browser-lanes, check-layering, check-headers, check-anchors, check-ledger, check-corpus-gates, tool-tests, clean-desktop and clippy exited 0. `rust-tests` exited 101 and the gate stopped there, so bindings, bindings-diff, corpus and diff-check did not run: this is a **partial** result.

- `c49_partial_group_addresses::the_49_report_plan_reproduces_offline`: known pre-existing environmental failure on this host.
- `knx-cli` `cli_bus_tcp::system2_validation::admission::tcp_connection_refusal_never_falls_back_and_udp_controls_are_handshake_only` panicked once with `AddrInUse` in the test peer helper. Isolated reruns (3 of 3) passed, so it is recorded as a transient port-race flake, not fixed here; candidate follow-up: let the probe peer bind an ephemeral port.
