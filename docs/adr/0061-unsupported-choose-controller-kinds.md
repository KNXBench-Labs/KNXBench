# ADR 0061: Unsupported declared choose controllers fail closed, not as missing references

Date: 2026-10-02
Status: Accepted — bounded controller public, selected private and integrated gates verified; remote publication readback pending.
Session: 4 (manufacturer semantics), AR07
Amends: Dynamic design D7/D9 and ADR-0041 structural-refusal coverage.

## Context

[Parameter semantics boundary](../PARAMETER_SEMANTICS_BOUNDARY.md) records the
fresh primary-source read and current parser/evaluator path. The local licensed
Project Schema23 v01.00.00 PDF, printed/PDF page 30/64, section 1.1.3.18
`Condition_t`, names `TypeNumber` and `TypeRestriction` as controllers. Its
integer forms do not authorize other parameter kinds merely because their
stored value looks numeric. This is a shared-simple-type constraint, not a
claim to implement the full manufacturer Dynamic XSD or ETS interpreter.

`resolve_control_kind` previously mapped every stored non-None kind to
Comparable. A synthetic ingest/load/evaluate RED on 2026-10-02 demonstrates
a Text declaration with value `1` activating a matching branch; positive
Number/Restriction and missing-declaration controls pass (2 passed, 1 failed,
exit 101). Unknown kinds are not missing declarations: their catalogue kind
and original source bytes already exist and must remain distinguishable.

The existing TypeNone sole-default exception is separately corpus-derived
(design D9), not normative. This decision neither widens nor removes it.
Existing ADR-0041 skipped-reference reporting and activation budgets provide
an established structural-refusal mechanism. No second evaluator is needed.

## Decision

- Only stored Number and Restriction map to Comparable. None retains the D9
  policy. Every other stored kind maps to `ControlKind::Unsupported(String)`,
  preserving the catalogue kind exactly. The runtime enum is no longer Copy;
  it is borrowed when dispatching a choice. Missing declaration chains remain
  `None` and produce UnresolvedParamRef, never a misleading unsupported tag.
- Unsupported declared controllers activate neither matching nor default
  branches. Emit `Diagnostic::UnsupportedControlKind` with choice node,
  controlling reference and stored kind. Enumerate refs in all skipped branches
  via ADR-0041 without activating them; retain module scope, deterministic order
  and the existing shared activation budget. Continue independent siblings.
- This diagnostic may hide refs. Consequently, communication-object projection
  must retain Undetermined rather than infer Inactive from a structural refusal;
  download planning must not mistake it for the allowed NoBranchMatched state.
- The server exposes a distinct `unsupportedControlKind` warning, fixed English
  fallback and the existing detailed diagnostic field. Its hand-mirrored Web
  kind union/message catalogue is owned by the UI track. Do not edit Web files
  or generated bindings here: the existing unknown-kind fallback in
  `ParameterPanel.tsx` preserves visible English warning text until the owner
  adds the token to `api.ts` and both language catalogues. That adoption and
  localized rendering remain an explicit handoff, not completed UI acceptance.
- No native-domain, ProductDB schema, migration, raw source or per-kind write
  validation changes. Read-only inspection preserves original type/source
  evidence, including Other when the parser does not model the type child.

## Alternatives considered

- Numeric-looking values from any kind: contradicts the documented declaration
  constraint and reproduces the RED; rejected.
- Report UnresolvedParamRef for a known declaration: conflates missing and
  unsupported evidence; rejected.
- Reuse an unrelated diagnostic tag or silently choose the default: misleading
  or invented semantics; rejected.
- Expand the allowed types from Value_t's broader encoding table: value encoding
  is not permission to control Condition_t; rejected.
- Remove the TypeNone exception or implement Repeat/Allocator/vendor handlers:
  unrelated scope, insufficient evidence; rejected.

## Consequences and verification

Previously accidental visibility for an unsupported controller becomes a named
structural refusal, not a compatibility guarantee. Require source-backed
positive/negative regressions, retained-source reopen, missing-versus-unsupported
identity, scoped diagnostics, projection Undetermined and server warning/wire
shape tests. Mutation controls must show the declaration guard, diagnostic,
skipped-ref inventory and may-hide classification are observable. Full gates,
selected authorized corpus, review and integration remain separate from the
initial RED and baseline. No full ETS parity or new vendor execution follows.
