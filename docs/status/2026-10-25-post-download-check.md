# Post-download check acceptance — 25 October 2026

## Published source

`8b5edad7f` is published on `main`; exact remote readback matched the accepted
candidate. The main-first merge `e9a9cf9e1` integrates the reviewed helper and
consumer feature `bd0274184` with the previously delivered scope advice. The
last commit corrects only a browser fixture's checklist expectation.

The [contract](../contracts/post-download-check.md) defines the regular
memory-download follow-up. CLI, server status, shareable reports, durable
activity history and EN/DE web views retain one payload-free vocabulary.
Transfer, restart, reach, identity and expected/observed load states are distinct.
Runtime remains unknown; matching identity does not prove the physical exemplar.
Restore and the existing manual verification checklist are unchanged.

## Independent review and corrections

Independent backend and contract/UI reviews requested focused corrections:
real CLI report/exit composition coverage, read-only frame allowlists,
truthful reached/unsupported wording, downgrade disclosure and mirrored
admission tests. Both correction reviews approved the source. The integration
review verified preservation of scope advice and the documentation merge; its
only blocking finding was a contradictory synthetic status/report restart,
corrected before final acceptance.

A first integrated browser attempt then passed 819 cases and failed four new
mismatch cases: an unconfirmed restart correctly adds an existing manual
restart check, while the new test still expected four checklist items. The
correction pins the exact ordered item codes and translated labels, including
the conditional restart item. Production inputs are unchanged by that fix.
Consent, saved-report byte equality, no-second-start and geometry checks remain.
Failed attempts are retained as failed; no timeout or test scope was relaxed.

## Final acceptance on `8b5edad7f`

All 15 stages selected by the maintained package runner passed. Source
fingerprints remained frozen; touched-user-data and unchecked-user-data lists
were both empty.

- Focused verification replay: all 16 EN/DE desktop/mobile cases passed.
- Nested web gate: all 34 selected stages passed, including 886 default browser
  cases (823 functional, 63 exclusive) and the selected production suite.
- Rust selection: 3,082 passed, zero failed, 131 ignored across 17 test binaries.
  This is the change-scoped selection, not a fresh whole-workspace gate.
- Private-corpus selection: 91/91 passed. The server integration target ran all
  49 exact selected cases, including all 29 HTTP download cases. The previously
  compile-only status, report, event and durable-history assertions therefore
  executed; they are not credited from an ignored routine test run.
- Formatting, Clippy, documentation, local-information, layering, headers,
  anchors, ledger, corpus-registration and diff checks passed.

Focused and integrated runs overlap and their totals are not added together.
The browser uses intercepted synthetic responses; actual server route evidence
comes from the separate isolated corpus/simulator tests, not browser fixtures.
The final fixture-only correction changed no production input. Earlier
screenshots remain bound to their original attempt; final geometry and saved
report assertions ran on the accepted candidate. Parent image-analysis calls
failed, so no new parent visual-review verdict is claimed.

## Compatibility and remaining work

Legacy missing results mean not checked. Explicit null and contradictory
records are refused. New readers accept old history; old strict format-2
readers refuse a page containing the new field. In an old server, reading that
page latches bus writes unavailable until restart, and paging it again latches
again. No records are rewritten or lost by that refusal. See the contract's
downgrade instructions rather than assuming bidirectional compatibility.

Software acceptance and source publication do not establish physical hardware
or installation-function acceptance. No real bus operation, release, deployment
or new device-family admission occurred. KNX Secure remains excluded.

This bounded increment closes neither [PRG-01](../open-work/PRG-01-programming-gaps.md)
nor parameter policies, full target/exemplar identity, runtime-state reading,
complete mutation effects or whole-mutation recovery.
