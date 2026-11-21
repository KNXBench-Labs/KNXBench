# PRG-01-N8 queue reconciliation — 21 November 2026

Document-only. No code, runtime, history or report change; no bus, device, VM or
hardware contact. KNX Secure excluded.

## Where PRG-01 increments stand

Observed against `origin/main` at `9b3e5c9f4`:

| Increment | State on main | Record |
|---|---|---|
| N4 unadmitted run-state masks stay refused | merged (#108) | [ADR-0182](../adr/0182-unadmitted-run-state-masks-stay-refused.md) |
| N5 remaining parameter-policy evidence | merged (#109) | [ADR-0183](../adr/0183-prg01-remaining-parameter-policies-need-evidence.md) |
| N6 mutation-effects observation scope | merged (#110) | [ADR-0184](../adr/0184-prg01-mutation-effects-observation-scope.md) |
| N7 exhaustive refusal classifiers and code pins | branch `work/prg01-n7-20261121`, not merged at this observation | none yet on main |
| `0702h` run-state mask (PR !100) and customer-adjustable root/Union plus module-local Options (PR #107) | queued; their branches were not inspected here | not claimed by this note |

Rows queued on PR branches are not claimed as delivered. When they merge, the
merging change updates the PRG-01 detail file and the OPEN_WORK row.

## Ledger

`PRG-01` has no row in the [ledger](LEDGER.md); it is tracked through
[OPEN_WORK](../OPEN_WORK.md) and its detail file, so no ledger status changed.

## Still open

Unchanged from the detail file: restart-stability and layout evidence, per-form
parameter rules, physical function and whole-mutation recovery. All software
increments remain *untested* on hardware and storage survival is not claimed.
