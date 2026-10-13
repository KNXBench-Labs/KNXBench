# Download-counter refusal policy

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-114)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 114. The Download Counter refusal is this project's own conservative rule, not a System B obligation

**Limitation.** `[C13]` `Downloader::partial_download`
(`crates/knx-net/src/commissioning/download.rs`) now reads
`PID_DOWNLOAD_COUNTER` from the Device Object before its first write and
refuses the partial download — returning `DownloadError::
DownloadCounterChanged` or `DownloadError::DownloadCounterUnavailable`
rather than proceeding — for two of `DownloadCounterCheck`'s five outcomes.
**Both consequences are Coupler Model 2.0's own, not System B's.**
CP §3.12.5, p. 100 (refusal on change) is written for Coupler Model 2.0's
own download procedure (CP §3.12.4, Filter Table and Router Object,
neither of which this project implements as a distinct procedure).
RES §5.3.2.2, p. 320 (refusal on absence) sits in RES §5.3 *"Resources for
Coupler Model 2.0"*, not System B — a full-text search of RES for *"shall
not perform a Partial Download"* finds this one occurrence, nowhere else.
**There is no System B clause requiring either refusal**; an earlier
revision of this entry and of `docs/IMPLEMENTATION_STATUS.md` said
otherwise and was wrong, corrected in review round 1. This module applies
both Coupler-Model-2.0-native consequences to the one generic CP §3.5.3
five-variant procedure it actually runs, for every part kind, rather than
building a second, Coupler-Model-2.0-specific download procedure that
CP §3.12 alone would otherwise call for — and, for the profiles this
project actually targets (System B in practice), that is now this
project's own conservative ruling, not the Standard's.

**Why keep it anyway.** Without a comparable Download Counter, the MaC
cannot establish that the device is untouched since the last
configuration it performed. CP §3.12.5's and RES §5.3.2.2's answer to that
uncertainty — refuse the partial download, run a complete one instead —
is the safer of the two available answers, and this project ranks data
integrity above the convenience of a partial download. That ruling does
not change; only its citation does.

**The practical consequence.** `[C18]` established that PID 30 is
unlisted — hence optional — for the S-Mode End-device Device Object,
Volume 6 Annex A A.2.3, pp. 138-140, which covers System B's masks 07B0h
and 17B0h along with every other S-Mode End-device system. **A conformant
System B device with no Download Counter will be refused every partial
download it is ever asked for, and will always get a complete one
instead.** That is not a bug in this project or a defect in the device —
it is what applying a Coupler Model 2.0 rule to a System B device,
conservatively, on purpose, actually does to it.

**A second simplification, same direction.** RES §5.3.2.2, p. 320 refuses
on absence *"of the part to be downloaded"* — per part, not per device —
and RES §4.2.30.1, p. 41 allows a downloadable part its own Download
Counter instance distinct from the Device Object's, or none at all. This
module's *absent* check reads only the Device Object's instance
(`OI = 0`), the same instance CP §3.12.4/3.12.5 read for the *changed*
check (which RES §4.2.30.3, p. 42 backs: an unchanged Device Object
instance lets the client conclude no other instance changed either).
Nothing backs doing the same simplification for *absence*. It is recorded
here rather than fixed by a per-part-instance lookup, because it errs
toward refusing more often — the safe direction — and because per-part
instance mapping (which object index owns which part's Download Counter)
is a larger change than this task's scope.

**One more case, deliberately not a refusal.** `DownloadCounterCheck::
NoStoredCounter` — this MaC never called `DownloadPlan::
with_stored_download_counter` for this plan — proceeds rather than
refusing. CP §3.12.5's comparison needs a value to compare against; with
none stored, there is nothing to have changed from, so nothing in either
clause is violated by proceeding. It is a gap in this application's own
bookkeeping, not a fourth Standard outcome, and treating it as a refusal
would invent an obligation neither clause states.

**Cause.** Nothing in this codebase distinguishes a System B device from a
Coupler Model 2.0 device at plan-building time (masks 07B0h/17B0h vs.
2920h, `[C13]`'s own research), and CP §3.5.3 itself, the clause this
sequencer transcribes step-for-step, "imposes nothing" about the Download
Counter at all — the obligation is bolted on from two clauses written for
a different procedure than the one this project actually runs. Building a
genuine second procedure for Filter Table/Router Object downloads was not
this task's job; refusing an unsafe partial download was.

**Impact.** A target this project cannot yet tell is Coupler Model 2.0
gets Coupler Model 2.0's own refusal behaviour applied to it regardless —
which for a System B device with no Download Counter (the common,
conformant case, per `[C18]`) means partial download is unavailable in
practice. Nothing here "falls back": `partial_download` returns `Err` and
this project has no caller that automatically retries as a complete
download — the caller must run one itself. A real Coupler Model 2.0
Filter Table download (CP §3.12.4's own steps, which this project has
never implemented) is also not being run under the name CP §3.12 gives it.

**Lifted when.** A device-profile model exists that can tell a Coupler
Model 2.0 target from a System B one at plan-building time, so the
Coupler Model 2.0 consequences apply only where CP §3.12 actually asks
for them; and CP §3.12.4's Filter Table/Router Object procedure is
implemented as its own `Procedure`, distinct from the CP §3.5.3 one this
module runs today. Per-part Download Counter instance mapping, for the
second simplification above, can be lifted independently of either.

**Accepted boundary, 2026-09-28 (goal-commission K8).** User decision
2026-09-28, asked for each K8 entry to fix it or accept it: *"das was am
sinnvollsten ist"* (whatever makes the most sense), i.e. the recommendation.
The conservative Download Counter refusal stays; a device-profile model that
can tell Coupler Model 2.0 from System B reopens it.
<!-- END retained catalogue record -->
