# Declarative procedure version writes

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-093)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

<a id="93-knx-cores-declarative-procedure-model-still-writes-pid_program_version-unconditionally-for-every-part--parked-deferred-to-task-c11"></a>

## 93. `knx-core`'s declarative procedure model still writes `PID_PROGRAM_VERSION` unconditionally, for every part — PARKED, unowned

**Limitation.** `crates/knx-core/src/commissioning/procedure.rs`'s
`load_one_part()` renders CP §3.5.2 step 06 as a fixed, five-step-plus-two
list, dry-runnable without a bus (spec §11.2's *"a procedure model, not a
script"*). Its step 5 is unconditional:
`step(5, "set the version", "PropertyWrite PID_PROGRAM_VERSION",
StepEffect::Write)`, for every loadable part, every time. Task C1 taught the
*executed* procedure in `crates/knx-net/src/commissioning/download.rs` to
know better: RES Table 90 (p. 288) and Table 91 (p. 290) give
`PID_PROGRAM_VERSION` to Application Program 1 and 2 only, and RES Table 77
(p. 238), Table 80 (p. 249) and Table 85 (p. 270) do not give it to the
Group Address Table, the Association Table or the Group Object Table —
`download.rs`'s `PartKind::has_program_version()` is that fact, and
`load_one_part` (the `knx-net` function, not the `knx-core` step-list
builder that shares its name) now writes the property unconditionally only
for the two kinds that have it. The declarative model in `knx-core` was not
told. It still renders step 5 as a `Write` for a part it cannot know is a
table, so a dry run of `procedure::load_one_part()` and a real
`download::load_one_part()` no longer agree on what step 5 does for three
of the five loadable parts.

**Cause.** `PartKind` lives in `knx-net`, one layer above `knx-core` in this
project's dependency direction (`knx-core` is the domain the rest of the
crates depend on, not the reverse — see `CLAUDE.md`'s architecture section).
Teaching `procedure::load_one_part()` about `PartKind` would make `knx-core`
name a `knx-net` type, which points that dependency backwards. Whether the
fix is moving `PartKind` down into `knx-core`, parameterising the step list
by an existing `knx-core` concept, or something else is a design decision,
not a bug fix, and task C1's brief explicitly scoped it out: *"do not build
`PartKind::ALL`... those are parked to C11 and C12 on purpose, because
building them now means guessing their shape twice."* This entry records
the resulting model disagreement as a **finding, deliberately not fixed
inside C1** (its audit number there is F8).

**Consequence.** Nothing that reads `download.rs`'s trace is wrong — the
executed procedure is the one with the correct, per-part-kind behaviour,
and its `VersionOutcome`/step-5 label say so accurately. What is wrong is
trusting `knx-core::commissioning::procedure::load_one_part()` alone, without
cross-checking `download.rs`, to describe what step 5 does for a table
part: the declarative model currently overstates it, still showing an
unconditional write where the real one is conditional or tolerant of a
refusal.

**Lifted when.** Corrected 2026-09-20: task C11 merged without touching
this — its actual scope was CP §3.5.3's five partial-download step-list
variants (`crates/knx-core/src/commissioning/partial_download_variant.rs`),
a different finding entirely, and task C12 (which followed it, wiring the
sequencer to that variant table) didn't touch it either.
`procedure.rs`'s `load_one_part()` step 5 is still the unconditional
`PropertyWrite PID_PROGRAM_VERSION` this entry originally described (see
that function, currently around lines 372-373). No task currently owns
the dependency-direction decision between `knx-core` and `knx-net` for
`PartKind` that this fix needs — it is unowned, not merely unscheduled.
Lifted when a task is opened for it and makes that call, or when
`knx-core` gains its own concept for "does this part kind carry a program
version" that `PartKind` could be expressed in terms of instead.

**Kept parked, 2026-09-28 (goal-commission K8).** User decision 2026-09-28,
asked for each K8 entry to fix it or accept it: *"das was am sinnvollsten
ist"* (whatever makes the most sense), i.e. the recommendation. Parked stays
parked: the v1 path (K9) uses `run_memory_download`, not the declarative
procedure model, so this does not reach a device. It reopens with any
feature that runs `procedure.rs` against hardware.
<!-- END retained catalogue record -->
