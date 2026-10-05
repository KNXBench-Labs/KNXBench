# AR15 slice 2: recount and reconciliation (Claude, 2026-10-06)

## Recount (reproducible)
```python
import re
kl=open('docs/KNOWN_LIMITATIONS.md').read()
nums=[int(n) for _,n in re.findall(r'^## (§)?(\d+)[. ]',kl,re.M)]
tri=[int(x) for x in re.findall(r'^\| (\d+) \|',open('docs/LIMITATION_TRIAGE.md').read(),re.M)]
sign={18,23,24,42,90,95,149,150,152,156}   # plus the duplicate 130-GATE
lim=set(nums)-sign
assert len(nums)==119 and len(lim)==108 and len(tri)==107
assert sorted(lim-set(tri))==[105] and not set(tri)-set(nums)
```

## Triage decisions
- New K3: §151 (large packages CLI-only), §153 (scheme-10 semantics by name
  evidence), §154 (flow view Chromium-only, motion cost), §155 (bridge tunnel
  needs Route Back), §157 (in-place upgrade).
- Signposts: §149, §150, §152, §156 (lifted; evidence in the entries/ledger).
- §29 K3 -> K4 (only `route-send` lacks `--project`). Reasons refreshed for
  §9, §11, §16, §60, §65, §128, §142.
- Ledger-DONE but kept: §61, §82, §86, §87, §121, §124 — their residual
  boundary still exists (triage rates the residue, not the work item).

## Reconciled claims
Scheme set 10–14/20/21/23; store schema 10, ProductDB 21; commissioning now
points at KL §7 in ARCHITECTURE §8 and COMPATIBILITY.
