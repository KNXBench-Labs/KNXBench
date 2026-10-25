# Load-control response acceptance — 25 October 2026

## Published source

`e4f5e7c6f` is published on `main` with matching live and tracking-ref readback.
Main-first parents are upstream `d30681c17` and feature `8d6bde1b7`. The delivered
region reader and other owners' reserved paths are unchanged. The contract index
preserves both additive entries; removing only the new row recovers the upstream
index exactly. The [contract](../contracts/load-control-response-state.md)
records the rule and limits.

The property-load-control write response is an actual load-state observation.
It is validated before verification polling, and a changed LoadCompleting
response seeds the existing conditional silence/reconnect wait. No state is
inferred from an absent reply. Invalid width, unknown values and forbidden
changed states fail before later polls can hide them. An unchanged answer stays
pending; even a settled answer retains the verification poll. Existing deadlines,
one-extra-attempt policy and memory-mapped three-read mechanism remain unchanged.

## Primary-source and regression evidence

The original Resources v01.10.01 §4.23.2.3.2, printed p. 294, requires the
LoadCompleting response before an offline completion interval. Management
Procedures v02.01.02 §3.31.3, printed p. 138, defines the state-bearing write
response; Resources §4.23.2.4.1, printed p. 297, supplies the existing wait rule.
Those passages were independently text-extracted and inspected. A rendered-page
viewer failed, so no visual-inspection claim is made.

The original simulator always answered with its final underlying state. An
opt-in response fixture now models the intermediate observation without changing
default behavior. The tests independently inspect the emitted response, call the
public write method and check silence/reconnection, timeout, event sent once,
malformed/unknown/forbidden responses, mask restrictions and unchanged/settled
controls. This remains a synthetic mechanism test, not physical-device evidence.

The retained original RED executable fails four of seven methods; corrected
execution passes seven. The original test bytes changed during development and
are not relabelled as final. A separate retained compilation of the pre-review-
nit final tests against original production plus module wiring also fails the
same four methods (three pass, exit 101), with exact source/binary bindings.
The later review amendment makes the allowed Error outcome explicit and passes
all seven methods; it does not change production. A proposed Loaded expectation
was rejected by the actual fixture and corrected to Error without altering it.
Independent source review approved the runtime; parent checks closed its small
test/documentation notes.

## Integrated acceptance on `e4f5e7c6f`

All 14 stages selected by the maintained change-scoped package runner passed in
the planned order. Start/end source commitments match; touched and unchecked
user-data lists are both empty.

- Rust: 3,152 passed, zero failed, 131 ignored across 17 result blocks, including
  the seven load-response and eleven region-read methods. Earlier focused
  runs overlap and are not added to this count.
- Private corpus: 91/91 across five selected targets in a loopback-only network
  namespace, including all 29 HTTP download and nine CLI download cases.
- Formatting, Clippy, documentation/local-information, layering, headers,
  anchors, ledger, corpus registration, tooling and final diff checks passed.
- Browser/web and generated-binding execution were not selected. The desktop
  build prerequisite was present, not an interactive desktop acceptance.

## Remaining boundaries

No public API, device-family admission, new bus operation, recovery procedure,
release or deployment was added. No physical device was contacted. The existing
retry classifier and authorization rules remain. A later poll determines the
final result; contradictory observations are not a guarantee of stable runtime.
KNX Secure stays excluded. [PRG-01](../open-work/PRG-01-programming-gaps.md) and
[KL-007](../limitations/KL-007-bounded-commissioning-support.md) remain open.
