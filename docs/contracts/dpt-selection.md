# DPT presentation, selection and provenance

## Name-first monitor and address extension (2026-10-12 source)

The shared display vocabulary now presents **Name (main.sub)**, for example
`Switch (1.001)` / `Schalten (1.001)` and `Temperature (9.001)` /
`Temperatur (9.001)`. Communication-object tables, selectors and exact filters,
group-address table/Inspector declarations and linked types, and the monitor
telegram table/details use the same presenter. Group-address search accepts
the visible name/number as well as the original reference. Conflicts remain
conflicts; their known references are named, never reduced to one winning type.

The monitor compose form now uses a native DPT select over the existing 454-ID
EN/DE vocabulary. A decoded row prefills its exact reference; selecting another
type only changes the send draft. Focus, blur and selection do not send a
telegram or edit the project. The automatic option reuses existing group-address
resolution; absent/conflicting/unknown resolution and closed/stale/unverified
session guards retain their existing authority. Only an explicit **Send** uses
the unchanged API. The server still validates encoding support; the vocabulary
is not a codec/device-compatibility list.

Known `DPST-...` identifiers are no longer inline in object summaries. The
exact original reference remains available in a tooltip and the object editor’s
closed-by-default **Technical reference / Technische Referenz** disclosure.
Main-only references display a family (`9.*`), not a guessed subtype. Unlisted
numeric subtypes display their dotted number without an invented name; opaque
or malformed values remain verbatim. Extra select options preserve exact
unknown and noncanonical values, including `DPST-09-001`, without turning them
into a reset or normalized write. Raw error text and exported capture data remain
unchanged. No domain/API/parser/project/product schema or dependency changes.

### Notation evidence and why the old IDs appeared

The existing Core [`DptRef`](../../crates/knx-core/src/dpt/mod.rs) already stores
`main: u16` and optional `sub: u16`; it is not a UI string domain model. Its
`Display` implementation emits the interchange reference `DPST-<main>-<sub>`
or `DPT-<main>`, which existing API/import/export callers use. The previous
monitor and group-address views rendered that technical representation directly.
That source-level leak explains the old UI; it is not evidence that an alternate
KNX standard was intended, nor justification for exposing XML-style identifiers
to an engineer. The original author’s undocumented intent is not inferred.

The primary [KNX Datapoint Type explanation](https://support.knx.org/hc/en-us/articles/115001133744-Datapoint-Type)
explicitly specifies `X.YYY` notation (main type and subtype). The already audited
v02.02.01 inventory above remains this vocabulary’s version; the support article
is notation evidence, not a replacement inventory or new support claim.
Keep at least three subtype digits and retain actual four-digit IDs; do not
format the identifier as a floating-point number.

### Verification

Named pre-change failures cover name-first vocabulary, the missing monitor
select, inline raw IDs, unresolved/conflict guidance, and an actual narrow
select overflow (471 px into a form with 332 px available). Final full Web suite:
2,841 passed / 0 failed across 182 files. Eight production-built, strictly
intercepted Chromium cases cover EN/DE, Graphite/Porcelain and 400/1440 px,
exact send payloads/zero incidental sends, alias preservation, closed/reachable
technical disclosure and bounded controls. All eight retained frames inspected.
The source graph guard admits only the pure label module; API/mutation
inventories remain unchanged. Build and explicit browser-fixture typing pass.
Self-review, not an independent review. No live bus, native WebKitGTK/Orca, new
codec, full ETS claim, release or running-container deployment is implied.

**Follow-up (2026-10-12, evening):** `device-editor-layout.e2e.ts` still expected
the raw `DPST-9-1` / `DPST-1-1` inline and a separate raw-reference line, so its
four cases (EN/DE × 360/1440 px) had been red since the name-first change. The
product matched this amendment; the test was wrong. It now asserts the name-first
label (`Temperature (9.001)` / `Temperatur (9.001)`, `Switch (1.001)` /
`Schalten (1.001)`), the exact reference in the cell tooltip, no `DPST` in the
visible cell, the separately labelled master text, and the existing
no-character-level-wrapping check. Reverting the cell to the raw reference fails
it by name.

The following section retains the original selection scope and evidence;
this amendment supersedes its number-first and always-inline-reference UI.


## Scope

Local source update, 2026-10-11, based on `9298242c`. This extends the
[communication-object table contract](../COMMUNICATION_OBJECT_TABLE.md), not
KNX codecs, manufacturer/application semantics or storage. Release, main
integration and deployment remain separate from the original local evidence.
The owner subsequently authorized feature commit/no-ff main merge/push; actual
merged-source acceptance is recorded in the
[publication receipt](../evidence/dpt-selection-publication-2026-10-11.json).
This does not publish a new release or replace the running container.

## Display vocabulary

`apps/knx-web/src/dpt-labels-v02.02.01.json` is a versioned **display
vocabulary**, with independently authored English/German engineering labels.
It is not a manufacturer database, a supported-codec registry or a device
compatibility allowlist. It contains no copied specification pages, diagrams,
encoding tables, manufacturer data or proprietary master XML.

Evidence: **03_07_02 Datapoint Types v02.02.01 AS**, §2, printed pages 17–25,
provides 453 numbered types. §6.16, printed page 192, defines **249.600**, which
is absent from that overview. The 454 IDs exactly match the earlier independent
[document inventory](../spec-audits/2026-10-07-dpt-document-audit.md) and
[type inventory](../spec-audits/2026-10-07-dpt-types.csv). Definition checks also
retain the status/command distinction (§4.8, p131), thermoelectric quantity
`14.073` in V/K (§3.15 table, p46), and meter volume flow `14.1200` in m³/h
(§9.4, p212). Acronyms without a verified expansion remain acronyms in the
localized label; no invented semantics follow from a short label.

The vocabulary resolves by the **UI language**, independently of product-text
language. Additional language packs use English labels. Existing master-data
`dpt_text` and its language-fallback indication remain separately visible as
**Master data / Stammdaten**, never rewritten or misrepresented as a new UI
translation. The original reference remains visible beside the numbered label.

## Selection and lossless presentation

- The DPT editor is a native select, not a free-text input. Its known options
  are numerically ordered and display `9.001 – Temperature` / `9.001 – Temperatur`,
  `17.001 – Scene number` / `17.001 – Szenennummer`, and equivalent labels.
- Subnumbers use at least three digits; actual four-digit subnumbers such as
  `1.1200` are neither truncated nor treated as decimals.
- Option values remain the existing canonical API grammar, e.g. `DPST-9-1`.
  The display vocabulary does not change `DptRef`, DTOs, history or project files.
- A main-only, unlisted, future/vendor-specific or noncanonical stored reference
  stays selected as its **exact original value**. An extra current/draft option
  prevents a native select from silently becoming empty when a value is unlisted.
  Recognizing a display alias does not normalize its stored value.
- An empty project value remains empty. If the server supplies `program_dpt`,
  the empty option explains the program default without saving it as a project
  edit. Choosing the empty option explicitly clears the project DPT through the
  existing `null` mutation; ordinary focus/blur performs no write.
- Existing edit gestures stay: select a draft, then leave the field or press
  Enter to apply. Existing per-object pending/lifetime/refusal rules remain in
  force. No synthetic edits, automatic retries or new server concurrency policy.
- Table rows, exact-type filters and search reuse the same localized names and
  dotted numbers. Raw-reference search and numeric DPT sorting are retained;
  display/search never mutate canonical objects or filters.

## Field-local provenance

The DPT and description each own their source badge in the field heading.
`Program` becomes **Source: application program / Quelle: Applikationsprogramm**,
with a tooltip explaining that it is application-supplied rather than a project
edit. `ProgramRef`, `Instance`, `Inferred` and `UserEdit` have distinct localized
labels/explanations. Unrecognized source layers retain their verbatim identifier,
including JavaScript prototype-property names; no source is guessed.
Since UI-02 (18 October 2026) the routine `Instance` layer is not shown in the
heading; the DPT or description field's own opened technical reference names it, and the data is
unchanged ([controls and separators](controls-and-separators.md#value-source-labels)).

A local inline-size container bounds the expanded editor to the table's viewport,
while the wide summary table retains horizontal scrolling and all its columns.
Native selects and badges wrap within their own fields rather than independent
flex items. No shell/theme-wide styling change is needed.

## Acceptance and boundaries

Named before-change failures cover missing native selects/localized field context,
unknown-value preservation, program-default presentation, description ownership,
localized search and narrow dropdown sizing. An additional self-review regression
pins verbatim unknown provenance rather than object-prototype lookup. Ordinary
regressions also cover current/draft aliases, explicit reset, the exact 454-ID
inventory, existing pending/refusal/removal/device-switch behaviour and translations.

The actual gate outcomes belong to the dated implementation-status entry and
local handover/evidence. Offline intercepted **production-built** Chromium checks
cover EN/DE, 1440/400 px, unchanged snapshots/zero focus-only writes, exact existing
write values, preserved unknowns and bounded dropdown geometry. They are not an
unmocked server, native WebKitGTK/Orca, manufacturer, ETS or real-hardware verdict.
The underlying domain/server validators remain authoritative; selecting a named
DPT does not establish that a device or KNXBench's bus codec implements it.
