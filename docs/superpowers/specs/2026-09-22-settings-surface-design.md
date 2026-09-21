# Design — T10 settings surface beyond appearance and language

**Status:** proposed, 2026-09-22. This document is the implementation gate for
goal task T10/D8. It extends ADR-0029's existing versioned settings record; it
does not replace that architecture.

## 1. Intent and success

The application already has one settings file and one settings overlay, but
the overlay is visually flat, line-scan exclusions can be edited only from the
scan view, gateway fields always start empty, and rare settings-file notices
reach a German UI as English server prose. The goal is one coherent settings
surface with only preferences that have real consumers today.

Success means:

1. appearance, language/data and bus/diagnostics preferences are visibly
   grouped in the existing `SettingsPanel`;
2. the user may set one optional preferred KNXnet/IP gateway that seeds a new
   Bus Monitor or Line Scan view without initiating any network activity;
3. protected line-scan exclusions have one editor and one storage key, rendered
   both in Settings and in Line Scan;
4. settings-file migration/refusal/quarantine diagnostics are translated from
   machine-readable status and parameters while the session log retains the
   same diagnosis;
5. every new value persists in ADR-0029's `settings.json` record, reloads, and
   is consumed; no dead option or second persistence mechanism appears.

Group-address output remains fixed KNX slash notation with no notation setting.
Individual addresses in the line-scan exclusion editor remain dotted, because
they are physical addresses rather than group addresses.

## 2. Measured inventory

ADR-0029 and the current frontend establish the following inventory:

| Key | Owner | Kind | Settings UI today | Decision |
| --- | --- | --- | --- | --- |
| `theme` | `theme.ts` | preference | yes | Appearance |
| `accent` | `appearance.ts` | preference | yes | Appearance |
| `density` | `appearance.ts` | preference | yes | Appearance |
| `motionLevel` | `motion.ts` | preference | yes | Appearance |
| `motionStyle` | `motion.ts` | preference | yes | Appearance |
| `uiLanguage` | `uiLanguage.ts` | preference | yes | Language & data |
| `uiLanguagePacks` | `languagePack.ts` | preference/data | yes | Language & data |
| `productLanguage` | `productLanguage.ts` | preference | yes | Language & data |
| `lineScanExclusions` | `LineScanPanel.tsx` | preference | no | Bus & diagnostics |
| `groupAddressNotation` | retired/ownerless | preserved legacy preference | no | never exposed; slash display is fixed |
| `settings-cache` | `settingsStore.ts` | cache | no | infrastructure only |
| `settings-adopted` | `settingsStore.ts` | migration marker | no | infrastructure only |
| `project-context` | `busContext.ts` | window/session state | no | remains local |
| `bus-session-context` | `busContext.ts` | window/session state | no | remains local |
| `context-changed` | `busContext.ts` | window signal | no | remains local |

One new preference is justified by current consumers:

| Key | Type/default | Consumers |
| --- | --- | --- |
| `preferredGateway` | optional trimmed string; absent by default | initial value of Bus Monitor and Line Scan gateway fields |

ADR-0029's browser-era adoption table remains closed historical data. Neither
`lineScanExclusions` nor `preferredGateway` has an old browser-era key, so no
settings schema migration is required.
The server continues to preserve unknown keys and to validate document shape,
while actual bus requests remain subject to the existing server-side
`SocketAddrV4` trust-boundary validation.

An old `groupAddressNotation` value may remain in `settings.json` under the
unknown-key preservation rule. Nothing reads, rewrites or deletes it, and it
does not justify a selector: group-address output remains slash-only.

## 3. Surface and ownership

### D1. Group the existing panel; do not create another settings mechanism

`SettingsPanel.tsx` remains the only application settings overlay. It receives
three semantic sections with translated headings:

- **Appearance:** theme, accent, density, motion style and motion level;
- **Language & data:** UI language, installed language packs and product-data
  language;
- **Bus & diagnostics:** preferred gateway and protected line-scan exclusions.

The sections use existing design tokens and overlay patterns. Any new motion
or transition remains inside
`@media (prefers-reduced-motion: no-preference)` and uses existing
`--knx-*` motion tokens. No per-theme CSS branch is introduced.

### D2. Preference modules own validation and defaults

Add a small `gatewayPreference.ts` beside the existing preference modules. It
exports the stable key, a loader that returns a trimmed non-empty string or
`""`, a saver that removes the key for blank input, and a hook subscribed via
`useSettingsRevision()`. It uses `settingsStorage`; it does not call `fetch`,
discover interfaces, open a socket or duplicate `SocketAddrV4` parsing.

Move the existing exclusion parsing and update operations from
`LineScanPanel.tsx` into `lineScanExclusions.ts`. Its loader preserves every
existing array string byte-for-byte and in order, including duplicates and
range-invalid values written by the old regex-only editor. Invalid legacy
entries are displayed and marked invalid, block estimate/start, and remain
removable; they are never silently dropped or rewritten. New additions must
parse as a complete individual address and may not duplicate an existing
entry. The shipped default remains an empty list.

No generic settings registry or schema-driven form is added. The record is
already intentionally open-ended, and each preference module already owns its
vocabulary.

`settingsStore` gains one data-integrity rule for its asynchronous bootstrap:
every `setSetting` call before authoritative hydration records the last patch
value for that key in a small in-memory journal, including `null` deletion.
After the final GET/adoption response is applied, the store overlays the
journal on that document, marks synchronization active, and sends the coalesced
patch through the normal serialized PUT queue. The server response may supply
untouched keys but may never erase a local edit made while it was in flight.
The journal is then cleared. A failed GET leaves the edited cache and journal
intact rather than pretending the server persisted them.

### D3. One exclusion editor, rendered in two places

Extract `LineScanExclusionsEditor.tsx` from the existing Line Scan markup. It
uses the shared exclusion module and preserves current behavior: list entries,
protected label, two-click removal confirmation, add validation, localized
copy and disabled controls while a scan owns its frozen exclusion snapshot.

`SettingsPanel` renders the editor enabled. `LineScanPanel` renders the same
component with its current `controlsLocked` state and continues to copy the
configured list into `activeExclusions` when a scan starts. Editing the global
setting never rewrites an active request.

The component is not a second store: both placements read and write the same
`settings.json` key through `settingsStore`.

## 4. Preferred gateway data flow

### D4. Seed once from the authoritative record, never write back implicitly

`BusMonitorPanel` and `LineScanPanel` initialize their local gateway field from
the synchronous cache so the first paint stays stable. They also observe one
explicit `settingsStore` hydration state. When the first authoritative GET or
adoption response arrives, each panel replaces the cached seed only if its
field is still untouched and no session, estimate or scan has started; it then
marks seeding complete. If the user typed or selected discovery before that
response, the local value wins. Later preference changes never reseed the
mounted panel. A failed GET leaves the cached seed in place.

After this one-time resolution the local field belongs to the workflow in
progress:

- typing a manual endpoint changes only that field;
- selecting a discovered interface changes only the Bus Monitor field;
- either action leaves `preferredGateway` untouched;
- changing the preference in Settings does not overwrite a resolved mounted
  field or an active session/scan;
- the next fresh mount starts with the then-current preference;
- an absent preference preserves today's empty field.

This makes preference changes explicit and prevents discovery from silently
turning a transient interface into a permanent default. Storing the string does
not connect, discover, scan or otherwise touch KNXnet/IP.

The settings input uses a documentation-address placeholder only. Blank input
unsets the preference. Invalid non-blank text may be stored, but the consuming
bus endpoint remains the authority and reports its existing actionable error;
the frontend must not introduce a subtly different endpoint grammar.

## 5. Machine-readable settings diagnostics

### D5. A typed diagnostic crosses both settings and session-log APIs

Replace `SettingsDto.notice` with an optional tagged `SettingsDiagnostic` whose
variants preserve every currently distinct outcome:

- `migrated { fromVersion, toVersion }` for an on-disk migration;
- `adopted { fromVersion, toVersion }` for browser-era handover;
- `refusedNewer { fileVersion, currentVersion }`;
- `quarantined { reason, movedTo }`, where `reason` is a stable enum such as
  `unreadable`, `invalidJson`, `notObject`, `missingSchemaVersion` or
  `settingsNotObject` rather than prose;
- `ok` and `absent` carry no diagnostic.

The quarantine path retains its original human-readable detail for the server
fallback/debug message, but the UI switches only on the stable reason code.
Adoption must not masquerade as a version-0 disk migration.

The same typed diagnostic is attached optionally to the corresponding session
log entry. The server continues to store an English fallback `message`, so
debug reports and older clients remain useful. `LogPanel` and `SettingsPanel`
share one `settingsDiagnostic.ts` formatter that maps recognized diagnostics
to typed keys and parameters in `messages/en.ts`/`de.ts`, falling back to
`message` only for an unknown or absent diagnostic. This resolves §122 in both
user-visible locations without matching English prose.

`settingsStore.ts` retains the latest non-quiet load/adoption diagnostic in its
subscribed in-memory state. The cached settings document remains preferences
only, so an old diagnostic is not replayed after a fresh page load unless the
server reports it again.

Write failures from `PUT /api/settings` remain ordinary server errors and keep
the current console warning; they are not load-status diagnostics. This task
does not create a new global notification bus.

## 6. Explicit non-goals

- No default for newly created domain entities: no current creation flow has a
  shared default contract that a setting could consume.
- No group-address-style or notation option: display stays slash-only and
  compatible dotted input/search remains a boundary behavior.
- No project path, catalog path or data-directory preference: those locations
  are owned by the server/launch configuration, and there is no safe live UI
  consumer.
- No routing, tunnelling, scan timing or write defaults beyond the one gateway
  seed and existing protected exclusions.
- No synchronization feed between open windows; ADR-0029 §Consequences and
  KNOWN_LIMITATIONS §121 remain unchanged.
- No KNX bus, multicast, LAN or hardware test.

## 7. Test contract

Implementation follows RED → GREEN for each behavior.

1. `gatewayPreference.test.ts` proves missing/blank defaults, persistence,
   reload from the settings cache and explicit unset.
2. Bus Monitor tests prove cache seeding, delayed authoritative replacement
   while untouched, preservation of typing/discovery before the response, no
   preference write-back and no later reseed.
3. Line Scan tests prove the same cache/hydration rules and that its request
   uses the resolved local field.
4. `lineScanExclusions.test.ts` starts from an old cache containing duplicates
   and invalid strings and proves byte-for-byte ordered preservation. Component
   tests prove both Settings and Line Scan edit the same key, mark invalid
   legacy entries, block scan work until removal, retain two-click removal and
   disable edits during a scan. New invalid/duplicate additions are rejected.
5. Server settings-route tests prove `migrated`, `adopted`, `refusedNewer` and
   every quarantine reason remain distinct without `notice`, and the session
   log entry carries the same typed diagnostic plus English fallback.
6. Settings-store/panel/LogPanel tests prove all diagnostics render localized
   English/German catalogue messages without matching server prose and unknown
   events fall back safely.
7. A delayed-GET store/SettingsPanel test changes `preferredGateway` and an
   existing preference before hydration, then proves the response keeps every
   untouched server key, preserves the local edits in cache and sends one
   coalesced PUT patch after synchronization. The adoption-conflict re-read
   path obeys the same journal rule.
8. Existing browser-era adoption tests prove the eight old preferences still
   survive migration unchanged; the new keys do not alter that closed table.
9. `motionGuard.test.ts`, TypeScript and the whole frontend suite protect the
   theme/motion/i18n constraints.

## 8. Files and documentation

Expected product files:

- add `apps/knx-web/src/gatewayPreference.ts` and test;
- add `apps/knx-web/src/lineScanExclusions.ts`, editor component and tests;
- add `apps/knx-web/src/settingsDiagnostic.ts` and tests as the one frontend
  diagnostic-to-message mapping;
- modify `SettingsPanel.tsx`, `LineScanPanel.tsx`, `BusMonitorPanel.tsx`, their
  tests, message catalogues and token-based styles;
- modify `settingsStore.ts` and tests for typed retained diagnostics;
- modify server settings/load types, `settings_routes.rs`, session-log DTOs and
  HTTP settings tests for the diagnostic contract and fallback log formatting;
- modify `LogPanel` and its tests to translate known diagnostic events;

All new `.ts`/`.tsx` files receive the required purpose-sentence header.
`docs/IMPLEMENTATION_STATUS.md` records the delivered surface and reconciles
D8; ADR-0029 is amended to state that `settings.json` may now contain an
unencrypted user-entered gateway endpoint (never credentials), is included in
data-directory backups and therefore carries installation-network metadata;
`docs/KNOWN_LIMITATIONS.md` §122 is marked resolved while §121 stays open.
`docs/LIMITATION_TRIAGE.md` is generated/guarded and must not be hand-edited.

Required completion gates are frontend Vitest and TypeScript, Rust formatting,
workspace Clippy/tests, layering, headers, anchors and `cargo deny`; no real
network or KNX operation is part of verification.
