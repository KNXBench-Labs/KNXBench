# WP-04.1 observer integration and rejected RSS pilot

## Scope

Opt-in measurement tooling only. No product schema, installer, parser budget,
source format, UI, hardware, deployment or release changes are authorised by
this work. The published v28 repair remains the predecessor, not a measurement
result. Original accepted/rejected attempt evidence stays outside Git.

## Independent review

The fixture generator and thin init/resolve probe were accepted for integration.
The Python review rejected target-memory attribution: a real identical tiny
executable acquired the Python parent's large peak, including after the parent
freed its allocation. A real source-byte verification also inflated the next
child's reported peak without a comparable target post-exec increase.
The direct-parent pilot's resource numbers are rejected; its 18 expected
production-path outcomes remain exploratory integration evidence only.

Two additional contract gaps were identified: hidden rowid exclusion for tables
with declared non-integer primary keys, and UTF-16 storage bytes being labelled
UTF-8 text bytes. Both require test-first corrections or an explicit narrowed
admission boundary, not silently reinterpreted counts.

## Local correction design

Keep the existing Python observer as the outer logging/session/cancellation
wrapper. A small fresh native supervisor launches/reaps the actual target and
reports its PID, raw wait status, exit/signal, peak, CPU and elapsed values.
Validate that receipt independently and bind the helper binary alongside the
release CLI and resolver probe. Never substitute outer-wrapper resource data,
subtract parent estimates or relabel failed/rejected attempts as successful.

The source-byte oracle must have an explicit supported resource boundary.
Typed snapshots must include accessible SQL-visible row identity. Non-UTF-8
catalogues may be refused rather than mislabelled.

## Evidence available at this checkpoint

Controller protocol tests first failed on the missing observer API. An original
study discriminator then reproduced target PID equal to the raw wrapper PID;
that assertion failed before integration. Protocol and study controls pass
with separate target/wrapper evidence after integration. Their Python helper
is explicitly synthetic protocol control, not native memory acceptance.

## Build-binding correction

The independent adapter review rejected a second-snapshot provenance gap:
a binary changed after build-receipt validation could become the recorded
start/end baseline. A test-first discriminator reproduced this for the CLI,
probe and supervisor (three intended assertion failures). The correction
keeps the authenticated build-receipt hashes authoritative, checks current
bytes against them before creating output, and compares final bytes against
the same hashes. All 23 protocol/study tests pass with ResourceWarnings
promoted to errors; independent correction review accepted the exact final
source and replayed the wrong-helper refusal controls. This
closes a demonstrated baseline-promotion gap, not every possible transient
executable-replacement race.

## Corrected helper/accounting handoff verification

The controller authenticated all six changed helper/Cargo/accounting source
hashes and 84 retained native evidence artifacts. Native worker evidence contains
26 executed example tests (13 probe plus 13 helper) and nine native Python
controls. The controller's combined accounting/adapter/fixture suite passes
55 methods with ResourceWarnings promoted to errors.

An additional controller run through the final Python adapter records tiny
target peaks of 2,588 / 2,712 / 2,656 KiB for fresh/held/freed-parent states,
79,812 KiB for a real 64 MiB allocating target, then 2,580 KiB for the tiny
successor. Native helper and adapter identities remained unchanged. These are
explicit debug-helper calibration controls, not a release product workload.
Two earlier diagnostic attempts could not establish wrapper growth because
the diagnostic itself loaded a large interpreter for hashing before calibration;
streamed hashing corrected that diagnostic, and both attempts remain retained.

Independent final native and accounting reviews both accepted their frozen
components. The controller authenticated current reviewed source identities,
64 native-review artifact digests and the independent accounting RED/GREEN
raw log hashes (22 passes versus 28 intended failures, zero setup errors).
The native reviewer executed eight additional real process/failure controls.
The method now explicitly records the Linux-only default example-test inventory;
this does not reclassify the ProductDB library as Linux-only. The six oracle
ceilings and row-identity/encoding refusal boundaries are synchronized.

The maintained opt-in control command is now implemented and the controller's
own run passed calibration and actual outer interruption cleanup through the
unchanged final adapter/native helper. All recorded helper/target/descendant
PIDs are absent; both adopted descendants were reaped. The combined maintained
Python suites pass 61 methods with ResourceWarnings promoted to errors.
The two new control source hashes and original RED/GREEN evidence were verified;
independent review of those control files remains pending.

The independent maintained-controls review subsequently rejected four P2
integration gaps. Its real TERM-ignore escalation fixture left the helper,
target and descendant alive because a second readiness alarm interrupted
cleanup before SIGKILL. Its accelerated hanging-target timeout fixture killed
only the controller and left the separately sessioned measured group alive.
The reviewer fixtures killed/reaped both exposed groups and verified absence;
normal-path successes are not credited as coverage of these failure paths.
The proof also lacks executing-source start/end binding, and maintained actual
ECHILD coverage is still required. The controller authenticated 81 review
artifact hashes and rechecked all recorded review PIDs absent.

A focused two-file test-first correction is active: one-shot interruption,
owned controller deadlines/fallback cleanup, streamed source manifest and a
required actual native wait-failure control. No shared native/adapter/accounting
repair is implied. Original rejected proofs and leaking-fixture evidence stay
retained separately.

The private semantic-oracle DEFAULT adapter is corrected for distinct target
and supervisor PIDs, authenticated wrapped argv, all three build/start/end
hashes and the current hidden-rowid stream layout. The controller authenticated
the preserved 25-file original snapshot, final 24-test/26-assertion-failure RED
with zero errors, final GREEN logs, eight pilot verdicts and all 1,420 unchanged
original pilot files. Its own 24-method run passes, but reports three unclosed
SQLite connection ResourceWarnings during finalization; independent review is
checking that bounded fixture/resource issue. No warning-free or real DEFAULT
full-study acceptance is credited to this protocol-control run.

Independent private DEFAULT review found no producer/oracle protocol mismatch
and traced the three warnings to the private toy test's three connections,
not the oracle or maintained reader. A controller lifecycle harness reproduced
all three open connections before correction. The narrow test-only change
uses explicit connection closure while preserving setup transaction semantics.
Exact updated test bytes pass all 24 methods with no open connections or
ResourceWarnings, and still produce 26 intended assertion failures with zero
test errors against the preserved original oracle. The corrected oracle source
is unchanged. Original test bytes, warning logs and receipts remain preserved;
a scoped independent lifecycle correction review is pending.

The scoped independent lifecycle correction review accepted the exact final
test bytes: all 20 reviewed input identities were authenticated, two fresh
24-method runs pass without ResourceWarnings, and independent instrumentation
proves all four connections are closed before harness cleanup with setup
transaction exit preceding closure. F1 is closed; the oracle itself is unchanged.

The focused maintained-driver R1–R4 correction is implemented. The controller
authenticated both final source hashes and 540 retained artifact hashes. Exact
final tests produce eight semantic failures and zero setup errors against the
immutable original, and 19 passes with the native helper enabled against the
corrected source. The fresh standalone CLI runs all five required controllers;
all 35 recorded final-run PIDs are absent and independent failsafe cleanup was
not needed. The controller also rechecked 271 distinct PIDs across 21 retained
failure-fixture owners absent; historical original-source leak witnesses and
their failsafe cleanup remain explicitly separate.

The controller's combined default Python run discovers 69 methods: 65 pass
and four explicitly opt-in native methods skip without a supplied helper. These
skips are not native acceptance; the retained native-enabled 19/19 execution
supplies that separate proof. Native helper, adapter, shared wrapper and
accounting source hashes remain unchanged. The method now records all five
controllers and scoped source-binding/deadline/fallback cleanup behavior.

The corrected-driver independent review accepts R1, R3 and R4, but rejects one
remaining R2 process-ownership race. A delayed real PID checkpoint plus the outer
hard timeout kills the controller before the helper PID is persisted; fallback
then rejects the legitimate PID-less stage without cleaning its already spawned
group. Actual helper/target/descendant processes were alive when the maintained
main returned failure. The reviewer separately killed/reaped that exact group.
The controller authenticated 126 new review artifact hashes and rechecked all
365 recorded review PIDs absent. A focused two-file ownership correction with
a maintained delayed-checkpoint regression is active; no shared-wrapper change
is requested.

The older 533-entry artifact manifest has one documented generator-stdout
mismatch because its summary was printed after hashing. Its 532 other entries
match. The later final 540-entry manifest separately matches completely; neither
manifest is rewritten or conflated, and no raw measurement/source mismatch was
found. The normal and fully checkpointed timeout passes retain their exact scope.

The narrow R2 checkpoint correction is now implemented in the same two driver
files. It recovers a live execed helper only among the scoped owner's
kernel-listed direct children and verifies its full argv, unique receipt path,
cwd, start identity, adopted parent and PID = PGID = SID before group cleanup.
The maintained delayed-checkpoint test verifies no measured-group survivor
before independent cleanup and no signal to a separate same-helper distractor.
The controller authenticated both final source hashes and all 488 frozen evidence
artifacts, and rechecked 288 recorded PIDs absent. Exact final tests yield one
intended semantic failure and zero errors on the pre-correction driver, versus
20 native-enabled passes after correction. The default combined suite discovers
70 methods: 65 pass and five native opt-in methods skip without a helper.
All five standalone controllers pass; these remain debug synthetic controls.
No dead/pre-exec leader recovery guarantee is inferred from the live-helper case.

Independent narrow R2 review accepted the exact source. The controller
verified all 131 new review artifact hashes, the current reviewed inputs and
363 recorded retained/fresh PIDs absent. The fresh review run passes all
20 native-enabled methods without failures, skips or ResourceWarnings. Every
implementation blocker is closed within its documented scope.

This checkpoint freezes the reviewed tooling candidate for a local source
commit and the actual source-bound release build/study. Clean-source release
builds, calibration of that release helper, the repeated 54-observation study,
actual DEFAULT semantic verification, result review, scoped gates and publication
remain pending. Component acceptance is not WP-04.1 completion, product
compatibility or an accepted release-memory baseline.

## Actual release-study acceptance

The clean tooling candidate `9834daff` supplied offline/locked release CLI,
probe and native supervisor builds with authenticated unchanged executable
identities. The successor private study completed five release controls,
20 native-enabled methods, 54 measured operations over three repetitions,
and the independent DEFAULT XML/SQL/typed-stream semantic oracle.

Independent actual-study review accepts without findings: 108 accounting
endpoints and 30 final databases reconcile, all target proofs/commands match,
three expected nonzero refusals retain raw failure with exact preservation,
and the separate semantic/resource verdicts are not conflated. The controller
rehashes all 29 frozen review artifacts and 4521 original evidence files,
checks the current candidate/binaries and all 253 recorded PIDs absent.
The original missing-private-evidence-parent attempt remains retained as a
setup failure before workload execution, not a product regression.

Public aggregate results record the exact measured source and limits rather
than private paths or system details. The narrow factual/privacy review accepts the figures and scope; its sole
stale-opening finding was corrected to bounded synthetic-study acceptance. Additional release-profile example tests actually
pass 26/26 (13 probe,13 supervisor). These are separate post-study executions,
not a replay of the 54-operation measurements. Maintained branch and merged
package gates, publication/readback and evidence archival/retirement are pending.

## Source publication

Published main-first `0f44135b3e593c6d26c3e883695bab50b51b2ae1` with verified
local/tracking/live equality. The maintained integrated gate passes 16/16,
5,498 Rust tests (215 ignored),155/155 selected corpus and410 registered tool
methods (six optional skips); input identities are frozen and user-data guards
clear. The separate branch gate passes5,497 Rust tests and155/155 corpus.
Counts are not combined. No functional browser suite was selected. Raw study
measurements remain bound to9834daff; neither gate replays that study.

The first private final verifier incorrectly demanded a nonempty successful
formatting log. Its corrected projection permits silent fmt/diff checks while
requiring exact ordered exits/proofs, completed plan, frozen input identities
and checked user-data guards; no test or gate replay was needed. The raw gate
receipt remains unchanged. A result-only documentation gate, publication
readback, compact evidence archive and owned retirement remain pending.
