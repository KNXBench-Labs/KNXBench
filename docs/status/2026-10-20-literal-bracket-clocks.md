# Literal-bracket browser clocks — 20 October 2026

## Contract and implementation

Recognise unescaped literal `Date["now"]()` and `performance['now']()` with
either quote and whitespace/newlines in code, including page function callbacks.
Find the receiver/opening bracket in already masked code and inspect only the
adjacent raw literal key and call. Do not globally unmask strings or change the
existing parser/helper fixed point. Findings retain original offsets, lines
and quote style. Existing tag admission and helper/import propagation apply.

This is a bounded developer safety net, not an adversarial JavaScript parser.
Computed/escaped keys, template-literal properties, getter/destructuring/alias
resolution and bracket calls inside decoded page-code strings are not added.
Ordinary non-call properties remain ignored; no new framework or dependency.
No project/domain, storage, protocol, compatibility, lane-selection, retries,
budgets or gate-admission change. No architecture/schema ADR is needed.

## Focused evidence (not delivery acceptance)

- Final test bytes against unchanged `b481842f` guard: 43 test methods execute,
  85 assertion failures across seven named methods, zero errors. The failures
  are missing findings, not setup/import failures. One helper expectation was
  corrected before implementation; original attempt retained separately.
- Candidate and controller reruns: all 43 methods pass (31 existing + 12 added).
  Existing test source is byte-preserved after removing the bounded insertion.
- Both clocks, both quotes, spaces/newlines, function callbacks, template code,
  transitive local/imported helpers, tag/describe positives, unrelated callers,
  module-level helper refusal, exact diagnostic lines and malformed input.
- Text-only comments/titles/strings/regex/template text remain ignored;
  computed/escaped/non-call and similarly named receiver controls stay ignored.
- Actual repository guard: zero untagged uses in 90 spec files. This is guard
  execution, not browser execution. Added-line security scan and whitespace pass.

Independent whole-change review accepted with zero Critical/Important/Minor
findings. Its separate execution passes 43 focused methods and 121 production
probes; all seven reviewed input hashes match before/after. Initial reviewer
module-import setup failed before tests; retained separately, not product RED.
The maintained merged package gate and publication remain pending.
`tools/browser_lanes.py` selects the whole default browser lane; focused Python
passes do not waive it. Publication/readback, archive verification and owned
retirement must be recorded only after execution.

[Contract](../contracts/web-test-gate.md) ·
[Detector](../../tools/browser_lanes.py) ·
[Tests](../../tools/tests/test_browser_lanes.py).

## Verified delivery and retirement

Delivered as main-first `c7a6aa8e` (feature `0a3ce3aa`) onto `b481842f`.
Maintained change-scoped gate: all 11 parent and 29 web stages pass, including
build, 22 derived typechecks, 4,136 Vitest cases and the complete 863-case
default browser inventory (798 functional / 65 exclusive). All 340 registered
tooling tests complete with one pre-existing optional skip. No production
suites, Rust runtime, private corpus or release gate was selected.

Controller checks require exact ordered stages, valid proofs, frozen input
fingerprints, unchanged build/tool identities within the run, and empty
parent touched/unchecked-user-data lists. Compiled browser identities match
both lanes exactly; no retry, skip or invented empty lane. Local, tracking
and live publication readback agreed. Source/test bytes match the review.

The publishing wrapper initially refused because it stripped the captured
handover's trailing newline on only one side. Exact raw blocks were identical;
the original refusal remains retained. Subsequent re-verification observed a
local Node update from v26.7.0 to v26.8.1 **after** the accepted gate. Retained
acceptance remains bound to the measured v26.7.0 start/end identity and unchanged
source/build/dependency inputs; it does not certify execution on the later
runtime. A separate retained-evidence verifier reconciles this distinction;
no runtime test was replayed or relabelled to repair publication bookkeeping.

The private compact archive was verified member-by-member: 256 evidence files
plus one manifest. Only the implementation worktree, two ancestor-checked
merged branches and owned scratch were retired; the rebuildable target stays
only through the result-documentation gate. The selected stale/dirty checkout,
foreign research and other owners' work were preserved. Claim released.
