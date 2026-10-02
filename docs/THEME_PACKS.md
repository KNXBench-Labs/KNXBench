# KNXBench theme packs v1

Contract resolved by U14, 2026-10-02. **U15 runtime foundation implemented in
the candidate; integrated acceptance and U16–U18 remain pending.**
This document defines a KNXBench-owned format, not an existing interoperability
standard or a claim that the application already imports themes. Decision:
[ADR-0060](adr/0060-versioned-declarative-theme-packs.md).

## Runtime implementation boundary

`themePack.ts` admits original JSON text through byte, duplicate, depth and node
limits, then validates the complete envelope, metadata, token/value grammars,
accent maps and existing unrounded role-pair contrast. Accepted values retain
their original spelling. `readThemePackStore()` revalidates settings/cache data,
retains diagnostics and does not mutate rejected entries. `themePackDom.ts`
revalidates before its reversible inline-property lease; `theme.ts` integrates
it with the existing OS, accent and settings lifecycle without persisting a
visual fallback over an absent/unsupported preference.

Observed RED/GREEN regressions cover these boundaries. Separate in-session
review found exact positive decimal underflow and non-data record admission;
both were fixed and the six-file/206-test focused suite plus TypeScript passed.
Thirty-seven runnable unit guard controls and one intercepted Chromium guard
control were caught and restored byte-exactly; the new-file type canary failed
with TS2322 as intended. These are self-review/local fixture evidence, not
independent approval, native/Orca or all-component accessibility acceptance.

Complete candidate acceptance proc_56f26c5c70d5 passed all 15 steps: Web 1,559,
intercepted Chromium 61, Rust 2,890 passed / zero failed / 163 ignored across
146 result blocks, with no missing-corpus markers. All 614 protected source/
configuration fingerprints match. Newer upstream AR06 source still requires
combined integration acceptance before publication.

U16 still owns acknowledged conditional persistence, strict file decoding and
lossless export. U17 owns the production manager, diagnostic presentation and
transactional preview. U18 owns extension-wide review and closing integration
evidence. No production import/export gesture is delivered by this foundation.

## Evidence and inspected baseline

The five built-in palettes/System resolver live in `apps/knx-web/src/theme.ts`;
`SettingsPanel.tsx` renders their selector. `themeTokens.ts` provides pure
ADR-0022 role-pair evaluation and derives the build-time token boundary from
`styles.css`. Installed fonts are Inter, Space Grotesk and JetBrains Mono.
`appearance.ts` owns accent/density and `motion.ts` owns motion. They remain
independent preferences. `settingsStore.ts` is the only settings client;
`settings.rs` keeps schema v1 preferences as an opaque JSON map, and the
settings route patches named keys while retaining unmentioned keys.

Reuse the language manager's localized diagnostics, file-download gesture and
settings subscription shape, not its permissive format semantics. Language
packs accept incomplete catalogues/newer formats and their synchronous import
only confirms the cache write; the later server write can fail. A theme must
be complete before application, and installation success must await server
acknowledgment. No second authoritative store is permitted.

JSON object names should be unique; duplicate-name behavior differs between
implementations.[6] Theme text therefore passes a bounded duplicate-aware
JSON reader before conversion to the typed pack, including decoded/escaped
names. `JSON.parse` plus a reviver alone cannot recover overwritten names.
CSS custom-property cycles make their values invalid at computed-value time.[2]
V1 rejects **all aliases**, including missing references and cycles, rather
than relying on the CSS engine for validation. CSSOM parses a property value
when setting it; this is not a theme-pack security allow-list.[3] Use only
validated token names/values through `style.setProperty`, never stylesheet
text, selectors, `innerHTML`, `@import` or network assets. React warns against
untrusted `dangerouslySetInnerHTML`; pack names/diagnostics remain text.[5]

## JSON shape

Required fields: `format`, `formatVersion`, `tokenVersion`, `id`, `name`,
`version`, `colorScheme`, `tokens`. Optional: `accents` only. Every object has
an exact key contract; unknown fields/tokens are a whole-pack rejection.

| Field | V1 contract |
| --- | --- |
| format | Literal `knxbench-theme` |
| formatVersion / tokenVersion | Integer 1, no coercion; other versions rejected |
| id | `user-` followed by 1–43 lowercase ASCII letters/digits/hyphens; suffix starts with a letter/digit; maximum total length 48. Built-in/System/legacy IDs cannot be shadowed. |
| name | Text with a nonempty trimmed value, maximum 80 Unicode code points; preserve the accepted original text |
| version | Text with a nonempty trimmed value, maximum 32 Unicode code points; not a compatibility selector |
| colorScheme | `light` or `dark` |
| tokens | Exactly the 27 keys below; every value must match its token class |
| accents | Optional object keyed by a subset of the existing `violet`, `mint`, `blue`, `amber`, `rose` registry. Each entry has exactly `--knx-accent` and `--knx-on-accent`. Both values are opaque colours. Absent/unprovided variations use the base pair; UI offers only the declared variations. |

No C0/C1 controls, bidi-control characters or unpaired surrogates in metadata
or values. Strings remain data; no HTML interpretation. Accepted pack values
and metadata are preserved, not case-folded or silently corrected. Canonical
export recursively sorts object keys, uses two-space JSON indentation and a
final newline. No project, bus, credentials or unrelated settings are exported.

Limits are **product policy**, not claims from the cited standards: 65,536
UTF-8 bytes per file/serialized pack; nesting depth 8; maximum 1,024 JSON value
nodes; maximum 512 code points per token value; 16 installed packs and 524,288
UTF-8 bytes for their serialized map. Check file size before reading; check
actual encoded bytes again in the pure parser. Original rejected files remain
with their owner; neither validation nor preview writes them.

## Complete token boundary, tokenVersion 1

Build tests must compare this contract's key set to `requiredThemeTokens()`.
Adding a component paint token must not make old packs silently incomplete:
revise the token contract/version and explicitly reject unsupported versions.
Component-owned motion/density/control tokens and `--app-ui-scale` are never
pack keys.

| Class | Exact token keys | Accepted value grammar |
| --- | --- | --- |
| Opaque role colours | `--knx-bg`, `--knx-surface`, `--knx-foreground`, `--knx-accent`, `--knx-on-accent` | `#RRGGBB`, optionally an opaque `ff` alpha byte, hex case-insensitive |
| Other colours | `--knx-muted`, `--knx-border`, `--knx-border-hover`, `--knx-accent-tertiary`, `--knx-error-color`, `--knx-warning-color`, `--knx-success-color`, `--knx-overlay-backdrop`, `--knx-overlay-shadow` | Six/eight-digit hex only; alpha permitted |
| Gradients | `--knx-gradient-primary`, `--knx-gradient-display` | `none` or `linear-gradient(angledeg, colour, colour[, colour[, colour]])`; finite decimal angle 0–360; hex colours only; no stops/aliases/other functions |
| Shadows | `--knx-shadow-raised`, `--knx-shadow-hover` | `none` or one `xpx ypx blurpx spreadpx colour`; finite decimal x/y/spread −64–64, blur 0–128; six/eight-digit hex colour; no inset/multiple shadows |
| Fonts | `--knx-font-heading`, `--knx-font-body`, `--knx-font-mono` | Exact allow-list: `"Inter", sans-serif`, `"Space Grotesk", sans-serif`, `"JetBrains Mono", monospace`, `system-ui, sans-serif`, `ui-monospace, monospace`; no pack-provided fonts/assets |
| Shape | `--knx-radius-card`, `--knx-radius-button`, `--knx-radius-input` | Finite decimal CSS px value, 0–999 |
| Backdrop | `--knx-backdrop-image`, `--knx-backdrop-mask` | `none` or the same bounded linear-gradient grammar |
| Backdrop geometry | `--knx-backdrop-size` | `auto` or two finite decimal px dimensions, each 1–512 |

Numeric spelling is ASCII `-?(0|[1-9][0-9]*)(\.[0-9]+)?`, then checked against
its class bounds; negative spelling is accepted only where the bound allows
it. No leading plus, exponent, leading-dot or leading-zero integer spelling.
Gradient commas may have zero or one ASCII space after them; shadow/dimension
components are separated by exactly one ASCII space. No leading/trailing
whitespace in token values. Do not normalize an accepted spelling on export.

Full-value matching is mandatory. Reject `var()`, aliases, CSS escapes,
comments, selectors, delimiters, `!important`, extra functions, URLs, data
URIs, external fonts, scripts and arbitrary CSS. The limited grammar is
intentional; v1 does not represent every complex built-in backdrop. Built-ins
are **not exported as supposedly lossless v1 packs**. Installed valid v1 packs
are exportable. Provide a complete v1 example/template, with a user-prefixed
ID, rather than claiming a conversion of all five built-ins.

## Contrast and application

Reuse `evaluateThemeContrast()` for each base and each declared accent:
foreground/background, foreground/surface, on-accent/accent. All three pairs
must meet 4.5:1 using the evaluator's unrounded result. WCAG thresholds must
not be rounded into acceptance.[4] Unsupported/transparent role values fail,
not skip. This limited role contract is **not all-component WCAG acceptance**.

Application extends the existing theme resolver and root DOM effect. There
is no new KNX Core dependency or parallel stylesheet engine. Apply only the
27 validated keys plus the enum `color-scheme`; remove/restore only owned
properties on switching, including original value/priority. Do not clear
unrelated inline properties, motion, density or project style. Validate again
when resolving settings/cache content, not only at file import. Missing,
invalid or unsupported stored selection applies System visually but retains
the stored data/identity and exposes a diagnostic; mount effects must not
write fallback IDs over it.

## Persistence, concurrency and preview

Store the pack map at `settings.uiThemePacks`; keep selected identity at
`settings.theme`. Existing settings schema v1 already carries unknown values;
no domain/project/manufacturer schema or settings migration is needed for
these additive keys. Readers revalidate each entry and disclose invalid ones
without dropping them. Do not replace a corrupt map implicitly: offer its
recoverable raw export and reject mutations that would lose uninterpreted
entries. Known newer/refused or unhydrated settings disable installation,
replacement, removal and durable selection.

Installation/replacement/removal/select use the existing settings queue and
one acknowledged multi-key patch where changes are coupled. Extend the
existing PUT route with optional `expectedSettings`, comparing the relevant
current `theme`/`uiThemePacks` values under the same settings write lock; null
means absence. A mismatch returns conflict before writing. Older callers
remain unchanged. The comparison is for the single server's serialized
read-modify-write, **not** multiprocess filesystem collaboration. Preserve all
unmentioned keys and existing newer-file refusal/atomic writer behavior.
Update the client only from validated authoritative acknowledgment; a cache
quota failure is a cache diagnostic after a durable write, not a false claim
that the server rolled back. Transport ambiguity stays explicit and triggers
an authoritative reread, never a blind replay.

An existing ID needs explicit replacement consent tied to the inspected pack
version/content; concurrent changes force reinspection. Removing the active
pack atomically selects System. Server rejection leaves the last acknowledged
selection/map intact. Foreign invalid entries remain exportable and diagnosed.

Preview is temporary application state in the existing theme path, not a
settings write. Apply commits via the same acknowledged conditional mutation.
Cancel, Escape, dialog close/unmount and failed Apply restore the latest
acknowledged selection without obsolete overrides. A change in authoritative
pack/selection during preview cancels it and shows that newer acknowledged
state; an unrelated preference change does not discard preview. Reset/Cancel
remain keyboard reachable; no custom palette may suppress their gesture.
Full native/Orca/visual acceptance is tracked separately.

## Negative-fixture and acceptance matrix

| Mechanism | Required tests / invariant |
| --- | --- |
| Text parser | Valid escaped strings; malformed/trailing JSON; duplicate keys at every depth including equivalent escaped names; depth/node/UTF-8 limits; lone surrogates |
| Contract | Required/unknown fields; non-object sections; wrong types; fractional/string/newer versions; reserved/malformed IDs; metadata controls/limits; every missing/extra token |
| Values | Boundary decimals and overflow; extra suffix/comment/delimiter; CSS escapes, URL/data URI, selectors/script/HTML/function input; alias/missing/cycle rejection; invalid accent names/pairs |
| Contrast | Every three-role base/accent pair, alpha refusal, strict unsupported refusal and a below-threshold unrounded failure |
| DOM | Valid pack only; no requests/rules injected by hostile values; removal/restoration of owned root properties; unchanged built-ins/System/OS changes and independent preferences |
| Persistence | Unknown settings unchanged; restart/cache/hydration; acknowledged-only success; newer/refused settings; storage/network/ambiguous failure; duplicate replacement consent; limits and active removal |
| Concurrency | Conditional conflict; queued mutations and refresh races; replacement/removal/select from two clients; no stale full-map overwrite |
| Preview/UI | Apply/Cancel/Escape/unmount/failure rollback; authoritative-change cancel; retained invalid-entry export; reset, accessible diagnostics and DE/EN keyboard flows |
| Delivery | Semantic export/reimport, hostile fixtures, realistic restored guard mutations, separate in-session full-diff review and required integrated gates |

U14 resolves the contract; U15 supplies parser/runtime proof, U16 durable
operations, U17 management/preview and U18 extension-wide acceptance. No row
is implementation evidence until its actual tests have run.

Sources are registered from retrieved primary pages; access date 2026-10-02.

## Sources

[2] https://www.w3.org/TR/css-variables-1
[3] https://www.w3.org/TR/cssom-1
[4] https://www.w3.org/WAI/WCAG22/Understanding/contrast-minimum.html
[5] https://react.dev/reference/react-dom/components/common
[6] https://www.rfc-editor.org/rfc/rfc8259.html
