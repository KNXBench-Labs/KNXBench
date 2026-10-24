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
Primary install/identity/Stop errors survive alongside cleanup errors. Unwind
continues with the original opaque panic payload after clean D settlement; if D
settlement also fails, an opaque pair retains both payload and typed cleanup
error. Panic text is not projected into public diagnostics. The existing W
unwind/quarantine policy is unchanged; D cleanup never proves W rollback.

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
