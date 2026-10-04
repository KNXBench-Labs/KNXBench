# Existing ZIP cap contracts — bounded public slice

Scope: KL151 evidence for the currently enforced member-count boundary; no
production cap decision, limit raise, streaming rewrite or manufacturer grammar.
Two synthetic tests added; initial execution was pending (accepted below):

- Exactly4096 ZIP entries (two known XML payloads and empty directories): install,
  retain exact original archive, replay without changing any persisted value.
- Exactly4097 entries: SizeLimit refusal with a previously installed nonempty
  database, complete all-column/all-table snapshot and retained seed unchanged.

Fixtures assert actual ZIP-reader entry counts. Existing standalone safety tests
only had a forged >64MiB member case against an empty database; this count-edge
slice is distinct. No real manufacturer fixtures, private corpus, network/hardware
or Web changes. Lower/upper member-cap mutation controls and gates pending.

Native worker started: `proc_89125de0613a`, PID3298119, notify-on-completion.
Inputs frozen; own fresh target; serialized canonical alpha/workspace leases;
network-isolated offline locked Cargo. Output/acceptance remains pending.

## Native reconciliation

Worker proc_89125de0613a exited0. Actual native2/0/0 verified from the
complete log; all frozen inputs/log/test-binary hashes checked. No silent
skips. Lower/upper cap mutation controls and broader gates still pending.

## Next acceptance work queued

Two isolated tracked-public-source snapshots change only MAX_MEMBERS to4095
and4097. Canonical source unchanged; each control has its own fresh target.
Worker `proc_528a4d9b2dea`/PID3313312 started and is live, queued before any
stage; actual lock blocker3311445 and child workspace holder3311543 verified.
At dispatch there was no killed-mutant or broad/publication claim. Separate in-session test
review found no blocking issue; no independent-model review was invoked.

## Delayed historical Scheme23 final10 notification

`proc_660197ee4edd` is already reconciled historical acceptance at aadd8820.
Original wrapper exit1/AssertionError remains retained: its post-stage check
wrongly required the prior Git-stamped CLI binary hash. Rechecked all ten
archived log hashes/exits0 and the permanent executable hash; network-isolated
`--version` returned `knx 0.1.0-alpha.1+gaadd8820`. No new tests or gates, no
double counting or Scheme23 restart, no transfer to the current cap slice.

## Delayed historical Master-evidence mutation retry

`proc_8b841d7d2950` is already reconciled AR06T history. Twelve archived
stage-log hashes and the producer summary hash verified; archived runner hash
also matches its permanent manifest. Master compile0/behavior101,0 passed/
1 failed:missing Optional evidence. Wrapper expected VariableLength and refused.
Original refusal retained beside independent historical prefix3 proof, not a
completed six-case retry sweep. No new tests/gates/current-cap kills. Initial
readback compared producer_sha256 to run.py incorrectly; exact matching archived
summary.json resolves it, not an archive corruption. No historical file edited.

## Count-cap mutation acceptance

proc_528a4d9b2dea completed exit0; two compiled semantic controls independently
accepted.4095 fails exact4096 install with named inclusive SizeLimit failure;
4097 wrongly accepts over4097 input and fails unwrap_err. Each selected test
is RED101/0 passed/1 failed, not a compile failure or production regression.
Logs, per-snapshot fresh binaries, tests and mutated source hashes checked.
All120 frozen dependency inputs compared against native candidate:only intended
package.rs constant differs. Canonical source never mutated, cap4096 unchanged.
Native2/0/0 and120 canonical hashes still exact. No private access, broad gate
acceptance, publication or full KL151 closure.

## Branch public8 acceptance

Eight actual commands exit0, all log hashes and784 source/config inputs exact.
ProductDB640 passed/0 failed/25 ignored across31 result blocks; native2 is a
subset, not extra passes. Strict package Clippy/fmt/whitespace and four doc
audits accepted. Fresh root-bound xtask:453 headed files/157 headerless at
ceiling,392 links/261 markdown files,365 Rust corpus-policy scans. Separate
in-session whole-slice review, no CRITICAL/IMPORTANT/MINOR findings; not an
independent-model approval. Source admission is at package.rs:840, seeded archive
INSERT at2389, replay source-name INSERT OR IGNORE at2370. Integrated gates and
remote publication/hygiene remain pending. No new private-corpus result.
