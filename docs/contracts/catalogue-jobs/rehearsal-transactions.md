# Disposable catalogue rehearsal transactions

The internal `install_admitted_product_file` entry requires a caller-owned
transaction on the **connection passed to it**. It commits nothing. An earlier
preview reservation on W, or apply's still-active W reservation, cannot cover the
separate disposable D connection. Both preview and apply revalidation therefore
explicitly begin a transaction on D before invoking that controlled installer.
Preview's before/after database report reads remain inside the same transaction;
apply extracts and compares the complete immutable preview identity before D is
settled. Neither rehearsal claims the live publication gate or commits D.

Every obtained rehearsal receipt converges on uncancellable, inspected rollback
and explicit native close after success, refusal, cancellation or unwinding work.
An already-automatic transaction end is observed, not called explicit rollback.
A rollback error, non-autocommit state and native close error remain independent
cleanup facts; successful close does not erase a failed rollback observation.
Failed native close retains the connection in the existing artifact inventory.
The same inventory retains the exact typed cleanup failure independently of
returned errors and opaque panic payloads, including after successful native
close. The workspace owns that inventory outside both apply's public facade
catch and preview's mounted worker catch. Dropping either catch's projection
cannot erase those facts; closed-ownership release refuses their removal.
Primary install/identity/Stop errors survive alongside cleanup errors. Unwind
continues with the original opaque panic payload after clean D settlement; if D
settlement also fails, the opaque pair also carries the typed cleanup error, but
is not its sole owner. Panic text is not projected into public diagnostics.
The facade's cleanup-failure signal and mounted workspace settlement/terminal
classification include this independent D observation, even with no retained
handle and a healthy W after explicit W rollback. Accepted cancellation remains
the primary stop cause, but cannot turn failed D cleanup into clean Cancelled or
cleanupComplete. When the nested rehearsal settlement has an installer refusal
or Stop, the facade retains that primary failure category separately from its
cleanup-failure signal. No W commit/rollback uncertainty is invented from a D
failure.
The existing W unwind/quarantine policy is unchanged; D cleanup never proves W
rollback.

No change is made to W's transaction, source admission, selected policy/profile,
publication gate, recovery-backup barrier, growth limits or durability settings.
The quiet committing wrapper is not a substitute for the controlled installer.

## Regression and evidence boundary

Test-only observations immediately before **both** actual D installer calls
assert non-autocommit. The synthetic preview and second-rehearsal controls also
check rollback-before-close, no retained handle after clean close, preserved
failure/cancellation and native-close ownership. D's installed rows are no longer
expected at the pre-close observation: immutable reports were extracted before
rollback. Actual write observations remain required; empty rows alone cannot
prove that rehearsal executed or that W was untouched.

Public `execute_controlled` controls cover D panic with clean settlement, native
Busy close and denied rollback followed by successful close, then drop the
returned execution and recheck the actual workspace facts/handle ownership.
Preview controls discard the outer facade unwind payload with the workspace
retained, matching the mounted catch's owner lifetime. Separate cancelled and
noncancelled installer-refusal controls deny D rollback without forcing Busy;
they require exact primary/cleanup facts, true cleanup-failure classification,
closed D handles, healthy/proved-not-committed W and no W DML. These are source
definitions, not claims of execution or mounted runtime acceptance.

The private Large mounted acceptance test retains a separate create-new 0600
`*.diagnostic.json` next to its configured evidence JSON, outside its owned
TempDir. It contains only the last bounded completed status GET body (64 KiB
maximum), last validated public DTO, monotonic request/validation milestones and
scoped sampler results. Failure capture joins only the test sampler, not the
native worker, and takes no worker-held workspace/native/SQL mutex. It copies no
active SQLite file or source payload. Its `resourceAcceptance` is always false;
the existing success report and all acceptance assertions remain authoritative.
The 1,800-second wait watchdog is unchanged. A failure snapshot does not establish
worker disposal/drain, native settlement, an importer substage or sync counts.

This source correction fixes a definite caller-contract omission. It does **not**
attribute a historical timeout or establish a speedup, Large compatibility or
resource acceptance. The added assertions and harness changes require owning
execution after review; no test/build/runtime acceptance is claimed here.
