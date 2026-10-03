# 2026-10-03 — Claude: plan crawler-corpus findings (AR06P)

- Input: KL-149..153 from `2cceea4e` (853 public product downloads, test only).
- Decision: new package AR06P before AR07 rather than reopening the DONE_SCOPED AR06;
  post-snapshot IDs are kept outside the 180-ID ledger (section 8 / separate ALPHA_READINESS section).
- Priorities (OFFENE_PUNKTE scale): KL-150 P1 (crash on a real product), KL-149/151/152/153 P2.
- Extra read-only scan: 1/852 ZIP packages nests ModuleDef (MDT RF-TAL55, the KL-150 file);
  10 packages / 1,070 NumericArg AllocatorRefId uses. Reported as R-MODULE-03/04 unblock input; status left to AR07.
- Caveat learned: "AllocatorRef" is an attribute (`AllocatorRefId`), not an element. An element search gave a false 0.
- Not changed: 180-ID ledger, status counts (pre-existing drift noted for AR15), code.
