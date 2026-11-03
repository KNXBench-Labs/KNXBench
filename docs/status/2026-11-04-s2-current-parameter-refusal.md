# S2-05 E13: malformed current parameter at mounted settlement — 4 November 2026

**Independently reviewed focused software proof; package acceptance pending.**
This test-only characterization exercises the existing current-request refusal
at the mounted terminal path. It changes no production behavior, source
admission, validator, timeout, schema or hardware permission, and does not close
all E13 or S2-05.

## Named gap and finite scope

The App already refuses `not-a-number` in the selected known numeric parameter
in `caption_both_families_preserve_current_request_dependency_masks_and_refs`.
This follow-up proves that the actual mounted worker maps that refusal to no
credit after an admitted start and a caption-only catalogue-source replacement.
It does not reimplement the App rule or claim a newly reproduced production bug.

Two new registered server tests in
`device_system2::tests::mounted_hardware::caption_mounted` cover **eight cells**:
standalone/retained selectors × namespaces20/23 × initially pending/marked.
The actual production prepare route remains retained-only; standalone reuses
the existing private fully prepared source seam before the same mounted start
admission. All cases use fictional unsaved projects and the existing socketless
hardware-kind simulated transport, not real hardware or a new public selector.

## Owning path and exact oracle

The real start is admitted with the original source, then the fixture pauses at
the first actual simulated data write. Managed catalogue replacement changes
only the selected caption and the winning source hash. Strict original-source
admission refuses. Before injecting unsupported current input, the same valid
current project must positively return `ProgrammingParts::ALL` through the
changed-caption comparison under project/catalogue ownership. An already
refused source cannot masquerade as this new current-request refusal.

The fixture then changes only the known current numeric parameter's raw value
to `not-a-number`, directly as unsupported fixture state. This is not a claim
that a UI or validated domain edit accepts that value. The same supervised
worker must genuinely finish with Verified transfer facts, while the final
programming flags are exactly `[marked,false,false,false,true]`: prior address
credit survives, pending address gains no new credit, all three memory claims
are pending and the medium flag remains true. The terminal receipt is
`sessionOnly`, not a transfer failure. No helper refusal assertion precedes
that terminal flag oracle.

Original observed-bundle bytes, Finished/Verified journal state, frozen download
facts, report/read-does-not-settle checks and reopened-journal equality remain
in the existing owning fixture. Original recovery binding and wrong-phrase
refusal are checked without performing a restore. Existing saved-project/E15
coverage remains separate and byte-unchanged; these eight new cells are unsaved,
not a new saved-history or physical recovery claim.

## Executed focused result and independent review

The parent ran `cargo test --offline --locked -p knx-server --lib caption_mounted`
with two test threads: **13 passed, zero failed, zero ignored**, 841 filtered,
239.31 seconds. This includes the two new leaves/eight cells and the eleven
pre-existing caption/E15 leaves; overlapping suites are not added as distinct
coverage. Both new tests passed; original fixture source identity was stable
through execution and the executed server test binary is retained with its hash.

The runner held the shared workspace lease and used an offline loopback-only
network namespace, DAC-bypass capability removal, isolated XDG homes and the
approved scratch root. This focused lane used no real bus, device, browser,
native GUI or private corpus. Independent review found no severity findings.
Its baseline and restored run each passed four selected tests: both new leaves
with all eight malformed cells plus two unchanged valid-source neighbors.

Two independently compiled terminal-worker variants replaced the final
refusal-to-NONE fallback with ALL or address-only credit. Each failed both
new tests at the actual terminal flag assertion while both valid controls
passed in the same binary. Setup positive-comparison, admitted start and
Finished/Verified assertions passed first; no compiler or timeout failure is
credited as a kill. Each variant reached only the initial pending namespace20
cell of each family before panic; the remaining six malformed cells are not
claimed as mutant executions. Restored production bytes and complete input
manifests match; four actual invocation binaries and raw results are retained.

Complete selected branch/main-first package gates and publication remain
pending. Source parsing/formatting was not runtime proof, and no original-source
semantic RED is claimed for already-correct production.

Evidence: `<profile-scratch>/e13-current-refusal-20261103/`, including
`focused-r1/receipt.json`, raw logs, `review-bindings.json`, `cases.json` and the
executed-binary binding. The independent review copy includes the actual public
base Git object, not only a supplied base identifier.

## Unchanged boundaries

The [caption-rule acceptance](2026-11-03-s2-caption-settlement-candidate.md) and
[separate E15 persistence proof](2026-11-03-s2-caption-persistence.md) retain
their own source/run identities. Broader current-input and terminal-lifecycle
crosses, E1–E17 adversaries, actual changed-source restore, races/read counts,
large-project measurements and hardware evidence remain open. No new start,
recovery, general equivalence, browser/native, release or deployment authority.

[Open task](../open-work/S2-05-result-and-settlement.md) ·
[E1–E17 obligations](../design/system2-settlement-source-equivalence.md)
