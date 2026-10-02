# ADR 0060: Versioned declarative theme packs, never arbitrary CSS

Date: 2026-10-02

Status: Accepted — U14 researched/reviewed contract; implementation pending U15–U18.

## Context

The published UI goal adds user-importable themes to an existing five-palette
registry, System light/dark resolution, ADR-0022 token/contrast boundary and
server-owned settings document. Language packs provide useful UI gestures but
not suitable theme validation or durable acknowledgment semantics. No KNX
project/manufacturer/domain type needs to change for a presentation preference.

Security-sensitive facts, primary sources, inspected code and the complete
negative-fixture matrix are recorded in [THEME_PACKS](../THEME_PACKS.md).

## Decision

Define `knxbench-theme` JSON formatVersion/tokenVersion 1. A complete 27-token
allow-list and strict full-value grammars cover hex colours, bounded gradients,
shadows/radii, installed fonts and backdrop values. Reject unknown fields,
versions, unsafe values and all aliases as a whole; no selectors, CSS rules,
HTML, URL/font assets, inline executable content or built-in ID shadowing.
Duplicate-aware text parsing must precede typed validation. Raw imports and
stored/cache values receive the same validation before any DOM effect.

Extend the existing theme path through bounded root `style.setProperty`
assignments; track and restore only owned properties and priorities. Preserve
ADR-0022's exact three role pairs and unrounded 4.5:1 evaluator for base and
accent palettes; do not label this full visual/AT acceptance. Independent
motion/density/project preferences cannot be pack tokens. Rejecting aliases
rather than adding a resolver keeps v1 smaller and removes missing/cyclic
reference ambiguity entirely.

Keep `uiThemePacks` and selected `theme` in the existing settings document.
No schema bump: its v1 opaque preference map already supports these additive
keys. A narrowly added conditional multi-key settings patch, using the
existing queue/route/write lock, prevents stale theme-map/selection updates
within the single server. Durable acknowledgment precedes success/cache
adoption. Preserve invalid entries and unknown settings; refusal of newer
settings remains fail-closed. No new generic persistence framework.

Preview overrides the existing presentation path without writing settings;
Cancel/Escape/close/unmount/failure restore the latest acknowledged state.
Authoritative theme changes cancel stale preview. Removing the active pack
atomically selects System. Exports contain only validated installed packs;
complex built-in backdrops are not misrepresented as lossless v1 exports.

## Alternatives

- Arbitrary CSS files/rule injection: rejected; much wider trust boundary and
  independent motion/density controls could be overridden.
- Browser-only pack storage: rejected; server settings are already authoritative.
- Reuse permissive language-pack validation: rejected; an incomplete palette
  cannot safely follow English-style fallback semantics.
- Token aliases or a general CSS parser: unnecessary for v1; explicitly rejected
  instead of depending on the browser to discover invalid/cyclic values.
- Separate theme service/domain schema: unrelated to KNX, unnecessary abstraction.

## Consequences and acceptance

The explicit grammar does not represent every possible CSS paint effect. This
is a disclosed format boundary, not silently stripped data. A future token
boundary change needs an explicit compatible tokenVersion policy. Metadata and
accepted values roundtrip; rejection leaves state untouched. Unknown/newer
stored entries remain recoverable and diagnosed. Conditional writes do not
promise multiprocess filesystem collaboration or project collaboration.

U14 accepts only the researched/reviewed contract and test matrix. U15–U18
must supply actual runtime, persistence, browser, mutation and integrated-gate
proof. Built-ins remain available as known-good fallback. Native WebKitGTK,
Orca and whole-app WCAG acceptance remain independent requirements; no alpha
release or hardware operation is authorized by this decision.
