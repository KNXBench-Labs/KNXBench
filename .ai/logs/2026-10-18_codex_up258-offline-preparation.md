# UP 258D12 offline preparation

18 October 2026; codex. Owner authorised offline work through the hardware
boundary, reporter-run hardware operations only with separate explicit consent.
Canonical scope and mixed recovery decision:
[goal](../../docs/goals/up258-programming-2026-10-18.md).

## Implemented slice

The existing byte-order probe gains offline preview. ProductDB supplies a
separate System-2 source-preview type using the existing evaluator, placements,
metadata witness and numeric checks. App reuses device/request resolution; CLI
requires explicit field selection and returns before runtime/network creation.
No manufacturer-specific code, dependency, schema, live admission, image
construction or recovery guarantee was added.

The exact digest-pinned public Siemens test exercises both selected A/B values,
the control location, excluded alternate reference, all eleven evaluation
diagnostics, and continued image/live-probe refusal. Raw originals and identity
ZIP remain private. [Contract](../../docs/contracts/system2-source-probe-preview.md).

## Observed development checks

- Missing-API RED observed for `preview_system2_probe`, followed by four initial
  synthetic tests passing and both exact public-package tests passing.
- Missing-CLI-fields RED observed, followed by eight probe unit tests passing.
- Built-CLI fixture initially failed the store's `UnreachableDevices` validation;
  corrected the synthetic installation membership, not the validator. Both real
  binary tests then passed, including no sends to a supplied loopback endpoint.
- Review found a field could extend beyond the 16-bit address space. New runtime
  assertion failed before checked arithmetic was added. All six source-preview
  tests now pass, including last-octet acceptance at `FFFFh` and eight/nine-field
  selection bounds.
- Touched-crate all-target Clippy with warnings denied passed. Documentation and
  whitespace checks passed. Static added-line secret/shell/eval/deserialisation
  scan found no candidates.

These are focused checks, not whole-repository acceptance. Full regression gate
and publication are pending at this commit. Review was in-session self-review,
not an independent approval. Independent read-only source/recovery assessments
confirmed remaining dependencies; they are not hardware evidence.

## Exact remaining boundary

No read or write was sent. The identity ZIP contains no parameter bytes or
allocation recovery state. Writing identical bytes cannot be described as a
no-op or proof of the complete application download. An ordinary authorised
connection can mutate Verify Mode before the requested memory write, including
on reconnect; read-only sessions cannot write. A narrower first-write operation
needs its own verified setup/range/recovery contract, not a bypass of the full
System-2 admission guard. Original source encoding/patch/resource transforms
and full-download recovery remain unresolved.
