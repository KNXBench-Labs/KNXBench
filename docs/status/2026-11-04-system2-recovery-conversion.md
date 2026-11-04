# System-2 recovery conversion reuse — bounded software repair

Observed: 4 November 2026. This is a main-only software change, not a release,
physical recovery claim or closure of S2-02/S2-04/S2-05.

## Cause and correct owner

The previously red optional mounted cancellation/recovery test reaches the correct
fresh recovery worker and makes transport progress. Its cached status projections
are non-atomic and were not used as terminal evidence.

A narrow temporary probe followed the actual management receiver. At normalized
recovery operation 15 (source index 12, AdditionalLoadControls), the outstanding
await was the load-control pre-write PID 5 read for object 3. The send had returned;
the first pending receiver poll was only 3.25 microseconds old, with exhausted
Tokio cooperative budget. It was not an observed seconds-long transport stall.

The source-bound timed run measured ten retained-source plan reconstructions before
the Net operation loop (6.806077586 seconds) and six more in the recovery-acquired
barrier at operation 1 (4.115689887 seconds). Their combined 10.921767473 seconds
already exceeded the unchanged ten-second task-finish deadline. Net operation-loop
clocks start after the App's initial validation; they cannot be subtracted from
worker-entry clocks as though they shared an origin. The first cooperative yield
exposed the expired deadline. These measurements describe this instrumented run,
not a universal performance guarantee or retrospective proof for every old failure.

The duplication belongs to App recovery conversion, not the simulator or fixture:
`RecoveryRecord::to_snapshot` reconstructed the immutable bundle twice;
`from_snapshot` did so four times, including its complete serialized-record
roundtrip validation. The real worker invokes several independent conversions.
A temporary one-millisecond settlement-loop sleep did not fix the failure and was
removed. No timeout, protocol timing, simulator or mounted-fixture change remains.

## Focused fix and preserved boundaries

Each public conversion now reconstructs its immutable bundle once, derives its
premise from that exact plan, and passes both owned values into a private,
synchronous conversion helper. The helper retains complete version/original-bundle,
resource, region/index/order, pointer-width, allocation-byte, observation and
evidence validation. The record still roundtrips through those checks.

This is operation-local reuse, not a persisted/global cache or freshness witness
across an await. Every independent public invocation still validates retained
sources. Current-source/project admission, target/consent and the durable original
backup barrier remain independent. Serialized records, dependencies and public
validation APIs are unchanged.

## Executed regression and counterprobe

- The complete owning App execution-test prefix passed: 18 passed, zero failed or
  ignored. Two new leaves count actual retained-source reconstruction, verify exact
  roundtrip, and retain malformed/rebound source, request, plan and evidence refusals
  without journal/progress mutation. Existing actual-original and barrier checks remain.
- The original digest-pinned public-package mounted simulation passed unchanged,
  then passed again after a reversible counterprobe restored the candidate bytes.
  Each invocation executed one test; these are overlapping runs, not unique coverage.
- Its actual task-finished/join/disconnect assertions, exact original simulator
  snapshot equality, durable `LocalOutcome::Restored` reopen, original backup equality,
  once-only refusal and fresh-server restart/consumed refusal all ran successfully.
  Status-cache equality alone would not satisfy this contract.
- Exact old production conversion bytes with final regression bytes unchanged gave
  16 App passes and two assertion failures: reconstruction counts two versus one
  and four versus one. The full reverted simulation failed earlier at its original
  first-write deadline; its recovery assertions were **unexecuted**, not a second
  exact recovery-wait kill. Restored candidate App tests passed 18/18 again.

The initial candidate controller declared the wrong stage order, leaving its raw
receipt incomplete despite all direct stages passing with frozen inputs. That raw
failure is retained; a separate adjudication authenticates the actual inventory.
The later complete counterprobe receipt records expected negative stages and the
restored-candidate green stages. Neither receipt is relabelled a green package gate.
Independent review and maintained package acceptance are separate from these
focused results and remain bound to their own source/receipt inventories.

## Independent safety-review closure

Independent review found no product defect in the fix, but initially blocked on
two relevant test gaps: deleting both direct original-bundle equality checks and
deleting exact-premise comparison each survived the initial 18-test prefix. The
serial-only premise fault was rejected by a different observation check, so it
was not a valid discriminator for the intended guard. Those survivors remain
recorded as survivors of that original suite.

The maintained suite now adds a different valid original-bundle record, an
internally admissible but different pointer-width premise, and version-entry
controls. The final compiled prefix passes 21/21. Each of the two prior survivors
now fails its specific new assertion (20 passed, one failed); a recompiled,
byte-restored control passes 21/21. Four completed-cell markers are reconciled
individually; a mutant's failed leaf does not earn its post-assertion marker.
The evidence-validation bypass was separately killed by the original allocation
control. Version-guard deletion and exhaustive guard mutation were not executed.
Finding-specific independent review is APPROVE; the production fix is unchanged.

## Scope and remaining limits

Execution used in-process simulated transports in offline namespaces, the existing
shared execution leases, scratch data/config homes, digest-pinned read-only public
input and bounded TERM/KILL watchdogs. Temporary probes were removed. No physical
device, bus, gateway, VM, vendor contact, release or deployment action was involved.

Only this simulated recovery debt and its causal App conversion duplication are
addressed. Field failures in [S2-04](../open-work/S2-04-tunnel-loss-after-checkpoint.md),
general source equivalence and wider S2-05 obligations remain open. The rejected
GATE-01 setup-bypass hypothesis and E17 admission work are not part of this repair.
