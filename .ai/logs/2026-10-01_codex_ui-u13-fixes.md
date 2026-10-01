# U13 — independent-review findings and offline discovery closure

## Review provenance and scope

The operator explicitly accepted the independent GPT-6.1-Sol report because
Claude was unavailable (user response: “ja, claude steht gerade nicht zur
verfuegung”). The archived report and its original session/revision provenance
remain in `.ai/logs/2026-10-01_codex_ui-u13-gpt-review-received.md`.
Its original verdict was **changes required**, with three P1 findings and
missing offline discovery transport coverage. The user accepted the reviewer
replacement, not unresolved defects. Invocation-only approval/lock receipt:
`83c217f2`. Implementation takes place exclusively in `ui-u13-fixes`.

The implementation session performed a separate full-diff review; that pass
is **self-review**, not a second independent whole-track verdict. No subagents
or Claude service retries. No productive KNX operation, live multicast,
programming-mode request, firewall modification, private-corpus write or
change to hardware confirmation/recovery gates.

## Fixes and root causes

1. **Selection/detail identity:** `selectEntity` updated selection but retained
   the old device detail during the asynchronous GET. Clear that detail in the
   same selection transition; reject mismatched detail IDs independently in the
   Properties Inspector and centre workspace. Retain request-ID/selection checks
   for out-of-order results. Tests cover pending and failed requests, successful
   replacement, and contradictory returned IDs without exposing old editors.
2. **Undo/Redo parameters:** the parameter GET effect depended only on device
   ID/product language. Pass the authoritative accepted `ProjectTree` snapshot
   from `DeviceWorkspace` as its refresh key. The same device refreshes after
   commands/Undo/Redo; existing request IDs reject older overlapping responses.
   No fabricated tree/history update, remount-per-tab or second persistence path.
3. **Autosave lifetime:** an already-running `runSave` could schedule another
   cycle in `finally` after effect cleanup. A local effect-lifetime flag blocks
   obsolete rescheduling after disable/unmount/cadence change; timer references
   and countdown are cleared. A rejected pending save still reports its failure.
4. **ISSUE-12 transport evidence:** privately extract the existing UDP exchange
   into `discover_on_socket`; production still supplies its unconnected wildcard
   socket, route-selected HPAI, multicast destination and ten-second window.
   A normalized comparison with the pre-change body confirmed identical
   exchange logic except injected destination/timeout. Two bounded non-skipping
   tests use only 127.0.0.1 and synthetic established codec-test DIBs. No public
   endpoint override, retry, packaging patch or newly assumed protocol behavior.

## RED/GREEN and negative controls

Behavioral RED before implementation:

- Device selection: 2 failures, old editable input still present after switching.
- Undo parameter display: 1 failure, value remained `6` instead of restored `5`.
- Pending autosave cleanup: 4 failures, obsolete timer remained or duplicated
  the newly configured cycle.

Initial discovery test compilation failed only because the private transport
seam did not yet exist; this is **not** a reproduced production discovery bug.
After extraction the two real UDP tests passed. The separate behavioral
negative control below proves the roundtrip is actually checking the shared
service-filtering path rather than only codec helpers.

Each temporary mutation failed its named regression and was restored:

| Mutation | Observed failure |
| --- | --- |
| Remove Inspector detail-ID guard | 1 assertion failure: wrong-device editor exists |
| Remove centre-workspace detail-ID guard | 1 assertion failure: wrong-device workspace exists |
| Remove autosave lifetime guard | 4 assertion failures: stale/duplicate timers |
| Remove parameter snapshot dependency | 2 assertion failures: old value and missing reload |
| Reverse shared discovery response-service filter | UDP roundtrip gateway equality failure; no-reply test still passes |

An initial Inspector mutation command used a nonmatching test-name filter and
ran no test; it is not counted as evidence. The corrected exact-name run above
failed the intended assertion. Restoration checks found each of the three new
identity/lifetime guards and the snapshot dependency exactly once. Subsequent
GREEN reruns are required before publication.

The additional parameter test initially asserted a program ID that the UI does
not render. The test now asserts the actual field caption and preserves the
new caption after resolving the older GET. No implementation was weakened.

## Discovery acceptance and limitations

The user-approved independent review accepts the previously documented host
firewall fix: UFW dropped a unicast response from UDP source port 3671; after
the operator's rule, unchanged CLI/HTTP discovery succeeded on 2026-09-30.
Manual numeric-IPv4 connection remains a first-class fallback. This package
neither applies nor broadens that firewall rule.

`client::tests::discovery_loopback_roundtrip_uses_advertised_hpai_and_filters_datagrams`
verifies the real SEARCH_REQUEST service/IP/bound-port HPAI, replies to that
advertised endpoint, filters bad frames/wrong services/invalid response bodies,
de-duplicates control endpoints, accepts multiple UDP reply sources and maps
gateway endpoint/address/name/tunnelling capability. No follow-up connect/send
is allowed by the test. `discovery_loopback_no_reply_returns_empty_at_the_deadline`
verifies the actual send and bounded empty result. Both have a two-second outer
deadline and no skip path.

This is **unicast loopback**, not multicast-loopback or real-network proof.
Native WebKitGTK Search, real screen-reader behavior and network-specific
routing/firewall/multi-homing checks remain disclosed verification boundaries.
RESEARCH §20.1 and KNOWN_LIMITATIONS §79 preserve them. No exact target identity,
whole-device recovery or productive commissioning-write approval is implied.

## Gate evidence

- Full Web after mutation restoration: 82 files / 1,312 tests; TypeScript
  has zero diagnostics and production Vite build is green.
- Mock-only Chromium: site 4, service-control 4, device-checks 4, monitor-control
  4, ISSUE-09 10, address-unavailable 4 passed. Every fixture uses local mock
  APIs; task-specific output directories avoid other sessions' artifacts.
- Branch Rust gates: 139 suites / 2,811 passed / 0 failed / 161 ignored /
  0 `SKIP:` or lowercase skipping markers. Both named discovery tests passed.
  Strict workspace/all-targets Clippy and fmt passed; the changed network crate
  was actually checked/compiled in a fresh task-owned target. Read-only
  OriginalData link and product-corpus environment prevented silent skipping.
- Branch layering, headers (366 valid / baseline ceiling 161), anchors
  (375 links / 226 Markdown files), corpus-gates and diff checks passed.
- Security scan of added code: zero matches for hard-coded secret literals,
  shell injection, eval or unsafe deserialization. Separate full-diff self-review
  found no remaining blocker; discovery-exchange body equivalence verified.
- Concurrent upstream commissioning commits changed backup-directory durability
  while the branch gates ran. Their handover was read; integrate them and rerun
  all required gates before publishing or releasing the Web lock.

## Final handover

Pending integrated gate verification, ISSUE-12 checkbox reconciliation,
publication/readback, Web-lock release and task-owned cleanup. Foreign root
changes and other worktrees remain untouched. The goal.md session owns global
statistics/triage/alpha/whole-goal review; commissioning retains its separate
hardware safety gates.
