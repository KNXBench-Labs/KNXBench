# Device parameter workspace

## Presentation contract (2026-10-08)

The device workspace has five keyboard-accessible tabs, in order:
Communication objects, Parameters, Product data, Diagnostics, Manufacturer fields.

- **Parameters** keeps user-accessible evaluated fields and actual edit errors.
  It does not repeat evaluation diagnostics, translation summaries, unmatched
  stored values or the former Access None disclosure above the editor.
- **Diagnostics** shows warnings and informational notes, translation summaries,
  and every stored-but-unmatched value. Headline groups use diagnostic kind,
  normalized severity, exact scope (module node/id/definition) and fallback
  message. The count is the number of original occurrences, not groups.
  Unknown/missing severity remains a warning; unknown kinds keep their server
  message. Different scopes, causes or fallback messages are never conflated.
- **Manufacturer fields** keeps evaluated fields explicitly marked Access Read
  or Access None, in source section/field order. Each has an access-specific
  reason and is inspection-only, even if malformed DTO flags claim editability.
  Other disabled fields remain in Parameters: they are not automatically
  classified as manufacturer access restrictions.

A repeated message is displayed once per group, e.g. six noBranchMatched
records become one note with “6 occurrences”. Its concise explanation says
that the current controlling value matched no option; it does not claim a
manufacturer defect or invent which value/condition caused it. Each original
technical record is available under an initially collapsed disclosure.
Copy details copies every record in group order, newline-separated; a single
record retains its exact original clipboard payload. Technical detail remains
untranslated English diagnostic data, not parsed into domain facts.

## Architecture and integrity

`DeviceWorkspace` owns one `useDeviceParameters` read model. All three parameter
views consume that same state via `ParameterPanelContent`: switching tabs does
not fetch again, and the existing successful write response updates the views
and republishes the authoritative project tree. Device/product-language/project
snapshot refresh and stale-GET guards retain the existing behavior. Only the
editor has writable user fields; backend write admission remains authoritative.

`parameterPresentation.ts` contains pure grouping/access partitioning. This is
UI presentation only: no core/application/protocol dependency, server/DTO change,
product/project migration, manufacturer-script execution or new compatibility
claim. ADR-0080 remains the source for existing effective-access/write rules;
this replaces its earlier per-section Access None fold presentation.

## Boundaries

Only fields already present in evaluated sections can be shown; this is not a
complete static manufacturer-field inventory. The DTO does not provide structured
controlling-value/condition information for every diagnostic, so the UI does not
invent detailed causes by parsing Rust Debug text. Existing unsupported
calculations/evaluation semantics remain unsupported. Actual load failures and
rejected edits still appear where the failed action happened.

## Verification

The retained [Chromium CLI verifier](parameter-workspace/verify-browser.js)
expects the offline fixture server on loopback port 4194. Start it from
`apps/knx-web` with `npx vite --config vite.fixtures.config.ts --port 4194
--strictPort --host 127.0.0.1`, open `about:blank` in an isolated Chromium CLI
session, then run `run-code --filename` with the verifier's absolute path.
The script intercepts all requests before navigating and refuses unexpected
API or non-loopback requests. It is a component fixture, not a production or
hardware test; never substitute the running installation's URL.

Test-first workspace regressions failed on the old inline diagnostics and
three-tab UI, then passed on the implementation. Tests cover grouping without
record loss, scope/severity/fallback distinctions, 10,000 repeated records,
unknown severities, exact clipboard details, restricted-field refusal despite
inconsistent write flags, normal editing, translations and keyboard navigation.

The [verification receipt](parameter-workspace/verification.json) records the
accepted source hashes and actual local tests/build/type/repository gates.
Actual Chromium exercised the real Inspector/DeviceWorkspace components with
explicitly synthetic intercepted API data: EN/DE, 400/1440px and the shipped
Porcelain/Graphite/LCARS palette CSS. No production or hardware requests.
This is not native WebKitGTK/Orca/full accessibility or device compatibility
certification. Source self-review, not independent approval. Local source
implementation is distinct from commit/push/container activation.
