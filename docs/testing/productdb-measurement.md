# Product database measurement (WP-04.1)

Status: the source-bound synthetic release study is independently accepted;
[results and limits](productdb-baseline-2026-10-20.md) record its install, resolver
and target-memory baseline. Source delivery and maintained integrated acceptance
are [recorded separately](../IMPLEMENTATION_STATUS.md#wp-041-synthetic-measurement-baseline-20-october-2026).
This contract is not a new product format, installer or production memory guarantee.
[Ordered work](../goals/alpha16-successor-sequence-2026-10-19.md) ·
[WP-04](../OPEN_WORK.md#wp-04--large-catalogues-measure-then-stream).

## Scope and commands

The opt-in Linux tool uses an already-built production `knx products ingest`
executable. A thin ProductDB example provides only new-current-database setup
and the existing read-only AP1 resolver. A separate native example observes
only the target process; it contains no ProductDB behaviour. Fixture generation and builds happen
outside measured operations; neither the observer nor the example installs
products through a second implementation.

The native observer requires Linux `wait4` resource semantics. Both examples
are registered with `test = true`, so default `cargo test -p knx-productdb`
and `--examples` builds include the Linux-only observer target. This is a
boundary of this example-test inventory, not a claim that the ProductDB
library itself requires Linux. Non-Linux example/package-test execution has
not been verified by this work.

```sh
python3 tools/measure_productdb.py fixtures --output <absolute-new-input-directory>
python3 tools/measure_productdb.py run \
  --knx <absolute-release-knx> \
  --probe <absolute-release-productdb_measure> \
  --supervisor <absolute-release-productdb_observe> \
  --build-receipt <absolute-build-receipt.json> \
  --manifest <absolute-manifest.json> \
  --output <absolute-new-attempt-directory> --repetitions 3
```

The paths above are placeholders, not shell-ready defaults. Inputs, outputs and
executable paths must be explicit and absolute. A run never builds, downloads,
discovers a private corpus, overwrites an attempt or uses a default product DB.
Each case uses a new disposable catalogue; sequential operations in that case
reopen the same catalogue. All children receive disposable HOME, XDG and TMPDIR
locations. Execution uses the repository's shared workspace lease. Repetitions
repeat the complete case, not a failed operation until it passes.

A strict version-1 manifest names ordered package/wrapper inputs, their expected
raw package and decoded-member identities, and ordered install/resolve steps.
The complete manifest and detailed receipts are private. The build receipt binds
all three actual release executable hashes (`knx`, `probe`, `supervisor`) to
their build source/profile; a caller-supplied
revision by itself does not prove binary provenance. Source, binaries and inputs
must remain unchanged throughout a study. No arbitrary shell-command or SQL
assertion is accepted from a manifest.

## Reproduce observer calibration

Current integration checkpoint: independent review rejected the maintained
control driver for TERM-ignore escalation, controller-timeout cleanup, missing
controller/adapter/wrapper source binding, and missing maintained actual native
wait-failure coverage. The corrected review accepts the escalation, source-binding
and wait-failure corrections, but an actual hard-timeout counterexample still
leaves a process group alive when its PID has not yet reached the stage checkpoint.
The focused correction now reconciles an already execed live helper from the
scoped owner's kernel-listed direct children using the exact stage command,
unique receipt path, cwd, parent, start identity and PID = PGID = SID. A real
delayed-checkpoint regression passes with no measured-group survivors before
fixture cleanup and leaves an unrelated same-helper process untouched.
Independent correction review accepts this bounded recovery, with a fresh
20-method native-enabled pass and exact-group cleanup before fixture rescue.
This does not claim authentication of pre-exec or already-dead leaders, and
earlier fully checkpointed timeout passes do not supply that missing evidence.
All implementation-review blockers are closed. The actual source-bound release
study is independently accepted; [results and limits](productdb-baseline-2026-10-20.md)
record its 54 observations. Maintained delivery gates and source publication
are separately recorded in the implementation delivery entry, not inferred from
study acceptance.

Before accepting target-memory measurements, exercise the exact built helper
through the maintained adapter and its actual outer interruption path:

```sh
python3 tools/verify_productdb_observer.py \
  --supervisor <canonical-absolute-productdb_observe> \
  --output <canonical-absolute-new-private-control-directory>
```

Both paths must be canonical, with no symlink components; the output's parent
must already exist. Output must not overlap the checkout or helper directory.
The command streams helper and explicit controller/adapter/wrapper source
identities at start/end and requires equality. Its required controller inventory
is `calibration`, `cancellation`, `term_ignore`, `timeout`, `wait_failure`.
It compares the same tiny target before/while/after a touched parent allocation,
runs a genuinely allocating target and checks the next tiny child. It also
checks exit/signal mapping, actual process-group interruption, TERM-ignore
escalation, a hanging-target deadline and an actual inherited-SIGCHLD native
wait failure. The wait-failure control must retain helper exit 125, an incomplete
real-target-PID checkpoint and unavailable target metrics, not a successful
measurement.

Readiness interruption is one-shot before cleanup. Internal controller deadlines
unwind through the actual measured-operation cleanup; a retained outer hard
bound owns the controller and the exact unfinished checkpointed process groups.
Test-only child adoption is scoped/restored in these opt-in processes, never
installed in the invoking test runner. Raw proofs remain in the new private
directory; a rejected or interrupted
measurement never becomes a successful target receipt. An already elevated
observer floor can make calibration inconclusive; it is not subtracted and the
discriminator is not relaxed. Passing these synthetic controls is required
observer evidence, not a product workload or release-memory baseline.

## What is measured

Each install runs the real CLI with an explicit owned database. Resolve runs the
example directly against a current-schema database and calls
`procedure_resolution::resolve_package_ap1`. Setup and migration are not hidden
inside a resolver timing. A null result, an unavailable resolution and a
nonempty expansion remain distinct; every resolution remains non-executable.

The native `productdb_observe` supervisor launches and reaps the exact target
PID with Linux `wait4`. Its separately validated receipt supplies the target's
absolute peak RSS in KiB, CPU time, exit/signal, raw wait status and elapsed
time around native launch/reap/checkpoint work. The Python `MeasuredRun` remains
the outer log/session/cancellation wrapper; its PID, elapsed and resource values
are retained separately and never substituted for target values. Helper and
target stay in the same outer process group for interruption cleanup.
The low native pre-exec footprint is included, not estimated or subtracted.
The number is
not an allocation delta, an internal parser-phase peak, a sampled disk peak or
an end-to-end user-interface latency. No operating-system cache flush, warmup
or database tuning is performed; cache conditions and repetitions are disclosed.
A short child still needs an actual reap-time observation; missing data is not
reported as zero.

Raw process outcomes are immutable evidence. An expected refusal remains a
nonzero process result and makes the raw process receipt unsuccessful. A
separate study acceptance can accept that observation only when the exact named
refusal, full post-operation accounting and preservation checks all pass. A
signal, spawn error, different refusal or missing after-read is not acceptance.

## Complete accounting and integrity

After each child exits, read-only observers enumerate every stored table/index,
including automatic indexes and the schema btree, and retain zero-growth objects.
Row counts and per-column UTF-8 text/BLOB lengths are logical metrics; they are
not estimates of physical SQLite storage. `dbstat` supplies btree pages, payload,
unused bytes and overflow pages for each object and index ownership. Free pages
are separate; schema pages are counted once as part of btree pages.

The initial supported observation layout is a quiescent, exclusively owned,
UTF-8 DELETE-journal database without auto-vacuum. Record actual pragmas and endpoint
file/sidecar sizes. Require the file/page reconciliation to close; unsupported
WAL/pointer-map/unattributed layouts or unavailable `dbstat` fail explicitly,
without checkpointing, VACUUM or changing production pragmas to manufacture an
accepted result. Endpoint sizes do not establish transient journal/disk peaks.

Retained original package bytes and deduplicated decoded source bytes are
separate fidelity layers. Member-reference bytes include multiplicity; distinct
referenced-blob bytes do not. Global source unknowns, package report rows,
occurrences, historical counters and unavailable report states are separate
units, not one combined unknown count. All evidence/provenance tables and their
indexes remain in the complete inventory, even when their growth is zero.

Authenticate retained packages, the complete ordered member inventory and every
decoded source against external input bytes, not only database self-hashes or
aggregate lengths. Full typed all-table snapshots include declared columns and accessible hidden
row identity on ordinary rowid tables, and detect same-count changes to
values, samples, occurrences and report history. Retry/rollback preservation
requires logical equality after reopen; physical allocation/free-page equality
is neither required nor a substitute. Integrity and foreign-key checks must pass.
Observer errors after a successful install still prevent study acceptance.
An ordinary rowid table with all three hidden-rowid aliases shadowed refuses
rather than supplying incomplete identity evidence. WITHOUT ROWID tables use
their complete declared-primary-key ordering. Non-UTF-8 databases refuse before
accounting, so logical text lengths cannot silently become storage-encoding
lengths.

The external-byte oracle deliberately materializes bounded inputs outside target
timings. Its supported-study ceilings are separate from production ingest
profiles and are not an observer RSS guarantee:

| Admission budget | Limit |
| --- | ---: |
| Aggregate external input bytes | 128 MiB |
| Aggregate retained package bytes, including expanded wrapper inners | 128 MiB |
| Aggregate decoded member bytes | 128 MiB |
| One decoded member | 32 MiB |
| Actual selected database values, including retained BLOBs and metadata | 256 MiB |
| Expected inventory records / actual database records | 100,000 |

Each verification call resets the budgets and charges admitted occurrences
before deduplication. Known oversized input/manifest/SQL inventories refuse
before mass loading; actual archive sizes and retained value lengths are
checked independently of declared size columns. Refusal rejects the complete
observation: no bytes, member references or history are trimmed to fit.
These limits admit the generated study's proposed sizes; only an actual study
can establish its complete production-path acceptance.

## Workloads and acceptance

The synthetic baseline includes meaningful persisted parameter rows at increasing
application-member sizes, repeated versus distinct unknown keys, distinct original
archives sharing decoded sources, same-path retry and explicit source-name alias,
and a seeded wrapper whose later malformed member rolls back an earlier new
package. AP1 cases include a nonempty expansion, a larger admitted expansion and
an installed source explicitly unavailable to the resolver at its independent
source ceiling. If another parser budget refuses a proposed near-byte-limit
fixture, report/refine its shape; never widen production limits to make a
benchmark succeed. MiB labels describe actual decoded member bytes, not ZIP or
catalogue size.

A tool, passing observer controls or a source-based fixture proposal alone does
not complete WP-04.1. Acceptance requires actual source-bound release runs,
complete table/index/source/evidence reconciliation, attributable process peaks,
expected failure preservation and a reviewed explanation of measured growth.
Detailed private outputs must not be copied into public logs or tracked files.
Synthetic results establish only their named workloads, not private-corpus,
universal KNX/ETS, native desktop or device compatibility.

WP-04.2 uses resolver-specific evidence to decide whether parser replacement is
justified; a documented no-change decision is valid. WP-04.3 separately measures
preview cost and implements bounded progress/cancellation/atomic web workflow.
Neither later delivery is supplied by this observer. WP-19 follows the complete
ordered WP-04 disposition. No hardware operation or release is authorised.

## Observer sources

The SQLite Project's **The DBSTAT Virtual Table** documentation describes its
btree metrics and omissions (freelist, pointer-map and lock pages). Python 3.14's
**os.wait4** documentation defines per-child resource usage at reap. Linux
**getrusage(2)** documents that resource metrics survive `execve`; direct
Python-parent launch/reap is therefore not sufficient target-memory evidence.
A real same-target control reproduced parent-memory contamination, including
after freeing the parent's allocation. A prior-heavy-child test alone did not
discriminate that defect. These rejected pilot RSS values are not a baseline.
Native-supervisor acceptance requires fresh/held/freed-parent controls, a real
heavy target, exact status validation and group interruption cleanup; mere
positive RSS or availability probes are not product measurements.

The observer change is local to this opt-in study. It does not repair or
reclassify historical resource metrics from other gate tools.

- <https://www.sqlite.org/dbstat.html>
- <https://docs.python.org/3.14/library/os.html#os.wait4>
- <https://man7.org/linux/man-pages/man2/getrusage.2.html>
- <https://man7.org/linux/man-pages/man2/wait4.2.html>
