# 2026-09-28 — Claude — K1: commissioning documents corrected

Task: `goal-commission.md` K1. Several passages still said that no device had ever received a write, which has been false since 2026-09-26.

## Corrected (in place, dated, no contradiction appended below)
- KNOWN_LIMITATIONS §7: "**Limitation.** The application does not program devices" plus a "Corrected 2026-09-28" block. The phase-2 closing sentence and the Lifted-when "first real write" paragraph are marked *overtaken*.
- KNOWN_LIMITATIONS §92: title changed, old anchor kept as `<a id>` (7 incoming links: IMPLEMENTATION_STATUS, GAP, KL §7, ROADMAP, 3× manual).
- GAP_ANALYSIS_ETS E1: status and closing condition.
- ROADMAP Session 7 and the "T30 phase 3" decision row ("decision pending", per goal-commission K1).
- Commissioning design spec: status update.

## Facts, from the repository
- IA write `03e358f` (2026-09-26), explicitly authorised (RESEARCH §8.8.6).
- Download 2026-09-28, merged `95a862c` (RESEARCH §19.4).
- Settling retry `be91fe3`: simulator-verified only.

## Not changed (not a commissioning row)
`docs/manual/*` (3 pages) is still out of date; it is in the handover under "For the goal.md session".
