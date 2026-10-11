# Communication-object DPT selection and provenance

## Scope

Local source update, 2026-10-11, based on `9298242c`. This extends the
[communication-object table contract](../COMMUNICATION_OBJECT_TABLE.md), not
KNX codecs, manufacturer/application semantics or storage. Release, main
integration and deployment remain separate.

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
