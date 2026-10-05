# KNXBench theme packs v1

Contract resolved by U14, 2026-10-02. **U15 runtime foundation delivered;
U16 persistence/file foundations are delivered/read back as 1f94808d;
U17 management is delivered/read back as4d9073ca. U18 extension-wide actual
acceptance is delivered/read back as1964fd6b; final metadata is in the handover.**
This document defines a KNXBench-owned format, not an existing interoperability
standard or a whole-application release/compatibility claim. Decision:
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
configuration fingerprints match. Combined source de1bf652 passes 22/22 steps
as proc_32a1aab25020: Web 1,559, Chromium 61, Rust 2,916 / zero failed /
164 ignored / 146 result blocks; twenty selected private offline cases and
the pinned 115-instance matrix pass. The same 614 protected files are stable.
Published as 9d1ae19d with fetched/live ref, full tree, all 21 owned artifacts
and zero outgoing commits verified. This is not independent/native approval.

U16 still owns acknowledged conditional persistence, strict file decoding and
lossless export. U17 owns the production manager, diagnostic presentation and
transactional preview. U18 owns extension-wide review and closing integration
evidence. No production import/export gesture is delivered by this foundation.

## U16 delivered persistence and file foundations

`themePackFiles.ts` checks declared and actual bytes, uses fatal UTF-8 with a
leading BOM accepted, then reuses duplicate-aware admission. Supported-pack
export revalidates its input and preserves accepted values/metadata in sorted
JSON. Built-ins are not converted into supposedly lossless v1 packs.

`themePackStorage.ts` plans installation, explicit replacement, selection and
active removal from a detached authoritative scope. Plans are deeply frozen;
replacement requires explicit consent even for the same ID/version. Selection
also reuses aggregate runtime admission, so a rejected store cannot acknowledge
a palette that the resolver will not paint. Unknown raw entries remain intact.

`settingsStore.ts` uses its existing queue, not another authoritative store.
The route advertises `conditionalPatchVersion: 1`; schema v1 alone is not that
capability. Preconditions compare theme/pack content under the existing write
lock. Client mutations reject unhydrated, pending, incompatible, uncertain or
unsupported scopes; all patched keys must have an inspected precondition.
Stored null is not absence and disarms these operations. Conflicts do not
replay; ambiguous responses reread authority and remain reported as ambiguous,
even if the reread succeeds. Cache failure after acknowledgment is distinct
from durable server failure. Older ordinary preference callers remain intact.

Every ordinary-write reply is also a new server observation: missing or changed
conditional capabilities revoke earlier claims. Only a compatible, matching
ordinary reply clears that intent's generation; a contradictory reply cannot
erase its local edit through a later unrelated acknowledgment. Ordinary
preferences still synchronize against older servers without conditional support,
while theme-pack conditional operations remain disarmed. Retired owner epochs
neither dispatch queued ordinary writes nor adopt late completions.

Recovery export is separately labelled `knxbench-theme-recovery`, version 1,
with `selectedTheme` and `uiThemePacks` only, at most 1,048,576 UTF-8 bytes.
It is an inert recovery document, **not an admitted/importable theme pack**.
It preserves browser-observed JSON values, not the original server file's
lexical bytes, duplicate names or numeric precision. Keep the original file
for original-file recovery; no byte-exact opaque recovery is claimed here.
Oversized/nonserializable recovery is diagnosed, never truncated or deleted.

Measured candidate evidence: 144 focused tests over five files and TypeScript
pass. The cold-restart file/settings roundtrip, same-version peer-content
conflicts for installation/removal/selection, mixed queue intentions and stale
refresh are named behavioral cases. Thirty-one unit guard controls are killed
and sources restored; two initially survived and now fail with strengthened
export/status assertions. A new-file TS2322 canary is caught and restored.
The separate in-session review's aggregate-selection finding is fixed after
observed RED/GREEN. The earlier frozen candidate passed 16 checks, but follow-up
review changed source: its acceptance is not final-current-source evidence.
Seven distinct compiled server controls are caught (the initially surviving
writer-lock control was closed by a held-lease/real-worker case, plus 20 repeated
named failures), with eight HTTP cases GREEN and original source restored.
Five additional ordinary queue controls are caught; latest capability observations,
unconfirmed intent preservation, legacy-server synchronization and owner reset
have RED/GREEN regressions. The new frozen candidate passed all 16 checks: Web 1,665, intercepted Chromium
61, Rust 2,925 passed / zero failed / 164 ignored, eight conditional HTTP cases,
17 generated bindings equal and 621 inputs unchanged. Actual-merged 36e922bc
passed all 22 checks as proc_613ec7154cf3: Web 1,665, intercepted Chromium 61,
ordinary Rust 2,940 / zero failed / 164 ignored across 148 result blocks,
eight conditional HTTP cases and 17 bindings equal. All 622 protected source/
configuration inputs and all 420 private files remain unchanged. Six explicitly
selected offline suites/11 private cases passed; this is narrower than U15's
historical twenty-case scope. The product matrix measured 115 instances and
113 unique packages, with status-only per-item outcomes and no private names.
Source 1f94808d5d9985b38fcf85021403bb4b05fea3e7 is published; exact remote ref,
20 owned artifacts and all 622 gated inputs read back equal. U17 still owns visible controls/diagnostics and
preview; U18 owns extension-wide acceptance. This is not independent approval.

<a id="u17-local-management-candidate-and-operator-workflow"></a>
## U17 verified management and operator workflow

Implementation passes the actual merged acceptance below; publication/readback
is recorded in the current handover, not inferred from a candidate run. Open
Settings → Appearance. Builtins are immutable and show application origin;
admitted imported entries show ID/name/version and saved selection. Preview is
temporary. Use Apply to confirm a guarded selection/install; Cancel/Escape or
closing the manager releases the candidate and restores the current confirmed
rendering. The root theme runtime owns the only DOM lease; report consumers are
read-only. Cross-client theme/map/accent changes revoke stale drafts/consent.

Import a `.knx-theme.json` file. File admission shows the actual structured kind
and path with live localized explanation. An ID collision opens a question
showing the old/new identity/version, initially focused on Cancel. Same ID/version
never implies consent or equivalent contents. Export on a validated installed
row downloads canonical JSON text; it preserves admitted values and metadata.
Builtins are not exported as misleading lossless v1 packs. Export recovery data
preserves the observed raw theme area and is deliberately non-importable; keep
the original settings file when original lexical bytes are required.

Remove requires a content-bound confirmation. Removing the active pack couples
map removal with System selection, retaining foreign entries and other settings.
Use system theme resets selection without deleting the map or rewriting accent,
density, motion or language. Available accent options follow the admitted visual
candidate; an unsupported stored accent remains retained and explained.

Apply uses U16's existing conditional queue. A dispatched write is not undone by
closing Settings; this is stated while pending, with false-undo Cancel disabled.
Repeated Apply dispatches once. A 409 restores the last known acknowledgment;
the normal focus refresh adopts a later peer snapshot. An uncertain response can
read once for reconciliation without replaying the write. Server acknowledgment
and local cache failure are reported separately in manager and original selector.

Historical candidate evidence:31 actual parent and five root cases, latest scoped74/TypeScript
without stderr, six caught/restored behavioral controls and a caught/restored
new-module TS2322 inclusion canary. The final frontend candidate passed
Web1,702/95 files, Chromium69/no skip/no flaky, production build and types over262
unchanged inputs. Eight behavioral controls were caught/restored. Full branch13/13 repository commands pass, Rust2940/0/164,17 equal bindings
and693 inputs unchanged; actual-merged acceptance/publication remain pending. Legacy full Web
suite emits fixture stderr; no warning-free whole-suite claim. Actual intercepted
Chromium is not native WebKitGTK/Orca or general WCAG/ETS/global-alpha approval.

U17-R1 acceptance audit added explicit actual-manager hostile-file rejection,
System reset/OS transitions and HTTP500 rollback/no replay. Eleven manager
browser cases and three further compiled browser guard controls pass, canonical
runtime restored unchanged. The previous full69-browser receipt predates this
test-only delta; renewed full actual-merged gates are recorded below. The cancelled first
actual attempt and initial unused-import mutant compilation are retained as
non-acceptance/instrumentation records, not successful behavioral evidence.

Actual chain proc_cdcd42b97d47 on f16f1e40 passed23 commands:17 repository
and six explicitly selected offline inventory/execution commands. Web1702 in95
files, Chromium72 with zero failure/skip/flaky, ordinary Rust2984/0/165 across149
result blocks, compiled ignored inventory165 and17 generated bindings equal.
Six selected offline suites execute27 private cases; the release matrix is one
additional case measuring115 instances/113 unique packages with status-only
per-item output. All697 source/configuration inputs and420 private files remain
unchanged; temporary corpus link removed. This is not a broad ignored/live sweep.
The deliberately cancelled attempt1 and lifecycle-interrupted attempt2 remain
non-acceptance records. Subsequent integration with upstream5ca570a0 changes
Markdown only; all697 gated inputs remain equal, with document gates required
before publication. U18-R1 cross-palette representative-component state coverage
was open at U17; U18 closes it in the actual extension receipt below.

## Local design example: Modern Retro Green CRT

An importable complete v1 phosphor-green palette and a separately labelled
interactive target-design study are documented in
[Modern Retro Green CRT](DESIGN_RETRO_GREEN_CRT.md). The study's animation,
dedicated selection fill and Save-only purple are not injected by its v1 pack.
The guide proposes explicit component/token-version changes for those effects;
this example does not expand the accepted format or claim new runtime features.
The reference-image revision is palette version1.1.0 with the same ID; replacing
an installed1.0 pack requires the existing explicit confirmation. Its softer
ink, quiet green rules and mint primary gradient are token-only changes. The
updated standalone native-table study and reproducible browser receipt remain
separate from production component behavior.

The 2026-10-03 production animation follow-up is independently selected as
**Motion style → CRT**, not injected by the pack. The real workbench/tree/address
table now have application-owned fill and bounded light/activation, with Off/OS
reduced-motion cancellation. Save-only purple and exact selection colors remain
unimplemented semantic-role proposals. The v1 token/schema boundary is unchanged;
see the guide's production section and ADR-0022 follow-up. The authorized delivery
committed it as16c9d774 and integrated it as3d03aea5 onto current main, with merged
frontend/browser/workspace gates. Current refs/handover establish publication;
importing a pack still cannot implicitly enable CRT or override Motion Off.

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

> **Update 2026-10-05 ([ADR-0079](adr/0079-theme-choice-is-one-dropdown.md)):**
> preview, the per-theme cards and the "Use system theme" button were removed on
> the user's decision. The Theme dropdown is the only chooser; an import is
> installed and selected in one conditional write; Export/Remove act on the
> selected pack. Shipped packs (`apps/knx-web/themes/*.knx-theme.json`, admitted by
> `src/bundledThemes.ts`) are selectable without being written into
> `uiThemePacks`; an installed pack with the same id takes their place. Neon Grid
> and Bitcoin DeFi are no longer built-in palettes. The preview paragraphs below
> are historical.

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

## U18 representative-state closing verification

The [original candidate-status anchor](history/IMPLEMENTATION_STATUS_2026-09.md#u18-representative-theme-states--candidate-acceptance-running)
is retained for backward-compatible links; its current section records actual acceptance.

The `ui-theme-closing` candidate adds test-only actual `GroupAddressTable`, `Inspector`, contextual inline-address validation, `Overlay` and structured theme diagnostics to the offline real Appearance/root fixture. Five built-ins, both System resolutions and admitted imported light/dark palettes are exercised. The frozen final-source matrix passed 10/10; three rendered-style sabotage controls were detected by named state assertions and a new-module TS2322 type-inclusion canary was detected/restored. Dedicated `npm run check:theme-fixtures` includes the new modules and test in the TypeScript project.

**Actual-integrated extension acceptance verified (2026-10-03 15:23 CEST):** Persistent `proc_e95dfc308724` completed normally; the coupled receipts on `1660911bdd5bf350c6042bbeb9a2b694ad66fa41` were checked against committed blobs and the final working tree. **24/24 commands** passed (18 repository, six explicit offline inventory/execution commands): Web **1,702**, intercepted Chromium **82** with zero failures/skips/flaky, representative matrix **10/10**, ordinary Rust **2,995 passed / 0 failed / 165 ignored across 150 blocks**, and 17 generated bindings identical. Three rendered-style controls failed the named selection/focus/validation assertions; the TS2322 fixture-inclusion canary remains separately recorded. All **703** protected source/config inputs match exactly.

Offline inventories reconcile **11 product + 16 injected-server + 1 matrix cases = 28 executed cases**. The matrix measured **115 instances / 113 unique packages**; its case is already included in28. All **421** private source files (420 OriginalData files plus project_dump) remained unchanged. Private stdout was classified in memory; only aggregate receipts were persisted. Own fixture links and transient matrix output were removed. Simulator download tests use injected transport, not a real KNX tunnel.

The separately labelled in-session full-extension self-review traces admission/file decoding → immutable plans → conditional queue/API acknowledgment → single root DOM lease → manager/selector/diagnostics → semantic export/reimport and representative states. U18-R1 is closed by real components, not generic markup. No new blocking production finding; this is **self-review, not independent approval**. Historical running/candidate-only observations are superseded for their stated scopes. Later integration `ef4cfb92` brings eight upstream Markdown paths, zero protected-source changes; all703 actual-gated inputs remain exact. Closure Markdown/integrity gates passed; source delivery1964fd6b was pushed and read back with live/fetched refs, full tree, all703 inputs and15 owned artifacts exact, zero outgoing commits.52 completed own scratch entries and own node_modules were removed after worker/cwd checks; only final metadata housekeeping remains. The UI reservation is released in the current handover.

No native WebKitGTK/Orca, all-component WCAG, Alpha/ETS compatibility or real discovery/tunnel/commissioning/hardware acceptance follows. User instructions for import/export, reversible preview, explicit replacement, recovery and fallback are in the U17 user instructions above.

**Candidate gate verified (2026-10-03 13:48 CEST):** The notified runner completed normally and its complete receipt was checked:18/18 commands, Web1702, Chromium82 (zero failures/skips/flaky), final palette matrix10/10, Rust2984 passed/0 failed/165 ignored across149 result blocks; all701 frozen source/config inputs remain exact. This supersedes the earlier running-candidate observations, not the still-pending actual-integrated/offline/private acceptance or publication. In-session review found no new blocking production issue; it is not an independent third-party approval. Integration against fetched documentation-only upstream8af45464 is next.
