# Archive — alpha 0.1 goals and their evidence dossiers

Read-only. Moved here verbatim on 2026-10-08 (only relative links were
adjusted, by script) because all three alpha goals are finished: every package
is delivered or carries a recorded user decision, and `v0.1.0-alpha.5` is
published as a pre-release. Older handovers, logs and ADRs cite these files;
keep them for provenance. **Do not record new status here.**

Per-ID status stays in the live [source-ID ledger](../../status/LEDGER.md)
(ADR-0076). What is still open after the alpha is collected in
[OPEN_WORK](../../OPEN_WORK.md).

## Goals

| File | What it was | Closed by |
| --- | --- | --- |
| [alpha-release-goal.md](alpha-release-goal.md) | Alpha-readiness goal (AR00–AR21), non-UI, non-commissioning | AR19 `DONE` 2026-10-07 (alpha.4, replaced by alpha.5). Four unchecked boxes in AR03/AR12 are user-deferred (`KL-129`, `KL-135` `ACCEPTED_BOUNDARY`); one in AR14B is an optional live run that needs a user go. |
| [goal-ui.md](goal-ui.md) | UI/UX track (U0–U21, UA1–UA8, theme packs, telegram flow) | Owner closure receipt 2026-10-06; no open package |
| [goal-commission.md](goal-commission.md) | Commissioning track (K1–K19) | K1–K19 delivered at their scope; user scope decision 2026-10-04 removed new hardware/vendor/ETS/power-loss validation. Its §3 recovery contract and §3c later goals are carried in [OPEN_WORK](../../OPEN_WORK.md). |

## Evidence dossiers and reviews

| File | Content |
| --- | --- |
| [ALPHA_READINESS.md](ALPHA_READINESS.md) | Alpha package evidence (AR packages) |
| [UI_ALPHA_READINESS.md](UI_ALPHA_READINESS.md) | UI owner evidence and closure receipt |
| [COMMISSIONING_ALPHA_LEDGER.md](COMMISSIONING_ALPHA_LEDGER.md) | Commissioning evidence, fallbacks and dispositions |
| [ALPHA_SCOPE_MATRIX.md](ALPHA_SCOPE_MATRIX.md) | AR15 scope, risk and decision matrix |
| [LIMITATION_TRIAGE.md](LIMITATION_TRIAGE.md) | AR15 criticality triage of `KNOWN_LIMITATIONS` (2026-10-06 count) |
| [MANUAL_ACCEPTANCE.md](MANUAL_ACCEPTANCE.md) | AR16 manual acceptance checklist |
| [ALPHA_CANDIDATE.md](ALPHA_CANDIDATE.md) | AR17 local AppImage candidate |
| [ALPHA_FINAL_GATES.md](ALPHA_FINAL_GATES.md) | AR18 gates, AR19 release and the alpha.5 replacement |
| [ADR0039_ENFORCEMENT_AUDIT.md](ADR0039_ENFORCEMENT_AUDIT.md) | AR03 audit of ADR-0039 phases 3–5 |
| [review/](review/) | AR18 independent review, briefs and recheck rounds 2–4 |

## Dated snapshots

| File | What it was |
| --- | --- |
| [PROJECT_STATS_2026-10-03.md](PROJECT_STATS_2026-10-03.md) | Former root `stats.md`; the maintained statistics page is [ProjectStats](../../ProjectStats.md) |
| [ETS6_COMPARISON_2026-10-02.md](ETS6_COMPARISON_2026-10-02.md) | Former root `compare.md`; current claims live in [COMPATIBILITY](../../COMPATIBILITY.md) and [GAP_ANALYSIS_ETS](../../GAP_ANALYSIS_ETS.md) |
