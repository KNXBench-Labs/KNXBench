# System-2 PID 7 wrong-width diagnostic — 25 October 2026

## Delivered boundary

Published merge: `5f1ad454eb27da49685062dbe37b3ed89aa8a3ac`, with main as first parent.
Feature `a2d04899e` and correction `28148aaed` preserve the existing wrong-width
refusal. A reply with a positive element count and a byte count other than four
still stops prewrite acquisition before mutation. The diagnostic now includes the
object, true element count, actual byte count and at most 16 hexadecimal octets;
empty data is `(none)` and longer prefixes end in `...`. No padding or pointer
interpretation is added. Zero-element refusal remains the existing observation.
Start-index matching is unchanged.

The **private** `Failed.local_diagnostic` journal entry retains this text across
reopen. The application returns it through the CLI execution error result; actual
CLI rendering was source-traced only, not exercised by a new CLI-specific test.
HTTP status and the shareable report deliberately do **not** carry the runtime
failure text. No new structured DTO, report field or UI diagnostic was delivered.

## Review and corrections

Independent Claude Opus 5.5 review requested changes for a false documentation
claim that the raw reply reached server status. The correction documents the real
boundary, adds an application-level journal-reopen regression, covers a positive
count with empty bytes and a count of two, and tests the 16/17-byte display boundary.
Re-review of `28148aaed` returned **APPROVE**, with no critical or important findings.
The inherited case of a non-one element count with exactly four bytes is explicitly
not newly validated semantics; it remains part of the future width-model work.

The empty-byte formatting regression first failed with the intended trailing-empty
hex diagnostic, then passed with `(none)`. Earlier RED/GREEN and the first branch
gate remain historical evidence, not measurements of the corrected merge.

## Integrated software acceptance

The maintained change-scoped package gate ran on exactly `5f1ad454e` against
`6ae555c52`. All **14** selected stages passed: fmt, documentation, browser-lane
classification, layering, headers, anchors, ledger, corpus-gate classification,
tooling tests, desktop cleanup, Clippy, selected Rust tests, selected corpus tests
and diff check. The receipt's revision, ordered stages, zero exits, valid proofs,
start/end fingerprints and empty touched/unchecked user-data collections were
validated against the saved plan and current candidate before publication.

- Rust: **3,279 passed**, zero failed, **131 ignored**, 17 binaries.
- Corpus: **91 selected / 91 passed**, run in the gate's network-isolated scope.
- No web/browser test lane was selected for this backend diagnostic delta.
- Frontend dependencies and bundle were prepared because tooling/desktop consumers
  require them. Two earlier setup attempts stopped at missing prerequisites; their
  receipts are retained separately and are not counted as accepted runs.
- Tests use simulated transports only, including the hardware-kind application
  adapter backed by a simulator. No real bus/device access, release, deployment,
  actual device-width measurement or recovery demonstration occurred.

Private receipts and review records are retained under
`<profile-scratch>/s201-delivery-20261025/` until verified evidence retirement.
The integrated producer receipt was
`target/package-gates/s201-integrated-20261025/receipt.json`.
Local HEAD, fetched tracking ref and the live remote `main` ref were equal to the
published merge immediately after push. The selected shared root checkout was
intentionally not synchronized; foreign edits and private files were preserved.

## Remaining work

[S2-01](../open-work/S2-01-pid7-raw-response.md) remains **P1, partially delivered**:
the real width and its meaning need device evidence, width-aware recovery admission
is not implemented, and structured status/report/UI projection is still open. A
bounded prefix is not a complete long-response capture. A backup does not justify
silently continuing past an unsupported response. Existing post-download memory,
Loaded and written-version verification stays hard.

[S2-02](../open-work/S2-02-gateway-busy-after-failed-runs.md) remains a separate
planned **P1** investigation. Issue #30 stays open; diagnostic delivery is not the
hardware programming/recovery outcome tracked by #22. No new device attempt is
required by this subdelivery.
