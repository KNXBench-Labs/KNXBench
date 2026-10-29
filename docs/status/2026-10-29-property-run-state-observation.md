# Property run-state observation: bounded software acceptance

Observed: 29 October 2026.

## Accepted scope

Runtime integration `1610100783d636f6fb5894251539f46f31e8b74d` is a main-first
merge on `8af99f7f8`, including feature `2a353f43a` and the separately reviewed
test-helper type correction `51c309957`. This is source/software acceptance,
not an Alpha 18 release change or hardware acceptance.

The [contract](../contracts/post-download-run-state.md) adds a read-only PID 6
observation on exact masks `0701h` and `0705h`. It requires the candidate
Applicationprogram object's type and plan identity to be freshly validated;
unsupported layouts and unavailable or undefined responses remain explicit.
The [source investigation](../research/2026-10-29-property-run-state-observation.md)
separates this property contract from the unresolved memory-state encoding.

Core, network, application, CLI, HTTP, web, report and history consumers retain
one shared, payload-free observation. Transfer, load state, property run state
and installation function remain separate. Existing records remain readable;
format 1 gains vocabulary, not a promise that older strict readers accept it.

## Executed acceptance

- Independent whole-feature Opus review and correction re-review: APPROVE,
  no critical, important or minor findings left open. I1 and M1–M6 were closed.
- Fifteen original guard mutants and five correction-calibration mutants were
  semantically rejected; restored controls passed. They remain bound to their
  respective source snapshots, not relabelled as mutations of later revisions.
- The maintained change-scoped feature gate at `51c309957` and main-first
  integrated gate at `161010078` each passed all 17 selected stages. Both receipts
  record complete execution, frozen inputs, valid per-stage proofs and neither
  touched nor unchecked user data.
- Selected checks include formatting, documentation and architecture/ledger
  gates, tooling tests, web build, fixture TypeScript checks, Vitest, functional
  and exclusive browser lanes, the production download-support lane, clippy,
  Rust tests, bindings and selected private corpus regression (155/155).
  Overlapping focused and package populations are not added into a unique total;
  ignored/opt-in tests are not claimed as executed.
- A real server binary was built from the integrated runtime source. In a fresh
  user/network namespace with loopback only, its version command returned 0,
  health and the exact built frontend returned HTTP 200, and an empty download
  status returned the expected HTTP 404. SIGTERM returned 0, the process was
  reaped and its listener was closed. This is startup/serving/shutdown evidence,
  not a real device operation.

The first feature gate stopped at a browser-fixture TypeScript error: the test
helper required literal English catalogue values while also accepting German.
The one-line readonly value-type correction preserves the same message keys,
assertions, runtime code and compiler configuration. It was independently
reviewed and the exact failed TypeScript scope passed before the entire feature
gate restarted. The original failure is retained and is not feature semantic-RED
credit. Earlier setup/compile failures likewise remain uncredited. React act
and chunk-size warnings are retained as unattributed, not declared baseline or
warning-free acceptance. Detailed corpus and licensed-source evidence stays
private; no manufacturer payloads are included in this receipt.

## Limits and remaining work

No real bus, device, analysis VM, deployed container, release tag or public
release asset was changed by this increment. No memory fallback, scan, new
session, authorisation, restart, Verify Mode, retry or wait policy was added.

The bounded property observation does not establish object uniqueness,
physical-exemplar identity, whole-device running status or installation function.
Other layouts, the memory encoding, parameter/Union/customer-write policies and
complete mutation effects/recovery remain open. **PRG-01 is not closed**; see
[its maintained open-work record](../open-work/PRG-01-programming-gaps.md).
