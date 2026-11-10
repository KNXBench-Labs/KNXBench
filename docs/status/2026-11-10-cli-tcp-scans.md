# Optional TCP CLI scans — software acceptance, 10 November 2026

## Delivered scope

Main-first merge `ac9b23814dac72c240d548aac8299fa9ba988ddd` was published on
10 November 2026. Local, tracking and live `main` refs were read back equal.
The [WP-29 contract](../contracts/tcp-tunnelling.md) now covers explicit
`--tunnel-transport udp|tcp` on CLI `knx bus scan`, for `--device` and
`--line`/`--range`. Omission remains UDP; explicit TCP never falls back to UDP.

The actual caller selects the transport, retains the diagnostic tap and reports
the opened transport. Target/exclusion safety, dry-run no-I/O, probe deadlines,
project comparison and the existing distinction between descriptor replies,
positive connect confirmations and silence remain unchanged. TCP socket acceptance
alone never establishes occupancy. Gateway admission shares the existing
monitor/write unicast-IPv4/nonzero-port validation.

TCP completion also reports a nonzero cumulative skipped-frame count before
consuming the tunnel. This is a pre-disconnect sample, not a final-wire count;
unknown/over-cap frames are not automatically lost KNX telegrams, and their
content is not retained. Success and transport-error completion use the same
sample and existing warning formatter. UDP and zero-skip TCP emit no skip warning.

This is source/software acceptance after Alpha 20, not a release. Server/Web
scans, programming, downloads/restores, address operations, serial, discovery,
routing and KNX IP Secure are not widened. Real-gateway interoperability remains
unverified and requires separate authorisation. Broader
[WP-29](../open-work/WP-29-knxnet-ip-tcp-tunnelling.md) stays open.

## Review and focused execution

The complete candidate received independent standards and spec reviews. Spec
review approved the bounded behavior; three isolated semantic mutants were
rejected by maintained actual-CLI tests: ignoring TCP at the actual constructor,
crediting socket acceptance as device evidence, and skipping graceful tunnel
teardown. Four unmodified and four restored positive-control executions passed.

Standards review requested skipped-frame disclosure. The correction added a
maintained actual-CLI regression for unknown plus oversized frames followed by
valid descriptor traffic or peer EOF. Independent correction review closed S1:
corrected/restored controls passed, while removing only the reporting call failed
at the expected missing-warning assertion. That mutant stops in the successful
scan cell; its later EOF cell is not independently credited as a mutant kill.
Both cells execute in the passing controls. Review evidence remains bound to its
original source epochs rather than relabelled as new execution.

Focused implementation checks passed: 40 scan-parser/format tests, eight endpoint
tests, then 18 TCP CLI integration tests after the correction, plus CLI all-target
Clippy with warnings denied. These populations overlap the package gate below;
they must not be added to its totals. Earlier integration fixture failures were
retained and corrected without claiming an original-source product failure.

## Integrated package gate

The maintained change-scoped gate ran on the published merge against first parent
`383e9e2857f9fc5070c878de19c09ac18618c3ab`. All **13 selected stages** passed:
format, documentation, browser-lane guard, layering, headers, anchors, ledger,
corpus registration, tooling tests, Clippy, Rust tests, selected corpus tests and
whitespace. The native receipt established complete plan execution, valid direct
stage proofs, frozen inputs and no touched or unchecked user-data directories.

- Rust: **646 passed, 0 failed, 22 ignored**, selected `knx-cli` and `xtask`.
- Separate selected corpus: **17/17 passed**.
- Tooling: **563 tests run, 16 existing environment/opt-in skips**, no failures.
- No Web browser suite, bindings, whole-workspace or release gate was selected.
  Fixture sockets used loopback; no real KNX bus/device/VM operation occurred.

The conditional publication wrapper checked the complete receipt, clean frozen
head and unchanged upstream base before pushing, then read back the exact ref.

## Preserved failures and mechanical corrections

The first integrated attempt stopped at documentation: a hand-appended generated
limitations overview was stale. Regeneration removed the redundant overview
appendix; the maintained KL-026 source section retains the scope explanation.
The second stopped at the tooling layout guard: the new tests ran through a
`#[path]` alias, but that alias was outside the repository's conventional module
layout. The file moved byte-identically to
`apps/knx-cli/tests/it/cli_bus_tcp/scan.rs`, retaining its logical test namespace;
the alias attribute was removed. The layout suite and all 18 TCP CLI tests passed,
then the complete third integrated attempt passed. Neither failed gate is
credited as acceptance, and neither reached the Rust/corpus product stages.

Implementation chain: `35fdc8bd4` (feature and reviewed correction), `057e02777`
(generated overview), `6b19842d6` (conventional test layout), main-first
`ac9b23814`. The result-only documentation successor does not change runtime code.

Raw attempts, review reports, isolated mutations/controls, source bindings and
executed binaries are retained privately at
`<profile>/evidence/2026-11-10-wp29-cli-scan/`. This record is not a claim that
all private evidence has been made public or that physical compatibility was tested.
