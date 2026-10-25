# Memory response service pairing acceptance — 25 October 2026

## Published source

`53400189e` is published on `main` with matching live and tracking-ref readback.
The main-first merge preserves upstream `1cc975ff0`, including the separately
delivered System-2 prestate observations. Only the two commissioning response
matchers, their shared private helper, test wiring, tests and contract/index are
changed by this increment. The [contract](../contracts/memory-response-service-pairing.md)
records the rule and boundaries.

A memory read or active-Verify write requires the response service and address
of its request. A numbered response of the other service is acknowledged but
cannot confirm the operation or become a refusal of the requested service.
The unchanged answer ladder waits for a matching response and otherwise returns
`NoAnswer`, without resending an acknowledged mutation. An empty matching
response remains `MemoryRefused`; mismatching matching-service write bytes remain
`ReadBackMismatch`. Read-back without active Verify inherits the same rule.

## Source and regression evidence

Application Layer v02.01.01 §3.5.3 (pp. 109–111), §3.5.6.2 (pp. 117–119),
§3.5.4 (p. 112) and §3.5.6.3 (p. 120) were text-extracted and inspected for the
separate response/Acon mappings. Read and write within one service legitimately
share their response PDU; no cross-service equivalence was established. This is
robustness against a synthetic nonconforming peer, not an observed device fault.

The test-local transport passes wrong and correct responses through the real
cEMI encoder and decoder before session ingress. Both service directions exercise
public reads and active-Verify writes, including wrong-only, wrong-then-correct,
wrong-empty and genuine same-service refusal/success controls. Frame contents,
sequence progression, acknowledgements and exact request schedules are checked.

- The original 20-test revision against original production plus module wiring
  passed eight controls and failed twelve intended semantic assertions. The
  corrected executable passed all twenty; parent retained-binary replay agreed.
- Review added eight characterizations: same-service wrong-address rejection for
  reads/active writes and successful/refused-other-service read-back without
  active Verify, both directions. Final execution passes all 28 methods. The
  original RED remains bound to its original test bytes, not these revised tests.
- Isolated removal of either address guard fails exactly its two new address
  tests (26 pass, two fail per variant). A broader service/address-acceptance
  mutant fails eighteen methods, including both unverified-write rejection cases;
  that is corroboration, not another original defect reproduction.
- Independent whole-feature review and narrow test/documentation re-review both
  approved. Parent authenticated the reviewed source and retained executables.

## Integrated acceptance on `53400189e`

All 14 stages selected by the maintained change-scoped package runner passed in
the planned order. Source commitments match at start/end; touched and unchecked
user-data lists are both empty.

- Rust: 3,207 passed, zero failed, 131 ignored across 17 result blocks, including
  all 28 pairing methods. Overlapping focused and owning-crate runs are not added.
- Private corpus: 91/91 across five selected targets in a loopback-only namespace,
  including the registered HTTP and CLI download cases.
- Formatting, Clippy, tooling, documentation/local-information, layering, headers,
  anchors, ledger, corpus registration and final diff checks passed.
- The frontend was freshly built for the desktop build prerequisite after the
  upstream UI changes. Browser and generated-binding execution were not selected;
  this is neither browser nor interactive desktop acceptance.

## Remaining boundaries

No physical device was contacted. No hardware/interface-coupler compatibility,
address-space aliasing, data loss, whole-device recovery, release or deployment
is established. Service selection, response-count policy, retry budgets, device
admission and other owners' delivered semantics remain unchanged. The wrapper
has no NAK-injection coverage; wrong-address read-back without active Verify is
covered by the common read rule rather than a dedicated fixture. KNX Secure
remains excluded. [PRG-01](../open-work/PRG-01-programming-gaps.md) and
[KL-007](../limitations/KL-007-bounded-commissioning-support.md) remain open.
