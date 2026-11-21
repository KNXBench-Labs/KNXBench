# WP-05n ref-bearing axes: verification and C49 corpus-env finding

Date: 2026-11-21

Decision: ADR-0177 (grid-axis captions, `docs/adr/0177-grid-axis-text-parameter-captions.md`;
the number is shared with the WP-07 ADR of a different name). Contracts:
`docs/contracts/parameter-workspace.md`, `docs/contracts/alpha16-parameter-layout.md`.

## Change

`literal_grid_axes` keeps axes declaring `TextParameterRefId` as bound caption hosts; the
translation-refusal sentinel for them applies in module scope only. Regression test:
`parameter_grid::translated_axes::text_parameter_ref_axis_refuses_module_scope_only`.

## Real test output (change branch rebased on origin/main `9b3e5c9f4`)

- `cargo test -p knx-productdb --test it parameter_grid`: 25 passed, 0 failed.
- `cargo test -p knx-productdb parameter_layout`: lib 9 passed; `it` 40 passed; 0 failed.
- `cargo test -p knx-server http_parameter`: 137 passed, 0 failed.

## C49 failure is pre-existing (reproduced on pristine origin/main `9b3e5c9f4`)

`cargo test -p knx-app --test it c49` fails with
`C49_PROGRAM_XML path: NotPresent` (`crates/knx-app/tests/it/c49_partial_group_addresses.rs:10`):
the test needs the environment variable `C49_PROGRAM_XML` pointing at the private issue #49
program XML, which is not available in a clean checkout. It is unrelated to this change.

## Package gate (partial)

`python3 tools/run_package_gate.py --base origin/main`: fmt, documentation, browser-lanes,
check-layering/headers/anchors/ledger/corpus-gates, tool-tests, clean-desktop and clippy
passed (exit=0, proof=True). `rust-tests` ran 667 passed, 1 failed, 50 ignored; the single
failure is the C49 test above, so the gate stopped there (success=False) and the stages after
`rust-tests` did not run. This gate is therefore partial.
